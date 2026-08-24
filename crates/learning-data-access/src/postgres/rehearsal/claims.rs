//! Verified claim state transitions through broker capabilities.

use domain::{
    DispatchedClaimHandle, RehearsalClaimRoot, RehearsalClaimSubmissionInput,
    RehearsalSubmissionClaimDecision, decide_submission_claim,
    mark_rehearsal_submission_dispatched, rehearsal_claim_submission_input_fingerprint,
};
use question_model::{RehearsalGradeOperationId, RehearsalSubmissionClaimId};

#[cfg(feature = "test-support")]
use domain::RehearsalValidatedSubmissionRequest;

use super::super::*;
use super::{auth, hydration, integrity};

#[cfg(feature = "test-support")]
pub(super) async fn claim(
    store: &PostgresStore,
    context: TenantContext,
    command: crate::ClaimRehearsalSubmissionCommand,
) -> Result<crate::RehearsalSubmissionClaimResult, StoreError> {
    let tenant = context.tenant_id();
    let mut tx = store.begin_tenant(context).await?;
    let result = claim_in_tx(&mut tx, tenant, command).await?;
    tx.commit().await.map_err(map_sqlx_error)?;
    Ok(result)
}

#[cfg(feature = "test-support")]
pub(super) async fn claim_in_tx(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    tenant: question_model::TenantId,
    command: crate::ClaimRehearsalSubmissionCommand,
) -> Result<crate::RehearsalSubmissionClaimResult, StoreError> {
    // Generic test-support authority cannot prove a browser presentation.
    // Product callers use `claim_from_route_in_tx`, whose Store-only witness
    // is rechecked by the SQL broker before it writes (ASVS 2.3.1, 15.4.2).
    let frozen = {
        let witness = auth::prepare_operation(tx, tenant, command.locator).await?;
        let aggregate =
            hydration::load_authorized(tx, tenant, command.locator, &witness.source()).await?;
        aggregate
            .frozen(command.attempt)
            .cloned()
            .ok_or(StoreError::NotFound)?
    };
    let request = RehearsalValidatedSubmissionRequest::try_from_frozen_attempt(
        &frozen,
        command.attempt,
        command.response.clone(),
    )
    .map_err(invalid)?;
    claim_in_tx_with_input(
        tx,
        tenant,
        command,
        RehearsalClaimSubmissionInput::durable(request),
    )
    .await
}

pub(super) async fn claim_from_route_in_tx(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    tenant: question_model::TenantId,
    command: crate::ClaimRehearsalSubmissionCommand,
    screen: question_model::RehearsalActiveScreenV1,
) -> Result<crate::RehearsalSubmissionClaimResult, StoreError> {
    let rendered = question_model::ValidatedRehearsalRenderedSubmissionV1::try_from_active_screen(
        question_model::RehearsalSubmissionRequestV1 {
            presentation_digest: screen.presentation_digest.clone(),
            response: command.response.clone(),
        },
        &screen,
    )
    .map_err(invalid)?;
    claim_in_tx_with_input(
        tx,
        tenant,
        command,
        RehearsalClaimSubmissionInput::rendered(rendered),
    )
    .await
}

async fn claim_in_tx_with_input(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    tenant: question_model::TenantId,
    command: crate::ClaimRehearsalSubmissionCommand,
    submission_input: RehearsalClaimSubmissionInput,
) -> Result<crate::RehearsalSubmissionClaimResult, StoreError> {
    let witness = auth::prepare_operation(tx, tenant, command.locator).await?;
    let source = witness.source();
    let aggregate = hydration::load_authorized(tx, tenant, command.locator, &source).await?;
    if aggregate.run.id != witness.run {
        return Err(StoreError::NotFound);
    }
    integrity::require_active(&aggregate.run)?;
    let frozen = aggregate
        .frozen(command.attempt)
        .ok_or(StoreError::NotFound)?;
    let fingerprint = rehearsal_claim_submission_input_fingerprint(
        aggregate.genesis(),
        frozen,
        &submission_input,
    )
    .map_err(invalid)?;
    let operation = RehearsalGradeOperationId::from_uuid(fresh("operation")?);
    let claim_id = RehearsalSubmissionClaimId::from_uuid(fresh("claim")?);
    let provisional = RehearsalClaimRoot::verify_persisted(
        aggregate.genesis(),
        frozen,
        domain::RehearsalPersistedClaimRoot::from_persisted(
            aggregate.run.id,
            claim_id,
            fingerprint,
            submission_input,
        ),
    )
    .map_err(invalid)?;
    let decision = decide_submission_claim(
        aggregate.run.receipt.lifecycle,
        true,
        aggregate
            .claim_by_key(command.idempotency_key.as_str())
            .map(|claim| &claim.snapshot),
        fingerprint,
        &provisional,
        operation,
    );
    match decision {
        RehearsalSubmissionClaimDecision::Replay { receipt } => {
            let stored = aggregate
                .claim_by_key(command.idempotency_key.as_str())
                .and_then(|claim| claim.outcome.as_ref())
                .ok_or_else(|| {
                    StoreError::InvalidRecord(
                        "completed rehearsal claim has no immutable receipt".into(),
                    )
                })?;
            if stored != &receipt {
                return Err(StoreError::InvalidRecord(
                    "rehearsal receipt proof mismatch".into(),
                ));
            }
            Ok(crate::RehearsalSubmissionClaimResult::Replay(
                crate::RehearsalSubmissionReceipt {
                    outcome: receipt,
                    replayed: true,
                },
            ))
        }
        RehearsalSubmissionClaimDecision::Pending => {
            Ok(crate::RehearsalSubmissionClaimResult::Pending)
        }
        RehearsalSubmissionClaimDecision::Conflict
        | RehearsalSubmissionClaimDecision::ReclaimRefused(_) => {
            Ok(crate::RehearsalSubmissionClaimResult::Conflict)
        }
        RehearsalSubmissionClaimDecision::StaleRevision
        | RehearsalSubmissionClaimDecision::TerminalLifecycle => Err(StoreError::Conflict),
        RehearsalSubmissionClaimDecision::New { handle } => {
            let stored = sqlx::query_scalar::<_, bool>(
                "SELECT ple_rehearsal_route_create_claim($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12)",
            )
            .bind(tenant.as_uuid())
            .bind(command.locator.actor.as_uuid())
            .bind(command.locator.course.as_uuid())
            .bind(source.assignment.as_uuid())
            .bind(revision(command.locator.revision)?)
            .bind(aggregate.run.id.as_uuid())
            .bind(claim_id.as_uuid())
            .bind(handle.operation().as_uuid())
            .bind(command.idempotency_key.as_str())
            // Route preparation accepts this witness only after proving the
            // sole issued screen. The write broker independently compares it
            // to immutable receipt evidence before inserting a claim (ASVS
            // 2.3.1, 8.3.1, 15.4.2).
            .bind(
                provisional
                    .submission_input()
                    .presentation_commitment()
                    .map(|value| value.as_bytes().to_vec())
                    .unwrap_or_else(|| vec![0; 32]),
            )
            .bind(fingerprint.as_bytes().to_vec())
            .bind(
                domain::rehearsal::persistence::encode_claim_submission_input(
                    provisional.submission_input(),
                ),
            )
            .fetch_one(&mut **tx)
            .await
            .map_err(map_sqlx_error)?;
            if !stored {
                return Err(StoreError::Conflict);
            }
            let after = hydration::load_authorized(tx, tenant, command.locator, &source).await?;
            let claim = take_claim(after, claim_id)?;
            let hydrated = claim
                .snapshot
                .into_prepared_handle()
                .map_err(|_| StoreError::Conflict)?;
            if !same_prepared(&hydrated, &handle) {
                return Err(StoreError::InvalidRecord(
                    "rehearsal prepared handle mismatch".into(),
                ));
            }
            Ok(crate::RehearsalSubmissionClaimResult::Claimed(
                crate::ClaimedRehearsalSubmission { handle: hydrated },
            ))
        }
        RehearsalSubmissionClaimDecision::Reclaimed { handle } => {
            let existing = aggregate
                .claim_by_key(command.idempotency_key.as_str())
                .ok_or(StoreError::NotFound)?;
            let append = ClaimEventAppend {
                tenant,
                locator: command.locator,
                assignment: source.assignment,
                run: aggregate.run.id,
                claim: existing.root.claim(),
                operation: handle.operation(),
                phase: "prepared",
                reason: None,
            };
            if !append_event(tx, &append).await? {
                return Err(StoreError::Conflict);
            }
            let after = hydration::load_authorized(tx, tenant, command.locator, &source).await?;
            let claim = take_claim(after, existing.root.claim())?;
            let hydrated = claim
                .snapshot
                .into_prepared_handle()
                .map_err(|_| StoreError::Conflict)?;
            if !same_prepared(&hydrated, &handle) {
                return Err(StoreError::InvalidRecord(
                    "rehearsal reclaimed handle mismatch".into(),
                ));
            }
            Ok(crate::RehearsalSubmissionClaimResult::Claimed(
                crate::ClaimedRehearsalSubmission { handle: hydrated },
            ))
        }
    }
}

pub(super) async fn mark_dispatched(
    store: &PostgresStore,
    context: TenantContext,
    command: crate::MarkRehearsalSubmissionDispatchedCommand,
) -> Result<DispatchedClaimHandle, StoreError> {
    let tenant = context.tenant_id();
    let mut tx = store.begin_tenant(context).await?;
    let result = mark_dispatched_in_tx(&mut tx, tenant, command).await?;
    tx.commit().await.map_err(map_sqlx_error)?;
    Ok(result)
}

pub(super) async fn mark_dispatched_in_tx(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    tenant: question_model::TenantId,
    command: crate::MarkRehearsalSubmissionDispatchedCommand,
) -> Result<DispatchedClaimHandle, StoreError> {
    let witness = auth::prepare_operation(tx, tenant, command.locator).await?;
    let source = witness.source();
    let aggregate = hydration::load_authorized(tx, tenant, command.locator, &source).await?;
    if aggregate.run.id != witness.run {
        return Err(StoreError::NotFound);
    }
    integrity::require_active(&aggregate.run)?;
    let claim = aggregate
        .claim(command.handle.claim())
        .ok_or(StoreError::NotFound)?;
    if !prepared_matches(&claim.snapshot, &command.handle) {
        return Err(StoreError::NotFound);
    }
    let append = ClaimEventAppend {
        tenant,
        locator: command.locator,
        assignment: source.assignment,
        run: aggregate.run.id,
        claim: command.handle.claim(),
        operation: command.handle.operation(),
        phase: "gradingDispatched",
        reason: None,
    };
    if !append_event(tx, &append).await? {
        return Err(StoreError::Conflict);
    }
    let after = hydration::load_authorized(tx, tenant, command.locator, &source).await?;
    let hydrated = take_claim(after, command.handle.claim())?
        .snapshot
        .into_dispatched_handle()
        .map_err(|_| StoreError::Conflict)?;
    if !same_dispatched(
        &hydrated,
        &mark_rehearsal_submission_dispatched(command.handle),
    ) {
        return Err(StoreError::InvalidRecord(
            "rehearsal dispatched handle mismatch".into(),
        ));
    }
    Ok(hydrated)
}

pub(super) async fn abandon(
    store: &PostgresStore,
    context: TenantContext,
    command: crate::AbandonRehearsalSubmissionBeforeDispatchCommand,
) -> Result<(), StoreError> {
    let tenant = context.tenant_id();
    let mut tx = store.begin_tenant(context).await?;
    abandon_in_tx(&mut tx, tenant, command).await?;
    tx.commit().await.map_err(map_sqlx_error)?;
    Ok(())
}

pub(super) async fn abandon_in_tx(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    tenant: question_model::TenantId,
    command: crate::AbandonRehearsalSubmissionBeforeDispatchCommand,
) -> Result<(), StoreError> {
    let witness = auth::prepare_operation(tx, tenant, command.locator).await?;
    let source = witness.source();
    let aggregate = hydration::load_authorized(tx, tenant, command.locator, &source).await?;
    if aggregate.run.id != witness.run {
        return Err(StoreError::NotFound);
    }
    integrity::require_active(&aggregate.run)?;
    let claim = aggregate
        .claim(command.handle.claim())
        .ok_or(StoreError::NotFound)?;
    if !prepared_matches(&claim.snapshot, &command.handle) {
        return Err(StoreError::NotFound);
    }
    let reason = match command.reason {
        domain::RehearsalPreDispatchAbandonReason::LocalPreparationFailed => {
            "localPreparationFailed"
        }
        domain::RehearsalPreDispatchAbandonReason::NativeBackendAdmissionRejected => {
            "nativeBackendAdmissionRejected"
        }
        domain::RehearsalPreDispatchAbandonReason::TrustedRendererAdmissionRejected => {
            "trustedRendererAdmissionRejected"
        }
    };
    let append = ClaimEventAppend {
        tenant,
        locator: command.locator,
        assignment: source.assignment,
        run: aggregate.run.id,
        claim: command.handle.claim(),
        operation: command.handle.operation(),
        phase: "abandonedBeforeDispatch",
        reason: Some(reason),
    };
    if !append_event(tx, &append).await? {
        return Err(StoreError::Conflict);
    }
    let after = hydration::load_authorized(tx, tenant, command.locator, &source).await?;
    if after
        .claim(command.handle.claim())
        .ok_or(StoreError::NotFound)?
        .snapshot
        .state()
        != domain::RehearsalSubmissionClaimState::AbandonedBeforeDispatch
    {
        return Err(StoreError::InvalidRecord(
            "rehearsal abandonment did not persist".into(),
        ));
    }
    Ok(())
}

/// One fully bound broker transition. Constructing this after the aggregate
/// lock keeps the authorization, source identity, and immutable operation
/// witness together at the persistence boundary.
struct ClaimEventAppend<'a> {
    tenant: question_model::TenantId,
    locator: crate::RehearsalLocator,
    assignment: question_model::AssignmentId,
    run: question_model::RehearsalRunId,
    claim: RehearsalSubmissionClaimId,
    operation: RehearsalGradeOperationId,
    phase: &'a str,
    reason: Option<&'a str>,
}

async fn append_event(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    command: &ClaimEventAppend<'_>,
) -> Result<bool, StoreError> {
    sqlx::query_scalar(
        "SELECT ple_rehearsal_route_append_claim_event($1,$2,$3,$4,$5,$6,$7,$8,$9,$10)",
    )
    .bind(command.tenant.as_uuid())
    .bind(command.locator.actor.as_uuid())
    .bind(command.locator.course.as_uuid())
    .bind(command.assignment.as_uuid())
    .bind(revision(command.locator.revision)?)
    .bind(command.run.as_uuid())
    .bind(command.claim.as_uuid())
    .bind(command.operation.as_uuid())
    .bind(command.phase)
    .bind(command.reason)
    .fetch_one(&mut **tx)
    .await
    .map_err(map_sqlx_error)
}
fn take_claim(
    mut aggregate: hydration::HydratedRehearsal,
    id: RehearsalSubmissionClaimId,
) -> Result<hydration::HydratedClaim, StoreError> {
    let index = aggregate
        .claims
        .iter()
        .position(|claim| claim.root.claim() == id)
        .ok_or(StoreError::NotFound)?;
    Ok(aggregate.claims.swap_remove(index))
}
fn prepared_matches(
    snapshot: &domain::RehearsalSubmissionClaimSnapshot,
    handle: &domain::PreparedClaimHandle,
) -> bool {
    snapshot.claim() == handle.claim()
        && snapshot.operation() == handle.operation()
        && snapshot.generation() == handle.generation()
        && snapshot.state() == domain::RehearsalSubmissionClaimState::Prepared
}
fn same_prepared(left: &domain::PreparedClaimHandle, right: &domain::PreparedClaimHandle) -> bool {
    left.claim() == right.claim()
        && left.operation() == right.operation()
        && left.generation() == right.generation()
        && left.rehearsal() == right.rehearsal()
}
fn same_dispatched(left: &DispatchedClaimHandle, right: &DispatchedClaimHandle) -> bool {
    left.claim() == right.claim()
        && left.operation() == right.operation()
        && left.generation() == right.generation()
        && left.rehearsal() == right.rehearsal()
}
fn revision(value: question_model::TeachingOperationRevision) -> Result<i64, StoreError> {
    i64::try_from(value.value())
        .map_err(|_| StoreError::InvalidRecord("teaching revision exceeds database range".into()))
}
fn fresh(kind: &str) -> Result<uuid::Uuid, StoreError> {
    crate::random_uuid::random_uuid_v4(|error| {
        StoreError::Unavailable(format!("rehearsal {kind} randomness unavailable: {error}"))
    })
}
fn invalid(_error: impl std::fmt::Debug) -> StoreError {
    StoreError::InvalidRecord("invalid rehearsal submission".into())
}

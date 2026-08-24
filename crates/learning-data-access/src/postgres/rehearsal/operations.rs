//! Execute-only adapters for migration 1821's rehearsal operation protocol.

use domain::rehearsal::persistence::restore_subject_fingerprint;
use domain::rehearsal::{
    RehearsalTimingDispatchDecisionV1, RehearsalTimingInputsV1, decide_rehearsal_timing_dispatch,
};
use question_model::{ActivityTimestamp, RehearsalEvidenceDigest, run_policy::TimingPolicy};
use sqlx::Row;

use super::super::*;

fn revision(value: question_model::TeachingOperationRevision) -> Result<i64, StoreError> {
    i64::try_from(value.value())
        .map_err(|_| StoreError::InvalidRecord("teaching revision exceeds database range".into()))
}

fn projection(
    value: serde_json::Value,
    limit: usize,
) -> Result<crate::RehearsalSafeProjection, StoreError> {
    crate::RehearsalSafeProjection::new(value, limit)
}

pub(super) fn active_screen(
    value: serde_json::Value,
    persisted_digest: Vec<u8>,
) -> Result<question_model::RehearsalActiveScreenV1, StoreError> {
    active_screen_with_commitment(value, persisted_digest).map(|(screen, _)| screen)
}

pub(super) fn active_screen_with_commitment(
    value: serde_json::Value,
    persisted_digest: Vec<u8>,
) -> Result<
    (
        question_model::RehearsalActiveScreenV1,
        question_model::RehearsalPresentationDigestV1,
    ),
    StoreError,
> {
    if !value.is_object()
        || serde_json::to_vec(&value)
            .map_err(|error| {
                StoreError::InvalidRecord(format!("invalid rehearsal screen JSON: {error}"))
            })?
            .len()
            > crate::MAX_REHEARSAL_OPERATION_SCREEN_BYTES
    {
        return Err(StoreError::InvalidRecord(
            "invalid or oversized rehearsal screen".into(),
        ));
    }
    let screen: question_model::RehearsalActiveScreenV1 = serde_json::from_value(value)
        .map_err(|error| StoreError::InvalidRecord(format!("invalid rehearsal screen: {error}")))?;
    let commitment = screen.commitment().map_err(|error| {
        StoreError::InvalidRecord(format!("invalid rehearsal screen commitment: {error:?}"))
    })?;
    let stored: [u8; 32] = persisted_digest
        .try_into()
        .map_err(|_| StoreError::InvalidRecord("invalid rehearsal screen digest".into()))?;
    if commitment.as_bytes() != stored {
        return Err(StoreError::InvalidRecord(
            "rehearsal screen commitment mismatch".into(),
        ));
    }
    Ok((screen, commitment))
}

fn digest(value: Vec<u8>) -> Result<crate::RehearsalOperationDigest, StoreError> {
    let bytes: [u8; 32] = value
        .try_into()
        .map_err(|_| StoreError::InvalidRecord("invalid rehearsal operation digest".into()))?;
    Ok(crate::RehearsalOperationDigest::from_bytes(bytes))
}

fn locator_args(
    locator: crate::RehearsalLocator,
) -> Result<(uuid::Uuid, uuid::Uuid, i32, i64, i64), StoreError> {
    Ok((
        locator.actor.as_uuid(),
        locator.course.as_uuid(),
        i32::try_from(locator.assignment.number()).map_err(|_| {
            StoreError::InvalidRecord("assignment reference exceeds database range".into())
        })?,
        revision(locator.revision)?,
        i64::from(locator.rehearsal.number()),
    ))
}

pub(super) async fn mark_dispatched_in_tx(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    tenant: question_model::TenantId,
    prepared: crate::PreparedRehearsalDelivery,
) -> Result<crate::RehearsalDeliveryDispatchResult, StoreError> {
    let locator = prepared.locator();
    let operation = prepared.operation();
    let (actor, course, assignment, revision, rehearsal) = locator_args(locator)?;
    let row =
        sqlx::query("SELECT * FROM ple_prepare_rehearsal_timing_dispatch($1,$2,$3,$4,$5,$6,$7)")
            .bind(tenant.as_uuid())
            .bind(actor)
            .bind(course)
            .bind(assignment)
            .bind(revision)
            .bind(rehearsal)
            .bind(operation.as_uuid())
            .fetch_optional(&mut **tx)
            .await
            .map_err(map_sqlx_error)?
            .ok_or(StoreError::Conflict)?;
    let timing_kind: String = row.try_get("timing_kind").map_err(map_sqlx_error)?;
    let seconds: Option<i32> = row.try_get("question_seconds").map_err(map_sqlx_error)?;
    let grace: Option<i32> = row
        .try_get("question_grace_seconds")
        .map_err(map_sqlx_error)?;
    let timing_policy = match timing_kind.as_str() {
        "untimed" => TimingPolicy::Untimed,
        "perQuestion" => TimingPolicy::PerQuestion {
            seconds: u32::try_from(seconds.ok_or(StoreError::Unavailable(
                "missing frozen question time limit".into(),
            ))?)
            .map_err(|_| StoreError::Unavailable("invalid frozen question time limit".into()))?,
            grace_seconds: u32::try_from(grace.ok_or(StoreError::Unavailable(
                "missing frozen question grace".into(),
            ))?)
            .map_err(|_| StoreError::Unavailable("invalid frozen question grace".into()))?,
        },
        "perAttempt" => TimingPolicy::PerAttempt {
            seconds: u32::try_from(seconds.ok_or(StoreError::Unavailable(
                "missing frozen attempt time limit".into(),
            ))?)
            .map_err(|_| StoreError::Unavailable("invalid frozen attempt time limit".into()))?,
            grace_seconds: u32::try_from(grace.ok_or(StoreError::Unavailable(
                "missing frozen attempt grace".into(),
            ))?)
            .map_err(|_| StoreError::Unavailable("invalid frozen attempt grace".into()))?,
        },
        _ => {
            return Err(StoreError::Unavailable(
                "invalid frozen rehearsal timing policy".into(),
            ));
        }
    };
    let fingerprint: Vec<u8> = row.try_get("subject_fingerprint").map_err(map_sqlx_error)?;
    let snapshot: Vec<u8> = row
        .try_get("issued_snapshot_sha256")
        .map_err(map_sqlx_error)?;
    let timing = decide_rehearsal_timing_dispatch(RehearsalTimingInputsV1 {
        subject_fingerprint: restore_subject_fingerprint(&fingerprint).map_err(|_| {
            StoreError::Unavailable("invalid frozen rehearsal subject fingerprint".into())
        })?,
        frozen_snapshot_digest: RehearsalEvidenceDigest::from_bytes(snapshot.try_into().map_err(
            |_| StoreError::Unavailable("invalid frozen rehearsal snapshot digest".into()),
        )?),
        timing_policy,
        subject_time_limit_seconds: row
            .try_get::<Option<i32>, _>("subject_limit_seconds")
            .map_err(map_sqlx_error)?
            .map(|value| {
                u32::try_from(value)
                    .map_err(|_| StoreError::Unavailable("invalid resolved subject limit".into()))
            })
            .transpose()?,
        run_started_at: ActivityTimestamp::from_unix_millis(
            row.try_get("run_started_at_millis")
                .map_err(map_sqlx_error)?,
        ),
        issued_at: ActivityTimestamp::from_unix_millis(
            row.try_get("issued_at_millis").map_err(map_sqlx_error)?,
        ),
    })
    .map_err(|_| StoreError::Unavailable("invalid frozen rehearsal timing witness".into()))?;
    let witness = match timing {
        RehearsalTimingDispatchDecisionV1::Witness(witness) => witness,
        RehearsalTimingDispatchDecisionV1::RunTimeExhausted { deadline } => {
            let exhausted = sqlx::query(
                "SELECT deadline_at_millis FROM ple_finalize_rehearsal_timing_exhausted($1,$2)",
            )
            .bind(tenant.as_uuid())
            .bind(operation.as_uuid())
            .fetch_optional(&mut **tx)
            .await
            .map_err(map_sqlx_error)?
            .ok_or(StoreError::Conflict)?;
            let persisted_deadline: i64 = exhausted
                .try_get("deadline_at_millis")
                .map_err(map_sqlx_error)?;
            if persisted_deadline != deadline.as_unix_millis() {
                return Err(StoreError::Conflict);
            }
            return Ok(crate::RehearsalDeliveryDispatchResult::RunTimeExhausted { deadline });
        }
    };
    let deadline = witness.deadline().map(|value| value.as_unix_millis());
    let expires = match (witness.deadline(), witness.grace_millis()) {
        (Some(deadline), Some(grace)) => {
            deadline
                .as_unix_millis()
                .checked_add(grace)
                .ok_or(StoreError::Unavailable(
                    "rehearsal timing expiration overflow".into(),
                ))?
        }
        _ => 0,
    };
    let source = match witness.deadline_source() {
        None => None,
        Some(domain::RehearsalDeadlineSourceV1::PerQuestion) => Some("perQuestion"),
        Some(domain::RehearsalDeadlineSourceV1::PerAttempt) => Some("perAttempt"),
        Some(domain::RehearsalDeadlineSourceV1::SubjectLimit) => Some("subjectLimit"),
    };
    let marked: bool = sqlx::query_scalar(
        "SELECT ple_finalize_rehearsal_timing_dispatch($1,$2,$3,$4,$5,$6,$7,$8,$9)",
    )
    .bind(tenant.as_uuid())
    .bind(operation.as_uuid())
    .bind(witness.issued_at().as_unix_millis())
    .bind(deadline)
    .bind(witness.grace_millis())
    .bind(if deadline.is_some() {
        Some(expires)
    } else {
        None
    })
    .bind(source)
    .bind(witness.canonical_bytes().to_vec())
    .bind(witness.commitment().as_bytes().to_vec())
    .fetch_one(&mut **tx)
    .await
    .map_err(map_sqlx_error)?;
    if !marked {
        return Err(StoreError::Conflict);
    }
    Ok(crate::RehearsalDeliveryDispatchResult::Dispatched {
        dispatched: crate::DispatchedRehearsalDelivery::mint(locator, operation),
    })
}

pub(super) async fn complete_delivery_in_tx(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    tenant: question_model::TenantId,
    command: crate::RehearsalDeliveryCompletionCommand,
) -> Result<question_model::RehearsalActiveScreenV1, StoreError> {
    let locator = command.dispatched.locator();
    let operation = command.dispatched.operation();
    let (actor, course, assignment, revision, rehearsal) = locator_args(locator)?;
    let screen_digest = command.screen.commitment().map_err(|error| {
        StoreError::InvalidRecord(format!("invalid rehearsal active screen: {error:?}"))
    })?;
    let screen = serde_json::to_value(&command.screen).map_err(|error| {
        StoreError::InvalidRecord(format!("invalid rehearsal active screen JSON: {error}"))
    })?;
    if !screen.is_object()
        || serde_json::to_vec(&screen)
            .map_err(|error| {
                StoreError::InvalidRecord(format!("invalid rehearsal active screen JSON: {error}"))
            })?
            .len()
            > crate::MAX_REHEARSAL_OPERATION_SCREEN_BYTES
    {
        return Err(StoreError::InvalidRecord(
            "invalid or oversized rehearsal active screen".into(),
        ));
    }
    let row =
        sqlx::query("SELECT * FROM ple_rehearsal_complete_delivery($1,$2,$3,$4,$5,$6,$7,$8,$9)")
            .bind(tenant.as_uuid())
            .bind(actor)
            .bind(course)
            .bind(assignment)
            .bind(revision)
            .bind(rehearsal)
            .bind(operation.as_uuid())
            .bind(screen)
            .bind(screen_digest.as_bytes().to_vec())
            .fetch_optional(&mut **tx)
            .await
            .map_err(map_sqlx_error)?
            .ok_or(StoreError::Conflict)?;
    active_screen(
        row.try_get("screen_projection").map_err(map_sqlx_error)?,
        row.try_get("screen_digest").map_err(map_sqlx_error)?,
    )
}

pub(super) async fn abandon_delivery_in_tx(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    tenant: question_model::TenantId,
    prepared: crate::PreparedRehearsalDelivery,
    reason: crate::RehearsalDeliveryPreDispatchAbandonReason,
) -> Result<(), StoreError> {
    let locator = prepared.locator();
    let operation = prepared.operation();
    let (actor, course, assignment, revision, rehearsal) = locator_args(locator)?;
    let reason = match reason {
        crate::RehearsalDeliveryPreDispatchAbandonReason::LocalPreparationFailed => {
            "localPreparationFailed"
        }
        crate::RehearsalDeliveryPreDispatchAbandonReason::NativeBackendAdmissionRejected => {
            "nativeBackendAdmissionRejected"
        }
        crate::RehearsalDeliveryPreDispatchAbandonReason::TrustedRendererAdmissionRejected => {
            "trustedRendererAdmissionRejected"
        }
    };
    let abandoned: bool = sqlx::query_scalar(
        "SELECT ple_rehearsal_abandon_delivery_before_dispatch($1,$2,$3,$4,$5,$6,$7,$8)",
    )
    .bind(tenant.as_uuid())
    .bind(actor)
    .bind(course)
    .bind(assignment)
    .bind(revision)
    .bind(rehearsal)
    .bind(operation.as_uuid())
    .bind(reason)
    .fetch_one(&mut **tx)
    .await
    .map_err(map_sqlx_error)?;
    if !abandoned {
        return Err(StoreError::Conflict);
    }
    Ok(())
}

pub(super) async fn discard_in_tx(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    tenant: question_model::TenantId,
    command: crate::RehearsalDiscardOperationCommand,
) -> Result<crate::RehearsalIdempotentProjectionResult, StoreError> {
    let (actor, course, assignment, revision, rehearsal) = locator_args(command.locator)?;
    let row = sqlx::query(
        "SELECT * FROM ple_prepare_rehearsal_discard_idempotent($1,$2,$3,$4,$5,$6,$7,$8)",
    )
    .bind(tenant.as_uuid())
    .bind(actor)
    .bind(course)
    .bind(assignment)
    .bind(revision)
    .bind(rehearsal)
    .bind(command.idempotency_key.as_str())
    .bind(command.request_fingerprint.as_bytes().to_vec())
    .fetch_optional(&mut **tx)
    .await
    .map_err(map_sqlx_error)?
    .ok_or(StoreError::Conflict)?;
    let kind: String = row.try_get("result_kind").map_err(map_sqlx_error)?;
    if kind == "conflict" {
        return Ok(crate::RehearsalIdempotentProjectionResult::Conflict);
    }
    if kind == "replay" {
        return Ok(crate::RehearsalIdempotentProjectionResult::Replay(
            projection(
                row.try_get("response_projection").map_err(map_sqlx_error)?,
                crate::MAX_REHEARSAL_OPERATION_RESPONSE_BYTES,
            )?,
        ));
    }
    let operation: uuid::Uuid = row.try_get("operation_id").map_err(map_sqlx_error)?;
    let nonce: uuid::Uuid = row.try_get("prepare_nonce").map_err(map_sqlx_error)?;
    let witness: Vec<u8> = row
        .try_get("structural_witness_digest")
        .map_err(map_sqlx_error)?;
    let completed =
        sqlx::query("SELECT * FROM ple_complete_rehearsal_discard_idempotent($1,$2,$3,$4,$5,$6)")
            .bind(tenant.as_uuid())
            .bind(operation)
            .bind(nonce)
            .bind(witness)
            .bind(command.response.as_value())
            .bind(command.response_digest.as_bytes().to_vec())
            .fetch_optional(&mut **tx)
            .await
            .map_err(map_sqlx_error)?
            .ok_or(StoreError::Conflict)?;
    Ok(crate::RehearsalIdempotentProjectionResult::Applied(
        projection(
            completed
                .try_get("response_projection")
                .map_err(map_sqlx_error)?,
            crate::MAX_REHEARSAL_OPERATION_RESPONSE_BYTES,
        )?,
    ))
}

/// Claims a delivery while preserving the caller's transaction boundary.
///
/// Route-shaped entry points resolve the public route identity before calling
/// this helper, so authorization, idempotency preparation, and admission all
/// observe one locked transaction. The ordinary store entry point uses the
/// same helper and owns its transaction commit.
pub(super) async fn claim_delivery_in_tx(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    tenant: question_model::TenantId,
    request: crate::RehearsalDeliveryRequest,
) -> Result<crate::RehearsalDeliveryClaimResult, StoreError> {
    let (actor, course, assignment, revision, rehearsal) = locator_args(request.locator)?;
    let prepared =
        sqlx::query("SELECT * FROM ple_prepare_rehearsal_delivery($1,$2,$3,$4,$5,$6,$7,$8)")
            .bind(tenant.as_uuid())
            .bind(actor)
            .bind(course)
            .bind(assignment)
            .bind(revision)
            .bind(rehearsal)
            .bind(request.idempotency_key.as_str())
            .bind(request.request_fingerprint.as_bytes().to_vec())
            .fetch_optional(&mut **tx)
            .await
            .map_err(map_sqlx_error)?
            .ok_or(StoreError::Conflict)?;
    let kind: String = prepared.try_get("result_kind").map_err(map_sqlx_error)?;
    match kind.as_str() {
        "replay" => {
            let screen = active_screen(
                prepared
                    .try_get("screen_projection")
                    .map_err(map_sqlx_error)?,
                prepared.try_get("screen_digest").map_err(map_sqlx_error)?,
            )?;
            return Ok(crate::RehearsalDeliveryClaimResult::Replay(screen));
        }
        "pending" => {
            let operation = crate::RehearsalOperationId::from_uuid(
                prepared.try_get("operation_id").map_err(map_sqlx_error)?,
            );
            return Ok(crate::RehearsalDeliveryClaimResult::Pending {
                dispatched: crate::DispatchedRehearsalDelivery::mint(request.locator, operation),
            });
        }
        "claimed" => {
            let operation = crate::RehearsalOperationId::from_uuid(
                prepared.try_get("operation_id").map_err(map_sqlx_error)?,
            );
            let descriptor = crate::RehearsalDeliveryExecutionDescriptorV1::decode_persisted(
                &prepared
                    .try_get("execution_descriptor")
                    .map_err(map_sqlx_error)?,
            )?;
            return Ok(crate::RehearsalDeliveryClaimResult::Prepared {
                prepared: crate::PreparedRehearsalDelivery::mint(
                    request.locator,
                    operation,
                    descriptor,
                ),
            });
        }
        "expired" => return Ok(crate::RehearsalDeliveryClaimResult::Expired),
        "runTimeExhausted" => {
            return Ok(crate::RehearsalDeliveryClaimResult::RunTimeExhausted {
                deadline: ActivityTimestamp::from_unix_millis(
                    prepared
                        .try_get("deadline_at_millis")
                        .map_err(map_sqlx_error)?,
                ),
            });
        }
        "conflict" => return Ok(crate::RehearsalDeliveryClaimResult::Conflict),
        "admit" => {}
        _ => {
            return Err(StoreError::InvalidRecord(
                "invalid rehearsal delivery prepare result".into(),
            ));
        }
    }
    let nonce = crate::RehearsalOperationNonce::from_uuid(
        prepared.try_get("prepare_nonce").map_err(map_sqlx_error)?,
    );
    let admission_digest = digest(
        prepared
            .try_get("admission_digest")
            .map_err(map_sqlx_error)?,
    )?;
    let witness = super::auth::prepare_operation(tx, tenant, request.locator).await?;
    let source = witness.source();
    let aggregate = super::hydration::load_authorized(tx, tenant, request.locator, &source).await?;
    if aggregate.run.id != witness.run {
        return Err(StoreError::NotFound);
    }
    super::integrity::require_active(&aggregate.run)?;
    // The Store-owned broker derives the lowest unresolved frozen ordinal and
    // mints the issue operation.  Rust supplies only the already authenticated
    // route idempotency witness; it never selects an attempt or plan.
    let claimed = sqlx::query(
        "SELECT * FROM ple_claim_prepared_rehearsal_delivery($1,$2,$3,$4,$5,$6,$7,$8,$9,$10)",
    )
    .bind(tenant.as_uuid())
    .bind(actor)
    .bind(course)
    .bind(assignment)
    .bind(revision)
    .bind(rehearsal)
    .bind(request.idempotency_key.as_str())
    .bind(request.request_fingerprint.as_bytes().to_vec())
    .bind(nonce.as_uuid())
    .bind(admission_digest.as_bytes().to_vec())
    .fetch_optional(&mut **tx)
    .await
    .map_err(map_sqlx_error)?
    .ok_or(StoreError::Conflict)?;
    let claimed_kind: String = claimed.try_get("result_kind").map_err(map_sqlx_error)?;
    match claimed_kind.as_str() {
        "claimed" => {
            let operation = crate::RehearsalOperationId::from_uuid(
                claimed.try_get("operation_id").map_err(map_sqlx_error)?,
            );
            let descriptor = crate::RehearsalDeliveryExecutionDescriptorV1::decode_persisted(
                &claimed
                    .try_get("execution_descriptor")
                    .map_err(map_sqlx_error)?,
            )?;
            Ok(crate::RehearsalDeliveryClaimResult::Prepared {
                prepared: crate::PreparedRehearsalDelivery::mint(
                    request.locator,
                    operation,
                    descriptor,
                ),
            })
        }
        "replay" => Ok(crate::RehearsalDeliveryClaimResult::Replay(active_screen(
            claimed
                .try_get("screen_projection")
                .map_err(map_sqlx_error)?,
            claimed.try_get("screen_digest").map_err(map_sqlx_error)?,
        )?)),
        "pending" => Ok(crate::RehearsalDeliveryClaimResult::Pending {
            dispatched: crate::DispatchedRehearsalDelivery::mint(
                request.locator,
                crate::RehearsalOperationId::from_uuid(
                    claimed.try_get("operation_id").map_err(map_sqlx_error)?,
                ),
            ),
        }),
        _ => Err(StoreError::Conflict),
    }
}

#[async_trait::async_trait]
impl crate::RehearsalOperationStore for PostgresStore {
    async fn discard_rehearsal_idempotent(
        &self,
        context: TenantContext,
        command: crate::RehearsalDiscardOperationCommand,
    ) -> Result<crate::RehearsalIdempotentProjectionResult, StoreError> {
        let tenant = context.tenant_id();
        let mut tx = self.begin_tenant(context).await?;
        let (actor, course, assignment, revision, rehearsal) = locator_args(command.locator)?;
        let row = sqlx::query(
            "SELECT * FROM ple_prepare_rehearsal_discard_idempotent($1,$2,$3,$4,$5,$6,$7,$8)",
        )
        .bind(tenant.as_uuid())
        .bind(actor)
        .bind(course)
        .bind(assignment)
        .bind(revision)
        .bind(rehearsal)
        .bind(command.idempotency_key.as_str())
        .bind(command.request_fingerprint.as_bytes().to_vec())
        .fetch_optional(&mut *tx)
        .await
        .map_err(map_sqlx_error)?
        .ok_or(StoreError::Conflict)?;
        let kind: String = row.try_get("result_kind").map_err(map_sqlx_error)?;
        if kind == "conflict" {
            return Ok(crate::RehearsalIdempotentProjectionResult::Conflict);
        }
        if kind == "replay" {
            let value = row.try_get("response_projection").map_err(map_sqlx_error)?;
            tx.commit().await.map_err(map_sqlx_error)?;
            return Ok(crate::RehearsalIdempotentProjectionResult::Replay(
                projection(value, crate::MAX_REHEARSAL_OPERATION_RESPONSE_BYTES)?,
            ));
        }
        let operation: uuid::Uuid = row.try_get("operation_id").map_err(map_sqlx_error)?;
        let nonce: uuid::Uuid = row.try_get("prepare_nonce").map_err(map_sqlx_error)?;
        let witness: Vec<u8> = row
            .try_get("structural_witness_digest")
            .map_err(map_sqlx_error)?;
        let completed = sqlx::query(
            "SELECT * FROM ple_complete_rehearsal_discard_idempotent($1,$2,$3,$4,$5,$6)",
        )
        .bind(tenant.as_uuid())
        .bind(operation)
        .bind(nonce)
        .bind(witness)
        .bind(command.response.as_value())
        .bind(command.response_digest.as_bytes().to_vec())
        .fetch_optional(&mut *tx)
        .await
        .map_err(map_sqlx_error)?
        .ok_or(StoreError::Conflict)?;
        let value = completed
            .try_get("response_projection")
            .map_err(map_sqlx_error)?;
        tx.commit().await.map_err(map_sqlx_error)?;
        Ok(crate::RehearsalIdempotentProjectionResult::Applied(
            projection(value, crate::MAX_REHEARSAL_OPERATION_RESPONSE_BYTES)?,
        ))
    }

    async fn claim_rehearsal_delivery(
        &self,
        context: TenantContext,
        request: crate::RehearsalDeliveryRequest,
    ) -> Result<crate::RehearsalDeliveryClaimResult, StoreError> {
        let tenant = context.tenant_id();
        let mut tx = self.begin_tenant(context).await?;
        let result = claim_delivery_in_tx(&mut tx, tenant, request).await?;
        tx.commit().await.map_err(map_sqlx_error)?;
        Ok(result)
    }

    async fn mark_rehearsal_delivery_dispatched(
        &self,
        context: TenantContext,
        prepared: crate::PreparedRehearsalDelivery,
    ) -> Result<crate::RehearsalDeliveryDispatchResult, StoreError> {
        let tenant = context.tenant_id();
        let mut tx = self.begin_tenant(context).await?;
        let dispatched = mark_dispatched_in_tx(&mut tx, tenant, prepared).await?;
        tx.commit().await.map_err(map_sqlx_error)?;
        Ok(dispatched)
    }

    async fn complete_rehearsal_delivery(
        &self,
        context: TenantContext,
        command: crate::RehearsalDeliveryCompletionCommand,
    ) -> Result<question_model::RehearsalActiveScreenV1, StoreError> {
        let tenant = context.tenant_id();
        let mut tx = self.begin_tenant(context).await?;
        let screen = complete_delivery_in_tx(&mut tx, tenant, command).await?;
        tx.commit().await.map_err(map_sqlx_error)?;
        Ok(screen)
    }
}

#[async_trait::async_trait]
impl crate::RehearsalDeliveryPreDispatchCompensationStore for PostgresStore {
    async fn abandon_rehearsal_delivery_before_dispatch(
        &self,
        context: TenantContext,
        prepared: crate::PreparedRehearsalDelivery,
        reason: crate::RehearsalDeliveryPreDispatchAbandonReason,
    ) -> Result<(), StoreError> {
        let tenant = context.tenant_id();
        let mut tx = self.begin_tenant(context).await?;
        let locator = prepared.locator();
        let operation = prepared.operation();
        let (actor, course, assignment, revision, rehearsal) = locator_args(locator)?;
        let reason = match reason {
            crate::RehearsalDeliveryPreDispatchAbandonReason::LocalPreparationFailed => {
                "localPreparationFailed"
            }
            crate::RehearsalDeliveryPreDispatchAbandonReason::NativeBackendAdmissionRejected => {
                "nativeBackendAdmissionRejected"
            }
            crate::RehearsalDeliveryPreDispatchAbandonReason::TrustedRendererAdmissionRejected => {
                "trustedRendererAdmissionRejected"
            }
        };
        let abandoned: bool = sqlx::query_scalar(
            "SELECT ple_rehearsal_abandon_delivery_before_dispatch($1,$2,$3,$4,$5,$6,$7,$8)",
        )
        .bind(tenant.as_uuid())
        .bind(actor)
        .bind(course)
        .bind(assignment)
        .bind(revision)
        .bind(rehearsal)
        .bind(operation.as_uuid())
        .bind(reason)
        .fetch_one(&mut *tx)
        .await
        .map_err(map_sqlx_error)?;
        if !abandoned {
            return Err(StoreError::Conflict);
        }
        tx.commit().await.map_err(map_sqlx_error)
    }
}

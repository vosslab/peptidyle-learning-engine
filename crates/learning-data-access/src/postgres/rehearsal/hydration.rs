//! Locked full-aggregate hydration and answer-free receipt projection.
//!
//! Database capabilities serialize mutations, while this module certifies the
//! private aggregate with closed decoding before an operation receives a handle.

use std::collections::BTreeMap;

use domain::{
    RehearsalClaimCompletionMaterial, RehearsalClaimRoot, RehearsalClaimTransitionEvent,
    RehearsalEvidenceChainEntry, RehearsalFrozenInventoryEntry, RehearsalGenesisContext,
    RehearsalSubmissionClaimPhase, RehearsalSubmissionClaimSnapshot, hydrate_claim_history,
    private_payload_digest, rehearsal_accepted_evidence_owner, verify_evidence_chain,
    verify_rehearsal_claim_completion_proof, verify_rehearsal_inventory,
};
use question_model::{
    ActivityTimestamp, RehearsalEvidenceDigest, RehearsalEvidenceKind, RehearsalReference,
    RehearsalSubmissionClaimId,
};
use sqlx::{Postgres, Row, Transaction};

use super::super::*;
use super::{auth, rows};

/// Private and deliberately non-serializable aggregate material.
pub(super) struct HydratedRehearsal {
    pub(super) run: rows::HydratedRun,
    pub(super) frozen: Vec<question_model::RehearsalFrozenItemEvidence>,
    pub(super) evidence: Vec<RehearsalEvidenceChainEntry>,
    pub(super) claims: Vec<HydratedClaim>,
}

pub(super) struct HydratedClaim {
    pub(super) idempotency_key: String,
    pub(super) root: RehearsalClaimRoot,
    pub(super) snapshot: RehearsalSubmissionClaimSnapshot,
    pub(super) outcome: Option<question_model::RehearsalPublicOutcome>,
}

impl HydratedRehearsal {
    pub(super) fn genesis(&self) -> RehearsalGenesisContext {
        genesis(&self.run)
    }
    pub(super) fn frozen(
        &self,
        attempt: question_model::RehearsalAttemptId,
    ) -> Option<&question_model::RehearsalFrozenItemEvidence> {
        self.frozen.iter().find(|item| item.attempt == attempt)
    }
    pub(super) fn claim(&self, claim: RehearsalSubmissionClaimId) -> Option<&HydratedClaim> {
        self.claims.iter().find(|item| item.root.claim() == claim)
    }
    pub(super) fn claim_by_key(&self, key: &str) -> Option<&HydratedClaim> {
        self.claims.iter().find(|item| item.idempotency_key == key)
    }
}

pub(super) async fn read(
    store: &PostgresStore,
    context: TenantContext,
    locator: crate::RehearsalLocator,
) -> Result<question_model::RehearsalRunReceipt, StoreError> {
    let tenant = context.tenant_id();
    let mut tx = store.begin_tenant(context).await?;
    let source = auth::lock_source(
        &mut tx,
        tenant,
        locator.actor,
        locator.course,
        locator.assignment,
        locator.revision,
    )
    .await?;
    let aggregate = load_authorized(&mut tx, tenant, locator, &source).await?;
    let receipt = aggregate.run.receipt.clone();
    tx.commit().await.map_err(map_sqlx_error)?;
    Ok(receipt)
}

pub(super) async fn load_authorized(
    tx: &mut Transaction<'_, Postgres>,
    tenant: question_model::TenantId,
    locator: crate::RehearsalLocator,
    source: &auth::LockedSource,
) -> Result<HydratedRehearsal, StoreError> {
    let public = load_locator(tx, tenant, locator.rehearsal).await?;
    require_locator(&public, tenant, locator, source)?;
    let run = load_authorized_private(tx, public).await?;
    load_complete(tx, run).await
}

async fn load_locator(
    tx: &mut Transaction<'_, Postgres>,
    tenant: question_model::TenantId,
    reference: RehearsalReference,
) -> Result<rows::RunLocator, StoreError> {
    let row = sqlx::query("SELECT tenant_id, rehearsal_run_id, rehearsal_reference, course_id, assignment_id, assignment_reference, direct_instructor_membership_id, actor_id, assignment_revision, lifecycle, (extract(epoch FROM started_at) * 1000)::bigint AS started_at_millis, (extract(epoch FROM updated_at) * 1000)::bigint AS updated_at_millis, CASE WHEN terminal_at IS NULL THEN NULL ELSE (extract(epoch FROM terminal_at) * 1000)::bigint END AS terminal_at_millis, evidence_head_digest, evidence_length FROM rehearsal_run WHERE tenant_id=$1 AND rehearsal_reference=$2")
        .bind(tenant.as_uuid()).bind(i64::from(reference.number())).fetch_optional(&mut **tx).await.map_err(map_sqlx_error)?.ok_or(StoreError::NotFound)?;
    rows::decode_locator(&row)
}

async fn load_authorized_private(
    tx: &mut Transaction<'_, Postgres>,
    locator: rows::RunLocator,
) -> Result<rows::HydratedRun, StoreError> {
    let row = sqlx::query("SELECT subject_payload, subject_fingerprint FROM rehearsal_run WHERE tenant_id=$1 AND rehearsal_run_id=$2")
        .bind(locator.tenant.as_uuid()).bind(locator.id.as_uuid()).fetch_one(&mut **tx).await.map_err(map_sqlx_error)?;
    let subject = row.try_get("subject_payload").map_err(map_sqlx_error)?;
    let fingerprint: Vec<u8> = row.try_get("subject_fingerprint").map_err(map_sqlx_error)?;
    rows::decode_authorized_run(locator, &subject, &fingerprint)
}

async fn load_complete(
    tx: &mut Transaction<'_, Postgres>,
    run: rows::HydratedRun,
) -> Result<HydratedRehearsal, StoreError> {
    let frozen = load_frozen(tx, &run).await?;
    let context = genesis(&run);
    let mut roots = load_roots(tx, &run, &frozen, context).await?;
    let evidence = load_evidence(tx, &run, &frozen, &roots).await?;
    verify_evidence_chain(context, run.head, &evidence).map_err(invalid)?;
    let claims = hydrate_claims(tx, &run, &evidence, &mut roots, context).await?;
    let owners = claims
        .iter()
        .filter(|claim| claim.snapshot.state() == domain::RehearsalSubmissionClaimState::Completed)
        .map(|claim| {
            verify_rehearsal_claim_completion_proof(context, run.head, &claim.root, &evidence)
                .map_err(invalid)
                .and_then(|proof| rehearsal_accepted_evidence_owner(proof).map_err(invalid))
        })
        .collect::<Result<Vec<_>, _>>()?;
    verify_rehearsal_inventory(
        frozen
            .iter()
            .map(|item| RehearsalFrozenInventoryEntry::new(item.attempt, item)),
        &evidence,
        owners,
    )
    .map_err(invalid)?;
    Ok(HydratedRehearsal {
        run,
        frozen,
        evidence,
        claims,
    })
}

/// Hydrate a run whose non-private locator has already been locked by a
/// source-context operation.  The caller must establish the source lock order
/// before locking the run, and must validate the closed source selector before
/// this function is called.  This deliberately does not authorize a current
/// source: source-removal workflows must be able to verify an active run just
/// before removing that source.
pub(super) async fn load_locked_source_aggregate(
    tx: &mut Transaction<'_, Postgres>,
    locator: rows::RunLocator,
) -> Result<HydratedRehearsal, StoreError> {
    let run = load_authorized_private(tx, locator).await?;
    load_complete(tx, run).await
}

async fn load_frozen(
    tx: &mut Transaction<'_, Postgres>,
    run: &rows::HydratedRun,
) -> Result<Vec<question_model::RehearsalFrozenItemEvidence>, StoreError> {
    let database_rows = sqlx::query("SELECT attempt_id, problem_id, version_id, response_definition, response_schema_digest, canonical_content_digest, (extract(epoch FROM frozen_at) * 1000)::bigint AS frozen_at_millis FROM rehearsal_frozen_item WHERE tenant_id=$1 AND rehearsal_run_id=$2 ORDER BY attempt_id").bind(run.tenant.as_uuid()).bind(run.id.as_uuid()).fetch_all(&mut **tx).await.map_err(map_sqlx_error)?;
    let mut result = Vec::with_capacity(database_rows.len());
    for row in database_rows {
        let response_definition =
            serde_json::from_value(row.try_get("response_definition").map_err(map_sqlx_error)?)
                .map_err(|_| {
                    StoreError::InvalidRecord("invalid frozen response definition".into())
                })?;
        let schema = digest(
            &row.try_get::<Vec<u8>, _>("response_schema_digest")
                .map_err(map_sqlx_error)?,
        )?;
        let item = question_model::RehearsalFrozenItemEvidence {
            attempt: question_model::RehearsalAttemptId::from_uuid(
                row.try_get("attempt_id").map_err(map_sqlx_error)?,
            ),
            problem: question_model::ProblemVersionRef {
                problem: question_model::ProblemId::from_uuid(
                    row.try_get("problem_id").map_err(map_sqlx_error)?,
                ),
                version: question_model::VersionId::from_uuid(
                    row.try_get("version_id").map_err(map_sqlx_error)?,
                ),
            },
            response_definition,
            canonical_content_digest: digest(
                &row.try_get::<Vec<u8>, _>("canonical_content_digest")
                    .map_err(map_sqlx_error)?,
            )?,
            frozen_at: ActivityTimestamp::from_unix_millis(
                row.try_get("frozen_at_millis").map_err(map_sqlx_error)?,
            ),
        };
        if domain::frozen_response_schema_digest(&item.response_definition) != schema
            || result
                .iter()
                .any(|prior: &question_model::RehearsalFrozenItemEvidence| {
                    prior.attempt == item.attempt
                })
        {
            return Err(invalid("invalid frozen rehearsal inventory"));
        }
        result.push(item);
    }
    Ok(result)
}

struct RootState {
    key: String,
    root: RehearsalClaimRoot,
}

async fn load_roots(
    tx: &mut Transaction<'_, Postgres>,
    run: &rows::HydratedRun,
    frozen: &[question_model::RehearsalFrozenItemEvidence],
    context: RehearsalGenesisContext,
) -> Result<Vec<RootState>, StoreError> {
    let database_rows = sqlx::query("SELECT claim_id, idempotency_key, attempt_id, request_fingerprint, sealed_request FROM rehearsal_submission_claim_root WHERE tenant_id=$1 AND rehearsal_run_id=$2 ORDER BY claim_id").bind(run.tenant.as_uuid()).bind(run.id.as_uuid()).fetch_all(&mut **tx).await.map_err(map_sqlx_error)?;
    let mut roots = Vec::with_capacity(database_rows.len());
    for row in database_rows {
        let attempt = question_model::RehearsalAttemptId::from_uuid(
            row.try_get("attempt_id").map_err(map_sqlx_error)?,
        );
        let frozen = frozen
            .iter()
            .find(|item| item.attempt == attempt)
            .ok_or_else(|| {
                StoreError::InvalidRecord("claim references missing frozen attempt".into())
            })?;
        let claim =
            RehearsalSubmissionClaimId::from_uuid(row.try_get("claim_id").map_err(map_sqlx_error)?);
        let bytes: Vec<u8> = row.try_get("request_fingerprint").map_err(map_sqlx_error)?;
        let sealed = row.try_get("sealed_request").map_err(map_sqlx_error)?;
        let persisted = domain::rehearsal::persistence::decode_persisted_claim_root(
            run.id, claim, &bytes, &sealed, frozen, attempt,
        )
        .map_err(invalid)?;
        let root =
            RehearsalClaimRoot::verify_persisted(context, frozen, persisted).map_err(invalid)?;
        let key: String = row.try_get("idempotency_key").map_err(map_sqlx_error)?;
        crate::RehearsalSubmissionIdempotencyKey::new(key.clone())?;
        if roots
            .iter()
            .any(|prior: &RootState| prior.key == key || prior.root.claim() == claim)
        {
            return Err(invalid("duplicate rehearsal claim root"));
        }
        roots.push(RootState { key, root });
    }
    Ok(roots)
}

async fn load_evidence(
    tx: &mut Transaction<'_, Postgres>,
    run: &rows::HydratedRun,
    frozen: &[question_model::RehearsalFrozenItemEvidence],
    roots: &[RootState],
) -> Result<Vec<RehearsalEvidenceChainEntry>, StoreError> {
    let database_rows = sqlx::query("SELECT sequence, kind, previous_digest, entry_digest, payload, payload_digest, (extract(epoch FROM recorded_at) * 1000)::bigint AS recorded_at_millis FROM rehearsal_evidence WHERE tenant_id=$1 AND rehearsal_run_id=$2 ORDER BY sequence").bind(run.tenant.as_uuid()).bind(run.id.as_uuid()).fetch_all(&mut **tx).await.map_err(map_sqlx_error)?;
    let mut entries = Vec::with_capacity(database_rows.len());
    for row in database_rows {
        let sequence = u32::try_from(row.try_get::<i64, _>("sequence").map_err(map_sqlx_error)?)
            .map_err(|_| StoreError::InvalidRecord("invalid rehearsal evidence sequence".into()))?;
        let kind: String = row.try_get("kind").map_err(map_sqlx_error)?;
        let kind = match kind.as_str() {
            "frozenItem" => RehearsalEvidenceKind::FrozenItem,
            "acceptedSubmission" => RehearsalEvidenceKind::AcceptedSubmission,
            _ => return Err(invalid("invalid rehearsal evidence kind")),
        };
        let payload = row.try_get("payload").map_err(map_sqlx_error)?;
        let at = ActivityTimestamp::from_unix_millis(
            row.try_get("recorded_at_millis").map_err(map_sqlx_error)?,
        );
        let decoded = match kind {
            RehearsalEvidenceKind::FrozenItem => one(
                frozen
                    .iter()
                    .filter_map(|item| {
                        domain::rehearsal::persistence::decode_frozen_evidence_payload(
                            &payload, item, at,
                        )
                        .ok()
                    })
                    .collect(),
                "frozen rehearsal evidence does not bind exactly one row",
            )?,
            RehearsalEvidenceKind::AcceptedSubmission => one(
                roots
                    .iter()
                    .filter_map(|root| {
                        frozen
                            .iter()
                            .find(|item| item.attempt == root.root.sealed_request().attempt())
                            .and_then(|item| {
                                domain::rehearsal::persistence::decode_accepted_evidence_payload(
                                    &payload, &root.root, item, at,
                                )
                                .ok()
                            })
                    })
                    .collect(),
                "accepted rehearsal evidence does not bind exactly one claim",
            )?,
            RehearsalEvidenceKind::Genesis => {
                return Err(invalid("persisted rehearsal evidence cannot be genesis"));
            }
        };
        if decoded.kind() != kind
            || private_payload_digest(&decoded)
                != digest(
                    &row.try_get::<Vec<u8>, _>("payload_digest")
                        .map_err(map_sqlx_error)?,
                )?
        {
            return Err(invalid("rehearsal evidence payload digest mismatch"));
        }
        entries.push(RehearsalEvidenceChainEntry {
            record: question_model::RehearsalEvidenceRecord {
                sequence,
                kind,
                previous_digest: Some(digest(
                    &row.try_get::<Vec<u8>, _>("previous_digest")
                        .map_err(map_sqlx_error)?,
                )?),
                digest: digest(
                    &row.try_get::<Vec<u8>, _>("entry_digest")
                        .map_err(map_sqlx_error)?,
                )?,
                recorded_at: at,
            },
            payload: decoded,
        });
    }
    Ok(entries)
}

async fn hydrate_claims(
    tx: &mut Transaction<'_, Postgres>,
    run: &rows::HydratedRun,
    evidence: &[RehearsalEvidenceChainEntry],
    roots: &mut Vec<RootState>,
    context: RehearsalGenesisContext,
) -> Result<Vec<HydratedClaim>, StoreError> {
    let events = sqlx::query("SELECT claim_id, sequence, operation_id, generation, phase, abandonment_reason, accepted_evidence_sequence, accepted_evidence_digest, receipt_digest, (extract(epoch FROM recorded_at) * 1000)::bigint AS recorded_at_millis FROM rehearsal_submission_claim_event WHERE tenant_id=$1 AND rehearsal_run_id=$2 ORDER BY claim_id, sequence").bind(run.tenant.as_uuid()).bind(run.id.as_uuid()).fetch_all(&mut **tx).await.map_err(map_sqlx_error)?;
    let receipt_rows = sqlx::query("SELECT claim_id, outcome_projection, receipt_digest FROM rehearsal_submission_receipt WHERE tenant_id=$1 AND rehearsal_run_id=$2 ORDER BY claim_id").bind(run.tenant.as_uuid()).bind(run.id.as_uuid()).fetch_all(&mut **tx).await.map_err(map_sqlx_error)?;
    let mut receipts = BTreeMap::new();
    for row in receipt_rows {
        let claim =
            RehearsalSubmissionClaimId::from_uuid(row.try_get("claim_id").map_err(map_sqlx_error)?);
        let value = (
            row.try_get("outcome_projection").map_err(map_sqlx_error)?,
            digest(
                &row.try_get::<Vec<u8>, _>("receipt_digest")
                    .map_err(map_sqlx_error)?,
            )?,
        );
        if receipts.insert(claim, value).is_some() {
            return Err(invalid("duplicate rehearsal receipt"));
        }
    }
    let mut result = Vec::with_capacity(roots.len());
    for state in roots.drain(..) {
        let history = events
            .iter()
            .filter(|row| {
                row.try_get::<sqlx::types::Uuid, _>("claim_id").ok()
                    == Some(state.root.claim().as_uuid())
            })
            .map(|row| decode_event(row, &state.root))
            .collect::<Result<Vec<_>, _>>()?;
        let completed = history
            .last()
            .is_some_and(|event| event.phase() == RehearsalSubmissionClaimPhase::Completed);
        let proof = completed
            .then(|| {
                verify_rehearsal_claim_completion_proof(context, run.head, &state.root, evidence)
                    .map_err(invalid)
            })
            .transpose()?;
        let outcome = proof.as_ref().map(|proof| proof.replay_receipt());
        match (outcome.as_ref(), receipts.remove(&state.root.claim())) {
            (Some(outcome), Some((projection, digest))) => {
                domain::rehearsal::persistence::verify_persisted_receipt_witness(
                    outcome,
                    &projection,
                    digest,
                )
                .map_err(invalid)?
            }
            (Some(_), None) | (None, Some(_)) => {
                return Err(invalid("rehearsal receipt lifecycle mismatch"));
            }
            (None, None) => {}
        }
        let snapshot = hydrate_claim_history(&state.root, &history, proof).map_err(invalid)?;
        result.push(HydratedClaim {
            idempotency_key: state.key,
            root: state.root,
            snapshot,
            outcome,
        });
    }
    if !receipts.is_empty() {
        return Err(invalid("receipt has no claim root"));
    }
    Ok(result)
}

fn decode_event(
    row: &sqlx::postgres::PgRow,
    root: &RehearsalClaimRoot,
) -> Result<RehearsalClaimTransitionEvent, StoreError> {
    let phase = match row
        .try_get::<String, _>("phase")
        .map_err(map_sqlx_error)?
        .as_str()
    {
        "prepared" => RehearsalSubmissionClaimPhase::Prepared,
        "gradingDispatched" => RehearsalSubmissionClaimPhase::GradingDispatched,
        "completed" => RehearsalSubmissionClaimPhase::Completed,
        "abandonedBeforeDispatch" => RehearsalSubmissionClaimPhase::AbandonedBeforeDispatch,
        "revokedStaleRevision" => RehearsalSubmissionClaimPhase::RevokedStaleRevision,
        "revokedTerminalLifecycle" => RehearsalSubmissionClaimPhase::RevokedTerminalLifecycle,
        "revokedSourceContextRemoved" => RehearsalSubmissionClaimPhase::RevokedSourceContextRemoved,
        _ => return Err(invalid("invalid rehearsal claim phase")),
    };
    let reason = match row
        .try_get::<Option<String>, _>("abandonment_reason")
        .map_err(map_sqlx_error)?
        .as_deref()
    {
        None => None,
        Some("localPreparationFailed") => {
            Some(domain::RehearsalPreDispatchAbandonReason::LocalPreparationFailed)
        }
        Some("nativeBackendAdmissionRejected") => {
            Some(domain::RehearsalPreDispatchAbandonReason::NativeBackendAdmissionRejected)
        }
        Some("trustedRendererAdmissionRejected") => {
            Some(domain::RehearsalPreDispatchAbandonReason::TrustedRendererAdmissionRejected)
        }
        _ => return Err(invalid("invalid rehearsal abandonment reason")),
    };
    let material = match (
        row.try_get::<Option<i64>, _>("accepted_evidence_sequence")
            .map_err(map_sqlx_error)?,
        row.try_get::<Option<Vec<u8>>, _>("accepted_evidence_digest")
            .map_err(map_sqlx_error)?,
        row.try_get::<Option<Vec<u8>>, _>("receipt_digest")
            .map_err(map_sqlx_error)?,
    ) {
        (None, None, None) => None,
        (Some(sequence), Some(accepted), Some(receipt)) => Some(
            RehearsalClaimCompletionMaterial::from_persisted(
                u64::try_from(sequence).map_err(|_| {
                    StoreError::InvalidRecord("invalid accepted evidence sequence".into())
                })?,
                digest(&accepted)?,
                digest(&receipt)?,
            )
            .ok_or_else(|| StoreError::InvalidRecord("invalid completion material".into()))?,
        ),
        _ => return Err(invalid("partial rehearsal completion material")),
    };
    Ok(root.restore_transition(
        u64::try_from(row.try_get::<i64, _>("sequence").map_err(map_sqlx_error)?)
            .map_err(|_| StoreError::InvalidRecord("invalid claim sequence".into()))?,
        question_model::RehearsalGradeOperationId::from_uuid(
            row.try_get("operation_id").map_err(map_sqlx_error)?,
        ),
        domain::RehearsalClaimGeneration::from_persisted(
            u32::try_from(
                row.try_get::<i64, _>("generation")
                    .map_err(map_sqlx_error)?,
            )
            .map_err(|_| StoreError::InvalidRecord("invalid claim generation".into()))?,
        )
        .ok_or_else(|| StoreError::InvalidRecord("invalid claim generation".into()))?,
        phase,
        ActivityTimestamp::from_unix_millis(
            row.try_get("recorded_at_millis").map_err(map_sqlx_error)?,
        ),
        reason,
        material,
    ))
}

fn genesis(run: &rows::HydratedRun) -> RehearsalGenesisContext {
    RehearsalGenesisContext {
        rehearsal: run.id,
        tenant: run.tenant,
        course: run.course,
        assignment: run.receipt.assignment,
        direct_instructor_membership: run.owner,
        revision: run.receipt.revision,
        subject_fingerprint: run.subject_fingerprint,
    }
}
fn digest(bytes: &[u8]) -> Result<RehearsalEvidenceDigest, StoreError> {
    Ok(RehearsalEvidenceDigest::from_bytes(
        bytes
            .try_into()
            .map_err(|_| StoreError::InvalidRecord("invalid rehearsal digest".into()))?,
    ))
}
fn one<T>(mut values: Vec<T>, message: &str) -> Result<T, StoreError> {
    (values.len() == 1)
        .then(|| values.pop().expect("checked one value"))
        .ok_or_else(|| StoreError::InvalidRecord(message.into()))
}
fn invalid(_error: impl std::fmt::Debug) -> StoreError {
    StoreError::InvalidRecord("invalid rehearsal aggregate".into())
}

pub(super) fn require_locator(
    run: &rows::RunLocator,
    tenant: question_model::TenantId,
    locator: crate::RehearsalLocator,
    source: &auth::LockedSource,
) -> Result<(), StoreError> {
    (run.tenant == tenant
        && run.course == locator.course
        && run.assignment_id == source.assignment
        && run.owner == source.owner
        && run.actor == locator.actor
        && run.assignment == locator.assignment
        && run.revision == locator.revision)
        .then_some(())
        .ok_or(StoreError::NotFound)
}

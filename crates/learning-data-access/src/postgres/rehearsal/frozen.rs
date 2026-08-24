//! Frozen-item append capability.

use domain::{RehearsalEvidencePayload, evidence_entry_digest, private_payload_digest};

use super::super::*;
use super::{auth, hydration, integrity};

pub(super) async fn append_in_tx(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    tenant: question_model::TenantId,
    locator: crate::RehearsalLocator,
    frozen: question_model::RehearsalFrozenItemEvidence,
    start_operation: uuid::Uuid,
    start_nonce: uuid::Uuid,
    ordinal: i32,
) -> Result<(), StoreError> {
    let witness = auth::prepare_operation(tx, tenant, locator).await?;
    let source = witness.source();
    let aggregate = hydration::load_authorized(tx, tenant, locator, &source).await?;
    if aggregate.run.id != witness.run {
        return Err(StoreError::NotFound);
    }
    integrity::require_active(&aggregate.run)?;
    crate::ensure_rehearsal_delivery_supported(&frozen.response_definition)?;
    let millis: i64 = sqlx::query_scalar("SELECT ple_rehearsal_now_millis()")
        .fetch_one(&mut **tx)
        .await
        .map_err(map_sqlx_error)?;
    let timestamp = question_model::ActivityTimestamp::from_unix_millis(millis);
    if frozen.frozen_at != timestamp {
        return Err(StoreError::Conflict);
    }
    let payload = RehearsalEvidencePayload::FrozenItem(frozen.clone());
    let sequence =
        aggregate.run.head.length().checked_add(1).ok_or_else(|| {
            StoreError::Unavailable("rehearsal evidence sequence exhausted".into())
        })?;
    let digest = evidence_entry_digest(
        sequence,
        payload.kind(),
        aggregate.run.head.digest(),
        private_payload_digest(&payload),
        timestamp,
    );
    let stored = sqlx::query_scalar::<_, bool>("SELECT ple_rehearsal_route_append_frozen_item($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15,$16,$17,$18,$19,$20,$21,$22)")
        .bind(tenant.as_uuid()).bind(locator.actor.as_uuid()).bind(locator.course.as_uuid()).bind(source.assignment.as_uuid()).bind(i64::try_from(locator.revision.value()).map_err(|_| StoreError::InvalidRecord("teaching revision exceeds database range".into()))?).bind(aggregate.run.id.as_uuid()).bind(aggregate.run.head.digest().as_bytes().to_vec()).bind(i64::from(aggregate.run.head.length())).bind(digest.as_bytes().to_vec()).bind(domain::rehearsal::persistence::encode_evidence_payload(&payload)).bind(private_payload_digest(&payload).as_bytes().to_vec()).bind(millis).bind(frozen.attempt.as_uuid()).bind(frozen.problem.problem.as_uuid()).bind(frozen.problem.version.as_uuid()).bind(serde_json::to_value(&frozen.response_definition).map_err(|_| StoreError::InvalidRecord("response definition serialization failed".into()))?).bind(domain::frozen_response_schema_digest(&frozen.response_definition).as_bytes().to_vec()).bind(frozen.canonical_content_digest.as_bytes().to_vec()).bind(millis)
        .bind(start_operation).bind(start_nonce).bind(ordinal)
        .fetch_one(&mut **tx).await.map_err(map_sqlx_error)?;
    if !stored {
        return Err(StoreError::Conflict);
    }
    let verified = hydration::load_authorized(tx, tenant, locator, &source).await?;
    if verified.frozen(frozen.attempt) != Some(&frozen) {
        return Err(StoreError::InvalidRecord(
            "frozen rehearsal item did not verify after mutation".into(),
        ));
    }
    Ok(())
}

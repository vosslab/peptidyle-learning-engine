//! Frozen-item append capability.

use domain::{RehearsalEvidencePayload, evidence_entry_digest, private_payload_digest};

use super::super::*;
use super::{auth, hydration, integrity};

pub(super) async fn append(
    store: &PostgresStore,
    context: TenantContext,
    command: crate::AppendRehearsalFrozenItemCommand,
) -> Result<(), StoreError> {
    let tenant = context.tenant_id();
    let mut tx = store.begin_tenant(context).await?;
    let witness = auth::prepare_operation(&mut tx, tenant, command.locator).await?;
    let source = witness.source();
    let aggregate = hydration::load_authorized(&mut tx, tenant, command.locator, &source).await?;
    if aggregate.run.id != witness.run {
        return Err(StoreError::NotFound);
    }
    integrity::require_active(&aggregate.run)?;
    crate::ensure_rehearsal_delivery_supported(&command.frozen.response_definition)?;
    let millis: i64 = sqlx::query_scalar("SELECT ple_rehearsal_now_millis()")
        .fetch_one(&mut *tx)
        .await
        .map_err(map_sqlx_error)?;
    let timestamp = question_model::ActivityTimestamp::from_unix_millis(millis);
    if command.frozen.frozen_at != timestamp {
        return Err(StoreError::Conflict);
    }
    let payload = RehearsalEvidencePayload::FrozenItem(command.frozen.clone());
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
    let stored = sqlx::query_scalar::<_, bool>("SELECT ple_rehearsal_append_frozen_item($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15,$16,$17,$18,$19)")
        .bind(tenant.as_uuid()).bind(command.locator.actor.as_uuid()).bind(command.locator.course.as_uuid()).bind(source.assignment.as_uuid()).bind(i64::try_from(command.locator.revision.value()).map_err(|_| StoreError::InvalidRecord("teaching revision exceeds database range".into()))?).bind(aggregate.run.id.as_uuid()).bind(aggregate.run.head.digest().as_bytes().to_vec()).bind(i64::from(aggregate.run.head.length())).bind(digest.as_bytes().to_vec()).bind(domain::rehearsal::persistence::encode_evidence_payload(&payload)).bind(private_payload_digest(&payload).as_bytes().to_vec()).bind(millis).bind(command.frozen.attempt.as_uuid()).bind(command.frozen.problem.problem.as_uuid()).bind(command.frozen.problem.version.as_uuid()).bind(serde_json::to_value(&command.frozen.response_definition).map_err(|_| StoreError::InvalidRecord("response definition serialization failed".into()))?).bind(domain::frozen_response_schema_digest(&command.frozen.response_definition).as_bytes().to_vec()).bind(command.frozen.canonical_content_digest.as_bytes().to_vec()).bind(millis)
        .fetch_one(&mut *tx).await.map_err(map_sqlx_error)?;
    if !stored {
        return Err(StoreError::Conflict);
    }
    let verified = hydration::load_authorized(&mut tx, tenant, command.locator, &source).await?;
    if verified.frozen(command.frozen.attempt) != Some(&command.frozen) {
        return Err(StoreError::InvalidRecord(
            "frozen rehearsal item did not verify after mutation".into(),
        ));
    }
    tx.commit().await.map_err(map_sqlx_error)?;
    Ok(())
}

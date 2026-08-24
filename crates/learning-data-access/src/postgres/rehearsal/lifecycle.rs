//! Terminal capability calls after an authorized receipt read.

use super::super::*;
use super::{auth, hydration, integrity};

pub(super) async fn terminalize(
    store: &PostgresStore,
    context: TenantContext,
    locator: crate::RehearsalLocator,
    lifecycle: &str,
) -> Result<question_model::RehearsalRunReceipt, StoreError> {
    let tenant = context.tenant_id();
    let mut tx = store.begin_tenant(context).await?;
    let witness = auth::prepare_operation(&mut tx, tenant, locator).await?;
    let source = witness.source();
    let aggregate = hydration::load_authorized(&mut tx, tenant, locator, &source).await?;
    if aggregate.run.id != witness.run {
        return Err(StoreError::NotFound);
    }
    integrity::require_active(&aggregate.run)?;
    let applied: bool =
        sqlx::query_scalar("SELECT ple_rehearsal_terminalize($1,$2,$3,$4,$5,$6,$7)")
            .bind(tenant.as_uuid())
            .bind(locator.actor.as_uuid())
            .bind(locator.course.as_uuid())
            .bind(source.assignment.as_uuid())
            .bind(i64::try_from(locator.revision.value()).map_err(|_| {
                StoreError::InvalidRecord("teaching revision exceeds database range".into())
            })?)
            .bind(aggregate.run.id.as_uuid())
            .bind(lifecycle)
            .fetch_one(&mut *tx)
            .await
            .map_err(map_sqlx_error)?;
    if !applied {
        return Err(StoreError::Conflict);
    }
    let verified = hydration::load_authorized(&mut tx, tenant, locator, &source).await?;
    let expected = match lifecycle {
        "completed" => question_model::RehearsalLifecycle::Completed,
        "discardedByInstructor" => question_model::RehearsalLifecycle::DiscardedByInstructor,
        _ => {
            return Err(StoreError::InvalidRecord(
                "invalid requested rehearsal lifecycle".into(),
            ));
        }
    };
    if verified.run.receipt.lifecycle != expected {
        return Err(StoreError::InvalidRecord(
            "rehearsal terminal transition did not persist requested lifecycle".into(),
        ));
    }
    let result = verified.run.receipt;
    tx.commit().await.map_err(map_sqlx_error)?;
    Ok(result)
}

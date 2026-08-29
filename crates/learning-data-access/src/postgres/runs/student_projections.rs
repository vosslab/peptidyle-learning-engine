//! Read-only authorization for Student run projections.

use question_model::{AssignmentRun, RunId, TenantId, UserId};
use sqlx::{Postgres, Transaction};

use crate::StoreError;

use super::super::{
    entitlement, load_assignment, load_postgres_enrollment, load_postgres_run, map_sqlx_error,
};

/// Authorizes one active Student run for a projection without acquiring
/// mutation locks. Attempt issuance and submission transitions use the 1817
/// broker-prepared witnesses instead of escalating this read capability.
pub(super) async fn active_student_run_for_read(
    transaction: &mut Transaction<'_, Postgres>,
    tenant: TenantId,
    actor: UserId,
    run: RunId,
) -> Result<Option<AssignmentRun>, StoreError> {
    let record = match load_postgres_run(transaction, tenant, run).await {
        Ok(value) => value,
        Err(StoreError::NotFound) => return Ok(None),
        Err(error) => return Err(error),
    };
    let enrollment = load_postgres_enrollment(transaction, tenant, record.enrollment).await?;
    let assignment = load_assignment(transaction, tenant, enrollment.assignment).await?;
    let accessible: bool =
        sqlx::query_scalar("SELECT public.ple_course_records_accessible($1, $2)")
            .bind(tenant.as_uuid())
            .bind(assignment.course_id.as_uuid())
            .fetch_one(&mut **transaction)
            .await
            .map_err(map_sqlx_error)?;
    if !accessible {
        return Err(StoreError::NotFound);
    }
    let decision = entitlement::evaluate_current_read_only(
        transaction,
        tenant,
        actor,
        assignment.course_id,
        enrollment.assignment,
    )
    .await?;
    match decision {
        domain::entitlement::EntitlementDecision::Granted(grant)
            if grant.student() == enrollment.student => {}
        domain::entitlement::EntitlementDecision::Granted(_)
        | domain::entitlement::EntitlementDecision::Denied(_) => return Ok(None),
    }
    Ok(Some(record))
}

#[cfg(test)]
mod tests {
    #[test]
    fn active_student_run_projection_does_not_request_source_locks() {
        let source = include_str!("student_projections.rs");
        let helper = source
            .split("pub(super) async fn active_student_run_for_read")
            .nth(1)
            .and_then(|section| section.split("#[cfg(test)]").next())
            .expect("active Student projection remains a discrete helper");
        assert!(!helper.contains("FOR UPDATE"));
        assert!(!helper.contains("load_enrollment_for_update"));
        assert!(!helper.contains("load_run_for_update"));
        assert!(!helper.contains("entitlement::evaluate_current("));
    }
}

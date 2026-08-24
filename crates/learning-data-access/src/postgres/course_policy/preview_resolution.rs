//! Shared read-only policy resolution for learner and T3 projections.

use super::*;

pub(super) async fn load_course_term_for_preview(
    tx: &mut Transaction<'_, Postgres>,
    tenant: TenantId,
    course: CourseId,
) -> Result<CourseTerm, StoreError> {
    let row = sqlx::query(concat!(
        "SELECT term_start_date::text AS term_start_date, ",
        "term_end_date::text AS term_end_date, time_zone ",
        "FROM course WHERE tenant_id=$1 AND course_id=$2"
    ))
    .bind(tenant.as_uuid())
    .bind(course.as_uuid())
    .fetch_optional(&mut **tx)
    .await
    .map_err(map_sqlx_error)?
    .ok_or(StoreError::NotFound)?;
    let start_date: String = row.try_get("term_start_date").map_err(map_sqlx_error)?;
    let end_date: String = row.try_get("term_end_date").map_err(map_sqlx_error)?;
    let time_zone: String = row.try_get("time_zone").map_err(map_sqlx_error)?;
    CourseTerm::from_parts(&start_date, &end_date, &time_zone)
        .map_err(|error| StoreError::Unavailable(format!("stored course term is invalid: {error}")))
}

pub(super) async fn resolve_granted_effective_policy_read_only(
    tx: &mut Transaction<'_, Postgres>,
    grant: domain::entitlement::EntitlementGrant,
    authorization: domain::effective_assignment_policy::AuthorizationGate,
    prior_run_count: u32,
) -> Result<
    (
        domain::effective_assignment_policy::EffectivePolicyDecision,
        AssignmentRevision,
    ),
    StoreError,
> {
    let tenant = grant.tenant();
    let assignment = grant.assignment();
    let row = sqlx::query(
        "SELECT revision, lifecycle FROM assignment WHERE tenant_id=$1 AND assignment_id=$2",
    )
    .bind(tenant.as_uuid())
    .bind(assignment.as_uuid())
    .fetch_optional(&mut **tx)
    .await
    .map_err(map_sqlx_error)?
    .ok_or(StoreError::NotFound)?;
    let lifecycle = assignment_lifecycle_gate(parse_assignment_lifecycle(
        &row.try_get::<String, _>("lifecycle")
            .map_err(map_sqlx_error)?,
    )?);
    let now = database_timestamp(tx).await?;
    // Preserve G1 -> S5 -> G3 precedence. Denials cannot observe mutable
    // M1--M4 rows; inert inputs produce the typed gate denial instead.
    let inputs = if matches!(lifecycle, AssignmentLifecycleGate::Open)
        && matches!(
            authorization,
            domain::effective_assignment_policy::AuthorizationGate::Authorized
        ) {
        load_inputs(
            tx,
            tenant,
            assignment,
            Some(grant.student()),
            Some(grant.applicable_policy_scopes()),
        )
        .await?
    } else {
        inert_inputs()?
    };
    let decision = resolve_effective_policy(ResolveEffectivePolicyInput {
        lifecycle,
        entitlement: domain::entitlement::EntitlementDecision::Granted(grant),
        authorization,
        now,
        prior_run_count,
        base: inputs.base,
        group_schedule_offsets: inputs.schedule_offsets,
        group_accommodations: inputs.accommodations,
        individual_exception: inputs.individual,
    })
    .map_err(|error| {
        StoreError::InvalidRecord(format!("invalid effective policy inputs: {error:?}"))
    })?;
    Ok((
        decision,
        AssignmentRevision::from_stored(row.try_get("revision").map_err(map_sqlx_error)?)?,
    ))
}

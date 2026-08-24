use super::*;

#[cfg(feature = "postgres")]
pub(super) async fn require_attempt_owner(
    transaction: &mut Transaction<'_, Postgres>,
    tenant: TenantId,
    attempt: QuestionAttemptId,
    actor: UserId,
) -> Result<(), StoreError> {
    let owner = sqlx::query(
        "SELECT a.course_id, e.assignment_id, e.student_id FROM question_attempt AS qa \
         JOIN assignment_run AS ar ON ar.tenant_id = qa.tenant_id AND ar.run_id = qa.run_id \
         JOIN enrollment AS e ON e.tenant_id = ar.tenant_id AND e.enrollment_id = ar.enrollment_id \
         JOIN assignment AS a ON a.tenant_id = e.tenant_id AND a.assignment_id = e.assignment_id \
         WHERE qa.tenant_id = $1 AND qa.attempt_id = $2 \
           AND public.ple_course_records_accessible(a.tenant_id, a.course_id) \
         FOR KEY SHARE OF qa",
    )
    .bind(tenant.as_uuid())
    .bind(attempt.as_uuid())
    .fetch_optional(&mut **transaction)
    .await
    .map_err(map_sqlx_error)?;
    let owner = owner.ok_or(StoreError::NotFound)?;
    let assignment =
        AssignmentId::from_uuid(owner.try_get("assignment_id").map_err(map_sqlx_error)?);
    let student =
        question_model::StudentId::from_uuid(owner.try_get("student_id").map_err(map_sqlx_error)?);
    let course = CourseId::from_uuid(owner.try_get("course_id").map_err(map_sqlx_error)?);
    let decision =
        super::entitlement::evaluate_current(transaction, tenant, actor, course, assignment)
            .await?;
    match decision {
        domain::entitlement::EntitlementDecision::Granted(grant) if grant.student() == student => {
            Ok(())
        }
        domain::entitlement::EntitlementDecision::Granted(_) => Err(StoreError::NotFound),
        domain::entitlement::EntitlementDecision::Denied(_) => Err(StoreError::NotFound),
    }
}

#[cfg(feature = "postgres")]
pub(super) async fn postgres_is_course_instructor(
    transaction: &mut Transaction<'_, Postgres>,
    tenant: TenantId,
    course: CourseId,
    actor: UserId,
) -> Result<bool, StoreError> {
    Ok(sqlx::query(
        "SELECT user_id FROM course_member \
         WHERE tenant_id = $1 AND course_id = $2 AND user_id = $3 \
           AND role = 'instructor' AND status = 'active' \
         FOR KEY SHARE",
    )
    .bind(tenant.as_uuid())
    .bind(course.as_uuid())
    .bind(actor.as_uuid())
    .fetch_optional(&mut **transaction)
    .await
    .map_err(map_sqlx_error)?
    .is_some())
}

#[cfg(feature = "postgres")]
pub(super) async fn database_timestamp(
    transaction: &mut Transaction<'_, Postgres>,
) -> Result<ActivityTimestamp, StoreError> {
    let milliseconds: i64 = sqlx::query_scalar(
        "SELECT floor(extract(epoch FROM transaction_timestamp()) * 1000)::bigint",
    )
    .fetch_one(&mut **transaction)
    .await
    .map_err(map_sqlx_error)?;
    Ok(ActivityTimestamp::from_unix_millis(milliseconds))
}

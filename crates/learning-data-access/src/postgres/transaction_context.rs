use super::*;
use crate::StudentWorkRoutingBinding;

#[cfg(feature = "postgres")]
/// Authorizes an immutable submission-receipt projection through its explicit
/// course/assignment route without acquiring mutation locks. Successor writes
/// use the 1817 attempt-preparation broker instead.
pub(super) async fn require_attempt_owner_for_read(
    transaction: &mut Transaction<'_, Postgres>,
    tenant: TenantId,
    attempt: QuestionAttemptId,
    actor: UserId,
) -> Result<StudentWorkRoutingBinding, StoreError> {
    let owner = sqlx::query(
        "SELECT a.course_id, e.assignment_id, e.student_id FROM question_attempt AS qa \
         JOIN assignment_run AS ar ON ar.tenant_id = qa.tenant_id AND ar.run_id = qa.run_id \
         JOIN enrollment AS e ON e.tenant_id = ar.tenant_id AND e.enrollment_id = ar.enrollment_id \
         JOIN assignment AS a ON a.tenant_id = e.tenant_id AND a.assignment_id = e.assignment_id \
         WHERE qa.tenant_id = $1 AND qa.attempt_id = $2 \
           AND public.ple_course_records_accessible(a.tenant_id, a.course_id)",
    )
    .bind(tenant.as_uuid())
    .bind(attempt.as_uuid())
    .fetch_optional(&mut **transaction)
    .await
    .map_err(map_sqlx_error)?;
    let owner = owner.ok_or(StoreError::NotFound)?;
    let binding = StudentWorkRoutingBinding::new(
        CourseId::from_uuid(owner.try_get("course_id").map_err(map_sqlx_error)?),
        AssignmentId::from_uuid(owner.try_get("assignment_id").map_err(map_sqlx_error)?),
    );
    let student =
        question_model::StudentId::from_uuid(owner.try_get("student_id").map_err(map_sqlx_error)?);
    let decision = super::entitlement::evaluate_current_read_only(
        transaction,
        tenant,
        actor,
        binding.course,
        binding.assignment,
    )
    .await?;
    match decision {
        domain::entitlement::EntitlementDecision::Granted(grant) if grant.student() == student => {
            Ok(binding)
        }
        domain::entitlement::EntitlementDecision::Granted(_) => Err(StoreError::NotFound),
        domain::entitlement::EntitlementDecision::Denied(_) => Err(StoreError::NotFound),
    }
}

#[cfg(feature = "postgres")]
pub(super) async fn postgres_is_course_instructor(
    transaction: &mut Transaction<'_, Postgres>,
    course: CourseId,
    actor: UserId,
) -> Result<bool, StoreError> {
    // This helper is a projection check used by both read models and the
    // public side of broker-owned mutations.  The broker must serialize and
    // revalidate mutation authority; ple_app deliberately has no UPDATE
    // privilege on course_member and therefore must not request a row lock.
    Ok(sqlx::query(
        "SELECT user_id FROM course_member \
         WHERE course_id = $1 AND user_id = $2 \
           AND role = 'instructor' AND status = 'active'",
    )
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

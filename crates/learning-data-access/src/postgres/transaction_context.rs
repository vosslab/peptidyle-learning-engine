use super::*;

#[cfg(feature = "postgres")]
pub(super) async fn require_attempt_owner(
    transaction: &mut Transaction<'_, Postgres>,
    tenant: TenantId,
    attempt: QuestionAttemptId,
    actor: UserId,
) -> Result<(), StoreError> {
    let owns_attempt = sqlx::query(
        "SELECT cm.user_id FROM question_attempt AS qa \
         JOIN assignment_run AS ar ON ar.tenant_id = qa.tenant_id AND ar.run_id = qa.run_id \
         JOIN enrollment AS e ON e.tenant_id = ar.tenant_id AND e.enrollment_id = ar.enrollment_id \
         JOIN assignment AS a ON a.tenant_id = e.tenant_id AND a.assignment_id = e.assignment_id \
         JOIN course_member AS cm ON cm.tenant_id = a.tenant_id AND cm.course_id = a.course_id \
             AND cm.user_id = e.user_id AND cm.role = 'student' \
         WHERE qa.tenant_id = $1 AND qa.attempt_id = $2 AND e.user_id = $3 \
           AND public.ple_course_records_accessible(a.tenant_id, a.course_id) \
         FOR KEY SHARE OF cm",
    )
    .bind(tenant.as_uuid())
    .bind(attempt.as_uuid())
    .bind(actor.as_uuid())
    .fetch_optional(&mut **transaction)
    .await
    .map_err(map_sqlx_error)?;
    if owns_attempt.is_some() {
        Ok(())
    } else {
        Err(StoreError::NotFound)
    }
}

/// Locks current membership so a learner operation and roster revocation are
/// serialized rather than authorized by a stale membership observation.
#[cfg(feature = "postgres")]
pub(super) async fn require_active_learner_membership(
    transaction: &mut Transaction<'_, Postgres>,
    tenant: TenantId,
    assignment: AssignmentId,
    actor: UserId,
) -> Result<(), StoreError> {
    let membership = sqlx::query(
        "SELECT cm.user_id FROM assignment AS a \
         JOIN course_member AS cm ON cm.tenant_id = a.tenant_id AND cm.course_id = a.course_id \
             AND cm.user_id = $3 AND cm.role = 'student' \
         WHERE a.tenant_id = $1 AND a.assignment_id = $2 \
           AND public.ple_course_records_accessible(a.tenant_id, a.course_id) \
         FOR KEY SHARE OF cm",
    )
    .bind(tenant.as_uuid())
    .bind(assignment.as_uuid())
    .bind(actor.as_uuid())
    .fetch_optional(&mut **transaction)
    .await
    .map_err(map_sqlx_error)?;
    membership.ok_or(StoreError::NotFound).map(|_| ())
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
         WHERE tenant_id = $1 AND course_id = $2 AND user_id = $3 AND role = 'instructor' \
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

#[cfg(feature = "postgres")]
pub(super) async fn load_published_record(
    transaction: &mut Transaction<'_, Postgres>,
    problem: ProblemId,
    version: VersionId,
) -> Result<PublishedProblemRecord, StoreError> {
    let row = sqlx::query(
        "SELECT payload, payload_sha256 FROM problem_version_payload \
         WHERE problem_id = $1 AND version_id = $2",
    )
    .bind(problem.as_uuid())
    .bind(version.as_uuid())
    .fetch_optional(&mut **transaction)
    .await
    .map_err(map_sqlx_error)?
    .ok_or(StoreError::NotFound)?;
    decode_payload_row(&row)
}

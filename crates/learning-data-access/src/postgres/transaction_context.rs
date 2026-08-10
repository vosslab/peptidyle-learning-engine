use super::*;

#[cfg(feature = "postgres")]
pub(super) async fn require_attempt_owner(
    transaction: &mut Transaction<'_, Postgres>,
    tenant: TenantId,
    attempt: QuestionAttemptId,
    actor: UserId,
) -> Result<(), StoreError> {
    let owns_attempt: bool = sqlx::query_scalar(
        "SELECT EXISTS( \
             SELECT 1 FROM question_attempt AS qa \
             JOIN assignment_run AS ar \
               ON ar.tenant_id = qa.tenant_id AND ar.run_id = qa.run_id \
             JOIN enrollment AS e \
               ON e.tenant_id = ar.tenant_id AND e.enrollment_id = ar.enrollment_id \
             WHERE qa.tenant_id = $1 AND qa.attempt_id = $2 AND e.user_id = $3 \
         )",
    )
    .bind(tenant.as_uuid())
    .bind(attempt.as_uuid())
    .bind(actor.as_uuid())
    .fetch_one(&mut **transaction)
    .await
    .map_err(map_sqlx_error)?;
    if owns_attempt {
        Ok(())
    } else {
        Err(StoreError::NotFound)
    }
}

#[cfg(feature = "postgres")]
pub(super) async fn postgres_is_course_instructor(
    transaction: &mut Transaction<'_, Postgres>,
    tenant: TenantId,
    course: CourseId,
    actor: UserId,
) -> Result<bool, StoreError> {
    sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM course_member \
         WHERE tenant_id = $1 AND course_id = $2 AND user_id = $3 AND role = 'instructor')",
    )
    .bind(tenant.as_uuid())
    .bind(course.as_uuid())
    .bind(actor.as_uuid())
    .fetch_one(&mut **transaction)
    .await
    .map_err(map_sqlx_error)
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

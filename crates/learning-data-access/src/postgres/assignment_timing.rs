//! PostgreSQL assignment timing, policy exceptions, and timed-attempt job helpers.

use super::connection::map_sqlx_error;
use super::runs::add_seconds;
use super::{decode_payload_row, decode_payload_row_named};

use question_model::run_policy::TimingPolicy;
use question_model::{
    ActivityTimestamp, AssignmentDeadlineBehavior, AssignmentEnrollment, AssignmentId,
    AssignmentPolicyExceptionId, AssignmentRun, AssignmentTimingPolicy, CourseGroupId, CourseId,
    EnrollmentId, LateSubmissionPolicy, QuestionAttemptId, StudentId, TenantId, UserId,
};
use sqlx::postgres::PgRow;
use sqlx::types::{Json, Uuid};
use sqlx::{Postgres, Row, Transaction};

use crate::{
    AssignmentExceptionLimit, AssignmentExceptionTimestamp, AssignmentPolicyException,
    AssignmentPolicyExceptionTarget, AssignmentRevision, JobId, JobPayload, ResolvedAttemptTiming,
    StoreError, StoredAssignmentTiming, resolve_assignment_policy,
    validate_assignment_policy_exception, validate_assignment_timing,
};

/// Decodes the shared PostgreSQL `INTEGER` duration column without widening
/// its domain into Rust's larger unsigned range.
pub(super) fn decode_postgres_assignment_time_limit(
    value: Option<i32>,
) -> Result<Option<u32>, StoreError> {
    let seconds = value
        .map(|value| {
            u32::try_from(value).map_err(|_| {
                StoreError::Unavailable("stored assignment time limit is invalid".to_string())
            })
        })
        .transpose()?;
    if seconds == Some(0)
        || seconds.is_some_and(|value| value > question_model::MAX_ASSIGNMENT_TIME_LIMIT_SECONDS)
    {
        return Err(StoreError::Unavailable(
            "stored assignment time limit is invalid".to_string(),
        ));
    }
    Ok(seconds)
}

pub(super) fn decode_stored_assignment_timing(
    row: &PgRow,
    tenant: TenantId,
) -> Result<StoredAssignmentTiming, StoreError> {
    let auto_submit: bool = row.try_get("auto_submit").map_err(map_sqlx_error)?;
    if !auto_submit {
        return Err(StoreError::Unavailable(
            "stored assignment uses unsupported overtime behavior".to_string(),
        ));
    }
    let late_submission: String = row
        .try_get("late_submission_policy")
        .map_err(map_sqlx_error)?;
    let timestamp = |name| -> Result<Option<ActivityTimestamp>, StoreError> {
        Ok(row
            .try_get::<Option<i64>, _>(name)
            .map_err(map_sqlx_error)?
            .map(ActivityTimestamp::from_unix_millis))
    };
    let time_limit_seconds: Option<i32> =
        row.try_get("time_limit_seconds").map_err(map_sqlx_error)?;
    let attempt_limit: Option<i32> = row.try_get("attempt_limit").map_err(map_sqlx_error)?;
    Ok(StoredAssignmentTiming {
        tenant,
        course: CourseId::from_uuid(row.try_get("course_id").map_err(map_sqlx_error)?),
        assignment: AssignmentId::from_uuid(row.try_get("assignment_id").map_err(map_sqlx_error)?),
        policy: AssignmentTimingPolicy {
            visible: row.try_get("visible").map_err(map_sqlx_error)?,
            available_at: timestamp("available_at_millis")?,
            due_at: timestamp("due_at_millis")?,
            closes_at: timestamp("closes_at_millis")?,
            late_submission: parse_late_submission_policy(&late_submission)?,
            time_limit_seconds: decode_postgres_assignment_time_limit(time_limit_seconds)?,
            attempt_limit: attempt_limit
                .map(|value| {
                    u32::try_from(value).map_err(|_| {
                        StoreError::Unavailable(
                            "stored assignment attempt limit is invalid".to_string(),
                        )
                    })
                })
                .transpose()?,
            deadline_behavior: AssignmentDeadlineBehavior::AutoSubmit,
        },
        revision: AssignmentRevision::from_stored(
            row.try_get("revision").map_err(map_sqlx_error)?,
        )?,
    })
}
pub(super) fn parse_late_submission_policy(
    value: &str,
) -> Result<LateSubmissionPolicy, StoreError> {
    match value {
        "accept" => Ok(LateSubmissionPolicy::Accept),
        "mark_late" => Ok(LateSubmissionPolicy::MarkLate),
        "reject" => Ok(LateSubmissionPolicy::Reject),
        _ => Err(StoreError::Unavailable(
            "stored late-submission policy is invalid".to_string(),
        )),
    }
}

#[cfg(feature = "postgres")]
pub(super) fn late_submission_policy_name(value: LateSubmissionPolicy) -> &'static str {
    match value {
        LateSubmissionPolicy::Accept => "accept",
        LateSubmissionPolicy::MarkLate => "mark_late",
        LateSubmissionPolicy::Reject => "reject",
    }
}

#[cfg(feature = "postgres")]
pub(super) async fn load_postgres_course_group_members(
    transaction: &mut Transaction<'_, Postgres>,
    tenant: TenantId,
    group: CourseGroupId,
) -> Result<Vec<UserId>, StoreError> {
    sqlx::query_scalar::<_, Uuid>(
        "SELECT user_id FROM course_group_member WHERE tenant_id = $1 \
         AND course_group_id = $2 ORDER BY user_id",
    )
    .bind(tenant.as_uuid())
    .bind(group.as_uuid())
    .fetch_all(&mut **transaction)
    .await
    .map_err(map_sqlx_error)
    .map(|members| members.into_iter().map(UserId::from_uuid).collect())
}

#[cfg(feature = "postgres")]
pub(super) async fn lock_postgres_assignment_policy(
    transaction: &mut Transaction<'_, Postgres>,
    tenant: TenantId,
    assignment: AssignmentId,
) -> Result<(), StoreError> {
    // Serialize attempt issue/start with base timing, exception, and group
    // membership changes. Callers take this advisory lock before assignment
    // and active attempt/timing row locks; multi-assignment callers sort IDs.
    sqlx::query(
        "SELECT pg_advisory_xact_lock(hashtextextended($1::uuid::text || ':' || $2::uuid::text, 0))",
    )
    .bind(tenant.as_uuid())
    .bind(assignment.as_uuid())
    .execute(&mut **transaction)
    .await
    .map_err(map_sqlx_error)?;
    Ok(())
}
pub(super) async fn load_postgres_assignment_timing(
    transaction: &mut Transaction<'_, Postgres>,
    tenant: TenantId,
    assignment: AssignmentId,
    lock: bool,
) -> Result<Option<StoredAssignmentTiming>, StoreError> {
    let row = if lock {
        sqlx::query(
            "SELECT assignment_id, course_id, visible, \
                    floor(extract(epoch FROM available_at) * 1000)::bigint AS available_at_millis, \
                    floor(extract(epoch FROM due_at) * 1000)::bigint AS due_at_millis, \
                    floor(extract(epoch FROM closes_at) * 1000)::bigint AS closes_at_millis, \
                    late_submission_policy, time_limit_seconds, auto_submit, attempt_limit, revision \
             FROM assignment WHERE tenant_id = $1 AND assignment_id = $2 FOR UPDATE",
        )
        .bind(tenant.as_uuid())
        .bind(assignment.as_uuid())
        .fetch_optional(&mut **transaction)
        .await
        .map_err(map_sqlx_error)?
    } else {
        sqlx::query(
            "SELECT assignment_id, course_id, visible, \
                    floor(extract(epoch FROM available_at) * 1000)::bigint AS available_at_millis, \
                    floor(extract(epoch FROM due_at) * 1000)::bigint AS due_at_millis, \
                    floor(extract(epoch FROM closes_at) * 1000)::bigint AS closes_at_millis, \
                    late_submission_policy, time_limit_seconds, auto_submit, attempt_limit, revision \
             FROM assignment WHERE tenant_id = $1 AND assignment_id = $2",
        )
        .bind(tenant.as_uuid())
        .bind(assignment.as_uuid())
        .fetch_optional(&mut **transaction)
        .await
        .map_err(map_sqlx_error)?
    };
    row.as_ref()
        .map(|row| decode_stored_assignment_timing(row, tenant))
        .transpose()
}

#[cfg(feature = "postgres")]
pub(super) async fn load_postgres_enrollment_by_student(
    transaction: &mut Transaction<'_, Postgres>,
    tenant: TenantId,
    assignment: AssignmentId,
    student: StudentId,
) -> Result<Option<AssignmentEnrollment>, StoreError> {
    let row = sqlx::query(
        "SELECT payload, payload_sha256 FROM enrollment WHERE tenant_id = $1 \
         AND assignment_id = $2 AND student_id = $3",
    )
    .bind(tenant.as_uuid())
    .bind(assignment.as_uuid())
    .bind(student.as_uuid())
    .fetch_optional(&mut **transaction)
    .await
    .map_err(map_sqlx_error)?;
    row.as_ref().map(decode_payload_row).transpose()
}

#[cfg(feature = "postgres")]
pub(super) fn postgres_exception_timestamp_columns(
    value: Option<AssignmentExceptionTimestamp>,
) -> (Option<&'static str>, Option<i64>) {
    match value {
        None => (None, None),
        Some(AssignmentExceptionTimestamp::Unrestricted) => (Some("unrestricted"), None),
        Some(AssignmentExceptionTimestamp::At(value)) => (Some("at"), Some(value.as_unix_millis())),
    }
}

#[cfg(feature = "postgres")]
pub(super) fn postgres_exception_limit_columns(
    value: Option<AssignmentExceptionLimit>,
) -> (Option<&'static str>, Option<i64>) {
    match value {
        None => (None, None),
        Some(AssignmentExceptionLimit::Unlimited) => (Some("unlimited"), None),
        Some(AssignmentExceptionLimit::Value(value)) => (Some("value"), Some(i64::from(value))),
    }
}

#[cfg(feature = "postgres")]
pub(super) fn decode_postgres_exception_timestamp(
    mode: Option<String>,
    millis: Option<i64>,
) -> Result<Option<AssignmentExceptionTimestamp>, StoreError> {
    match (mode.as_deref(), millis) {
        (None, None) => Ok(None),
        (Some("unrestricted"), None) => Ok(Some(AssignmentExceptionTimestamp::Unrestricted)),
        (Some("at"), Some(value)) => Ok(Some(AssignmentExceptionTimestamp::At(
            ActivityTimestamp::from_unix_millis(value),
        ))),
        _ => Err(StoreError::Unavailable(
            "stored assignment exception timestamp is invalid".to_string(),
        )),
    }
}

#[cfg(feature = "postgres")]
pub(super) fn decode_postgres_exception_limit(
    mode: Option<String>,
    value: Option<i32>,
) -> Result<Option<AssignmentExceptionLimit>, StoreError> {
    match (mode.as_deref(), value) {
        (None, None) => Ok(None),
        (Some("unlimited"), None) => Ok(Some(AssignmentExceptionLimit::Unlimited)),
        (Some("value"), Some(value)) => Ok(Some(AssignmentExceptionLimit::Value(
            u32::try_from(value).map_err(|_| {
                StoreError::Unavailable("stored assignment exception limit is invalid".to_string())
            })?,
        ))),
        _ => Err(StoreError::Unavailable(
            "stored assignment exception limit is invalid".to_string(),
        )),
    }
}

#[cfg(feature = "postgres")]
pub(super) fn decode_postgres_policy_exception(
    row: &PgRow,
) -> Result<AssignmentPolicyException, StoreError> {
    let student: Option<Uuid> = row.try_get("student_id").map_err(map_sqlx_error)?;
    let group: Option<Uuid> = row.try_get("course_group_id").map_err(map_sqlx_error)?;
    let target = match (student, group) {
        (Some(student), None) => {
            AssignmentPolicyExceptionTarget::Student(StudentId::from_uuid(student))
        }
        (None, Some(group)) => {
            AssignmentPolicyExceptionTarget::CourseGroup(CourseGroupId::from_uuid(group))
        }
        _ => {
            return Err(StoreError::Unavailable(
                "stored assignment exception target is invalid".to_string(),
            ));
        }
    };
    let exception = AssignmentPolicyException {
        id: AssignmentPolicyExceptionId::from_uuid(
            row.try_get("assignment_policy_exception_id")
                .map_err(map_sqlx_error)?,
        ),
        target,
        available_at: decode_postgres_exception_timestamp(
            row.try_get("available_mode").map_err(map_sqlx_error)?,
            row.try_get("available_at_millis").map_err(map_sqlx_error)?,
        )?,
        closes_at: decode_postgres_exception_timestamp(
            row.try_get("closes_mode").map_err(map_sqlx_error)?,
            row.try_get("closes_at_millis").map_err(map_sqlx_error)?,
        )?,
        time_limit_seconds: decode_postgres_exception_limit(
            row.try_get("time_limit_mode").map_err(map_sqlx_error)?,
            row.try_get("time_limit_seconds").map_err(map_sqlx_error)?,
        )?,
        attempt_limit: decode_postgres_exception_limit(
            row.try_get("attempt_limit_mode").map_err(map_sqlx_error)?,
            row.try_get("attempt_limit").map_err(map_sqlx_error)?,
        )?,
    };
    validate_assignment_policy_exception(&exception).map_err(|error| {
        StoreError::Unavailable(format!("stored assignment exception is invalid: {error}"))
    })?;
    Ok(exception)
}

#[cfg(feature = "postgres")]
pub(super) async fn load_postgres_policy_exception_identity_rows(
    transaction: &mut Transaction<'_, Postgres>,
    tenant: TenantId,
    assignment: AssignmentId,
    exception: AssignmentPolicyExceptionId,
    target: AssignmentPolicyExceptionTarget,
) -> Result<Vec<PgRow>, StoreError> {
    match target {
        AssignmentPolicyExceptionTarget::Student(student) => sqlx::query(
            "SELECT assignment_policy_exception_id, student_id, course_group_id, \
                    available_mode, \
                    floor(extract(epoch FROM available_at) * 1000)::bigint AS available_at_millis, \
                    closes_mode, \
                    floor(extract(epoch FROM closes_at) * 1000)::bigint AS closes_at_millis, \
                    time_limit_mode, time_limit_seconds, attempt_limit_mode, attempt_limit \
             FROM assignment_policy_exception WHERE tenant_id = $1 AND assignment_id = $2 \
               AND (assignment_policy_exception_id = $3 OR student_id = $4) FOR UPDATE",
        )
        .bind(tenant.as_uuid())
        .bind(assignment.as_uuid())
        .bind(exception.as_uuid())
        .bind(student.as_uuid())
        .fetch_all(&mut **transaction)
        .await
        .map_err(map_sqlx_error),
        AssignmentPolicyExceptionTarget::CourseGroup(group) => sqlx::query(
            "SELECT assignment_policy_exception_id, student_id, course_group_id, \
                    available_mode, \
                    floor(extract(epoch FROM available_at) * 1000)::bigint AS available_at_millis, \
                    closes_mode, \
                    floor(extract(epoch FROM closes_at) * 1000)::bigint AS closes_at_millis, \
                    time_limit_mode, time_limit_seconds, attempt_limit_mode, attempt_limit \
             FROM assignment_policy_exception WHERE tenant_id = $1 AND assignment_id = $2 \
               AND (assignment_policy_exception_id = $3 OR course_group_id = $4) FOR UPDATE",
        )
        .bind(tenant.as_uuid())
        .bind(assignment.as_uuid())
        .bind(exception.as_uuid())
        .bind(group.as_uuid())
        .fetch_all(&mut **transaction)
        .await
        .map_err(map_sqlx_error),
    }
}

#[cfg(feature = "postgres")]
pub(super) async fn load_postgres_resolved_assignment_policy(
    transaction: &mut Transaction<'_, Postgres>,
    tenant: TenantId,
    assignment: AssignmentId,
    enrollment: &AssignmentEnrollment,
    base_override: Option<AssignmentTimingPolicy>,
) -> Result<crate::ResolvedAssignmentTimingPolicy, StoreError> {
    let base = match base_override {
        Some(policy) => policy,
        None => load_postgres_assignment_timing_policy(transaction, tenant, assignment).await?,
    };
    let rows = sqlx::query(
        "SELECT exception.assignment_policy_exception_id, exception.student_id, \
                exception.course_group_id, exception.available_mode, \
                floor(extract(epoch FROM exception.available_at) * 1000)::bigint AS available_at_millis, \
                exception.closes_mode, \
                floor(extract(epoch FROM exception.closes_at) * 1000)::bigint AS closes_at_millis, \
                exception.time_limit_mode, exception.time_limit_seconds, \
                exception.attempt_limit_mode, exception.attempt_limit \
         FROM assignment_policy_exception AS exception \
         WHERE exception.tenant_id = $1 AND exception.assignment_id = $2 \
           AND (exception.student_id = $3 OR EXISTS ( \
                SELECT 1 FROM course_group_member AS member \
                 WHERE member.tenant_id = exception.tenant_id \
                   AND member.course_group_id = exception.course_group_id \
                   AND member.user_id = $4)) \
         ORDER BY exception.student_id NULLS LAST, exception.course_group_id NULLS LAST",
    )
    .bind(tenant.as_uuid())
    .bind(assignment.as_uuid())
    .bind(enrollment.student.as_uuid())
    .bind(enrollment.user.as_uuid())
    .fetch_all(&mut **transaction)
    .await
    .map_err(map_sqlx_error)?;
    let exceptions = rows
        .iter()
        .map(decode_postgres_policy_exception)
        .collect::<Result<Vec<_>, StoreError>>()?;
    resolve_assignment_policy(base, &exceptions)
}

#[cfg(feature = "postgres")]
pub(super) async fn lock_postgres_active_timing_rows(
    transaction: &mut Transaction<'_, Postgres>,
    tenant: TenantId,
    assignment: AssignmentId,
) -> Result<Vec<PgRow>, StoreError> {
    let active_attempts = sqlx::query_scalar::<_, Uuid>(
        "SELECT attempt.attempt_id FROM question_attempt AS attempt \
         JOIN assignment_run AS run ON run.tenant_id = attempt.tenant_id \
            AND run.run_id = attempt.run_id \
         JOIN enrollment ON enrollment.tenant_id = run.tenant_id \
            AND enrollment.enrollment_id = run.enrollment_id \
         WHERE attempt.tenant_id = $1 AND enrollment.assignment_id = $2 \
           AND attempt.attempt_status = 'in_progress' \
         ORDER BY attempt.attempt_id FOR UPDATE OF attempt",
    )
    .bind(tenant.as_uuid())
    .bind(assignment.as_uuid())
    .fetch_all(&mut **transaction)
    .await
    .map_err(map_sqlx_error)?;
    let rows = sqlx::query(
        "SELECT timing.attempt_id, timing.authored_grace_seconds, timing.timing_generation, \
                timing.job_id, job.state AS job_state, run.payload AS run_payload, \
                run.payload_sha256 AS run_payload_sha256, run.enrollment_id, \
                floor(extract(epoch FROM timing.authored_deadline) * 1000)::bigint \
                    AS authored_deadline_millis \
         FROM attempt_timing_current AS timing \
         JOIN question_attempt AS attempt ON attempt.tenant_id = timing.tenant_id \
            AND attempt.attempt_id = timing.attempt_id \
            AND attempt.occurred_at = timing.attempt_occurred_at \
         JOIN assignment_run AS run ON run.tenant_id = attempt.tenant_id \
            AND run.run_id = attempt.run_id \
         LEFT JOIN worker_job AS job ON job.job_id = timing.job_id \
         WHERE timing.tenant_id = $1 AND timing.assignment_id = $2 \
           AND attempt.attempt_status = 'in_progress' \
         ORDER BY timing.attempt_id FOR UPDATE OF timing",
    )
    .bind(tenant.as_uuid())
    .bind(assignment.as_uuid())
    .fetch_all(&mut **transaction)
    .await
    .map_err(map_sqlx_error)?;
    if rows.len() != active_attempts.len() {
        return Err(StoreError::Unavailable(
            "an active attempt is missing its current timing row".to_string(),
        ));
    }
    Ok(rows)
}

#[cfg(feature = "postgres")]
pub(super) async fn apply_postgres_locked_timing_rows(
    transaction: &mut Transaction<'_, Postgres>,
    tenant: TenantId,
    assignment: AssignmentId,
    base_override: Option<AssignmentTimingPolicy>,
    now: ActivityTimestamp,
    rows: Vec<PgRow>,
) -> Result<(), StoreError> {
    for row in rows {
        let enrollment_id =
            EnrollmentId::from_uuid(row.try_get("enrollment_id").map_err(map_sqlx_error)?);
        let enrollment_row = sqlx::query(
            "SELECT payload, payload_sha256 FROM enrollment WHERE tenant_id = $1 \
             AND enrollment_id = $2",
        )
        .bind(tenant.as_uuid())
        .bind(enrollment_id.as_uuid())
        .fetch_optional(&mut **transaction)
        .await
        .map_err(map_sqlx_error)?
        .ok_or(StoreError::NotFound)?;
        let enrollment: AssignmentEnrollment = decode_payload_row(&enrollment_row)?;
        let resolution = load_postgres_resolved_assignment_policy(
            transaction,
            tenant,
            assignment,
            &enrollment,
            base_override,
        )
        .await?;
        apply_postgres_active_timing_update(transaction, tenant, &resolution, now, &row).await?;
    }
    Ok(())
}

#[cfg(feature = "postgres")]
pub(super) async fn update_postgres_assignment_revision(
    transaction: &mut Transaction<'_, Postgres>,
    tenant: TenantId,
    assignment: AssignmentId,
    previous: AssignmentRevision,
    next: AssignmentRevision,
) -> Result<(), StoreError> {
    let updated = sqlx::query(
        "UPDATE assignment SET revision = $3, updated_at = transaction_timestamp() \
         WHERE tenant_id = $1 AND assignment_id = $2 AND revision = $4",
    )
    .bind(tenant.as_uuid())
    .bind(assignment.as_uuid())
    .bind(i64::try_from(next.value()).map_err(|_| StoreError::Conflict)?)
    .bind(i64::try_from(previous.value()).map_err(|_| StoreError::Conflict)?)
    .execute(&mut **transaction)
    .await
    .map_err(map_sqlx_error)?;
    if updated.rows_affected() != 1 {
        return Err(StoreError::Conflict);
    }
    Ok(())
}

#[cfg(feature = "postgres")]
pub(super) fn decode_postgres_resolved_attempt_timing(
    row: &PgRow,
    attempt: QuestionAttemptId,
) -> Result<ResolvedAttemptTiming, StoreError> {
    let timestamp = |column| {
        row.try_get::<Option<i64>, _>(column)
            .map_err(map_sqlx_error)
            .map(|value| value.map(ActivityTimestamp::from_unix_millis))
    };
    let time_limit: Option<i32> = row
        .try_get("resolved_time_limit_seconds")
        .map_err(map_sqlx_error)?;
    let attempt_limit: Option<i32> = row
        .try_get("resolved_attempt_limit")
        .map_err(map_sqlx_error)?;
    let sources: Json<Vec<AssignmentPolicyExceptionTarget>> =
        row.try_get("resolution_sources").map_err(map_sqlx_error)?;
    let policy = AssignmentTimingPolicy {
        visible: row.try_get("resolved_visible").map_err(map_sqlx_error)?,
        available_at: timestamp("available_at_millis")?,
        due_at: timestamp("due_at_millis")?,
        closes_at: timestamp("closes_at_millis")?,
        late_submission: parse_late_submission_policy(
            &row.try_get::<String, _>("resolved_late_submission_policy")
                .map_err(map_sqlx_error)?,
        )?,
        time_limit_seconds: decode_postgres_assignment_time_limit(time_limit)?,
        attempt_limit: attempt_limit
            .map(|value| {
                u32::try_from(value).map_err(|_| {
                    StoreError::Unavailable("stored resolved attempt limit is invalid".to_string())
                })
            })
            .transpose()?,
        deadline_behavior: AssignmentDeadlineBehavior::AutoSubmit,
    };
    validate_assignment_timing(policy)?;
    Ok(ResolvedAttemptTiming {
        attempt,
        policy,
        contributors: sources.0,
    })
}

#[cfg(feature = "postgres")]
pub(super) struct ResolvedPostgresAttemptTiming {
    pub(super) effective_deadline: Option<ActivityTimestamp>,
    pub(super) effective_grace_seconds: u32,
    pub(super) auto_submit_at: Option<ActivityTimestamp>,
    pub(super) resolution_kind: &'static str,
}

#[cfg(feature = "postgres")]
pub(super) fn resolved_postgres_attempt_timing(
    policy: AssignmentTimingPolicy,
    run: &AssignmentRun,
    authored_deadline: Option<ActivityTimestamp>,
    authored_grace_seconds: u32,
) -> Result<ResolvedPostgresAttemptTiming, StoreError> {
    let mut resolved = authored_deadline
        .map(|deadline| (deadline, authored_grace_seconds, 4_u8, "authored_question"));
    let mut consider =
        |deadline: ActivityTimestamp, grace_seconds: u32, priority: u8, source: &'static str| {
            if resolved.is_none_or(|(current_deadline, current_grace, current_priority, _)| {
                (deadline, grace_seconds, priority)
                    < (current_deadline, current_grace, current_priority)
            }) {
                resolved = Some((deadline, grace_seconds, priority, source));
            }
        };
    if let Some(seconds) = policy.time_limit_seconds {
        consider(
            add_seconds(run.started_at, seconds, "assignment time limit")?,
            0,
            3,
            "assignment_time_limit",
        );
    }
    if policy.late_submission == LateSubmissionPolicy::Reject
        && let Some(due_at) = policy.due_at
    {
        consider(due_at, 0, 2, "due_at");
    }
    if let Some(closes_at) = policy.closes_at {
        consider(closes_at, 0, 1, "closes_at");
    }
    let auto_submit_at = resolved
        .map(|(deadline, grace_seconds, _, _)| {
            add_seconds(deadline, grace_seconds, "attempt auto-submit deadline")
        })
        .transpose()?;
    let (effective_deadline, effective_grace_seconds, resolution_kind) = match resolved {
        Some((deadline, grace_seconds, _, source)) => (Some(deadline), grace_seconds, source),
        None => (None, 0, "untimed"),
    };
    Ok(ResolvedPostgresAttemptTiming {
        effective_deadline,
        effective_grace_seconds,
        auto_submit_at,
        resolution_kind,
    })
}

#[cfg(feature = "postgres")]
pub(super) async fn apply_postgres_active_timing_update(
    transaction: &mut Transaction<'_, Postgres>,
    tenant: TenantId,
    resolution: &crate::ResolvedAssignmentTimingPolicy,
    now: ActivityTimestamp,
    row: &PgRow,
) -> Result<(), StoreError> {
    let attempt = QuestionAttemptId::from_uuid(row.try_get("attempt_id").map_err(map_sqlx_error)?);
    let authored_deadline = row
        .try_get::<Option<i64>, _>("authored_deadline_millis")
        .map_err(map_sqlx_error)?
        .map(ActivityTimestamp::from_unix_millis);
    let authored_grace = u32::try_from(
        row.try_get::<i32, _>("authored_grace_seconds")
            .map_err(map_sqlx_error)?,
    )
    .map_err(|_| StoreError::Unavailable("stored authored grace is invalid".to_string()))?;
    let generation = u64::try_from(
        row.try_get::<i64, _>("timing_generation")
            .map_err(map_sqlx_error)?,
    )
    .map_err(|_| StoreError::Unavailable("stored timing generation is invalid".to_string()))?
    .checked_add(1)
    .ok_or(StoreError::Conflict)?;
    let run: AssignmentRun = decode_payload_row_named(row, "run_payload", "run_payload_sha256")?;
    let ResolvedPostgresAttemptTiming {
        effective_deadline,
        effective_grace_seconds: effective_grace,
        auto_submit_at,
        resolution_kind,
    } = resolved_postgres_attempt_timing(
        resolution.policy,
        &run,
        authored_deadline,
        authored_grace,
    )?;
    let previous_job = row
        .try_get::<Option<Uuid>, _>("job_id")
        .map_err(map_sqlx_error)?
        .map(JobId::from_uuid);
    let job_state: Option<String> = row.try_get("job_state").map_err(map_sqlx_error)?;
    let immediate = auto_submit_at.is_some_and(|deadline| deadline <= now);

    if immediate || auto_submit_at.is_none() {
        if let (Some(job), Some(state)) = (previous_job, job_state.as_deref())
            && matches!(state, "ready" | "leased")
        {
            let canceled: bool = sqlx::query_scalar("SELECT ple_cancel_attempt_timing_job($1, $2)")
                .bind(tenant.as_uuid())
                .bind(job.as_uuid())
                .fetch_one(&mut **transaction)
                .await
                .map_err(map_sqlx_error)?;
            if !canceled {
                return Err(StoreError::Conflict);
            }
        }
        update_postgres_attempt_timing_row(
            transaction,
            tenant,
            attempt,
            effective_deadline,
            effective_grace,
            auto_submit_at,
            resolution_kind,
            resolution,
            generation,
            None,
        )
        .await?;
        if immediate {
            let updated = sqlx::query(
                "UPDATE question_attempt SET attempt_status = 'auto_submitted', \
                        submitted_at = transaction_timestamp() \
                 WHERE tenant_id = $1 AND attempt_id = $2 AND attempt_status = 'in_progress'",
            )
            .bind(tenant.as_uuid())
            .bind(attempt.as_uuid())
            .execute(&mut **transaction)
            .await
            .map_err(map_sqlx_error)?;
            if updated.rows_affected() != 1 {
                return Err(StoreError::Conflict);
            }
        }
        return Ok(());
    }

    let available_at = auto_submit_at.expect("timed attempt has an auto-submit time");
    let payload = serde_json::to_value(JobPayload::AutoSubmitAttempt {
        attempt,
        timing_generation: generation,
    })
    .map_err(|error| StoreError::InvalidRecord(error.to_string()))?;
    match (previous_job, job_state.as_deref()) {
        (Some(job), Some("ready")) => {
            update_postgres_attempt_timing_row(
                transaction,
                tenant,
                attempt,
                effective_deadline,
                effective_grace,
                auto_submit_at,
                resolution_kind,
                resolution,
                generation,
                Some(job),
            )
            .await?;
            let changed: bool = sqlx::query_scalar(
                "SELECT ple_reschedule_attempt_timing_job($1, $2, $3, $4, \
                    TIMESTAMPTZ 'epoch' + $5::bigint * INTERVAL '1 millisecond')",
            )
            .bind(tenant.as_uuid())
            .bind(job.as_uuid())
            .bind(Option::<Uuid>::None)
            .bind(payload)
            .bind(available_at.as_unix_millis())
            .fetch_one(&mut **transaction)
            .await
            .map_err(map_sqlx_error)?;
            if !changed {
                return Err(StoreError::Conflict);
            }
        }
        (Some(job), Some("leased")) => {
            update_postgres_attempt_timing_row(
                transaction,
                tenant,
                attempt,
                effective_deadline,
                effective_grace,
                auto_submit_at,
                resolution_kind,
                resolution,
                generation,
                Some(job),
            )
            .await?;
        }
        _ => {
            let job = JobId::generate()?;
            sqlx::query(
                "INSERT INTO worker_job \
                 (job_id, tenant_id, payload, state, available_at, max_attempts) \
                 VALUES ($1, $2, $3, 'ready', \
                    TIMESTAMPTZ 'epoch' + $4::bigint * INTERVAL '1 millisecond', 10)",
            )
            .bind(job.as_uuid())
            .bind(tenant.as_uuid())
            .bind(payload)
            .bind(available_at.as_unix_millis())
            .execute(&mut **transaction)
            .await
            .map_err(map_sqlx_error)?;
            update_postgres_attempt_timing_row(
                transaction,
                tenant,
                attempt,
                effective_deadline,
                effective_grace,
                auto_submit_at,
                resolution_kind,
                resolution,
                generation,
                Some(job),
            )
            .await?;
        }
    }
    Ok(())
}

#[cfg(feature = "postgres")]
pub(super) async fn load_postgres_assignment_timing_policy(
    transaction: &mut Transaction<'_, Postgres>,
    tenant: TenantId,
    assignment: AssignmentId,
) -> Result<AssignmentTimingPolicy, StoreError> {
    let row = sqlx::query(
        "SELECT assignment_id, course_id, visible, \
                floor(extract(epoch FROM available_at) * 1000)::bigint AS available_at_millis, \
                floor(extract(epoch FROM due_at) * 1000)::bigint AS due_at_millis, \
                floor(extract(epoch FROM closes_at) * 1000)::bigint AS closes_at_millis, \
                late_submission_policy, time_limit_seconds, auto_submit, attempt_limit, revision \
         FROM assignment WHERE tenant_id = $1 AND assignment_id = $2",
    )
    .bind(tenant.as_uuid())
    .bind(assignment.as_uuid())
    .fetch_optional(&mut **transaction)
    .await
    .map_err(map_sqlx_error)?
    .ok_or(StoreError::NotFound)?;
    Ok(decode_stored_assignment_timing(&row, tenant)?.policy)
}

#[cfg(feature = "postgres")]
pub(super) async fn cancel_postgres_attempt_timing_job(
    transaction: &mut Transaction<'_, Postgres>,
    tenant: TenantId,
    attempt: QuestionAttemptId,
) -> Result<(), StoreError> {
    let row = sqlx::query(
        "SELECT timing.job_id, job.state AS job_state \
         FROM attempt_timing_current AS timing \
         LEFT JOIN worker_job AS job ON job.job_id = timing.job_id \
         WHERE timing.tenant_id = $1 AND timing.attempt_id = $2 FOR UPDATE OF timing",
    )
    .bind(tenant.as_uuid())
    .bind(attempt.as_uuid())
    .fetch_optional(&mut **transaction)
    .await
    .map_err(map_sqlx_error)?
    .ok_or_else(|| {
        StoreError::Unavailable("attempt is missing its current timing row".to_string())
    })?;
    let job = row
        .try_get::<Option<Uuid>, _>("job_id")
        .map_err(map_sqlx_error)?
        .map(JobId::from_uuid);
    let state: Option<String> = row.try_get("job_state").map_err(map_sqlx_error)?;
    if let (Some(job), Some(state)) = (job, state.as_deref())
        && matches!(state, "ready" | "leased")
    {
        let canceled: bool = sqlx::query_scalar("SELECT ple_cancel_attempt_timing_job($1, $2)")
            .bind(tenant.as_uuid())
            .bind(job.as_uuid())
            .fetch_one(&mut **transaction)
            .await
            .map_err(map_sqlx_error)?;
        if !canceled {
            return Err(StoreError::Conflict);
        }
    }
    sqlx::query(
        "UPDATE attempt_timing_current SET job_id = NULL, updated_at = transaction_timestamp() \
         WHERE tenant_id = $1 AND attempt_id = $2",
    )
    .bind(tenant.as_uuid())
    .bind(attempt.as_uuid())
    .execute(&mut **transaction)
    .await
    .map_err(map_sqlx_error)?;
    Ok(())
}

#[cfg(feature = "postgres")]
pub(super) fn timing_policy_grace_seconds(policy: TimingPolicy) -> u32 {
    match policy {
        TimingPolicy::Untimed => 0,
        TimingPolicy::PerQuestion { grace_seconds, .. }
        | TimingPolicy::PerAttempt { grace_seconds, .. } => grace_seconds,
    }
}

#[cfg(feature = "postgres")]
#[allow(clippy::too_many_arguments)]
pub(super) async fn update_postgres_attempt_timing_row(
    transaction: &mut Transaction<'_, Postgres>,
    tenant: TenantId,
    attempt: QuestionAttemptId,
    effective_deadline: Option<ActivityTimestamp>,
    effective_grace_seconds: u32,
    auto_submit_at: Option<ActivityTimestamp>,
    resolution_kind: &str,
    resolution: &crate::ResolvedAssignmentTimingPolicy,
    timing_generation: u64,
    job: Option<JobId>,
) -> Result<(), StoreError> {
    let updated = sqlx::query(
        "UPDATE attempt_timing_current \
         SET effective_deadline = TIMESTAMPTZ 'epoch' + $3::bigint * INTERVAL '1 millisecond', \
             effective_grace_seconds = $4, \
             auto_submit_at = TIMESTAMPTZ 'epoch' + $5::bigint * INTERVAL '1 millisecond', \
             resolution_kind = $6, resolved_visible = $7, \
             resolved_available_at = TIMESTAMPTZ 'epoch' + $8::bigint * INTERVAL '1 millisecond', \
             resolved_due_at = TIMESTAMPTZ 'epoch' + $9::bigint * INTERVAL '1 millisecond', \
             resolved_closes_at = TIMESTAMPTZ 'epoch' + $10::bigint * INTERVAL '1 millisecond', \
             resolved_late_submission_policy = $11, resolved_time_limit_seconds = $12, \
             resolved_attempt_limit = $13, resolution_sources = $14, \
             timing_generation = $15, job_id = $16, \
             updated_at = transaction_timestamp() \
         WHERE tenant_id = $1 AND attempt_id = $2",
    )
    .bind(tenant.as_uuid())
    .bind(attempt.as_uuid())
    .bind(effective_deadline.map(|value| value.as_unix_millis()))
    .bind(i64::from(effective_grace_seconds))
    .bind(auto_submit_at.map(|value| value.as_unix_millis()))
    .bind(resolution_kind)
    .bind(resolution.policy.visible)
    .bind(
        resolution
            .policy
            .available_at
            .map(|value| value.as_unix_millis()),
    )
    .bind(resolution.policy.due_at.map(|value| value.as_unix_millis()))
    .bind(
        resolution
            .policy
            .closes_at
            .map(|value| value.as_unix_millis()),
    )
    .bind(late_submission_policy_name(
        resolution.policy.late_submission,
    ))
    .bind(resolution.policy.time_limit_seconds.map(i64::from))
    .bind(resolution.policy.attempt_limit.map(i64::from))
    .bind(
        serde_json::to_value(&resolution.contributors)
            .map_err(|error| StoreError::InvalidRecord(error.to_string()))?,
    )
    .bind(i64::try_from(timing_generation).map_err(|_| StoreError::Conflict)?)
    .bind(job.map(JobId::as_uuid))
    .execute(&mut **transaction)
    .await
    .map_err(map_sqlx_error)?;
    if updated.rows_affected() != 1 {
        return Err(StoreError::Conflict);
    }
    Ok(())
}

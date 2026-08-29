//! PostgreSQL attempt-clock helpers over sealed effective-policy receipts.

use question_model::{ActivityTimestamp, CourseMembershipId, run_policy::TimingPolicy};

use super::*;

pub(super) async fn load_postgres_course_group_members(
    transaction: &mut Transaction<'_, Postgres>,
    tenant: TenantId,
    group: CourseGroupId,
) -> Result<Vec<CourseMembershipId>, StoreError> {
    sqlx::query_scalar::<_, Uuid>(
        "SELECT course_membership_id FROM course_group_member WHERE tenant_id=$1 AND course_group_id=$2 ORDER BY course_membership_id",
    )
    .bind(tenant.as_uuid())
    .bind(group.as_uuid())
    .fetch_all(&mut **transaction)
    .await
    .map_err(map_sqlx_error)
    .map(|ids| ids.into_iter().map(CourseMembershipId::from_uuid).collect())
}

pub(super) async fn cancel_postgres_effective_policy_job(
    transaction: &mut Transaction<'_, Postgres>,
    tenant: TenantId,
    attempt: QuestionAttemptId,
) -> Result<(), StoreError> {
    let job = sqlx::query_scalar::<_, Option<Uuid>>(
        "SELECT job_id FROM attempt_effective_policy_current WHERE tenant_id=$1 AND attempt_id=$2 FOR UPDATE",
    )
    .bind(tenant.as_uuid())
    .bind(attempt.as_uuid())
    .fetch_optional(&mut **transaction)
    .await
    .map_err(map_sqlx_error)?
    .ok_or_else(|| StoreError::Unavailable("attempt is missing its effective-policy pointer".to_string()))?
    .map(JobId::from_uuid);
    if let Some(job) = job {
        let cancelled: bool = sqlx::query_scalar("SELECT ple_cancel_attempt_timing_job($1, $2)")
            .bind(tenant.as_uuid())
            .bind(job.as_uuid())
            .fetch_one(&mut **transaction)
            .await
            .map_err(map_sqlx_error)?;
        if !cancelled {
            return Err(StoreError::Conflict);
        }
    }
    sqlx::query("UPDATE attempt_effective_policy_current SET job_id=NULL, updated_at=transaction_timestamp() WHERE tenant_id=$1 AND attempt_id=$2")
        .bind(tenant.as_uuid()).bind(attempt.as_uuid()).execute(&mut **transaction).await.map_err(map_sqlx_error)?;
    Ok(())
}

pub(super) fn timing_policy_grace_seconds(policy: TimingPolicy) -> u32 {
    match policy {
        TimingPolicy::Untimed => 0,
        TimingPolicy::PerQuestion { grace_seconds, .. }
        | TimingPolicy::PerAttempt { grace_seconds, .. } => grace_seconds,
    }
}

pub(super) struct ResolvedPostgresAttemptTiming {
    pub(super) effective_deadline: Option<ActivityTimestamp>,
    pub(super) effective_grace_seconds: u32,
    pub(super) auto_submit_at: Option<ActivityTimestamp>,
}

pub(super) fn resolved_postgres_attempt_timing(
    policy: &domain::effective_assignment_policy::EffectiveAssignmentPolicy,
    run_started_at: ActivityTimestamp,
    authored_deadline: Option<ActivityTimestamp>,
    authored_grace_seconds: u32,
) -> Result<ResolvedPostgresAttemptTiming, StoreError> {
    let mut resolved = authored_deadline.map(|deadline| (deadline, authored_grace_seconds));
    let mut consider = |deadline, grace| {
        if resolved.is_none_or(|current| (deadline, grace) < current) {
            resolved = Some((deadline, grace));
        }
    };
    if let Some(limit) = policy.time_limit_seconds.value {
        consider(
            super::runs::add_seconds(run_started_at, limit.get(), "assignment time limit")?,
            0,
        );
    }
    if policy.late_submission.value == LateSubmissionPolicy::Reject
        && let Some(due) = policy.due_at.value
    {
        consider(due, 0);
    }
    if let Some(closes) = policy.closes_at.value {
        consider(closes, 0);
    }
    let auto_submit_at = resolved
        .map(|(deadline, grace)| {
            super::runs::add_seconds(deadline, grace, "attempt auto-submit deadline")
        })
        .transpose()?;
    Ok(ResolvedPostgresAttemptTiming {
        effective_deadline: resolved.map(|(deadline, _)| deadline),
        effective_grace_seconds: resolved.map_or(0, |(_, grace)| grace),
        auto_submit_at,
    })
}

pub(super) fn late_submission_policy_name(value: LateSubmissionPolicy) -> &'static str {
    match value {
        LateSubmissionPolicy::Accept => "accept",
        LateSubmissionPolicy::Reject => "reject",
        LateSubmissionPolicy::MarkLate => "mark_late",
    }
}

//! Immutable definition-derived timing evidence for an issued attempt.

use question_model::{ActivityTimestamp, AttemptTimerRecord, run_policy::TimingPolicy};
use sqlx::{Postgres, Row, Transaction};

use crate::{IssueQuestionAttemptCommand, StoreError, TenantId};

use super::super::connection::map_sqlx_error;

pub(in crate::postgres) struct AuthoredAttemptTiming {
    pub(in crate::postgres) deadline: Option<ActivityTimestamp>,
    pub(in crate::postgres) grace_seconds: u32,
}

pub(in crate::postgres) fn issued_timer(
    issued_at: ActivityTimestamp,
    run_started_at: ActivityTimestamp,
    policy: TimingPolicy,
) -> Result<AttemptTimerRecord, StoreError> {
    let deadline = match policy {
        TimingPolicy::Untimed => None,
        TimingPolicy::PerQuestion { seconds, .. } => {
            Some(add_seconds(issued_at, seconds, "question deadline")?)
        }
        TimingPolicy::PerAttempt { seconds, .. } => {
            let deadline = add_seconds(run_started_at, seconds, "run deadline")?;
            if deadline < issued_at {
                return Err(StoreError::TimedOut);
            }
            Some(deadline)
        }
    };
    Ok(AttemptTimerRecord {
        issued_at,
        deadline,
        submitted_at: None,
    })
}

pub(in crate::postgres) fn add_seconds(
    timestamp: ActivityTimestamp,
    seconds: u32,
    description: &str,
) -> Result<ActivityTimestamp, StoreError> {
    timestamp
        .as_unix_millis()
        .checked_add(i64::from(seconds) * 1_000)
        .map(ActivityTimestamp::from_unix_millis)
        .ok_or_else(|| StoreError::InvalidRecord(format!("{description} overflow")))
}

pub(in crate::postgres) async fn validate_postgres_assignment_position(
    transaction: &mut Transaction<'_, Postgres>,
    tenant: TenantId,
    command: &IssueQuestionAttemptCommand,
) -> Result<(), StoreError> {
    let position = i32::try_from(command.assignment_position)
        .map_err(|_| StoreError::InvalidRecord("assignment position is too large".to_string()))?;
    let row = sqlx::query("SELECT problem_id, version_id FROM assignment_run_item WHERE tenant_id=$1 AND run_id=$2 AND issued_position=$3")
        .bind(tenant.as_uuid()).bind(command.run.as_uuid()).bind(position)
        .fetch_optional(&mut **transaction).await.map_err(map_sqlx_error)?
        .ok_or_else(|| StoreError::InvalidRecord("question position is outside the run".to_string()))?;
    let problem: sqlx::types::Uuid = row.try_get("problem_id").map_err(map_sqlx_error)?;
    let version: sqlx::types::Uuid = row.try_get("version_id").map_err(map_sqlx_error)?;
    (problem == command.problem.as_uuid() && version == command.question_version.as_uuid())
        .then_some(())
        .ok_or_else(|| {
            StoreError::InvalidRecord(
                "question identity does not match its run position".to_string(),
            )
        })
}

/// Fail closed when private timing evidence is malformed.  Its absence never
/// causes policy work to reread catalog or mutable presentation state.
pub(in crate::postgres) fn decode_authored_attempt_timing(
    row: &sqlx::postgres::PgRow,
) -> Result<AuthoredAttemptTiming, StoreError> {
    let deadline_millis: Option<i64> = row
        .try_get("authored_timing_deadline")
        .map_err(map_sqlx_error)?;
    let grace: i64 = row
        .try_get("authored_timing_grace_seconds")
        .map_err(map_sqlx_error)?;
    let grace_seconds = u32::try_from(grace).map_err(|_| {
        StoreError::Unavailable("stored authored timing grace is invalid".to_string())
    })?;
    if deadline_millis.is_none() && grace_seconds != 0 {
        return Err(StoreError::Unavailable(
            "stored authored timing shape is invalid".to_string(),
        ));
    }
    Ok(AuthoredAttemptTiming {
        deadline: deadline_millis.map(ActivityTimestamp::from_unix_millis),
        grace_seconds,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn issue_timer_anchors_each_timing_policy_at_its_authoritative_event() {
        let issued = ActivityTimestamp::from_unix_millis(20_000);
        let run_started = ActivityTimestamp::from_unix_millis(10_000);

        let untimed = issued_timer(issued, run_started, TimingPolicy::Untimed)
            .expect("untimed issue is valid");
        assert_eq!(untimed.deadline, None);

        let per_question = issued_timer(
            issued,
            run_started,
            TimingPolicy::PerQuestion {
                seconds: 30,
                grace_seconds: 7,
            },
        )
        .expect("per-question issue is valid");
        assert_eq!(
            per_question.deadline,
            Some(ActivityTimestamp::from_unix_millis(50_000))
        );

        let per_attempt = issued_timer(
            issued,
            run_started,
            TimingPolicy::PerAttempt {
                seconds: 30,
                grace_seconds: 7,
            },
        )
        .expect("per-attempt issue is valid");
        assert_eq!(
            per_attempt.deadline,
            Some(ActivityTimestamp::from_unix_millis(40_000))
        );
    }
}

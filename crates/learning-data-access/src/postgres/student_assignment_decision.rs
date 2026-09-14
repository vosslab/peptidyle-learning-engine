//! Shared PostgreSQL row decoder for the browser-owned Assignment decision.

use browser_api_contract::student_assignment_decision::{
    AssignmentStartDecision, StudentAssignmentDecisionSummary,
};
use question_model::{AccountTimeZone, LateWorkRule, Timestamp};
use sqlx::Row;

use super::connection::map_sqlx_error;
use crate::StoreError;

pub(super) fn decode(
    row: &sqlx::postgres::PgRow,
) -> Result<StudentAssignmentDecisionSummary, StoreError> {
    // ASVS 1.5.2 and 14.2.6: positively decode the closed browser DTO from
    // effective values only; private row identity never enters this type.
    let start_decision = start_decision(
        &row.try_get::<String, _>("start_decision")
            .map_err(map_sqlx_error)?,
    )?;
    let public_reason = public_reason(start_decision).map(str::to_string);
    Ok(StudentAssignmentDecisionSummary {
        available_at: optional_timestamp(row, "available_at_millis")?,
        due_at: optional_timestamp(row, "due_at_millis")?,
        closes_at: optional_timestamp(row, "closes_at_millis")?,
        time_limit_seconds: optional_positive_integer(row, "time_limit_seconds")?,
        attempt_limit: optional_positive_integer(row, "attempt_limit")?,
        late_work_rule: late_work_rule(
            &row.try_get::<String, _>("late_work_rule")
                .map_err(map_sqlx_error)?,
        )?,
        display_time_zone: AccountTimeZone::parse(
            &row.try_get::<String, _>("display_time_zone")
                .map_err(map_sqlx_error)?,
        )
        .map_err(|_| invalid("Student display time zone"))?,
        evaluated_at: Timestamp::from_unix_millis(
            row.try_get("evaluated_at_millis").map_err(map_sqlx_error)?,
        ),
        start_decision,
        public_reason,
    })
}

fn optional_timestamp(
    row: &sqlx::postgres::PgRow,
    column: &str,
) -> Result<Option<Timestamp>, StoreError> {
    Ok(row
        .try_get::<Option<i64>, _>(column)
        .map_err(map_sqlx_error)?
        .map(Timestamp::from_unix_millis))
}

fn optional_positive_integer(
    row: &sqlx::postgres::PgRow,
    column: &str,
) -> Result<Option<u32>, StoreError> {
    row.try_get::<Option<i32>, _>(column)
        .map_err(map_sqlx_error)?
        .map(u32::try_from)
        .transpose()
        .map_err(|_| invalid(column))
        .and_then(|value| match value {
            Some(0) => Err(invalid(column)),
            value => Ok(value),
        })
}

fn start_decision(value: &str) -> Result<AssignmentStartDecision, StoreError> {
    match value {
        "may_start" => Ok(AssignmentStartDecision::MayStart),
        "not_yet_available" => Ok(AssignmentStartDecision::NotYetAvailable),
        "closed" => Ok(AssignmentStartDecision::Closed),
        "attempt_limit_reached" => Ok(AssignmentStartDecision::AttemptLimitReached),
        "late_work_refused" => Ok(AssignmentStartDecision::LateWorkRefused),
        _ => Err(invalid("Assignment Start Decision")),
    }
}

fn late_work_rule(value: &str) -> Result<LateWorkRule, StoreError> {
    match value {
        "accept" => Ok(LateWorkRule::Accept),
        "mark_late" => Ok(LateWorkRule::MarkLate),
        "reject" => Ok(LateWorkRule::Reject),
        _ => Err(invalid("late-work rule")),
    }
}

fn public_reason(decision: AssignmentStartDecision) -> Option<&'static str> {
    match decision {
        AssignmentStartDecision::MayStart => None,
        AssignmentStartDecision::NotYetAvailable => Some("This Assignment is not yet available."),
        AssignmentStartDecision::Closed => Some("This Assignment is closed for new work."),
        AssignmentStartDecision::AttemptLimitReached => {
            Some("The allowed number of Assignment Attempts has been reached.")
        }
        AssignmentStartDecision::LateWorkRefused => {
            Some("New work is not available under the late-work policy.")
        }
    }
}

fn invalid(name: &str) -> StoreError {
    StoreError::InvalidRecord(format!("{name} is invalid"))
}

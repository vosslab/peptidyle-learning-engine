//! Student Assignment Access storage projection.

use super::{
    assignment_delivery::{PostgresLiveAssignmentDeliveryStore, optional_positive_i32},
    connection::map_sqlx_error,
};
use crate::{
    LiveAssignmentAccess, LiveAssignmentPreviousAttempt, LiveAssignmentStartDecision,
    SessionTokenHash, StoreError,
};
use question_model::{AssignmentReference, CourseInstanceReference};
use sqlx::Row;

pub(super) async fn read(
    store: &PostgresLiveAssignmentDeliveryStore,
    token: SessionTokenHash,
    course: CourseInstanceReference,
    assignment: AssignmentReference,
) -> Result<LiveAssignmentAccess, StoreError> {
    let mut tx = store.begin(token).await?;
    let row = sqlx::query(
        "SELECT start_decision, assignment_title, question_count, points_possible, \
         assignment_attempt_time_limit_seconds, previous_attempts \
         FROM ple_api.live_demo_assignment_access($1, $2)",
    )
    .bind(i64::from(course.number()))
    .bind(i64::from(assignment.number()))
    .fetch_one(&mut *tx)
    .await
    .map_err(map_sqlx_error)?;
    let value: String = row.try_get("start_decision").map_err(map_sqlx_error)?;
    let title: String = row.try_get("assignment_title").map_err(map_sqlx_error)?;
    let question_count = u32::try_from(
        row.try_get::<i32, _>("question_count")
            .map_err(map_sqlx_error)?,
    )
    .map_err(|_| StoreError::InvalidRecord("Question count is invalid".to_string()))?;
    let points_possible: f64 = row.try_get("points_possible").map_err(map_sqlx_error)?;
    if !points_possible.is_finite() || points_possible < 0.0 {
        return Err(StoreError::InvalidRecord(
            "Assignment points possible is invalid".to_string(),
        ));
    }
    let time_limit_seconds = optional_positive_i32(
        &row,
        "assignment_attempt_time_limit_seconds",
        "Assignment time limit",
    )?;
    let previous_attempts: Vec<LiveAssignmentPreviousAttempt> = serde_json::from_value(
        row.try_get("previous_attempts").map_err(map_sqlx_error)?,
    )
    .map_err(|_| StoreError::InvalidRecord("Assignment Attempt history is invalid".to_string()))?;
    if previous_attempts.iter().any(|attempt| match attempt.score {
        Some(score) => {
            !score.points_earned.is_finite()
                || !score.points_possible.is_finite()
                || score.points_earned < 0.0
                || score.points_possible < score.points_earned
        }
        None => false,
    }) {
        return Err(StoreError::InvalidRecord(
            "Assignment Attempt score is invalid".to_string(),
        ));
    }
    let active_assignment_attempt =
        PostgresLiveAssignmentDeliveryStore::optional_active_attempt_reference(
            &mut tx, course, assignment,
        )
        .await?;
    tx.commit().await.map_err(map_sqlx_error)?;
    Ok(LiveAssignmentAccess {
        start_decision: decision(&value)?,
        active_assignment_attempt,
        title,
        question_count,
        points_possible,
        time_limit_seconds,
        previous_attempts,
    })
}

fn decision(value: &str) -> Result<LiveAssignmentStartDecision, StoreError> {
    match value {
        "may_start" => Ok(LiveAssignmentStartDecision::MayStart),
        "not_yet_available" => Ok(LiveAssignmentStartDecision::NotYetAvailable),
        "closed" => Ok(LiveAssignmentStartDecision::Closed),
        "attempt_limit_reached" => Ok(LiveAssignmentStartDecision::AttemptLimitReached),
        "late_work_refused" => Ok(LiveAssignmentStartDecision::LateWorkRefused),
        _ => Err(StoreError::InvalidRecord(
            "Assignment Access decision is invalid".to_string(),
        )),
    }
}

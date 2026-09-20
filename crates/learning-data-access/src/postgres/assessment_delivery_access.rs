//! Student Assessment Access storage projection.

use super::{assessment_delivery::PostgresLiveAssessmentDeliveryStore, connection::map_sqlx_error};
use crate::{LiveAssessmentAccess, LiveAssessmentPreviousAttempt, SessionTokenHash, StoreError};
use question_model::{AssessmentId, AssessmentType, CourseInstanceId};
use sqlx::Row;

pub(super) async fn read(
    store: &PostgresLiveAssessmentDeliveryStore,
    token: SessionTokenHash,
    course_instance_id: CourseInstanceId,
    assessment_id: AssessmentId,
) -> Result<LiveAssessmentAccess, StoreError> {
    let mut tx = store.begin(token).await?;
    let row = sqlx::query(
        "SELECT start_decision, assessment_title, assessment_type, question_count, points_possible, \
         assessment_attempt_time_limit_seconds AS time_limit_seconds, \
         assessment_attempt_limit AS attempt_limit, \
         late_work_rule, display_time_zone, \
         CASE WHEN available_at IS NULL THEN NULL ELSE \
             floor(extract(epoch FROM available_at) * 1000)::bigint END AS available_at_millis, \
         CASE WHEN due_at IS NULL THEN NULL ELSE \
             floor(extract(epoch FROM due_at) * 1000)::bigint END AS due_at_millis, \
         CASE WHEN closes_at IS NULL THEN NULL ELSE \
             floor(extract(epoch FROM closes_at) * 1000)::bigint END AS closes_at_millis, \
         floor(extract(epoch FROM evaluated_at) * 1000)::bigint AS evaluated_at_millis, \
         previous_assessment_attempts \
         FROM ple_api.read_student_assessment_access($1, $2)",
    )
    .bind(course_instance_id.as_string())
    .bind(assessment_id.as_string())
    .fetch_one(&mut *tx)
    .await
    .map_err(map_sqlx_error)?;
    let decision = super::student_assessment_decision::decode(&row)?;
    let title: String = row.try_get("assessment_title").map_err(map_sqlx_error)?;
    let assessment_type = serde_json::from_value::<AssessmentType>(serde_json::Value::String(
        row.try_get("assessment_type").map_err(map_sqlx_error)?,
    ))
    .map_err(|_| StoreError::InvalidRecord("Assessment Type is invalid".to_string()))?;
    let question_count = u32::try_from(
        row.try_get::<i32, _>("question_count")
            .map_err(map_sqlx_error)?,
    )
    .map_err(|_| StoreError::InvalidRecord("Question count is invalid".to_string()))?;
    let points_possible: f64 = row.try_get("points_possible").map_err(map_sqlx_error)?;
    if !points_possible.is_finite() || points_possible < 0.0 {
        return Err(StoreError::InvalidRecord(
            "Assessment points possible is invalid".to_string(),
        ));
    }
    let previous_attempts: Vec<LiveAssessmentPreviousAttempt> = serde_json::from_value(
        row.try_get("previous_assessment_attempts")
            .map_err(map_sqlx_error)?,
    )
    .map_err(|_| StoreError::InvalidRecord("Assessment Attempt history is invalid".to_string()))?;
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
            "Assessment Attempt score is invalid".to_string(),
        ));
    }
    let active_assessment_attempt =
        PostgresLiveAssessmentDeliveryStore::optional_active_attempt_id(
            &mut tx,
            course_instance_id,
            assessment_id,
        )
        .await?;
    tx.commit().await.map_err(map_sqlx_error)?;
    Ok(LiveAssessmentAccess {
        decision,
        active_assessment_attempt_id: active_assessment_attempt,
        title,
        assessment_type,
        question_count,
        points_possible,
        previous_attempts,
    })
}

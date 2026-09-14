//! PostgreSQL preparation and commit adapter for whole Assignment Attempt finalization.

use super::{assignment_delivery::PostgresLiveAssignmentDeliveryStore, connection::map_sqlx_error};
use crate::{
    SessionTokenHash, StoreError, StudentAssignmentAttemptFinalization,
    StudentAssignmentAttemptFinalizationBackend, StudentAssignmentAttemptFinalizationEvaluation,
    StudentAssignmentAttemptFinalizationKind, StudentAssignmentAttemptFinalizationPreparation,
    StudentAssignmentAttemptFinalizationPreparationOutcome,
    StudentAssignmentAttemptFinalizationSource,
};
use question_model::{AssignmentAttemptReference, Timestamp};
use sqlx::Row;
use uuid::Uuid;

pub(super) async fn prepare(
    store: &PostgresLiveAssignmentDeliveryStore,
    token: SessionTokenHash,
    assignment_attempt: AssignmentAttemptReference,
) -> Result<StudentAssignmentAttemptFinalizationPreparationOutcome, StoreError> {
    let mut tx = store.begin(token).await?;
    let rows = sqlx::query(
        "SELECT preparation_state, finalization_kind, missing_positions, \
                points_earned, points_possible, question_attempt_id, saved_at_millis, \
                question_id, revision_number, source_object_id::text AS source_object_id, \
                source_object_checksum, question_seed::text AS question_seed, student_response, \
                backend, webwork_pg_path \
         FROM ple_api.prepare_student_assignment_attempt_finalization($1)",
    )
    .bind(i64::from(assignment_attempt.number()))
    .fetch_all(&mut *tx)
    .await
    .map_err(map_sqlx_error)?;
    let first = rows.first().ok_or_else(|| {
        StoreError::InvalidRecord("Assignment Attempt preparation is empty".to_string())
    })?;
    let state: String = first.try_get("preparation_state").map_err(map_sqlx_error)?;
    let missing_positions = first
        .try_get::<Vec<i32>, _>("missing_positions")
        .map_err(map_sqlx_error)?
        .into_iter()
        .map(|position| {
            u32::try_from(position)
                .ok()
                .filter(|position| *position > 0)
                .ok_or_else(|| {
                    StoreError::InvalidRecord(
                        "Missing Student response position is invalid".to_string(),
                    )
                })
        })
        .collect::<Result<Vec<_>, StoreError>>()?;
    let preparation = match state.as_str() {
        "already_submitted" if missing_positions.is_empty() => {
            StudentAssignmentAttemptFinalizationPreparationOutcome::AlreadySubmitted {
                score: optional_score_from_row(first)?,
            }
        }
        "missing_responses" if !missing_positions.is_empty() => {
            StudentAssignmentAttemptFinalizationPreparationOutcome::MissingResponses {
                positions: missing_positions,
            }
        }
        "ready" if missing_positions.is_empty() => {
            let kind = finalization_kind(
                &first
                    .try_get::<String, _>("finalization_kind")
                    .map_err(map_sqlx_error)?,
            )?;
            let saved_responses = rows
                .iter()
                .filter_map(|row| finalization_source_from_row(row).transpose())
                .collect::<Result<Vec<_>, _>>()?;
            StudentAssignmentAttemptFinalizationPreparationOutcome::Ready(
                StudentAssignmentAttemptFinalizationPreparation {
                    kind,
                    saved_responses,
                },
            )
        }
        _ => {
            return Err(StoreError::InvalidRecord(
                "Assignment Attempt preparation is invalid".to_string(),
            ));
        }
    };
    tx.commit().await.map_err(map_sqlx_error)?;
    Ok(preparation)
}

pub(super) async fn commit(
    store: &PostgresLiveAssignmentDeliveryStore,
    token: SessionTokenHash,
    assignment_attempt: AssignmentAttemptReference,
    preparation: StudentAssignmentAttemptFinalizationPreparation,
    evaluations: Vec<StudentAssignmentAttemptFinalizationEvaluation>,
) -> Result<StudentAssignmentAttemptFinalization, StoreError> {
    validate_evaluations(&preparation, &evaluations)?;
    let evaluation_payload = serde_json::Value::Array(
        evaluations
            .iter()
            .map(|evaluation| {
                serde_json::json!({
                    "question_attempt_id": evaluation.question_attempt_id,
                    "saved_at_millis": evaluation.saved_at.as_unix_millis(),
                    "student_response": evaluation.student_response,
                    "normalized_credit": evaluation.normalized_credit,
                })
            })
            .collect(),
    );
    let mut tx = store.begin(token).await?;
    let row = sqlx::query(
        "SELECT points_earned, points_possible \
         FROM ple_api.commit_student_assignment_attempt_finalization($1, $2, $3)",
    )
    .bind(i64::from(assignment_attempt.number()))
    .bind(finalization_kind_name(preparation.kind))
    .bind(evaluation_payload)
    .fetch_one(&mut *tx)
    .await
    .map_err(map_sqlx_error)?;
    let score = score_from_row(&row)?;
    tx.commit().await.map_err(map_sqlx_error)?;
    Ok(StudentAssignmentAttemptFinalization::Submitted { score: Some(score) })
}

fn validate_evaluations(
    preparation: &StudentAssignmentAttemptFinalizationPreparation,
    evaluations: &[StudentAssignmentAttemptFinalizationEvaluation],
) -> Result<(), StoreError> {
    if evaluations.len() != preparation.saved_responses.len()
        || evaluations.iter().any(|evaluation| {
            !evaluation.normalized_credit.is_finite()
                || !(0.0..=1.0).contains(&evaluation.normalized_credit)
        })
    {
        return Err(StoreError::InvalidRecord(
            "Assignment Attempt evaluation is invalid".to_string(),
        ));
    }
    let expected = preparation
        .saved_responses
        .iter()
        .map(|source| source.question_attempt_id)
        .collect::<std::collections::BTreeSet<_>>();
    let actual = evaluations
        .iter()
        .map(|evaluation| evaluation.question_attempt_id)
        .collect::<std::collections::BTreeSet<_>>();
    if expected != actual {
        return Err(StoreError::InvalidRecord(
            "Assignment Attempt evaluation does not match its snapshot".to_string(),
        ));
    }
    Ok(())
}

fn finalization_kind(value: &str) -> Result<StudentAssignmentAttemptFinalizationKind, StoreError> {
    match value {
        "student" => Ok(StudentAssignmentAttemptFinalizationKind::Student),
        "deadline" => Ok(StudentAssignmentAttemptFinalizationKind::Deadline),
        _ => Err(StoreError::InvalidRecord(
            "Assignment Attempt finalization kind is invalid".to_string(),
        )),
    }
}

const fn finalization_kind_name(value: StudentAssignmentAttemptFinalizationKind) -> &'static str {
    match value {
        StudentAssignmentAttemptFinalizationKind::Student => "student",
        StudentAssignmentAttemptFinalizationKind::Deadline => "deadline",
    }
}

fn score_from_row(
    row: &sqlx::postgres::PgRow,
) -> Result<crate::LiveAssignmentAttemptScore, StoreError> {
    let points_earned = finite_nonnegative(
        row.try_get("points_earned").map_err(map_sqlx_error)?,
        "Assignment Attempt points earned",
    )?;
    let points_possible = finite_nonnegative(
        row.try_get("points_possible").map_err(map_sqlx_error)?,
        "Assignment Attempt points possible",
    )?;
    if points_earned > points_possible {
        return Err(StoreError::InvalidRecord(
            "Assignment Attempt score ordering is invalid".to_string(),
        ));
    }
    Ok(crate::LiveAssignmentAttemptScore {
        points_earned,
        points_possible,
    })
}

fn optional_score_from_row(
    row: &sqlx::postgres::PgRow,
) -> Result<Option<crate::LiveAssignmentAttemptScore>, StoreError> {
    let points_earned = row
        .try_get::<Option<f64>, _>("points_earned")
        .map_err(map_sqlx_error)?;
    let points_possible = row
        .try_get::<Option<f64>, _>("points_possible")
        .map_err(map_sqlx_error)?;
    match (points_earned, points_possible) {
        (None, None) => Ok(None),
        (Some(_), Some(_)) => score_from_row(row).map(Some),
        _ => Err(StoreError::InvalidRecord(
            "Assignment Attempt score is incomplete".to_string(),
        )),
    }
}

pub(super) fn finalization_source_from_row(
    row: &sqlx::postgres::PgRow,
) -> Result<Option<StudentAssignmentAttemptFinalizationSource>, StoreError> {
    let Some(question_attempt_id) = row
        .try_get::<Option<Uuid>, _>("question_attempt_id")
        .map_err(map_sqlx_error)?
    else {
        return Ok(None);
    };
    let saved_at = Timestamp::from_unix_millis(
        row.try_get::<i64, _>("saved_at_millis")
            .map_err(map_sqlx_error)?,
    );
    let question_id = row
        .try_get::<String, _>("question_id")
        .map_err(map_sqlx_error)?
        .parse()
        .map_err(|_| {
            StoreError::InvalidRecord("Finalization Question ID is invalid".to_string())
        })?;
    let revision_number = u32::try_from(
        row.try_get::<i32, _>("revision_number")
            .map_err(map_sqlx_error)?,
    )
    .ok()
    .filter(|value| *value > 0)
    .ok_or_else(|| StoreError::InvalidRecord("Finalization revision is invalid".to_string()))?;
    let question_seed = row
        .try_get::<String, _>("question_seed")
        .map_err(map_sqlx_error)?
        .parse()
        .map_err(|_| StoreError::InvalidRecord("Finalization seed is invalid".to_string()))?;
    let student_response = serde_json::from_value(
        row.try_get::<serde_json::Value, _>("student_response")
            .map_err(map_sqlx_error)?,
    )
    .map_err(|_| StoreError::InvalidRecord("Finalization response is invalid".to_string()))?;
    let backend = match row
        .try_get::<String, _>("backend")
        .map_err(map_sqlx_error)?
        .as_str()
    {
        "ple" => StudentAssignmentAttemptFinalizationBackend::Ple,
        "webwork" => StudentAssignmentAttemptFinalizationBackend::Webwork {
            pg_path: row
                .try_get::<Option<String>, _>("webwork_pg_path")
                .map_err(map_sqlx_error)?
                .filter(|path| !path.is_empty())
                .ok_or_else(|| {
                    StoreError::InvalidRecord("Finalization WeBWorK path is invalid".to_string())
                })?,
        },
        _ => {
            return Err(StoreError::InvalidRecord(
                "Finalization Question Backend is invalid".to_string(),
            ));
        }
    };
    Ok(Some(StudentAssignmentAttemptFinalizationSource {
        question_attempt_id,
        saved_at,
        question_id,
        revision_number,
        source_object_id: row.try_get("source_object_id").map_err(map_sqlx_error)?,
        source_object_checksum: row
            .try_get("source_object_checksum")
            .map_err(map_sqlx_error)?,
        question_seed,
        student_response,
        backend,
    }))
}

fn finite_nonnegative(value: f64, label: &str) -> Result<f64, StoreError> {
    if value.is_finite() && value >= 0.0 {
        Ok(value)
    } else {
        Err(StoreError::InvalidRecord(format!("{label} is invalid")))
    }
}

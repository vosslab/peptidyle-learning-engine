//! Selected completed-Attempt history storage projection.

use std::str::FromStr;

use super::{assignment_delivery::PostgresLiveAssignmentDeliveryStore, connection::map_sqlx_error};
use crate::{
    LiveAssignmentPreviousAttemptState, SessionTokenHash, StoreError,
    StudentAssignmentAttemptHistory, StudentAssignmentAttemptHistoryAssignment,
    StudentAssignmentAttemptHistoryCourse, StudentAssignmentAttemptHistoryEvidence,
    StudentAssignmentAttemptHistoryQuestion,
};
use question_model::{
    AssignmentAttemptReference, AssignmentReference, CourseInstanceReference, CourseTheme,
    GradingResult, QuestionId, QuestionRevisionNumber, QuestionRevisionReference, StudentFeedback,
    StudentFeedbackReleaseRule, Timestamp,
};
use serde::Deserialize;
use sqlx::Row;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct StoredQuestion {
    position: u32,
    question_id: String,
    revision_number: u32,
    response_state: LiveAssignmentPreviousAttemptState,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct StoredGradingResult {
    position: u32,
    correct: bool,
    points_earned: f64,
    points_possible: f64,
}

pub(super) async fn read(
    store: &PostgresLiveAssignmentDeliveryStore,
    token: SessionTokenHash,
    assignment_attempt: AssignmentAttemptReference,
) -> Result<StudentAssignmentAttemptHistoryEvidence, StoreError> {
    let mut tx = store.begin(token).await?;
    // ASVS 8.2.2 and 8.3.1: the parameter is only a public reference; the
    // SECURITY DEFINER reader re-checks exact Student ownership and membership.
    let row = sqlx::query(
        "SELECT course_reference_number, course_short_name, course_long_name, course_theme, \
         assignment_reference_number, assignment_title, attempt_number, state, questions, \
         feedback_rule, due_at_millis, closes_at_millis, submitted_at_millis, evaluated_at_millis, \
         grading_is_current, grading_results FROM ple_api.read_student_assignment_attempt_history($1)",
    )
    .bind(i64::from(assignment_attempt.number()))
    .fetch_optional(&mut *tx)
    .await
    .map_err(map_sqlx_error)?
    .ok_or(StoreError::NotFound)?;
    let number: i64 = row
        .try_get("assignment_reference_number")
        .map_err(map_sqlx_error)?;
    let assignment =
        AssignmentReference::new(u64::try_from(number).map_err(|_| {
            StoreError::InvalidRecord("Assignment reference is invalid".to_string())
        })?)
        .ok_or_else(|| StoreError::InvalidRecord("Assignment reference is invalid".to_string()))?;
    let attempt_number = u32::try_from(
        row.try_get::<i32, _>("attempt_number")
            .map_err(map_sqlx_error)?,
    )
    .map_err(|_| StoreError::InvalidRecord("Assignment Attempt number is invalid".to_string()))?;
    let state = state(row.try_get("state").map_err(map_sqlx_error)?)?;
    let stored_questions = serde_json::from_value::<Vec<StoredQuestion>>(
        row.try_get("questions").map_err(map_sqlx_error)?,
    )
    .map_err(|_| {
        StoreError::InvalidRecord("Assignment Attempt positions are invalid".to_string())
    })?;
    let questions = stored_questions
        .into_iter()
        .map(decode_question)
        .collect::<Result<Vec<_>, StoreError>>()?;
    let feedback_rule = serde_json::from_value::<StudentFeedbackReleaseRule>(
        row.try_get("feedback_rule").map_err(map_sqlx_error)?,
    )
    .map_err(|_| {
        StoreError::InvalidRecord("Student Feedback Release Rule is invalid".to_string())
    })?;
    let grading_is_current = row.try_get("grading_is_current").map_err(map_sqlx_error)?;
    let grading_results = decode_results(
        row.try_get("grading_results").map_err(map_sqlx_error)?,
        &questions,
        grading_is_current,
    )?;
    let history = StudentAssignmentAttemptHistory {
        assignment_attempt,
        attempt_number,
        course: StudentAssignmentAttemptHistoryCourse {
            reference: reference(
                row.try_get("course_reference_number")
                    .map_err(map_sqlx_error)?,
                "Course reference",
                CourseInstanceReference::new,
            )?,
            short_name: name(
                row.try_get("course_short_name").map_err(map_sqlx_error)?,
                "Course short name",
            )?,
            long_name: name(
                row.try_get("course_long_name").map_err(map_sqlx_error)?,
                "Course long name",
            )?,
            theme: CourseTheme::from_str(
                &row.try_get::<String, _>("course_theme")
                    .map_err(map_sqlx_error)?,
            )
            .map_err(|_| StoreError::InvalidRecord("Course theme is invalid".to_string()))?,
        },
        assignment: StudentAssignmentAttemptHistoryAssignment {
            reference: assignment,
            title: row.try_get("assignment_title").map_err(map_sqlx_error)?,
        },
        state,
        score: None,
        questions,
    };
    let result = StudentAssignmentAttemptHistoryEvidence {
        history,
        feedback_rule,
        due_at: timestamp(row.try_get("due_at_millis").map_err(map_sqlx_error)?),
        closes_at: timestamp(row.try_get("closes_at_millis").map_err(map_sqlx_error)?),
        submitted_at: timestamp(row.try_get("submitted_at_millis").map_err(map_sqlx_error)?),
        evaluated_at: Timestamp::from_unix_millis(
            row.try_get("evaluated_at_millis").map_err(map_sqlx_error)?,
        ),
        grading_is_current,
        grading_results,
    };
    tx.commit().await.map_err(map_sqlx_error)?;
    Ok(result)
}

fn reference<T>(
    value: i64,
    label: &str,
    build: impl FnOnce(u64) -> Option<T>,
) -> Result<T, StoreError> {
    u64::try_from(value)
        .ok()
        .and_then(build)
        .ok_or_else(|| StoreError::InvalidRecord(format!("{label} is invalid")))
}

fn name(value: String, label: &str) -> Result<String, StoreError> {
    (value == value.trim() && !value.is_empty() && value.chars().count() <= 200)
        .then_some(value)
        .ok_or_else(|| StoreError::InvalidRecord(format!("{label} is invalid")))
}

fn state(value: String) -> Result<LiveAssignmentPreviousAttemptState, StoreError> {
    match value.as_str() {
        "submitted" => Ok(LiveAssignmentPreviousAttemptState::Submitted),
        "closed" => Ok(LiveAssignmentPreviousAttemptState::Closed),
        _ => Err(StoreError::InvalidRecord(
            "Assignment Attempt state is invalid".to_string(),
        )),
    }
}

fn timestamp(value: Option<i64>) -> Option<Timestamp> {
    value.map(Timestamp::from_unix_millis)
}

fn public_issued_position(position: u32) -> Result<u32, StoreError> {
    (position > 0)
        .then_some(position)
        .ok_or_else(|| StoreError::InvalidRecord("Issued Question position is invalid".to_string()))
}

fn decode_question(
    question: StoredQuestion,
) -> Result<StudentAssignmentAttemptHistoryQuestion, StoreError> {
    let question_id = QuestionId::from_str(&question.question_id)
        .map_err(|_| StoreError::InvalidRecord("Issued Question ID is invalid".to_string()))?;
    let revision_number = QuestionRevisionNumber::new(question.revision_number).map_err(|_| {
        StoreError::InvalidRecord("Issued Question Revision Number is invalid".to_string())
    })?;
    let position = public_issued_position(question.position)?;
    Ok(StudentAssignmentAttemptHistoryQuestion {
        position,
        question_revision: QuestionRevisionReference {
            question_id,
            revision_number,
        },
        response_state: question.response_state,
        response: None,
        feedback: StudentFeedback::empty(),
    })
}

fn decode_results(
    value: serde_json::Value,
    questions: &[StudentAssignmentAttemptHistoryQuestion],
    grading_is_current: bool,
) -> Result<Vec<Option<GradingResult>>, StoreError> {
    if !grading_is_current {
        return Ok(vec![None; questions.len()]);
    }
    let stored = serde_json::from_value::<Vec<StoredGradingResult>>(value).map_err(|_| {
        StoreError::InvalidRecord("Assignment Attempt grading results are invalid".to_string())
    })?;
    if stored
        .iter()
        .any(|result| public_issued_position(result.position).is_err())
        || stored.len() != questions.len()
        || stored
            .iter()
            .zip(questions)
            .any(|(result, question)| result.position != question.position)
    {
        return Err(StoreError::InvalidRecord(
            "Assignment Attempt grading results do not match issued positions".to_string(),
        ));
    }
    let results = stored
        .into_iter()
        .map(|result| {
            if !result.points_earned.is_finite()
                || !result.points_possible.is_finite()
                || result.points_earned < 0.0
                || result.points_possible < result.points_earned
            {
                return Err(StoreError::InvalidRecord(
                    "Assignment Attempt grading result is invalid".to_string(),
                ));
            }
            Ok(Some(GradingResult {
                correct: result.correct,
                points_earned: result.points_earned,
                points_possible: result.points_possible,
            }))
        })
        .collect::<Result<Vec<_>, StoreError>>()?;
    Ok(results)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn history_question_uses_the_exact_retained_issued_revision() {
        let question = decode_question(StoredQuestion {
            position: 1,
            question_id: "ABCDEF1".to_string(),
            revision_number: 3,
            response_state: LiveAssignmentPreviousAttemptState::Submitted,
        })
        .expect("retained issued Question evidence is valid");

        assert_eq!(
            question.question_revision.question_id.to_string(),
            "ABC-DEF1"
        );
        assert_eq!(question.question_revision.revision_number.get(), 3);
    }

    #[test]
    fn history_question_rejects_invalid_issued_revision_evidence() {
        let error = decode_question(StoredQuestion {
            position: 1,
            question_id: "ABCDEF1".to_string(),
            revision_number: 0,
            response_state: LiveAssignmentPreviousAttemptState::Closed,
        })
        .expect_err("invalid issued Question evidence fails closed");

        assert!(matches!(error, StoreError::InvalidRecord(_)));
    }

    #[test]
    fn history_rejects_zero_based_public_issued_positions() {
        let question = decode_question(StoredQuestion {
            position: 0,
            question_id: "ABCDEF1".to_string(),
            revision_number: 1,
            response_state: LiveAssignmentPreviousAttemptState::Submitted,
        });
        assert!(matches!(question, Err(StoreError::InvalidRecord(_))));

        let retained_question = decode_question(StoredQuestion {
            position: 1,
            question_id: "ABCDEF1".to_string(),
            revision_number: 1,
            response_state: LiveAssignmentPreviousAttemptState::Submitted,
        })
        .expect("one-based Question evidence is valid");
        let results = decode_results(
            serde_json::json!([{
                "position": 0,
                "correct": true,
                "pointsEarned": 1.0,
                "pointsPossible": 1.0,
            }]),
            &[retained_question],
            true,
        );
        assert!(matches!(results, Err(StoreError::InvalidRecord(_))));
    }
}

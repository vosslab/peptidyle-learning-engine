//! Selected completed-Attempt history storage projection.

use std::str::FromStr;

use super::{assessment_delivery::PostgresLiveAssessmentDeliveryStore, connection::map_sqlx_error};
use crate::{
    LiveAssessmentPreviousAttemptState, SessionTokenHash, StoreError,
    StudentAssessmentAttemptHistory, StudentAssessmentAttemptHistoryAssessment,
    StudentAssessmentAttemptHistoryCourse, StudentAssessmentAttemptHistoryEvidence,
    StudentAssessmentAttemptHistoryQuestion,
};
use question_model::{
    AssessmentAttemptId, AssessmentId, AssessmentType, CourseInstanceId, CourseTheme,
    GradingResult, PublishedQuestionId, PublishedQuestionRevisionTuple, QuestionRevisionNumber,
    StudentFeedback, StudentFeedbackReleaseRule, Timestamp,
};
use serde::Deserialize;
use sqlx::Row;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct StoredQuestion {
    position: u32,
    question_id: String,
    revision_number: u32,
    response_state: LiveAssessmentPreviousAttemptState,
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
    store: &PostgresLiveAssessmentDeliveryStore,
    token: SessionTokenHash,
    assessment_attempt: AssessmentAttemptId,
) -> Result<StudentAssessmentAttemptHistoryEvidence, StoreError> {
    let mut tx = store.begin(token).await?;
    // ASVS 8.2.2 and 8.3.1: the parameter is only a public ID; the
    // SECURITY DEFINER reader re-checks exact Student ownership and membership.
    let row = sqlx::query(
        "SELECT course_instance_id, course_short_name, course_long_name, course_theme, \
         assessment_id, assessment_title, assessment_type, assessment_attempt_number, state, questions, \
         feedback_rule, due_at_millis, closes_at_millis, submitted_at_millis, evaluated_at_millis, \
         all_students_completed, grading_is_current, grading_results \
         FROM ple_api.read_student_assessment_attempt_history($1)",
    )
    .bind(assessment_attempt.as_uuid())
    .fetch_optional(&mut *tx)
    .await
    .map_err(map_sqlx_error)?
    .ok_or(StoreError::NotFound)?;
    let assessment_id_value: String = row.try_get("assessment_id").map_err(map_sqlx_error)?;
    let assessment = AssessmentId::new(assessment_id_value)
        .map_err(|_| StoreError::InvalidRecord("Assessment ID is invalid".to_string()))?;
    let attempt_number = u32::try_from(
        row.try_get::<i32, _>("assessment_attempt_number")
            .map_err(map_sqlx_error)?,
    )
    .map_err(|_| StoreError::InvalidRecord("Assessment Attempt number is invalid".to_string()))?;
    let state = state(row.try_get("state").map_err(map_sqlx_error)?)?;
    let stored_questions = serde_json::from_value::<Vec<StoredQuestion>>(
        row.try_get("questions").map_err(map_sqlx_error)?,
    )
    .map_err(|_| {
        StoreError::InvalidRecord("Assessment Attempt positions are invalid".to_string())
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
    let assessment_type = AssessmentType::parse(
        &row.try_get::<String, _>("assessment_type")
            .map_err(map_sqlx_error)?,
    )
    .ok_or_else(|| StoreError::InvalidRecord("Assessment Type is invalid".to_string()))?;
    let history = StudentAssessmentAttemptHistory {
        assessment_attempt_id: assessment_attempt,
        attempt_number,
        course: StudentAssessmentAttemptHistoryCourse {
            id: course_instance_id(row.try_get("course_instance_id").map_err(map_sqlx_error)?)?,
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
        assessment: StudentAssessmentAttemptHistoryAssessment {
            id: assessment,
            assessment_type,
            title: row.try_get("assessment_title").map_err(map_sqlx_error)?,
        },
        state,
        score: None,
        questions,
    };
    let result = StudentAssessmentAttemptHistoryEvidence {
        history,
        assessment_type,
        feedback_rule,
        due_at: timestamp(row.try_get("due_at_millis").map_err(map_sqlx_error)?),
        closes_at: timestamp(row.try_get("closes_at_millis").map_err(map_sqlx_error)?),
        submitted_at: timestamp(row.try_get("submitted_at_millis").map_err(map_sqlx_error)?),
        evaluated_at: Timestamp::from_unix_millis(
            row.try_get("evaluated_at_millis").map_err(map_sqlx_error)?,
        ),
        all_students_completed: row
            .try_get("all_students_completed")
            .map_err(map_sqlx_error)?,
        grading_is_current,
        grading_results,
    };
    tx.commit().await.map_err(map_sqlx_error)?;
    Ok(result)
}

fn course_instance_id(value: String) -> Result<CourseInstanceId, StoreError> {
    CourseInstanceId::new(value)
        .map_err(|_| StoreError::InvalidRecord("Course ID is invalid".to_string()))
}

fn name(value: String, label: &str) -> Result<String, StoreError> {
    (value == value.trim() && !value.is_empty() && value.chars().count() <= 200)
        .then_some(value)
        .ok_or_else(|| StoreError::InvalidRecord(format!("{label} is invalid")))
}

fn state(value: String) -> Result<LiveAssessmentPreviousAttemptState, StoreError> {
    match value.as_str() {
        "submitted" => Ok(LiveAssessmentPreviousAttemptState::Submitted),
        "closed" => Ok(LiveAssessmentPreviousAttemptState::Closed),
        _ => Err(StoreError::InvalidRecord(
            "Assessment Attempt state is invalid".to_string(),
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
) -> Result<StudentAssessmentAttemptHistoryQuestion, StoreError> {
    let question_id = PublishedQuestionId::from_str(&question.question_id)
        .map_err(|_| StoreError::InvalidRecord("Issued Question ID is invalid".to_string()))?;
    let revision_number = QuestionRevisionNumber::new(question.revision_number).map_err(|_| {
        StoreError::InvalidRecord("Issued Question Revision Number is invalid".to_string())
    })?;
    let position = public_issued_position(question.position)?;
    Ok(StudentAssessmentAttemptHistoryQuestion {
        position,
        published_question_revision_tuple: PublishedQuestionRevisionTuple {
            published_question_id: question_id,
            revision_number,
        },
        response_state: question.response_state,
        response: None,
        backend_answer_review: None,
        feedback: StudentFeedback::empty(),
    })
}

fn decode_results(
    value: serde_json::Value,
    questions: &[StudentAssessmentAttemptHistoryQuestion],
    grading_is_current: bool,
) -> Result<Vec<Option<GradingResult>>, StoreError> {
    if !grading_is_current {
        return Ok(vec![None; questions.len()]);
    }
    let stored = serde_json::from_value::<Vec<StoredGradingResult>>(value).map_err(|_| {
        StoreError::InvalidRecord("Assessment Attempt grading results are invalid".to_string())
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
            "Assessment Attempt grading results do not match issued positions".to_string(),
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
                    "Assessment Attempt grading result is invalid".to_string(),
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

    fn question_id() -> String {
        PublishedQuestionId::from_random_identifier("ABCDEFG")
            .expect("test Question random identity is canonical")
            .to_string()
    }

    #[test]
    fn history_question_uses_the_exact_retained_issued_revision() {
        let question = decode_question(StoredQuestion {
            position: 1,
            question_id: question_id(),
            revision_number: 3,
            response_state: LiveAssessmentPreviousAttemptState::Submitted,
        })
        .expect("retained issued Question evidence is valid");

        assert_eq!(
            question
                .published_question_revision_tuple
                .published_question_id
                .to_string(),
            question_id()
        );
        assert_eq!(
            question
                .published_question_revision_tuple
                .revision_number
                .get(),
            3
        );
    }

    #[test]
    fn history_question_rejects_invalid_issued_revision_evidence() {
        let error = decode_question(StoredQuestion {
            position: 1,
            question_id: question_id(),
            revision_number: 0,
            response_state: LiveAssessmentPreviousAttemptState::Closed,
        })
        .expect_err("invalid issued Question evidence fails closed");

        assert!(matches!(error, StoreError::InvalidRecord(_)));
    }

    #[test]
    fn history_rejects_zero_based_public_issued_positions() {
        let question = decode_question(StoredQuestion {
            position: 0,
            question_id: question_id(),
            revision_number: 1,
            response_state: LiveAssessmentPreviousAttemptState::Submitted,
        });
        assert!(matches!(question, Err(StoreError::InvalidRecord(_))));

        let retained_question = decode_question(StoredQuestion {
            position: 1,
            question_id: question_id(),
            revision_number: 1,
            response_state: LiveAssessmentPreviousAttemptState::Submitted,
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

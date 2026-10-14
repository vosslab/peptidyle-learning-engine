//! Private response submission and status handlers.

use axum::{
    Json,
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
};
use browser_api_contract::student_question_display_duration::{
    StudentQuestionDisplayDurationCheckpoint, StudentQuestionDisplayDurationCheckpointRequest,
};
use learning_data_access::{
    StudentAssessmentAttemptFinalization, StudentAssessmentAttemptFinalizationPreparationOutcome,
    StudentAssessmentAttemptResponseSaveResult,
};
use question_model::{
    AssessmentAttemptId, StudentResponse,
    presentation::{
        IssuedQuestionPresentation, StudentResponseInspection,
        project_durable_response_to_presentation_response_item_ids,
        translate_presentation_response_item_ids,
    },
};
use serde::{Deserialize, Serialize};

use super::{
    StartError, StateData, concealed, direct_finalization, error,
    reproduce_selected_issued_presentation, student, submission_store_error,
};
use question_model::response::{
    ResponseItemId, StudentHotspotSelection, StudentMatch, StudentTextEntry,
};

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct SavedResponseRequest {
    response: StudentResponse,
    expected_saved_response_revision: u64,
}

/// Stores the cumulative milliseconds this Question has been shown while visible.
pub(super) async fn checkpoint_question_display_duration(
    State(state): State<StateData>,
    headers: HeaderMap,
    Path((assessment_attempt, position)): Path<(String, u32)>,
    Json(request): Json<StudentQuestionDisplayDurationCheckpointRequest>,
) -> Response {
    let assessment_attempt = match assessment_attempt.parse::<AssessmentAttemptId>() {
        Ok(value) if position > 0 => value,
        _ => return concealed(),
    };
    if request.cumulative_display_duration_ms > 9_007_199_254_740_991 {
        return error(
            StatusCode::BAD_REQUEST,
            "Question display duration is invalid",
        );
    }
    let token = match student(&state, &headers).await {
        Ok(value) => value,
        Err(value) => return *value,
    };
    match state
        .delivery
        .checkpoint_student_question_display_duration(
            token,
            assessment_attempt,
            position,
            request.cumulative_display_duration_ms,
        )
        .await
    {
        Ok(cumulative_display_duration_ms) => crate::auth::no_store(
            Json(StudentQuestionDisplayDurationCheckpoint {
                assessment_attempt_id: assessment_attempt,
                position,
                cumulative_display_duration_ms,
            })
            .into_response(),
        ),
        Err(value) => submission_store_error(value),
    }
}

/// Saves one response selected by a public Assessment Attempt and fixed
/// position. The durable record receives only canonical IDs recovered
/// from the exact issued presentation.
pub(super) async fn save_selected_response(
    State(state): State<StateData>,
    headers: HeaderMap,
    Path((assessment_attempt, position)): Path<(String, u32)>,
    Json(request): Json<SavedResponseRequest>,
) -> Response {
    let assessment_attempt = match assessment_attempt.parse::<AssessmentAttemptId>() {
        Ok(value) if position > 0 => value,
        _ => return concealed(),
    };
    let token = match student(&state, &headers).await {
        Ok(value) => value,
        Err(value) => return *value,
    };
    let source = match state
        .delivery
        .student_assessment_attempt_presentation_evidence(token, assessment_attempt, position)
        .await
    {
        Ok(value) => value,
        Err(value) => return submission_store_error(value),
    };
    let issued = match reproduce_selected_issued_presentation(source) {
        Ok(value) => value,
        Err(StartError::Unavailable) => {
            return error(
                StatusCode::SERVICE_UNAVAILABLE,
                "Question response unavailable",
            );
        }
        Err(StartError::Invalid | StartError::Store(_)) => return invalid_response(),
    };
    if !domain::validation::validate_presentation_response_format(
        &issued.presentation.response,
        &request.response,
    )
    .is_valid()
    {
        return invalid_response();
    }
    let response = match translate_presentation_response_item_ids(&request.response, &issued) {
        Ok(value) => value,
        Err(_) => return invalid_response(),
    };
    match state
        .delivery
        .save_student_assessment_attempt_response(
            token,
            assessment_attempt,
            position,
            response,
            request.expected_saved_response_revision,
        )
        .await
    {
        Ok(StudentAssessmentAttemptResponseSaveResult::Saved(saved))
            if saved.assessment_attempt_id == assessment_attempt && saved.position == position =>
        {
            crate::auth::no_store(
                Json(SavedResponseAcknowledgement {
                    assessment_attempt_id: assessment_attempt,
                    position,
                    response_state: "saved",
                    saved_response_revision: saved.saved_response_revision,
                })
                .into_response(),
            )
        }
        Ok(StudentAssessmentAttemptResponseSaveResult::Conflict {
            saved_response_revision,
        }) => crate::auth::no_store(
            (
                StatusCode::PRECONDITION_FAILED,
                Json(SavedResponseConflict {
                    saved_response_revision,
                }),
            )
                .into_response(),
        ),
        Ok(StudentAssessmentAttemptResponseSaveResult::Saved(_)) => error(
            StatusCode::SERVICE_UNAVAILABLE,
            "Student Response save unavailable",
        ),
        Err(value) => submission_store_error(value),
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct SavedResponseAcknowledgement {
    assessment_attempt_id: AssessmentAttemptId,
    position: u32,
    response_state: &'static str,
    saved_response_revision: u64,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct SavedResponseConflict {
    saved_response_revision: u64,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct AssessmentAttemptSubmissionAcknowledgement {
    assessment_attempt_id: AssessmentAttemptId,
    submission_state: &'static str,
}

/// Finalizes the whole Assessment Attempt with every response the Student saved.
/// Unanswered issued positions remain part of the terminal Attempt history.
pub(super) async fn finalize_assessment_attempt(
    State(state): State<StateData>,
    headers: HeaderMap,
    Path(assessment_attempt): Path<String>,
) -> Response {
    let assessment_attempt = match assessment_attempt.parse::<AssessmentAttemptId>() {
        Ok(value) => value,
        Err(_) => return concealed(),
    };
    let token = match student(&state, &headers).await {
        Ok(value) => value,
        Err(value) => return *value,
    };
    let preparation = match state
        .delivery
        .prepare_student_assessment_attempt_finalization(token, assessment_attempt)
        .await
    {
        Ok(value) => value,
        Err(value) => return submission_store_error(value),
    };
    let finalization = match preparation {
        StudentAssessmentAttemptFinalizationPreparationOutcome::AlreadySubmitted { score } => {
            StudentAssessmentAttemptFinalization::Submitted { score }
        }
        StudentAssessmentAttemptFinalizationPreparationOutcome::Ready(preparation) => {
            let evaluations = match direct_finalization::evaluate_saved_responses(
                &state.objects,
                state.webwork.as_ref(),
                &preparation.saved_responses,
            )
            .await
            {
                Ok(value) => value,
                Err(value) => return submission_store_error(value),
            };
            match state
                .delivery
                .commit_student_assessment_attempt_finalization(
                    token,
                    assessment_attempt,
                    preparation,
                    evaluations,
                )
                .await
            {
                Ok(value) => value,
                Err(value) => return submission_store_error(value),
            }
        }
    };
    let StudentAssessmentAttemptFinalization::Submitted { .. } = finalization;
    crate::auth::no_store(
        Json(AssessmentAttemptSubmissionAcknowledgement {
            assessment_attempt_id: assessment_attempt,
            submission_state: "submitted",
        })
        .into_response(),
    )
}

/// Reconstructs the Student wire response using only presentation-scoped IDs.
/// Durable authored identifiers remain on the server.
pub(super) fn restore_saved_response(
    response: &StudentResponse,
    presentation: &IssuedQuestionPresentation,
) -> Result<StudentResponse, ()> {
    let inspection =
        project_durable_response_to_presentation_response_item_ids(response, presentation)
            .map_err(|_| ())?;
    Ok(match inspection {
        StudentResponseInspection::Numeric { value } => StudentResponse::Numeric { value },
        StudentResponseInspection::MultipleChoice { selected } => StudentResponse::MultipleChoice {
            selected: selected.into_iter().map(presentation_item_id).collect(),
        },
        StudentResponseInspection::ShortText { text } => StudentResponse::ShortText { text },
        StudentResponseInspection::MultiBlank { answers } => StudentResponse::MultiBlank {
            answers: answers
                .into_iter()
                .map(|answer| StudentTextEntry {
                    slot: presentation_item_id(answer.slot),
                    text: answer.text,
                })
                .collect(),
        },
        StudentResponseInspection::Matching { matches } => StudentResponse::Matching {
            matches: matches
                .into_iter()
                .map(|pair| StudentMatch {
                    prompt: presentation_item_id(pair.prompt),
                    choice: presentation_item_id(pair.choice),
                })
                .collect(),
        },
        StudentResponseInspection::Ordering { order } => StudentResponse::Ordering {
            order: order.into_iter().map(presentation_item_id).collect(),
        },
        StudentResponseInspection::Hotspot { selected_regions } => StudentResponse::Hotspot {
            selections: selected_regions
                .into_iter()
                .map(|region| StudentHotspotSelection {
                    region: presentation_item_id(region),
                })
                .collect(),
        },
        StudentResponseInspection::ImathasQuestionBackend { .. } => {
            StudentResponse::ImathasQuestionBackend {}
        }
        StudentResponseInspection::BackendOwned { payload } => {
            StudentResponse::BackendOwned { payload }
        }
    })
}

fn presentation_item_id(value: question_model::PresentationResponseItemId) -> ResponseItemId {
    ResponseItemId::new(value.as_str())
}

fn invalid_response() -> Response {
    error(
        StatusCode::UNPROCESSABLE_ENTITY,
        "Student Response is invalid",
    )
}

#[cfg(test)]
mod tests {
    use super::SavedResponseRequest;
    use serde_json::json;

    #[test]
    fn response_save_accepts_the_browser_revision_contract() {
        let response = json!({"kind": "shortText", "text": "replication"});
        let request: SavedResponseRequest = serde_json::from_value(json!({
            "response": response,
            "expectedSavedResponseRevision": 2,
        }))
        .expect("the browser's revision-bearing response must be accepted");
        assert_eq!(request.expected_saved_response_revision, 2);
        assert!(
            serde_json::from_value::<SavedResponseRequest>(json!({"response": response})).is_err()
        );
    }
}

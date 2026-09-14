//! Private response submission and status handlers.

use axum::{
    Json,
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
};
use learning_data_access::{
    LiveAssignmentAttemptScore, LiveAssignmentDeliveryStore, StudentAssignmentAttemptFinalization,
    StudentAssignmentAttemptFinalizationPreparationOutcome,
};
use question_model::{
    AssignmentAttemptReference, StudentResponse,
    presentation::{
        IssuedQuestionPresentation, StudentResponseInspection,
        project_durable_response_to_presentation_response_item_references,
        translate_presentation_response_item_references,
    },
};
use serde::{Deserialize, Serialize};

use super::{
    StartError, StateData, concealed, direct_finalization, error,
    reproduce_selected_issued_presentation, student, submission_store_error,
};
use question_model::response::{
    ResponseItemReference, StudentHotspotSelection, StudentMatch, StudentTextEntry,
};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct SavedResponseRequest {
    response: StudentResponse,
}

/// Saves one response selected by a public Assignment Attempt and fixed
/// position. The durable record receives only canonical references recovered
/// from the exact issued presentation.
pub(super) async fn save_selected_response(
    State(state): State<StateData>,
    headers: HeaderMap,
    Path((assignment_attempt, position)): Path<(String, u32)>,
    Json(request): Json<SavedResponseRequest>,
) -> Response {
    let assignment_attempt = match assignment_attempt.parse::<AssignmentAttemptReference>() {
        Ok(value) if position > 0 => value,
        _ => return concealed(),
    };
    let token = match student(&state, &headers).await {
        Ok(value) => value,
        Err(value) => return *value,
    };
    let source = match state
        .delivery
        .student_assignment_attempt_presentation_evidence(token, assignment_attempt, position)
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
                "Question Submission unavailable",
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
    let response = match translate_presentation_response_item_references(&request.response, &issued)
    {
        Ok(value) => value,
        Err(_) => return invalid_response(),
    };
    match state
        .delivery
        .save_student_assignment_attempt_response(token, assignment_attempt, position, response)
        .await
    {
        Ok(saved)
            if saved.assignment_attempt == assignment_attempt && saved.position == position =>
        {
            crate::auth::no_store(
                Json(SavedResponseAcknowledgement {
                    assignment_attempt,
                    position,
                    response_state: "saved",
                })
                .into_response(),
            )
        }
        Ok(_) => error(
            StatusCode::SERVICE_UNAVAILABLE,
            "Student Response save unavailable",
        ),
        Err(value) => submission_store_error(value),
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct SavedResponseAcknowledgement {
    assignment_attempt: AssignmentAttemptReference,
    position: u32,
    response_state: &'static str,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct AssignmentAttemptSubmissionAcknowledgement {
    assignment_attempt: AssignmentAttemptReference,
    submission_state: &'static str,
    score: Option<LiveAssignmentAttemptScore>,
}

/// Finalizes the whole Assignment Attempt only after the store verifies every
/// fixed issued position has a saved response.
pub(super) async fn finalize_assignment_attempt(
    State(state): State<StateData>,
    headers: HeaderMap,
    Path(assignment_attempt): Path<String>,
) -> Response {
    let assignment_attempt = match assignment_attempt.parse::<AssignmentAttemptReference>() {
        Ok(value) => value,
        Err(_) => return concealed(),
    };
    let token = match student(&state, &headers).await {
        Ok(value) => value,
        Err(value) => return *value,
    };
    let preparation = match state
        .delivery
        .prepare_student_assignment_attempt_finalization(token, assignment_attempt)
        .await
    {
        Ok(value) => value,
        Err(value) => return submission_store_error(value),
    };
    let finalization = match preparation {
        StudentAssignmentAttemptFinalizationPreparationOutcome::AlreadySubmitted { score } => {
            StudentAssignmentAttemptFinalization::Submitted { score }
        }
        StudentAssignmentAttemptFinalizationPreparationOutcome::MissingResponses { .. } => {
            return error(
                StatusCode::UNPROCESSABLE_ENTITY,
                "Save a response for every Question before submitting",
            );
        }
        StudentAssignmentAttemptFinalizationPreparationOutcome::Ready(preparation) => {
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
                .commit_student_assignment_attempt_finalization(
                    token,
                    assignment_attempt,
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
    match finalization {
        StudentAssignmentAttemptFinalization::Submitted { score } => crate::auth::no_store(
            Json(AssignmentAttemptSubmissionAcknowledgement {
                assignment_attempt,
                submission_state: "submitted",
                score,
            })
            .into_response(),
        ),
        StudentAssignmentAttemptFinalization::MissingResponses { .. } => error(
            StatusCode::UNPROCESSABLE_ENTITY,
            "Save a response for every Question before submitting",
        ),
    }
}

/// Reconstructs the Student wire response using only presentation-scoped IDs.
/// Durable authored identifiers remain on the server.
pub(super) fn restore_saved_response(
    response: &StudentResponse,
    presentation: &IssuedQuestionPresentation,
) -> Result<StudentResponse, ()> {
    let inspection =
        project_durable_response_to_presentation_response_item_references(response, presentation)
            .map_err(|_| ())?;
    Ok(match inspection {
        StudentResponseInspection::Numeric { value } => StudentResponse::Numeric { value },
        StudentResponseInspection::MultipleChoice { selected } => StudentResponse::MultipleChoice {
            selected: selected.into_iter().map(presentation_reference).collect(),
        },
        StudentResponseInspection::ShortText { text } => StudentResponse::ShortText { text },
        StudentResponseInspection::MultiBlank { answers } => StudentResponse::MultiBlank {
            answers: answers
                .into_iter()
                .map(|answer| StudentTextEntry {
                    slot: presentation_reference(answer.slot),
                    text: answer.text,
                })
                .collect(),
        },
        StudentResponseInspection::Matching { matches } => StudentResponse::Matching {
            matches: matches
                .into_iter()
                .map(|pair| StudentMatch {
                    prompt: presentation_reference(pair.prompt),
                    choice: presentation_reference(pair.choice),
                })
                .collect(),
        },
        StudentResponseInspection::Ordering { order } => StudentResponse::Ordering {
            order: order.into_iter().map(presentation_reference).collect(),
        },
        StudentResponseInspection::Hotspot { selected_regions } => StudentResponse::Hotspot {
            selections: selected_regions
                .into_iter()
                .map(|region| StudentHotspotSelection {
                    region: presentation_reference(region),
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

fn presentation_reference(
    value: question_model::PresentationResponseItemReference,
) -> ResponseItemReference {
    ResponseItemReference::new(value.as_str())
}

fn invalid_response() -> Response {
    error(
        StatusCode::UNPROCESSABLE_ENTITY,
        "Student Response is invalid",
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use question_model::answer::ResponseSelectionRule;
    use question_model::{
        NativeChoiceOrder, QuestionContentBlock, QuestionRevisionNumber, QuestionRevisionReference,
        QuestionVariation, QuestionVariationPresentation,
        generation::QuestionSeed,
        presentation::build_question_presentation,
        response::{QuestionChoice, QuestionResponseFormat},
    };

    fn issued_multiple_choice() -> IssuedQuestionPresentation {
        let presentation = QuestionVariationPresentation {
            variation: QuestionVariation::from_question_revision_and_question_seed(
                QuestionRevisionReference {
                    question_id: "123-4567".parse().expect("question id"),
                    revision_number: QuestionRevisionNumber::new(1).expect("revision"),
                },
                QuestionSeed::new(42),
            ),
            question_title: "Peptide bond".to_owned(),
            prompt: vec![QuestionContentBlock::Text {
                markdown: "Select one.".to_owned(),
            }],
            response: QuestionResponseFormat::MultipleChoice {
                choices: vec![QuestionChoice {
                    id: ResponseItemReference::new("durable-choice"),
                    body: vec![QuestionContentBlock::Text {
                        markdown: "Choice".to_owned(),
                    }],
                }],
                selection: ResponseSelectionRule::ExactlyOne,
            },
            native_choice_order: NativeChoiceOrder::Fixed,
        };
        build_question_presentation(&presentation, &[]).expect("issued presentation")
    }

    fn issued_backend_owned() -> IssuedQuestionPresentation {
        let presentation = QuestionVariationPresentation {
            variation: QuestionVariation::from_question_revision_and_question_seed(
                QuestionRevisionReference {
                    question_id: "123-4567".parse().expect("question id"),
                    revision_number: QuestionRevisionNumber::new(1).expect("revision"),
                },
                QuestionSeed::new(42),
            ),
            question_title: "Backend-owned question".to_owned(),
            prompt: Vec::new(),
            response: QuestionResponseFormat::BackendOwned {},
            native_choice_order: NativeChoiceOrder::Fixed,
        };
        build_question_presentation(&presentation, &[]).expect("issued presentation")
    }

    #[test]
    fn restored_saved_response_uses_presentation_reference_not_durable_identifier() {
        let issued = issued_multiple_choice();
        let response = StudentResponse::MultipleChoice {
            selected: vec![ResponseItemReference::new("durable-choice")],
        };

        let restored = restore_saved_response(&response, &issued).expect("restore response");
        let StudentResponse::MultipleChoice { selected } = restored else {
            panic!("multiple choice response");
        };
        assert_eq!(selected.len(), 1);
        assert_ne!(selected[0].as_str(), "durable-choice");
        assert_eq!(selected[0].as_str().len(), 4);
    }

    #[test]
    fn restored_backend_owned_response_preserves_opaque_bytes_without_interpretation() {
        let response = StudentResponse::BackendOwned {
            payload: b"AnSwEr0001=value&control=next".to_vec(),
        };

        let restored = restore_saved_response(&response, &issued_backend_owned())
            .expect("restore bounded opaque response");
        assert_eq!(restored, response);
    }

    #[test]
    fn submission_acknowledgement_retains_an_explicit_deferred_score() {
        let wire = serde_json::to_value(AssignmentAttemptSubmissionAcknowledgement {
            assignment_attempt: "R-1".parse().expect("Assignment Attempt reference"),
            submission_state: "submitted",
            score: None,
        })
        .expect("submission acknowledgement serializes");

        assert_eq!(
            wire,
            serde_json::json!({
                "assignmentAttempt": "R-1",
                "submissionState": "submitted",
                "score": null,
            })
        );
    }
}

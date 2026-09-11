//! Private response submission and status handlers.

use axum::{
    Json,
    extract::{FromRef, Path, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::{MethodRouter, get},
};
use learning_data_access::{
    LiveAssignmentDeliveryStore, NativePleSubmissionStatus, NativePleSubmissionStore,
    StudentAssignmentAttemptFinalization,
};
use question_model::{
    AssignmentAttemptReference, StudentResponse,
    presentation::{
        IssuedQuestionPresentation, QuestionPresentationNonce, StudentResponseInspection,
        project_durable_response_to_presentation_response_item_references,
        translate_presentation_response_item_references,
    },
};
use serde::{Deserialize, Serialize};

use super::{
    NativePleSubmissionStatusState, StartError, StateData, concealed, error, refs,
    reproduce_selected_issued_presentation, student, student_with_sessions, submission_store_error,
};
use question_model::response::{
    ResponseItemReference, StudentHotspotSelection, StudentMatch, StudentTextEntry,
};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct NativePleSubmissionRequest {
    response: StudentResponse,
}

/// Saves one response selected by a public Assignment Attempt and fixed
/// position. The durable record receives only canonical references recovered
/// from the exact issued presentation.
pub(super) async fn save_selected_response(
    State(state): State<StateData>,
    headers: HeaderMap,
    Path((assignment_attempt, position)): Path<(String, u32)>,
    Json(request): Json<NativePleSubmissionRequest>,
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
        .student_assignment_attempt_presentation_source(token, assignment_attempt, position)
        .await
    {
        Ok(value) => value,
        Err(value) => return submission_store_error(value),
    };
    let issued = match reproduce_selected_issued_presentation(&state, source).await {
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
    match state
        .delivery
        .finalize_student_assignment_attempt(token, assignment_attempt)
        .await
    {
        Ok(StudentAssignmentAttemptFinalization::Submitted) => crate::auth::no_store(
            Json(AssignmentAttemptSubmissionAcknowledgement {
                assignment_attempt,
                submission_state: "submitted",
            })
            .into_response(),
        ),
        Ok(StudentAssignmentAttemptFinalization::MissingResponses { .. }) => error(
            StatusCode::UNPROCESSABLE_ENTITY,
            "Save a response for every Question before submitting",
        ),
        Err(value) => submission_store_error(value),
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
    })
}

fn presentation_reference(
    value: question_model::PresentationResponseItemReference,
) -> ResponseItemReference {
    ResponseItemReference::new(value.as_str())
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{
        Router,
        body::Body,
        http::{Request, StatusCode},
    };
    use learning_data_access::postgres::{
        PostgresNativePleSubmissionStore, PostgresSessionStore, lazy_pool,
    };
    use question_model::answer::ResponseSelectionRule;
    use question_model::{
        NativeChoiceOrder, QuestionContentBlock, QuestionRevisionNumber, QuestionRevisionReference,
        QuestionVariation, QuestionVariationPresentation,
        generation::QuestionSeed,
        presentation::build_question_presentation,
        response::{QuestionChoice, QuestionResponseFormat},
    };
    use std::sync::Arc;
    use tower::ServiceExt;

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

    #[tokio::test]
    async fn status_route_rejects_legacy_per_question_post() {
        let pool = lazy_pool("postgres://ple_api:unused@localhost/ple_test")
            .expect("test pool configuration");
        let state = NativePleSubmissionStatusState {
            sessions: Arc::new(PostgresSessionStore::new(pool.clone())),
            submissions: PostgresNativePleSubmissionStore::new(pool),
        };
        let response = Router::new()
            .route(
                "/api/course-instances/C-1/assignments/A-1/presentations/abcd/submissions",
                native_ple_submission_status_route::<NativePleSubmissionStatusState>(),
            )
            .with_state(state)
            .oneshot(
                Request::post(
                    "/api/course-instances/C-1/assignments/A-1/presentations/abcd/submissions",
                )
                .body(Body::empty())
                .expect("request"),
            )
            .await
            .expect("response");

        assert_eq!(response.status(), StatusCode::METHOD_NOT_ALLOWED);
        assert_eq!(response.headers().get("allow").expect("allow"), "GET,HEAD");
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct NativePleSubmissionStatusResponse {
    presentation_nonce: String,
    grading_state: &'static str,
}

/// ASVS 2.3.1 and 4.1.4: the retained status route exposes only GET; whole-
/// Attempt finalization owns immutable submission state.
pub(super) fn native_ple_submission_status_route<S>() -> MethodRouter<S>
where
    S: Clone + Send + Sync + 'static,
    NativePleSubmissionStatusState: FromRef<S>,
{
    get(native_ple_submission_status)
}

async fn native_ple_submission_status(
    State(state): State<NativePleSubmissionStatusState>,
    headers: HeaderMap,
    Path((course, assignment, presentation_nonce)): Path<(String, String, String)>,
) -> Response {
    let (course, assignment) = match refs(&course, &assignment) {
        Ok(value) => value,
        Err(value) => return value,
    };
    let nonce = match QuestionPresentationNonce::parse(&presentation_nonce) {
        Ok(value) => value,
        Err(_) => return concealed(),
    };
    let token = match student_with_sessions(state.sessions.as_ref(), &headers).await {
        Ok(value) => value,
        Err(value) => return *value,
    };
    let response = match state
        .submissions
        .native_ple_submission_status(
            token,
            u64::from(course.number()),
            u64::from(assignment.number()),
            &nonce.to_hex(),
        )
        .await
    {
        Ok(NativePleSubmissionStatus {
            presentation_nonce,
            grading_state,
        }) => NativePleSubmissionStatusResponse {
            presentation_nonce,
            grading_state: match grading_state {
                learning_data_access::StudentQuestionSubmissionGradingState::Pending => "pending",
                learning_data_access::StudentQuestionSubmissionGradingState::Graded => "graded",
                learning_data_access::StudentQuestionSubmissionGradingState::InstructorAttention => {
                    "instructorAttention"
                }
            },
        },
        Err(value) => return submission_store_error(value),
    };
    crate::auth::no_store(Json(response).into_response())
}

fn invalid_response() -> Response {
    error(
        StatusCode::UNPROCESSABLE_ENTITY,
        "Student Response is invalid",
    )
}

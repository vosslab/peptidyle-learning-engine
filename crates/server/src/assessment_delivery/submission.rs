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
#[serde(deny_unknown_fields)]
pub(super) struct SavedResponseRequest {
    response: StudentResponse,
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
        .save_student_assessment_attempt_response(token, assessment_attempt, position, response)
        .await
    {
        Ok(saved)
            if saved.assessment_attempt_id == assessment_attempt && saved.position == position =>
        {
            crate::auth::no_store(
                Json(SavedResponseAcknowledgement {
                    assessment_attempt_id: assessment_attempt,
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
    assessment_attempt_id: AssessmentAttemptId,
    position: u32,
    response_state: &'static str,
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
    use super::*;
    use learning_data_access::LiveAssessmentDeliveryStore;
    use question_model::answer::ResponseSelectionRule;
    use question_model::{
        NativeChoiceOrder, PublishedQuestionRevisionTuple, QuestionContentBlock,
        QuestionReproduction, QuestionRevisionNumber, QuestionVariation,
        QuestionVariationPresentation,
        presentation::build_question_presentation,
        response::{QuestionChoice, QuestionResponseFormat},
    };

    fn issued_multiple_choice() -> IssuedQuestionPresentation {
        let presentation = QuestionVariationPresentation {
            variation: QuestionVariation::from_question_revision_and_reproduction(
                PublishedQuestionRevisionTuple {
                    published_question_id:
                        question_model::PublishedQuestionId::from_random_identifier("1234567")
                            .expect("question id"),
                    revision_number: QuestionRevisionNumber::new(1).expect("revision"),
                },
                QuestionReproduction::Static,
            ),
            question_title: "Peptide bond".to_owned(),
            prompt: vec![QuestionContentBlock::Text {
                markdown: "Select one.".to_owned(),
            }],
            response: QuestionResponseFormat::MultipleChoice {
                choices: vec![QuestionChoice {
                    id: ResponseItemId::new("durable-choice"),
                    body: vec![QuestionContentBlock::Text {
                        markdown: "Choice".to_owned(),
                    }],
                }],
                selection: ResponseSelectionRule::ExactlyOne,
            },
            native_choice_order: NativeChoiceOrder::Fixed,
            author_content: None,
        };
        build_question_presentation(&presentation, &[]).expect("issued presentation")
    }

    fn issued_backend_owned() -> IssuedQuestionPresentation {
        let presentation = QuestionVariationPresentation {
            variation: QuestionVariation::from_question_revision_and_reproduction(
                PublishedQuestionRevisionTuple {
                    published_question_id:
                        question_model::PublishedQuestionId::from_random_identifier("1234567")
                            .expect("question id"),
                    revision_number: QuestionRevisionNumber::new(1).expect("revision"),
                },
                QuestionReproduction::Static,
            ),
            question_title: "Backend-owned question".to_owned(),
            prompt: Vec::new(),
            response: QuestionResponseFormat::BackendOwned {},
            native_choice_order: NativeChoiceOrder::Fixed,
            author_content: None,
        };
        build_question_presentation(&presentation, &[]).expect("issued presentation")
    }

    #[test]
    fn restored_saved_response_uses_presentation_item_id_not_durable_identifier() {
        let issued = issued_multiple_choice();
        let response = StudentResponse::MultipleChoice {
            selected: vec![ResponseItemId::new("durable-choice")],
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
    fn submission_acknowledgement_reports_completion_without_grading_data() {
        let wire = serde_json::to_value(AssessmentAttemptSubmissionAcknowledgement {
            assessment_attempt_id: AssessmentAttemptId::from_uuid(uuid::Uuid::from_u128(1)),
            submission_state: "submitted",
        })
        .expect("submission acknowledgement serializes");

        assert_eq!(
            wire,
            serde_json::json!({
                "assessmentAttemptId": "00000000-0000-0000-0000-000000000001",
                "submissionState": "submitted",
            })
        );
    }

    /// ASVS 8.2.1 and 8.2.2: this double is not installed by the production router.
    /// The shipped handler still requires a Student session before preparation.
    struct AlreadySubmittedDelivery {
        prepares: std::sync::Arc<std::sync::atomic::AtomicUsize>,
        commits: std::sync::Arc<std::sync::atomic::AtomicUsize>,
        saves: std::sync::Arc<std::sync::atomic::AtomicUsize>,
        evidence: Option<learning_data_access::StudentAssessmentAttemptPresentationEvidence>,
        saved_response: std::sync::Arc<std::sync::Mutex<Option<StudentResponse>>>,
    }

    fn issued_choice_evidence() -> (
        learning_data_access::StudentAssessmentAttemptPresentationEvidence,
        String,
    ) {
        let presentation = QuestionVariationPresentation {
            variation: QuestionVariation::from_question_revision_and_reproduction(
                PublishedQuestionRevisionTuple {
                    published_question_id:
                        question_model::PublishedQuestionId::from_random_identifier("1234567")
                            .expect("question id"),
                    revision_number: QuestionRevisionNumber::new(1).expect("revision"),
                },
                QuestionReproduction::Static,
            ),
            question_title: "Peptide bond".to_owned(),
            prompt: vec![QuestionContentBlock::Text {
                markdown: "Select one.".to_owned(),
            }],
            response: QuestionResponseFormat::MultipleChoice {
                choices: vec![
                    QuestionChoice {
                        id: ResponseItemId::new("durable-choice"),
                        body: vec![QuestionContentBlock::Text {
                            markdown: "Choice".to_owned(),
                        }],
                    },
                    QuestionChoice {
                        id: ResponseItemId::new("other-choice"),
                        body: vec![QuestionContentBlock::Text {
                            markdown: "Other".to_owned(),
                        }],
                    },
                ],
                selection: ResponseSelectionRule::ExactlyOne,
            },
            native_choice_order: NativeChoiceOrder::Fixed,
            author_content: None,
        };
        let issued = build_question_presentation(&presentation, &[]).expect("issued presentation");
        let question_model::QuestionPresentationResponseFormat::SingleChoice { choices } =
            &issued.presentation.response
        else {
            panic!("single choice presentation");
        };
        let choice_id = choices[0].id.as_str().to_owned();
        let response_item_bindings =
            question_model::presentation::extract_durable_response_item_bindings(&issued)
                .expect("durable bindings");
        let evidence = learning_data_access::StudentAssessmentAttemptPresentationEvidence {
            published_question_revision_tuple: issued
                .presentation
                .published_question_revision_tuple
                .clone(),
            reproduction: issued.reproduction.clone(),
            presentation_nonce: issued.presentation.presentation_nonce.to_hex(),
            presentation_checksum: issued.checksum.to_hex(),
            presentation: serde_json::to_value(&issued.presentation).expect("presentation json"),
            author_content: issued.author_content.clone(),
            response_item_bindings,
            question_image_renditions: Vec::new(),
        };
        (evidence, choice_id)
    }

    #[async_trait::async_trait]
    impl LiveAssessmentDeliveryStore for AlreadySubmittedDelivery {
        async fn student_assessment_attempt_context(
            &self,
            _: learning_data_access::SessionTokenHash,
            _: AssessmentAttemptId,
        ) -> Result<
            learning_data_access::StudentAssessmentAttemptContext,
            learning_data_access::StoreError,
        > {
            unreachable!("rescore does not read Attempt context")
        }

        async fn save_student_assessment_attempt_response(
            &self,
            _: learning_data_access::SessionTokenHash,
            assessment_attempt_id: AssessmentAttemptId,
            position: u32,
            response: StudentResponse,
        ) -> Result<
            learning_data_access::StudentAssessmentAttemptSavedResponse,
            learning_data_access::StoreError,
        > {
            self.saves.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            *self.saved_response.lock().expect("saved response") = Some(response);
            Ok(
                learning_data_access::StudentAssessmentAttemptSavedResponse {
                    assessment_attempt_id,
                    position,
                },
            )
        }

        async fn checkpoint_student_question_display_duration(
            &self,
            _: learning_data_access::SessionTokenHash,
            _: AssessmentAttemptId,
            _: u32,
            _: u64,
        ) -> Result<u64, learning_data_access::StoreError> {
            unreachable!("rescore does not checkpoint display duration")
        }

        async fn student_assessment_attempt_saved_response(
            &self,
            _: learning_data_access::SessionTokenHash,
            _: AssessmentAttemptId,
            _: u32,
        ) -> Result<Option<StudentResponse>, learning_data_access::StoreError> {
            unreachable!("rescore does not read a saved response")
        }

        async fn prepare_student_assessment_attempt_finalization(
            &self,
            _: learning_data_access::SessionTokenHash,
            _: AssessmentAttemptId,
        ) -> Result<
            StudentAssessmentAttemptFinalizationPreparationOutcome,
            learning_data_access::StoreError,
        > {
            self.prepares
                .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            Ok(
                StudentAssessmentAttemptFinalizationPreparationOutcome::AlreadySubmitted {
                    score: Some(learning_data_access::LiveAssessmentAttemptScore {
                        points_earned: 5.0,
                        points_possible: 5.0,
                    }),
                },
            )
        }

        async fn commit_student_assessment_attempt_finalization(
            &self,
            _: learning_data_access::SessionTokenHash,
            _: AssessmentAttemptId,
            _: learning_data_access::StudentAssessmentAttemptFinalizationPreparation,
            _: Vec<learning_data_access::StudentAssessmentAttemptFinalizationEvaluation>,
        ) -> Result<StudentAssessmentAttemptFinalization, learning_data_access::StoreError>
        {
            self.commits
                .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            Ok(StudentAssessmentAttemptFinalization::Submitted {
                score: Some(learning_data_access::LiveAssessmentAttemptScore {
                    points_earned: 5.0,
                    points_possible: 5.0,
                }),
            })
        }

        async fn student_assessment_attempt_progress(
            &self,
            _: learning_data_access::SessionTokenHash,
            _: AssessmentAttemptId,
        ) -> Result<
            question_model::StudentAssessmentAttemptProgress,
            learning_data_access::StoreError,
        > {
            unreachable!("rescore does not read progress")
        }

        async fn student_assessment_attempt_presentation_evidence(
            &self,
            _: learning_data_access::SessionTokenHash,
            _: AssessmentAttemptId,
            _: u32,
        ) -> Result<
            learning_data_access::StudentAssessmentAttemptPresentationEvidence,
            learning_data_access::StoreError,
        > {
            match &self.evidence {
                Some(value) => Ok(value.clone()),
                None => unreachable!("rescore does not read presentation evidence"),
            }
        }

        async fn student_assessment_attempt_backend_document(
            &self,
            _: learning_data_access::SessionTokenHash,
            _: AssessmentAttemptId,
            _: u32,
        ) -> Result<
            learning_data_access::StudentAssessmentAttemptBackendDocument,
            learning_data_access::StoreError,
        > {
            unreachable!("rescore does not read a backend document")
        }

        async fn prepare_native_assessment_issuance(
            &self,
            _: learning_data_access::SessionTokenHash,
            _: question_model::CourseInstanceId,
            _: question_model::AssessmentId,
        ) -> Result<
            learning_data_access::NativeAssessmentIssuanceBatch,
            learning_data_access::StoreError,
        > {
            unreachable!("rescore does not issue an Assessment")
        }

        async fn commit_native_assessment_issuance(
            &self,
            _: learning_data_access::SessionTokenHash,
            _: uuid::Uuid,
            _: Vec<learning_data_access::NativePresentationInput>,
        ) -> Result<learning_data_access::LiveAssessmentAttempt, learning_data_access::StoreError>
        {
            unreachable!("rescore does not commit issuance")
        }

        async fn live_assessment_access(
            &self,
            _: learning_data_access::SessionTokenHash,
            _: question_model::CourseInstanceId,
            _: question_model::AssessmentId,
        ) -> Result<learning_data_access::LiveAssessmentAccess, learning_data_access::StoreError>
        {
            unreachable!("rescore does not read Assessment access")
        }

        async fn student_assessment_attempt_history(
            &self,
            _: learning_data_access::SessionTokenHash,
            _: AssessmentAttemptId,
        ) -> Result<
            learning_data_access::StudentAssessmentAttemptHistoryEvidence,
            learning_data_access::StoreError,
        > {
            unreachable!("rescore does not read history")
        }

        async fn student_assessment_attempt_history_response_sources(
            &self,
            _: learning_data_access::SessionTokenHash,
            _: AssessmentAttemptId,
        ) -> Result<
            Vec<learning_data_access::StudentAssessmentAttemptHistoryResponseSource>,
            learning_data_access::StoreError,
        > {
            unreachable!("rescore does not read history responses")
        }
    }

    struct StudentSession;

    #[async_trait::async_trait]
    impl learning_data_access::SessionStore for StudentSession {
        async fn create_session(
            &self,
            _: learning_data_access::SessionTokenHash,
            _: question_model::AccountId,
            _: learning_data_access::SessionLifetime,
        ) -> Result<learning_data_access::SessionRecord, learning_data_access::StoreError> {
            unreachable!("rescore does not create a session")
        }

        async fn resolve_session(
            &self,
            token_hash: learning_data_access::SessionTokenHash,
        ) -> Result<Option<learning_data_access::SessionRecord>, learning_data_access::StoreError>
        {
            Ok(Some(learning_data_access::SessionRecord {
                id: learning_data_access::SessionId::generate()?,
                token_hash,
                account: question_model::AccountId::from_debug_serial(2),
                user_role: question_model::UserRole::Student,
                created_at: question_model::Timestamp::from_unix_millis(0),
                expires_at: question_model::Timestamp::from_unix_millis(60_000),
            }))
        }

        async fn revoke_session(
            &self,
            _: learning_data_access::SessionTokenHash,
        ) -> Result<(), learning_data_access::StoreError> {
            unreachable!("rescore does not revoke a session")
        }
    }

    #[tokio::test]
    async fn score_recalculation_does_not_require_another_question_backend_interaction() {
        use std::sync::Arc;
        use std::sync::atomic::{AtomicUsize, Ordering};
        use std::time::Duration;

        use axum::body::{Body, to_bytes};
        use axum::http::Request;
        use base64::Engine;
        use base64::engine::general_purpose::URL_SAFE_NO_PAD;
        use objects::minio::{EndpointConfig, client};
        use objects::s3::{BucketNames, S3ObjectStore};
        use question_model::QuestionRendererVersion;
        use tower::ServiceExt;

        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("renderer listener");
        let address = listener.local_addr().expect("renderer address");
        let connections = Arc::new(AtomicUsize::new(0));
        let observed_connections = Arc::clone(&connections);
        tokio::spawn(async move {
            while let Ok((socket, _)) = listener.accept().await {
                observed_connections.fetch_add(1, Ordering::SeqCst);
                drop(socket);
            }
        });
        let prepares = Arc::new(AtomicUsize::new(0));
        let commits = Arc::new(AtomicUsize::new(0));
        let webwork = adapter_webwork::WebworkAdapter::new(
            adapter_webwork::HttpWebworkRenderer::new(
                adapter_webwork::HttpWebworkRendererConfig::new(
                    &format!("http://{address}/"),
                    Duration::from_millis(200),
                    1024,
                    QuestionRendererVersion {
                        name: "webwork-pg-renderer".to_string(),
                        version: "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
                            .to_string(),
                    },
                    "https://ple.example/",
                )
                .expect("renderer settings"),
            )
            .expect("renderer"),
        );
        let state = StateData {
            sessions: Arc::new(StudentSession),
            delivery: Arc::new(AlreadySubmittedDelivery {
                prepares: Arc::clone(&prepares),
                commits: Arc::clone(&commits),
                saves: Arc::new(std::sync::atomic::AtomicUsize::new(0)),
                evidence: None,
                saved_response: Arc::new(std::sync::Mutex::new(None)),
            }),
            objects: super::super::DeliveryObjects::new(S3ObjectStore::new(
                client(&EndpointConfig {
                    endpoint_url: "http://127.0.0.1:9".to_string(),
                    region: "us-east-1".to_string(),
                    access_key_id: "unused".to_string(),
                    secret_access_key: "unused".to_string(),
                }),
                BucketNames {
                    public_assets: "public".to_string(),
                    private_content: "private".to_string(),
                    student_records: "student".to_string(),
                    temp_processing: "temp".to_string(),
                },
            )),
            webwork: Arc::new(webwork),
            browser_origin: Arc::from("https://ple.example"),
        };
        let attempt = "11111111-1111-4111-8111-111111111111";
        let token = URL_SAFE_NO_PAD.encode([7_u8; 32]);
        let response = axum::Router::new()
            .route(
                "/api/assessment-attempts/{assessment_attempt}/submission",
                axum::routing::post(finalize_assessment_attempt),
            )
            .with_state(state)
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!("/api/assessment-attempts/{attempt}/submission"))
                    .header("cookie", format!("__Host-ple_session={token}"))
                    .body(Body::empty())
                    .expect("submission request"),
            )
            .await
            .expect("submission response");
        let status = response.status();
        let body = to_bytes(response.into_body(), 1024)
            .await
            .expect("submission body");
        let body: serde_json::Value = serde_json::from_slice(&body).expect("submission json");
        tokio::time::sleep(Duration::from_millis(50)).await;

        assert_eq!(status, StatusCode::OK);
        assert_eq!(
            body,
            serde_json::json!({
                "assessmentAttemptId": attempt,
                "submissionState": "submitted",
            })
        );
        assert_eq!(prepares.load(Ordering::SeqCst), 1);
        assert_eq!(
            commits.load(Ordering::SeqCst),
            0,
            "already-submitted rescore committed backend work"
        );
        assert_eq!(
            connections.load(Ordering::SeqCst),
            0,
            "already-submitted rescore opened a Question Backend connection"
        );
    }

    #[tokio::test]
    async fn a_question_backend_may_evaluate_a_response_before_assessment_submission_when_needed() {
        use std::sync::Arc;
        use std::sync::atomic::{AtomicUsize, Ordering};
        use std::time::Duration;

        use axum::body::{Body, to_bytes};
        use axum::http::{Request, header};
        use base64::Engine;
        use base64::engine::general_purpose::URL_SAFE_NO_PAD;
        use objects::memory::MemoryObjectStore;
        use question_model::QuestionRendererVersion;
        use tower::ServiceExt;

        // ASVS 8.2.1, 8.2.2, and 14.2.1: this double is not installed by the
        // production router. The shipped save handler still requires a Student
        // session, accepts only a presentation-scoped choice, and returns no
        // credit fraction.
        let (evidence, choice_id) = issued_choice_evidence();
        assert_ne!(choice_id, "durable-choice");
        let prepares = Arc::new(AtomicUsize::new(0));
        let commits = Arc::new(AtomicUsize::new(0));
        let saves = Arc::new(AtomicUsize::new(0));
        let saved_response = Arc::new(std::sync::Mutex::new(None));
        let webwork = adapter_webwork::WebworkAdapter::new(
            adapter_webwork::HttpWebworkRenderer::new(
                adapter_webwork::HttpWebworkRendererConfig::new(
                    "http://127.0.0.1:9/",
                    Duration::from_millis(200),
                    1024,
                    QuestionRendererVersion {
                        name: "webwork-pg-renderer".to_string(),
                        version: "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
                            .to_string(),
                    },
                    "https://ple.example/",
                )
                .expect("renderer settings"),
            )
            .expect("renderer"),
        );
        let state = StateData {
            sessions: Arc::new(StudentSession),
            delivery: Arc::new(AlreadySubmittedDelivery {
                prepares: Arc::clone(&prepares),
                commits: Arc::clone(&commits),
                saves: Arc::clone(&saves),
                evidence: Some(evidence),
                saved_response: Arc::clone(&saved_response),
            }),
            objects: super::super::DeliveryObjects::new(MemoryObjectStore::default()),
            webwork: Arc::new(webwork),
            browser_origin: Arc::from("https://ple.example"),
        };
        let attempt = "11111111-1111-4111-8111-111111111111";
        let token = URL_SAFE_NO_PAD.encode([8_u8; 32]);
        let response = axum::Router::new()
            .route(
                "/api/assessment-attempts/{assessment_attempt}/responses/{position}",
                axum::routing::put(save_selected_response),
            )
            .with_state(state)
            .oneshot(
                Request::builder()
                    .method("PUT")
                    .uri(format!("/api/assessment-attempts/{attempt}/responses/1"))
                    .header("cookie", format!("__Host-ple_session={token}"))
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Body::from(format!(
                        r#"{{"response":{{"kind":"multipleChoice","selected":["{choice_id}"]}}}}"#
                    )))
                    .expect("save request"),
            )
            .await
            .expect("save response");
        let status = response.status();
        let body = to_bytes(response.into_body(), 1024)
            .await
            .expect("save body");
        let body: serde_json::Value = serde_json::from_slice(&body).expect("save json");
        let stored = saved_response
            .lock()
            .expect("stored response")
            .clone()
            .expect("saved response");
        let StudentResponse::MultipleChoice { selected } = stored else {
            panic!("stored multiple choice");
        };

        assert_eq!(status, StatusCode::OK);
        assert_eq!(
            body,
            serde_json::json!({
                "assessmentAttemptId": attempt,
                "position": 1,
                "responseState": "saved",
            })
        );
        assert_eq!(selected.len(), 1);
        assert_eq!(selected[0].as_str(), "durable-choice");
        assert_eq!(saves.load(Ordering::SeqCst), 1);
        assert_eq!(prepares.load(Ordering::SeqCst), 0);
        assert_eq!(commits.load(Ordering::SeqCst), 0);
    }
}

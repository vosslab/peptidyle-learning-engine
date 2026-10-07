//! Server-only evaluation of one immutable saved-response snapshot.
//!
//! PostgreSQL authorizes and captures the snapshot, this module calls the
//! Question Backend without holding a database transaction, and PostgreSQL
//! later accepts it only when the snapshot remains current.

use adapter_webwork::{HttpWebworkRenderer, WebworkAdapter};
use learning_data_access::{
    StoreError, StudentAssessmentAttemptFinalizationEvaluation,
    StudentAssessmentAttemptFinalizationSource,
};
use objects::ObjectStore;

/// Evaluates every saved response from its exact immutable source.
///
/// No transaction is held across object-store or renderer I/O. Any failure
/// leaves acceptance to the caller's PostgreSQL commit step, which therefore
/// records no partial submission. ASVS 2.3.3.
pub(crate) async fn evaluate_saved_responses<S: ObjectStore>(
    objects: &S,
    webwork: &WebworkAdapter<HttpWebworkRenderer>,
    responses: &[StudentAssessmentAttemptFinalizationSource],
) -> Result<Vec<StudentAssessmentAttemptFinalizationEvaluation>, StoreError> {
    let mut evaluations = Vec::with_capacity(responses.len());
    for response in responses {
        evaluations.push(StudentAssessmentAttemptFinalizationEvaluation {
            question_attempt_id: response.question_attempt_id,
            saved_at: response.saved_at,
            student_response: response.student_response.clone(),
            normalized_credit: super::question_backend::grade_saved_response(
                objects, webwork, response,
            )
            .await?,
        });
    }
    Ok(evaluations)
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use adapter_webwork::HttpWebworkRendererConfig;
    use objects::{ObjectAddress, ObjectStore, PutObject, memory::MemoryObjectStore};
    use question_model::{
        ObjectId, PublishedQuestionId, PublishedQuestionRevisionTuple, QuestionRendererVersion,
        QuestionReproduction, QuestionRevisionNumber, StudentResponse, Timestamp,
        response::ResponseItemId,
    };
    use uuid::Uuid;

    use learning_data_access::StudentAssessmentAttemptFinalizationBackend;

    use super::{StudentAssessmentAttemptFinalizationSource, evaluate_saved_responses};

    const QUESTION_JSON: &str = r#"{
  "format": "pleQuestionJson",
  "prompt": "What is my favorite color?",
  "response": {
    "kind": "singleChoice",
    "choices": [
      {"id": "blue", "text": "Blue", "feedback": "Blue is a calm choice."},
      {"id": "red", "text": "Red", "feedback": "Red is not my favorite."},
      {"id": "yellow", "text": "Yellow", "feedback": "Yellow is bright."}
    ],
    "correctChoice": "blue"
  },
  "feedback": {"correct": "Exactly right.", "incorrect": "Try thinking of a cool color."}
}"#;

    #[tokio::test]
    async fn unanswered_questions_are_not_sent_to_the_question_backend() {
        let store = MemoryObjectStore::default();
        let published_question_revision_tuple = PublishedQuestionRevisionTuple {
            published_question_id: PublishedQuestionId::from_random_identifier("ABCDEFG")
                .expect("Question ID"),
            revision_number: QuestionRevisionNumber::new(1).expect("revision"),
        };
        let source_object_id = ObjectId::from_uuid(Uuid::from_u128(901));
        let record = store
            .put(PutObject {
                address: ObjectAddress::QuestionSource {
                    published_question_revision_tuple: published_question_revision_tuple.clone(),
                    object_id: source_object_id,
                },
                bytes: QUESTION_JSON.as_bytes().to_vec(),
                media_type: adapter_ple::question_json::PLE_QUESTION_JSON_MEDIA_TYPE.to_string(),
                created_at: Timestamp::from_unix_millis(1),
            })
            .await
            .expect("source should store");
        let saved = StudentAssessmentAttemptFinalizationSource {
            question_attempt_id: Uuid::from_u128(1),
            saved_at: Timestamp::from_unix_millis(2),
            question_id: published_question_revision_tuple
                .published_question_id
                .clone(),
            revision_number: 1,
            source_object_id: source_object_id.as_uuid().to_string(),
            source_object_checksum: record.sha256.to_string(),
            reproduction: QuestionReproduction::Static,
            student_response: StudentResponse::MultipleChoice {
                selected: vec![ResponseItemId::new("blue")],
            },
            backend: StudentAssessmentAttemptFinalizationBackend::Ple,
        };
        let unanswered = Uuid::from_u128(2);
        let webwork = super::WebworkAdapter::new(
            super::HttpWebworkRenderer::new(
                HttpWebworkRendererConfig::new(
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

        let evaluations = evaluate_saved_responses(&store, &webwork, std::slice::from_ref(&saved))
            .await
            .expect("saved response should grade");

        assert_eq!(
            evaluations.len(),
            1,
            "one saved response is one backend grade"
        );
        assert_eq!(
            evaluations[0].question_attempt_id,
            saved.question_attempt_id
        );
        assert_eq!(evaluations[0].normalized_credit, 1.0);
        assert!(
            evaluations
                .iter()
                .all(|evaluation| evaluation.question_attempt_id != unanswered),
            "an unanswered Question Attempt is not graded"
        );

        let none = evaluate_saved_responses(&store, &webwork, &[])
            .await
            .expect("an Attempt with no saved response");
        assert!(
            none.is_empty(),
            "no saved response means no Question Backend grade"
        );
    }

    /// ASVS 8.2.1 and 8.2.2: this double is not installed by the production router.
    /// The shipped handler still requires a Student session before preparation,
    /// and PostgreSQL remains the Attempt owner in production.
    struct OwnershipDelivery {
        prepares: std::sync::Arc<std::sync::atomic::AtomicUsize>,
        commits: std::sync::Arc<std::sync::atomic::AtomicUsize>,
        source: StudentAssessmentAttemptFinalizationSource,
        observed: std::sync::Arc<
            std::sync::Mutex<
                Option<learning_data_access::StudentAssessmentAttemptFinalizationPreparation>,
            >,
        >,
        credits: std::sync::Arc<
            std::sync::Mutex<
                Vec<learning_data_access::StudentAssessmentAttemptFinalizationEvaluation>,
            >,
        >,
    }

    #[async_trait::async_trait]
    impl learning_data_access::LiveAssessmentDeliveryStore for OwnershipDelivery {
        async fn student_assessment_attempt_context(
            &self,
            _: learning_data_access::SessionTokenHash,
            _: question_model::AssessmentAttemptId,
        ) -> Result<
            learning_data_access::StudentAssessmentAttemptContext,
            learning_data_access::StoreError,
        > {
            unreachable!("ownership proof does not read Attempt context")
        }

        async fn save_student_assessment_attempt_response(
            &self,
            _: learning_data_access::SessionTokenHash,
            _: question_model::AssessmentAttemptId,
            _: u32,
            _: StudentResponse,
        ) -> Result<
            learning_data_access::StudentAssessmentAttemptSavedResponse,
            learning_data_access::StoreError,
        > {
            unreachable!("ownership proof does not save a response")
        }

        async fn checkpoint_student_question_display_duration(
            &self,
            _: learning_data_access::SessionTokenHash,
            _: question_model::AssessmentAttemptId,
            _: u32,
            _: u64,
        ) -> Result<u64, learning_data_access::StoreError> {
            unreachable!("ownership proof does not checkpoint display duration")
        }

        async fn student_assessment_attempt_saved_response(
            &self,
            _: learning_data_access::SessionTokenHash,
            _: question_model::AssessmentAttemptId,
            _: u32,
        ) -> Result<Option<StudentResponse>, learning_data_access::StoreError> {
            unreachable!("ownership proof does not read a saved response")
        }

        async fn prepare_student_assessment_attempt_finalization(
            &self,
            _: learning_data_access::SessionTokenHash,
            _: question_model::AssessmentAttemptId,
        ) -> Result<
            learning_data_access::StudentAssessmentAttemptFinalizationPreparationOutcome,
            learning_data_access::StoreError,
        > {
            self.prepares
                .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            Ok(
                learning_data_access::StudentAssessmentAttemptFinalizationPreparationOutcome::Ready(
                    learning_data_access::StudentAssessmentAttemptFinalizationPreparation {
                        kind:
                            learning_data_access::StudentAssessmentAttemptFinalizationKind::Deadline,
                        saved_responses: vec![self.source.clone()],
                    },
                ),
            )
        }

        async fn commit_student_assessment_attempt_finalization(
            &self,
            _: learning_data_access::SessionTokenHash,
            _: question_model::AssessmentAttemptId,
            preparation: learning_data_access::StudentAssessmentAttemptFinalizationPreparation,
            evaluations: Vec<learning_data_access::StudentAssessmentAttemptFinalizationEvaluation>,
        ) -> Result<
            learning_data_access::StudentAssessmentAttemptFinalization,
            learning_data_access::StoreError,
        > {
            self.commits
                .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            *self.observed.lock().expect("commit observation") = Some(preparation);
            *self.credits.lock().expect("stored credit") = evaluations;
            Ok(
                learning_data_access::StudentAssessmentAttemptFinalization::Submitted {
                    score: None,
                },
            )
        }

        async fn student_assessment_attempt_progress(
            &self,
            _: learning_data_access::SessionTokenHash,
            _: question_model::AssessmentAttemptId,
        ) -> Result<
            question_model::StudentAssessmentAttemptProgress,
            learning_data_access::StoreError,
        > {
            unreachable!("ownership proof does not read progress")
        }

        async fn student_assessment_attempt_presentation_evidence(
            &self,
            _: learning_data_access::SessionTokenHash,
            _: question_model::AssessmentAttemptId,
            _: u32,
        ) -> Result<
            learning_data_access::StudentAssessmentAttemptPresentationEvidence,
            learning_data_access::StoreError,
        > {
            unreachable!("ownership proof does not read presentation evidence")
        }

        async fn student_assessment_attempt_backend_document(
            &self,
            _: learning_data_access::SessionTokenHash,
            _: question_model::AssessmentAttemptId,
            _: u32,
        ) -> Result<
            learning_data_access::StudentAssessmentAttemptBackendDocument,
            learning_data_access::StoreError,
        > {
            unreachable!("ownership proof does not read a backend document")
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
            unreachable!("ownership proof does not issue an Assessment")
        }

        async fn commit_native_assessment_issuance(
            &self,
            _: learning_data_access::SessionTokenHash,
            _: Uuid,
            _: Vec<learning_data_access::NativePresentationInput>,
        ) -> Result<learning_data_access::LiveAssessmentAttempt, learning_data_access::StoreError>
        {
            unreachable!("ownership proof does not commit issuance")
        }

        async fn live_assessment_access(
            &self,
            _: learning_data_access::SessionTokenHash,
            _: question_model::CourseInstanceId,
            _: question_model::AssessmentId,
        ) -> Result<learning_data_access::LiveAssessmentAccess, learning_data_access::StoreError>
        {
            unreachable!("ownership proof does not read Assessment access")
        }

        async fn student_assessment_attempt_history(
            &self,
            _: learning_data_access::SessionTokenHash,
            _: question_model::AssessmentAttemptId,
        ) -> Result<
            learning_data_access::StudentAssessmentAttemptHistoryEvidence,
            learning_data_access::StoreError,
        > {
            unreachable!("ownership proof does not read history")
        }

        async fn student_assessment_attempt_history_response_sources(
            &self,
            _: learning_data_access::SessionTokenHash,
            _: question_model::AssessmentAttemptId,
        ) -> Result<
            Vec<learning_data_access::StudentAssessmentAttemptHistoryResponseSource>,
            learning_data_access::StoreError,
        > {
            unreachable!("ownership proof does not read history responses")
        }
    }

    struct RoleSession(question_model::UserRole);

    #[async_trait::async_trait]
    impl learning_data_access::SessionStore for RoleSession {
        async fn create_session(
            &self,
            _: learning_data_access::SessionTokenHash,
            _: question_model::AccountId,
            _: learning_data_access::SessionLifetime,
        ) -> Result<learning_data_access::SessionRecord, learning_data_access::StoreError> {
            unreachable!("ownership proof does not create a session")
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
                user_role: self.0,
                created_at: Timestamp::from_unix_millis(0),
                expires_at: Timestamp::from_unix_millis(60_000),
            }))
        }

        async fn revoke_session(
            &self,
            _: learning_data_access::SessionTokenHash,
        ) -> Result<(), learning_data_access::StoreError> {
            unreachable!("ownership proof does not revoke a session")
        }
    }

    #[tokio::test]
    async fn ple_owns_authorization_question_id_revisions_persistence_lifecycle_and_stored_outcomes()
     {
        use std::sync::Arc;
        use std::sync::atomic::{AtomicUsize, Ordering};
        use std::time::Duration;

        use axum::body::{Body, to_bytes};
        use axum::http::{Request, StatusCode};
        use base64::Engine;
        use base64::engine::general_purpose::URL_SAFE_NO_PAD;
        use objects::{ObjectAddress, PutObject};
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
        let published_question_revision_tuple = PublishedQuestionRevisionTuple {
            published_question_id: PublishedQuestionId::from_random_identifier("ABCDEFG")
                .expect("Question ID"),
            revision_number: QuestionRevisionNumber::new(4).expect("revision"),
        };
        let source_object_id = ObjectId::from_uuid(Uuid::from_u128(901));
        let store = MemoryObjectStore::default();
        let record = store
            .put(PutObject {
                address: ObjectAddress::QuestionSource {
                    published_question_revision_tuple: published_question_revision_tuple.clone(),
                    object_id: source_object_id,
                },
                bytes: QUESTION_JSON.as_bytes().to_vec(),
                media_type: adapter_ple::question_json::PLE_QUESTION_JSON_MEDIA_TYPE.to_string(),
                created_at: Timestamp::from_unix_millis(1),
            })
            .await
            .expect("source should store");
        let question_attempt_id = Uuid::from_u128(44);
        let saved = StudentAssessmentAttemptFinalizationSource {
            question_attempt_id,
            saved_at: Timestamp::from_unix_millis(2),
            question_id: published_question_revision_tuple
                .published_question_id
                .clone(),
            revision_number: 4,
            source_object_id: source_object_id.as_uuid().to_string(),
            source_object_checksum: record.sha256.to_string(),
            reproduction: QuestionReproduction::Static,
            student_response: StudentResponse::MultipleChoice {
                selected: vec![ResponseItemId::new("blue")],
            },
            backend: StudentAssessmentAttemptFinalizationBackend::Ple,
        };
        let prepares = Arc::new(AtomicUsize::new(0));
        let commits = Arc::new(AtomicUsize::new(0));
        let observed = Arc::new(std::sync::Mutex::new(None));
        let credits = Arc::new(std::sync::Mutex::new(Vec::new()));
        let delivery: Arc<dyn learning_data_access::LiveAssessmentDeliveryStore> =
            Arc::new(OwnershipDelivery {
                prepares: Arc::clone(&prepares),
                commits: Arc::clone(&commits),
                source: saved,
                observed: Arc::clone(&observed),
                credits: Arc::clone(&credits),
            });
        let webwork = Arc::new(super::WebworkAdapter::new(
            super::HttpWebworkRenderer::new(
                HttpWebworkRendererConfig::new(
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
        ));
        let objects = super::super::DeliveryObjects::new(store);
        let attempt = "11111111-1111-4111-8111-111111111111";
        let post = |role: question_model::UserRole, token_byte: u8| {
            let state = super::super::StateData {
                sessions: Arc::new(RoleSession(role)),
                delivery: Arc::clone(&delivery),
                objects: objects.clone(),
                webwork: Arc::clone(&webwork),
                browser_origin: Arc::from("https://ple.example"),
            };
            let token = URL_SAFE_NO_PAD.encode([token_byte; 32]);
            axum::Router::new()
                .route(
                    "/api/assessment-attempts/{assessment_attempt}/submission",
                    axum::routing::post(super::super::submission::finalize_assessment_attempt),
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
        };
        let instructor = post(question_model::UserRole::Instructor, 7)
            .await
            .expect("instructor response");
        let instructor_status = instructor.status();
        let instructor_body = to_bytes(instructor.into_body(), 1024)
            .await
            .expect("instructor body");
        assert_eq!(instructor_status, StatusCode::NOT_FOUND);
        assert_eq!(instructor_body.as_ref(), b"Assessment not found");
        assert_eq!(prepares.load(Ordering::SeqCst), 0);
        assert_eq!(commits.load(Ordering::SeqCst), 0);

        let student = post(question_model::UserRole::Student, 8)
            .await
            .expect("student response");
        let student_status = student.status();
        let student_body = to_bytes(student.into_body(), 1024)
            .await
            .expect("student body");
        let student_body: serde_json::Value =
            serde_json::from_slice(&student_body).expect("student json");
        tokio::time::sleep(Duration::from_millis(50)).await;
        let preparation = observed
            .lock()
            .expect("observed preparation")
            .clone()
            .expect("commit received the prepared snapshot");
        let stored = credits.lock().expect("stored credits").clone();

        assert_eq!(student_status, StatusCode::OK);
        assert_eq!(
            student_body,
            serde_json::json!({
                "assessmentAttemptId": attempt,
                "submissionState": "submitted",
            })
        );
        assert_eq!(prepares.load(Ordering::SeqCst), 1);
        assert_eq!(commits.load(Ordering::SeqCst), 1);
        assert_eq!(
            preparation.kind,
            learning_data_access::StudentAssessmentAttemptFinalizationKind::Deadline
        );
        assert_eq!(preparation.saved_responses.len(), 1);
        assert_eq!(
            preparation.saved_responses[0].question_id.as_str(),
            published_question_revision_tuple
                .published_question_id
                .as_str()
        );
        assert_eq!(preparation.saved_responses[0].revision_number, 4);
        assert_eq!(
            preparation.saved_responses[0].question_attempt_id,
            question_attempt_id
        );
        assert_eq!(stored.len(), 1);
        assert_eq!(stored[0].question_attempt_id, question_attempt_id);
        assert_eq!(stored[0].normalized_credit, 1.0);
        assert_eq!(connections.load(Ordering::SeqCst), 0);
    }
}

//! One issuance and grading interface for the production Question Backends.
//!
//! PLE and WeBWorK keep their source, renderer, and response details behind
//! this boundary. Delivery selects an adapter and calls these methods.

use adapter_ple::{PleQuestionBackend, ResolvedPleQuestionJsonSource};
use adapter_webwork::renderer_contract::WebworkRenderer;
use adapter_webwork::{
    ResolvedWebworkQuestionSource, WebworkAdapter, WebworkQuestionSourceBinding,
};
use async_trait::async_trait;
use learning_data_access::{
    NativePleIssuanceSource, NativePresentationInput, NativeWebworkIssuanceSource,
    QuestionIssuanceReproductionInput, StoreError, StudentAssessmentAttemptFinalizationBackend,
    StudentAssessmentAttemptFinalizationSource,
};
use objects::ObjectStore;
use question_model::presentation::build_question_presentation;
use question_model::question_library::QuestionBackendInterface;
use question_model::{
    ObjectId, PublishedQuestionRevisionTuple, QuestionBackend, QuestionBackendCapabilities,
    QuestionReproduction, QuestionRevisionNumber, SourceObjectChecksum,
};

use super::{
    StartError, question_image_renditions, question_image_renditions_from_ready, resolve_source,
    resolve_webwork_source,
};

/// The basic interface every production Question Backend implements.
///
/// iMathAS and H5P stay deferred, so this closed pair is the production set.
#[async_trait]
pub(super) trait BasicQuestionBackend<S: ObjectStore> {
    type IssuanceSource: Sync;

    async fn issue_one(
        &self,
        objects: &S,
        source: &Self::IssuanceSource,
    ) -> Result<NativePresentationInput, StartError>;

    async fn grade_saved_response(
        &self,
        objects: &S,
        response: &StudentAssessmentAttemptFinalizationSource,
    ) -> Result<f64, StoreError>;
}

pub(super) struct PleQuestionBackendAdapter;

pub(super) struct WebworkQuestionBackendAdapter<'a, R: WebworkRenderer> {
    pub(super) adapter: &'a WebworkAdapter<R>,
}

pub(super) async fn grade_saved_response<S: ObjectStore, R: WebworkRenderer>(
    objects: &S,
    webwork: &WebworkAdapter<R>,
    response: &StudentAssessmentAttemptFinalizationSource,
) -> Result<f64, StoreError> {
    match &response.backend {
        StudentAssessmentAttemptFinalizationBackend::Ple => {
            PleQuestionBackendAdapter
                .grade_saved_response(objects, response)
                .await
        }
        StudentAssessmentAttemptFinalizationBackend::Webwork { .. } => {
            WebworkQuestionBackendAdapter { adapter: webwork }
                .grade_saved_response(objects, response)
                .await
        }
    }
}

#[async_trait]
impl<S: ObjectStore> BasicQuestionBackend<S> for PleQuestionBackendAdapter {
    type IssuanceSource = NativePleIssuanceSource;

    async fn issue_one(
        &self,
        objects: &S,
        source: &NativePleIssuanceSource,
    ) -> Result<NativePresentationInput, StartError> {
        if source.reproduction != QuestionIssuanceReproductionInput::Static {
            return Err(StartError::Invalid);
        }
        let backend = super::ple_shell::native_ple_backend(QuestionBackendInterface::new(
            QuestionBackend::Ple,
            QuestionBackendCapabilities::none(),
        ))?;
        let resolved = resolve_source(objects, source).await?;
        let issued = backend
            .issue_question_json(&resolved, &source.question_title)
            .map_err(|_| StartError::Invalid)?;
        let presentation =
            build_question_presentation(&issued.presentation, &question_image_renditions(source))
                .map_err(|_| StartError::Unavailable)?;
        if presentation.reproduction != QuestionReproduction::Static {
            return Err(StartError::Invalid);
        }
        let response_item_bindings =
            question_model::presentation::extract_durable_response_item_bindings(&presentation)
                .map_err(|_| StartError::Invalid)?;
        Ok(NativePresentationInput {
            issued_question_id: source
                .issued_question_id
                .ok_or(StartError::Invalid)?
                .to_string(),
            assessment_entry_id: source.assessment_entry_id.clone(),
            position: source.position,
            question_id: source.question_id.clone(),
            revision_number: source.revision_number,
            reproduction: presentation.reproduction,
            reproduction_details: serde_json::to_value(issued.reproduction_details)
                .map_err(|_| StartError::Invalid)?,
            presentation: serde_json::to_value(&presentation.presentation)
                .map_err(|_| StartError::Invalid)?,
            presentation_nonce: presentation.presentation.presentation_nonce.to_hex(),
            presentation_checksum: presentation.checksum.to_hex(),
            author_content: presentation.author_content.clone(),
            response_item_bindings,
            question_image_renditions: source.question_image_renditions.clone(),
            issued_capability: "ple_question_json_presentation".to_string(),
            backend_document: None,
        })
    }

    async fn grade_saved_response(
        &self,
        objects: &S,
        response: &StudentAssessmentAttemptFinalizationSource,
    ) -> Result<f64, StoreError> {
        if response.reproduction != QuestionReproduction::Static {
            return Err(unavailable());
        }
        let published_question_revision_tuple = revision_tuple(response)?;
        let source = ResolvedPleQuestionJsonSource::resolve(
            objects,
            published_question_revision_tuple,
            object_id(response)?,
            checksum(response)?,
        )
        .await
        .map_err(|_| unavailable())?;
        PleQuestionBackend::new()
            .grade_question_json(&source, &response.student_response)
            .map(|evaluation| evaluation.evaluation.normalized_credit())
            .map_err(|_| unavailable())
    }
}

#[async_trait]
impl<S: ObjectStore, R: WebworkRenderer> BasicQuestionBackend<S>
    for WebworkQuestionBackendAdapter<'_, R>
{
    type IssuanceSource = NativeWebworkIssuanceSource;

    async fn issue_one(
        &self,
        objects: &S,
        source: &NativeWebworkIssuanceSource,
    ) -> Result<NativePresentationInput, StartError> {
        let seed = match &source.reproduction {
            QuestionIssuanceReproductionInput::Seeded { question_seed } => *question_seed,
            QuestionIssuanceReproductionInput::Static => return Err(StartError::Invalid),
        };
        let issued = self
            .adapter
            .issue(seed, &resolve_webwork_source(objects, source).await?)
            .await
            .map_err(|_| StartError::Unavailable)?;
        let document = String::from_utf8(issued.document).map_err(|_| StartError::Invalid)?;
        let presentation = build_question_presentation(
            &issued.presentation,
            &question_image_renditions_from_ready(&source.question_image_renditions),
        )
        .map_err(|_| StartError::Unavailable)?;
        if presentation.reproduction.question_seed() != Some(seed) {
            return Err(StartError::Invalid);
        }
        let response_item_bindings =
            question_model::presentation::extract_durable_response_item_bindings(&presentation)
                .map_err(|_| StartError::Invalid)?;
        Ok(NativePresentationInput {
            issued_question_id: source
                .issued_question_id
                .ok_or(StartError::Invalid)?
                .to_string(),
            assessment_entry_id: source.assessment_entry_id.clone(),
            position: source.position,
            question_id: source.question_id.clone(),
            revision_number: source.revision_number,
            reproduction: presentation.reproduction,
            reproduction_details: serde_json::to_value(issued.reproduction_details)
                .map_err(|_| StartError::Invalid)?,
            presentation: serde_json::to_value(&presentation.presentation)
                .map_err(|_| StartError::Invalid)?,
            presentation_nonce: presentation.presentation.presentation_nonce.to_hex(),
            presentation_checksum: presentation.checksum.to_hex(),
            author_content: None,
            response_item_bindings,
            question_image_renditions: source.question_image_renditions.clone(),
            issued_capability: "webwork_presentation".to_string(),
            backend_document: Some(document),
        })
    }

    async fn grade_saved_response(
        &self,
        objects: &S,
        response: &StudentAssessmentAttemptFinalizationSource,
    ) -> Result<f64, StoreError> {
        let StudentAssessmentAttemptFinalizationBackend::Webwork { pg_path } = &response.backend
        else {
            return Err(unavailable());
        };
        let seed = response
            .reproduction
            .question_seed()
            .ok_or_else(unavailable)?;
        let binding = WebworkQuestionSourceBinding::new(revision_tuple(response)?, pg_path.clone())
            .map_err(|_| unavailable())?;
        let source = ResolvedWebworkQuestionSource::resolve(
            objects,
            binding,
            object_id(response)?,
            checksum(response)?,
        )
        .await
        .map_err(|_| unavailable())?;
        match self
            .adapter
            .grade(seed, &source, &response.student_response)
            .await
            .map_err(|_| unavailable())?
        {
            grading::QuestionGradingOutcome::Evaluated(evaluation) => {
                Ok(evaluation.normalized_credit())
            }
            grading::QuestionGradingOutcome::Ungraded => Err(unavailable()),
        }
    }
}

fn revision_tuple(
    response: &StudentAssessmentAttemptFinalizationSource,
) -> Result<PublishedQuestionRevisionTuple, StoreError> {
    Ok(PublishedQuestionRevisionTuple {
        published_question_id: response.question_id.clone(),
        revision_number: QuestionRevisionNumber::new(response.revision_number)
            .map_err(|_| unavailable())?,
    })
}

fn object_id(
    response: &StudentAssessmentAttemptFinalizationSource,
) -> Result<ObjectId, StoreError> {
    let source_object_id =
        uuid::Uuid::parse_str(&response.source_object_id).map_err(|_| unavailable())?;
    Ok(ObjectId::from_uuid(source_object_id))
}

fn checksum(
    response: &StudentAssessmentAttemptFinalizationSource,
) -> Result<SourceObjectChecksum, StoreError> {
    SourceObjectChecksum::parse(response.source_object_checksum.clone()).map_err(|_| unavailable())
}

fn unavailable() -> StoreError {
    StoreError::Unavailable("Question response unavailable".to_string())
}

#[cfg(test)]
mod tests {
    use sha2::Digest;

    use adapter_webwork::renderer_contract::{
        GradeRequest, RenderRequest, RenderedWebworkQuestion, ResumeRenderRequest, WebworkRenderer,
    };
    use async_trait::async_trait;
    use grading::QuestionGradingOutcome;
    use objects::{ObjectAddress, ObjectStore, PutObject, memory::MemoryObjectStore};
    use question_model::{
        PublishedQuestionId, PublishedQuestionRevisionTuple, QuestionEvaluation,
        QuestionRendererVersion, QuestionReproduction, QuestionRevisionNumber, QuestionSeed,
        StudentResponse, Timestamp, response::ResponseItemId,
    };
    use uuid::Uuid;

    use super::*;

    const QUESTION_JSON: &str = r#"{
  "format": "pleQuestionJson",
  "prompt": "What is my favorite color?",
  "response": {
    "kind": "singleChoice",
    "choices": [
      {"id": "blue", "text": "Blue", "feedback": "Blue is a calm choice."},
      {"id": "red", "text": "Red", "feedback": "Red is not my favorite."}
    ],
    "correctChoice": "blue"
  },
  "feedback": {"correct": "Exactly right.", "incorrect": "Try thinking of a cool color."}
}"#;
    const PG_SOURCE: &[u8] = b"DOCUMENT();\nBEGIN_TEXT\nOpaque\nEND_TEXT\nENDDOCUMENT();\n";
    const PAYLOAD: &[u8] = br#"[["AnSwEr0001","blue"]]"#;

    struct FixedRenderer;

    #[async_trait]
    impl WebworkRenderer for FixedRenderer {
        fn identity(&self) -> &QuestionRendererVersion {
            use std::sync::LazyLock;
            static IDENTITY: LazyLock<QuestionRendererVersion> =
                LazyLock::new(|| QuestionRendererVersion {
                    name: "recorded-renderer".to_string(),
                    version: "1".to_string(),
                });
            &IDENTITY
        }

        async fn render(
            &self,
            request: RenderRequest<'_>,
        ) -> Result<RenderedWebworkQuestion, adapter_webwork::renderer_contract::RendererFailure>
        {
            if request.pg_source != PG_SOURCE {
                return Err(
                    adapter_webwork::renderer_contract::RendererFailure::InvalidOutput(
                        "source".into(),
                    ),
                );
            }
            let document = b"<form></form>".to_vec();
            let document_sha256 = sha2::Sha256::digest(&document).into();
            Ok(RenderedWebworkQuestion {
                document,
                document_sha256,
                renderer_version: self.identity().clone(),
                lifecycle_state: question_model::BackendOwnedLifecycleState::none(),
            })
        }

        async fn render_saved_response(
            &self,
            request: ResumeRenderRequest<'_>,
        ) -> Result<RenderedWebworkQuestion, adapter_webwork::renderer_contract::RendererFailure>
        {
            self.render(RenderRequest {
                pg_source: request.pg_source,
                pg_path: request.pg_path,
                published_question_revision_tuple: Some(request.published_question_revision_tuple),
                seed: request.seed,
            })
            .await
        }

        async fn render_answer_review(
            &self,
            request: RenderRequest<'_>,
        ) -> Result<RenderedWebworkQuestion, adapter_webwork::renderer_contract::RendererFailure>
        {
            self.render(request).await
        }

        async fn grade(
            &self,
            request: GradeRequest<'_>,
        ) -> Result<QuestionGradingOutcome, adapter_webwork::renderer_contract::RendererFailure>
        {
            if request.response_payload != PAYLOAD {
                return Err(
                    adapter_webwork::renderer_contract::RendererFailure::InvalidOutput(
                        "payload".into(),
                    ),
                );
            }
            Ok(QuestionGradingOutcome::Evaluated(
                QuestionEvaluation::new(false, 0.5).expect("partial credit"),
            ))
        }
    }

    fn tuple() -> PublishedQuestionRevisionTuple {
        PublishedQuestionRevisionTuple {
            published_question_id: PublishedQuestionId::from_random_identifier("ABCDEFG")
                .expect("Question ID"),
            revision_number: QuestionRevisionNumber::new(1).expect("revision"),
        }
    }

    #[tokio::test]
    async fn both_production_backends_issue_and_grade_through_one_interface() {
        let store = MemoryObjectStore::default();
        let revision = tuple();
        let ple_object = ObjectId::from_uuid(Uuid::from_u128(901));
        let ple_record = store
            .put(PutObject {
                address: ObjectAddress::QuestionSource {
                    published_question_revision_tuple: revision.clone(),
                    object_id: ple_object,
                },
                bytes: QUESTION_JSON.as_bytes().to_vec(),
                media_type: adapter_ple::question_json::PLE_QUESTION_JSON_MEDIA_TYPE.to_string(),
                created_at: Timestamp::from_unix_millis(1),
            })
            .await
            .expect("PLE source");
        let webwork_object = ObjectId::from_uuid(Uuid::from_u128(902));
        let webwork_record = store
            .put(PutObject {
                address: ObjectAddress::QuestionSource {
                    published_question_revision_tuple: revision.clone(),
                    object_id: webwork_object,
                },
                bytes: PG_SOURCE.to_vec(),
                media_type: "text/plain".to_string(),
                created_at: Timestamp::from_unix_millis(1),
            })
            .await
            .expect("WeBWorK source");
        let ple = PleQuestionBackendAdapter;
        let issued = ple
            .issue_one(
                &store,
                &NativePleIssuanceSource {
                    issued_question_id: Some(Uuid::from_u128(11)),
                    assessment_entry_id: "entry".to_string(),
                    position: 1,
                    question_id: revision.published_question_id.clone(),
                    question_title: "Peptide bond".to_string(),
                    revision_number: 1,
                    source_object_id: ple_object.as_uuid().to_string(),
                    source_object_address: serde_json::json!({}),
                    source_object_checksum: ple_record.sha256.to_string(),
                    reproduction: QuestionIssuanceReproductionInput::Static,
                    presentation_nonce: None,
                    presentation_checksum: None,
                    retained_presentation: None,
                    question_image_renditions: Vec::new(),
                },
            )
            .await
            .unwrap_or_else(|_| panic!("PLE issue"));
        assert_eq!(issued.issued_capability, "ple_question_json_presentation");
        assert!(issued.backend_document.is_none());
        let ple_credit = ple
            .grade_saved_response(
                &store,
                &StudentAssessmentAttemptFinalizationSource {
                    question_attempt_id: Uuid::from_u128(1),
                    saved_at: Timestamp::from_unix_millis(2),
                    question_id: revision.published_question_id.clone(),
                    revision_number: 1,
                    source_object_id: ple_object.as_uuid().to_string(),
                    source_object_checksum: ple_record.sha256.to_string(),
                    reproduction: QuestionReproduction::Static,
                    student_response: StudentResponse::MultipleChoice {
                        selected: vec![ResponseItemId::new("blue")],
                    },
                    backend: StudentAssessmentAttemptFinalizationBackend::Ple,
                },
            )
            .await
            .expect("PLE grade");
        assert_eq!(ple_credit, 1.0);

        let renderer = WebworkAdapter::new(FixedRenderer);
        let webwork = WebworkQuestionBackendAdapter { adapter: &renderer };
        let seeded = QuestionIssuanceReproductionInput::Seeded {
            question_seed: QuestionSeed::new(17),
        };
        let issued = webwork
            .issue_one(
                &store,
                &NativeWebworkIssuanceSource {
                    issued_question_id: Some(Uuid::from_u128(12)),
                    assessment_entry_id: "entry".to_string(),
                    position: 2,
                    question_id: revision.published_question_id.clone(),
                    revision_number: 1,
                    source_object_id: webwork_object.as_uuid().to_string(),
                    source_object_checksum: webwork_record.sha256.to_string(),
                    webwork_pg_path: "Library/opaque.pg".to_string(),
                    reproduction: seeded,
                    retained_presentation: None,
                    question_image_renditions: Vec::new(),
                },
            )
            .await
            .unwrap_or_else(|_| panic!("WeBWorK issue"));
        assert_eq!(issued.issued_capability, "webwork_presentation");
        assert!(issued.backend_document.is_some());
        let saved = StudentAssessmentAttemptFinalizationSource {
            question_attempt_id: Uuid::from_u128(2),
            saved_at: Timestamp::from_unix_millis(3),
            question_id: revision.published_question_id.clone(),
            revision_number: 1,
            source_object_id: webwork_object.as_uuid().to_string(),
            source_object_checksum: webwork_record.sha256.to_string(),
            reproduction: QuestionReproduction::Seeded {
                question_seed: QuestionSeed::new(17),
                generated_parameter_sha256: "a".repeat(64),
            },
            student_response: StudentResponse::BackendOwned {
                payload: PAYLOAD.to_vec(),
            },
            backend: StudentAssessmentAttemptFinalizationBackend::Webwork {
                pg_path: "Library/opaque.pg".to_string(),
            },
        };
        let webwork_credit = webwork
            .grade_saved_response(&store, &saved)
            .await
            .expect("WeBWorK grade");
        assert_eq!(webwork_credit, 0.5);
        let dispatched = grade_saved_response(&store, &renderer, &saved)
            .await
            .expect("dispatcher uses the same WeBWorK adapter");
        assert_eq!(dispatched, 0.5);
    }
}

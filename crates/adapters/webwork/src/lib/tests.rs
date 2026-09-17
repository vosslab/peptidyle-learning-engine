use async_trait::async_trait;
use grading::QuestionGradingOutcome;
use objects::memory::MemoryObjectStore;
use objects::{ObjectAddress, ObjectStore, PutObject, Sha256Checksum};
use question_model::generation::QuestionSeed;
use question_model::{
    BackendOwnedLifecycleState, ObjectId, QuestionEvaluation, QuestionId, QuestionRendererVersion,
    QuestionRevisionNumber, QuestionRevisionReference, SourceObjectChecksum, SourceObjectReference,
    StudentResponse, Timestamp,
};
use uuid::Uuid;

use super::*;
use crate::WebworkQuestionSourceBinding;
use crate::renderer_contract::{
    GradeRequest, RenderRequest, RenderedWebworkQuestion, ResumeRenderRequest,
};

const SOURCE: &[u8] =
    b"DOCUMENT();\nBEGIN_TEXT\nOpaque backend question\nEND_TEXT\nENDDOCUMENT();\n";
const DOCUMENT: &[u8] = b"<!doctype html><form><input name=AnSwEr0001></form>";
const PAYLOAD: &[u8] = br#"[["AnSwEr0001","student value"],["hidden","1"]]"#;

fn question_revision() -> QuestionRevisionReference {
    QuestionRevisionReference {
        question_id: QuestionId::from_random_identifier("ABCDEFG").expect("Question ID"),
        revision_number: QuestionRevisionNumber::new(2).expect("positive version"),
    }
}

#[derive(Clone)]
struct RecordedRenderer {
    identity: QuestionRendererVersion,
    lifecycle_state: BackendOwnedLifecycleState,
}

#[async_trait]
impl WebworkRenderer for RecordedRenderer {
    fn identity(&self) -> &QuestionRendererVersion {
        &self.identity
    }

    async fn render_answer_review(
        &self,
        request: RenderRequest<'_>,
    ) -> Result<RenderedWebworkQuestion, RendererFailure> {
        assert_eq!(request.seed, 17);
        assert_eq!(request.question_revision, &question_revision());
        self.render(request).await
    }

    async fn render(
        &self,
        request: RenderRequest<'_>,
    ) -> Result<RenderedWebworkQuestion, RendererFailure> {
        if request.pg_source != SOURCE || request.pg_path != "Library/opaque.pg" {
            return Err(RendererFailure::InvalidOutput(
                "recorded source did not match issuance".to_string(),
            ));
        }
        let mut rendered =
            RenderedWebworkQuestion::from_document(DOCUMENT.to_vec(), self.identity.clone());
        rendered.lifecycle_state = self.lifecycle_state.clone();
        Ok(rendered)
    }

    async fn render_saved_response(
        &self,
        request: ResumeRenderRequest<'_>,
    ) -> Result<RenderedWebworkQuestion, RendererFailure> {
        if request.pg_source != SOURCE
            || request.pg_path != "Library/opaque.pg"
            || request.response_payload != PAYLOAD
        {
            return Err(RendererFailure::InvalidOutput(
                "recorded source or response did not match resume".to_string(),
            ));
        }
        let mut rendered =
            RenderedWebworkQuestion::from_document(DOCUMENT.to_vec(), self.identity.clone());
        rendered.lifecycle_state = self.lifecycle_state.clone();
        Ok(rendered)
    }

    async fn grade(
        &self,
        request: GradeRequest<'_>,
    ) -> Result<QuestionGradingOutcome, RendererFailure> {
        if request.pg_source != SOURCE
            || request.pg_path != "Library/opaque.pg"
            || request.response_payload != PAYLOAD
            || request.lifecycle_state.as_deref().is_some()
        {
            return Err(RendererFailure::InvalidOutput(
                "recorded opaque grade request did not match".to_string(),
            ));
        }
        Ok(QuestionGradingOutcome::Evaluated(
            QuestionEvaluation::new(true, 1.0).expect("fixed evaluation is valid"),
        ))
    }
}

fn recorded_renderer() -> RecordedRenderer {
    RecordedRenderer {
        identity: QuestionRendererVersion {
            name: "recorded-renderer".to_string(),
            version: "1".to_string(),
        },
        lifecycle_state: BackendOwnedLifecycleState::none(),
    }
}

#[derive(Clone)]
struct NativeResponseRejectingRenderer {
    identity: QuestionRendererVersion,
}

#[async_trait]
impl WebworkRenderer for NativeResponseRejectingRenderer {
    fn identity(&self) -> &QuestionRendererVersion {
        &self.identity
    }

    async fn render_answer_review(
        &self,
        _: RenderRequest<'_>,
    ) -> Result<RenderedWebworkQuestion, RendererFailure> {
        panic!("native PLE response validation must reject before review")
    }

    async fn render(
        &self,
        _: RenderRequest<'_>,
    ) -> Result<RenderedWebworkQuestion, RendererFailure> {
        panic!("native PLE response validation must reject before rendering")
    }

    async fn render_saved_response(
        &self,
        _: ResumeRenderRequest<'_>,
    ) -> Result<RenderedWebworkQuestion, RendererFailure> {
        panic!("native PLE response validation must reject before resuming")
    }

    async fn grade(&self, _: GradeRequest<'_>) -> Result<QuestionGradingOutcome, RendererFailure> {
        panic!("native PLE response validation must reject before grading")
    }
}

async fn source(store: &MemoryObjectStore) -> ResolvedWebworkQuestionSource {
    let binding =
        WebworkQuestionSourceBinding::new(question_revision(), "Library/opaque.pg".to_string())
            .expect("fixed path is valid");
    let source_object_reference = SourceObjectReference {
        object: ObjectId::from_uuid(Uuid::from_u128(4)),
    };
    let source_object_checksum =
        SourceObjectChecksum::parse(Sha256Checksum::compute(SOURCE).to_string())
            .expect("checksum is canonical");
    store
        .put(PutObject {
            address: ObjectAddress::QuestionSource {
                question_revision: binding.question_revision().clone(),
                object: source_object_reference.object,
            },
            bytes: SOURCE.to_vec(),
            media_type: "text/x-wework-pg".to_string(),
            created_at: Timestamp::from_unix_millis(1),
        })
        .await
        .expect("source stores under immutable key");
    ResolvedWebworkQuestionSource::resolve(
        store,
        binding,
        source_object_reference,
        source_object_checksum,
    )
    .await
    .expect("source resolves through trusted storage")
}

#[tokio::test]
/// Protects exact source/seed review and the stateless document-only adapter contract.
async fn answer_review_returns_only_the_bound_document_and_refuses_lifecycle_state() {
    let source = source(&MemoryObjectStore::default()).await;
    let mut renderer = recorded_renderer();
    let adapter = WebworkAdapter::new(renderer.clone());
    assert_eq!(
        adapter
            .answer_review_document(QuestionSeed::new(17), &source)
            .await
            .unwrap(),
        DOCUMENT
    );
    renderer.lifecycle_state = BackendOwnedLifecycleState::from_bytes(vec![1]).unwrap();
    assert!(
        WebworkAdapter::new(renderer)
            .answer_review_document(QuestionSeed::new(17), &source)
            .await
            .is_err()
    );
}

#[tokio::test]
/// Prevents a backend-owned attempt from acquiring PLE control semantics.
async fn opaque_lifecycle_issues_and_grades_one_backend_owned_payload() {
    let store = MemoryObjectStore::default();
    let source = source(&store).await;
    let adapter = WebworkAdapter::new(recorded_renderer());

    let issued = adapter
        .issue(QuestionSeed::new(17), &source)
        .await
        .expect("renderer issues one document");
    assert_eq!(issued.document, DOCUMENT);
    assert_eq!(issued.document_sha256.len(), 32);
    assert!(issued.lifecycle_state.as_deref().is_none());
    assert!(matches!(
        issued.presentation.response,
        QuestionResponseFormat::BackendOwned {}
    ));
    assert!(issued.presentation.prompt.is_empty());

    let outcome = adapter
        .grade(
            QuestionSeed::new(17),
            &source,
            &StudentResponse::BackendOwned {
                payload: PAYLOAD.to_vec(),
            },
        )
        .await
        .expect("renderer grades exactly the opaque payload");
    assert!(matches!(outcome, QuestionGradingOutcome::Evaluated(result) if result.correct()));
}

#[tokio::test]
/// Prevents stateful renderer behavior from silently violating the selected E1 stateless lifecycle.
async fn issuance_refuses_renderer_lifecycle_state_for_stateless_webwork() {
    let store = MemoryObjectStore::default();
    let source = source(&store).await;
    let adapter = WebworkAdapter::new(RecordedRenderer {
        lifecycle_state: BackendOwnedLifecycleState::from_bytes(vec![1])
            .expect("fixed lifecycle state is bounded"),
        ..recorded_renderer()
    });

    let result = adapter.issue(QuestionSeed::new(17), &source).await;

    assert_eq!(
        result,
        Err(WebworkAdapterError::Renderer(
            RendererFailure::InvalidOutput(
                "WeBWorK renderer returned unexpected lifecycle state; this integration grades one submission without renderer-issued state"
                    .to_string(),
            )
        ))
    );
}

#[tokio::test]
/// Prevents native PLE response types from reaching the WeBWorK grading boundary.
async fn opaque_grading_refuses_native_ple_responses_without_a_renderer_call() {
    let store = MemoryObjectStore::default();
    let source = source(&store).await;
    let adapter = WebworkAdapter::new(NativeResponseRejectingRenderer {
        identity: QuestionRendererVersion {
            name: "native-response-rejecting-renderer".to_string(),
            version: "1".to_string(),
        },
    });
    let result = adapter
        .grade(
            QuestionSeed::new(17),
            &source,
            &StudentResponse::MultipleChoice {
                selected: Vec::new(),
            },
        )
        .await;
    assert!(matches!(result, Err(WebworkAdapterError::Renderer(_))));
}

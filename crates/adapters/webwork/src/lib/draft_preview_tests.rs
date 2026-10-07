use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use grading::QuestionGradingOutcome;
use question_model::{
    QuestionEvaluation, QuestionRendererVersion, StudentResponse, generation::QuestionSeed,
};

use crate::renderer_contract::{
    GradeRequest, RenderRequest, RenderedWebworkQuestion, RendererFailure, ResumeRenderRequest,
    WebworkRenderer,
};

#[derive(Default)]
struct Calls {
    render_tuple_present: Vec<bool>,
    grade_tuple_present: Vec<bool>,
    paths: Vec<String>,
}

struct RecordingRenderer {
    calls: Arc<Mutex<Calls>>,
    identity: QuestionRendererVersion,
}

#[async_trait]
impl WebworkRenderer for RecordingRenderer {
    fn identity(&self) -> &QuestionRendererVersion {
        &self.identity
    }

    async fn render(
        &self,
        request: RenderRequest<'_>,
    ) -> Result<RenderedWebworkQuestion, RendererFailure> {
        let mut calls = self.calls.lock().expect("calls mutex");
        calls
            .render_tuple_present
            .push(request.published_question_revision_tuple.is_some());
        calls.paths.push(request.pg_path.to_owned());
        Ok(RenderedWebworkQuestion::from_document(
            b"<!doctype html><form><input name=\"AnSwEr0001\"></form>".to_vec(),
            self.identity.clone(),
        ))
    }

    async fn render_saved_response(
        &self,
        _request: ResumeRenderRequest<'_>,
    ) -> Result<RenderedWebworkQuestion, RendererFailure> {
        Err(RendererFailure::Unavailable)
    }

    async fn render_answer_review(
        &self,
        _request: RenderRequest<'_>,
    ) -> Result<RenderedWebworkQuestion, RendererFailure> {
        Err(RendererFailure::Unavailable)
    }

    async fn grade(
        &self,
        request: GradeRequest<'_>,
    ) -> Result<QuestionGradingOutcome, RendererFailure> {
        let mut calls = self.calls.lock().expect("calls mutex");
        calls
            .grade_tuple_present
            .push(request.published_question_revision_tuple.is_some());
        calls.paths.push(request.pg_path.to_owned());
        assert!(request.lifecycle_state.as_deref().is_none());
        assert!(!request.response_payload.is_empty());
        Ok(QuestionGradingOutcome::Evaluated(
            QuestionEvaluation::new(true, 1.0).expect("valid evaluation"),
        ))
    }
}

#[tokio::test]
async fn draft_preview_and_test_use_registered_path_without_published_identity() {
    let calls = Arc::new(Mutex::new(Calls::default()));
    let renderer = RecordingRenderer {
        calls: Arc::clone(&calls),
        identity: QuestionRendererVersion {
            name: "test-renderer".into(),
            version: "1".into(),
        },
    };
    let adapter = crate::WebworkAdapter::new(renderer);
    let pg_source = b"DOCUMENT();\nTEXT(beginproblem());";
    let pg_path = "Library/Genetics/draft.pg";

    let document = adapter
        .preview_draft_document(QuestionSeed::new(17), pg_source, pg_path)
        .await
        .expect("Draft render");
    assert!(document.starts_with(b"<!doctype html>"));

    let response = StudentResponse::BackendOwned {
        payload: br#"[["AnSwEr0001","genotype"]]"#.to_vec(),
    };
    let grade = adapter
        .grade_draft_response(QuestionSeed::new(17), pg_source, pg_path, &response)
        .await
        .expect("Draft test grade");
    assert!(matches!(grade, QuestionGradingOutcome::Evaluated(value) if value.correct()));

    let calls = calls.lock().expect("calls mutex");
    assert_eq!(calls.render_tuple_present, [false]);
    assert_eq!(calls.grade_tuple_present, [false]);
    assert_eq!(calls.paths, [pg_path, pg_path]);
}

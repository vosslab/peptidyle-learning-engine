use super::*;

#[test]
fn backend_selection_uses_registered_native_and_webwork_formats() {
    assert_eq!(
        select_backend(QuestionBackend::Ple, QuestionFormat::PleQuestionJson, None)
            .expect("Native binding"),
        DraftPreviewBackend::Native
    );
    assert_eq!(
        select_backend(
            QuestionBackend::Webwork,
            QuestionFormat::WebworkPg,
            Some("Library/Genetics/question.pg")
        )
        .expect("PG binding"),
        DraftPreviewBackend::Webwork {
            pg_path: "Library/Genetics/question.pg"
        }
    );
    assert_eq!(
        select_backend(
            QuestionBackend::Webwork,
            QuestionFormat::WebworkPgml,
            Some("Library/Genetics/question.pgml")
        )
        .expect("PGML binding"),
        DraftPreviewBackend::Webwork {
            pg_path: "Library/Genetics/question.pgml"
        }
    );
}

#[test]
fn backend_selection_rejects_incoherent_and_unsupported_bindings() {
    assert!(select_backend(QuestionBackend::Ple, QuestionFormat::WebworkPg, None).is_err());
    assert!(select_backend(QuestionBackend::Webwork, QuestionFormat::WebworkPg, None).is_err());
    assert!(matches!(
        select_backend(QuestionBackend::Imathas, QuestionFormat::Imathas, None),
        Err(DraftPreviewError::UnsupportedBackend)
    ));
}

#[test]
fn test_operation_contract_is_transient_and_draft_scoped() {
    // The operation surface accepts only a Draft UUID/Edit Number and backend response. It has
    // no publication tuple, Attempt, Student Work, or write-store parameter.
    fn accepts_draft_operation(_: &dyn DraftQuestionPreviewOperations) {}

    struct ReadOnlyShape;
    #[async_trait::async_trait]
    impl DraftQuestionPreviewOperations for ReadOnlyShape {
        async fn preview(
            &self,
            _: SessionTokenHash,
            _: DraftQuestionUuid,
            _: DraftQuestionEditNumber,
            _: Option<QuestionSeed>,
        ) -> Result<DraftPreviewOutput, DraftPreviewError> {
            Ok(DraftPreviewOutput::Webwork(Vec::new()))
        }

        async fn test(
            &self,
            _: SessionTokenHash,
            _: DraftQuestionUuid,
            _: DraftQuestionEditNumber,
            _: Option<QuestionSeed>,
            _: StudentResponse,
        ) -> Result<DraftTestResponse, DraftPreviewError> {
            Ok(DraftTestResponse::Ungraded {})
        }
    }

    accepts_draft_operation(&ReadOnlyShape);
}

#[tokio::test]
async fn webwork_preview_failures_remain_visible_inside_the_existing_sandbox_policy() {
    let response = preview_document_error(DraftPreviewError::Unavailable);
    assert_eq!(
        response.status(),
        axum::http::StatusCode::SERVICE_UNAVAILABLE
    );
    let csp = response
        .headers()
        .get("content-security-policy")
        .and_then(|value| value.to_str().ok())
        .expect("preview CSP");
    assert!(csp.starts_with("sandbox allow-scripts;"));
    assert!(!csp.contains("allow-forms"));
    assert!(!csp.contains("allow-same-origin"));
    assert_eq!(response.headers().get("cache-control").unwrap(), "no-store");
    let body = axum::body::to_bytes(response.into_body(), 4096)
        .await
        .expect("bounded error document");
    let body = String::from_utf8(body.to_vec()).expect("UTF-8 error document");
    assert!(body.contains("Draft preview or testing is unavailable"));
    assert!(body.contains("saved Draft work was preserved"));
}

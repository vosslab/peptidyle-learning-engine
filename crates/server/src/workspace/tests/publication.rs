use super::{
    BackendCapabilities, Capability, DraftQuestionSource, FixtureRegistry, PageRequest, PageSize,
    TenantContext, TimingPolicy, WorkspaceId, draft, id, issued_cookie, response_json, router,
    test_router,
};
use axum::body::Body;
use axum::http::header::ETAG;
use axum::http::{Request, StatusCode};
use learning_data_access::in_memory::MemoryStore;
use learning_data_access::{CatalogStore, DraftRecord, Store};
use question_model::{ProblemId, ProblemVersionRef, TenantId, UserId, UserRole, VersionId};
use std::sync::Arc;
use tower::ServiceExt;

#[tokio::test]
async fn publication_validation_and_diff_use_persisted_draft_safe_semantics() {
    let store = Arc::new(MemoryStore::default());
    let tenant = TenantId::from_uuid(id(60));
    let actor = UserId::from_uuid(id(61));
    let prior_workspace = WorkspaceId::from_uuid(id(62));
    let workspace = WorkspaceId::from_uuid(id(63));
    let prior_reference = ProblemVersionRef {
        problem: ProblemId::from_uuid(id(64)),
        version: VersionId::from_uuid(id(65)),
    };
    let context = TenantContext::from_authenticated_session(tenant);
    let cookie = issued_cookie(&store, tenant, vec![UserRole::Instructor], actor).await;
    let prior_draft = DraftRecord {
        tenant,
        question: draft(prior_workspace, "Earlier peptide question"),
        derived_from: None,
    };
    let saved = store
        .upsert_draft(context, actor, None, prior_draft.clone())
        .await
        .expect("prior draft save");
    store
        .publish_draft(
            context,
            actor,
            learning_data_access::PublishDraftCommand {
                expected_draft: prior_draft,
                expected_revision: saved.revision,
                publication: prior_reference,
                published_source: question_model::QuestionSource::Native {
                    family: "workspace-fixture".to_string(),
                },
                source_artifact: None,
                qti_promotion: None,
                flat_question_promotion: None,
                publisher: actor,
                scope: question_model::PublicationScope::Public,
                byline: question_model::PublicByline::new(vec![
                    question_model::PublicAuthorName::new("PLE fixture".to_string())
                        .expect("valid test byline"),
                ])
                .expect("valid test byline"),
                capabilities: BackendCapabilities::from_iter([Capability::ServerGrading]),
            },
        )
        .await
        .expect("prior publication");
    let mut revised_question = draft(workspace, "Revised peptide question");
    revised_question.timing_policy = TimingPolicy::PerQuestion {
        seconds: 90,
        grace_seconds: 5,
    };
    store
        .upsert_draft(
            context,
            actor,
            None,
            DraftRecord {
                tenant,
                question: revised_question,
                derived_from: Some(prior_reference),
            },
        )
        .await
        .expect("revision draft save");
    let app = test_router(Arc::clone(&store));

    let validation = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!(
                    "/api/workspaces/{workspace}/publication-validation"
                ))
                .header("cookie", &cookie)
                .body(Body::empty())
                .expect("validation request"),
        )
        .await
        .expect("validation response");
    assert_eq!(validation.status(), StatusCode::OK);
    assert_eq!(
        validation.headers().get("cache-control").unwrap(),
        "no-store"
    );
    assert_eq!(validation.headers().get(ETAG).unwrap(), "\"1\"");
    assert_eq!(
        response_json(validation).await,
        serde_json::json!({
            "violations": [{
                "workspace": workspace,
                "title": "Revised peptide question",
                "capability": "perQuestionTiming"
            }]
        })
    );

    let nonempty_validation = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!(
                    "/api/workspaces/{workspace}/publication-validation"
                ))
                .header("cookie", &cookie)
                .body(Body::from("{}"))
                .expect("nonempty validation request"),
        )
        .await
        .expect("nonempty validation response");
    assert_eq!(nonempty_validation.status(), StatusCode::BAD_REQUEST);

    let diff = app
        .oneshot(
            Request::builder()
                .uri(format!("/api/workspaces/{workspace}/publication-diff"))
                .header("cookie", &cookie)
                .body(Body::empty())
                .expect("diff request"),
        )
        .await
        .expect("diff response");
    assert_eq!(diff.status(), StatusCode::OK);
    assert_eq!(diff.headers().get(ETAG).unwrap(), "\"1\"");
    let diff = response_json(diff).await;
    assert_eq!(diff["draftRevision"], 1);
    assert_eq!(diff["baseline"], "newQuestion");
    assert_eq!(diff["changed"], serde_json::json!([]));
    assert_eq!(diff["current"]["sourceBackend"], "native");
    let serialized = diff.to_string();
    for forbidden in [
        r#""source":"#,
        r#""family":"#,
        r#""provider":"#,
        r#""itemRef":"#,
        r#""grading":"#,
        r#""answerKey":"#,
        r#""artifact":"#,
    ] {
        assert!(
            !serialized.contains(forbidden),
            "semantic diff leaked forbidden field {forbidden}"
        );
    }
}

#[tokio::test]
async fn external_publication_validation_refuses_without_publishing_or_changing_draft() {
    let store = Arc::new(MemoryStore::default());
    let tenant = TenantId::from_uuid(id(66));
    let actor = UserId::from_uuid(id(67));
    let workspace = WorkspaceId::from_uuid(id(68));
    let context = TenantContext::from_authenticated_session(tenant);
    let cookie = issued_cookie(&store, tenant, vec![UserRole::Instructor], actor).await;
    let mut question = draft(workspace, "iMathAS source snapshot fixture");
    question.source = DraftQuestionSource::Imathas {
        provider: "institution-imathas".to_string(),
        item_ref: "1842".to_string(),
    };
    let candidate = DraftRecord {
        tenant,
        question,
        derived_from: None,
    };
    store
        .upsert_draft(context, actor, None, candidate.clone())
        .await
        .expect("external draft save");
    let app = router(
        Arc::clone(&store),
        Arc::new(FixtureRegistry {
            capabilities: BackendCapabilities::from_iter([
                Capability::ServerGrading,
                Capability::Hints,
            ]),
        }),
    );

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!(
                    "/api/workspaces/{workspace}/publication-validation"
                ))
                .header("cookie", &cookie)
                .body(Body::empty())
                .expect("external validation request"),
        )
        .await
        .expect("external validation response");
    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(response.headers().get("cache-control").unwrap(), "no-store");
    assert!(
        store
            .list_catalog(
                context,
                PageRequest::first(PageSize::new(10).expect("valid page size")),
            )
            .await
            .expect("catalog listing")
            .items
            .is_empty(),
        "publication validation must not mint a catalog record"
    );
    assert_eq!(
        store
            .get_draft(context, actor, workspace)
            .await
            .expect("external draft lookup")
            .map(|draft| draft.record),
        Some(candidate),
        "publication validation must leave its stored draft unchanged"
    );
}

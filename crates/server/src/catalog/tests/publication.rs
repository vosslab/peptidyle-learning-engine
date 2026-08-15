use super::*;
use crate::catalog::mint_publication_reference;
use axum::body::Body;
use axum::http::Request;
use learning_data_access::in_memory::MemoryStore;
use question_model::{
    BackendCapabilities, Capability, TenantId, UserId, UserRole, VersionId, WorkspaceId,
};
use std::sync::Arc;
use tower::ServiceExt;

#[test]
fn publication_mint_creates_a_distinct_exact_identity_pair() {
    let first = mint_publication_reference();
    let second = mint_publication_reference();

    assert_ne!(first, second);
}

#[tokio::test]
async fn qid_lifecycle_routes_authorize_then_resolve_exact_visible_questions() {
    let store = Arc::new(MemoryStore::default());
    let tenant = TenantId::from_uuid(id(1));
    let publisher = UserId::from_uuid(id(8_101));
    let workspace = WorkspaceId::from_uuid(id(8_102));
    let revision = store
        .upsert_draft(
            TenantContext::from_authenticated_session(tenant),
            publisher,
            None,
            draft(tenant, workspace, VersionId::from_uuid(id(8_103))),
        )
        .await
        .expect("draft save")
        .revision;
    let app = router(
        Arc::clone(&store),
        Arc::new(FixtureRegistry {
            capabilities: BackendCapabilities::from_iter([Capability::ServerGrading]),
        }),
        Arc::new(ReviewNotRequired),
    );
    let instructor_cookie = issued_cookie(&store, vec![UserRole::Instructor], publisher).await;
    let student_cookie = issued_cookie(
        &store,
        vec![UserRole::Student],
        UserId::from_uuid(id(8_104)),
    )
    .await;

    let unauthorized = app
        .clone()
        .oneshot(
            Request::post("/api/problems/by-id/not-a-question/deprecate")
                .header("cookie", &student_cookie)
                .header("content-type", "application/json")
                .body(Body::from(r#"{"reason":"unused"}"#))
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(unauthorized.status(), axum::http::StatusCode::FORBIDDEN);

    let malformed = app
        .clone()
        .oneshot(
            Request::post("/api/problems/by-id/not-a-question/deprecate")
                .header("cookie", &instructor_cookie)
                .header("content-type", "application/json")
                .body(Body::from(r#"{"reason":"unused"}"#))
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(malformed.status(), axum::http::StatusCode::BAD_REQUEST);

    let published = app
        .clone()
        .oneshot(
            Request::post(format!("/api/problems/{workspace}/publish"))
                .header("cookie", &instructor_cookie)
                .header("if-match", strong_if_match(revision))
                .header("content-type", "application/json")
                .body(Body::from(r#"{"scope":"institution"}"#))
                .expect("request"),
        )
        .await
        .expect("response");
    let published = response_json(published).await;
    let question_id = published["questionId"].as_str().expect("Question ID");

    let unavailable = app
        .clone()
        .oneshot(
            Request::post("/api/problems/by-id/000-0000/archive")
                .header("cookie", &instructor_cookie)
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(unavailable.status(), axum::http::StatusCode::NOT_FOUND);

    let deprecated = app
        .oneshot(
            Request::post(format!("/api/problems/by-id/{question_id}/deprecate"))
                .header("cookie", instructor_cookie)
                .header("content-type", "application/json")
                .body(Body::from(r#"{"reason":"Use the newer question."}"#))
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(deprecated.status(), axum::http::StatusCode::OK);
}

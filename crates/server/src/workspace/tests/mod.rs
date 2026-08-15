mod publication;

use super::router;
use crate::catalog::{BackendRegistry, BackendRegistryError};
use axum::Router;
use axum::body::{Body, to_bytes};
use axum::http::header::{ETAG, IF_MATCH};
use axum::http::{Request, StatusCode};
use axum::response::Response;
use learning_data_access::in_memory::MemoryStore;
use learning_data_access::{
    DraftRecord, PageRequest, PageSize, SessionLifetime, SessionSubject, Store, TenantContext,
};
use question_model::answer::TextMatchMode;
use question_model::envelope::ContentBlock;
use question_model::generation::RandomizationDefinition;
use question_model::response::ResponseDefinition;
use question_model::run_policy::{AttemptPolicy, FeedbackDisclosure, TimingPolicy};
use question_model::taxonomy::License;
use question_model::{
    BackendCapabilities, Capability, DraftQuestionDefinition, DraftQuestionSource,
    GradingDefinition, ProblemId, ProblemVersionRef, QuestionMetadata, TenantId, UserId, UserRole,
    VersionId, WorkspaceId,
};
use std::sync::Arc;
use tower::ServiceExt;
use uuid::Uuid;

use super::support::MAX_WORKSPACE_BODY_BYTES;

#[derive(Debug, Default)]
struct FixtureRegistry {
    capabilities: BackendCapabilities,
}

impl BackendRegistry for FixtureRegistry {
    fn capabilities(
        &self,
        _source: &DraftQuestionSource,
    ) -> Result<BackendCapabilities, BackendRegistryError> {
        Ok(self.capabilities.clone())
    }
}

fn test_router(store: Arc<MemoryStore>) -> Router {
    router(
        store,
        Arc::new(FixtureRegistry {
            capabilities: BackendCapabilities::from_iter([Capability::ServerGrading]),
        }),
    )
}

fn id(value: u128) -> Uuid {
    Uuid::from_u128(value)
}

fn draft(workspace: WorkspaceId, title: &str) -> DraftQuestionDefinition {
    DraftQuestionDefinition {
        workspace,
        source: DraftQuestionSource::Native {
            family: "workspace-fixture".to_string(),
        },
        prompt: vec![ContentBlock::Text {
            markdown: "Name the bond joining amino acids.".to_string(),
        }],
        response: ResponseDefinition::ShortText {
            match_mode: TextMatchMode::Normalized,
            max_length: 64,
        },
        attempt_policy: AttemptPolicy {
            max_attempts: None,
            feedback: FeedbackDisclosure::ImmediateCorrectness,
        },
        timing_policy: TimingPolicy::Untimed,
        randomization: RandomizationDefinition::Static,
        grading: GradingDefinition::AllOrNothing { points: 1.0 },
        metadata: QuestionMetadata {
            title: title.to_string(),
            tags: Vec::new(),
            taxonomy: Vec::new(),
            license: License::CcBy,
            language: "en-US".to_string(),
        },
    }
}

async fn issued_cookie(
    store: &MemoryStore,
    tenant: TenantId,
    roles: Vec<UserRole>,
    user: UserId,
) -> String {
    let subject =
        SessionSubject::new(tenant, user, "Workspace Fixture", roles).expect("fixture identity");
    let issued = crate::auth::issue_session(
        store,
        subject,
        crate::auth::SessionConfig::new(
            SessionLifetime::from_seconds(3_600).expect("positive lifetime"),
            crate::auth::CookieTransport::LocalHttp,
        ),
    )
    .await
    .expect("fixture session");
    issued
        .set_cookie
        .split(';')
        .next()
        .expect("cookie pair")
        .to_string()
}

async fn response_json(response: Response) -> serde_json::Value {
    let body = to_bytes(response.into_body(), 128 * 1_024)
        .await
        .expect("response body");
    serde_json::from_slice(&body).expect("JSON response")
}

#[tokio::test]
async fn author_can_save_list_refresh_and_delete_its_workspace() {
    let store = Arc::new(MemoryStore::default());
    let tenant = TenantId::from_uuid(id(1));
    let workspace = WorkspaceId::from_uuid(id(2));
    let cookie = issued_cookie(
        &store,
        tenant,
        vec![UserRole::Instructor],
        UserId::from_uuid(id(3)),
    )
    .await;
    let candidate = draft(workspace, "Peptide bond draft");
    let app = test_router(Arc::clone(&store));
    let prior_revision = ProblemVersionRef {
        problem: ProblemId::from_uuid(id(20)),
        version: VersionId::from_uuid(id(21)),
    };
    store
        .upsert_draft(
            TenantContext::from_authenticated_session(tenant),
            UserId::from_uuid(id(3)),
            None,
            DraftRecord {
                tenant,
                question: draft(workspace, "Earlier title"),
                derived_from: Some(prior_revision),
            },
        )
        .await
        .expect("seed prior draft lineage");
    let initial_revision = store
        .get_draft(
            TenantContext::from_authenticated_session(tenant),
            UserId::from_uuid(id(3)),
            workspace,
        )
        .await
        .expect("seed draft lookup")
        .expect("seed draft exists")
        .revision;

    let saved = app
        .clone()
        .clone()
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri(format!("/api/workspaces/{workspace}"))
                .header("cookie", &cookie)
                .header("content-type", "application/json")
                .header(IF_MATCH, format!("\"{}\"", initial_revision.value()))
                .body(Body::from(
                    serde_json::to_vec(&candidate).expect("draft JSON"),
                ))
                .expect("save request"),
        )
        .await
        .expect("save response");
    assert_eq!(saved.status(), StatusCode::OK);
    assert_eq!(saved.headers().get("cache-control").unwrap(), "no-store");
    let saved_revision = saved
        .headers()
        .get(ETAG)
        .expect("save response revision")
        .to_str()
        .expect("revision is ASCII")
        .to_string();
    assert_eq!(response_json(saved).await, serde_json::json!(candidate));
    assert_eq!(
        store
            .get_draft(
                TenantContext::from_authenticated_session(tenant),
                UserId::from_uuid(id(3)),
                workspace,
            )
            .await
            .expect("saved draft lookup")
            .expect("saved draft exists")
            .record
            .derived_from,
        Some(prior_revision),
        "browser refresh retains descriptive provenance"
    );

    let listed = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/workspaces?pageSize=1")
                .header("cookie", &cookie)
                .body(Body::empty())
                .expect("list request"),
        )
        .await
        .expect("list response");
    assert_eq!(listed.status(), StatusCode::OK);
    let listed = response_json(listed).await;
    assert_eq!(
        listed["items"][0]["workspace"],
        serde_json::json!(workspace)
    );
    assert_eq!(listed["items"][0]["title"], "Peptide bond draft");
    assert_eq!(listed["items"][0]["sourceBackend"], "native");
    assert!(listed["items"][0].get("problem").is_none());
    assert!(listed["items"][0].get("version").is_none());

    let refreshed = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!("/api/workspaces/{workspace}"))
                .header("cookie", &cookie)
                .body(Body::empty())
                .expect("get request"),
        )
        .await
        .expect("get response");
    assert_eq!(refreshed.status(), StatusCode::OK);
    assert_eq!(
        refreshed
            .headers()
            .get(ETAG)
            .unwrap()
            .to_str()
            .expect("revision is ASCII"),
        saved_revision
    );
    assert_eq!(response_json(refreshed).await, serde_json::json!(candidate));

    let deleted = app
        .clone()
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri(format!("/api/workspaces/{workspace}"))
                .header("cookie", &cookie)
                .header(IF_MATCH, &saved_revision)
                .body(Body::empty())
                .expect("delete request"),
        )
        .await
        .expect("delete response");
    assert_eq!(deleted.status(), StatusCode::NO_CONTENT);
    assert_eq!(
        store
            .get_draft(
                TenantContext::from_authenticated_session(tenant),
                UserId::from_uuid(id(3)),
                workspace,
            )
            .await
            .expect("draft lookup"),
        None
    );
}

#[tokio::test]
async fn workspace_delete_requires_one_strong_etag() {
    let store = Arc::new(MemoryStore::default());
    let tenant = TenantId::from_uuid(id(22));
    let workspace = WorkspaceId::from_uuid(id(23));
    let actor = UserId::from_uuid(id(24));
    let cookie = issued_cookie(&store, tenant, vec![UserRole::Instructor], actor).await;
    store
        .upsert_draft(
            TenantContext::from_authenticated_session(tenant),
            actor,
            None,
            DraftRecord {
                tenant,
                question: draft(workspace, "Deletion precondition fixture"),
                derived_from: None,
            },
        )
        .await
        .expect("draft save");
    let app = test_router(Arc::clone(&store));

    let missing = app
        .clone()
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri(format!("/api/workspaces/{workspace}"))
                .header("cookie", &cookie)
                .body(Body::empty())
                .expect("missing precondition request"),
        )
        .await
        .expect("missing precondition response");
    assert_eq!(missing.status(), StatusCode::PRECONDITION_REQUIRED);
    assert_eq!(missing.headers().get("cache-control").unwrap(), "no-store");

    let malformed = app
        .clone()
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri(format!("/api/workspaces/{workspace}"))
                .header("cookie", &cookie)
                .header(IF_MATCH, "W/\"1\"")
                .body(Body::empty())
                .expect("malformed precondition request"),
        )
        .await
        .expect("malformed precondition response");
    assert_eq!(malformed.status(), StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(
        malformed.headers().get("cache-control").unwrap(),
        "no-store"
    );

    for malformed_revision in ["\"0\"", "\"9223372036854775808\""] {
        let malformed = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("DELETE")
                    .uri(format!("/api/workspaces/{workspace}"))
                    .header("cookie", &cookie)
                    .header(IF_MATCH, malformed_revision)
                    .body(Body::empty())
                    .expect("out-of-range precondition request"),
            )
            .await
            .expect("out-of-range precondition response");
        assert_eq!(malformed.status(), StatusCode::UNPROCESSABLE_ENTITY);
        assert_eq!(
            malformed.headers().get("cache-control").unwrap(),
            "no-store"
        );
        assert!(
            store
                .get_draft(
                    TenantContext::from_authenticated_session(tenant),
                    actor,
                    workspace,
                )
                .await
                .expect("draft lookup")
                .is_some(),
            "malformed deletion revision must not remove the draft"
        );
    }
}

#[tokio::test]
async fn authoring_route_rejects_path_mismatch_unknown_fields_and_bad_cursors() {
    let store = Arc::new(MemoryStore::default());
    let tenant = TenantId::from_uuid(id(1));
    let workspace = WorkspaceId::from_uuid(id(2));
    let other_workspace = WorkspaceId::from_uuid(id(3));
    let cookie = issued_cookie(
        &store,
        tenant,
        vec![UserRole::Instructor],
        UserId::from_uuid(id(4)),
    )
    .await;
    let app = test_router(store);
    let candidate = draft(other_workspace, "Mismatch");

    let mismatch = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri(format!("/api/workspaces/{workspace}"))
                .header("cookie", &cookie)
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::to_vec(&candidate).expect("draft JSON"),
                ))
                .expect("mismatch request"),
        )
        .await
        .expect("mismatch response");
    assert_eq!(mismatch.status(), StatusCode::UNPROCESSABLE_ENTITY);

    let mut unknown_candidate =
        serde_json::to_value(draft(workspace, "Unknown")).expect("draft JSON value");
    unknown_candidate["metadata"]
        .as_object_mut()
        .expect("draft metadata JSON object")
        .insert("answerKey".to_string(), serde_json::json!("private-answer"));
    let unknown = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri(format!("/api/workspaces/{workspace}"))
                .header("cookie", &cookie)
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::to_vec(&unknown_candidate).expect("unknown draft JSON"),
                ))
                .expect("unknown request"),
        )
        .await
        .expect("unknown response");
    let unknown_status = unknown.status();
    let unknown_cache = unknown.headers().get("cache-control").cloned();
    assert_eq!(unknown_status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(
        unknown_cache.as_ref().and_then(|value| value.to_str().ok()),
        Some("no-store")
    );

    let bad_cursor = app
        .oneshot(
            Request::builder()
                .uri("/api/workspaces?cursor=not-a-valid-cursor")
                .header("cookie", &cookie)
                .body(Body::empty())
                .expect("cursor request"),
        )
        .await
        .expect("cursor response");
    assert_eq!(bad_cursor.status(), StatusCode::UNPROCESSABLE_ENTITY);
}

#[tokio::test]
async fn students_and_foreign_tenants_cannot_enumerate_private_workspaces() {
    let store = Arc::new(MemoryStore::default());
    let owner_tenant = TenantId::from_uuid(id(1));
    let foreign_tenant = TenantId::from_uuid(id(9));
    let workspace = WorkspaceId::from_uuid(id(2));
    let owner_cookie = issued_cookie(
        &store,
        owner_tenant,
        vec![UserRole::Instructor],
        UserId::from_uuid(id(3)),
    )
    .await;
    let student_cookie = issued_cookie(
        &store,
        owner_tenant,
        vec![UserRole::Student],
        UserId::from_uuid(id(4)),
    )
    .await;
    let foreign_cookie = issued_cookie(
        &store,
        foreign_tenant,
        vec![UserRole::Instructor],
        UserId::from_uuid(id(5)),
    )
    .await;
    let second_instructor_cookie = issued_cookie(
        &store,
        owner_tenant,
        vec![UserRole::Instructor],
        UserId::from_uuid(id(6)),
    )
    .await;
    let app = test_router(Arc::clone(&store));
    let candidate = draft(workspace, "Private draft");
    let saved = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri(format!("/api/workspaces/{workspace}"))
                .header("cookie", &owner_cookie)
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::to_vec(&candidate).expect("draft JSON"),
                ))
                .expect("save request"),
        )
        .await
        .expect("save response");
    assert_eq!(saved.status(), StatusCode::OK);
    let current_revision = saved
        .headers()
        .get(ETAG)
        .expect("save response revision")
        .to_str()
        .expect("revision is ASCII")
        .to_string();

    let student_put = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri(format!("/api/workspaces/{workspace}"))
                .header("cookie", &student_cookie)
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::to_vec(&draft(workspace, "Student overwrite")).expect("draft JSON"),
                ))
                .expect("student save request"),
        )
        .await
        .expect("student save response");
    assert_eq!(student_put.status(), StatusCode::FORBIDDEN);
    let student_delete = app
        .clone()
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri(format!("/api/workspaces/{workspace}"))
                .header("cookie", &student_cookie)
                .body(Body::empty())
                .expect("student delete request"),
        )
        .await
        .expect("student delete response");
    assert_eq!(student_delete.status(), StatusCode::FORBIDDEN);

    let second_instructor_put = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri(format!("/api/workspaces/{workspace}"))
                .header("cookie", &second_instructor_cookie)
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::to_vec(&draft(workspace, "Unauthorized overwrite"))
                        .expect("draft JSON"),
                ))
                .expect("second instructor save request"),
        )
        .await
        .expect("second instructor save response");
    assert_eq!(second_instructor_put.status(), StatusCode::NOT_FOUND);
    let second_instructor_delete = app
        .clone()
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri(format!("/api/workspaces/{workspace}"))
                .header("cookie", &second_instructor_cookie)
                .header(IF_MATCH, &current_revision)
                .body(Body::empty())
                .expect("second instructor delete request"),
        )
        .await
        .expect("second instructor delete response");
    assert_eq!(second_instructor_delete.status(), StatusCode::NOT_FOUND);
    assert_eq!(
        store
            .get_draft(
                TenantContext::from_authenticated_session(owner_tenant),
                UserId::from_uuid(id(3)),
                workspace,
            )
            .await
            .expect("owner draft lookup")
            .expect("owner draft remains")
            .record
            .question
            .metadata
            .title,
        "Private draft",
        "nonowners cannot mutate or delete a private workspace"
    );

    let student_list = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/workspaces")
                .header("cookie", &student_cookie)
                .body(Body::empty())
                .expect("student list request"),
        )
        .await
        .expect("student list response");
    assert_eq!(student_list.status(), StatusCode::FORBIDDEN);

    let student_get = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!("/api/workspaces/{workspace}"))
                .header("cookie", &student_cookie)
                .body(Body::empty())
                .expect("student get request"),
        )
        .await
        .expect("student get response");
    assert_eq!(student_get.status(), StatusCode::FORBIDDEN);

    let foreign_get = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!("/api/workspaces/{workspace}"))
                .header("cookie", &foreign_cookie)
                .body(Body::empty())
                .expect("foreign get request"),
        )
        .await
        .expect("foreign get response");
    assert_eq!(foreign_get.status(), StatusCode::NOT_FOUND);

    let foreign_delete = app
        .clone()
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri(format!("/api/workspaces/{workspace}"))
                .header("cookie", &foreign_cookie)
                .header(IF_MATCH, &current_revision)
                .body(Body::empty())
                .expect("foreign delete request"),
        )
        .await
        .expect("foreign delete response");
    assert_eq!(foreign_delete.status(), StatusCode::NOT_FOUND);

    let foreign_list = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/workspaces")
                .header("cookie", &foreign_cookie)
                .body(Body::empty())
                .expect("foreign list request"),
        )
        .await
        .expect("foreign list response");
    assert_eq!(foreign_list.status(), StatusCode::OK);
    assert_eq!(
        response_json(foreign_list).await["items"],
        serde_json::json!([])
    );
}

#[tokio::test]
async fn workspace_save_requires_a_fresh_revision_and_preserves_newer_content() {
    let store = Arc::new(MemoryStore::default());
    let tenant = TenantId::from_uuid(id(40));
    let workspace = WorkspaceId::from_uuid(id(41));
    let actor = UserId::from_uuid(id(42));
    let cookie = issued_cookie(&store, tenant, vec![UserRole::Instructor], actor).await;
    let app = test_router(Arc::clone(&store));
    let original = draft(workspace, "Original");

    let created = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri(format!("/api/workspaces/{workspace}"))
                .header("cookie", &cookie)
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::to_vec(&original).expect("draft JSON"),
                ))
                .expect("create request"),
        )
        .await
        .expect("create response");
    assert_eq!(created.status(), StatusCode::OK);
    let stale_revision = created
        .headers()
        .get(ETAG)
        .expect("create response revision")
        .to_str()
        .expect("revision is ASCII")
        .to_string();

    let newer = draft(workspace, "Newer author edit");
    let fresh = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri(format!("/api/workspaces/{workspace}"))
                .header("cookie", &cookie)
                .header("content-type", "application/json")
                .header(IF_MATCH, &stale_revision)
                .body(Body::from(serde_json::to_vec(&newer).expect("draft JSON")))
                .expect("fresh save request"),
        )
        .await
        .expect("fresh save response");
    assert_eq!(fresh.status(), StatusCode::OK);

    let stale = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri(format!("/api/workspaces/{workspace}"))
                .header("cookie", &cookie)
                .header("content-type", "application/json")
                .header(IF_MATCH, stale_revision)
                .body(Body::from(
                    serde_json::to_vec(&draft(workspace, "Stale overwrite")).expect("draft JSON"),
                ))
                .expect("stale save request"),
        )
        .await
        .expect("stale save response");
    assert_eq!(stale.status(), StatusCode::CONFLICT);
    assert_eq!(stale.headers().get("cache-control").unwrap(), "no-store");
    assert_eq!(
        store
            .get_draft(
                TenantContext::from_authenticated_session(tenant),
                actor,
                workspace
            )
            .await
            .expect("owner lookup")
            .expect("workspace remains")
            .record
            .question
            .metadata
            .title,
        "Newer author edit"
    );
}

#[tokio::test]
async fn invited_collaborator_can_read_and_save_with_the_issued_revision() {
    let store = Arc::new(MemoryStore::default());
    let tenant = TenantId::from_uuid(id(45));
    let workspace = WorkspaceId::from_uuid(id(46));
    let owner = UserId::from_uuid(id(47));
    let collaborator = UserId::from_uuid(id(48));
    let owner_cookie = issued_cookie(&store, tenant, vec![UserRole::Instructor], owner).await;
    let collaborator_cookie =
        issued_cookie(&store, tenant, vec![UserRole::Instructor], collaborator).await;
    store
        .upsert_draft(
            TenantContext::from_authenticated_session(tenant),
            owner,
            None,
            DraftRecord {
                tenant,
                question: draft(workspace, "Owner draft"),
                derived_from: None,
            },
        )
        .await
        .expect("owner draft creation");
    store
        .grant_draft_collaborator(
            TenantContext::from_authenticated_session(tenant),
            owner,
            workspace,
            collaborator,
        )
        .await
        .expect("owner invitation");
    let app = test_router(Arc::clone(&store));

    let loaded = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!("/api/workspaces/{workspace}"))
                .header("cookie", &collaborator_cookie)
                .body(Body::empty())
                .expect("collaborator get request"),
        )
        .await
        .expect("collaborator get response");
    assert_eq!(loaded.status(), StatusCode::OK);
    let revision = loaded
        .headers()
        .get(ETAG)
        .expect("collaborator read revision")
        .to_str()
        .expect("revision is ASCII")
        .to_string();

    let saved = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri(format!("/api/workspaces/{workspace}"))
                .header("cookie", &collaborator_cookie)
                .header("content-type", "application/json")
                .header(IF_MATCH, &revision)
                .body(Body::from(
                    serde_json::to_vec(&draft(workspace, "Collaborator revision"))
                        .expect("draft JSON"),
                ))
                .expect("collaborator save request"),
        )
        .await
        .expect("collaborator save response");
    assert_eq!(saved.status(), StatusCode::OK);
    assert_eq!(saved.headers().get("cache-control").unwrap(), "no-store");
    let collaborator_revision = saved
        .headers()
        .get(ETAG)
        .expect("collaborator save revision")
        .to_str()
        .expect("revision is ASCII")
        .to_string();

    let stale_owner_delete = app
        .clone()
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri(format!("/api/workspaces/{workspace}"))
                .header("cookie", &owner_cookie)
                .header(IF_MATCH, &revision)
                .body(Body::empty())
                .expect("stale owner delete request"),
        )
        .await
        .expect("stale owner delete response");
    assert_eq!(stale_owner_delete.status(), StatusCode::CONFLICT);

    let collaborator_delete = app
        .clone()
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri(format!("/api/workspaces/{workspace}"))
                .header("cookie", &collaborator_cookie)
                .header(IF_MATCH, &collaborator_revision)
                .body(Body::empty())
                .expect("collaborator delete request"),
        )
        .await
        .expect("collaborator delete response");
    assert_eq!(collaborator_delete.status(), StatusCode::NOT_FOUND);
    assert_eq!(
        store
            .get_draft(
                TenantContext::from_authenticated_session(tenant),
                owner,
                workspace
            )
            .await
            .expect("owner lookup")
            .expect("owner workspace remains")
            .record
            .question
            .metadata
            .title,
        "Collaborator revision"
    );

    // Keep the owner session exercised as a reminder that the actor is a
    // persisted ACL input, not whichever authoring role happened to issue
    // the last save.
    let owner_list = app
        .oneshot(
            Request::builder()
                .uri("/api/workspaces")
                .header("cookie", &owner_cookie)
                .body(Body::empty())
                .expect("owner list request"),
        )
        .await
        .expect("owner list response");
    assert_eq!(owner_list.status(), StatusCode::OK);
}

#[tokio::test]
async fn workspace_body_limit_rejects_without_storing_or_caching() {
    let store = Arc::new(MemoryStore::default());
    let tenant = TenantId::from_uuid(id(50));
    let workspace = WorkspaceId::from_uuid(id(51));
    let actor = UserId::from_uuid(id(52));
    let cookie = issued_cookie(&store, tenant, vec![UserRole::Instructor], actor).await;
    let app = test_router(Arc::clone(&store));
    let oversized = format!("\"{}\"", "x".repeat(MAX_WORKSPACE_BODY_BYTES));

    let response = app
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri(format!("/api/workspaces/{workspace}"))
                .header("cookie", &cookie)
                .header("content-type", "application/json")
                .body(Body::from(oversized))
                .expect("oversized request"),
        )
        .await
        .expect("oversized response");
    assert_eq!(response.status(), StatusCode::PAYLOAD_TOO_LARGE);
    assert_eq!(response.headers().get("cache-control").unwrap(), "no-store");
    assert_eq!(
        store
            .get_draft(
                TenantContext::from_authenticated_session(tenant),
                actor,
                workspace
            )
            .await
            .expect("workspace lookup"),
        None
    );
}

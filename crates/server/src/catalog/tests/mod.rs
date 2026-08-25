mod publication;
mod search;

use super::*;
use std::cell::Cell;
use std::sync::Arc;

use async_trait::async_trait;
use axum::body::{Body, to_bytes};
use axum::http::Request;
use axum::http::StatusCode;
use axum::http::header::IF_MATCH;
use axum::response::Response;
use learning_data_access::in_memory::MemoryStore;
use learning_data_access::{
    AccountIdentityStore, ApproveInstructorAccount, AuthenticationEmail,
    AuthenticationRateLimitKey, BeginEmailAuthentication, BrowserBindingHash,
    CompleteEmailAuthentication, DraftRecord, EmailAuthenticationPurpose, EmailChallengeId,
    EmailChallengeLifetime, EmailChallengeSecretHash, SessionLifetime, SessionSubject, Store,
    TeachingAuthorityStore, TenantContext, WorkspaceDraftRevision,
};
use question_model::answer::NumericTolerance;
use question_model::envelope::ContentBlock;
use question_model::generation::RandomizationDefinition;
use question_model::response::ResponseDefinition;
use question_model::run_policy::{AttemptPolicy, TimingPolicy};
use question_model::taxonomy::{License, TaxonomyTerm};
use question_model::{
    ActivityTimestamp, BackendCapabilities, Capability, DraftQuestionDefinition,
    DraftQuestionSource, GradingDefinition, QuestionMetadata, TenantId, UserId, UserRole,
    VersionId, WorkspaceId, WorkspaceImportId,
};
use std::sync::atomic::{AtomicUsize, Ordering};
use tower::ServiceExt;
use uuid::Uuid;

#[derive(Clone)]
struct FixtureRegistry {
    capabilities: BackendCapabilities,
}

struct ReviewRequired;

/// Delivers a later adapter declaration to prove publication does not
/// trust a capability result obtained before its final draft re-read.
struct ChangingRegistry {
    initial: BackendCapabilities,
    current: BackendCapabilities,
    calls: AtomicUsize,
}

/// Arranges a collaborator saving while an institutional public-review
/// workflow is in flight. The route must re-check the browser's original
/// revision after this gate returns, before minting an identity.
struct CollaboratorEditingReviewGate {
    store: Arc<MemoryStore>,
    collaborator: UserId,
}

impl BackendRegistry for FixtureRegistry {
    fn capabilities(
        &self,
        _source: &DraftQuestionSource,
    ) -> Result<BackendCapabilities, BackendRegistryError> {
        Ok(self.capabilities.clone())
    }
}

impl BackendRegistry for ChangingRegistry {
    fn capabilities(
        &self,
        _source: &DraftQuestionSource,
    ) -> Result<BackendCapabilities, BackendRegistryError> {
        if self.calls.fetch_add(1, Ordering::SeqCst) == 0 {
            Ok(self.initial.clone())
        } else {
            Ok(self.current.clone())
        }
    }
}

#[async_trait]
impl PublicReviewGate for ReviewRequired {
    async fn allows_publication(
        &self,
        _tenant: TenantContext,
        _publisher: UserId,
        _draft: &DraftRecord,
    ) -> Result<bool, ReviewGateError> {
        Ok(false)
    }
}

#[async_trait]
impl PublicReviewGate for CollaboratorEditingReviewGate {
    async fn allows_publication(
        &self,
        tenant: TenantContext,
        _publisher: UserId,
        draft: &DraftRecord,
    ) -> Result<bool, ReviewGateError> {
        let current = self
            .store
            .get_draft(tenant, self.collaborator, draft.question.workspace)
            .await
            .map_err(|error| ReviewGateError(error.to_string()))?
            .ok_or_else(|| ReviewGateError("review draft disappeared".to_string()))?;
        let mut replacement = current.record;
        replacement
            .question
            .metadata
            .title
            .push_str(" reviewed edit");
        self.store
            .upsert_draft(
                tenant,
                self.collaborator,
                Some(current.revision),
                replacement,
            )
            .await
            .map_err(|error| ReviewGateError(error.to_string()))?;
        Ok(true)
    }
}

fn id(value: u128) -> Uuid {
    Uuid::from_u128(value)
}

fn strong_if_match(revision: WorkspaceDraftRevision) -> String {
    format!("\"{}\"", revision.value())
}

fn draft(tenant: TenantId, workspace: WorkspaceId, version: VersionId) -> DraftRecord {
    DraftRecord {
        tenant,
        question: DraftQuestionDefinition {
            workspace,
            source: DraftQuestionSource::Native {
                family: "catalog-fixture".to_string(),
            },
            prompt: vec![ContentBlock::Text {
                markdown: "What is the molecular mass?".to_string(),
            }],
            response: ResponseDefinition::Numeric {
                tolerance: NumericTolerance::Relative { fraction: 0.01 },
                unit: Some("g/mol".to_string()),
            },
            attempt_policy: AttemptPolicy {
                max_attempts: Some(2),
            },
            timing_policy: TimingPolicy::Untimed,
            randomization: RandomizationDefinition::Static,
            grading: GradingDefinition::AllOrNothing { points: 1.0 },
            metadata: QuestionMetadata {
                title: format!("Catalog fixture {version}"),
                tags: Vec::new(),
                taxonomy: vec![TaxonomyTerm {
                    scheme: "discipline".to_string(),
                    code: format!("BIO-{version}"),
                    label: "Biochemistry".to_string(),
                }],
                license: License::CcBySa,
                language: "en-US".to_string(),
            },
        },
        derived_from: None,
    }
}

async fn issued_cookie(store: &MemoryStore, roles: Vec<UserRole>, user: UserId) -> String {
    let subject = SessionSubject::new(TenantId::from_uuid(id(1)), user, "Catalog Fixture", roles)
        .expect("fixture identity");
    let issued = crate::auth::issue_session(
        store,
        subject,
        crate::auth::SessionConfig::new(
            SessionLifetime::from_seconds(3_600).expect("positive lifetime"),
            crate::auth::CookieTransport::FirstPartyHttps,
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

async fn create_instructor_account(store: &MemoryStore, user: UserId) {
    let token = EmailChallengeSecretHash::compute(b"catalog-approved-instructor-token");
    let binding = BrowserBindingHash::compute(b"catalog-approved-instructor-binding");
    store
        .begin_email_authentication(BeginEmailAuthentication {
            id: EmailChallengeId::from_uuid(id(101)),
            token_hash: token,
            browser_binding: binding,
            email_rate_limit_key: AuthenticationRateLimitKey::compute(
                b"catalog-approved-instructor-rate",
            ),
            email: AuthenticationEmail::parse("catalog-instructor@example.edu")
                .expect("fixture email"),
            purpose: EmailAuthenticationPurpose::SignInOrRegister,
            lifetime: EmailChallengeLifetime::from_seconds(600).expect("fixture lifetime"),
        })
        .await
        .expect("fixture account challenge");
    store
        .complete_email_authentication(CompleteEmailAuthentication {
            token_hash: token,
            browser_binding: binding,
            proposed_user: user,
            proposed_display_name: "Catalog Instructor".to_owned(),
        })
        .await
        .expect("fixture account");
}

async fn issued_approved_instructor_cookie(store: &MemoryStore, user: UserId) -> String {
    let tenant = TenantId::from_uuid(id(1));
    create_instructor_account(store, user).await;
    let instructor_cookie = issued_cookie(store, vec![UserRole::Instructor], user).await;
    let sysadmin = UserId::from_uuid(id(u128::MAX));
    let sysadmin_subject = SessionSubject::new(
        tenant,
        sysadmin,
        "Catalog Fixture Sysadmin",
        vec![UserRole::Sysadmin],
    )
    .expect("fixture Sysadmin identity");
    let sysadmin_session = crate::auth::issue_session(
        store,
        sysadmin_subject,
        crate::auth::SessionConfig::new(
            SessionLifetime::from_seconds(3_600).expect("positive lifetime"),
            crate::auth::CookieTransport::FirstPartyHttps,
        ),
    )
    .await
    .expect("fixture Sysadmin session");
    store
        .approve_instructor_account(
            TenantContext::from_authenticated_session(tenant),
            ApproveInstructorAccount {
                session: sysadmin_session.record.token_hash,
                target: user,
                expected_revision: None,
            },
        )
        .await
        .expect("fixture Instructor approval");
    instructor_cookie
}

async fn response_json(response: Response) -> serde_json::Value {
    let body = to_bytes(response.into_body(), 128 * 1_024)
        .await
        .expect("response body");
    serde_json::from_slice(&body).expect("JSON response")
}

#[tokio::test]
async fn publication_uses_server_capabilities_roles_and_fresh_problem_identity() {
    let store = Arc::new(MemoryStore::default());
    store
        .set_authoritative_time(ActivityTimestamp::from_unix_millis(1_000))
        .expect("fixture clock");
    let tenant = TenantId::from_uuid(id(1));
    let workspace = WorkspaceId::from_uuid(id(2));
    let version = VersionId::from_uuid(id(3));
    let publisher = UserId::from_uuid(id(4));
    let mut candidate = draft(tenant, workspace, version);
    candidate.question.grading = GradingDefinition::PartialCredit { points: 1.0 };
    let draft_revision = store
        .upsert_draft(
            TenantContext::from_authenticated_session(tenant),
            publisher,
            None,
            candidate.clone(),
        )
        .await
        .expect("draft save")
        .revision;
    let cookie = issued_cookie(&store, vec![UserRole::Instructor], publisher).await;

    let failing_app = router(
        Arc::clone(&store),
        Arc::new(FixtureRegistry {
            capabilities: BackendCapabilities::none(),
        }),
        Arc::new(ReviewNotRequired),
    );
    let rejected = failing_app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/problems/{workspace}/publish"))
                .header("cookie", &cookie)
                .header(IF_MATCH, strong_if_match(draft_revision))
                .header("content-type", "application/json")
                .body(Body::from(
                    r#"{"scope":"public","byline":{"names":["PLE fixture"]}}"#,
                ))
                .expect("publish request"),
        )
        .await
        .expect("publish response");
    assert_eq!(rejected.status(), StatusCode::UNPROCESSABLE_ENTITY);
    let rejected = response_json(rejected).await;
    assert_eq!(
        rejected["violations"],
        serde_json::json!([
            {
                "workspace": workspace,
                "title": format!("Catalog fixture {version}"),
                "capability": "serverGrading"
            },
            {
                "workspace": workspace,
                "title": format!("Catalog fixture {version}"),
                "capability": "partialCredit"
            }
        ])
    );
    let still_draft = store
        .get_draft(
            TenantContext::from_authenticated_session(tenant),
            publisher,
            workspace,
        )
        .await
        .expect("draft lookup")
        .expect("validation failure retains draft");
    assert_eq!(still_draft.record.question.workspace, workspace);

    let passing_app = router(
        Arc::clone(&store),
        Arc::new(FixtureRegistry {
            capabilities: BackendCapabilities::from_iter([
                Capability::ServerGrading,
                Capability::PartialCredit,
            ]),
        }),
        Arc::new(ReviewNotRequired),
    );
    let student_cookie =
        issued_cookie(&store, vec![UserRole::Student], UserId::from_uuid(id(5))).await;
    let role_rejected = passing_app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/problems/{workspace}/publish"))
                .header("cookie", student_cookie)
                .header(IF_MATCH, strong_if_match(draft_revision))
                .header("content-type", "application/json")
                .body(Body::from(
                    r#"{"scope":"public","byline":{"names":["PLE fixture"]}}"#,
                ))
                .expect("publish request"),
        )
        .await
        .expect("publish response");
    assert_eq!(role_rejected.status(), StatusCode::FORBIDDEN);

    let review_app = router(
        Arc::clone(&store),
        Arc::new(FixtureRegistry {
            capabilities: BackendCapabilities::from_iter([
                Capability::ServerGrading,
                Capability::PartialCredit,
            ]),
        }),
        Arc::new(ReviewRequired),
    );
    let review_rejected = review_app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/problems/{workspace}/publish"))
                .header("cookie", &cookie)
                .header(IF_MATCH, strong_if_match(draft_revision))
                .header("content-type", "application/json")
                .body(Body::from(
                    r#"{"scope":"public","byline":{"names":["PLE fixture"]}}"#,
                ))
                .expect("publish request"),
        )
        .await
        .expect("publish response");
    assert_eq!(review_rejected.status(), StatusCode::FORBIDDEN);

    let published = passing_app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/problems/{workspace}/publish"))
                .header("cookie", cookie)
                .header(IF_MATCH, strong_if_match(draft_revision))
                .header("content-type", "application/json")
                .body(Body::from(
                    r#"{"scope":"public","byline":{"names":["PLE fixture"]}}"#,
                ))
                .expect("publish request"),
        )
        .await
        .expect("publish response");
    assert_eq!(published.status(), StatusCode::CREATED);
    let published = response_json(published).await;
    assert!(published["questionId"].is_string());
    assert!(published.get("problem").is_none());
    assert!(published.get("version").is_none());
}

#[tokio::test]
async fn publication_requires_a_current_strong_workspace_revision_before_minting() {
    let store = Arc::new(MemoryStore::default());
    let tenant = TenantId::from_uuid(id(1));
    let context = TenantContext::from_authenticated_session(tenant);
    let workspace = WorkspaceId::from_uuid(id(702));
    let publisher = UserId::from_uuid(id(703));
    let initial_revision = store
        .upsert_draft(
            context,
            publisher,
            None,
            draft(tenant, workspace, VersionId::from_uuid(id(704))),
        )
        .await
        .expect("draft save")
        .revision;
    let cookie = issued_cookie(&store, vec![UserRole::Instructor], publisher).await;
    let app = router(
        Arc::clone(&store),
        Arc::new(FixtureRegistry {
            capabilities: BackendCapabilities::from_iter([Capability::ServerGrading]),
        }),
        Arc::new(ReviewNotRequired),
    );

    PUBLICATION_MINT_COUNT.with(|count| count.set(0));
    let missing = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/problems/{workspace}/publish"))
                .header("cookie", &cookie)
                .header("content-type", "application/json")
                .body(Body::from(
                    r#"{"scope":"institution","byline":{"names":["PLE fixture"]}}"#,
                ))
                .expect("missing revision request"),
        )
        .await
        .expect("missing revision response");
    assert_eq!(missing.status(), StatusCode::PRECONDITION_REQUIRED);
    assert_eq!(missing.headers()["cache-control"], "no-store");

    for malformed in ["W/\"1\"", "\"0\"", "\"9223372036854775808\""] {
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!("/api/problems/{workspace}/publish"))
                    .header("cookie", &cookie)
                    .header(IF_MATCH, malformed)
                    .header("content-type", "application/json")
                    .body(Body::from(
                        r#"{"scope":"institution","byline":{"names":["PLE fixture"]}}"#,
                    ))
                    .expect("malformed revision request"),
            )
            .await
            .expect("malformed revision response");
        assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
        assert_eq!(response.headers()["cache-control"], "no-store");
    }

    let current_revision = store
        .upsert_draft(
            context,
            publisher,
            Some(initial_revision),
            draft(tenant, workspace, VersionId::from_uuid(id(704))),
        )
        .await
        .expect("fixture update")
        .revision;
    assert_ne!(initial_revision, current_revision);

    let stale = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/problems/{workspace}/publish"))
                .header("cookie", cookie)
                .header(IF_MATCH, strong_if_match(initial_revision))
                .header("content-type", "application/json")
                .body(Body::from(
                    r#"{"scope":"institution","byline":{"names":["PLE fixture"]}}"#,
                ))
                .expect("stale revision request"),
        )
        .await
        .expect("stale revision response");
    assert_eq!(stale.status(), StatusCode::CONFLICT);
    assert_eq!(stale.headers()["cache-control"], "no-store");
    assert_eq!(PUBLICATION_MINT_COUNT.with(Cell::get), 0);
}

#[tokio::test]
async fn publication_refuses_a_collaborator_edit_that_arrives_during_review_before_minting() {
    let store = Arc::new(MemoryStore::default());
    let tenant = TenantId::from_uuid(id(1));
    let context = TenantContext::from_authenticated_session(tenant);
    let workspace = WorkspaceId::from_uuid(id(712));
    let publisher = UserId::from_uuid(id(713));
    let collaborator = UserId::from_uuid(id(714));
    let published_revision = store
        .upsert_draft(
            context,
            publisher,
            None,
            draft(tenant, workspace, VersionId::from_uuid(id(715))),
        )
        .await
        .expect("draft save")
        .revision;
    store
        .grant_draft_collaborator(context, publisher, workspace, collaborator)
        .await
        .expect("collaborator grant");
    let cookie = issued_cookie(&store, vec![UserRole::Instructor], publisher).await;
    let app = router(
        Arc::clone(&store),
        Arc::new(FixtureRegistry {
            capabilities: BackendCapabilities::from_iter([Capability::ServerGrading]),
        }),
        Arc::new(CollaboratorEditingReviewGate {
            store: Arc::clone(&store),
            collaborator,
        }),
    );

    PUBLICATION_MINT_COUNT.with(|count| count.set(0));
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/problems/{workspace}/publish"))
                .header("cookie", cookie)
                .header(IF_MATCH, strong_if_match(published_revision))
                .header("content-type", "application/json")
                .body(Body::from(
                    r#"{"scope":"public","byline":{"names":["PLE fixture"]}}"#,
                ))
                .expect("publication request"),
        )
        .await
        .expect("publication response");
    assert_eq!(response.status(), StatusCode::CONFLICT);
    assert_eq!(PUBLICATION_MINT_COUNT.with(Cell::get), 0);
    assert_eq!(
        store
            .get_draft(context, publisher, workspace)
            .await
            .expect("draft reload")
            .expect("draft stays editable")
            .revision
            .value(),
        2
    );
}

#[tokio::test]
async fn same_tenant_nonowner_publisher_cannot_mint_from_a_private_workspace() {
    let store = Arc::new(MemoryStore::default());
    let tenant = TenantId::from_uuid(id(1));
    let workspace = WorkspaceId::from_uuid(id(81));
    let owner = UserId::from_uuid(id(82));
    let owner_revision = store
        .upsert_draft(
            TenantContext::from_authenticated_session(tenant),
            owner,
            None,
            draft(tenant, workspace, VersionId::from_uuid(id(83))),
        )
        .await
        .expect("owner draft save")
        .revision;
    let nonowner = UserId::from_uuid(id(84));
    let cookie = issued_cookie(&store, vec![UserRole::Instructor], nonowner).await;
    let app = router(
        Arc::clone(&store),
        Arc::new(FixtureRegistry {
            capabilities: BackendCapabilities::from_iter([Capability::ServerGrading]),
        }),
        Arc::new(ReviewNotRequired),
    );

    PUBLICATION_MINT_COUNT.with(|count| count.set(0));
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/problems/{workspace}/publish"))
                .header("cookie", cookie)
                .header(IF_MATCH, strong_if_match(owner_revision))
                .header("content-type", "application/json")
                .body(Body::from(
                    r#"{"scope":"institution","byline":{"names":["PLE fixture"]}}"#,
                ))
                .expect("publish request"),
        )
        .await
        .expect("publish response");

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    assert_eq!(PUBLICATION_MINT_COUNT.with(Cell::get), 0);
}

#[tokio::test]
async fn changed_server_capabilities_refuse_before_minting_and_preserve_the_draft() {
    let store = Arc::new(MemoryStore::default());
    let tenant = TenantId::from_uuid(id(1));
    let context = TenantContext::from_authenticated_session(tenant);
    let workspace = WorkspaceId::from_uuid(id(85));
    let publisher = UserId::from_uuid(id(86));
    let candidate = draft(tenant, workspace, VersionId::from_uuid(id(87)));
    let draft_revision = store
        .upsert_draft(context, publisher, None, candidate.clone())
        .await
        .expect("draft save")
        .revision;
    let cookie = issued_cookie(&store, vec![UserRole::Instructor], publisher).await;
    let registry = Arc::new(ChangingRegistry {
        initial: BackendCapabilities::from_iter([Capability::ServerGrading]),
        current: BackendCapabilities::none(),
        calls: AtomicUsize::new(0),
    });
    let app = router(
        Arc::clone(&store),
        Arc::clone(&registry),
        Arc::new(ReviewNotRequired),
    );

    PUBLICATION_MINT_COUNT.with(|count| count.set(0));
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/problems/{workspace}/publish"))
                .header("cookie", cookie)
                .header(IF_MATCH, strong_if_match(draft_revision))
                .header("content-type", "application/json")
                .body(Body::from(
                    r#"{"scope":"institution","byline":{"names":["PLE fixture"]}}"#,
                ))
                .expect("publish request"),
        )
        .await
        .expect("publish response");

    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(PUBLICATION_MINT_COUNT.with(Cell::get), 0);
    assert_eq!(registry.calls.load(Ordering::SeqCst), 2);
    assert_eq!(
        store
            .get_draft(context, publisher, workspace)
            .await
            .map(|draft| draft.map(|draft| draft.record)),
        Ok(Some(candidate)),
    );
}

#[tokio::test]
async fn unprepared_imathas_refusal_preserves_draft_without_minting_an_identity() {
    let store = Arc::new(MemoryStore::default());
    let tenant = TenantId::from_uuid(id(1));
    let workspace = WorkspaceId::from_uuid(id(91));
    let mut candidate = draft(tenant, workspace, VersionId::from_uuid(id(92)));
    candidate.question.source = DraftQuestionSource::Imathas {
        provider: "institution-imathas".to_string(),
        item_ref: "1842".to_string(),
    };
    let publisher = UserId::from_uuid(id(93));
    let draft_revision = store
        .upsert_draft(
            TenantContext::from_authenticated_session(tenant),
            publisher,
            None,
            candidate,
        )
        .await
        .expect("draft save")
        .revision;
    let cookie = issued_cookie(&store, vec![UserRole::Instructor], publisher).await;
    let app = router(
        Arc::clone(&store),
        Arc::new(FixtureRegistry {
            capabilities: BackendCapabilities::from_iter([Capability::ServerGrading]),
        }),
        Arc::new(ReviewNotRequired),
    );

    // The thread-local seam observes this route task only. Source
    // preparation is intentionally before mint_publication_reference.
    PUBLICATION_MINT_COUNT.with(|count| count.set(0));
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/problems/{workspace}/publish"))
                .header("cookie", cookie)
                .header(IF_MATCH, strong_if_match(draft_revision))
                .header("content-type", "application/json")
                .body(Body::from(
                    r#"{"scope":"institution","byline":{"names":["PLE fixture"]}}"#,
                ))
                .expect("publish request"),
        )
        .await
        .expect("publish response");

    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(PUBLICATION_MINT_COUNT.with(Cell::get), 0);
    assert!(
        store
            .get_draft(
                TenantContext::from_authenticated_session(tenant),
                publisher,
                workspace,
            )
            .await
            .expect("draft lookup")
            .is_some()
    );
}

#[tokio::test]
async fn corrupt_legacy_titles_refuse_at_http_boundary_before_minting() {
    let tenant = TenantId::from_uuid(id(1));
    let context = TenantContext::from_authenticated_session(tenant);
    for (offset, title) in [(0_u128, " \t\n ".to_string()), (1, "\u{1F9EC}".repeat(513))] {
        let store = Arc::new(MemoryStore::default());
        let workspace = WorkspaceId::from_uuid(id(300 + offset));
        let mut legacy = draft(tenant, workspace, VersionId::from_uuid(id(310 + offset)));
        legacy.question.metadata.title = title;
        store
            .insert_legacy_draft_for_test(legacy.clone())
            .expect("legacy injection is test-only");
        let cookie = issued_cookie(
            &store,
            vec![UserRole::Instructor],
            UserId::from_uuid(id(320 + offset)),
        )
        .await;
        let app = router(
            Arc::clone(&store),
            Arc::new(FixtureRegistry {
                capabilities: BackendCapabilities::from_iter([Capability::ServerGrading]),
            }),
            Arc::new(ReviewNotRequired),
        );

        PUBLICATION_MINT_COUNT.with(|count| count.set(0));
        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!("/api/problems/{workspace}/publish"))
                    .header("cookie", cookie)
                    .header(IF_MATCH, "\"1\"")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        r#"{"scope":"institution","byline":{"names":["PLE fixture"]}}"#,
                    ))
                    .expect("publish request"),
            )
            .await
            .expect("publish response");

        // Legacy rows without an explicitly migrated owner remain absent
        // to every actor; a later caller must never acquire them merely
        // by attempting publication.
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
        assert_eq!(PUBLICATION_MINT_COUNT.with(Cell::get), 0);
        assert!(
            store
                .get_draft(context, UserId::from_uuid(id(320 + offset)), workspace)
                .await
                .expect("draft lookup")
                .is_none(),
            "unowned legacy data must not become visible to the caller"
        );
    }
}

#[tokio::test]
async fn every_unprepared_source_backed_draft_refuses_before_identity_minting() {
    // `issued_cookie` deliberately models the fixture institution tenant.
    let tenant = TenantId::from_uuid(id(1));
    let context = TenantContext::from_authenticated_session(tenant);
    let sources = [
        DraftQuestionSource::Webwork {
            pg_path: "Library/Calc/test.pg".to_string(),
        },
        DraftQuestionSource::Qti {
            item_id: "item-1".to_string(),
            import_id: WorkspaceImportId::from_uuid(id(511)),
        },
        DraftQuestionSource::H5p {
            content_type: "H5P.MultiChoice".to_string(),
        },
        DraftQuestionSource::Imathas {
            provider: "institution-imathas".to_string(),
            item_ref: "1842".to_string(),
        },
    ];
    for (offset, source) in sources.into_iter().enumerate() {
        let store = Arc::new(MemoryStore::default());
        let workspace = WorkspaceId::from_uuid(id(121 + offset as u128));
        let mut candidate = draft(
            tenant,
            workspace,
            VersionId::from_uuid(id(130 + offset as u128)),
        );
        candidate.question.source = source;
        let publisher = UserId::from_uuid(id(140 + offset as u128));
        let draft_revision = store
            .upsert_draft(context, publisher, None, candidate.clone())
            .await
            .expect("source-backed draft should save")
            .revision;
        let cookie = issued_cookie(&store, vec![UserRole::Instructor], publisher).await;
        let app = router(
            Arc::clone(&store),
            Arc::new(FixtureRegistry {
                capabilities: BackendCapabilities::from_iter([Capability::ServerGrading]),
            }),
            Arc::new(ReviewNotRequired),
        );
        PUBLICATION_MINT_COUNT.with(|count| count.set(0));
        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!("/api/problems/{workspace}/publish"))
                    .header("cookie", cookie)
                    .header(IF_MATCH, strong_if_match(draft_revision))
                    .header("content-type", "application/json")
                    .body(Body::from(
                        r#"{"scope":"institution","byline":{"names":["PLE fixture"]}}"#,
                    ))
                    .expect("publish request"),
            )
            .await
            .expect("publish response");
        assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
        assert_eq!(PUBLICATION_MINT_COUNT.with(Cell::get), 0);
        assert_eq!(
            store
                .get_draft(context, publisher, workspace)
                .await
                .map(|draft| draft.map(|draft| draft.record)),
            Ok(Some(candidate))
        );
    }
}

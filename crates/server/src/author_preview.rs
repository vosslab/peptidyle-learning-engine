//! Protected instructor answer presentation for private workspace drafts.
//!
//! This route is intentionally separate from the browser/WASM draft preview.
//! It resolves an actor-scoped private draft, asks the server-only native
//! adapter for display-ready teaching blocks, and never serializes an answer
//! key, grading rule, source locator, or published identity.

use std::sync::Arc;

use axum::extract::{Path, Query, State};
use axum::http::header::{ETAG, IF_MATCH};
use axum::http::{HeaderMap, HeaderValue, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use axum::{Json, Router, middleware};
use question_model::catalog::QuestionBackend;
use question_model::envelope::ContentBlock;
use question_model::generation::Seed;
use question_model::response::ResponseDefinition;
use question_model::{DraftQuestionSource, UserRole, WorkspaceId};
use serde::{Deserialize, Serialize};
use store::{SessionStore, Store, StoreError, WorkspaceDraftRevision};

use crate::auth::{auth_error_response, no_store, resolve_request_session};

/// Browser preview seeds are unsigned 32-bit values.  Keeping this exact
/// range aligned with the TypeScript boundary avoids a browser/server variant
/// disagreement from JavaScript number handling.
const MAX_AUTHOR_PREVIEW_SEED: u64 = u32::MAX as u64;

/// Builds the actor-scoped server-only author-presentation route.
pub fn router<S>(store: Arc<S>, native: Arc<adapter_native::NativeAdapter>) -> Router
where
    S: Store + SessionStore + 'static,
{
    Router::new()
        .route(
            "/api/workspaces/{workspace}/author-preview",
            get(author_preview::<S>),
        )
        // Also covers extractor rejections such as a missing/malformed seed.
        .layer(middleware::map_response(no_store_response))
        .with_state(AuthorPreviewState { store, native })
}

struct AuthorPreviewState<S> {
    store: Arc<S>,
    native: Arc<adapter_native::NativeAdapter>,
}

impl<S> Clone for AuthorPreviewState<S> {
    fn clone(&self) -> Self {
        Self {
            store: Arc::clone(&self.store),
            native: Arc::clone(&self.native),
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct AuthorPreviewQuery {
    seed: u64,
}

/// Exact browser response for the private instructor answer-key view.
///
/// `Available` contains rendered educational content, not the underlying
/// `AnswerKey`: clients cannot use it as a reusable grading contract.  The
/// `Unavailable` form is used for external sources and native families that
/// have not proven a safe display-ready presentation.
#[derive(Clone, PartialEq, Serialize)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum AuthorPreviewResponse {
    /// One deterministic native variant with instructor teaching material.
    Available {
        /// Learner-facing question title.
        title: String,
        /// Materialized learner-facing prompt.
        prompt: Vec<ContentBlock>,
        /// Browser-safe response shape.
        response: ResponseDefinition,
        /// Deterministic seed selected by the author.
        seed: u64,
        /// Display-ready explanation of the correct response.
        correct_response: Vec<ContentBlock>,
        /// Optional teaching rationale supplied by the native family.
        #[serde(skip_serializing_if = "Option::is_none")]
        rationale: Option<Vec<ContentBlock>>,
    },
    /// The source cannot supply a reviewed server-side author presentation.
    Unavailable {
        /// Adapter family, without its private source locator.
        backend: QuestionBackend,
        /// Stable, UI-safe reason code.
        reason: AuthorPreviewUnavailableReason,
    },
}

/// Stable reasons for an intentionally unavailable author answer view.
#[derive(Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum AuthorPreviewUnavailableReason {
    /// The source must use its dedicated server-side preview workflow.
    SourceRequiresServerPreview,
    /// This native family has not supplied a reviewed display-safe answer view.
    NativeFamilyPresentationUnavailable,
}

async fn author_preview<S>(
    State(state): State<AuthorPreviewState<S>>,
    headers: HeaderMap,
    Path(workspace): Path<WorkspaceId>,
    Query(query): Query<AuthorPreviewQuery>,
) -> Response
where
    S: Store + SessionStore + 'static,
{
    let authenticated = match resolve_request_session(state.store.as_ref(), &headers).await {
        Ok(authenticated) => authenticated,
        Err(error) => return auth_error_response(error),
    };
    if !may_author_workspaces(authenticated.record.subject.roles()) {
        // The author-presentation endpoint is a private exact-workspace
        // surface.  Returning the same result as a missing binding prevents a
        // student from using it as an existence oracle.
        return error_response(StatusCode::NOT_FOUND, "workspace not found");
    }
    if query.seed > MAX_AUTHOR_PREVIEW_SEED {
        return error_response(
            StatusCode::UNPROCESSABLE_ENTITY,
            "author preview seed must fit the browser-safe range",
        );
    }
    let draft = match state
        .store
        .get_draft(
            authenticated.tenant_context,
            authenticated.record.subject.user(),
            workspace,
        )
        .await
    {
        Ok(Some(draft)) => draft,
        Ok(None) | Err(StoreError::TenantMismatch | StoreError::Forbidden) => {
            return error_response(StatusCode::NOT_FOUND, "workspace not found");
        }
        Err(error) => return store_error_response(error),
    };
    let expected_revision = match required_revision(&headers) {
        Ok(revision) => revision,
        Err(RequiredRevisionError::Missing) => {
            return error_response(
                StatusCode::PRECONDITION_REQUIRED,
                "If-Match is required for an author preview",
            );
        }
        Err(RequiredRevisionError::Malformed) => {
            return error_response(
                StatusCode::UNPROCESSABLE_ENTITY,
                "If-Match must contain one strong workspace revision",
            );
        }
    };
    if expected_revision != draft.revision {
        return error_response(StatusCode::CONFLICT, "workspace changed; reload it");
    }

    let revision = draft.revision;
    let backend = QuestionBackend::from(&draft.record.question.source);
    if !matches!(
        draft.record.question.source,
        DraftQuestionSource::Native { .. }
    ) {
        return revisioned_response(
            revision,
            AuthorPreviewResponse::Unavailable {
                backend,
                reason: AuthorPreviewUnavailableReason::SourceRequiresServerPreview,
            },
        );
    }
    match state
        .native
        .author_presentation(&draft.record.question, Seed::new(query.seed))
    {
        Ok(Some(presentation)) => revisioned_response(
            revision,
            AuthorPreviewResponse::Available {
                title: presentation.title,
                prompt: presentation.prompt,
                response: presentation.response,
                seed: query.seed,
                correct_response: presentation.correct_response,
                rationale: presentation.rationale,
            },
        ),
        Ok(None)
        | Err(adapter_native::NativeAdapterError::UnsupportedSource)
        | Err(adapter_native::NativeAdapterError::UnknownFamily(_))
        | Err(adapter_native::NativeAdapterError::UnknownGenerator { .. }) => revisioned_response(
            revision,
            AuthorPreviewResponse::Unavailable {
                backend,
                reason: AuthorPreviewUnavailableReason::NativeFamilyPresentationUnavailable,
            },
        ),
        // Adapter failures can include authored field names or family-specific
        // validation detail.  This protected answer view returns a stable
        // recovery message rather than reflecting private draft content.
        Err(_) => error_response(
            StatusCode::UNPROCESSABLE_ENTITY,
            "author preview cannot be generated for this saved draft",
        ),
    }
}

fn may_author_workspaces(roles: &[UserRole]) -> bool {
    roles.iter().any(|role| {
        matches!(
            role,
            UserRole::Instructor | UserRole::Publisher | UserRole::Administrator
        )
    })
}

fn store_error_response(error: StoreError) -> Response {
    match error {
        StoreError::NotFound | StoreError::TenantMismatch | StoreError::Forbidden => {
            error_response(StatusCode::NOT_FOUND, "workspace not found")
        }
        StoreError::Unavailable(_) => error_response(
            StatusCode::SERVICE_UNAVAILABLE,
            "workspace storage unavailable",
        ),
        StoreError::InvalidRecord(message) => {
            error_response(StatusCode::UNPROCESSABLE_ENTITY, &message)
        }
        StoreError::AlreadyExists | StoreError::Conflict | StoreError::TimedOut => {
            error_response(StatusCode::CONFLICT, "workspace changed; reload it")
        }
        StoreError::RunModel(error) => {
            error_response(StatusCode::UNPROCESSABLE_ENTITY, &error.to_string())
        }
    }
}

fn error_response(status: StatusCode, message: &str) -> Response {
    no_store((status, Json(serde_json::json!({ "error": message }))).into_response())
}

async fn no_store_response(response: Response) -> Response {
    no_store(response)
}

/// Binds a protected author answer view to the exact saved draft revision.
/// The UI can warn before showing a view generated from a stale workspace;
/// no internal identity or answer material is added to the JSON body.
fn revisioned_response(revision: WorkspaceDraftRevision, body: AuthorPreviewResponse) -> Response {
    let mut response = Json(body).into_response();
    response.headers_mut().insert(
        ETAG,
        HeaderValue::from_str(&format!("\"{}\"", revision.value()))
            .expect("a decimal workspace revision is a valid ETag"),
    );
    no_store(response)
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum RequiredRevisionError {
    Missing,
    Malformed,
}

/// Accepts exactly one positive strong workspace revision ETag.  This is kept
/// at the protected endpoint rather than trusting a client JSON field: the
/// revision is a server-issued concurrency token, not authored content.
fn required_revision(headers: &HeaderMap) -> Result<WorkspaceDraftRevision, RequiredRevisionError> {
    let mut values = headers.get_all(IF_MATCH).iter();
    let Some(value) = values.next() else {
        return Err(RequiredRevisionError::Missing);
    };
    if values.next().is_some() {
        return Err(RequiredRevisionError::Malformed);
    }
    let value = value
        .to_str()
        .map_err(|_| RequiredRevisionError::Malformed)?;
    let Some(value) = value
        .strip_prefix('"')
        .and_then(|value| value.strip_suffix('"'))
    else {
        return Err(RequiredRevisionError::Malformed);
    };
    if value.is_empty() || !value.bytes().all(|byte| byte.is_ascii_digit()) {
        return Err(RequiredRevisionError::Malformed);
    }
    let numeric = value
        .parse::<u64>()
        .map_err(|_| RequiredRevisionError::Malformed)?;
    if numeric == 0 || numeric > i64::MAX as u64 {
        return Err(RequiredRevisionError::Malformed);
    }
    serde_json::from_str(value).map_err(|_| RequiredRevisionError::Malformed)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    use axum::body::{Body, to_bytes};
    use axum::http::Request;
    use question_model::answer::SelectionCardinality;
    use question_model::envelope::ContentBlock;
    use question_model::generation::{GeneratorReference, ParameterSpec, RandomizationDefinition};
    use question_model::response::{ChoiceId, ChoiceOption};
    use question_model::run_policy::{AttemptPolicy, FeedbackDisclosure, TimingPolicy};
    use question_model::taxonomy::License;
    use question_model::{
        DraftQuestionDefinition, DraftQuestionSource, GradingDefinition, QuestionMetadata,
        TenantId, UserId,
    };
    use store::memory::MemoryStore;
    use store::{DraftRecord, SessionLifetime, SessionSubject, TenantContext};
    use tower::ServiceExt;
    use uuid::Uuid;

    #[test]
    fn author_preview_wire_shape_excludes_raw_key_and_locators() {
        let response = AuthorPreviewResponse::Available {
            title: "Peptide geometry".to_string(),
            prompt: vec![ContentBlock::Text {
                markdown: "Which linkage is planar?".to_string(),
            }],
            response: ResponseDefinition::ShortText {
                match_mode: question_model::answer::TextMatchMode::Exact,
                max_length: 32,
            },
            seed: 7,
            correct_response: vec![ContentBlock::Text {
                markdown: "amide linkage".to_string(),
            }],
            rationale: None,
        };
        let value = serde_json::to_value(response).expect("author preview serializes");
        assert_eq!(value["kind"], "available");
        assert_eq!(value["correctResponse"][0]["markdown"], "amide linkage");
        for forbidden in [
            "answerKey",
            "correct",
            "expected",
            "grading",
            "source",
            "provider",
            "problem",
            "version",
            "workspace",
        ] {
            assert!(
                value.get(forbidden).is_none(),
                "must not expose {forbidden}"
            );
        }
    }

    #[test]
    fn unavailable_wire_shape_names_backend_without_locator() {
        let value = serde_json::to_value(AuthorPreviewResponse::Unavailable {
            backend: QuestionBackend::Imathas,
            reason: AuthorPreviewUnavailableReason::SourceRequiresServerPreview,
        })
        .expect("unavailable author preview serializes");
        assert_eq!(value["kind"], "unavailable");
        assert_eq!(value["backend"], "imathas");
        assert!(value.get("provider").is_none());
        assert!(value.get("itemRef").is_none());
    }

    fn id(value: u128) -> Uuid {
        Uuid::from_u128(value)
    }

    fn choice(id: &str, text: &str) -> ChoiceOption {
        ChoiceOption {
            id: ChoiceId::new(id),
            body: vec![ContentBlock::Text {
                markdown: text.to_string(),
            }],
        }
    }

    fn native_draft(workspace: WorkspaceId) -> DraftQuestionDefinition {
        let mut parameters = BTreeMap::new();
        parameters.insert(
            "residue".to_string(),
            ParameterSpec::Choice {
                options: vec!["alanine".to_string(), "glycine".to_string()],
            },
        );
        DraftQuestionDefinition {
            workspace,
            source: DraftQuestionSource::Native {
                family: adapter_native::peptide_bond_geometry::FAMILY_ID.to_string(),
            },
            prompt: vec![ContentBlock::Text {
                markdown: "In a peptide containing {{residue}}, which linkage is planar?"
                    .to_string(),
            }],
            response: ResponseDefinition::MultipleChoice {
                choices: vec![
                    choice("ester", "ester linkage"),
                    choice("amide", "amide linkage"),
                ],
                selection: SelectionCardinality::ExactlyOne,
            },
            attempt_policy: AttemptPolicy {
                max_attempts: None,
                feedback: FeedbackDisclosure::ImmediateCorrectness,
            },
            timing_policy: TimingPolicy::Untimed,
            randomization: RandomizationDefinition::Seeded {
                generator: GeneratorReference {
                    id: adapter_native::peptide_bond_geometry::GENERATOR_ID.to_string(),
                    version: adapter_native::peptide_bond_geometry::GENERATOR_VERSION.to_string(),
                },
                parameters,
            },
            grading: GradingDefinition::AllOrNothing { points: 1.0 },
            metadata: QuestionMetadata {
                title: "Peptide geometry".to_string(),
                tags: Vec::new(),
                taxonomy: Vec::new(),
                license: License::CcBy,
                language: "en-US".to_string(),
            },
        }
    }

    fn external_draft(workspace: WorkspaceId) -> DraftQuestionDefinition {
        let mut draft = native_draft(workspace);
        draft.source = DraftQuestionSource::Webwork {
            pg_path: "Library/Basic/answer.pg".to_string(),
        };
        draft
    }

    async fn issued_cookie(
        store: &MemoryStore,
        tenant: TenantId,
        roles: Vec<UserRole>,
        user: UserId,
    ) -> String {
        let subject =
            SessionSubject::new(tenant, user, "Preview Fixture", roles).expect("fixture identity");
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
        serde_json::from_slice(
            &to_bytes(response.into_body(), 128 * 1024)
                .await
                .expect("response body"),
        )
        .expect("JSON response")
    }

    #[tokio::test]
    async fn owner_gets_deterministic_display_ready_native_answer_without_key_material() {
        let store = Arc::new(MemoryStore::default());
        let tenant = TenantId::from_uuid(id(1));
        let owner = UserId::from_uuid(id(2));
        let collaborator = UserId::from_uuid(id(4));
        let workspace = WorkspaceId::from_uuid(id(3));
        let cookie = issued_cookie(&store, tenant, vec![UserRole::Instructor], owner).await;
        let collaborator_cookie =
            issued_cookie(&store, tenant, vec![UserRole::Instructor], collaborator).await;
        let saved = store
            .upsert_draft(
                TenantContext::from_authenticated_session(tenant),
                owner,
                None,
                DraftRecord {
                    tenant,
                    question: native_draft(workspace),
                    revises: None,
                    derived_from: None,
                },
            )
            .await
            .expect("owner seed draft");
        store
            .grant_draft_collaborator(
                TenantContext::from_authenticated_session(tenant),
                owner,
                workspace,
                collaborator,
            )
            .await
            .expect("owner grants preview collaborator");
        let app = router(
            Arc::clone(&store),
            Arc::new(adapter_native::NativeAdapter::new()),
        );
        let endpoint = format!("/api/workspaces/{workspace}/author-preview?seed=17");
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri(&endpoint)
                    .header("cookie", &cookie)
                    .header(IF_MATCH, format!("\"{}\"", saved.revision.value()))
                    .body(Body::empty())
                    .expect("preview request"),
            )
            .await
            .expect("preview response");
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(response.headers()["cache-control"], "no-store");
        assert_eq!(
            response.headers()[ETAG],
            format!("\"{}\"", saved.revision.value())
        );
        let value = response_json(response).await;
        assert_eq!(value["kind"], "available");
        assert_eq!(value["seed"], 17);
        assert_eq!(value["correctResponse"][0]["markdown"], "amide linkage");
        assert!(
            value["rationale"][0]["markdown"]
                .as_str()
                .is_some_and(|text| text.contains("partial double-bond"))
        );
        for forbidden in [
            "answerKey",
            "expected",
            "correct",
            "grading",
            "source",
            "provider",
            "itemRef",
            "problem",
            "version",
        ] {
            assert!(
                value.get(forbidden).is_none(),
                "must not expose {forbidden}"
            );
        }

        let collaborator_response = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri(&endpoint)
                    .header("cookie", collaborator_cookie)
                    .header(IF_MATCH, format!("\"{}\"", saved.revision.value()))
                    .body(Body::empty())
                    .expect("collaborator preview request"),
            )
            .await
            .expect("collaborator preview response");
        assert_eq!(collaborator_response.status(), StatusCode::OK);
        assert_eq!(
            collaborator_response.headers()[ETAG],
            format!("\"{}\"", saved.revision.value())
        );
        assert_eq!(response_json(collaborator_response).await, value);

        let varied_response = app
            .oneshot(
                Request::builder()
                    .uri(format!("/api/workspaces/{workspace}/author-preview?seed=1"))
                    .header("cookie", cookie)
                    .header(IF_MATCH, format!("\"{}\"", saved.revision.value()))
                    .body(Body::empty())
                    .expect("varied preview request"),
            )
            .await
            .expect("varied preview response");
        assert_eq!(varied_response.status(), StatusCode::OK);
        let varied = response_json(varied_response).await;
        assert_ne!(varied["prompt"], value["prompt"]);
    }

    #[tokio::test]
    async fn external_sources_are_explicitly_unavailable_and_private_drafts_do_not_enumerate() {
        let store = Arc::new(MemoryStore::default());
        let tenant = TenantId::from_uuid(id(10));
        let owner = UserId::from_uuid(id(11));
        let noncollaborator = UserId::from_uuid(id(12));
        let foreign_tenant = TenantId::from_uuid(id(13));
        let foreign_user = UserId::from_uuid(id(14));
        let workspace = WorkspaceId::from_uuid(id(15));
        store
            .upsert_draft(
                TenantContext::from_authenticated_session(tenant),
                owner,
                None,
                DraftRecord {
                    tenant,
                    question: external_draft(workspace),
                    revises: None,
                    derived_from: None,
                },
            )
            .await
            .expect("owner seed external draft");
        let owner_cookie = issued_cookie(&store, tenant, vec![UserRole::Instructor], owner).await;
        let noncollaborator_cookie =
            issued_cookie(&store, tenant, vec![UserRole::Instructor], noncollaborator).await;
        let foreign_cookie = issued_cookie(
            &store,
            foreign_tenant,
            vec![UserRole::Instructor],
            foreign_user,
        )
        .await;
        let student_cookie = issued_cookie(
            &store,
            tenant,
            vec![UserRole::Student],
            UserId::from_uuid(id(16)),
        )
        .await;
        let app = router(
            Arc::clone(&store),
            Arc::new(adapter_native::NativeAdapter::new()),
        );
        let endpoint = format!("/api/workspaces/{workspace}/author-preview?seed=2");

        let owner_response = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri(&endpoint)
                    .header("cookie", owner_cookie)
                    .header(IF_MATCH, "\"1\"")
                    .body(Body::empty())
                    .expect("owner request"),
            )
            .await
            .expect("owner response");
        assert_eq!(owner_response.status(), StatusCode::OK);
        let owner_value = response_json(owner_response).await;
        assert_eq!(owner_value["kind"], "unavailable");
        assert_eq!(owner_value["backend"], "webwork");
        assert_eq!(owner_value["reason"], "sourceRequiresServerPreview");

        for cookie in [noncollaborator_cookie, foreign_cookie] {
            let response = app
                .clone()
                .oneshot(
                    Request::builder()
                        .uri(&endpoint)
                        .header("cookie", cookie)
                        .header(IF_MATCH, "\"1\"")
                        .body(Body::empty())
                        .expect("private request"),
                )
                .await
                .expect("private response");
            assert_eq!(response.status(), StatusCode::NOT_FOUND);
            assert_eq!(response.headers()["cache-control"], "no-store");
        }
        let student_response = app
            .oneshot(
                Request::builder()
                    .uri(endpoint)
                    .header("cookie", student_cookie)
                    .header(IF_MATCH, "\"1\"")
                    .body(Body::empty())
                    .expect("student request"),
            )
            .await
            .expect("student response");
        assert_eq!(student_response.status(), StatusCode::NOT_FOUND);
        assert_eq!(student_response.headers()["cache-control"], "no-store");
    }

    #[tokio::test]
    async fn author_preview_requires_the_exact_saved_revision_before_exposing_content() {
        let store = Arc::new(MemoryStore::default());
        let tenant = TenantId::from_uuid(id(21));
        let owner = UserId::from_uuid(id(22));
        let workspace = WorkspaceId::from_uuid(id(23));
        store
            .upsert_draft(
                TenantContext::from_authenticated_session(tenant),
                owner,
                None,
                DraftRecord {
                    tenant,
                    question: native_draft(workspace),
                    revises: None,
                    derived_from: None,
                },
            )
            .await
            .expect("owner seed draft");
        let cookie = issued_cookie(&store, tenant, vec![UserRole::Instructor], owner).await;
        let app = router(
            Arc::clone(&store),
            Arc::new(adapter_native::NativeAdapter::new()),
        );
        let endpoint = format!("/api/workspaces/{workspace}/author-preview?seed=3");

        for (header, status) in [
            (None, StatusCode::PRECONDITION_REQUIRED),
            (Some("W/\"1\""), StatusCode::UNPROCESSABLE_ENTITY),
            (Some("\"2\""), StatusCode::CONFLICT),
        ] {
            let mut request = Request::builder().uri(&endpoint).header("cookie", &cookie);
            if let Some(header) = header {
                request = request.header(IF_MATCH, header);
            }
            let response = app
                .clone()
                .oneshot(request.body(Body::empty()).expect("precondition request"))
                .await
                .expect("precondition response");
            assert_eq!(response.status(), status);
            assert_eq!(response.headers()["cache-control"], "no-store");
            let body = response_json(response).await;
            assert!(body["error"].is_string());
            for forbidden in ["prompt", "correctResponse", "rationale", "answerKey"] {
                assert!(
                    body.get(forbidden).is_none(),
                    "precondition error must redact {forbidden}"
                );
            }
        }

        let accepted = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri(format!(
                        "/api/workspaces/{workspace}/author-preview?seed={MAX_AUTHOR_PREVIEW_SEED}"
                    ))
                    .header("cookie", &cookie)
                    .header(IF_MATCH, "\"1\"")
                    .body(Body::empty())
                    .expect("largest seed request"),
            )
            .await
            .expect("largest seed response");
        assert_eq!(accepted.status(), StatusCode::OK);
        assert_eq!(accepted.headers()["cache-control"], "no-store");

        let rejected = app
            .oneshot(
                Request::builder()
                    .uri(format!(
                        "/api/workspaces/{workspace}/author-preview?seed={}",
                        MAX_AUTHOR_PREVIEW_SEED + 1
                    ))
                    .header("cookie", cookie)
                    .header(IF_MATCH, "\"1\"")
                    .body(Body::empty())
                    .expect("out-of-range seed request"),
            )
            .await
            .expect("out-of-range seed response");
        assert_eq!(rejected.status(), StatusCode::UNPROCESSABLE_ENTITY);
        assert_eq!(rejected.headers()["cache-control"], "no-store");
    }
}

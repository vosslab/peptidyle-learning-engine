//! Authenticated HTTP fallbacks for browser-safe validation (MOD-CLIENT).
//!
//! These routes expose the same key-free pure functions as the WebAssembly
//! bridge. They keep the interface usable when WebAssembly cannot initialize,
//! but they do not replace trusted publication checks, server-clock timing, or
//! server-only grading.

use std::sync::Arc;

use axum::extract::{DefaultBodyLimit, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::post;
use axum::{Json, Router};
use domain::policy::{AssignmentConfig, Violation, validate_assignment_config};
use domain::timing::{TimerEvaluation, TimerVerdict, timer_verdict};
use domain::validation::{ResponseFormatReport, validate_response_format};
use question_model::response::{ResponseDefinition, StudentResponse};
use serde::{Deserialize, Serialize};
use store::SessionStore;

use crate::auth::{auth_error_response, no_store, resolve_request_session};

const MAX_VALIDATION_BODY_BYTES: usize = 256 * 1_024;

/// Builds the authenticated browser-safe validation fallback routes.
pub fn router<S>(store: Arc<S>) -> Router
where
    S: SessionStore + 'static,
{
    Router::new()
        .route(
            "/api/validation/response-format",
            post(response_format::<S>),
        )
        .route("/api/validation/timer", post(timer::<S>))
        .route(
            "/api/validation/assignment-capabilities",
            post(assignment_capabilities::<S>),
        )
        .layer(DefaultBodyLimit::max(MAX_VALIDATION_BODY_BYTES))
        .with_state(ValidationRouteState { store })
}

struct ValidationRouteState<S> {
    store: Arc<S>,
}

impl<S> Clone for ValidationRouteState<S> {
    fn clone(&self) -> Self {
        Self {
            store: Arc::clone(&self.store),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ResponseFormatRequest {
    definition: ResponseDefinition,
    response: StudentResponse,
}

async fn response_format<S>(
    State(state): State<ValidationRouteState<S>>,
    headers: HeaderMap,
    Json(request): Json<ResponseFormatRequest>,
) -> Response
where
    S: SessionStore + 'static,
{
    if let Err(error) = resolve_request_session(state.store.as_ref(), &headers).await {
        return auth_error_response(error);
    }
    let report: ResponseFormatReport =
        validate_response_format(&request.definition, &request.response);
    no_store(Json(report).into_response())
}

async fn timer<S>(
    State(state): State<ValidationRouteState<S>>,
    headers: HeaderMap,
    Json(evaluation): Json<TimerEvaluation>,
) -> Response
where
    S: SessionStore + 'static,
{
    if let Err(error) = resolve_request_session(state.store.as_ref(), &headers).await {
        return auth_error_response(error);
    }
    match timer_verdict(&evaluation) {
        Ok(verdict) => {
            let verdict: TimerVerdict = verdict;
            no_store(Json(verdict).into_response())
        }
        Err(error) => error_response(StatusCode::UNPROCESSABLE_ENTITY, &error.to_string()),
    }
}

async fn assignment_capabilities<S>(
    State(state): State<ValidationRouteState<S>>,
    headers: HeaderMap,
    Json(config): Json<AssignmentConfig>,
) -> Response
where
    S: SessionStore + 'static,
{
    if let Err(error) = resolve_request_session(state.store.as_ref(), &headers).await {
        return auth_error_response(error);
    }
    let violations: Vec<Violation> = validate_assignment_config(&config);
    no_store(Json(violations).into_response())
}

fn error_response(status: StatusCode, message: &str) -> Response {
    no_store((status, Json(serde_json::json!({ "error": message }))).into_response())
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::{Body, to_bytes};
    use axum::http::Request;
    use question_model::answer::TextMatchMode;
    use question_model::envelope::ContentBlock;
    use question_model::generation::RandomizationDefinition;
    use question_model::response::ResponseDefinition;
    use question_model::run_policy::{AttemptPolicy, FeedbackDisclosure, TimingPolicy};
    use question_model::taxonomy::License;
    use question_model::{
        ActivityTimestamp, BackendCapabilities, GradingDefinition, QuestionDefinition,
        QuestionMetadata, QuestionSource, TenantId, UserId, UserRole, VersionId, WorkspaceId,
    };
    use store::memory::MemoryStore;
    use store::{SessionLifetime, SessionSubject};
    use tower::ServiceExt;
    use uuid::Uuid;

    fn id(value: u128) -> Uuid {
        Uuid::from_u128(value)
    }

    async fn issued_cookie(store: &MemoryStore) -> String {
        let subject = SessionSubject::new(
            TenantId::from_uuid(id(1)),
            UserId::from_uuid(id(2)),
            "Validation Fixture",
            vec![UserRole::Student],
        )
        .expect("fixture identity");
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

    fn question() -> QuestionDefinition {
        QuestionDefinition {
            version: VersionId::from_uuid(id(10)),
            problem: None,
            workspace: WorkspaceId::from_uuid(id(11)),
            source: QuestionSource::Native {
                family: "validation-fixture".to_string(),
            },
            prompt: vec![ContentBlock::Text {
                markdown: "Name the bond joining amino acids.".to_string(),
            }],
            response: ResponseDefinition::ShortText {
                match_mode: TextMatchMode::Normalized,
                max_length: 7,
            },
            attempt_policy: AttemptPolicy {
                max_attempts: Some(1),
                feedback: FeedbackDisclosure::Deferred,
            },
            timing_policy: TimingPolicy::Untimed,
            randomization: RandomizationDefinition::Static,
            grading: GradingDefinition::AllOrNothing { points: 1.0 },
            metadata: QuestionMetadata {
                title: "Peptide bond".to_string(),
                tags: Vec::new(),
                taxonomy: Vec::new(),
                license: License::CcBy,
                language: "en-US".to_string(),
            },
        }
    }

    async fn json(response: Response) -> serde_json::Value {
        let body = to_bytes(response.into_body(), 512 * 1_024)
            .await
            .expect("response body");
        serde_json::from_slice(&body).expect("JSON response")
    }

    #[tokio::test]
    async fn authenticated_fallbacks_delegate_to_the_key_free_domain_functions() {
        let store = Arc::new(MemoryStore::default());
        store
            .set_authoritative_time(ActivityTimestamp::from_unix_millis(10_000))
            .expect("fixture clock");
        let cookie = issued_cookie(&store).await;
        let app = router(store);

        let format_request = ResponseFormatRequest {
            definition: question().response,
            response: StudentResponse::ShortText {
                text: "peptide bond".to_string(),
            },
        };
        let format_response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/validation/response-format")
                    .header("cookie", &cookie)
                    .header("content-type", "application/json")
                    .body(Body::from(
                        serde_json::to_vec(&format_request).expect("format request"),
                    ))
                    .expect("format HTTP request"),
            )
            .await
            .expect("format response");
        assert_eq!(format_response.status(), StatusCode::OK);
        assert_eq!(
            json(format_response).await,
            serde_json::json!({
                "violations": [{"kind": "textTooLong", "maxLength": 7, "actualLength": 12}]
            })
        );

        let timer_response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/validation/timer")
                    .header("cookie", &cookie)
                    .header("content-type", "application/json")
                    .body(Body::from(
                        r#"{
                            "policy":{"kind":"perQuestion","seconds":30,"graceSeconds":2},
                            "timer":{"issuedAt":1000,"deadline":31000,"submittedAt":null},
                            "evaluatedAt":2000,
                            "pauseExtensionMillis":0
                        }"#,
                    ))
                    .expect("timer request"),
            )
            .await
            .expect("timer response");
        assert_eq!(timer_response.status(), StatusCode::OK);
        assert_eq!(json(timer_response).await, serde_json::json!("open"));

        let capability_request = AssignmentConfig {
            questions: vec![domain::policy::AssignmentQuestionConfig {
                question: question(),
                backend_capabilities: BackendCapabilities::default(),
            }],
            required_capabilities: Vec::new(),
        };
        let capability_response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/validation/assignment-capabilities")
                    .header("cookie", &cookie)
                    .header("content-type", "application/json")
                    .body(Body::from(
                        serde_json::to_vec(&capability_request).expect("capability request"),
                    ))
                    .expect("capability HTTP request"),
            )
            .await
            .expect("capability response");
        assert_eq!(capability_response.status(), StatusCode::OK);
        assert_eq!(
            json(capability_response).await,
            serde_json::json!([{
                "question": VersionId::from_uuid(id(10)),
                "capability": "serverGrading"
            }])
        );
    }

    #[tokio::test]
    async fn fallback_routes_require_a_session_reject_bad_timers_and_bound_bodies() {
        let store = Arc::new(MemoryStore::default());
        store
            .set_authoritative_time(ActivityTimestamp::from_unix_millis(10_000))
            .expect("fixture clock");
        let cookie = issued_cookie(&store).await;
        let app = router(store);

        let anonymous = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/validation/assignment-capabilities")
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"questions":[],"requiredCapabilities":[]}"#))
                    .expect("anonymous request"),
            )
            .await
            .expect("anonymous response");
        assert_eq!(anonymous.status(), StatusCode::UNAUTHORIZED);
        assert_eq!(anonymous.headers()["cache-control"], "no-store");

        let malformed_timer = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/validation/timer")
                    .header("cookie", &cookie)
                    .header("content-type", "application/json")
                    .body(Body::from(
                        r#"{
                            "policy":{"kind":"untimed"},
                            "timer":{"issuedAt":1000,"deadline":2000,"submittedAt":null},
                            "evaluatedAt":2000,
                            "pauseExtensionMillis":0
                        }"#,
                    ))
                    .expect("malformed timer request"),
            )
            .await
            .expect("malformed timer response");
        assert_eq!(malformed_timer.status(), StatusCode::UNPROCESSABLE_ENTITY);
        assert_eq!(malformed_timer.headers()["cache-control"], "no-store");

        let oversized = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/validation/response-format")
                    .header("cookie", &cookie)
                    .header("content-type", "application/json")
                    .body(Body::from(vec![b' '; MAX_VALIDATION_BODY_BYTES + 1]))
                    .expect("oversized request"),
            )
            .await
            .expect("oversized response");
        assert_eq!(oversized.status(), StatusCode::PAYLOAD_TOO_LARGE);
    }
}

//! Safe process-level request telemetry and API shutdown behavior.
//!
//! This boundary intentionally observes only a server-minted correlation ID,
//! HTTP method, status, and elapsed time.  Paths, query strings, headers,
//! request bodies, response bodies, identity, answers, signed URLs, and object
//! names are not safe general-purpose telemetry fields and must not cross it.

use std::time::{Duration, Instant};

use axum::Router;
use axum::extract::Request;
use axum::http::{HeaderValue, StatusCode};
use axum::middleware::{self, Next};
use axum::response::{IntoResponse, Response};
use base64::Engine;
use tracing::Instrument;

/// A bounded drain lets an orchestrator replace an unhealthy instance without
/// allowing a wedged keep-alive or streaming request to hold the process
/// indefinitely. Submission handlers are already bound to one accepted Question
/// Submission per Question Attempt; the API
/// intentionally has no blanket per-request cancellation timeout because a
/// timeout after a durable commit would make a completed submission look
/// ambiguous to a student.
pub const API_DRAIN_TIMEOUT: Duration = Duration::from_secs(30);

const REQUEST_ID_HEADER: &str = "x-request-id";

/// Applies the one process-wide request boundary after every route is merged.
pub fn apply_request_lifecycle(router: Router) -> Router {
    router.layer(middleware::from_fn(request_lifecycle))
}

/// Mints a request ID locally and emits a deliberately low-cardinality,
/// answer-free completion event. Client supplied request identifiers are never
/// reflected or trusted, preventing log injection and cross-course correlation.
pub async fn request_lifecycle(request: Request, next: Next) -> Response {
    let request_id = match mint_request_id() {
        Ok(value) => value,
        Err(()) => {
            tracing::error!(
                event = "http_request_rejected",
                reason = "entropy_unavailable"
            );
            return StatusCode::SERVICE_UNAVAILABLE.into_response();
        }
    };
    let method = request.method().clone();
    let started = Instant::now();
    // SMTP and other trusted adapters inherit this span.  They may add a
    // bounded operator event without ever receiving browser input or a
    // recipient/token value as an argument.
    let request_span = tracing::info_span!("http_request", request_id = %request_id);
    let mut response = next.run(request).instrument(request_span).await;
    let elapsed_ms = u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX);

    // The generated value is URL/header safe by construction. Failure cannot
    // occur here, but retain a defensive fallback that never echoes client data.
    if let Ok(value) = HeaderValue::from_str(&request_id) {
        response.headers_mut().insert(REQUEST_ID_HEADER, value);
    }
    tracing::info!(
        event = "http_request_completed",
        request_id = %request_id,
        method = %method,
        status = response.status().as_u16(),
        elapsed_ms,
    );
    response
}

fn mint_request_id() -> Result<String, ()> {
    let mut bytes = [0_u8; 16];
    getrandom::fill(&mut bytes).map_err(|_| ())?;
    Ok(base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(bytes))
}

#[cfg(test)]
mod tests {
    use axum::body::Body;
    use axum::http::Request;
    use axum::routing::post;
    use tower::ServiceExt;

    use super::*;

    #[tokio::test]
    async fn lifecycle_mints_and_returns_a_server_owned_request_id() {
        let app = apply_request_lifecycle(Router::new().route(
            "/private/object-key",
            post(|| async { StatusCode::NO_CONTENT }),
        ));
        let response = app
            .oneshot(
                Request::post("/private/object-key?signature=not-for-logs")
                    .header(REQUEST_ID_HEADER, "attacker-controlled")
                    .body(Body::from("an answer must not enter telemetry"))
                    .expect("request"),
            )
            .await
            .expect("response");

        assert_eq!(response.status(), StatusCode::NO_CONTENT);
        let request_id = response
            .headers()
            .get(REQUEST_ID_HEADER)
            .expect("server request id")
            .to_str()
            .expect("ASCII request id");
        assert_ne!(request_id, "attacker-controlled");
        assert!(
            request_id
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-' || byte == b'_')
        );
    }
}

//! Move-only preservation of route-level HTTP refusals.

use std::fmt;

use axum::response::{IntoResponse, Response};

/// A route refusal that preserves the already-constructed HTTP transport.
///
/// Route constructors remain responsible for choosing every response detail.
/// This boundary only moves that complete response to the handler boundary, so
/// ASVS 3.3, 3.4, and 4.1 response controls remain unchanged.
pub(crate) struct HttpRefusal(Box<Response>);

/// Result returned by helpers that can finish a request with an HTTP refusal.
pub(crate) type HttpResult<T> = Result<T, HttpRefusal>;

impl HttpRefusal {
    /// Consumes this refusal and returns the unchanged raw Axum response.
    pub(crate) fn into_response(self) -> Response {
        *self.0
    }
}

impl From<Response> for HttpRefusal {
    fn from(response: Response) -> Self {
        Self(Box::new(response))
    }
}

impl IntoResponse for HttpRefusal {
    fn into_response(self) -> Response {
        self.into_response()
    }
}

impl fmt::Debug for HttpRefusal {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("HttpRefusal")
            .field("status", &self.0.status())
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use axum::body::{Body, to_bytes};
    use axum::http::header::{CACHE_CONTROL, SET_COOKIE};
    use axum::http::{HeaderName, Response, StatusCode};

    use super::HttpRefusal;

    #[tokio::test]
    async fn refusal_round_trip_preserves_complete_http_transport() {
        let response = Response::builder()
            .status(StatusCode::UNAUTHORIZED)
            .header(CACHE_CONTROL, "no-store")
            .header(SET_COOKIE, "__Host-ple-session=opaque; Secure; HttpOnly")
            .header("x-ple-refusal-marker", "preserve-me")
            .body(Body::from("refusal body"))
            .expect("response construction succeeds");

        let response = HttpRefusal::from(response).into_response();

        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
        assert_eq!(
            response.headers().get(CACHE_CONTROL),
            Some(&"no-store".parse().unwrap())
        );
        assert_eq!(
            response.headers().get(SET_COOKIE),
            Some(
                &"__Host-ple-session=opaque; Secure; HttpOnly"
                    .parse()
                    .unwrap()
            )
        );
        assert_eq!(
            response
                .headers()
                .get(HeaderName::from_static("x-ple-refusal-marker")),
            Some(&"preserve-me".parse().unwrap())
        );
        assert_eq!(
            to_bytes(response.into_body(), 1_024)
                .await
                .expect("refusal body remains readable"),
            "refusal body"
        );
    }
}

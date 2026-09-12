//! Fixed-origin, bounded JSON transport for Live Demo activity convergence.

use super::{BrowserEndpoint, MAX_RESPONSE_BYTES, REQUEST_TIMEOUT, TemporarySession};
use anyhow::{Result, ensure};
use reqwest::{Client, Method, StatusCode, header};
use serde_json::Value;

pub(super) struct ProductApi {
    client: Client,
    request_base: url::Url,
    origin_header: header::HeaderValue,
    host_header: header::HeaderValue,
}

impl ProductApi {
    pub(super) fn new(endpoint: &BrowserEndpoint) -> Result<Self> {
        let client = Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .timeout(REQUEST_TIMEOUT)
            .build()
            .map_err(|_| anyhow::anyhow!("Live Demo HTTP client is unavailable"))?;
        Ok(Self::with_client(endpoint, client))
    }

    fn with_client(endpoint: &BrowserEndpoint, client: Client) -> Self {
        Self {
            client,
            request_base: endpoint.request_base.clone(),
            origin_header: endpoint.origin_header.clone(),
            host_header: endpoint.host_header.clone(),
        }
    }

    pub(super) async fn request(
        &self,
        session: &TemporarySession,
        operation: &'static str,
        method: Method,
        path: &str,
        body: Option<Value>,
    ) -> Result<ProductResponse> {
        // ASVS V12.3 and V13.2: a fixed path and validated origin prevent
        // configuration from redirecting temporary authenticated requests.
        ensure!(
            path.starts_with("/api/") && !path.contains(['\0', '\n', '\r', '#']),
            "Live Demo API path is invalid"
        );
        let url = self
            .request_base
            .join(path)
            .map_err(|_| anyhow::anyhow!("Live Demo API path is invalid"))?;
        let mut request = self
            .client
            .request(method, url)
            .header(header::ACCEPT, "application/json")
            .header(header::ORIGIN, self.origin_header.clone())
            .header(header::HOST, self.host_header.clone())
            .header(header::COOKIE, session.cookie.clone());
        if let Some(body) = body {
            request = request.json(&body);
        }
        let mut response = request
            .send()
            .await
            .map_err(|_| anyhow::anyhow!("Live Demo {operation} request did not complete"))?;
        let status = response.status();
        if !status.is_success() {
            return Ok(ProductResponse { status, body: None });
        }
        let content_type = response
            .headers()
            .get(header::CONTENT_TYPE)
            .and_then(|value| value.to_str().ok());
        ensure!(
            content_type.is_some_and(|value| value.starts_with("application/json")),
            "Live Demo {operation} response is not JSON"
        );
        let mut bytes = Vec::new();
        while let Some(chunk) = response
            .chunk()
            .await
            .map_err(|_| anyhow::anyhow!("Live Demo {operation} response did not complete"))?
        {
            ensure!(
                chunk.len() <= MAX_RESPONSE_BYTES.saturating_sub(bytes.len()),
                "Live Demo {operation} response is too large"
            );
            bytes.extend_from_slice(&chunk);
        }
        let body = serde_json::from_slice(&bytes)
            .map_err(|_| anyhow::anyhow!("Live Demo {operation} response is invalid JSON"))?;
        Ok(ProductResponse {
            status,
            body: Some(body),
        })
    }
}

pub(super) struct ProductResponse {
    pub(super) status: StatusCode,
    pub(super) body: Option<Value>,
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use learning_data_access::SessionTokenHash;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::TcpListener;

    use super::*;
    use crate::installation_data_activity::{BrowserEndpoint, TemporarySession};

    #[tokio::test]
    async fn disposable_local_transport_sends_canonical_browser_headers() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let server = tokio::spawn(async move {
            let (mut stream, _) = tokio::time::timeout(Duration::from_secs(2), listener.accept())
                .await
                .unwrap()
                .unwrap();
            let mut bytes = Vec::new();
            while !bytes.windows(4).any(|window| window == b"\r\n\r\n") {
                let mut chunk = [0_u8; 1024];
                let length = stream.read(&mut chunk).await.unwrap();
                assert_ne!(length, 0, "request ended before its headers");
                bytes.extend_from_slice(&chunk[..length]);
            }
            let request = String::from_utf8(bytes).unwrap();
            assert!(request.starts_with("GET /api/course-instances HTTP/1.1\r\n"));
            assert!(
                request
                    .lines()
                    .any(|line| line.eq_ignore_ascii_case("origin: https://localhost:55001"))
            );
            assert!(
                request
                    .lines()
                    .any(|line| line.eq_ignore_ascii_case("host: localhost:55001"))
            );
            stream
                .write_all(
                    b"HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ncontent-length: 2\r\nconnection: close\r\n\r\n{}",
                )
                .await
                .unwrap();
        });
        let endpoint = BrowserEndpoint {
            // `reqwest` intentionally honors an explicit URL port over a test
            // resolver's port. The endpoint test owns the fixed `api:3000`
            // selection; this wire test owns the special Host override.
            request_base: url::Url::parse("http://api/").unwrap(),
            origin_header: header::HeaderValue::from_static("https://localhost:55001"),
            host_header: header::HeaderValue::from_static("localhost:55001"),
        };
        let client = Client::builder()
            .resolve("api", address)
            .timeout(Duration::from_secs(2))
            .build()
            .unwrap();
        let api = ProductApi::with_client(&endpoint, client);
        let session = TemporarySession {
            cookie: header::HeaderValue::from_static("__Host-ple_session=test"),
            token_hash: SessionTokenHash::compute(b"test session"),
        };
        let response = api
            .request(
                &session,
                "transport test",
                Method::GET,
                "/api/course-instances",
                None,
            )
            .await
            .unwrap();
        assert_eq!(response.status, StatusCode::OK);
        server.await.unwrap();
    }
}

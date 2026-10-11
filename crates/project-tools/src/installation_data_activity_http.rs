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

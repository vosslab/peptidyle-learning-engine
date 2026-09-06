//! Readiness reporting for the container health check.
//!
//! The rule the containers are gated on: `/health` reports ready only when
//! every backing dependency answered a real request. A health endpoint that
//! returns 200 because the process is alive tells an orchestrator nothing, so
//! readiness here is a function of probe results and has no default.

use std::io::{Read, Write};
use std::net::{SocketAddr, TcpStream};
use std::time::Duration;

use axum::{Json, extract::State, http::StatusCode, response::IntoResponse};
use learning_data_access::postgres::Pool;
use serde::Serialize;
use tokio::net::TcpStream as TokioTcpStream;

/// The result of probing one backing dependency.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProbeResult {
    /// Dependency name as it appears in the health body, for example
    /// `postgres` or `object-store`.
    pub name: &'static str,
    /// Whether the dependency answered a real request.
    pub ok: bool,
}

impl ProbeResult {
    /// Records a dependency that answered.
    pub fn ready(name: &'static str) -> Self {
        ProbeResult { name, ok: true }
    }

    /// Records a dependency that did not answer.
    pub fn failed(name: &'static str) -> Self {
        ProbeResult { name, ok: false }
    }
}

/// Whether the process is ready to serve traffic.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Readiness {
    /// Every probe answered.
    Ready,
    /// At least one probe failed; carries the failing dependency names.
    Degraded(Vec<&'static str>),
}

/// Decides readiness from the probe results.
///
/// An empty probe list is `Degraded`, not `Ready`. A process that has not
/// checked anything has not proven anything, and treating "no evidence" as
/// health defeats meaningful container readiness reporting.
///
/// # Examples
///
/// ```
/// use server_core::health::{readiness, ProbeResult, Readiness};
///
/// let probes = vec![ProbeResult::ready("postgres"), ProbeResult::ready("object-store")];
/// assert_eq!(readiness(&probes), Readiness::Ready);
///
/// let probes = vec![ProbeResult::ready("postgres"), ProbeResult::failed("object-store")];
/// assert_eq!(readiness(&probes), Readiness::Degraded(vec!["object-store"]));
/// ```
pub fn readiness(probes: &[ProbeResult]) -> Readiness {
    if probes.is_empty() {
        return Readiness::Degraded(vec!["no-probes-configured"]);
    }
    let failed: Vec<&'static str> = probes
        .iter()
        .filter(|probe| !probe.ok)
        .map(|probe| probe.name)
        .collect();
    if failed.is_empty() {
        Readiness::Ready
    } else {
        Readiness::Degraded(failed)
    }
}

const DISPOSABLE_STORAGE_TOPOLOGY: &str = "disposable-local";
const LOCAL_RENDERER_READINESS_ADDRESS: &str = "webwork-renderer:3000";
const LOCAL_WORKER_READINESS_ADDRESS: &str = "worker:3001";
const DEPENDENCY_PROBE_TIMEOUT: Duration = Duration::from_secs(2);

/// Read-only dependencies checked by the disposable Live Demo health route.
///
/// Production infrastructure owns its separate managed-dependency health
/// contract. The fixed local stack is the only topology that names MinIO, the
/// standalone renderer, and the private worker socket directly.
#[derive(Clone)]
pub struct ReadinessState {
    pool: Pool,
    local_object_store: Option<LocalObjectStoreReadiness>,
}

#[derive(Clone)]
struct LocalObjectStoreReadiness {
    client: objects::minio::S3Client,
    public_assets_bucket: String,
}

impl ReadinessState {
    /// Build the configured readiness checks without retaining raw credentials.
    pub fn from_environment(pool: Pool) -> Result<Self, String> {
        if std::env::var("PLE_STORAGE_TOPOLOGY").ok().as_deref()
            != Some(DISPOSABLE_STORAGE_TOPOLOGY)
        {
            return Ok(Self {
                pool,
                local_object_store: None,
            });
        }
        let settings = objects::minio::EndpointConfig {
            endpoint_url: required_environment("PLE_S3_ENDPOINT")?,
            region: required_environment("PLE_S3_REGION")?,
            access_key_id: required_environment("AWS_ACCESS_KEY_ID")?,
            secret_access_key: required_environment("AWS_SECRET_ACCESS_KEY")?,
        };
        let public_assets_bucket = required_environment("PLE_PUBLIC_ASSETS_BUCKET")?;
        Ok(Self {
            pool,
            local_object_store: Some(LocalObjectStoreReadiness {
                client: objects::minio::client(&settings),
                public_assets_bucket,
            }),
        })
    }

    async fn probe(&self) -> Vec<ProbeResult> {
        let mut results = vec![ProbeResult::ready("api")];
        let database = tokio::time::timeout(DEPENDENCY_PROBE_TIMEOUT, self.pool.acquire()).await;
        results.push(match database {
            Ok(Ok(_connection)) => ProbeResult::ready("database"),
            Ok(Err(_)) | Err(_) => ProbeResult::failed("database"),
        });
        if let Some(object_store) = &self.local_object_store {
            let object_store_probe = tokio::time::timeout(
                DEPENDENCY_PROBE_TIMEOUT,
                objects::minio::probe_bucket(
                    &object_store.client,
                    &object_store.public_assets_bucket,
                ),
            )
            .await;
            results.push(match object_store_probe {
                Ok(Ok(())) => ProbeResult::ready("object-store"),
                Ok(Err(_)) | Err(_) => ProbeResult::failed("object-store"),
            });
            results.push(tcp_probe("renderer", LOCAL_RENDERER_READINESS_ADDRESS).await);
            results.push(tcp_probe("worker", LOCAL_WORKER_READINESS_ADDRESS).await);
        }
        results
    }
}

fn required_environment(name: &str) -> Result<String, String> {
    let value = std::env::var(name).map_err(|_| format!("missing required {name}"))?;
    if value.is_empty() {
        return Err(format!("missing required {name}"));
    }
    Ok(value)
}

async fn tcp_probe(name: &'static str, address: &'static str) -> ProbeResult {
    match tokio::time::timeout(DEPENDENCY_PROBE_TIMEOUT, TokioTcpStream::connect(address)).await {
        Ok(Ok(_connection)) => ProbeResult::ready(name),
        Ok(Err(_)) | Err(_) => ProbeResult::failed(name),
    }
}

/// Browser-safe readiness body with no network, credential, or provider detail.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ReadinessResponse {
    status: &'static str,
    unavailable: Vec<&'static str>,
}

fn readiness_response(probes: &[ProbeResult]) -> (StatusCode, ReadinessResponse) {
    match readiness(probes) {
        Readiness::Ready => (
            StatusCode::OK,
            ReadinessResponse {
                status: "ready",
                unavailable: Vec::new(),
            },
        ),
        Readiness::Degraded(unavailable) => (
            StatusCode::SERVICE_UNAVAILABLE,
            ReadinessResponse {
                status: "degraded",
                unavailable,
            },
        ),
    }
}

/// Return real local dependency readiness rather than process liveness.
// ASVS 14.2.1: the public response names only closed dependency categories;
// connection strings, provider failures, and credentials remain server-side.
pub async fn readiness_handler(State(state): State<ReadinessState>) -> impl IntoResponse {
    let probes = state.probe().await;
    let (status, body) = readiness_response(&probes);
    (status, Json(body))
}

/// Asks a running server for its own readiness over HTTP.
///
/// This backs the container `HEALTHCHECK`. It speaks HTTP/1.1 directly over a
/// socket rather than pulling in an HTTP client, which keeps the runtime image
/// down to one executable with no curl or wget to be borrowed by an attacker.
///
/// Only the status line matters: 200 means every dependency probe answered.
///
/// # Errors
///
/// Returns a message when the socket cannot be opened, the exchange times out,
/// or the response is anything other than 200.
pub fn probe_over_http(addr: SocketAddr, browser_authority: &str) -> Result<(), String> {
    // A health check that can hang is worse than one that fails: the
    // orchestrator would wait instead of restarting.
    let timeout = Duration::from_secs(2);

    let mut stream =
        TcpStream::connect_timeout(&addr, timeout).map_err(|error| format!("connect: {error}"))?;
    stream
        .set_read_timeout(Some(timeout))
        .map_err(|error| format!("read timeout: {error}"))?;
    stream
        .set_write_timeout(Some(timeout))
        .map_err(|error| format!("write timeout: {error}"))?;

    let request = health_request(browser_authority);
    stream
        .write_all(request.as_bytes())
        .map_err(|error| format!("write: {error}"))?;

    let mut response = String::new();
    stream
        .read_to_string(&mut response)
        .map_err(|error| format!("read: {error}"))?;

    let status_line = response.lines().next().unwrap_or_default();
    if status_line.contains(" 200 ") {
        Ok(())
    } else {
        Err(format!("unexpected status: {status_line}"))
    }
}

fn health_request(browser_authority: &str) -> String {
    format!("GET /health HTTP/1.1\r\nHost: {browser_authority}\r\nConnection: close\r\n\r\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_probes_answering_is_ready() {
        let probes = vec![
            ProbeResult::ready("postgres"),
            ProbeResult::ready("object-store"),
        ];
        assert_eq!(readiness(&probes), Readiness::Ready);
    }

    #[test]
    fn one_failing_probe_names_that_dependency() {
        let probes = vec![
            ProbeResult::failed("postgres"),
            ProbeResult::ready("object-store"),
        ];
        assert_eq!(readiness(&probes), Readiness::Degraded(vec!["postgres"]));
    }

    #[test]
    fn no_probes_is_not_ready() {
        assert_eq!(
            readiness(&[]),
            Readiness::Degraded(vec!["no-probes-configured"])
        );
    }

    #[test]
    fn self_probe_uses_the_canonical_browser_authority() {
        assert_eq!(
            health_request("localhost:8443"),
            "GET /health HTTP/1.1\r\nHost: localhost:8443\r\nConnection: close\r\n\r\n"
        );
    }

    #[test]
    fn health_response_is_safe_and_degraded_when_one_dependency_fails() {
        let (status, body) =
            readiness_response(&[ProbeResult::ready("api"), ProbeResult::failed("worker")]);

        assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
        assert_eq!(body.status, "degraded");
        assert_eq!(body.unavailable, ["worker"]);
    }

    #[test]
    fn health_response_reports_every_ready_dependency_without_details() {
        let (status, body) =
            readiness_response(&[ProbeResult::ready("api"), ProbeResult::ready("database")]);

        assert_eq!(status, StatusCode::OK);
        assert_eq!(body.status, "ready");
        assert!(body.unavailable.is_empty());
    }
}

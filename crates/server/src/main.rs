//! MOD-API: entry point for the Peptidyle Learning Engine API server.
//!
//! `main` stays thin on purpose (docs/RUST_STYLE.md section 15): it reads
//! configuration, builds the router, and sends errors to stderr. Everything
//! testable lives in the library half of this crate.
//!
//! Route groups (auth, catalog, course, run, asset) arrive in M2. What exists
//! today is the health surface WP-F4 gates the containers on.
//!
//! Configuration is read once, at startup, from the process environment.
//! Missing configuration does not stop the server from starting: it starts and
//! reports `degraded` naming what is missing, because an operator can curl a
//! running container but cannot curl one that refused to boot.

use std::net::SocketAddr;
use std::sync::Arc;

use axum::Router;
use axum::extract::State;
use axum::http::StatusCode;
use axum::routing::get;

use server_core::health::{ProbeResult, Readiness, readiness};

/// Default bind address. Overridable with `PLE_BIND_ADDR` so a container can
/// move the port without a rebuild.
const DEFAULT_BIND_ADDR: &str = "0.0.0.0:3000";

/// Everything the health path needs, built once at startup.
///
/// Both clients are lazy: constructing them does not contact the service, so a
/// slow-starting PostgreSQL or MinIO delays readiness rather than the process.
struct AppState {
    /// `None` when `DATABASE_URL` is missing or unparseable.
    postgres: Option<store::postgres::Pool>,
    /// `None` when any of the object-store settings are missing.
    objects: Option<ObjectStoreState>,
}

/// The object-store client plus the bucket the health probe checks.
struct ObjectStoreState {
    client: objects::minio::S3Client,
    /// Probing one bucket is enough: the three buckets share an endpoint and
    /// credentials, so a failure in one is a failure in all.
    probe_bucket: String,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let bind_addr: SocketAddr = std::env::var("PLE_BIND_ADDR")
        .unwrap_or_else(|_| DEFAULT_BIND_ADDR.to_string())
        .parse()?;

    // Container health check mode. The same binary probes its own /health so
    // the runtime image needs no curl or wget, which keeps the attack surface
    // to one executable.
    if std::env::args().any(|arg| arg == "--health-probe") {
        // The server binds 0.0.0.0 (every interface), which is not an address
        // a client can connect *to*. Probe the loopback interface on the same
        // port instead.
        let probe_addr = if bind_addr.ip().is_unspecified() {
            SocketAddr::from(([127, 0, 0, 1], bind_addr.port()))
        } else {
            bind_addr
        };
        return match server_core::health::probe_over_http(probe_addr) {
            Ok(()) => Ok(()),
            Err(message) => {
                eprintln!("health probe failed: {message}");
                std::process::exit(1);
            }
        };
    }

    let state = Arc::new(build_state());

    let app = Router::new()
        .route("/health", get(health_handler))
        .with_state(state);

    let listener = tokio::net::TcpListener::bind(bind_addr).await?;
    eprintln!("peptidyle api listening on {bind_addr}");
    axum::serve(listener, app).await?;

    Ok(())
}

/// Reads configuration from the environment and builds the lazy clients.
///
/// Every credential arrives at run time. Nothing here has a default that would
/// let the server connect somewhere unintended when a variable is unset.
fn build_state() -> AppState {
    let postgres = match std::env::var("DATABASE_URL") {
        Ok(url) => match store::postgres::lazy_pool(&url) {
            Ok(pool) => Some(pool),
            Err(error) => {
                eprintln!("DATABASE_URL rejected by the driver: {error}");
                None
            }
        },
        Err(_) => {
            eprintln!("DATABASE_URL unset; /health will report degraded");
            None
        }
    };

    let objects = build_object_store_state();

    AppState { postgres, objects }
}

/// Builds the object-store client when every setting is present.
fn build_object_store_state() -> Option<ObjectStoreState> {
    let endpoint_url = read_required("PLE_S3_ENDPOINT")?;
    let access_key_id = read_required("AWS_ACCESS_KEY_ID")?;
    let secret_access_key = read_required("AWS_SECRET_ACCESS_KEY")?;
    let probe_bucket = read_required("PLE_CONTENT_BUCKET")?;
    // MinIO ignores the region but the SDK requires one, so a default here is
    // safe in a way the credentials above are not.
    let region = std::env::var("PLE_S3_REGION").unwrap_or_else(|_| "us-east-1".to_string());

    let settings = objects::minio::EndpointConfig {
        endpoint_url,
        region,
        access_key_id,
        secret_access_key,
    };

    Some(ObjectStoreState {
        client: objects::minio::client(&settings),
        probe_bucket,
    })
}

/// Reads a required environment variable, reporting the name when it is unset.
fn read_required(name: &str) -> Option<String> {
    match std::env::var(name) {
        Ok(value) if !value.is_empty() => Some(value),
        _ => {
            eprintln!("{name} unset; /health will report degraded");
            None
        }
    }
}

/// Reports readiness for the container health check.
///
/// Returns 200 only when every dependency answered a real request, and 503
/// naming the failing dependencies otherwise. The body is deliberately
/// specific: an operator restarting a container should be able to see which
/// backing service was missing without reading the logs.
async fn health_handler(State(state): State<Arc<AppState>>) -> (StatusCode, String) {
    let probes = probe_dependencies(&state).await;
    match readiness(&probes) {
        Readiness::Ready => (
            StatusCode::OK,
            serde_json::json!({ "status": "ready" }).to_string(),
        ),
        Readiness::Degraded(failing) => (
            StatusCode::SERVICE_UNAVAILABLE,
            serde_json::json!({ "status": "degraded", "failing": failing }).to_string(),
        ),
    }
}

/// Probes every backing dependency, per request.
///
/// Per request, not cached: a health endpoint answering from a startup-time
/// cache reports the past, and an orchestrator acting on it would route
/// traffic to a replica whose database went away ten minutes ago.
async fn probe_dependencies(state: &AppState) -> Vec<ProbeResult> {
    let mut probes = Vec::with_capacity(2);

    probes.push(match &state.postgres {
        None => ProbeResult::failed("postgres-config"),
        Some(pool) => match store::postgres::ping(pool).await {
            Ok(()) => ProbeResult::ready("postgres"),
            Err(error) => {
                eprintln!("postgres probe failed: {error}");
                ProbeResult::failed("postgres")
            }
        },
    });

    probes.push(match &state.objects {
        None => ProbeResult::failed("object-store-config"),
        Some(object_store) => {
            match objects::minio::probe_bucket(&object_store.client, &object_store.probe_bucket)
                .await
            {
                Ok(()) => ProbeResult::ready("object-store"),
                Err(error) => {
                    eprintln!("object store probe failed: {error}");
                    ProbeResult::failed("object-store")
                }
            }
        }
    });

    probes
}

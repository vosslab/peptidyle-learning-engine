//! Production-only API composition (MOD-SRV).
//!
//! This module is the one place that chooses concrete persistent backends.
//! Route modules remain generic so their behavior can be exercised with their
//! focused contract fixtures, while this root makes it impossible for the API
//! binary to acquire process-local educational state by accident.

#[path = "composition/backend.rs"]
mod backend;
#[cfg(feature = "local-development-auth")]
#[path = "composition/local_identity.rs"]
mod local_identity;
#[path = "composition/router.rs"]
mod router;
#[path = "composition/settings.rs"]
mod settings;
#[path = "composition/worker.rs"]
mod worker;

use backend::PersistentDependencies;
#[cfg(feature = "local-development-auth")]
use local_identity::*;
use settings::StorageRuntime;
#[cfg(feature = "local-development-auth")]
pub use worker::{
    run_local_development_invitation_delivery_worker_from_env,
    run_local_development_worker_from_env,
};
pub use worker::{
    run_production_invitation_delivery_worker_from_env, run_production_worker_from_env,
    run_public_asset_publisher_from_env,
};

pub(super) use std::net::SocketAddr;
pub(super) use std::sync::Arc;

pub(super) use anyhow::{Context, Result, bail};
pub(super) use axum::Extension;
pub(super) use axum::http::StatusCode;
pub(super) use axum::response::IntoResponse;
pub(super) use axum::routing::get;
pub(super) use axum::{Json, Router};
#[cfg(feature = "e2e-observability")]
use axum::{
    extract::{Request as AxumRequest, State},
    http::{HeaderName, HeaderValue},
    middleware,
    response::Response,
};
pub(super) use base64::Engine;
pub(super) use base64::engine::general_purpose::URL_SAFE_NO_PAD;
pub(super) use learning_data_access::postgres::{
    Pool, PostgresGraderStore, PostgresStore, ProductionLoginProfile, SchemaCompatibilityError,
    lazy_pool, production_pool,
};
pub(super) use learning_data_access::{
    AssetStore, AuthoritativeTimeStore, CatalogStore, CourseAppearanceStore,
    CourseItemAnalysisStore, CourseRecordsAccessStore, ExportJobStore, FlatImportProvenanceStore,
    FlatQuestionGradingStore, FlatQuestionStore, ManualGradingStore, QtiImportApiStore,
    QtiImportStore, RetentionApiStore, RetentionStore, SessionStore, Store,
};
pub(super) use serde_json::json;

pub(super) use crate::asset::{PublicAssetBaseUrl, PublicAssetUrlResolver};
#[cfg(all(test, feature = "local-development-auth"))]
pub(super) use crate::auth::IdentityProvider;
pub(super) use crate::auth::SessionConfig;
pub(super) use crate::catalog::{BackendRegistry, PublicReviewGate};
pub(super) use crate::composite_backend::CompositeBackend;
pub(super) use crate::health::{ProbeResult, Readiness, readiness};
pub(super) use crate::imathas_backend::{ImathasBackend, LaunchStateAead};
pub(super) use crate::native_backend::NativeBackend;
pub(super) use crate::qti_backend::QtiBackend;
pub(super) use crate::run::{RunBackend, external_tool_router};
pub(super) use crate::webwork_backend::WebworkBackend;
pub(super) use adapter_imathas::broker_provider::{
    ContractedScoredEmbedConfig, ContractedScoredEmbedProvider,
};
pub(super) use adapter_imathas::http_transport::{
    HttpContractedScoredEmbedConfig, HttpContractedScoredEmbedTransport,
};
pub(super) use adapter_imathas::scored_embed::ScoredEmbedProfileConfig;
pub(super) use adapter_imathas::{CorrelationIssuer, ImathasAdapter, SupportedProfile};
pub(super) use adapter_webwork::renderer_contract::RendererIdentity;
pub(super) use adapter_webwork::{HttpWebworkRenderer, HttpWebworkRendererConfig, WebworkAdapter};

/// Builds the actual production application router from explicit startup
/// settings.
///
/// Production enters PLE-owned passwordless account authentication directly.
/// It does not read the local-file identity provider or expose its legacy
/// provider-backed login route. The local launcher remains separately paired
/// with explicit development-only configuration.
pub async fn production_router_from_env() -> Result<Router> {
    let persistent = PersistentDependencies::from_env(StorageRuntime::api_from_env()?).await?;
    persistent.production_router()
}

/// Builds the explicitly opted-in local-development router.
///
/// This is intentionally separate from [`production_router_from_env`] so the
/// production entry point cannot load the file-backed bearer identity scheme.
/// It remains available for local fixture work only and keeps plain-HTTP
/// cookies and public-publication denial coupled to that mode.
#[cfg(feature = "local-development-auth")]
pub async fn local_development_router_from_env() -> Result<Router> {
    let persistent =
        PersistentDependencies::from_local_development_env(StorageRuntime::local_development_api())
            .await?;
    let local_authentication = local_development_authentication_from_env()?;
    Ok(persistent
        .local_development_router(local_authentication, Arc::new(LocalDevelopmentReviewGate)))
}

/// The address the binary should bind, read once at process startup.
pub fn bind_address_from_env() -> Result<SocketAddr> {
    std::env::var("PLE_BIND_ADDR")
        .unwrap_or_else(|_| "0.0.0.0:3000".to_string())
        .parse()
        .context("PLE_BIND_ADDR must be a socket address")
}

#[cfg(test)]
#[path = "composition/tests/storage_topology.rs"]
mod storage_topology_tests;
#[cfg(all(test, feature = "local-development-auth"))]
#[path = "composition/tests/mod.rs"]
mod tests;

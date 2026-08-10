//! Production-only API composition (MOD-SRV).
//!
//! This module is the one place that chooses concrete persistent backends.
//! Route modules remain generic so their behavior can be exercised with their
//! focused contract fixtures, while this root makes it impossible for the API
//! binary to acquire process-local educational state by accident.

#[path = "composition/backend.rs"]
mod backend;
#[path = "composition/local_identity.rs"]
mod local_identity;
#[path = "composition/router.rs"]
mod router;
#[path = "composition/settings.rs"]
mod settings;
#[path = "composition/worker.rs"]
mod worker;

use backend::PersistentDependencies;
use local_identity::*;
pub use worker::run_production_worker_from_env;

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
    Pool, PostgresGraderStore, PostgresStore, SchemaCompatibilityError, lazy_pool,
};
pub(super) use learning_data_access::{
    AssetStore, AuthoritativeTimeStore, CatalogStore, CourseAppearanceStore,
    CourseItemAnalysisStore, CourseRecordsAccessStore, ExportJobStore, FlatImportProvenanceStore,
    FlatQuestionGradingStore, FlatQuestionStore, ManualGradingStore, QtiImportApiStore,
    QtiImportStore, RetentionApiStore, RetentionStore, SessionStore, Store,
};
pub(super) use serde_json::json;

pub(super) use crate::asset::{PublicAssetBaseUrl, PublicAssetUrlResolver};
pub(super) use crate::auth::{IdentityProvider, SessionConfig};
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

/// Builds the actual application router from explicit startup settings.
///
/// An institution OIDC implementation remains required for deployments. The
/// sole startable pre-deployment mode is `PLE_AUTH_PROVIDER=local-file`; it is
/// intentionally fail-closed unless an operator also enables local development
/// and supplies the untracked identity file. It never accepts identities from
/// request headers, query strings, or browser-provided roles.
pub async fn production_router_from_env() -> Result<Router> {
    let persistent = PersistentDependencies::from_env().await?;
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
#[path = "composition/tests/mod.rs"]
mod tests;

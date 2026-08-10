//! Production-only API composition (MOD-SRV).
//!
//! This module is the one place that chooses concrete persistent backends.
//! Route modules remain generic so their behavior can be exercised with their
//! focused contract fixtures, while this root makes it impossible for the API
//! binary to acquire process-local educational state by accident.

#[path = "composition/worker.rs"]
mod worker;

pub use worker::run_production_worker_from_env;

use std::collections::HashSet;
use std::net::SocketAddr;
use std::path::Path;
use std::sync::Arc;

use anyhow::{Context, Result, bail};
use axum::Extension;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::routing::get;
use axum::{Json, Router};
#[cfg(feature = "e2e-observability")]
use axum::{
    extract::{Request as AxumRequest, State},
    http::{HeaderName, HeaderValue},
    middleware,
    response::Response,
};
use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use learning_data_access::postgres::{
    Pool, PostgresGraderStore, PostgresStore, SchemaCompatibilityError, lazy_pool,
};
use learning_data_access::{
    AssetStore, AuthoritativeTimeStore, CatalogStore, CourseAppearanceStore,
    CourseItemAnalysisStore, CourseRecordsAccessStore, ExportJobStore, FlatImportProvenanceStore,
    FlatQuestionGradingStore, FlatQuestionStore, ManualGradingStore, QtiImportApiStore,
    QtiImportStore, RetentionApiStore, RetentionStore, SessionLifetime, SessionStore,
    SessionSubject, Store,
};
use question_model::{TenantId, UserId, UserRole};
use serde_json::json;
use sha2::{Digest, Sha256};
use subtle::ConstantTimeEq;
use uuid::Uuid;

use crate::asset::{PublicAssetBaseUrl, PublicAssetUrlResolver};
use crate::auth::{CookieTransport, IdentityProvider, IdentityProviderError, SessionConfig};
use crate::catalog::{BackendRegistry, PublicReviewGate, ReviewGateError};
use crate::composite_backend::CompositeBackend;
use crate::health::{ProbeResult, Readiness, readiness};
use crate::imathas_backend::{ImathasBackend, LaunchStateAead};
use crate::native_backend::NativeBackend;
use crate::qti_backend::QtiBackend;
use crate::run::{RunBackend, external_tool_router};
use crate::webwork_backend::WebworkBackend;
use adapter_imathas::broker_provider::{
    ContractedScoredEmbedConfig, ContractedScoredEmbedProvider,
};
use adapter_imathas::http_transport::{
    HttpContractedScoredEmbedConfig, HttpContractedScoredEmbedTransport,
};
use adapter_imathas::scored_embed::ScoredEmbedProfileConfig;
use adapter_imathas::{CorrelationIssuer, ImathasAdapter, SupportedProfile};
use adapter_webwork::renderer_contract::RendererIdentity;
use adapter_webwork::{HttpWebworkRenderer, HttpWebworkRendererConfig, WebworkAdapter};

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

fn local_development_session_config() -> SessionConfig {
    SessionConfig::new(
        SessionLifetime::from_seconds(8 * 60 * 60)
            .expect("eight-hour local development session lifetime is positive"),
        CookieTransport::LocalHttp,
    )
}

const LOCAL_CREDENTIAL_BYTES: usize = 32;
const LOCAL_CREDENTIAL_ENCODED_LEN: usize = 43;

/// The private request shape for the sole local-only authentication path.
/// Operator-owned configuration maps its bearer credential to identity; the
/// browser cannot choose tenant, user, display name, or roles.
#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct LocalLoginPresentation {
    credential: String,
}

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct LocalIdentityFile {
    credentials: Vec<LocalIdentityRecord>,
}

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct LocalIdentityRecord {
    credential_sha256: String,
    tenant_id: Uuid,
    user_id: Uuid,
    display_name: String,
    roles: Vec<UserRole>,
}

/// File-backed identity is deliberately binary-private. The only constructor
/// pairs it with [`CookieTransport::LocalHttp`] below, so library consumers
/// cannot accidentally deploy this local bearer scheme over another transport.
struct LocalFileIdentityProvider {
    identities: Vec<LocalFileIdentity>,
}

struct LocalFileIdentity {
    credential_hash: [u8; 32],
    subject: SessionSubject,
}

impl LocalFileIdentityProvider {
    fn from_path(path: impl AsRef<Path>) -> Result<Self, IdentityProviderError> {
        let bytes = std::fs::read(path).map_err(|_| {
            IdentityProviderError::Unavailable(
                "local development identity configuration is unreadable".to_string(),
            )
        })?;
        Self::from_json_bytes(&bytes)
    }

    fn from_json_bytes(bytes: &[u8]) -> Result<Self, IdentityProviderError> {
        let file: LocalIdentityFile = serde_json::from_slice(bytes).map_err(|_| {
            IdentityProviderError::Unavailable(
                "local development identity configuration is invalid".to_string(),
            )
        })?;
        if file.credentials.is_empty() {
            return Err(IdentityProviderError::Unavailable(
                "local development identity configuration is invalid".to_string(),
            ));
        }

        let mut hashes = HashSet::with_capacity(file.credentials.len());
        let mut identities = Vec::with_capacity(file.credentials.len());
        for record in file.credentials {
            if record.tenant_id.is_nil() || record.user_id.is_nil() {
                return Err(IdentityProviderError::Unavailable(
                    "local development identity configuration is invalid".to_string(),
                ));
            }
            let credential_hash =
                decode_lowercase_sha256(&record.credential_sha256).ok_or_else(|| {
                    IdentityProviderError::Unavailable(
                        "local development identity configuration is invalid".to_string(),
                    )
                })?;
            if !hashes.insert(credential_hash) {
                return Err(IdentityProviderError::Unavailable(
                    "local development identity configuration is invalid".to_string(),
                ));
            }
            let subject = SessionSubject::new(
                TenantId::from_uuid(record.tenant_id),
                UserId::from_uuid(record.user_id),
                record.display_name,
                record.roles,
            )
            .map_err(|_| {
                IdentityProviderError::Unavailable(
                    "local development identity configuration is invalid".to_string(),
                )
            })?;
            identities.push(LocalFileIdentity {
                credential_hash,
                subject,
            });
        }
        Ok(Self { identities })
    }
}

#[async_trait::async_trait]
impl IdentityProvider for LocalFileIdentityProvider {
    type Presentation = LocalLoginPresentation;

    async fn verify(
        &self,
        presentation: &Self::Presentation,
    ) -> Result<SessionSubject, IdentityProviderError> {
        let credential = canonical_local_credential(&presentation.credential)
            .ok_or(IdentityProviderError::Rejected)?;
        let presented_hash: [u8; 32] = Sha256::digest(credential).into();
        // Compare every configured identity so the configured record cannot
        // affect lookup timing. Only raw, validated 32-byte bearer material is
        // hashed; the base64url transport spelling is never persisted.
        let mut matched: Option<&SessionSubject> = None;
        for identity in &self.identities {
            if bool::from(identity.credential_hash.ct_eq(&presented_hash)) {
                matched = Some(&identity.subject);
            }
        }
        matched.cloned().ok_or(IdentityProviderError::Rejected)
    }
}

fn canonical_local_credential(value: &str) -> Option<[u8; LOCAL_CREDENTIAL_BYTES]> {
    if value.len() != LOCAL_CREDENTIAL_ENCODED_LEN {
        return None;
    }
    let decoded = URL_SAFE_NO_PAD.decode(value).ok()?;
    let credential: [u8; LOCAL_CREDENTIAL_BYTES] = decoded.try_into().ok()?;
    (URL_SAFE_NO_PAD.encode(credential) == value).then_some(credential)
}

fn decode_lowercase_sha256(value: &str) -> Option<[u8; 32]> {
    if value.len() != 64
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        return None;
    }
    let mut bytes = [0_u8; 32];
    for (index, pair) in value.as_bytes().chunks_exact(2).enumerate() {
        let high = hex_nibble(pair[0])?;
        let low = hex_nibble(pair[1])?;
        bytes[index] = (high << 4) | low;
    }
    Some(bytes)
}

fn hex_nibble(value: u8) -> Option<u8> {
    match value {
        b'0'..=b'9' => Some(value - b'0'),
        b'a'..=b'f' => Some(value - b'a' + 10),
        _ => None,
    }
}

/// An unforgeable local-only pairing of the file-backed bearer provider and
/// the deliberately insecure plain-HTTP cookie policy. It is private to this
/// composition root so production callers cannot accidentally use the local
/// provider with an HTTPS or embedded transport setting.
struct LocalDevelopmentAuthentication {
    provider: Arc<LocalFileIdentityProvider>,
    session_config: SessionConfig,
}

fn local_development_authentication_from_env() -> Result<LocalDevelopmentAuthentication> {
    let provider = required_env("PLE_AUTH_PROVIDER")?;
    let development_flag = required_env("PLE_ENABLE_LOCAL_DEVELOPMENT_AUTH")?;
    let path = required_env("PLE_LOCAL_AUTH_FILE")?;
    local_development_authentication(&provider, &development_flag, &path)
}

fn local_development_authentication(
    provider: &str,
    development_flag: &str,
    path: impl AsRef<std::path::Path>,
) -> Result<LocalDevelopmentAuthentication> {
    if provider != "local-file" {
        bail!(
            "PLE_AUTH_PROVIDER={provider:?} is not available; deployment requires a configured institution OIDC provider"
        );
    }
    if development_flag != "1" {
        bail!(
            "PLE_ENABLE_LOCAL_DEVELOPMENT_AUTH must be exactly 1 when PLE_AUTH_PROVIDER=local-file"
        );
    }
    let provider = LocalFileIdentityProvider::from_path(path).map_err(|_| {
        anyhow::anyhow!("local development identity configuration could not be loaded")
    })?;
    Ok(LocalDevelopmentAuthentication {
        provider: Arc::new(provider),
        session_config: local_development_session_config(),
    })
}

/// Local work must not accidentally publish shared educational content.
#[derive(Debug, Clone, Copy)]
struct LocalDevelopmentReviewGate;

#[async_trait::async_trait]
impl PublicReviewGate for LocalDevelopmentReviewGate {
    async fn allows_publication(
        &self,
        _tenant: learning_data_access::TenantContext,
        _publisher: question_model::UserId,
        _draft: &learning_data_access::DraftRecord,
    ) -> Result<bool, ReviewGateError> {
        Ok(false)
    }
}

/// The address the binary should bind, read once at process startup.
pub fn bind_address_from_env() -> Result<SocketAddr> {
    std::env::var("PLE_BIND_ADDR")
        .unwrap_or_else(|_| "0.0.0.0:3000".to_string())
        .parse()
        .context("PLE_BIND_ADDR must be a socket address")
}

/// Concrete dependency construction for the future institution adapter.
/// It contains only replica-safe backends.
// Until an institution adapter implements `IdentityProvider`, the normal
// binary intentionally fails before it can call `router`. These fields remain
// available for that adapter and the route-level integration boundary.
#[allow(dead_code)]
struct PersistentDependencies {
    store: Arc<PostgresStore>,
    /// This capability is retained only for server-owned grading backends.
    /// It is never included in route state or browser-facing APIs.
    grader: Arc<PostgresGraderStore>,
    objects: Arc<objects::s3::S3ObjectStore>,
    public_assets: Arc<PublicAssetBaseUrl>,
    webwork_renderer: HttpWebworkRenderer,
    imathas: Option<ConfiguredImathas>,
    qti: Option<Arc<ProductionQtiBackend>>,
    health: Arc<HealthState>,
}

type ProductionImathasBackend = ImathasBackend<
    PostgresStore,
    objects::s3::S3ObjectStore,
    ContractedScoredEmbedProvider<HttpContractedScoredEmbedTransport>,
>;
type ProductionQtiBackend =
    QtiBackend<PostgresStore, PostgresGraderStore, objects::s3::S3ObjectStore>;
struct ConfiguredImathas {
    backend: Arc<ProductionImathasBackend>,
    aead: Arc<LaunchStateAead>,
}

const SCHEMA_VERIFICATION_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(2);

async fn verify_application_schema_bounded(pool: &Pool) -> Result<(), SchemaCompatibilityError> {
    match tokio::time::timeout(
        SCHEMA_VERIFICATION_TIMEOUT,
        learning_data_access::postgres::verify_application_schema(pool),
    )
    .await
    {
        Ok(result) => result,
        Err(_) => Err(SchemaCompatibilityError::Unavailable),
    }
}

#[allow(dead_code)]
impl PersistentDependencies {
    async fn from_env() -> Result<Self> {
        let settings = ProductionSettings::from_env()?;
        let dependencies = Self::from_settings(&settings).await?;
        dependencies.verify_startup_schema().await?;
        Ok(dependencies)
    }

    async fn verify_startup_schema(&self) -> Result<()> {
        match verify_application_schema_bounded(&self.health.postgres).await {
            Ok(()) => Ok(()),
            Err(SchemaCompatibilityError::Unavailable) => {
                eprintln!("database schema check unavailable; API starting degraded");
                Ok(())
            }
            Err(SchemaCompatibilityError::Incompatible(reason)) => {
                bail!("database schema is incompatible: {reason}")
            }
        }
    }

    async fn from_settings(settings: &ProductionSettings) -> Result<Self> {
        let LazyStorageDependencies {
            store,
            objects,
            pool,
            object_client,
        } = LazyStorageDependencies::from_settings(&settings.storage)?;
        let public_assets = Arc::new(
            PublicAssetBaseUrl::new(settings.public_asset_base_url.clone())
                .context("PLE_PUBLIC_ASSET_BASE_URL rejected")?,
        );
        // This only validates and constructs a private HTTP client.  It makes
        // no renderer request: renderer availability must not gate API startup,
        // native questions, or the API health endpoint.
        let webwork_renderer = settings.webwork_renderer()?;
        let imathas = settings.imathas(&store, &objects)?;
        let qti_runtime_enabled = settings.qti_runtime_enabled()?;
        let grader_database_url = settings.grader_database_url()?;
        let grader = Arc::new(
            PostgresGraderStore::connect(grader_database_url)
                .await
                .map_err(|_| anyhow::anyhow!("PLE grader connection could not be established"))?,
        );
        let qti = if qti_runtime_enabled {
            Some(Arc::new(QtiBackend::new(
                Arc::clone(&store),
                Arc::clone(&grader),
                Arc::clone(&objects),
            )))
        } else {
            None
        };
        Ok(Self {
            store,
            grader,
            objects,
            public_assets,
            webwork_renderer,
            imathas,
            qti,
            health: Arc::new(HealthState {
                postgres: pool,
                object_client,
                content_bucket: settings.storage.content_bucket.clone(),
            }),
        })
    }

    /// Composes every production route group after trusted institution-owned
    /// dependencies have been supplied.  Neither generic route state nor this
    /// root stores educational records in the process.
    fn router<P, R>(
        &self,
        identity_provider: Arc<P>,
        review_gate: Arc<R>,
        session_config: SessionConfig,
    ) -> Router
    where
        P: IdentityProvider + 'static,
        P::Presentation: serde::de::DeserializeOwned + Send + Sync + 'static,
        R: PublicReviewGate + 'static,
    {
        let native_adapter = Arc::new(adapter_native::NativeAdapter::new());
        let flat_grader: Arc<dyn FlatQuestionGradingStore> = self.grader.clone();
        let native = NativeBackend::with_flat_grader(
            Arc::clone(&native_adapter),
            Arc::clone(&self.store),
            flat_grader,
        );
        let webwork_adapter = Arc::new(WebworkAdapter::new(
            self.objects.as_ref().clone(),
            self.webwork_renderer.clone(),
        ));
        let webwork = WebworkBackend::new(
            Arc::clone(&self.store),
            Arc::clone(&self.objects),
            webwork_adapter,
        );
        let mut backends = CompositeBackend::new(native, webwork);
        if let Some(imathas) = &self.imathas {
            backends = backends.with_imathas(imathas.backend.clone());
        }
        if let Some(qti) = &self.qti {
            backends = backends.with_qti(qti.clone());
        }
        let backends = Arc::new(backends);
        let mut router = compose_router(
            Arc::clone(&self.store),
            Arc::clone(&self.objects),
            Arc::clone(&self.public_assets),
            Arc::clone(&backends),
            native_adapter,
            identity_provider,
            review_gate,
            session_config,
            Arc::clone(&self.health),
        );
        if let Some(imathas) = &self.imathas {
            router = router.merge(external_tool_router(
                Arc::clone(&self.store),
                Arc::clone(&backends),
                Arc::clone(&imathas.aead),
            ));
        }
        router
    }

    fn local_development_router<R>(
        &self,
        local_authentication: LocalDevelopmentAuthentication,
        review_gate: Arc<R>,
    ) -> Router
    where
        R: PublicReviewGate + 'static,
    {
        self.router(
            local_authentication.provider,
            review_gate,
            local_authentication.session_config,
        )
    }
}

/// Merges every ready API route group.  Keeping this generic makes the exact
/// production concrete types visible above and prevents a route from quietly
/// acquiring a different state store.
#[allow(clippy::too_many_arguments)]
#[allow(dead_code)]
fn compose_router<S, O, C, B, P, R>(
    store: Arc<S>,
    objects: Arc<O>,
    public_assets: Arc<C>,
    backends: Arc<B>,
    native_adapter: Arc<adapter_native::NativeAdapter>,
    identity_provider: Arc<P>,
    review_gate: Arc<R>,
    session_config: SessionConfig,
    health: Arc<HealthState>,
) -> Router
where
    S: Store
        + CatalogStore
        + FlatQuestionStore
        + FlatImportProvenanceStore
        + CourseItemAnalysisStore
        + ExportJobStore
        + ManualGradingStore
        + QtiImportApiStore
        + QtiImportStore
        + RetentionStore
        + RetentionApiStore
        + CourseRecordsAccessStore
        + SessionStore
        + AssetStore
        + CourseAppearanceStore
        + AuthoritativeTimeStore
        + 'static,
    O: objects::ObjectStore + 'static,
    C: PublicAssetUrlResolver + 'static,
    B: BackendRegistry + RunBackend + 'static,
    P: IdentityProvider + 'static,
    P::Presentation: serde::de::DeserializeOwned + Send + Sync + 'static,
    R: PublicReviewGate + 'static,
{
    let router = Router::new()
        .route("/health", get(health_handler))
        .merge(crate::auth::router(
            identity_provider,
            Arc::clone(&store),
            session_config,
        ))
        .merge(crate::catalog::router(
            Arc::clone(&store),
            Arc::clone(&backends),
            Arc::clone(&review_gate),
        ))
        .merge(crate::qti_publication::router(
            Arc::clone(&store),
            Arc::clone(&objects),
            Arc::clone(&backends),
            Arc::clone(&review_gate),
        ))
        .merge(crate::flat_question_publication::router(
            Arc::clone(&store),
            Arc::clone(&objects),
            Arc::clone(&backends),
            Arc::clone(&review_gate),
        ))
        .merge(crate::qti_profile_import::router(
            Arc::clone(&store),
            Arc::clone(&objects),
        ))
        .merge(crate::qti_profile_conversion::router(
            Arc::clone(&store),
            Arc::clone(&objects),
        ))
        .merge(crate::workspace::router(
            Arc::clone(&store),
            Arc::clone(&backends),
        ))
        .merge(crate::author_preview::router(
            Arc::clone(&store),
            native_adapter,
        ))
        .merge(crate::course::router(Arc::clone(&store)))
        .merge(crate::course_appearance::router(
            Arc::clone(&store),
            Arc::clone(&objects),
        ))
        .merge(crate::item_analysis::router(Arc::clone(&store)))
        .merge(crate::export::router(Arc::clone(&store)))
        .merge(crate::retention::router(Arc::clone(&store)))
        .merge(crate::run::router(Arc::clone(&store), backends))
        .merge(crate::asset::router(store.clone(), objects, public_assets))
        .merge(crate::validation::router(store))
        .layer(Extension(health));

    apply_e2e_replica_attribution(router, e2e_replica_attribution_from_env())
}

/// Opaque container identity carried only by the test-only replica E2E build.
/// It deliberately has no access to the request, route state, tenant, object
/// store, or any other process configuration.
#[cfg(feature = "e2e-observability")]
#[derive(Clone)]
struct ReplicaAttribution(HeaderValue);

#[cfg(not(feature = "e2e-observability"))]
type ReplicaAttribution = ();

#[cfg(feature = "e2e-observability")]
const E2E_REPLICA_HEADER: HeaderName = HeaderName::from_static("x-ple-e2e-replica");
#[cfg(feature = "e2e-observability")]
const E2E_REPLICA_PREFIX: &str = "ple-replica-e2e-api-";
#[cfg(feature = "e2e-observability")]
const E2E_REPLICA_SUFFIX_LEN: usize = 12;

fn apply_e2e_replica_attribution(
    router: Router,
    attribution: Option<ReplicaAttribution>,
) -> Router {
    #[cfg(feature = "e2e-observability")]
    {
        if let Some(attribution) = attribution {
            return router.layer(middleware::from_fn_with_state(
                attribution,
                attach_e2e_replica_attribution,
            ));
        }
        router
    }

    #[cfg(not(feature = "e2e-observability"))]
    {
        // Keep the feature-off build structurally unable to attach the header,
        // even if an operator happens to set its test environment variables.
        let _ = attribution;
        router
    }
}

#[cfg(feature = "e2e-observability")]
async fn attach_e2e_replica_attribution(
    State(ReplicaAttribution(value)): State<ReplicaAttribution>,
    request: AxumRequest,
    next: middleware::Next,
) -> Response {
    let mut response = next.run(request).await;
    response.headers_mut().insert(E2E_REPLICA_HEADER, value);
    response
}

#[cfg(feature = "e2e-observability")]
fn e2e_replica_attribution_from_env() -> Option<ReplicaAttribution> {
    e2e_replica_attribution_from_values(
        std::env::var("PLE_ENABLE_E2E_OBSERVABILITY")
            .ok()
            .as_deref(),
        std::env::var("HOSTNAME").ok().as_deref(),
    )
}

#[cfg(not(feature = "e2e-observability"))]
fn e2e_replica_attribution_from_env() -> Option<ReplicaAttribution> {
    // Do not even inspect test-only environment variables in normal builds.
    None
}

#[cfg(any(feature = "e2e-observability", test))]
fn e2e_replica_attribution_from_values(
    enabled: Option<&str>,
    hostname: Option<&str>,
) -> Option<ReplicaAttribution> {
    #[cfg(feature = "e2e-observability")]
    {
        if enabled != Some("1") {
            return None;
        }
        let hostname = hostname?;
        validated_e2e_hostname(hostname).map(ReplicaAttribution)
    }

    #[cfg(not(feature = "e2e-observability"))]
    {
        let _ = (enabled, hostname);
        None
    }
}

#[cfg(feature = "e2e-observability")]
fn validated_e2e_hostname(hostname: &str) -> Option<HeaderValue> {
    // Do not reflect a generic hostname: it is an arbitrary environment
    // value and could accidentally contain an operator-provided secret.  The
    // E2E runner maps this fixed test-project prefix plus the container's
    // normal 12-character short ID to `podman inspect`.  Compose can also
    // provide the already-normalized form without changing the result.
    let suffix = hostname
        .strip_prefix(E2E_REPLICA_PREFIX)
        .unwrap_or(hostname);
    if suffix.len() != E2E_REPLICA_SUFFIX_LEN
        || !suffix
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        return None;
    }
    HeaderValue::from_str(&format!("{E2E_REPLICA_PREFIX}{suffix}")).ok()
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
struct HealthState {
    postgres: Pool,
    object_client: objects::minio::S3Client,
    content_bucket: String,
}

fn postgres_schema_probe(verification: Result<(), SchemaCompatibilityError>) -> ProbeResult {
    if verification.is_ok() {
        ProbeResult::ready("postgres")
    } else {
        ProbeResult::failed("postgres")
    }
}

#[allow(dead_code)]
async fn health_handler(Extension(state): Extension<Arc<HealthState>>) -> impl IntoResponse {
    // Re-run the exact check on every request so a failover or replacement
    // cannot inherit readiness from a previously compatible database.
    let postgres = postgres_schema_probe(verify_application_schema_bounded(&state.postgres).await);
    if !postgres.ok {
        eprintln!("postgres schema readiness probe failed");
    }
    let objects =
        match objects::minio::probe_bucket(&state.object_client, &state.content_bucket).await {
            Ok(()) => ProbeResult::ready("object-store"),
            Err(error) => {
                eprintln!("object store probe failed: {error}");
                ProbeResult::failed("object-store")
            }
        };
    match readiness(&[postgres, objects]) {
        Readiness::Ready => (StatusCode::OK, Json(json!({ "status": "ready" }))).into_response(),
        Readiness::Degraded(failing) => (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(json!({ "status": "degraded", "failing": failing })),
        )
            .into_response(),
    }
}

struct StorageSettings {
    database_url: String,
    s3_endpoint: String,
    s3_region: String,
    access_key_id: String,
    secret_access_key: String,
    content_bucket: String,
    student_records_bucket: String,
    temp_processing_bucket: String,
}

impl StorageSettings {
    fn from_env() -> Result<Self> {
        Ok(Self {
            database_url: required_env("DATABASE_URL")?,
            s3_endpoint: required_env("PLE_S3_ENDPOINT")?,
            s3_region: required_env("PLE_S3_REGION")?,
            access_key_id: required_env("AWS_ACCESS_KEY_ID")?,
            secret_access_key: required_env("AWS_SECRET_ACCESS_KEY")?,
            content_bucket: required_env("PLE_CONTENT_BUCKET")?,
            student_records_bucket: required_env("PLE_STUDENT_RECORDS_BUCKET")?,
            temp_processing_bucket: required_env("PLE_TEMP_PROCESSING_BUCKET")?,
        })
    }
}

struct LazyStorageDependencies {
    store: Arc<PostgresStore>,
    objects: Arc<objects::s3::S3ObjectStore>,
    pool: Pool,
    object_client: objects::minio::S3Client,
}

impl LazyStorageDependencies {
    fn from_settings(settings: &StorageSettings) -> Result<Self> {
        let pool = lazy_pool(&settings.database_url)
            .context("DATABASE_URL rejected by PostgreSQL driver")?;
        let store = Arc::new(PostgresStore::new(pool.clone()));
        let object_client = objects::minio::client(&objects::minio::EndpointConfig {
            endpoint_url: settings.s3_endpoint.clone(),
            region: settings.s3_region.clone(),
            access_key_id: settings.access_key_id.clone(),
            secret_access_key: settings.secret_access_key.clone(),
        });
        let objects = Arc::new(objects::s3::S3ObjectStore::new(
            object_client.clone(),
            objects::s3::BucketNames {
                content: settings.content_bucket.clone(),
                student_records: settings.student_records_bucket.clone(),
                temp_processing: settings.temp_processing_bucket.clone(),
            },
        ));
        Ok(Self {
            store,
            objects,
            pool,
            object_client,
        })
    }
}

struct ProductionSettings {
    storage: StorageSettings,
    public_asset_base_url: String,
    webwork_renderer_base_url: String,
    webwork_request_timeout_seconds: u64,
    webwork_max_response_bytes: usize,
    webwork_renderer_id: String,
    webwork_renderer_version: String,
    webwork_course_id: String,
    webwork_user: String,
    webwork_password_file: String,
    imathas_provider_key: Option<String>,
    qti_runtime_enabled: Option<String>,
    grader_database_url: Option<String>,
}

impl ProductionSettings {
    fn from_env() -> Result<Self> {
        Ok(Self {
            storage: StorageSettings::from_env()?,
            public_asset_base_url: required_env("PLE_PUBLIC_ASSET_BASE_URL")?,
            webwork_renderer_base_url: required_env("PLE_WEBWORK_RENDERER_BASE_URL")?,
            webwork_request_timeout_seconds: positive_u64_env(
                "PLE_WEBWORK_REQUEST_TIMEOUT_SECONDS",
            )?,
            webwork_max_response_bytes: positive_usize_env("PLE_WEBWORK_MAX_RESPONSE_BYTES")?,
            webwork_renderer_id: required_env("PLE_WEBWORK_RENDERER_ID")?,
            webwork_renderer_version: required_env("PLE_WEBWORK_RENDERER_VERSION")?,
            webwork_course_id: required_env("PLE_WEBWORK_RENDER_COURSE_ID")?,
            webwork_user: required_env("PLE_WEBWORK_RENDER_USER")?,
            webwork_password_file: required_env("PLE_WEBWORK_RENDER_PASSWORD_FILE")?,
            imathas_provider_key: std::env::var("PLE_IMATHAS_PROVIDER_KEY").ok(),
            qti_runtime_enabled: std::env::var("PLE_QTI_RUNTIME_ENABLED").ok(),
            grader_database_url: std::env::var("PLE_GRADER_DATABASE_URL").ok(),
        })
    }

    fn webwork_renderer(&self) -> Result<HttpWebworkRenderer> {
        // A renderer base URI is an endpoint, never a credential carrier.
        // Keeping query and fragment components out also ensures the adapter's
        // redacted Debug implementation cannot accidentally reveal a token.
        if self.webwork_renderer_base_url.contains(['?', '#']) {
            bail!("PLE_WEBWORK_RENDERER_BASE_URL must not contain a query or fragment");
        }
        let settings = HttpWebworkRendererConfig::new(
            &self.webwork_renderer_base_url,
            std::time::Duration::from_secs(self.webwork_request_timeout_seconds),
            self.webwork_max_response_bytes,
            RendererIdentity {
                id: self.webwork_renderer_id.clone(),
                version: self.webwork_renderer_version.clone(),
            },
            &self.webwork_course_id,
            &self.webwork_user,
            &read_webwork_password_file(&self.webwork_password_file)?,
        )
        .context("PLE_WEBWORK renderer configuration is invalid")?;
        HttpWebworkRenderer::new(settings).context("PLE_WEBWORK renderer configuration is invalid")
    }

    fn imathas(
        &self,
        store: &Arc<PostgresStore>,
        objects: &Arc<objects::s3::S3ObjectStore>,
    ) -> Result<Option<ConfiguredImathas>> {
        let Some(provider_key) = &self.imathas_provider_key else {
            return Ok(None);
        };
        if provider_key.trim().is_empty() {
            bail!("PLE_IMATHAS_PROVIDER_KEY must not be empty");
        }
        let base = required_env("PLE_IMATHAS_BASE_URL")?;
        let timeout = positive_u64_env("PLE_IMATHAS_REQUEST_TIMEOUT_SECONDS")?;
        let transport_bytes = positive_usize_env("PLE_IMATHAS_MAX_TRANSPORT_BYTES")?;
        let snapshot_bytes = positive_usize_env("PLE_IMATHAS_MAX_SNAPSHOT_BYTES")?;
        let result_bytes = positive_usize_env("PLE_IMATHAS_MAX_RESULT_BYTES")?;
        let ttl = positive_u64_env("PLE_IMATHAS_LAUNCH_TTL_MILLIS")?;
        let launch_state = decode_secret32("PLE_IMATHAS_LAUNCH_STATE_SECRET")?;
        let correlation_secret = decode_secret32("PLE_IMATHAS_CORRELATION_SECRET")?;
        let launch_signing = required_env("PLE_IMATHAS_LAUNCH_SIGNING_SECRET")?;
        let result_verify = required_env("PLE_IMATHAS_RESULT_VERIFICATION_SECRET")?;
        let auth_name = std::env::var("PLE_IMATHAS_PROVIDER_AUTH_HEADER_NAME").ok();
        let auth_value = std::env::var("PLE_IMATHAS_PROVIDER_AUTH_VALUE").ok();
        let auth = match (auth_name, auth_value) {
            (None, None) => None,
            (Some(name), Some(value)) if name == "x-ple-provider-auth" && !value.is_empty() => {
                Some(value)
            }
            _ => bail!(
                "PLE_IMATHAS provider authentication requires exact x-ple-provider-auth name and non-empty value"
            ),
        };
        let profile =
            ScoredEmbedProfileConfig::contracted_self_hosted(provider_key.clone(), true, true)
                .map_err(|_| anyhow::anyhow!("PLE_IMATHAS contracted profile is invalid"))?;
        let provider_config = ContractedScoredEmbedConfig::new(
            profile,
            launch_signing.as_bytes(),
            result_verify.as_bytes(),
            ttl,
        )
        .map_err(|_| anyhow::anyhow!("PLE_IMATHAS contracted profile is invalid"))?
        .with_limits(snapshot_bytes, result_bytes)
        .map_err(|_| anyhow::anyhow!("PLE_IMATHAS limits are invalid"))?;
        let transport_config = HttpContractedScoredEmbedConfig::https(
            &base,
            std::time::Duration::from_secs(timeout),
            transport_bytes,
        )
        .map_err(|_| anyhow::anyhow!("PLE_IMATHAS transport configuration is invalid"))?;
        let transport_config = match auth {
            Some(value) => transport_config
                .with_private_auth(&value)
                .map_err(|_| anyhow::anyhow!("PLE_IMATHAS transport configuration is invalid"))?,
            None => transport_config,
        };
        let transport = HttpContractedScoredEmbedTransport::new(transport_config)
            .map_err(|_| anyhow::anyhow!("PLE_IMATHAS transport configuration is invalid"))?;
        let provider = ContractedScoredEmbedProvider::new(provider_config, transport);
        let adapter = Arc::new(ImathasAdapter::new(
            objects.as_ref().clone(),
            provider,
            [SupportedProfile::new(
                adapter_imathas::scored_embed::SCORED_EMBED_BROKER_PROFILE_ID,
                true,
                true,
                true,
            )
            .expect("fixed profile")],
        ));
        let backend = Arc::new(ImathasBackend::new(
            Arc::clone(store),
            Arc::clone(objects),
            adapter,
            Arc::new(CorrelationIssuer::from_server_secret(correlation_secret)),
        ));
        Ok(Some(ConfiguredImathas {
            backend,
            aead: Arc::new(
                LaunchStateAead::from_server_secret(launch_state)
                    .map_err(|_| anyhow::anyhow!("PLE_IMATHAS launch state secret is invalid"))?,
            ),
        }))
    }

    /// Flat native questions are registered in every production adapter, so
    /// their separately authenticated grader connection is mandatory even
    /// when the optional QTI runtime is disabled.
    fn grader_database_url(&self) -> Result<&str> {
        let database_url = self
            .grader_database_url
            .as_deref()
            .filter(|value| !value.trim().is_empty())
            .ok_or_else(|| anyhow::anyhow!("PLE_GRADER_DATABASE_URL must be set"))?;
        if !database_url.starts_with("postgres://") && !database_url.starts_with("postgresql://") {
            bail!("PLE_GRADER_DATABASE_URL must be a PostgreSQL connection URL");
        }
        Ok(database_url)
    }

    /// QTI remains explicitly opt-in; it shares the already-required native
    /// flat-question grader connection rather than creating another pool.
    fn qti_runtime_enabled(&self) -> Result<bool> {
        match self.qti_runtime_enabled.as_deref() {
            None => Ok(false),
            Some("1") => Ok(true),
            Some(_) => bail!("PLE_QTI_RUNTIME_ENABLED must be exactly 1 when set"),
        }
    }
}

fn decode_secret32(name: &str) -> Result<[u8; 32]> {
    let value = required_env(name)?;
    parse_secret32(name, &value)
}

fn parse_secret32(name: &str, value: &str) -> Result<[u8; 32]> {
    let bytes = URL_SAFE_NO_PAD
        .decode(value.as_bytes())
        .with_context(|| format!("{name} must be canonical base64url"))?;
    if bytes.len() != 32 || URL_SAFE_NO_PAD.encode(&bytes) != value {
        bail!("{name} must be canonical 32-byte base64url");
    }
    Ok(bytes.try_into().expect("checked length"))
}

fn required_env(name: &str) -> Result<String> {
    let value = std::env::var(name).with_context(|| format!("{name} must be set"))?;
    if value.trim().is_empty() {
        bail!("{name} must not be empty");
    }
    Ok(value)
}

fn read_webwork_password_file(path: &str) -> Result<String> {
    #[cfg(unix)]
    {
        read_webwork_password_file_unix(path)
    }
    #[cfg(not(unix))]
    read_webwork_password_file_portable(path)
}

#[cfg(unix)]
fn read_webwork_password_file_unix(path: &str) -> Result<String> {
    use std::io::Read as _;
    use std::os::unix::fs::{OpenOptionsExt as _, PermissionsExt as _};

    const MAX_SECRET_BYTES: u64 = 4096;
    // O_NOFOLLOW makes the open itself reject a symlink, closing the race
    // between metadata inspection and reading the mounted secret.
    let mut file = std::fs::OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_NOFOLLOW)
        .open(path)
        .with_context(|| "PLE_WEBWORK_RENDER_PASSWORD_FILE could not be inspected")?;
    let metadata = file
        .metadata()
        .with_context(|| "PLE_WEBWORK_RENDER_PASSWORD_FILE could not be inspected")?;
    if !metadata.is_file() || metadata.len() > MAX_SECRET_BYTES {
        bail!("PLE_WEBWORK_RENDER_PASSWORD_FILE must name a non-empty bounded regular file");
    }
    if metadata.permissions().mode() & 0o777 != 0o600 {
        bail!("PLE_WEBWORK_RENDER_PASSWORD_FILE must have Unix mode 0600");
    }
    let mut password = String::new();
    file.read_to_string(&mut password)
        .with_context(|| "PLE_WEBWORK_RENDER_PASSWORD_FILE could not be read")?;
    normalize_webwork_password(password)
}

#[cfg(not(unix))]
fn read_webwork_password_file_portable(path: &str) -> Result<String> {
    const MAX_SECRET_BYTES: u64 = 4096;
    // Non-Unix platforms lack a portable O_NOFOLLOW equivalent in std.  They
    // still reject a visible link and every non-regular or oversized target.
    let metadata = std::fs::symlink_metadata(path)
        .with_context(|| "PLE_WEBWORK_RENDER_PASSWORD_FILE could not be inspected")?;
    if metadata.file_type().is_symlink() || !metadata.is_file() || metadata.len() > MAX_SECRET_BYTES
    {
        bail!("PLE_WEBWORK_RENDER_PASSWORD_FILE must name a non-empty bounded regular file");
    }
    let password = std::fs::read_to_string(path)
        .with_context(|| "PLE_WEBWORK_RENDER_PASSWORD_FILE could not be read")?;
    normalize_webwork_password(password)
}

fn normalize_webwork_password(password: String) -> Result<String> {
    let password = password.trim_end_matches(['\r', '\n']).to_string();
    if password.trim().is_empty() {
        bail!("PLE_WEBWORK_RENDER_PASSWORD_FILE must not be empty");
    }
    Ok(password)
}

fn positive_u64_env(name: &str) -> Result<u64> {
    let value = required_env(name)?;
    parse_positive_u64(name, &value)
}

fn positive_usize_env(name: &str) -> Result<usize> {
    let value = required_env(name)?;
    parse_positive_usize(name, &value)
}

fn parse_positive_u64(name: &str, value: &str) -> Result<u64> {
    let parsed = value
        .parse::<u64>()
        .with_context(|| format!("{name} must be a positive whole number"))?;
    if parsed == 0 {
        bail!("{name} must be a positive whole number");
    }
    Ok(parsed)
}

fn parse_positive_usize(name: &str, value: &str) -> Result<usize> {
    let parsed = value
        .parse::<usize>()
        .with_context(|| format!("{name} must be a positive whole number"))?;
    if parsed == 0 {
        bail!("{name} must be a positive whole number");
    }
    Ok(parsed)
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use async_trait::async_trait;
    use axum::body::Body;
    use axum::http::{HeaderValue, Request, StatusCode};
    use learning_data_access::in_memory::MemoryStore;
    use learning_data_access::{SessionLifetime, SessionSubject};
    use objects::memory::MemoryObjectStore;
    use question_model::UserId;
    use tower::ServiceExt;

    use super::*;
    use crate::auth::{CookieTransport, IdentityProviderError};
    use crate::catalog::ReviewGateError;

    #[derive(Debug)]
    struct TestIdentity;

    #[async_trait]
    impl IdentityProvider for TestIdentity {
        type Presentation = serde_json::Value;

        async fn verify(
            &self,
            _presentation: &Self::Presentation,
        ) -> Result<SessionSubject, IdentityProviderError> {
            Err(IdentityProviderError::Unavailable(
                "test identity is unavailable".to_string(),
            ))
        }
    }

    #[derive(Debug)]
    struct TestReview;

    #[async_trait]
    impl PublicReviewGate for TestReview {
        async fn allows_publication(
            &self,
            _tenant: learning_data_access::TenantContext,
            _publisher: UserId,
            _draft: &learning_data_access::DraftRecord,
        ) -> Result<bool, ReviewGateError> {
            Err(ReviewGateError("test review is unavailable".to_string()))
        }
    }

    fn session_config() -> SessionConfig {
        SessionConfig::new(
            SessionLifetime::from_seconds(3_600).expect("positive lifetime"),
            CookieTransport::LocalHttp,
        )
    }

    #[test]
    fn postgres_readiness_requires_exact_schema_compatibility() {
        assert_eq!(
            postgres_schema_probe(Ok(())),
            ProbeResult::ready("postgres")
        );
        assert_eq!(
            postgres_schema_probe(Err(SchemaCompatibilityError::Unavailable)),
            ProbeResult::failed("postgres")
        );
        assert_eq!(
            postgres_schema_probe(Err(SchemaCompatibilityError::Incompatible(
                "test-only private detail".to_string(),
            ))),
            ProbeResult::failed("postgres")
        );
    }

    fn composed_memory_router() -> Router {
        let (store, grader) = MemoryStore::with_flat_question_grader();
        let store = Arc::new(store);
        let objects = Arc::new(MemoryObjectStore::default());
        let native_adapter = Arc::new(adapter_native::NativeAdapter::new());
        let native = NativeBackend::with_flat_grader(
            Arc::clone(&native_adapter),
            Arc::clone(&store),
            Arc::new(grader),
        );
        let renderer = HttpWebworkRenderer::new(
            HttpWebworkRendererConfig::new(
                "http://renderer.internal/webwork2/",
                std::time::Duration::from_secs(1),
                1_024,
                RendererIdentity {
                    id: "test-renderer".to_string(),
                    version: "1".to_string(),
                },
                "test-course",
                "test-user",
                "test-password",
            )
            .expect("valid test renderer configuration"),
        )
        .expect("valid test renderer");
        let webwork = WebworkBackend::new(
            Arc::clone(&store),
            Arc::clone(&objects),
            Arc::new(WebworkAdapter::new(objects.as_ref().clone(), renderer)),
        );
        let backends = Arc::new(CompositeBackend::new(native, webwork));
        let public_assets = Arc::new(
            PublicAssetBaseUrl::new("https://cdn.example.test/content")
                .expect("valid public asset base"),
        );
        let health = Arc::new(HealthState {
            postgres: lazy_pool("postgres://user:password@127.0.0.1:1/ple")
                .expect("valid lazy postgres pool"),
            object_client: objects::minio::client(&objects::minio::EndpointConfig {
                endpoint_url: "http://127.0.0.1:1".to_string(),
                region: "us-east-1".to_string(),
                access_key_id: "test-access".to_string(),
                secret_access_key: "test-secret".to_string(),
            }),
            content_bucket: "content".to_string(),
        });
        compose_router(
            store,
            objects,
            public_assets,
            backends,
            native_adapter,
            Arc::new(TestIdentity),
            Arc::new(TestReview),
            session_config(),
            health,
        )
    }

    #[tokio::test]
    async fn composition_mounts_private_qti_profile_routes() {
        let app = composed_memory_router();
        for request in [
            Request::builder()
                .method("GET")
                .uri("/api/workspaces/00000000-0000-0000-0000-000000000001/qti-imports/00000000-0000-0000-0000-000000000002")
                .body(Body::empty())
                .expect("QTI report request"),
            Request::builder()
                .method("POST")
                .uri("/api/workspaces/00000000-0000-0000-0000-000000000001/qti-imports/00000000-0000-0000-0000-000000000002/items/item-1/convert-flat")
                .body(Body::empty())
                .expect("QTI conversion request"),
        ] {
            let response = app
                .clone()
                .oneshot(request)
                .await
                .expect("QTI route response");
            assert!(!response.status().is_success());
            assert_eq!(response.headers().get("cache-control"), Some(&HeaderValue::from_static("no-store")));
        }
    }

    fn local_provider() -> LocalFileIdentityProvider {
        LocalFileIdentityProvider::from_json_bytes(
            br#"{
                "credentials": [{
                    "credential_sha256": "630dcd2966c4336691125448bbb25b4ff412a49c732db2c8abc1b8581bd710dd",
                    "tenant_id": "00000000-0000-0000-0000-000000000001",
                    "user_id": "00000000-0000-0000-0000-000000000002",
                    "display_name": "Local Student",
                    "roles": ["student"]
                }]
            }"#,
        )
        .expect("valid local provider fixture")
    }

    async fn replica_attribution_header(
        enabled: Option<&str>,
        hostname: Option<&str>,
    ) -> Option<HeaderValue> {
        let app = apply_e2e_replica_attribution(
            Router::new().route("/ordinary", get(|| async { StatusCode::NO_CONTENT })),
            e2e_replica_attribution_from_values(enabled, hostname),
        );
        app.oneshot(
            Request::builder()
                .uri("/ordinary")
                .body(Body::empty())
                .expect("ordinary request"),
        )
        .await
        .expect("ordinary response")
        .headers()
        .get("x-ple-e2e-replica")
        .cloned()
    }

    #[tokio::test]
    async fn default_build_never_emits_replica_attribution() {
        let header = replica_attribution_header(Some("1"), Some("0123456789ab")).await;
        #[cfg(not(feature = "e2e-observability"))]
        assert!(header.is_none());
        #[cfg(feature = "e2e-observability")]
        assert_eq!(
            header,
            Some(HeaderValue::from_static("ple-replica-e2e-api-0123456789ab"))
        );
    }

    #[tokio::test]
    async fn feature_build_without_runtime_toggle_emits_no_replica_attribution() {
        let header = replica_attribution_header(None, Some("0123456789ab")).await;
        assert!(header.is_none());
    }

    #[cfg(feature = "e2e-observability")]
    #[tokio::test]
    async fn feature_build_emits_only_a_safe_compose_replica_identity() {
        assert_eq!(
            replica_attribution_header(Some("1"), Some("0123456789ab")).await,
            Some(HeaderValue::from_static("ple-replica-e2e-api-0123456789ab"))
        );
        assert_eq!(
            replica_attribution_header(Some("1"), Some("ple-replica-e2e-api-0123456789ab")).await,
            Some(HeaderValue::from_static("ple-replica-e2e-api-0123456789ab"))
        );
        for invalid in [
            "api-1.example.test",
            "sk-proj-0123456789abcdefghijklmnopqrstuv",
            "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef",
            "YWJjZGVmZ2hpamts",
            "ple-replica-e2e-api-sk-proj-0123456789",
            "ple-replica-e2e-api-0123456789AB",
            "ple-replica-e2e-api-0123456789a",
            "ple-replica-e2e-api-0123456789abc",
        ] {
            assert!(
                replica_attribution_header(Some("1"), Some(invalid))
                    .await
                    .is_none(),
                "invalid hostname {invalid:?} must not be exposed"
            );
        }
    }

    #[test]
    fn production_settings_require_all_persistent_configuration() {
        let missing = required_env("PLE_COMPOSITION_TEST_MISSING").expect_err("must be absent");
        assert!(missing.to_string().contains("PLE_COMPOSITION_TEST_MISSING"));
    }

    fn production_settings() -> ProductionSettings {
        ProductionSettings {
            storage: StorageSettings {
                database_url: "postgres://user:password@127.0.0.1:1/ple".to_string(),
                s3_endpoint: "http://127.0.0.1:1".to_string(),
                s3_region: "us-east-1".to_string(),
                access_key_id: "test-access".to_string(),
                secret_access_key: "test-secret".to_string(),
                content_bucket: "content".to_string(),
                student_records_bucket: "student-records".to_string(),
                temp_processing_bucket: "temp-processing".to_string(),
            },
            public_asset_base_url: "https://cdn.example.test/content".to_string(),
            webwork_renderer_base_url: "http://webwork-renderer:8080/webwork2/".to_string(),
            webwork_request_timeout_seconds: 15,
            webwork_max_response_bytes: 1_048_576,
            webwork_renderer_id: "ple-webwork-renderer".to_string(),
            webwork_renderer_version: "1".to_string(),
            webwork_course_id: "ple_render".to_string(),
            webwork_user: "ple_service".to_string(),
            webwork_password_file: "/private/tmp/ple-test-webwork-password".to_string(),
            imathas_provider_key: None,
            qti_runtime_enabled: None,
            grader_database_url: None,
        }
    }

    #[test]
    fn grader_runtime_is_required_redacted_and_qti_is_explicit() {
        let settings = production_settings();
        let error = settings
            .grader_database_url()
            .expect_err("flat native grading requires a dedicated grader URL")
            .to_string();
        assert!(error.contains("PLE_GRADER_DATABASE_URL"));

        for url in [
            "   ",
            "https://ple_grading_reader:secret@db/ple",
            "not-a-url-with-secret",
        ] {
            let mut settings = production_settings();
            settings.grader_database_url = Some(url.to_string());
            let error = settings
                .grader_database_url()
                .expect_err("malformed grader URL must reject")
                .to_string();
            assert!(
                !error.contains("secret"),
                "grader configuration failure must not expose its database URL: {error}"
            );
        }

        let mut settings = production_settings();
        settings.grader_database_url =
            Some("postgres://ple_grading_reader:secret@db.internal/ple".to_string());
        assert!(
            settings
                .grader_database_url()
                .unwrap()
                .starts_with("postgres://")
        );
        assert!(!settings.qti_runtime_enabled().unwrap());

        settings.qti_runtime_enabled = Some("1".to_string());
        assert!(settings.qti_runtime_enabled().unwrap());
        settings.qti_runtime_enabled = Some("true".to_string());
        assert!(settings.qti_runtime_enabled().is_err());
    }

    #[test]
    fn webwork_renderer_settings_fail_closed_before_router_construction() {
        let password_file =
            std::env::temp_dir().join(format!("ple-webwork-password-{}", std::process::id()));
        std::fs::write(&password_file, "test-render-password\n")
            .expect("test password file should be writable");
        #[cfg(unix)]
        std::fs::set_permissions(
            &password_file,
            std::os::unix::fs::PermissionsExt::from_mode(0o600),
        )
        .expect("test password file permissions should be writable");
        let mut valid = production_settings();
        valid.webwork_password_file = password_file.display().to_string();
        assert!(valid.webwork_renderer().is_ok());
        std::fs::remove_file(&password_file).expect("test password file should be removable");

        let mut invalid_base = production_settings();
        invalid_base.webwork_renderer_base_url = "ftp://renderer.example.test".to_string();
        assert!(invalid_base.webwork_renderer().is_err());

        let mut query_base = production_settings();
        query_base.webwork_renderer_base_url =
            "http://webwork-renderer:8080?secret=not-allowed".to_string();
        assert!(query_base.webwork_renderer().is_err());

        let mut zero_timeout = production_settings();
        zero_timeout.webwork_request_timeout_seconds = 0;
        assert!(zero_timeout.webwork_renderer().is_err());

        let mut zero_bytes = production_settings();
        zero_bytes.webwork_max_response_bytes = 0;
        assert!(zero_bytes.webwork_renderer().is_err());

        let mut missing_id = production_settings();
        missing_id.webwork_renderer_id = " ".to_string();
        assert!(missing_id.webwork_renderer().is_err());

        let mut missing_version = production_settings();
        missing_version.webwork_renderer_version.clear();
        assert!(missing_version.webwork_renderer().is_err());

        assert!(parse_positive_u64("PLE_WEBWORK_REQUEST_TIMEOUT_SECONDS", "nan").is_err());
        assert!(parse_positive_usize("PLE_WEBWORK_MAX_RESPONSE_BYTES", "0").is_err());
    }

    #[cfg(unix)]
    #[test]
    fn webwork_password_file_refuses_symlink_and_permissive_mode() {
        use std::os::unix::fs::{PermissionsExt as _, symlink};

        let nonce = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("system clock is after the Unix epoch")
            .as_nanos();
        let root = std::env::temp_dir().join(format!(
            "ple-webwork-password-{}-{nonce}",
            std::process::id()
        ));
        std::fs::create_dir(&root).expect("private test directory should be created");
        let file = root.join("secret");
        std::fs::write(&file, "test-render-password\n").expect("test secret should be written");
        std::fs::set_permissions(&file, std::fs::Permissions::from_mode(0o644))
            .expect("test secret permissions should be writable");
        assert!(read_webwork_password_file(file.to_str().expect("UTF-8 test path")).is_err());
        std::fs::set_permissions(&file, std::fs::Permissions::from_mode(0o600))
            .expect("test secret permissions should be writable");
        assert!(read_webwork_password_file(file.to_str().expect("UTF-8 test path")).is_ok());
        let link = root.join("secret-link");
        symlink(&file, &link).expect("test symlink should be created");
        assert!(read_webwork_password_file(link.to_str().expect("UTF-8 test path")).is_err());
        std::fs::remove_file(&link).expect("test symlink should be removable");
        std::fs::remove_file(&file).expect("test secret should be removable");
        std::fs::remove_dir(&root).expect("private test directory should be removable");
    }

    #[test]
    fn imathas_state_secret_requires_canonical_exact_256_bits() {
        let encoded = URL_SAFE_NO_PAD.encode([7_u8; 32]);
        assert_eq!(
            parse_secret32("PLE_COMPOSITION_TEST_IMATHAS_SECRET", &encoded).unwrap(),
            [7; 32]
        );
        assert!(parse_secret32("PLE_COMPOSITION_TEST_IMATHAS_SECRET", "not-base64").is_err());
    }

    const IMATHAS_ENV_NAMES: [&str; 12] = [
        "PLE_IMATHAS_BASE_URL",
        "PLE_IMATHAS_REQUEST_TIMEOUT_SECONDS",
        "PLE_IMATHAS_MAX_TRANSPORT_BYTES",
        "PLE_IMATHAS_MAX_SNAPSHOT_BYTES",
        "PLE_IMATHAS_MAX_RESULT_BYTES",
        "PLE_IMATHAS_LAUNCH_TTL_MILLIS",
        "PLE_IMATHAS_LAUNCH_STATE_SECRET",
        "PLE_IMATHAS_CORRELATION_SECRET",
        "PLE_IMATHAS_LAUNCH_SIGNING_SECRET",
        "PLE_IMATHAS_RESULT_VERIFICATION_SECRET",
        "PLE_IMATHAS_PROVIDER_AUTH_HEADER_NAME",
        "PLE_IMATHAS_PROVIDER_AUTH_VALUE",
    ];

    fn with_imathas_environment<T>(
        replacements: &[(&str, Option<&str>)],
        action: impl FnOnce() -> T,
    ) -> T {
        static ENVIRONMENT_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());
        let _lock = ENVIRONMENT_LOCK.lock().expect("iMathAS environment lock");
        let saved = IMATHAS_ENV_NAMES
            .iter()
            .map(|name| (*name, std::env::var(name).ok()))
            .collect::<Vec<_>>();
        for name in IMATHAS_ENV_NAMES {
            // Test-only process environment setup is serialized above. These
            // names are read solely by the production configuration boundary.
            unsafe { std::env::remove_var(name) };
        }
        for (name, value) in replacements {
            unsafe { std::env::remove_var(name) };
            if let Some(value) = value {
                unsafe { std::env::set_var(name, value) };
            }
        }
        let output = action();
        for name in IMATHAS_ENV_NAMES {
            unsafe { std::env::remove_var(name) };
        }
        for (name, value) in saved {
            if let Some(value) = value {
                unsafe { std::env::set_var(name, value) };
            }
        }
        output
    }

    fn valid_imathas_environment() -> Vec<(&'static str, Option<&'static str>)> {
        let secret = "AQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQE";
        vec![
            (
                "PLE_IMATHAS_BASE_URL",
                Some("https://provider.internal.example"),
            ),
            ("PLE_IMATHAS_REQUEST_TIMEOUT_SECONDS", Some("15")),
            ("PLE_IMATHAS_MAX_TRANSPORT_BYTES", Some("1048576")),
            ("PLE_IMATHAS_MAX_SNAPSHOT_BYTES", Some("1048576")),
            ("PLE_IMATHAS_MAX_RESULT_BYTES", Some("8192")),
            ("PLE_IMATHAS_LAUNCH_TTL_MILLIS", Some("30000")),
            ("PLE_IMATHAS_LAUNCH_STATE_SECRET", Some(secret)),
            ("PLE_IMATHAS_CORRELATION_SECRET", Some(secret)),
            (
                "PLE_IMATHAS_LAUNCH_SIGNING_SECRET",
                Some("test-launch-signing-secret"),
            ),
            (
                "PLE_IMATHAS_RESULT_VERIFICATION_SECRET",
                Some("test-result-verification-secret"),
            ),
        ]
    }

    #[tokio::test]
    async fn imathas_production_configuration_fails_closed_without_provider_ping() {
        let mut configured = production_settings();
        configured.imathas_provider_key = Some("institution-imathas".to_string());
        let store = Arc::new(PostgresStore::new(
            lazy_pool(&configured.storage.database_url).expect("lazy postgres pool"),
        ));
        let object_client = objects::minio::client(&objects::minio::EndpointConfig {
            endpoint_url: configured.storage.s3_endpoint.clone(),
            region: configured.storage.s3_region.clone(),
            access_key_id: configured.storage.access_key_id.clone(),
            secret_access_key: configured.storage.secret_access_key.clone(),
        });
        let objects = Arc::new(objects::s3::S3ObjectStore::new(
            object_client,
            objects::s3::BucketNames {
                content: configured.storage.content_bucket.clone(),
                student_records: configured.storage.student_records_bucket.clone(),
                temp_processing: configured.storage.temp_processing_bucket.clone(),
            },
        ));

        assert!(production_settings().imathas(&store, &objects).is_ok());
        let valid = valid_imathas_environment();
        let valid_result = with_imathas_environment(&valid, || {
            configured
                .imathas(&store, &objects)
                .err()
                .map(|error| error.to_string())
        });
        assert!(
            valid_result.is_none(),
            "valid config rejected: {valid_result:?}"
        );

        for (name, value) in [
            ("PLE_IMATHAS_BASE_URL", Some("http://provider.example.test")),
            (
                "PLE_IMATHAS_BASE_URL",
                Some("https://provider.example.test?token=secret"),
            ),
            ("PLE_IMATHAS_REQUEST_TIMEOUT_SECONDS", Some("0")),
            ("PLE_IMATHAS_REQUEST_TIMEOUT_SECONDS", Some("not-a-number")),
            ("PLE_IMATHAS_MAX_TRANSPORT_BYTES", Some("0")),
            ("PLE_IMATHAS_MAX_SNAPSHOT_BYTES", Some("0")),
            ("PLE_IMATHAS_MAX_SNAPSHOT_BYTES", Some("1048577")),
            ("PLE_IMATHAS_MAX_RESULT_BYTES", Some("0")),
            ("PLE_IMATHAS_MAX_RESULT_BYTES", Some("8193")),
            ("PLE_IMATHAS_LAUNCH_TTL_MILLIS", Some("0")),
            ("PLE_IMATHAS_LAUNCH_TTL_MILLIS", Some("300001")),
            ("PLE_IMATHAS_LAUNCH_STATE_SECRET", Some("not-32-bytes")),
            ("PLE_IMATHAS_CORRELATION_SECRET", Some("AQE")),
            ("PLE_IMATHAS_LAUNCH_SIGNING_SECRET", None),
            ("PLE_IMATHAS_RESULT_VERIFICATION_SECRET", None),
            (
                "PLE_IMATHAS_PROVIDER_AUTH_HEADER_NAME",
                Some("wrong-header"),
            ),
            (
                "PLE_IMATHAS_PROVIDER_AUTH_VALUE",
                Some("secret-without-name"),
            ),
        ] {
            let mut values = valid_imathas_environment();
            values.push((name, value));
            let error =
                with_imathas_environment(&values, || match configured.imathas(&store, &objects) {
                    Ok(_) => panic!("invalid iMathAS setting must reject construction"),
                    Err(error) => error.to_string(),
                });
            assert!(
                !error.contains("secret-without-name") && !error.contains("token=secret"),
                "configuration errors must redact values: {error}"
            );
        }
    }

    #[test]
    fn renderer_password_is_required_and_never_exposed_by_debug() {
        let authenticated = production_settings();
        let secret = "renderer-password-file-secret";
        let config = HttpWebworkRendererConfig::new(
            &authenticated.webwork_renderer_base_url,
            std::time::Duration::from_secs(authenticated.webwork_request_timeout_seconds),
            authenticated.webwork_max_response_bytes,
            RendererIdentity {
                id: authenticated.webwork_renderer_id.clone(),
                version: authenticated.webwork_renderer_version.clone(),
            },
            &authenticated.webwork_course_id,
            &authenticated.webwork_user,
            secret,
        )
        .expect("valid renderer settings");
        let debug = format!("{config:?}");
        assert!(!debug.contains(secret));
        assert!(debug.contains("[REDACTED]"));

        assert!(
            HttpWebworkRendererConfig::new(
                &authenticated.webwork_renderer_base_url,
                std::time::Duration::from_secs(authenticated.webwork_request_timeout_seconds),
                authenticated.webwork_max_response_bytes,
                RendererIdentity {
                    id: authenticated.webwork_renderer_id.clone(),
                    version: authenticated.webwork_renderer_version.clone()
                },
                &authenticated.webwork_course_id,
                &authenticated.webwork_user,
                "",
            )
            .is_err()
        );
    }

    #[tokio::test]
    async fn composition_mounts_every_route_group() {
        let app = composed_memory_router();

        for (method, path) in [
            ("GET", "/api/auth/session"),
            ("GET", "/api/problems"),
            ("GET", "/api/taxonomy"),
            ("GET", "/api/courses"),
            (
                "GET",
                "/api/courses/00000000-0000-0000-0000-000000000001/appearance",
            ),
            ("GET", "/api/runs/example"),
            (
                "PUT",
                "/api/workspaces/00000000-0000-0000-0000-000000000001/flat-question",
            ),
            (
                "POST",
                "/api/problems/00000000-0000-0000-0000-000000000001/flat-question-publish",
            ),
            // A POST to a GET-only asset route reaches axum's method router
            // (405) without opening the deliberately unreachable test DB.
            ("POST", "/api/assets/not-a-uuid"),
            ("POST", "/api/validation/timer"),
        ] {
            let request = Request::builder()
                .method(method)
                .uri(path)
                .body(Body::empty())
                .expect("request");
            let response = app.clone().oneshot(request).await.expect("router response");
            assert_ne!(
                response.status(),
                StatusCode::NOT_FOUND,
                "missing route {path}"
            );
        }
    }

    #[tokio::test]
    async fn disabled_imathas_does_not_mount_protected_broker_routes() {
        let app = composed_memory_router();
        for (method, path) in [
            (
                "GET",
                "/api/attempts/00000000-0000-0000-0000-000000000001/external-tool/launch",
            ),
            (
                "GET",
                "/api/attempts/00000000-0000-0000-0000-000000000001/external-tool/launch/activity",
            ),
            (
                "POST",
                "/api/attempts/00000000-0000-0000-0000-000000000001/external-tool/launch/submission",
            ),
        ] {
            let response = app
                .clone()
                .oneshot(
                    Request::builder()
                        .method(method)
                        .uri(path)
                        .body(Body::empty())
                        .expect("protected broker request"),
                )
                .await
                .expect("protected broker response");
            assert_eq!(
                response.status(),
                StatusCode::NOT_FOUND,
                "{path} must be unmounted"
            );
        }
    }

    #[test]
    fn composition_uses_only_key_free_composite_backends() {
        // The composition root accepts a server-only composite: native keeps
        // its key-free issue/reproduce contract while WeBWorK resolves its PG
        // source and grades only on the server.
        fn accepts_only_composite(
            _: &CompositeBackend<PostgresStore, objects::s3::S3ObjectStore, HttpWebworkRenderer>,
        ) {
        }
        let _ = accepts_only_composite;
    }

    #[test]
    fn local_identity_mode_is_explicit_and_other_modes_fail_closed() {
        let Err(missing_flag) =
            local_development_authentication("local-file", "", "/does/not/matter")
        else {
            panic!("local mode requires its explicit development flag");
        };
        assert!(
            missing_flag
                .to_string()
                .contains("PLE_ENABLE_LOCAL_DEVELOPMENT_AUTH")
        );

        let Err(oidc) = local_development_authentication("oidc", "1", "/does/not/matter") else {
            panic!("OIDC is not silently replaced with a local identity");
        };
        assert!(oidc.to_string().contains("OIDC"));

        assert_eq!(
            local_development_session_config().transport(),
            CookieTransport::LocalHttp
        );
    }

    #[tokio::test]
    async fn local_provider_hashes_raw_bearer_bytes_not_base64url_spelling() {
        let provider = local_provider();
        let credential = "AAECAwQFBgcICQoLDA0ODxAREhMUFRYXGBkaGxwdHh8";
        assert!(
            provider
                .verify(&LocalLoginPresentation {
                    credential: credential.to_string(),
                })
                .await
                .is_ok()
        );

        let encoded_hash = Sha256::digest(credential.as_bytes())
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>();
        let encoded_provider = LocalFileIdentityProvider::from_json_bytes(
            format!(
                r#"{{"credentials":[{{"credential_sha256":"{encoded_hash}","tenant_id":"00000000-0000-0000-0000-000000000001","user_id":"00000000-0000-0000-0000-000000000002","display_name":"Local Student","roles":["student"]}}]}}"#
            )
            .as_bytes(),
        )
        .expect("encoded-spelling hash is syntactically valid configuration");
        assert!(matches!(
            encoded_provider
                .verify(&LocalLoginPresentation {
                    credential: credential.to_string(),
                })
                .await,
            Err(IdentityProviderError::Rejected)
        ));
    }

    #[tokio::test]
    async fn local_provider_only_accepts_canonical_fixed_identity_login() {
        let app = crate::auth::router(
            Arc::new(local_provider()),
            Arc::new(MemoryStore::default()),
            local_development_session_config(),
        );
        let credential = "AAECAwQFBgcICQoLDA0ODxAREhMUFRYXGBkaGxwdHh8";
        let accepted = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/auth/login")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        serde_json::json!({ "credential": credential }).to_string(),
                    ))
                    .expect("login request"),
            )
            .await
            .expect("login response");
        assert_eq!(accepted.status(), StatusCode::OK);

        let injection = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/auth/login")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        serde_json::json!({
                            "credential": credential,
                            "tenantId": "ffffffff-ffff-ffff-ffff-ffffffffffff"
                        })
                        .to_string(),
                    ))
                    .expect("injection request"),
            )
            .await
            .expect("injection response");
        assert_eq!(injection.status(), StatusCode::UNPROCESSABLE_ENTITY);

        for invalid in [
            "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA",
            "AAECAwQFBgcICQoLDA0ODxAREhMUFRYXGBkaGxwdHh8=",
            "AAECAwQFBgcICQoLDA0ODxAREhMUFRYXGBkaGxwdHh+",
        ] {
            assert!(matches!(
                local_provider()
                    .verify(&LocalLoginPresentation {
                        credential: invalid.to_string(),
                    })
                    .await,
                Err(IdentityProviderError::Rejected)
            ));
        }
    }

    #[test]
    fn local_identity_file_rejects_invalid_records() {
        for invalid in [
            br#"{"credentials":[]}"#.as_slice(),
            br#"{"credentials":[{"credential_sha256":"ABCDEF","tenant_id":"00000000-0000-0000-0000-000000000001","user_id":"00000000-0000-0000-0000-000000000002","display_name":"Student","roles":["student"]}]}"#.as_slice(),
            br#"{"credentials":[{"credential_sha256":"630dcd2966c4336691125448bbb25b4ff412a49c732db2c8abc1b8581bd710dd","tenant_id":"00000000-0000-0000-0000-000000000000","user_id":"00000000-0000-0000-0000-000000000002","display_name":"Student","roles":["student"]}]}"#.as_slice(),
        ] {
            assert!(matches!(
                LocalFileIdentityProvider::from_json_bytes(invalid),
                Err(IdentityProviderError::Unavailable(_))
            ));
        }
    }
}

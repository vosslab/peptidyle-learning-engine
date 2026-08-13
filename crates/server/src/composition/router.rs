//! Route and readiness composition.

use super::*;

const SCHEMA_VERIFICATION_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(2);

pub(super) async fn verify_application_schema_bounded(
    pool: &Pool,
) -> Result<(), SchemaCompatibilityError> {
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

/// Test-only legacy-provider composition used to prove the real local route
/// graph. Production cannot compile this helper or its provider login merge.
#[cfg(all(test, feature = "local-development-auth"))]
#[allow(clippy::too_many_arguments)]
pub(super) fn compose_router<S, O, C, B, P, R>(
    store: Arc<S>,
    objects: Arc<O>,
    public_assets: Arc<C>,
    backends: Arc<B>,
    native_adapter: Arc<adapter_native::NativeAdapter>,
    identity_provider: Arc<P>,
    review_gate: Arc<R>,
    session_config: SessionConfig,
    invitation_issuer: crate::course::CourseInvitationIssuer,
    invitation_delivery: Arc<dyn crate::course::CourseInvitationDelivery>,
    passwordless_email_delivery: Arc<dyn crate::auth::PasswordlessEmailDelivery>,
    passwordless_rate_limit_issuer: crate::auth::PasswordlessRateLimitIssuer,
    client_address_policy: crate::auth::ClientAddressPolicy,
    webauthn: Option<crate::auth::PasswordlessWebauthn>,
    health: Arc<HealthState>,
) -> Router
where
    S: Store
        + CatalogStore
        + learning_data_access::FlatQuestionAssetStore
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
        + learning_data_access::CourseRosterStore
        + learning_data_access::ManualGradeExportStore
        + learning_data_access::AccountIdentityStore
        + learning_data_access::AccountSessionStore
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
    crate::http_security::apply_api_security_headers(
        compose_passwordless_router(
            Arc::clone(&store),
            objects,
            public_assets,
            backends,
            native_adapter,
            review_gate,
            session_config,
            invitation_issuer,
            invitation_delivery,
            passwordless_email_delivery,
            passwordless_rate_limit_issuer,
            client_address_policy,
            webauthn,
            None,
            health,
        )
        .merge(crate::auth::provider_login_router(
            identity_provider,
            store,
            session_config,
        )),
    )
}

/// Merges every route that uses PLE-owned account authentication or no
/// identity provider at all. Production uses this composition directly so it
/// cannot accidentally read or expose the local-file development login path.
#[allow(clippy::too_many_arguments)]
pub(super) fn compose_passwordless_router<S, O, C, B, R>(
    store: Arc<S>,
    objects: Arc<O>,
    public_assets: Arc<C>,
    backends: Arc<B>,
    native_adapter: Arc<adapter_native::NativeAdapter>,
    review_gate: Arc<R>,
    session_config: SessionConfig,
    invitation_issuer: crate::course::CourseInvitationIssuer,
    invitation_delivery: Arc<dyn crate::course::CourseInvitationDelivery>,
    passwordless_email_delivery: Arc<dyn crate::auth::PasswordlessEmailDelivery>,
    passwordless_rate_limit_issuer: crate::auth::PasswordlessRateLimitIssuer,
    client_address_policy: crate::auth::ClientAddressPolicy,
    webauthn: Option<crate::auth::PasswordlessWebauthn>,
    local_teaching_roster: Option<Arc<crate::course::LocalTeachingRosterDirectory>>,
    health: Arc<HealthState>,
) -> Router
where
    S: Store
        + CatalogStore
        + learning_data_access::FlatQuestionAssetStore
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
        + learning_data_access::CourseRosterStore
        + learning_data_access::ManualGradeExportStore
        + learning_data_access::AccountIdentityStore
        + learning_data_access::AccountSessionStore
        + SessionStore
        + AssetStore
        + CourseAppearanceStore
        + AuthoritativeTimeStore
        + 'static,
    O: objects::ObjectStore + 'static,
    C: PublicAssetUrlResolver + 'static,
    B: BackendRegistry + RunBackend + 'static,
    R: PublicReviewGate + 'static,
{
    let passkey_rate_limit_issuer = passwordless_rate_limit_issuer.clone();
    let passkey_client_address_policy = client_address_policy.clone();
    let mut router = Router::new()
        .route("/health", get(health_handler))
        .merge(crate::auth::session_router(
            Arc::clone(&store),
            session_config,
        ))
        .merge(crate::auth::passwordless_router(
            Arc::clone(&store),
            passwordless_email_delivery,
            passwordless_rate_limit_issuer,
            client_address_policy,
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
        .merge(crate::flat_question_assets::router(
            Arc::clone(&store),
            Arc::clone(&objects),
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
        .merge(crate::course::router_with_invitations_and_local_teaching(
            Arc::clone(&store),
            invitation_issuer,
            invitation_delivery,
            local_teaching_roster,
        ))
        .merge(crate::course_appearance::router(
            Arc::clone(&store),
            Arc::clone(&objects),
        ))
        .merge(crate::item_analysis::router(Arc::clone(&store)))
        .merge(crate::export::router(Arc::clone(&store)))
        .merge(crate::retention::router(Arc::clone(&store)))
        .merge(crate::run::router(Arc::clone(&store), backends))
        .merge(crate::asset::router(store.clone(), objects, public_assets))
        .merge(crate::validation::router(Arc::clone(&store)))
        .layer(Extension(health));

    if let Some(webauthn) = webauthn {
        router = router.merge(crate::auth::passkey_router(
            Arc::clone(&store),
            webauthn,
            passkey_rate_limit_issuer,
            passkey_client_address_policy,
            session_config,
        ));
    }

    crate::route_policy::apply_route_method_policy(apply_e2e_replica_attribution(
        router,
        e2e_replica_attribution_from_env(),
    ))
}

/// Opaque container identity carried only by the test-only replica E2E build.
/// It deliberately has no access to the request, route state, tenant, object
/// store, or any other process configuration.
#[cfg(feature = "e2e-observability")]
#[derive(Clone)]
pub(super) struct ReplicaAttribution(HeaderValue);

#[cfg(not(feature = "e2e-observability"))]
type ReplicaAttribution = ();

#[cfg(feature = "e2e-observability")]
const E2E_REPLICA_HEADER: HeaderName = HeaderName::from_static("x-ple-e2e-replica");
#[cfg(feature = "e2e-observability")]
const E2E_REPLICA_PREFIX: &str = "ple-replica-e2e-api-";
#[cfg(feature = "e2e-observability")]
const E2E_REPLICA_SUFFIX_LEN: usize = 12;

pub(super) fn apply_e2e_replica_attribution(
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
#[cfg_attr(not(feature = "e2e-observability"), allow(dead_code))]
pub(super) fn e2e_replica_attribution_from_values(
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
pub(super) struct HealthState {
    pub(super) postgres: Pool,
    pub(super) object_client: objects::minio::S3Client,
    pub(super) public_assets_bucket: String,
    pub(super) private_content_bucket: String,
    pub(super) student_records_bucket: String,
    pub(super) temp_processing_bucket: String,
}

pub(super) fn postgres_schema_probe(
    verification: Result<(), SchemaCompatibilityError>,
) -> ProbeResult {
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
    let public_assets = probe_object_bucket(
        &state.object_client,
        &state.public_assets_bucket,
        "public-assets",
    )
    .await;
    let private_content = probe_object_bucket(
        &state.object_client,
        &state.private_content_bucket,
        "private-content",
    )
    .await;
    let student_records = probe_object_bucket(
        &state.object_client,
        &state.student_records_bucket,
        "student-records",
    )
    .await;
    let temp_processing = probe_object_bucket(
        &state.object_client,
        &state.temp_processing_bucket,
        "temp-processing",
    )
    .await;
    match readiness(&[
        postgres,
        public_assets,
        private_content,
        student_records,
        temp_processing,
    ]) {
        Readiness::Ready => (StatusCode::OK, Json(json!({ "status": "ready" }))).into_response(),
        Readiness::Degraded(failing) => (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(json!({ "status": "degraded", "failing": failing })),
        )
            .into_response(),
    }
}

async fn probe_object_bucket(
    client: &objects::minio::S3Client,
    bucket: &str,
    name: &'static str,
) -> ProbeResult {
    match objects::minio::probe_bucket(client, bucket).await {
        Ok(()) => ProbeResult::ready(name),
        Err(error) => {
            eprintln!("{name} object-store probe failed: {error}");
            ProbeResult::failed(name)
        }
    }
}

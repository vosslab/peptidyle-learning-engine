use std::sync::Arc;

use async_trait::async_trait;
use axum::body::Body;
use axum::http::{HeaderValue, Request, StatusCode};
use learning_data_access::SessionLifetime;
use learning_data_access::in_memory::MemoryStore;
use objects::memory::MemoryObjectStore;
use question_model::UserId;
use tower::ServiceExt;

use super::router::{
    HealthState, apply_e2e_replica_attribution, compose_passwordless_router,
    e2e_replica_attribution_from_values, postgres_schema_probe,
};
use super::settings::{
    ObjectStorageConnection, ProcessRole, ProductionSettings, StorageRuntime, StorageSettings,
    StorageTopology, WebworkRendererSettings, parse_positive_u64, parse_positive_usize,
    parse_secret32, required_env,
};
use super::*;
use crate::auth::CookieTransport;
use crate::catalog::ReviewGateError;

mod account_fixture;
mod live_demo_sysadmin_settings;
mod presentation_routes;
mod production_router;

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
        CookieTransport::FirstPartyHttps,
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
    composed_memory_router_and_store().0
}

pub(super) fn composed_memory_router_and_store() -> (Router, Arc<MemoryStore>) {
    composed_memory_router_and_store_with_session_config(session_config())
}

pub(super) fn composed_memory_router_and_store_with_session_config(
    session_config: SessionConfig,
) -> (Router, Arc<MemoryStore>) {
    composed_memory_router_and_store_with_live_demo_selector(session_config, None)
}

pub(super) fn composed_memory_router_and_store_with_live_demo_selector(
    session_config: SessionConfig,
    live_demo_selector: Option<crate::auth::SeededAccountSelectorConfig>,
) -> (Router, Arc<MemoryStore>) {
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
            "http://renderer.internal/",
            std::time::Duration::from_secs(1),
            1_024,
            RendererIdentity {
                id: "test-renderer".to_string(),
                version: "1".to_string(),
            },
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
        public_assets_bucket: "public-assets".to_string(),
        private_content_bucket: "private-content".to_string(),
        student_records_bucket: "student-records".to_string(),
        temp_processing_bucket: "temp-processing".to_string(),
    });
    let sealed_memory = Arc::new(
        learning_data_access::in_memory::MemorySealedPrivateExecutionStore::new(Arc::clone(&store)),
    );
    let sealed_execution: Arc<dyn learning_data_access::SealedPrivateExecutionStore> =
        sealed_memory.clone();
    let rehearsal_sealed: Arc<dyn learning_data_access::SealedRehearsalDeliveryExecutionStore> =
        sealed_memory;
    let rehearsal_coordinator = Arc::new(crate::rehearsal::RehearsalExecutionCoordinator::new(
        Arc::clone(&backends),
        rehearsal_sealed,
    ));
    let router = compose_passwordless_router(
        Arc::clone(&store),
        objects,
        public_assets,
        backends,
        sealed_execution,
        rehearsal_coordinator,
        native_adapter,
        Arc::new(TestReview),
        session_config,
        crate::course::CourseInvitationIssuer::unavailable(),
        Arc::new(crate::auth::UnavailablePasswordlessEmailDelivery),
        crate::auth::PasswordlessRateLimitIssuer::unavailable(),
        crate::auth::ClientAddressPolicy::direct(),
        live_demo_selector,
        None,
        Some(
            crate::auth::PasswordlessWebauthn::new(
                "localhost",
                "http://localhost:3000",
                "PLE test",
            )
            .expect("valid test WebAuthn configuration"),
        ),
        health,
    );
    (router, store)
}

#[tokio::test]
async fn deployment_enabled_selector_reaches_the_complete_route_composition() {
    let users = [
        UserId::from_uuid(uuid::Uuid::from_u128(1)),
        UserId::from_uuid(uuid::Uuid::from_u128(2)),
        UserId::from_uuid(uuid::Uuid::from_u128(3)),
        UserId::from_uuid(uuid::Uuid::from_u128(4)),
    ];
    let selector = crate::auth::SeededAccountSelectorConfig::new(
        Arc::from("https://demo.example.test"),
        users,
    )
    .expect("selector configuration");
    let (app, store) =
        composed_memory_router_and_store_with_live_demo_selector(session_config(), Some(selector));
    for (user, name) in users.into_iter().zip(["Elena", "Mary", "Jack", "Avery"]) {
        account_fixture::provision_account(store.as_ref(), user, name).await;
    }
    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/auth/live-demo/accounts")
                .body(Body::empty())
                .expect("selector availability request"),
        )
        .await
        .expect("selector availability response");
    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn production_composition_does_not_mount_provider_login() {
    let response = composed_memory_router()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/auth/login")
                .body(Body::from("{}"))
                .expect("provider login request"),
        )
        .await
        .expect("provider login response");
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
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

#[test]
fn object_security_domains_require_distinct_trimmed_bucket_names() {
    use super::settings::validate_object_bucket_names;

    assert!(
        validate_object_bucket_names(
            "public-assets",
            "private-content",
            "student-records",
            "temp-processing",
        )
        .is_ok()
    );
    assert!(
        validate_object_bucket_names(
            "public-assets",
            "public-assets",
            "student-records",
            "temp-processing",
        )
        .is_err()
    );
    assert!(
        validate_object_bucket_names(
            "public-assets ",
            "private-content",
            "student-records",
            "temp-processing",
        )
        .is_err()
    );
}

fn production_settings() -> ProductionSettings {
    ProductionSettings {
        storage: StorageSettings {
            runtime: StorageRuntime {
                role: ProcessRole::Api,
                topology: StorageTopology::DisposableLocal,
            },
            database_url: "postgres://user:password@127.0.0.1:1/ple".to_string(),
            question_id_secret: None,
            object_connection: ObjectStorageConnection::LocalMinio(
                objects::minio::EndpointConfig {
                    endpoint_url: "http://127.0.0.1:1".to_string(),
                    region: "us-east-1".to_string(),
                    access_key_id: "test-access".to_string(),
                    secret_access_key: "test-secret".to_string(),
                },
            ),
            public_assets_bucket: "public-assets".to_string(),
            private_content_bucket: "private-content".to_string(),
            student_records_bucket: "student-records".to_string(),
            temp_processing_bucket: "temp-processing".to_string(),
        },
        public_asset_base_url: PublicAssetBaseUrl::new("https://cdn.example.test/content")
            .expect("valid public asset base"),
        webwork: Some(WebworkRendererSettings {
            webwork_renderer_base_url: "http://webwork-renderer:3000/".to_string(),
            webwork_request_timeout_seconds: 15,
            webwork_max_response_bytes: 1_048_576,
            webwork_renderer_id: "vosslab-webwork-pg-renderer".to_string(),
            webwork_renderer_version: "1".to_string(),
        }),
        imathas_provider_key: None,
        qti_runtime_enabled: None,
        grader_database_url: None,
        enrollment_secret: None,
        enrollment_email: None,
        webauthn: crate::auth::PasswordlessWebauthn::new(
            "localhost",
            "http://localhost:3000",
            "PLE test",
        )
        .expect("valid test WebAuthn configuration"),
        browser_boundary: crate::auth::ProductionBrowserBoundary::new(Arc::from(
            "https://learn.example.test",
        ))
        .expect("test browser boundary"),
        client_address_policy: crate::auth::ClientAddressPolicy::direct(),
        live_demo_selector: None,
        live_demo_sysadmin_ownership: None,
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
    let valid = production_settings();
    assert!(valid.webwork_renderer().unwrap().is_some());

    let mut native_only = production_settings();
    native_only.webwork = None;
    assert!(native_only.webwork_renderer().unwrap().is_none());

    let mut invalid_base = production_settings();
    invalid_base
        .webwork
        .as_mut()
        .expect("configured WebWork")
        .webwork_renderer_base_url = "ftp://renderer.example.test".to_string();
    assert!(invalid_base.webwork_renderer().is_err());

    let mut query_base = production_settings();
    query_base
        .webwork
        .as_mut()
        .expect("configured WebWork")
        .webwork_renderer_base_url = "http://webwork-renderer:8080?secret=not-allowed".to_string();
    assert!(query_base.webwork_renderer().is_err());

    let mut zero_timeout = production_settings();
    zero_timeout
        .webwork
        .as_mut()
        .expect("configured WebWork")
        .webwork_request_timeout_seconds = 0;
    assert!(zero_timeout.webwork_renderer().is_err());

    let mut zero_bytes = production_settings();
    zero_bytes
        .webwork
        .as_mut()
        .expect("configured WebWork")
        .webwork_max_response_bytes = 0;
    assert!(zero_bytes.webwork_renderer().is_err());

    let mut missing_id = production_settings();
    missing_id
        .webwork
        .as_mut()
        .expect("configured WebWork")
        .webwork_renderer_id = " ".to_string();
    assert!(missing_id.webwork_renderer().is_err());

    let mut missing_version = production_settings();
    missing_version
        .webwork
        .as_mut()
        .expect("configured WebWork")
        .webwork_renderer_version
        .clear();
    assert!(missing_version.webwork_renderer().is_err());

    assert!(parse_positive_u64("PLE_WEBWORK_REQUEST_TIMEOUT_SECONDS", "nan").is_err());
    assert!(parse_positive_usize("PLE_WEBWORK_MAX_RESPONSE_BYTES", "0").is_err());
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

fn replace_imathas_environment_variable(name: &str, value: Option<&str>) {
    // SAFETY: `with_imathas_environment` holds `ENVIRONMENT_LOCK` for this complete
    // mutation-and-read interval. Every test that mutates these iMathAS-only names uses
    // that helper, so no test thread reads or writes them concurrently.
    unsafe {
        match value {
            Some(value) => std::env::set_var(name, value),
            None => std::env::remove_var(name),
        }
    }
}

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
        replace_imathas_environment_variable(name, None);
    }
    for (name, value) in replacements {
        replace_imathas_environment_variable(name, *value);
    }
    let output = action();
    for name in IMATHAS_ENV_NAMES {
        replace_imathas_environment_variable(name, None);
    }
    for (name, value) in saved {
        replace_imathas_environment_variable(name, value.as_deref());
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
        ("PLE_IMATHAS_LAUNCH_SIGNING_SECRET", Some(secret)),
        ("PLE_IMATHAS_RESULT_VERIFICATION_SECRET", Some(secret)),
    ]
}

#[tokio::test]
async fn imathas_production_configuration_fails_closed_without_provider_ping() {
    let mut configured = production_settings();
    configured.imathas_provider_key = Some("institution-imathas".to_string());
    let store = Arc::new(PostgresStore::new(
        lazy_pool(&configured.storage.database_url).expect("lazy postgres pool"),
    ));
    let ObjectStorageConnection::LocalMinio(endpoint) = &configured.storage.object_connection
    else {
        panic!("test storage must use local MinIO");
    };
    let object_client = objects::minio::client(endpoint);
    let objects = Arc::new(objects::s3::S3ObjectStore::new(
        object_client,
        objects::s3::BucketNames {
            public_assets: configured.storage.public_assets_bucket.clone(),
            private_content: configured.storage.private_content_bucket.clone(),
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
        ("PLE_IMATHAS_REQUEST_TIMEOUT_SECONDS", Some("56")),
        ("PLE_IMATHAS_MAX_TRANSPORT_BYTES", Some("0")),
        ("PLE_IMATHAS_MAX_SNAPSHOT_BYTES", Some("0")),
        ("PLE_IMATHAS_MAX_SNAPSHOT_BYTES", Some("1048577")),
        ("PLE_IMATHAS_MAX_RESULT_BYTES", Some("0")),
        ("PLE_IMATHAS_MAX_RESULT_BYTES", Some("8193")),
        ("PLE_IMATHAS_LAUNCH_TTL_MILLIS", Some("0")),
        ("PLE_IMATHAS_LAUNCH_TTL_MILLIS", Some("300001")),
        ("PLE_IMATHAS_LAUNCH_STATE_SECRET", Some("not-32-bytes")),
        ("PLE_IMATHAS_CORRELATION_SECRET", Some("AQE")),
        ("PLE_IMATHAS_LAUNCH_SIGNING_SECRET", Some("too-short")),
        (
            "PLE_IMATHAS_RESULT_VERIFICATION_SECRET",
            Some("AQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQE="),
        ),
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
fn renderer_configuration_debug_exposes_no_credentials() {
    let configured = production_settings().webwork.expect("configured WebWork");
    let config = HttpWebworkRendererConfig::new(
        &configured.webwork_renderer_base_url,
        std::time::Duration::from_secs(configured.webwork_request_timeout_seconds),
        configured.webwork_max_response_bytes,
        RendererIdentity {
            id: configured.webwork_renderer_id.clone(),
            version: configured.webwork_renderer_version.clone(),
        },
    )
    .expect("valid renderer settings");
    let debug = format!("{config:?}");
    assert!(debug.contains("vosslab-webwork-pg-renderer"));
    assert!(!debug.contains("password") && !debug.contains("course_id"));
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
async fn composition_mounts_protected_asset_delivery_as_a_post_route() {
    let response = composed_memory_router()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/assets/00000000-0000-0000-0000-000000000001/delivery")
                .body(Body::empty())
                .expect("protected asset delivery request"),
        )
        .await
        .expect("protected asset delivery response");

    // The valid identifier reaches the route and its authentication boundary.
    // This avoids using the public GET endpoint as a route-presence oracle:
    // that endpoint deliberately returns the same 404 for protected and
    // nonexistent assets.
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn disabled_imathas_does_not_mount_protected_broker_routes() {
    let app = composed_memory_router();
    for (method, path) in [
        (
            "GET",
            "/api/courses/00000000-0000-0000-0000-000000000002/assignments/00000000-0000-0000-0000-000000000003/attempts/00000000-0000-0000-0000-000000000001/external-tool/launch",
        ),
        (
            "GET",
            "/api/courses/00000000-0000-0000-0000-000000000002/assignments/00000000-0000-0000-0000-000000000003/attempts/00000000-0000-0000-0000-000000000001/external-tool/launch/activity",
        ),
        (
            "POST",
            "/api/courses/00000000-0000-0000-0000-000000000002/assignments/00000000-0000-0000-0000-000000000003/attempts/00000000-0000-0000-0000-000000000001/external-tool/launch/submission",
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

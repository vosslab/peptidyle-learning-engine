//! Explicit assembly for the current Account and Authenticated Session server.

use std::{net::SocketAddr, sync::Arc};

use adapter_webwork::{HttpWebworkRenderer, HttpWebworkRendererConfig, WebworkAdapter};
use anyhow::{Context, Result, bail};
use axum::{Router, routing::get};
use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use learning_data_access::{
    SessionLifetime,
    postgres::{
        PostgresAuthoringDraftStore, PostgresBlueprintCourseStore, PostgresCourseInstanceStore,
        PostgresCourseRosterStore, PostgresDraftQuestionSourceBindingStore,
        PostgresInstructorAccountStore, PostgresInvitationExportStore,
        PostgresLiveDemoGradebookStore,
        PostgresLiveAssignmentDeliveryStore, PostgresLiveAssignmentStore,
        PostgresNativePleGradingStore, PostgresNativePleSubmissionStore,
        PostgresWebworkGradingStore,
        PostgresWebworkSubmissionStore,
        PostgresPublicAssetPublicationStore, PostgresQuestionAssetDeliveryStore,
        PostgresQuestionLibraryStore, PostgresSessionStore, PostgresSupportCapabilityStore,
        ProductionLoginProfile, local_development_pool, production_pool,
    },
};
use objects::{
    minio::{EndpointConfig, client as minio_client},
    s3::{BucketNames, S3ObjectStore},
};
use question_model::AccountId;
use question_model::QuestionRendererVersion;

use crate::auth::{
    CookieTransport, ProductionBrowserBoundary, SeededDemoAccount, SeededDemoConfig,
    SeededDemoPersona, SessionConfig, live_demo_router, session_router,
};

const LIVE_DEMO_ACCOUNT_ID_ENV: [(SeededDemoPersona, &str, &str); 5] = [
    (
        SeededDemoPersona::ElenaInstructor,
        "PLE_LIVE_DEMO_ELENA_INSTRUCTOR_ACCOUNT_ID",
        "Elena Instructor",
    ),
    (
        SeededDemoPersona::MaryStudent,
        "PLE_LIVE_DEMO_MARY_STUDENT_ACCOUNT_ID",
        "Mary Student",
    ),
    (
        SeededDemoPersona::JackStudent,
        "PLE_LIVE_DEMO_JACK_STUDENT_ACCOUNT_ID",
        "Jack Student",
    ),
    (
        SeededDemoPersona::AveryStudent,
        "PLE_LIVE_DEMO_AVERY_STUDENT_ACCOUNT_ID",
        "Avery Student",
    ),
    (
        SeededDemoPersona::MorganSysadmin,
        "PLE_LIVE_DEMO_MORGAN_SYSADMIN_ACCOUNT_ID",
        "Morgan Sysadmin",
    ),
];

/// Builds the current route surface from explicit environment configuration.
pub async fn production_router_from_env() -> Result<Router> {
    let database_url = required_env("DATABASE_URL")?;
    let pool = if std::env::var("PLE_STORAGE_TOPOLOGY").ok().as_deref() == Some("disposable-local")
    {
        local_development_pool(&database_url, ProductionLoginProfile::Api)
    } else {
        production_pool(&database_url, ProductionLoginProfile::Api)
    }
    .context("could not construct the attested API database pool")?;
    pool.acquire()
        .await
        .context("the attested API database pool could not connect")?;

    let readiness = crate::health::ReadinessState::from_environment(pool.clone())
        .map_err(anyhow::Error::msg)
        .context("could not configure API readiness checks")?;
    let sessions = Arc::new(PostgresSessionStore::new(pool.clone()));
    let question_library_store = PostgresQuestionLibraryStore::new(pool.clone());
    let blueprint_courses = PostgresBlueprintCourseStore::new(pool.clone());
    let course_instances = PostgresCourseInstanceStore::new(pool.clone());
    let course_roster = PostgresCourseRosterStore::new(pool.clone());
    let instructor_accounts = PostgresInstructorAccountStore::new(pool.clone());
    let support_capabilities = PostgresSupportCapabilityStore::new(pool.clone());
    let invitation_exports = PostgresInvitationExportStore::new(pool.clone());
    let gradebook = PostgresLiveDemoGradebookStore::new(pool.clone());
    let assignments = PostgresLiveAssignmentStore::new(pool.clone());
    let assignment_delivery = PostgresLiveAssignmentDeliveryStore::new(pool.clone());
    let native_ple_submissions = PostgresNativePleSubmissionStore::new(pool.clone());
    let webwork_submissions = PostgresWebworkSubmissionStore::new(pool.clone());
    let question_asset_delivery = PostgresQuestionAssetDeliveryStore::new(pool.clone());
    let authoring_drafts = PostgresAuthoringDraftStore::new(pool.clone());
    let authoring_publication = PostgresDraftQuestionSourceBindingStore::new(pool);
    let question_library_objects = question_library_object_store_from_env().await?;
    let webwork_adapter = webwork_adapter_from_env(question_library_objects.clone())?;
    let question_id_issuer = question_id_issuer_from_env()?;
    let session_config = production_session_config();
    let readiness_router = Router::new()
        .route("/health", get(crate::health::readiness_handler))
        .with_state(readiness);
    let router = Router::new()
        .merge(readiness_router)
        .merge(session_router(Arc::clone(&sessions), session_config))
        .merge(live_demo_router(
            Arc::clone(&sessions),
            live_demo_config_from_env()?,
            session_config,
        ))
        .merge(crate::question_library::question_library_router(
            Arc::clone(&sessions),
            question_library_store.clone(),
            question_library_objects.clone(),
        ))
        .merge(crate::authoring::authoring_router(
            Arc::clone(&sessions),
            authoring_drafts,
            authoring_publication,
            question_library_objects.clone(),
            question_id_issuer,
        ))
        .merge(crate::blueprint_course::blueprint_course_router(
            Arc::clone(&sessions),
            blueprint_courses,
            question_library_store,
            question_library_objects.clone(),
        ))
        .merge(crate::course_instance::course_instance_router(
            Arc::clone(&sessions),
            course_instances,
        ))
        .merge(crate::course_roster::course_roster_router(
            Arc::clone(&sessions),
            course_roster,
        ))
        .merge(crate::instructor_account::instructor_account_router(
            Arc::clone(&sessions),
            instructor_accounts,
        ))
        .merge(crate::support_capability::support_capability_router(
            Arc::clone(&sessions),
            support_capabilities,
        ))
        .merge(crate::invitation_export::invitation_export_router(
            Arc::clone(&sessions),
            invitation_exports,
            invitation_signup_url_from_env()?,
        ))
        .merge(crate::live_gradebook::live_gradebook_router(
            Arc::clone(&sessions),
            gradebook,
        ))
        .merge(crate::assignment_release::assignment_release_router(
            Arc::clone(&sessions),
            assignments,
        ))
        .merge(crate::assignment_delivery::assignment_delivery_router(
            Arc::clone(&sessions),
            assignment_delivery,
            native_ple_submissions,
            webwork_submissions,
            question_library_objects.clone(),
            webwork_adapter,
        ))
        .merge(
            crate::question_asset_delivery::question_asset_delivery_router(
                Arc::clone(&sessions),
                question_asset_delivery,
                crate::question_asset_delivery::public_asset_base_url(&required_env(
                    "PLE_PUBLIC_ASSET_BASE_URL",
                )?)
                .map_err(anyhow::Error::msg)?,
            ),
        );
    let browser_boundary = production_browser_boundary_from_env()?;
    Ok(crate::http_security::apply_api_security_headers(
        router.layer(axum::middleware::from_fn_with_state(
            browser_boundary,
            crate::auth::production_cookie_boundary,
        )),
    ))
}

/// Constructs the private WeBWorK boundary from deployment-owned settings.
/// The renderer version is an attested private file, never a browser input.
fn webwork_adapter_from_env(
    objects: S3ObjectStore,
) -> Result<Arc<WebworkAdapter<S3ObjectStore, HttpWebworkRenderer>>> {
    let version_file = required_env("PLE_WEBWORK_RENDERER_VERSION_FILE")?;
    let attestation = std::fs::read_to_string(version_file)
        .context("could not read the Question Renderer Version attestation")?;
    let version = attestation
        .lines()
        .find_map(|line| line.strip_prefix("oci_id="))
        .filter(|value| value.starts_with("sha256:") && value.len() == 71)
        .ok_or_else(|| anyhow::anyhow!("Question Renderer Version attestation is invalid"))?;
    let timeout = required_env("PLE_WEBWORK_REQUEST_TIMEOUT_SECONDS")?
        .parse::<u64>()
        .context("PLE_WEBWORK_REQUEST_TIMEOUT_SECONDS must be an integer")?;
    let limit = required_env("PLE_WEBWORK_MAX_RESPONSE_BYTES")?
        .parse::<usize>()
        .context("PLE_WEBWORK_MAX_RESPONSE_BYTES must be an integer")?;
    let config = HttpWebworkRendererConfig::new(
        &required_env("PLE_WEBWORK_RENDERER_BASE_URL")?,
        std::time::Duration::from_secs(timeout),
        limit,
        QuestionRendererVersion {
            name: required_env("PLE_WEBWORK_RENDERER_ID")?,
            version: version.to_string(),
        },
    )
    .map_err(anyhow::Error::msg)?;
    let renderer = HttpWebworkRenderer::new(config).map_err(anyhow::Error::msg)?;
    Ok(Arc::new(WebworkAdapter::new(objects, renderer)))
}

/// Reads the deployment-owned HMAC key that validates newly minted Question IDs.
/// The base64url capability remains in the mounted private file and is never
/// emitted in diagnostics or browser data.
fn question_id_issuer_from_env() -> Result<crate::question_publication::HmacQuestionIdIssuer> {
    let path = required_env("PLE_QUESTION_ID_SECRET_FILE")?;
    let encoded = std::fs::read_to_string(path)
        .context("could not read the Question ID secret capability")?;
    let encoded = encoded.trim_end_matches(['\r', '\n']);
    let bytes = URL_SAFE_NO_PAD
        .decode(encoded)
        .context("Question ID secret capability must be unpadded base64url")?;
    let bytes: [u8; 32] = bytes.try_into().map_err(|_| {
        anyhow::anyhow!("Question ID secret capability must contain exactly 32 bytes")
    })?;
    Ok(crate::question_publication::HmacQuestionIdIssuer::new(
        crate::question_publication::QuestionIdSecret::from_bytes(bytes),
    ))
}

/// Constructs the API-owned object reader used to compile answer-free Question
/// Library views. The disposable topology may use its explicitly configured
/// MinIO endpoint; production uses workload identity only.
async fn question_library_object_store_from_env() -> Result<S3ObjectStore> {
    let buckets = BucketNames {
        public_assets: required_env("PLE_PUBLIC_ASSETS_BUCKET")?,
        private_content: required_env("PLE_PRIVATE_CONTENT_BUCKET")?,
        student_records: required_env("PLE_STUDENT_RECORDS_BUCKET")?,
        temp_processing: required_env("PLE_TEMP_PROCESSING_BUCKET")?,
    };
    let client =
        if std::env::var("PLE_STORAGE_TOPOLOGY").ok().as_deref() == Some("disposable-local") {
            minio_client(&EndpointConfig {
                endpoint_url: required_env("PLE_S3_ENDPOINT")?,
                region: required_env("PLE_S3_REGION")?,
                access_key_id: required_env("AWS_ACCESS_KEY_ID")?,
                secret_access_key: required_env("AWS_SECRET_ACCESS_KEY")?,
            })
        } else {
            objects::aws::container_role_client(&objects::aws::ContainerRoleConfig {
                region: required_env("PLE_S3_REGION")?,
            })
            .await
        };
    Ok(S3ObjectStore::new(client, buckets))
}

/// Attests the one worker login without constructing an API router or listener.
// ASVS 8.3.1: the worker's database URL must attest the worker profile before
// it can later claim a typed Job. No Account/session authority is constructed.
pub async fn verify_worker_database_login_from_env() -> Result<()> {
    let database_url = required_env("DATABASE_URL")?;
    let pool = if std::env::var("PLE_STORAGE_TOPOLOGY").ok().as_deref() == Some("disposable-local")
    {
        local_development_pool(
            &database_url,
            ProductionLoginProfile::ImathasQuestionBackendGradingWorker,
        )
    } else {
        production_pool(
            &database_url,
            ProductionLoginProfile::ImathasQuestionBackendGradingWorker,
        )
    }
    .context("could not construct the attested worker database pool")?;
    pool.acquire()
        .await
        .context("the attested worker database pool could not connect")?;
    Ok(())
}

/// Runs the native-PLE-only worker with a separate database capability and
/// restricted private-content object-reader identity.
pub async fn run_native_ple_grading_worker_from_env() -> Result<()> {
    let database_url = required_env("DATABASE_URL")?;
    let pool = if std::env::var("PLE_STORAGE_TOPOLOGY").ok().as_deref() == Some("disposable-local")
    {
        local_development_pool(
            &database_url,
            ProductionLoginProfile::NativePleGradingWorker,
        )
    } else {
        production_pool(
            &database_url,
            ProductionLoginProfile::NativePleGradingWorker,
        )
    }
    .context("could not construct the attested native PLE worker database pool")?;
    pool.acquire()
        .await
        .context("the attested native PLE worker database pool could not connect")?;
    crate::worker::run_native_ple_until_shutdown(
        PostgresNativePleGradingStore::new(pool),
        native_ple_worker_object_store_from_env().await?,
    )
    .await
}

/// Attest only the native-PLE worker's distinct database Service Identity.
pub async fn verify_native_ple_worker_database_login_from_env() -> Result<()> {
    let database_url = required_env("DATABASE_URL")?;
    let pool = if std::env::var("PLE_STORAGE_TOPOLOGY").ok().as_deref() == Some("disposable-local")
    {
        local_development_pool(
            &database_url,
            ProductionLoginProfile::NativePleGradingWorker,
        )
    } else {
        production_pool(
            &database_url,
            ProductionLoginProfile::NativePleGradingWorker,
        )
    }
    .context("could not construct the attested native PLE worker database pool")?;
    pool.acquire()
        .await
        .context("the attested native PLE worker database pool could not connect")?;
    Ok(())
}

/// Runs the renderer-connected WeBWorK worker with only its own database and
/// private Question Source capabilities.
pub async fn run_webwork_grading_worker_from_env() -> Result<()> {
    let database_url = required_env("DATABASE_URL")?;
    let pool = if std::env::var("PLE_STORAGE_TOPOLOGY").ok().as_deref() == Some("disposable-local") {
        local_development_pool(&database_url, ProductionLoginProfile::WebworkGradingWorker)
    } else {
        production_pool(&database_url, ProductionLoginProfile::WebworkGradingWorker)
    }.context("could not construct the attested WeBWorK worker database pool")?;
    pool.acquire().await.context("the attested WeBWorK worker database pool could not connect")?;
    let objects = webwork_worker_object_store_from_env().await?;
    let adapter = webwork_adapter_from_env(objects.clone())?;
    crate::worker::run_webwork_until_shutdown(PostgresWebworkGradingStore::new(pool), objects, adapter).await
}

/// Attest only the WeBWorK worker's distinct database Service Identity.
pub async fn verify_webwork_worker_database_login_from_env() -> Result<()> {
    let database_url = required_env("DATABASE_URL")?;
    let pool = if std::env::var("PLE_STORAGE_TOPOLOGY").ok().as_deref() == Some("disposable-local") {
        local_development_pool(&database_url, ProductionLoginProfile::WebworkGradingWorker)
    } else {
        production_pool(&database_url, ProductionLoginProfile::WebworkGradingWorker)
    }.context("could not construct the attested WeBWorK worker database pool")?;
    pool.acquire().await.context("the attested WeBWorK worker database pool could not connect")?;
    Ok(())
}

async fn webwork_worker_object_store_from_env() -> Result<S3ObjectStore> {
    let buckets = BucketNames {
        public_assets: required_env("PLE_PUBLIC_ASSETS_BUCKET")?,
        private_content: required_env("PLE_PRIVATE_CONTENT_BUCKET")?,
        student_records: required_env("PLE_STUDENT_RECORDS_BUCKET")?,
        temp_processing: required_env("PLE_TEMP_PROCESSING_BUCKET")?,
    };
    let client = minio_client(&EndpointConfig {
        endpoint_url: required_env("PLE_S3_ENDPOINT")?,
        region: required_env("PLE_S3_REGION")?,
        access_key_id: required_env("PLE_WEBWORK_WORKER_S3_ACCESS_KEY_ID")?,
        secret_access_key: required_env("PLE_WEBWORK_WORKER_S3_SECRET_ACCESS_KEY")?,
    });
    Ok(S3ObjectStore::new(client, buckets))
}

async fn native_ple_worker_object_store_from_env() -> Result<S3ObjectStore> {
    let buckets = BucketNames {
        public_assets: required_env("PLE_PUBLIC_ASSETS_BUCKET")?,
        private_content: required_env("PLE_PRIVATE_CONTENT_BUCKET")?,
        student_records: required_env("PLE_STUDENT_RECORDS_BUCKET")?,
        temp_processing: required_env("PLE_TEMP_PROCESSING_BUCKET")?,
    };
    let client =
        if std::env::var("PLE_STORAGE_TOPOLOGY").ok().as_deref() == Some("disposable-local") {
            minio_client(&EndpointConfig {
                endpoint_url: required_env("PLE_S3_ENDPOINT")?,
                region: required_env("PLE_S3_REGION")?,
                access_key_id: required_env("PLE_NATIVE_PLE_WORKER_S3_ACCESS_KEY_ID")?,
                secret_access_key: required_env("PLE_NATIVE_PLE_WORKER_S3_SECRET_ACCESS_KEY")?,
            })
        } else {
            objects::aws::container_role_client(&objects::aws::ContainerRoleConfig {
                region: required_env("PLE_S3_REGION")?,
            })
            .await
        };
    Ok(S3ObjectStore::new(client, buckets))
}

/// Runs the separate publisher with its one database capability and dedicated
/// object-store identity. It constructs neither API/session state nor a listener.
pub async fn publish_one_public_asset_from_env() -> Result<bool> {
    let database_url = required_env("DATABASE_URL")?;
    let pool = if std::env::var("PLE_STORAGE_TOPOLOGY").ok().as_deref() == Some("disposable-local")
    {
        local_development_pool(&database_url, ProductionLoginProfile::PublicAssetPublisher)
    } else {
        production_pool(&database_url, ProductionLoginProfile::PublicAssetPublisher)
    }
    .context("could not construct the attested public-asset publisher database pool")?;
    pool.acquire()
        .await
        .context("the attested public-asset publisher database pool could not connect")?;
    let publication_store = PostgresPublicAssetPublicationStore::new(pool);
    let objects = public_asset_publisher_object_store_from_env().await?;
    crate::public_asset_publisher::publish_one(&publication_store, &objects).await
}

async fn public_asset_publisher_object_store_from_env() -> Result<S3ObjectStore> {
    let buckets = BucketNames {
        public_assets: required_env("PLE_PUBLIC_ASSETS_BUCKET")?,
        private_content: required_env("PLE_PRIVATE_CONTENT_BUCKET")?,
        student_records: required_env("PLE_STUDENT_RECORDS_BUCKET")?,
        temp_processing: required_env("PLE_TEMP_PROCESSING_BUCKET")?,
    };
    let client =
        if std::env::var("PLE_STORAGE_TOPOLOGY").ok().as_deref() == Some("disposable-local") {
            minio_client(&EndpointConfig {
                endpoint_url: required_env("PLE_S3_ENDPOINT")?,
                region: required_env("PLE_S3_REGION")?,
                access_key_id: required_env("PLE_PUBLISHER_S3_ACCESS_KEY_ID")?,
                secret_access_key: required_env("PLE_PUBLISHER_S3_SECRET_ACCESS_KEY")?,
            })
        } else {
            objects::aws::container_role_client(&objects::aws::ContainerRoleConfig {
                region: required_env("PLE_S3_REGION")?,
            })
            .await
        };
    Ok(S3ObjectStore::new(client, buckets))
}

/// The address the binary binds, parsed once at startup.
pub fn bind_address_from_env() -> Result<SocketAddr> {
    std::env::var("PLE_BIND_ADDR")
        .unwrap_or_else(|_| "0.0.0.0:3000".to_string())
        .parse()
        .context("PLE_BIND_ADDR must be a socket address")
}

/// Returns the canonical browser `Host` authority after validating the configured origin.
///
/// Container health checks reach the API over loopback but must still satisfy
/// the same public-host boundary as browser traffic.
pub fn browser_authority_from_env() -> Result<String> {
    Ok(production_browser_boundary_from_env()?
        .authority
        .to_string())
}

fn production_browser_boundary_from_env() -> Result<ProductionBrowserBoundary> {
    ProductionBrowserBoundary::new(Arc::from(required_env("PLE_BROWSER_ORIGIN")?))
        .map_err(anyhow::Error::msg)
}

/// Returns the one deployment-owned, recipient-independent HTTPS signup URL.
fn invitation_signup_url_from_env() -> Result<Arc<str>> {
    let origin = required_env("PLE_BROWSER_ORIGIN")?;
    let parsed = url::Url::parse(&origin).context("PLE_BROWSER_ORIGIN must be an absolute URL")?;
    if parsed.scheme() != "https"
        || parsed.host_str().is_none()
        || parsed.username() != ""
        || parsed.password().is_some()
        || parsed.path() != "/"
        || parsed.query().is_some()
        || parsed.fragment().is_some()
    {
        bail!("PLE_BROWSER_ORIGIN must be a canonical credential-free HTTPS origin");
    }
    Ok(Arc::from(parsed.origin().ascii_serialization()))
}

fn production_session_config() -> SessionConfig {
    SessionConfig::new(
        SessionLifetime::from_seconds(8 * 60 * 60).expect("positive session lifetime"),
        CookieTransport::FirstPartyHttps,
    )
}

fn live_demo_config_from_env() -> Result<Option<SeededDemoConfig>> {
    live_demo_config_from_environment_values(|name| std::env::var(name).ok())
}

fn live_demo_config_from_environment_values(
    mut value_for: impl FnMut(&str) -> Option<String>,
) -> Result<Option<SeededDemoConfig>> {
    // ASVS 2.2.1 and 16.5.2: each trusted deployment value is independently
    // allow-listed as one fixed persona and malformed values affect only it.
    let configured = LIVE_DEMO_ACCOUNT_ID_ENV
        .iter()
        .filter_map(|(persona, name, display_name)| {
            let value = value_for(name)?;
            let id = uuid::Uuid::parse_str(&value).ok()?;
            Some((*persona, AccountId::from_uuid(id), *display_name))
        })
        .collect::<Vec<_>>();
    live_demo_config_from_account_mappings(configured)
}

fn live_demo_config_from_account_mappings(
    configured: Vec<(SeededDemoPersona, AccountId, &'static str)>,
) -> Result<Option<SeededDemoConfig>> {
    let duplicate_accounts = configured.iter().fold(
        std::collections::BTreeMap::<AccountId, usize>::new(),
        |mut counts, (_, account, _)| {
            *counts.entry(*account).or_default() += 1;
            counts
        },
    );
    let accounts = configured
        .into_iter()
        // ASVS 8.2.1-8.2.3: never infer authority from a duplicate mapping.
        .filter(|(_, account, _)| duplicate_accounts[account] == 1)
        .map(|(persona, account, display_name)| {
            SeededDemoAccount::new(persona, account, display_name).map_err(anyhow::Error::msg)
        })
        .collect::<Result<Vec<_>>>()?;
    (!accounts.is_empty())
        .then(|| SeededDemoConfig::new(accounts).map_err(anyhow::Error::msg))
        .transpose()
}

fn required_env(name: &str) -> Result<String> {
    let value = std::env::var(name).with_context(|| format!("{name} must be set"))?;
    if value.is_empty() {
        bail!("{name} must not be empty");
    }
    Ok(value)
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use axum::{
        body::Body,
        http::{Request, StatusCode},
        routing::get,
    };
    use learning_data_access::{
        SessionId, SessionLifetime, SessionRecord, SessionStore, SessionTokenHash, StoreError,
    };
    use question_model::{ProductRole, Timestamp};
    use std::{collections::BTreeMap, sync::Mutex};
    use tower::ServiceExt;

    fn account(value: u128) -> AccountId {
        AccountId::from_uuid(uuid::Uuid::from_u128(value))
    }

    #[test]
    fn duplicate_account_mappings_are_all_omitted_while_valid_mappings_remain() {
        let config = live_demo_config_from_account_mappings(vec![
            (
                SeededDemoPersona::ElenaInstructor,
                account(1),
                "Elena Instructor",
            ),
            (SeededDemoPersona::MaryStudent, account(1), "Mary Student"),
            (
                SeededDemoPersona::MorganSysadmin,
                account(2),
                "Morgan Sysadmin",
            ),
        ])
        .expect("trusted mappings should parse")
        .expect("one unambiguous mapping remains");

        assert_eq!(config.unavailable_account_count(), 4);
    }

    #[test]
    fn only_conflicting_mappings_leave_the_demo_capability_absent() {
        let config = live_demo_config_from_account_mappings(vec![
            (
                SeededDemoPersona::ElenaInstructor,
                account(1),
                "Elena Instructor",
            ),
            (SeededDemoPersona::MaryStudent, account(1), "Mary Student"),
        ])
        .expect("trusted mappings should parse");

        assert!(config.is_none());
    }

    #[test]
    fn missing_or_malformed_persona_values_do_not_remove_valid_demo_personas() {
        let values = BTreeMap::from([
            (
                "PLE_LIVE_DEMO_ELENA_INSTRUCTOR_ACCOUNT_ID",
                uuid::Uuid::from_u128(1).to_string(),
            ),
            (
                "PLE_LIVE_DEMO_MARY_STUDENT_ACCOUNT_ID",
                "not-an-account-id".to_string(),
            ),
            (
                "PLE_LIVE_DEMO_MORGAN_SYSADMIN_ACCOUNT_ID",
                uuid::Uuid::from_u128(2).to_string(),
            ),
        ]);

        let config = live_demo_config_from_environment_values(|name| values.get(name).cloned())
            .expect("independent deployment values should parse")
            .expect("the two valid persona mappings remain available");

        assert_eq!(config.unavailable_account_count(), 3);
    }

    #[derive(Clone, Default)]
    struct MemorySessionStore(Arc<Mutex<BTreeMap<SessionTokenHash, SessionRecord>>>);

    #[async_trait]
    impl SessionStore for MemorySessionStore {
        async fn create_session(
            &self,
            token_hash: SessionTokenHash,
            account: AccountId,
            lifetime: SessionLifetime,
        ) -> Result<SessionRecord, StoreError> {
            let record = SessionRecord {
                id: SessionId::generate()?,
                token_hash,
                account,
                product_role: ProductRole::Student,
                created_at: Timestamp::from_unix_millis(0),
                expires_at: Timestamp::from_unix_millis(i64::from(lifetime.as_seconds()) * 1_000),
            };
            self.0
                .lock()
                .expect("test store lock")
                .insert(token_hash, record.clone());
            Ok(record)
        }

        async fn resolve_session(
            &self,
            token_hash: SessionTokenHash,
        ) -> Result<Option<SessionRecord>, StoreError> {
            Ok(self
                .0
                .lock()
                .expect("test store lock")
                .get(&token_hash)
                .cloned())
        }

        async fn revoke_session(&self, token_hash: SessionTokenHash) -> Result<(), StoreError> {
            self.0.lock().expect("test store lock").remove(&token_hash);
            Ok(())
        }
    }

    #[tokio::test]
    async fn absent_demo_config_leaves_health_and_session_routes_mounted() {
        let sessions = Arc::new(MemorySessionStore::default());
        let session_config = production_session_config();
        let router = Router::new()
            .route("/health", get(|| async { StatusCode::OK }))
            .merge(session_router(Arc::clone(&sessions), session_config))
            .merge(live_demo_router(sessions, None, session_config));

        let health = router
            .clone()
            .oneshot(
                Request::get("/health")
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("response");
        assert_eq!(health.status(), StatusCode::OK);

        let current_session = router
            .clone()
            .oneshot(
                Request::get("/api/auth/session")
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("response");
        assert_eq!(current_session.status(), StatusCode::UNAUTHORIZED);

        let logout = router
            .clone()
            .oneshot(
                Request::post("/api/auth/logout")
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("response");
        assert_eq!(logout.status(), StatusCode::OK);

        let demo = router
            .oneshot(
                Request::get("/api/auth/live-demo/accounts")
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("response");
        assert_eq!(demo.status(), StatusCode::NOT_FOUND);
    }
}

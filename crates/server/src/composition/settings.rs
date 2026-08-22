//! Fail-closed production configuration and secret ingestion.

use super::backend::ConfiguredImathas;
use super::*;

pub(super) struct StorageSettings {
    pub(super) runtime: StorageRuntime,
    pub(super) database_url: String,
    pub(super) question_id_secret: Option<[u8; 32]>,
    pub(super) object_connection: ObjectStorageConnection,
    pub(super) public_assets_bucket: String,
    pub(super) private_content_bucket: String,
    pub(super) student_records_bucket: String,
    pub(super) temp_processing_bucket: String,
}

#[derive(Debug, Clone)]
pub(super) enum ObjectStorageConnection {
    LocalMinio(objects::minio::EndpointConfig),
    AwsContainerRole {
        client: objects::aws::ContainerRoleConfig,
        kms_keys: objects::s3::KmsKeyNames,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum ProcessRole {
    Api,
    Worker,
    PublicAssetPublisher,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum StorageTopology {
    AwsWorkload,
    #[cfg_attr(not(feature = "local-disposable-storage"), allow(dead_code))]
    DisposableLocal,
}

impl StorageTopology {
    const ENVIRONMENT_VARIABLE: &str = "PLE_STORAGE_TOPOLOGY";

    pub(super) fn from_env() -> Result<Self> {
        match std::env::var(Self::ENVIRONMENT_VARIABLE) {
            Ok(value) => Self::from_value(Some(&value)),
            Err(std::env::VarError::NotPresent) => Self::from_value(None),
            Err(std::env::VarError::NotUnicode(_)) => {
                bail!("PLE_STORAGE_TOPOLOGY must be valid UTF-8")
            }
        }
    }

    pub(super) fn from_value(value: Option<&str>) -> Result<Self> {
        match value {
            None | Some("aws-workload") => Ok(Self::AwsWorkload),
            Some("disposable-local") => {
                #[cfg(feature = "local-disposable-storage")]
                {
                    Ok(Self::DisposableLocal)
                }
                #[cfg(not(feature = "local-disposable-storage"))]
                {
                    bail!(
                        "PLE_STORAGE_TOPOLOGY=disposable-local requires a binary built with local-disposable-storage"
                    )
                }
            }
            Some(value) => bail!(
                "PLE_STORAGE_TOPOLOGY must be unset, aws-workload, or disposable-local; got {value:?}"
            ),
        }
    }
}

/// Storage construction has two independent axes. Process role chooses the
/// least-privilege database identity, while topology chooses AWS or the
/// explicitly feature-gated disposable PostgreSQL/MinIO backing services.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct StorageRuntime {
    pub(super) role: ProcessRole,
    pub(super) topology: StorageTopology,
}

impl StorageRuntime {
    pub(super) fn api_from_env() -> Result<Self> {
        Ok(Self {
            role: ProcessRole::Api,
            topology: StorageTopology::from_env()?,
        })
    }

    pub(super) fn worker_from_env() -> Result<Self> {
        Ok(Self {
            role: ProcessRole::Worker,
            topology: StorageTopology::from_env()?,
        })
    }

    pub(super) fn publisher_from_env() -> Result<Self> {
        let topology = StorageTopology::from_env()?;
        if topology != StorageTopology::AwsWorkload {
            bail!("public-asset publisher requires PLE_STORAGE_TOPOLOGY=aws-workload");
        }
        Ok(Self {
            role: ProcessRole::PublicAssetPublisher,
            topology,
        })
    }

    pub(super) fn database_variable(self) -> &'static str {
        match (self.role, self.topology) {
            (ProcessRole::Worker, StorageTopology::AwsWorkload) => "PLE_WORKER_DATABASE_URL",
            (ProcessRole::PublicAssetPublisher, StorageTopology::AwsWorkload) => {
                "PLE_PUBLISHER_DATABASE_URL"
            }
            (ProcessRole::Api, _) | (ProcessRole::Worker, StorageTopology::DisposableLocal) => {
                "DATABASE_URL"
            }
            (ProcessRole::PublicAssetPublisher, StorageTopology::DisposableLocal) => {
                unreachable!("publisher rejects disposable local storage")
            }
        }
    }

    #[cfg(test)]
    pub(super) fn uses_disposable_local_storage(self) -> bool {
        self.topology == StorageTopology::DisposableLocal
    }
}

impl StorageSettings {
    pub(super) fn from_env(runtime: StorageRuntime) -> Result<Self> {
        if runtime.role == ProcessRole::PublicAssetPublisher
            && runtime.topology != StorageTopology::AwsWorkload
        {
            bail!("public-asset publisher requires PLE_STORAGE_TOPOLOGY=aws-workload");
        }
        let question_id_secret = match (runtime.role, runtime.topology) {
            (ProcessRole::Api, StorageTopology::DisposableLocal) => {
                let path = required_env("PLE_QUESTION_ID_SECRET_FILE")?;
                let encoded = read_secret_file(&path, "PLE_QUESTION_ID_SECRET_FILE")?;
                Some(parse_secret32("PLE_QUESTION_ID_SECRET_FILE", &encoded)?)
            }
            (ProcessRole::Api, StorageTopology::AwsWorkload) => {
                Some(decode_secret32("PLE_QUESTION_ID_SECRET")?)
            }
            (ProcessRole::Worker, _) | (ProcessRole::PublicAssetPublisher, _) => None,
        };
        let region = required_env("PLE_S3_REGION")?;
        let object_connection = match runtime.topology {
            StorageTopology::DisposableLocal => {
                ObjectStorageConnection::LocalMinio(objects::minio::EndpointConfig {
                    endpoint_url: required_env("PLE_S3_ENDPOINT")?,
                    region,
                    access_key_id: required_env("AWS_ACCESS_KEY_ID")?,
                    secret_access_key: required_env("AWS_SECRET_ACCESS_KEY")?,
                })
            }
            StorageTopology::AwsWorkload => {
                reject_present_env("PLE_S3_ENDPOINT")?;
                reject_present_env("AWS_ACCESS_KEY_ID")?;
                reject_present_env("AWS_SECRET_ACCESS_KEY")?;
                reject_present_env("AWS_SESSION_TOKEN")?;
                ObjectStorageConnection::AwsContainerRole {
                    client: objects::aws::ContainerRoleConfig { region },
                    kms_keys: objects::s3::KmsKeyNames::new(
                        required_env("PLE_PUBLIC_ASSETS_KMS_KEY_ARN")?,
                        required_env("PLE_PRIVATE_CONTENT_KMS_KEY_ARN")?,
                        required_env("PLE_STUDENT_RECORDS_KMS_KEY_ARN")?,
                        required_env("PLE_TEMP_PROCESSING_KMS_KEY_ARN")?,
                    )
                    .map_err(anyhow::Error::msg)?,
                }
            }
        };
        let public_assets_bucket = required_env("PLE_PUBLIC_ASSETS_BUCKET")?;
        let private_content_bucket = required_env("PLE_PRIVATE_CONTENT_BUCKET")?;
        let student_records_bucket = required_env("PLE_STUDENT_RECORDS_BUCKET")?;
        let temp_processing_bucket = required_env("PLE_TEMP_PROCESSING_BUCKET")?;
        validate_object_bucket_names(
            &public_assets_bucket,
            &private_content_bucket,
            &student_records_bucket,
            &temp_processing_bucket,
        )?;
        Ok(Self {
            runtime,
            database_url: required_env(runtime.database_variable())?,
            question_id_secret,
            object_connection,
            public_assets_bucket,
            private_content_bucket,
            student_records_bucket,
            temp_processing_bucket,
        })
    }
}

/// Rejects deployments which point two security domains at one physical
/// bucket. KMS separation cannot compensate for this collapse: S3 policies,
/// object retention, CDN access, and IAM grants are bucket-scoped.
pub(super) fn validate_object_bucket_names(
    public_assets: &str,
    private_content: &str,
    student_records: &str,
    temp_processing: &str,
) -> Result<()> {
    let names = [
        public_assets,
        private_content,
        student_records,
        temp_processing,
    ];
    if names.iter().any(|name| name.trim() != *name)
        || names
            .iter()
            .enumerate()
            .any(|(index, name)| names.iter().skip(index + 1).any(|other| name == other))
    {
        bail!("object security domains require four distinct, trimmed bucket names");
    }
    Ok(())
}

pub(super) struct LazyStorageDependencies {
    pub(super) store: Arc<PostgresStore>,
    pub(super) objects: Arc<objects::s3::S3ObjectStore>,
    pub(super) pool: Pool,
    pub(super) object_client: objects::minio::S3Client,
}

impl LazyStorageDependencies {
    pub(super) async fn from_settings(settings: &StorageSettings) -> Result<Self> {
        let pool = match (settings.runtime.role, settings.runtime.topology) {
            (_, StorageTopology::DisposableLocal) => lazy_pool(&settings.database_url),
            (ProcessRole::Api, StorageTopology::AwsWorkload) => {
                production_pool(&settings.database_url, ProductionLoginProfile::Api)
            }
            (ProcessRole::Worker, StorageTopology::AwsWorkload) => {
                production_pool(&settings.database_url, ProductionLoginProfile::Worker)
            }
            (ProcessRole::PublicAssetPublisher, StorageTopology::AwsWorkload) => {
                production_pool(&settings.database_url, ProductionLoginProfile::Publisher)
            }
        }
        .map_err(|_| anyhow::anyhow!("database connection configuration was rejected"))?;
        let store = Arc::new(match settings.question_id_secret {
            Some(secret) => PostgresStore::with_question_id_secret(pool.clone(), secret),
            None => PostgresStore::new(pool.clone()),
        });
        let buckets = objects::s3::BucketNames {
            public_assets: settings.public_assets_bucket.clone(),
            private_content: settings.private_content_bucket.clone(),
            student_records: settings.student_records_bucket.clone(),
            temp_processing: settings.temp_processing_bucket.clone(),
        };
        let (object_client, objects) = match &settings.object_connection {
            ObjectStorageConnection::LocalMinio(settings) => {
                let client = objects::minio::client(settings);
                let store = objects::s3::S3ObjectStore::new(client.clone(), buckets);
                (client, store)
            }
            ObjectStorageConnection::AwsContainerRole { client, kms_keys } => {
                let client = objects::aws::container_role_client(client).await;
                let store = objects::s3::S3ObjectStore::new_kms_encrypted(
                    client.clone(),
                    buckets,
                    kms_keys.clone(),
                );
                (client, store)
            }
        };
        let objects = Arc::new(objects);
        Ok(Self {
            store,
            objects,
            pool,
            object_client,
        })
    }
}

/// Object-storage and database dependencies for the publisher's distinct
/// process identity. It never constructs a general `PostgresStore`.
pub(super) struct PublisherStorageDependencies {
    pub(super) store: Arc<learning_data_access::postgres::PostgresPublicAssetPublisherStore>,
    pub(super) objects: Arc<objects::s3::S3ObjectStore>,
    pub(super) pool: Pool,
}

impl PublisherStorageDependencies {
    pub(super) async fn from_settings(settings: &StorageSettings) -> Result<Self> {
        if settings.runtime.role != ProcessRole::PublicAssetPublisher {
            bail!("publisher dependencies require the publisher runtime");
        }
        let pool = production_pool(&settings.database_url, ProductionLoginProfile::Publisher)
            .map_err(|_| anyhow::anyhow!("database connection configuration was rejected"))?;
        let buckets = objects::s3::BucketNames {
            public_assets: settings.public_assets_bucket.clone(),
            private_content: settings.private_content_bucket.clone(),
            student_records: settings.student_records_bucket.clone(),
            temp_processing: settings.temp_processing_bucket.clone(),
        };
        let objects = match &settings.object_connection {
            ObjectStorageConnection::AwsContainerRole { client, kms_keys } => {
                let client = objects::aws::container_role_client(client).await;
                objects::s3::S3ObjectStore::new_kms_encrypted(client, buckets, kms_keys.clone())
            }
            ObjectStorageConnection::LocalMinio(_) => {
                bail!("publisher runtime must use workload-owned AWS credentials");
            }
        };
        Ok(Self {
            store: Arc::new(
                learning_data_access::postgres::PostgresPublicAssetPublisherStore::new(
                    pool.clone(),
                ),
            ),
            objects: Arc::new(objects),
            pool,
        })
    }
}

pub(super) struct ProductionSettings {
    pub(super) storage: StorageSettings,
    pub(super) public_asset_base_url: PublicAssetBaseUrl,
    pub(super) webwork: Option<WebworkRendererSettings>,
    pub(super) imathas_provider_key: Option<String>,
    pub(super) qti_runtime_enabled: Option<String>,
    pub(super) grader_database_url: Option<String>,
    pub(super) enrollment_secret: Option<EnrollmentSecretSettings>,
    pub(super) enrollment_email: Option<EnrollmentEmailSettings>,
    pub(super) webauthn: crate::auth::PasswordlessWebauthn,
    pub(super) browser_boundary: crate::auth::ProductionBrowserBoundary,
    pub(super) client_address_policy: crate::auth::ClientAddressPolicy,
    pub(super) live_demo_selector: Option<crate::auth::SeededAccountSelectorConfig>,
    pub(super) live_demo_sysadmin_ownership: Option<crate::auth::SeededSysadminOwnershipConfig>,
}

pub(super) struct EnrollmentEmailSettings {
    pub(super) smtp_relay: String,
    pub(super) smtp_port: u16,
    pub(super) smtp_tls_mode: crate::course::SmtpTlsMode,
    pub(super) smtp_username: String,
    pub(super) smtp_password_file: String,
    pub(super) smtp_from: String,
    pub(super) public_app_base_url: String,
}

pub(super) struct EnrollmentSecretSettings {
    pub(super) invitation_token_secret_file: String,
}

pub(super) struct WebworkRendererSettings {
    pub(super) webwork_renderer_base_url: String,
    pub(super) webwork_request_timeout_seconds: u64,
    pub(super) webwork_max_response_bytes: usize,
    pub(super) webwork_renderer_id: String,
    pub(super) webwork_renderer_version: String,
}

impl ProductionSettings {
    pub(super) fn from_env(runtime: StorageRuntime) -> Result<Self> {
        if runtime.role != ProcessRole::Api {
            bail!("persistent API dependencies require the API process role");
        }
        let public_asset_base_url =
            PublicAssetBaseUrl::new(required_env("PLE_PUBLIC_ASSET_BASE_URL")?)
                .map_err(|_| anyhow::anyhow!("PLE_PUBLIC_ASSET_BASE_URL is invalid"))?;
        let client_address_policy = crate::auth::ClientAddressPolicy::behind_trusted_proxies(
            &required_env("PLE_TRUSTED_PROXY_CIDRS")?,
        )
        .map_err(anyhow::Error::msg)?;
        let webauthn_origin = required_env("PLE_WEBAUTHN_ORIGIN")?;
        let browser_boundary = browser_boundary_for(&webauthn_origin)?;
        let live_demo_selector = live_demo_selector_from_env(&webauthn_origin)?;
        let live_demo_sysadmin_ownership = live_demo_sysadmin_ownership_from_env(&webauthn_origin)?;
        validate_live_demo_identity_config(
            live_demo_selector.as_ref(),
            live_demo_sysadmin_ownership.as_ref(),
        )?;
        Ok(Self {
            storage: StorageSettings::from_env(runtime)?,
            public_asset_base_url,
            webwork: WebworkRendererSettings::from_env()?,
            imathas_provider_key: std::env::var("PLE_IMATHAS_PROVIDER_KEY").ok(),
            qti_runtime_enabled: std::env::var("PLE_QTI_RUNTIME_ENABLED").ok(),
            grader_database_url: std::env::var("PLE_GRADER_DATABASE_URL").ok(),
            enrollment_secret: EnrollmentSecretSettings::from_env()?,
            enrollment_email: EnrollmentEmailSettings::from_env()?,
            webauthn: crate::auth::PasswordlessWebauthn::new(
                &required_env("PLE_WEBAUTHN_RP_ID")?,
                &webauthn_origin,
                &required_env("PLE_WEBAUTHN_RP_NAME")?,
            )
            .map_err(anyhow::Error::msg)?,
            browser_boundary,
            client_address_policy,
            live_demo_selector,
            live_demo_sysadmin_ownership,
        })
    }

    pub(super) fn webwork_renderer(&self) -> Result<Option<HttpWebworkRenderer>> {
        self.webwork
            .as_ref()
            .map(WebworkRendererSettings::renderer)
            .transpose()
    }

    pub(super) fn imathas(
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
        let provider_timeout = std::time::Duration::from_secs(timeout);
        let external_tool_timing =
            crate::imathas_backend::ExternalToolTiming::from_provider_timeout(provider_timeout)
                .map_err(anyhow::Error::msg)?;
        let transport_bytes = positive_usize_env("PLE_IMATHAS_MAX_TRANSPORT_BYTES")?;
        let snapshot_bytes = positive_usize_env("PLE_IMATHAS_MAX_SNAPSHOT_BYTES")?;
        let result_bytes = positive_usize_env("PLE_IMATHAS_MAX_RESULT_BYTES")?;
        let ttl = positive_u64_env("PLE_IMATHAS_LAUNCH_TTL_MILLIS")?;
        let launch_state = decode_secret32("PLE_IMATHAS_LAUNCH_STATE_SECRET")?;
        let correlation_secret = decode_secret32("PLE_IMATHAS_CORRELATION_SECRET")?;
        let launch_signing = decode_secret32("PLE_IMATHAS_LAUNCH_SIGNING_SECRET")?;
        let result_verify = decode_secret32("PLE_IMATHAS_RESULT_VERIFICATION_SECRET")?;
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
        let provider_config =
            ContractedScoredEmbedConfig::new(profile, launch_signing, result_verify, ttl)
                .map_err(|_| anyhow::anyhow!("PLE_IMATHAS contracted profile is invalid"))?
                .with_limits(snapshot_bytes, result_bytes)
                .map_err(|_| anyhow::anyhow!("PLE_IMATHAS limits are invalid"))?;
        let transport_config =
            HttpContractedScoredEmbedConfig::https(&base, provider_timeout, transport_bytes)
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
            external_tool_timing,
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
    pub(super) fn grader_database_url(&self) -> Result<&str> {
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
    pub(super) fn qti_runtime_enabled(&self) -> Result<bool> {
        match self.qti_runtime_enabled.as_deref() {
            None => Ok(false),
            Some("1") => Ok(true),
            Some(_) => bail!("PLE_QTI_RUNTIME_ENABLED must be exactly 1 when set"),
        }
    }
}

pub(super) fn browser_boundary_for(origin: &str) -> Result<crate::auth::ProductionBrowserBoundary> {
    crate::auth::ProductionBrowserBoundary::new(Arc::from(origin.to_string()))
        .map_err(anyhow::Error::msg)
}

const LIVE_DEMO_SELECTOR_USER_ID_ENV: [&str; 4] = [
    "PLE_LIVE_DEMO_ELENA_INSTRUCTOR_USER_ID",
    "PLE_LIVE_DEMO_MARY_STUDENT_USER_ID",
    "PLE_LIVE_DEMO_JACK_STUDENT_USER_ID",
    "PLE_LIVE_DEMO_AVERY_STUDENT_USER_ID",
];

const LIVE_DEMO_SYSADMIN_USER_ID_ENV: &str = "PLE_LIVE_DEMO_SYSADMIN_USER_ID";
const LIVE_DEMO_SYSADMIN_CLAIM_CONTEXT_FILE_ENV: &str = "PLE_LIVE_DEMO_SYSADMIN_CLAIM_CONTEXT_FILE";

fn live_demo_selector_from_env(
    origin: &str,
) -> Result<Option<crate::auth::SeededAccountSelectorConfig>> {
    let values = LIVE_DEMO_SELECTOR_USER_ID_ENV.map(std::env::var);
    if values.iter().all(Result::is_err) {
        return Ok(None);
    }
    let users = values
        .into_iter()
        .map(|value| {
            let value = value.map_err(|_| {
                anyhow::anyhow!("live-demo selector requires all four configured account IDs")
            })?;
            let uuid = uuid::Uuid::parse_str(&value)
                .map_err(|_| anyhow::anyhow!("live-demo selector account ID must be a UUID"))?;
            Ok(question_model::UserId::from_uuid(uuid))
        })
        .collect::<Result<Vec<_>>>()?
        .try_into()
        .expect("four configured live-demo account IDs");
    crate::auth::SeededAccountSelectorConfig::new(Arc::from(origin.to_string()), users)
        .map(Some)
        .map_err(anyhow::Error::msg)
}

#[derive(serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct SeededSysadminClaimContextFile {
    installation_generation: String,
    sysadmin_user_id: String,
    ownership_proof: String,
}

pub(super) fn live_demo_sysadmin_ownership_from_env(
    origin: &str,
) -> Result<Option<crate::auth::SeededSysadminOwnershipConfig>> {
    let (configured_user, path) = match (
        std::env::var_os(LIVE_DEMO_SYSADMIN_USER_ID_ENV),
        std::env::var_os(LIVE_DEMO_SYSADMIN_CLAIM_CONTEXT_FILE_ENV),
    ) {
        (None, None) => return Ok(None),
        (Some(_), None) => {
            bail!(
                "{LIVE_DEMO_SYSADMIN_CLAIM_CONTEXT_FILE_ENV} must be set when {LIVE_DEMO_SYSADMIN_USER_ID_ENV} is set"
            );
        }
        (None, Some(_)) => {
            bail!(
                "{LIVE_DEMO_SYSADMIN_USER_ID_ENV} must be set when {LIVE_DEMO_SYSADMIN_CLAIM_CONTEXT_FILE_ENV} is set"
            );
        }
        (Some(user), Some(path)) => (user, path),
    };
    let path = path.into_string().map_err(|_| {
        anyhow::anyhow!("{LIVE_DEMO_SYSADMIN_CLAIM_CONTEXT_FILE_ENV} must be valid Unicode")
    })?;
    let configured_user = configured_user.into_string().map_err(|_| {
        anyhow::anyhow!("{LIVE_DEMO_SYSADMIN_USER_ID_ENV} must be a canonical UUID")
    })?;
    let configured_user_uuid = uuid::Uuid::parse_str(&configured_user).map_err(|_| {
        anyhow::anyhow!("{LIVE_DEMO_SYSADMIN_USER_ID_ENV} must be a canonical UUID")
    })?;
    if configured_user_uuid.to_string() != configured_user {
        bail!("{LIVE_DEMO_SYSADMIN_USER_ID_ENV} must be a canonical UUID");
    }
    let value = read_secret_file(&path, LIVE_DEMO_SYSADMIN_CLAIM_CONTEXT_FILE_ENV)?;
    let context: SeededSysadminClaimContextFile = serde_json::from_str(&value).map_err(|_| {
        anyhow::anyhow!(
            "PLE_LIVE_DEMO_SYSADMIN_CLAIM_CONTEXT_FILE must be canonical claim context JSON"
        )
    })?;
    if serde_json::to_string(&context).expect("claim context serializes") != value {
        bail!("PLE_LIVE_DEMO_SYSADMIN_CLAIM_CONTEXT_FILE must be canonical claim context JSON");
    }
    let installation_generation = uuid::Uuid::parse_str(&context.installation_generation)
        .map_err(|_| anyhow::anyhow!("live-demo claim installationGeneration must be a UUID"))?;
    if installation_generation.to_string() != context.installation_generation {
        bail!("live-demo claim installationGeneration must be a canonical UUID");
    }
    let sysadmin_user = uuid::Uuid::parse_str(&context.sysadmin_user_id)
        .map_err(|_| anyhow::anyhow!("live-demo claim sysadminUserId must be a UUID"))?;
    if sysadmin_user.to_string() != context.sysadmin_user_id {
        bail!("live-demo claim sysadminUserId must be a canonical UUID");
    }
    if configured_user_uuid != sysadmin_user {
        bail!("{LIVE_DEMO_SYSADMIN_USER_ID_ENV} must equal live-demo claim sysadminUserId");
    }
    let ownership_proof =
        parse_secret32("live-demo claim ownershipProof", &context.ownership_proof)?;
    crate::auth::SeededSysadminOwnershipConfig::new(
        Arc::from(origin.to_string()),
        installation_generation,
        question_model::UserId::from_uuid(sysadmin_user),
        ownership_proof,
    )
    .map(Some)
    .map_err(anyhow::Error::msg)
}

pub(super) fn validate_live_demo_identity_config(
    selector: Option<&crate::auth::SeededAccountSelectorConfig>,
    ownership: Option<&crate::auth::SeededSysadminOwnershipConfig>,
) -> Result<()> {
    if selector
        .is_some_and(|selector| ownership.is_some_and(|claim| selector.contains_user(claim.user())))
    {
        bail!("live-demo Sysadmin claim user must differ from selector accounts");
    }
    Ok(())
}

impl EnrollmentEmailSettings {
    const ENV_NAMES: [&'static str; 7] = [
        "PLE_SMTP_RELAY",
        "PLE_SMTP_PORT",
        "PLE_SMTP_TLS_MODE",
        "PLE_SMTP_USERNAME",
        "PLE_SMTP_PASSWORD_FILE",
        "PLE_SMTP_FROM",
        "PLE_PUBLIC_APP_BASE_URL",
    ];

    fn from_env() -> Result<Option<Self>> {
        if !Self::ENV_NAMES
            .iter()
            .any(|name| std::env::var_os(name).is_some())
        {
            return Ok(None);
        }
        let smtp_port = required_env("PLE_SMTP_PORT")?
            .parse::<u16>()
            .ok()
            .filter(|port| *port > 0)
            .ok_or_else(|| {
                anyhow::anyhow!("PLE_SMTP_PORT must be an integer from 1 through 65535")
            })?;
        Ok(Some(Self {
            smtp_relay: required_env("PLE_SMTP_RELAY")?,
            smtp_port,
            smtp_tls_mode: match required_env("PLE_SMTP_TLS_MODE")?.as_str() {
                "starttls" => crate::course::SmtpTlsMode::StartTls,
                "implicit-tls" => crate::course::SmtpTlsMode::ImplicitTls,
                _ => {
                    bail!("PLE_SMTP_TLS_MODE must be exactly starttls or implicit-tls")
                }
            },
            smtp_username: required_env("PLE_SMTP_USERNAME")?,
            smtp_password_file: required_env("PLE_SMTP_PASSWORD_FILE")?,
            smtp_from: required_env("PLE_SMTP_FROM")?,
            public_app_base_url: required_env("PLE_PUBLIC_APP_BASE_URL")?,
        }))
    }

    pub(super) fn delivery(&self) -> Result<Arc<crate::course::SmtpCourseInvitationDelivery>> {
        let password = read_secret_file(&self.smtp_password_file, "PLE_SMTP_PASSWORD_FILE")?;
        let delivery = crate::course::SmtpCourseInvitationDelivery::new(
            crate::course::SmtpCourseInvitationDeliveryConfig {
                relay: self.smtp_relay.clone(),
                port: self.smtp_port,
                tls_mode: self.smtp_tls_mode,
                username: self.smtp_username.clone(),
                password,
                from: self.smtp_from.clone(),
                public_app_base_url: self.public_app_base_url.clone(),
            },
        )
        .map_err(|_| anyhow::anyhow!("PLE SMTP invitation configuration is invalid"))?;
        Ok(Arc::new(delivery))
    }
}

/// Constructs the capability for the dedicated invitation-delivery process.
/// An incomplete SMTP configuration leaves that process alive but deliberately
/// unable to claim a delivery.
pub(super) fn invitation_delivery_worker_from_env() -> Result<
    Option<(
        crate::course::CourseInvitationIssuer,
        Arc<dyn crate::course::CourseInvitationDelivery>,
    )>,
> {
    let Some(email) = EnrollmentEmailSettings::from_env()? else {
        return Ok(None);
    };
    let secret = EnrollmentSecretSettings::from_env()?.ok_or_else(|| {
        anyhow::anyhow!("PLE_INVITATION_TOKEN_SECRET_FILE must be set when PLE SMTP is configured")
    })?;
    let (issuer, _) = secret.issuers()?;
    let delivery = email.delivery()? as Arc<dyn crate::course::CourseInvitationDelivery>;
    Ok(Some((issuer, delivery)))
}

pub(super) fn invitation_delivery_worker_database_url_from_env() -> Result<String> {
    required_env("PLE_INVITATION_DELIVERY_DATABASE_URL")
}

impl EnrollmentSecretSettings {
    fn from_env() -> Result<Option<Self>> {
        if std::env::var_os("PLE_INVITATION_TOKEN_SECRET_FILE").is_none() {
            return Ok(None);
        }
        Ok(Some(Self {
            invitation_token_secret_file: required_env("PLE_INVITATION_TOKEN_SECRET_FILE")?,
        }))
    }

    pub(super) fn issuers(
        &self,
    ) -> Result<(
        crate::course::CourseInvitationIssuer,
        crate::auth::PasswordlessRateLimitIssuer,
    )> {
        let issuer_secret = parse_secret32(
            "PLE_INVITATION_TOKEN_SECRET_FILE",
            &read_secret_file(
                &self.invitation_token_secret_file,
                "PLE_INVITATION_TOKEN_SECRET_FILE",
            )?,
        )?;
        Ok((
            crate::course::CourseInvitationIssuer::from_server_secret(issuer_secret),
            crate::auth::PasswordlessRateLimitIssuer::from_server_secret(issuer_secret),
        ))
    }
}

impl WebworkRendererSettings {
    pub(super) const ENV_NAMES: [&'static str; 5] = [
        "PLE_WEBWORK_RENDERER_BASE_URL",
        "PLE_WEBWORK_REQUEST_TIMEOUT_SECONDS",
        "PLE_WEBWORK_MAX_RESPONSE_BYTES",
        "PLE_WEBWORK_RENDERER_ID",
        "PLE_WEBWORK_RENDERER_VERSION",
    ];

    pub(super) fn from_env() -> Result<Option<Self>> {
        if !Self::ENV_NAMES
            .iter()
            .any(|name| std::env::var_os(name).is_some())
        {
            return Ok(None);
        }
        Ok(Some(Self {
            webwork_renderer_base_url: required_env("PLE_WEBWORK_RENDERER_BASE_URL")?,
            webwork_request_timeout_seconds: positive_u64_env(
                "PLE_WEBWORK_REQUEST_TIMEOUT_SECONDS",
            )?,
            webwork_max_response_bytes: positive_usize_env("PLE_WEBWORK_MAX_RESPONSE_BYTES")?,
            webwork_renderer_id: required_env("PLE_WEBWORK_RENDERER_ID")?,
            webwork_renderer_version: required_env("PLE_WEBWORK_RENDERER_VERSION")?,
        }))
    }

    pub(super) fn renderer(&self) -> Result<HttpWebworkRenderer> {
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
        )
        .context("PLE_WEBWORK renderer configuration is invalid")?;
        HttpWebworkRenderer::new(settings).context("PLE_WEBWORK renderer configuration is invalid")
    }
}

fn decode_secret32(name: &str) -> Result<[u8; 32]> {
    let value = required_env(name)?;
    parse_secret32(name, &value)
}

pub(super) fn parse_secret32(name: &str, value: &str) -> Result<[u8; 32]> {
    let bytes = URL_SAFE_NO_PAD
        .decode(value.as_bytes())
        .with_context(|| format!("{name} must be canonical base64url"))?;
    if bytes.len() != 32 || URL_SAFE_NO_PAD.encode(&bytes) != value {
        bail!("{name} must be canonical 32-byte base64url");
    }
    Ok(bytes.try_into().expect("checked length"))
}

pub(super) fn required_env(name: &str) -> Result<String> {
    let value = std::env::var(name).with_context(|| format!("{name} must be set"))?;
    if value.trim().is_empty() {
        bail!("{name} must not be empty");
    }
    Ok(value)
}

fn reject_present_env(name: &str) -> Result<()> {
    match std::env::var(name) {
        Err(std::env::VarError::NotPresent) => Ok(()),
        Ok(_) | Err(std::env::VarError::NotUnicode(_)) => {
            bail!("{name} is forbidden in production")
        }
    }
}

fn read_secret_file(path: &str, name: &str) -> Result<String> {
    #[cfg(unix)]
    {
        read_secret_file_unix(path, name)
    }
    #[cfg(not(unix))]
    read_secret_file_portable(path, name)
}

#[cfg(unix)]
fn read_secret_file_unix(path: &str, name: &str) -> Result<String> {
    use std::io::Read as _;
    use std::os::unix::fs::{OpenOptionsExt as _, PermissionsExt as _};

    const MAX_SECRET_BYTES: u64 = 4096;
    // O_NOFOLLOW makes the open itself reject a symlink, closing the race
    // between metadata inspection and reading the mounted secret.
    let mut file = std::fs::OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_NOFOLLOW)
        .open(path)
        .with_context(|| format!("{name} could not be inspected"))?;
    let metadata = file
        .metadata()
        .with_context(|| format!("{name} could not be inspected"))?;
    if !metadata.is_file() || metadata.len() > MAX_SECRET_BYTES {
        bail!("{name} must name a non-empty bounded regular file");
    }
    if metadata.permissions().mode() & 0o777 != 0o600 {
        bail!("{name} must have Unix mode 0600");
    }
    let mut password = String::new();
    file.read_to_string(&mut password)
        .with_context(|| format!("{name} could not be read"))?;
    normalize_secret_file(password, name)
}

#[cfg(not(unix))]
fn read_secret_file_portable(path: &str, name: &str) -> Result<String> {
    const MAX_SECRET_BYTES: u64 = 4096;
    // Non-Unix platforms lack a portable O_NOFOLLOW equivalent in std.  They
    // still reject a visible link and every non-regular or oversized target.
    let metadata = std::fs::symlink_metadata(path)
        .with_context(|| format!("{name} could not be inspected"))?;
    if metadata.file_type().is_symlink() || !metadata.is_file() || metadata.len() > MAX_SECRET_BYTES
    {
        bail!("{name} must name a non-empty bounded regular file");
    }
    let password =
        std::fs::read_to_string(path).with_context(|| format!("{name} could not be read"))?;
    normalize_secret_file(password, name)
}

fn normalize_secret_file(value: String, name: &str) -> Result<String> {
    let value = value.trim_end_matches(['\r', '\n']).to_string();
    if value.trim().is_empty() {
        bail!("{name} must not be empty");
    }
    Ok(value)
}

fn positive_u64_env(name: &str) -> Result<u64> {
    let value = required_env(name)?;
    parse_positive_u64(name, &value)
}

fn positive_usize_env(name: &str) -> Result<usize> {
    let value = required_env(name)?;
    parse_positive_usize(name, &value)
}

pub(super) fn parse_positive_u64(name: &str, value: &str) -> Result<u64> {
    let parsed = value
        .parse::<u64>()
        .with_context(|| format!("{name} must be a positive whole number"))?;
    if parsed == 0 {
        bail!("{name} must be a positive whole number");
    }
    Ok(parsed)
}

pub(super) fn parse_positive_usize(name: &str, value: &str) -> Result<usize> {
    let parsed = value
        .parse::<usize>()
        .with_context(|| format!("{name} must be a positive whole number"))?;
    if parsed == 0 {
        bail!("{name} must be a positive whole number");
    }
    Ok(parsed)
}

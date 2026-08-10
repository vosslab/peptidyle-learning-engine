//! Fail-closed production configuration and secret ingestion.

use super::backend::ConfiguredImathas;
use super::*;

pub(super) struct StorageSettings {
    pub(super) database_url: String,
    pub(super) s3_endpoint: String,
    pub(super) s3_region: String,
    pub(super) access_key_id: String,
    pub(super) secret_access_key: String,
    pub(super) content_bucket: String,
    pub(super) student_records_bucket: String,
    pub(super) temp_processing_bucket: String,
}

impl StorageSettings {
    pub(super) fn from_env() -> Result<Self> {
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

pub(super) struct LazyStorageDependencies {
    pub(super) store: Arc<PostgresStore>,
    pub(super) objects: Arc<objects::s3::S3ObjectStore>,
    pub(super) pool: Pool,
    pub(super) object_client: objects::minio::S3Client,
}

impl LazyStorageDependencies {
    pub(super) fn from_settings(settings: &StorageSettings) -> Result<Self> {
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

pub(super) struct ProductionSettings {
    pub(super) storage: StorageSettings,
    pub(super) public_asset_base_url: String,
    pub(super) webwork: Option<WebworkRendererSettings>,
    pub(super) imathas_provider_key: Option<String>,
    pub(super) qti_runtime_enabled: Option<String>,
    pub(super) grader_database_url: Option<String>,
    pub(super) enrollment_secret: Option<EnrollmentSecretSettings>,
    pub(super) enrollment_email: Option<EnrollmentEmailSettings>,
    pub(super) webauthn: crate::auth::PasswordlessWebauthn,
}

pub(super) struct EnrollmentEmailSettings {
    pub(super) smtp_relay: String,
    pub(super) smtp_port: u16,
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
    pub(super) webwork_course_id: String,
    pub(super) webwork_user: String,
    pub(super) webwork_password_file: String,
}

impl ProductionSettings {
    pub(super) fn from_env() -> Result<Self> {
        Ok(Self {
            storage: StorageSettings::from_env()?,
            public_asset_base_url: required_env("PLE_PUBLIC_ASSET_BASE_URL")?,
            webwork: WebworkRendererSettings::from_env()?,
            imathas_provider_key: std::env::var("PLE_IMATHAS_PROVIDER_KEY").ok(),
            qti_runtime_enabled: std::env::var("PLE_QTI_RUNTIME_ENABLED").ok(),
            grader_database_url: std::env::var("PLE_GRADER_DATABASE_URL").ok(),
            enrollment_secret: EnrollmentSecretSettings::from_env()?,
            enrollment_email: EnrollmentEmailSettings::from_env()?,
            webauthn: crate::auth::PasswordlessWebauthn::new(
                &required_env("PLE_WEBAUTHN_RP_ID")?,
                &required_env("PLE_WEBAUTHN_ORIGIN")?,
                &required_env("PLE_WEBAUTHN_RP_NAME")?,
            )
            .map_err(anyhow::Error::msg)?,
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

impl EnrollmentEmailSettings {
    const ENV_NAMES: [&'static str; 6] = [
        "PLE_SMTP_RELAY",
        "PLE_SMTP_PORT",
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
    pub(super) const ENV_NAMES: [&'static str; 8] = [
        "PLE_WEBWORK_RENDERER_BASE_URL",
        "PLE_WEBWORK_REQUEST_TIMEOUT_SECONDS",
        "PLE_WEBWORK_MAX_RESPONSE_BYTES",
        "PLE_WEBWORK_RENDERER_ID",
        "PLE_WEBWORK_RENDERER_VERSION",
        "PLE_WEBWORK_RENDER_COURSE_ID",
        "PLE_WEBWORK_RENDER_USER",
        "PLE_WEBWORK_RENDER_PASSWORD_FILE",
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
            webwork_course_id: required_env("PLE_WEBWORK_RENDER_COURSE_ID")?,
            webwork_user: required_env("PLE_WEBWORK_RENDER_USER")?,
            webwork_password_file: required_env("PLE_WEBWORK_RENDER_PASSWORD_FILE")?,
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
            &self.webwork_course_id,
            &self.webwork_user,
            &read_webwork_password_file(&self.webwork_password_file)?,
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

pub(super) fn read_webwork_password_file(path: &str) -> Result<String> {
    read_secret_file(path, "PLE_WEBWORK_RENDER_PASSWORD_FILE")
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

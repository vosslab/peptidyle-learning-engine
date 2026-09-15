//! Fail-closed object-storage configuration for server composition.

use anyhow::{Result, bail};
use objects::{
    minio::{EndpointConfig, client as minio_client},
    s3::{BucketNames, S3ObjectStore},
};

#[derive(Clone, Copy)]
enum ObjectStorageTopology {
    DisposableLocal,
}

impl ObjectStorageTopology {
    fn parse(value: Option<&str>) -> Result<Self> {
        // ASVS 2.2.1-2.2.2: accept only the one supported topology at the
        // trusted composition boundary; absence and unknown values fail closed.
        match value {
            Some("disposable-local") => Ok(Self::DisposableLocal),
            Some(_) => bail!("PLE_STORAGE_TOPOLOGY must be set to disposable-local"),
            None => bail!("PLE_STORAGE_TOPOLOGY must be set to disposable-local"),
        }
    }
}

/// Least-privilege process identity used to select private runtime credentials.
#[derive(Clone, Copy)]
pub(super) enum ObjectStoragePrincipal {
    ApiAndWorker,
    PublicAssetPublisher,
}

impl ObjectStoragePrincipal {
    fn credential_environment_names(self) -> (&'static str, &'static str) {
        match self {
            Self::ApiAndWorker => ("AWS_ACCESS_KEY_ID", "AWS_SECRET_ACCESS_KEY"),
            Self::PublicAssetPublisher => (
                "PLE_PUBLISHER_S3_ACCESS_KEY_ID",
                "PLE_PUBLISHER_S3_SECRET_ACCESS_KEY",
            ),
        }
    }
}

struct S3CompatibleStorageConfig {
    endpoint: EndpointConfig,
    buckets: BucketNames,
}

/// Builds the supported authenticated MinIO store from private runtime values.
pub(super) fn object_store_from_env(principal: ObjectStoragePrincipal) -> Result<S3ObjectStore> {
    let config =
        s3_compatible_storage_config_from_values(principal, |name| std::env::var(name).ok())?;
    Ok(S3ObjectStore::new(
        minio_client(&config.endpoint),
        config.buckets,
    ))
}

fn s3_compatible_storage_config_from_values(
    principal: ObjectStoragePrincipal,
    mut value: impl FnMut(&str) -> Option<String>,
) -> Result<S3CompatibleStorageConfig> {
    let topology_value = value("PLE_STORAGE_TOPOLOGY");
    let topology = ObjectStorageTopology::parse(topology_value.as_deref())?;
    let ObjectStorageTopology::DisposableLocal = topology;
    let (access_key_name, secret_key_name) = principal.credential_environment_names();

    // ASVS 13.2.2 and 13.3.2: retain the controller-provided least-privilege
    // identity selected for this process instead of sharing one credential.
    let endpoint = EndpointConfig {
        endpoint_url: required_configuration_value(&mut value, "PLE_S3_ENDPOINT")?,
        region: required_configuration_value(&mut value, "PLE_S3_REGION")?,
        access_key_id: required_configuration_value(&mut value, access_key_name)?,
        secret_access_key: required_configuration_value(&mut value, secret_key_name)?,
    };
    let buckets = BucketNames {
        public_assets: required_configuration_value(&mut value, "PLE_PUBLIC_ASSETS_BUCKET")?,
        private_content: required_configuration_value(&mut value, "PLE_PRIVATE_CONTENT_BUCKET")?,
        student_records: required_configuration_value(&mut value, "PLE_STUDENT_RECORDS_BUCKET")?,
        temp_processing: required_configuration_value(&mut value, "PLE_TEMP_PROCESSING_BUCKET")?,
    };
    Ok(S3CompatibleStorageConfig { endpoint, buckets })
}

fn required_configuration_value(
    value: &mut impl FnMut(&str) -> Option<String>,
    name: &str,
) -> Result<String> {
    match value(name) {
        Some(value) if !value.is_empty() => Ok(value),
        Some(_) => bail!("{name} must not be empty"),
        None => bail!("{name} must be set"),
    }
}

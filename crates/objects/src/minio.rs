//! MinIO backend and the S3-compatible client.
//!
//! MinIO is the S3-compatible endpoint used by the development and test
//! containers. Production AWS construction is deliberately separate in
//! [`crate::aws`] so endpoint overrides and static credentials cannot leak into
//! the production configuration shape.

#[cfg(feature = "s3")]
use aws_sdk_s3::Client;
#[cfg(feature = "s3")]
use aws_sdk_s3::config::{BehaviorVersion, Config, Credentials, Region};

/// The object-store client type, re-exported so callers do not need the AWS
/// SDK in their own manifest.
///
/// This crate owns the S3 client, and no AWS type may leak through the
/// `ObjectStore` trait. The alias keeps the health path from becoming the
/// exception that drags the SDK into every caller.
#[cfg(feature = "s3")]
pub type S3Client = Client;

/// Connection settings for an S3-compatible endpoint.
#[cfg(feature = "s3")]
#[derive(Debug, Clone)]
pub struct EndpointConfig {
    /// Base URL, for example `http://minio:9000`.
    pub endpoint_url: String,
    /// Region name. MinIO ignores it, but the SDK requires one.
    pub region: String,
    /// Access key from the environment.
    pub access_key_id: String,
    /// Secret key from the environment.
    pub secret_access_key: String,
}

/// Builds an S3 client for the local MinIO endpoint.
///
/// Path-style addressing is forced because MinIO in a container is reached by
/// host name, and virtual-host-style addressing would require per-bucket DNS
/// that does not exist there.
#[cfg(feature = "s3")]
pub fn client(settings: &EndpointConfig) -> Client {
    let credentials = Credentials::new(
        settings.access_key_id.clone(),
        settings.secret_access_key.clone(),
        None,
        None,
        "ple-environment",
    );
    let config = Config::builder()
        .behavior_version(BehaviorVersion::latest())
        .region(Region::new(settings.region.clone()))
        .endpoint_url(settings.endpoint_url.clone())
        .credentials_provider(credentials)
        .force_path_style(true)
        .build();
    Client::from_conf(config)
}

/// Confirms a bucket exists and is reachable with the configured credentials.
///
/// This is the health probe. `HeadBucket` is a real authorized request, so it
/// fails when the endpoint is down, when the credentials are wrong, and when
/// the bucket is missing -- all three of which break the API, and none of
/// which a TCP connection check would catch.
///
/// # Errors
///
/// Returns a message naming the bucket and the underlying cause.
#[cfg(feature = "s3")]
pub async fn probe_bucket(client: &Client, bucket: &str) -> Result<(), String> {
    client
        .head_bucket()
        .bucket(bucket)
        .send()
        .await
        .map_err(|error| format!("bucket {bucket} not reachable: {error}"))?;
    Ok(())
}

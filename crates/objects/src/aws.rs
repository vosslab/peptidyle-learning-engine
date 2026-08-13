//! Production AWS S3 client construction.
//!
//! Production deliberately does not use the AWS default credential chain: its
//! environment and shared-profile sources would make long-lived static keys a
//! valid deployment shape. The container provider accepts ECS task roles and
//! EKS pod identity, both of which issue bounded, automatically rotated
//! workload credentials.

#[cfg(feature = "s3")]
use aws_config::BehaviorVersion;
#[cfg(feature = "s3")]
use aws_config::ecs::EcsCredentialsProvider;
#[cfg(feature = "s3")]
use aws_sdk_s3::Client;
#[cfg(feature = "s3")]
use aws_sdk_s3::config::Region;

/// Production S3 settings that are safe to supply as ordinary configuration.
#[cfg(feature = "s3")]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContainerRoleConfig {
    /// AWS region containing the three policy-separated buckets.
    pub region: String,
}

/// Builds an HTTPS AWS S3 client backed only by container workload identity.
#[cfg(feature = "s3")]
pub async fn container_role_client(settings: &ContainerRoleConfig) -> Client {
    let credentials = EcsCredentialsProvider::builder().build();
    let sdk_config = aws_config::defaults(BehaviorVersion::latest())
        .region(Region::new(settings.region.clone()))
        .credentials_provider(credentials)
        .load()
        .await;
    Client::new(&sdk_config)
}

//! Explicit typed-runtime construction used only by live cross-store acceptance tests.

use std::sync::Arc;

pub(super) fn minio_objects(
    runtime: &acceptance_runtime::CourseAppearanceRuntime,
) -> Arc<objects::s3::S3ObjectStore> {
    use objects::minio::{EndpointConfig, client};
    use objects::s3::{BucketNames, S3ObjectStore};

    let minio = runtime.minio();
    let settings = EndpointConfig {
        endpoint_url: minio.endpoint_url().to_owned(),
        region: minio.region().to_owned(),
        access_key_id: minio.access_key_id().to_owned(),
        secret_access_key: minio.secret_access_key().to_owned(),
    };
    Arc::new(S3ObjectStore::new(
        client(&settings),
        BucketNames::default(),
    ))
}

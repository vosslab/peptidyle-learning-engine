//! Isolated worker and publisher startup with process-specific authority.

use anyhow::{Context, Result};
use learning_data_access::postgres::{
    Pool, PostgresAssessmentAttemptExpirySweepStore, PostgresCourseRetentionNotificationStore,
    PostgresCourseRetentionStore, PostgresPublicAssetPublicationStore, ProductionLoginProfile,
    local_development_pool, production_pool,
};
use objects::s3::S3ObjectStore;

use super::{environment, object_storage};

/// Attests the one worker login without constructing an API router or listener.
// ASVS 8.3.1: the worker's database URL must attest the worker profile before
// it can sweep expired Assessment Attempts. No Account/session authority is constructed.
pub async fn verify_worker_database_login_from_env() -> Result<()> {
    let database_url = environment::required_env("DATABASE_URL")?;
    let pool = if std::env::var("PLE_STORAGE_TOPOLOGY").ok().as_deref() == Some("disposable-local")
    {
        local_development_pool(
            &database_url,
            ProductionLoginProfile::AssessmentAttemptExpiryWorker,
        )
    } else {
        production_pool(
            &database_url,
            ProductionLoginProfile::AssessmentAttemptExpiryWorker,
        )
    }
    .context("could not construct the attested worker database pool")?;
    pool.acquire()
        .await
        .context("the attested worker database pool could not connect")?;
    Ok(())
}

/// Runs the isolated Course-retention process from its two attested pools.
///
/// This constructs no listener, object/session/renderer client, or API state.
/// The executor and notifier stay separate so a provider-facing notification
/// attempt cannot acquire Course-transition authority.
pub async fn run_course_retention_process_from_env() -> Result<()> {
    let executor_url = environment::required_env("PLE_COURSE_RETENTION_EXECUTOR_DATABASE_URL")?;
    let notifier_url = environment::required_env("PLE_COURSE_RETENTION_NOTIFIER_DATABASE_URL")?;
    let local = std::env::var("PLE_STORAGE_TOPOLOGY").ok().as_deref() == Some("disposable-local");
    let executor = if local {
        local_development_pool(
            &executor_url,
            ProductionLoginProfile::CourseRetentionExecutor,
        )
    } else {
        production_pool(
            &executor_url,
            ProductionLoginProfile::CourseRetentionExecutor,
        )
    }
    .context("could not construct the attested Course-retention executor database pool")?;
    let notifier = if local {
        local_development_pool(
            &notifier_url,
            ProductionLoginProfile::CourseRetentionNotifier,
        )
    } else {
        production_pool(
            &notifier_url,
            ProductionLoginProfile::CourseRetentionNotifier,
        )
    }
    .context("could not construct the attested Course-retention notifier database pool")?;
    executor
        .acquire()
        .await
        .context("the attested Course-retention executor database pool could not connect")?;
    notifier
        .acquire()
        .await
        .context("the attested Course-retention notifier database pool could not connect")?;

    crate::course_retention_worker::run_until_shutdown(
        PostgresCourseRetentionStore::new(executor),
        PostgresCourseRetentionNotificationStore::new(notifier),
        environment::course_retention_notification_delivery_from_env()?,
    )
    .await
}

/// Runs the attested Assessment Attempt expiry worker.
pub async fn run_attempt_expiry_worker_from_env() -> Result<()> {
    let database_url = environment::required_env("DATABASE_URL")?;
    let pool = if std::env::var("PLE_STORAGE_TOPOLOGY").ok().as_deref() == Some("disposable-local")
    {
        local_development_pool(
            &database_url,
            ProductionLoginProfile::AssessmentAttemptExpiryWorker,
        )
    } else {
        production_pool(
            &database_url,
            ProductionLoginProfile::AssessmentAttemptExpiryWorker,
        )
    }
    .context("could not construct the attested Attempt expiry worker database pool")?;
    pool.acquire()
        .await
        .context("the attested Attempt expiry worker database pool could not connect")?;
    let objects = environment::question_library_object_store_from_env().await?;
    let webwork = environment::webwork_adapter_from_env()?;
    crate::worker::run_until_shutdown(
        PostgresAssessmentAttemptExpirySweepStore::new(pool),
        objects,
        webwork,
    )
    .await
}

/// Runs the separate publisher with its one database capability and dedicated
/// object-store identity. It constructs neither API/session state nor a listener.
pub async fn run_public_asset_publisher_from_env() -> Result<()> {
    let pool = public_asset_publisher_pool_from_env().await?;
    let publication_store = PostgresPublicAssetPublicationStore::new(pool);
    let objects = public_asset_publisher_object_store_from_env().await?;
    crate::public_asset_publisher::run_until_shutdown(publication_store, objects).await
}

/// Attests only the dedicated publisher login, without claiming a Job.
pub async fn verify_public_asset_publisher_database_login_from_env() -> Result<()> {
    let _pool = public_asset_publisher_pool_from_env().await?;
    Ok(())
}

// ASVS 13.2.2: both startup and probing attest the publisher-only login;
// neither constructs the API, Account, session, or renderer capability.
async fn public_asset_publisher_pool_from_env() -> Result<Pool> {
    let database_url = environment::required_env("DATABASE_URL")?;
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
    Ok(pool)
}

async fn public_asset_publisher_object_store_from_env() -> Result<S3ObjectStore> {
    object_storage::object_store_from_env(
        object_storage::ObjectStoragePrincipal::PublicAssetPublisher,
    )
}

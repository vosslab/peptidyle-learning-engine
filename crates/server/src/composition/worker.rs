//! Production-only composition for the supported durable worker families.

use std::{sync::Arc, time::Duration};

use anyhow::{Context, Result, bail};
use learning_data_access::{JobKind, postgres::SchemaCompatibilityError};

use super::settings::{
    LazyStorageDependencies, PublisherStorageDependencies, StorageRuntime, StorageSettings,
    invitation_delivery_worker_database_url_from_env, invitation_delivery_worker_from_env,
};
use crate::{
    course::invitation_delivery_worker::InvitationDeliveryWorker,
    export_worker::{ExportJobCommitter, ExportJobHandler},
    item_analysis_worker::{CourseItemAnalysisCommitter, CourseItemAnalysisHandler},
    public_asset_publication_worker::{
        PublicAssetPublicationCommitter, PublicAssetPublicationHandler,
    },
    qti_import::{QtiImportCommitter, QtiImportHandler},
    retention_worker::{RetentionJobCommitter, RetentionJobHandler},
    scoring_worker::{AssignmentScoringCommitter, AssignmentScoringHandler},
    timing_worker::{AttemptAutoSubmitCommitter, AttemptAutoSubmitHandler},
    worker::{
        EffectCommitter, JobHandler, JobRegistry, JobRegistryEntry, Worker, WorkerSettings, runtime,
    },
};

const DEFAULT_LEASE_SECONDS: u32 = 120;
const DEFAULT_PREPARATION_TIMEOUT_SECONDS: u64 = 90;
const PRODUCTION_BATCH_SIZE: usize = 1;
const DEFAULT_POLL_MILLIS: u64 = 500;

struct ProductionWorkerSettings {
    worker: WorkerSettings,
    poll_interval: Duration,
}

impl ProductionWorkerSettings {
    fn from_env() -> Result<Self> {
        let lease_seconds = bounded_env(
            "PLE_WORKER_LEASE_SECONDS",
            DEFAULT_LEASE_SECONDS,
            1_u32,
            900_u32,
        )?;
        let preparation_timeout_seconds = bounded_env(
            "PLE_WORKER_PREPARATION_TIMEOUT_SECONDS",
            DEFAULT_PREPARATION_TIMEOUT_SECONDS,
            1_u64,
            899_u64,
        )?;
        let poll_millis = bounded_env(
            "PLE_WORKER_POLL_MILLIS",
            DEFAULT_POLL_MILLIS,
            50_u64,
            60_000_u64,
        )?;
        let worker = WorkerSettings::new(
            lease_seconds,
            Duration::from_secs(preparation_timeout_seconds),
            PRODUCTION_BATCH_SIZE,
        )
        .context("worker execution settings are incompatible")?;
        Ok(Self {
            worker,
            poll_interval: Duration::from_millis(poll_millis),
        })
    }
}

fn bounded_env<T>(name: &str, default: T, minimum: T, maximum: T) -> Result<T>
where
    T: Copy + Ord + std::str::FromStr,
{
    let value = match std::env::var(name) {
        Ok(value) => value
            .parse::<T>()
            .map_err(|_| anyhow::anyhow!("{name} must be an integer"))?,
        Err(std::env::VarError::NotPresent) => default,
        Err(std::env::VarError::NotUnicode(_)) => bail!("{name} must be valid UTF-8"),
    };
    if value < minimum || value > maximum {
        bail!("{name} is outside its supported range");
    }
    Ok(value)
}

/// Starts the complete production registry. Render and generic Import remain
/// reserved and are absent from the broker filter until their implementations
/// have both a handler and an atomic committer.
pub async fn run_production_worker_from_env() -> Result<()> {
    run_worker_from_env(StorageRuntime::worker_from_env()?).await
}

/// Starts the production invitation-delivery process. Its database login has
/// only the outbox broker capability; it never constructs the generic Store.
pub async fn run_production_invitation_delivery_worker_from_env() -> Result<()> {
    run_invitation_delivery_worker_from_env().await
}

/// Starts the least-authority publisher process. It claims no educational
/// work: its sole capability is materializing already-committed public asset
/// bytes, then activating their matching registry records.
pub async fn run_public_asset_publisher_from_env() -> Result<()> {
    let storage = StorageSettings::from_env(StorageRuntime::publisher_from_env()?)?;
    let settings = ProductionWorkerSettings::from_env()?;
    let dependencies = PublisherStorageDependencies::from_settings(&storage).await?;
    verify_publisher_schema(&dependencies.pool).await?;
    let store = dependencies.store;
    let objects = dependencies.objects;
    let registry = JobRegistry::new([entry(
        JobKind::PublishPublicAssets,
        PublicAssetPublicationHandler::new(Arc::clone(&store), objects),
        PublicAssetPublicationCommitter::new(Arc::clone(&store)),
    )])
    .context("public-asset publisher registry is invalid")?;
    let worker = Worker::new(Arc::clone(&store), registry, settings.worker);
    eprintln!("peptidyle public-asset publisher ready");
    runtime::run_until_shutdown(worker, settings.poll_interval, runtime::shutdown_signal())
        .await
        .context("public-asset publisher runtime failed")
}

async fn run_worker_from_env(runtime: StorageRuntime) -> Result<()> {
    let storage = StorageSettings::from_env(runtime)?;
    let settings = ProductionWorkerSettings::from_env()?;
    let dependencies = LazyStorageDependencies::from_settings(&storage).await?;
    verify_worker_schema(&dependencies.pool).await?;

    let store = dependencies.store;
    let objects = dependencies.objects;
    let registry = JobRegistry::new([
        entry(
            JobKind::RecalculateAssignment,
            AssignmentScoringHandler::new(Arc::clone(&store)),
            AssignmentScoringCommitter::new(Arc::clone(&store)),
        ),
        entry(
            JobKind::RecalculateCourseItemAnalysis,
            CourseItemAnalysisHandler::new(Arc::clone(&store)),
            CourseItemAnalysisCommitter::new(Arc::clone(&store)),
        ),
        entry(
            JobKind::AutoSubmitAttempt,
            AttemptAutoSubmitHandler::new(),
            AttemptAutoSubmitCommitter::new(Arc::clone(&store)),
        ),
        entry(
            JobKind::Retention,
            RetentionJobHandler::new(Arc::clone(&store), Arc::clone(&objects)),
            RetentionJobCommitter::new(Arc::clone(&store)),
        ),
        entry(
            JobKind::Export,
            ExportJobHandler::new(Arc::clone(&store), Arc::clone(&objects)),
            ExportJobCommitter::new(Arc::clone(&store)),
        ),
        entry(
            JobKind::QtiImport,
            QtiImportHandler::new(Arc::clone(&store), Arc::clone(&objects)),
            QtiImportCommitter::new(Arc::clone(&store)),
        ),
    ])
    .context("production worker registry is invalid")?;
    let worker = Worker::new(store, registry, settings.worker);
    eprintln!("peptidyle worker ready with 6 supported job families");
    runtime::run_until_shutdown(worker, settings.poll_interval, runtime::shutdown_signal())
        .await
        .context("production worker runtime failed")
}

async fn run_invitation_delivery_worker_from_env() -> Result<()> {
    let settings = ProductionWorkerSettings::from_env()?;
    let database_url = invitation_delivery_worker_database_url_from_env()?;
    let pool = learning_data_access::postgres::production_pool(
        &database_url,
        learning_data_access::postgres::ProductionLoginProfile::InvitationDeliveryWorker,
    )
    .map_err(|_| anyhow::anyhow!("database connection configuration was rejected"))?;
    verify_invitation_delivery_worker_schema(&pool).await?;
    let store =
        Arc::new(learning_data_access::postgres::PostgresInvitationDeliveryWorkerStore::new(pool));
    let (issuer, delivery) = invitation_delivery_worker_from_env()?.ok_or_else(|| {
        anyhow::anyhow!(
            "invitation-delivery worker requires complete SMTP and PLE_INVITATION_TOKEN_SECRET_FILE configuration"
        )
    })?;
    let worker = Arc::new(InvitationDeliveryWorker::new(
        store,
        issuer,
        delivery,
        u16::try_from(PRODUCTION_BATCH_SIZE).expect("fixed batch fits u16"),
        DEFAULT_LEASE_SECONDS,
    )?);
    eprintln!("peptidyle invitation-delivery worker ready");
    runtime::run_bounded_dispatch_until_shutdown(
        worker,
        settings.poll_interval,
        runtime::shutdown_signal(),
    )
    .await
    .context("invitation-delivery worker runtime failed")
}

fn entry<H, C>(kind: JobKind, handler: H, committer: C) -> JobRegistryEntry
where
    H: JobHandler,
    C: EffectCommitter,
{
    let handler: Arc<dyn JobHandler> = Arc::new(handler);
    let committer: Arc<dyn EffectCommitter> = Arc::new(committer);
    JobRegistryEntry::new(kind, handler, committer)
}

async fn verify_worker_schema(pool: &learning_data_access::postgres::Pool) -> Result<()> {
    match tokio::time::timeout(
        Duration::from_secs(5),
        learning_data_access::postgres::verify_application_schema(pool),
    )
    .await
    {
        Ok(Ok(())) => Ok(()),
        Ok(Err(SchemaCompatibilityError::Unavailable)) | Err(_) => {
            bail!("database schema verification is unavailable; worker will not drain")
        }
        Ok(Err(SchemaCompatibilityError::Incompatible(reason))) => {
            bail!("database schema is incompatible: {reason}")
        }
    }
}

async fn verify_invitation_delivery_worker_schema(
    pool: &learning_data_access::postgres::Pool,
) -> Result<()> {
    match tokio::time::timeout(
        Duration::from_secs(5),
        learning_data_access::postgres::verify_invitation_delivery_worker_schema(pool),
    )
    .await
    {
        Ok(Ok(())) => Ok(()),
        Ok(Err(SchemaCompatibilityError::Unavailable)) | Err(_) => {
            bail!("invitation-delivery schema verification is unavailable; worker will not drain")
        }
        Ok(Err(SchemaCompatibilityError::Incompatible(_))) => {
            bail!("invitation-delivery schema is incompatible; worker will not drain")
        }
    }
}

async fn verify_publisher_schema(pool: &learning_data_access::postgres::Pool) -> Result<()> {
    match tokio::time::timeout(
        Duration::from_secs(5),
        learning_data_access::postgres::verify_public_asset_publisher_schema(pool),
    )
    .await
    {
        Ok(Ok(())) => Ok(()),
        Ok(Err(SchemaCompatibilityError::Unavailable)) | Err(_) => {
            bail!("database schema verification timed out or is unavailable")
        }
        Ok(Err(SchemaCompatibilityError::Incompatible(_))) => {
            bail!("database schema verification failed")
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn worker_settings_refuse_invalid_lease_and_preparation_deadline() {
        assert!(WorkerSettings::new(0, Duration::from_secs(1), 1).is_err());
        assert!(WorkerSettings::new(10, Duration::from_secs(10), 1).is_err());
    }

    #[test]
    fn public_asset_publisher_claim_filter_has_no_other_authority() {
        let filter = learning_data_access::JobClaimFilter::new([JobKind::PublishPublicAssets])
            .expect("one closed publisher family");
        assert!(filter.contains(JobKind::PublishPublicAssets));
        assert!(!filter.contains(JobKind::Export));
        assert!(!filter.contains(JobKind::QtiImport));
        assert!(!filter.contains(JobKind::Retention));
    }
}

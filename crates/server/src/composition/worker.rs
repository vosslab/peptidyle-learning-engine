//! Production-only composition for the supported durable worker families.

use std::{sync::Arc, time::Duration};

use anyhow::{Context, Result, bail};
use learning_data_access::{JobKind, postgres::SchemaCompatibilityError};

use super::{LazyStorageDependencies, StorageSettings};
use crate::{
    export_worker::{ExportJobCommitter, ExportJobHandler},
    item_analysis_worker::{CourseItemAnalysisCommitter, CourseItemAnalysisHandler},
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
    let storage = StorageSettings::from_env()?;
    let settings = ProductionWorkerSettings::from_env()?;
    let dependencies = LazyStorageDependencies::from_settings(&storage)?;
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
            QtiImportHandler::new(Arc::clone(&store), objects),
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn worker_defaults_are_valid_and_bounds_are_named() {
        WorkerSettings::new(
            DEFAULT_LEASE_SECONDS,
            Duration::from_secs(DEFAULT_PREPARATION_TIMEOUT_SECONDS),
            PRODUCTION_BATCH_SIZE,
        )
        .expect("production defaults");
        assert!(bounded_env("PLE_WORKER_TEST_MISSING", 5_u64, 1, 10).is_ok());
    }
}

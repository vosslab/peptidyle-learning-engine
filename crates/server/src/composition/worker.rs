//! Production-only composition for the supported durable worker families.

use std::{sync::Arc, time::Duration};

use anyhow::{Context, Result, bail};
use learning_data_access::{JobKind, postgres::SchemaCompatibilityError};
use uuid::Uuid;

use super::backend::{build_production_grading_backend, connect_production_grader};
use super::settings::{
    AcceptedSubmissionExecutionSettings, GradingBackendSettings, LazyStorageDependencies,
    ProcessRole, PublisherStorageDependencies, StorageRuntime, StorageSettings,
    invitation_delivery_worker_database_url_from_env, invitation_delivery_worker_from_env,
};
use super::{PostgresAcceptedSubmissionRecoveryStore, WorkerId};
use crate::{
    accepted_submission_worker::AcceptedSubmissionExecutionWorker,
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
        EffectCommitter, FairWorkerDispatcher, JobHandler, JobRegistry, JobRegistryEntry, Worker,
        WorkerSettings, runtime,
    },
};

const DEFAULT_POLL_MILLIS: u64 = 500;
const DEFAULT_LEASE_SECONDS: u32 = 120;
const PRODUCTION_BATCH_SIZE: usize = 1;

/// These are the complete generic queue families. Accepted submissions are
/// intentionally absent: their private material is reachable only through the
/// sealed execution capability below.
const GENERIC_WORKER_FAMILIES: [JobKind; 6] = [
    JobKind::RecalculateAssignment,
    JobKind::RecalculateCourseItemAnalysis,
    JobKind::AutoSubmitAttempt,
    JobKind::Retention,
    JobKind::Export,
    JobKind::QtiImport,
];

#[cfg(test)]
const SEALED_ACCEPTED_SUBMISSION_FAMILY: JobKind = JobKind::GradeAcceptedSubmission;
const SUPPORTED_WORKER_FAMILY_COUNT: usize = GENERIC_WORKER_FAMILIES.len() + 1;

/// The generic and sealed paths must receive the same validated bounds, while
/// the sealed path retains one process-stable identity across every pass.
#[derive(Clone, Copy)]
struct WorkerExecutionPlan {
    settings: WorkerSettings,
    worker_id: WorkerId,
}

impl WorkerExecutionPlan {
    fn new(settings: WorkerSettings, worker_id: WorkerId) -> Self {
        Self {
            settings,
            worker_id,
        }
    }

    fn generic_settings(self) -> WorkerSettings {
        self.settings
    }

    fn sealed_settings(self) -> WorkerSettings {
        self.settings
    }

    fn worker_id(self) -> WorkerId {
        self.worker_id
    }
}

struct ProductionWorkerSettings {
    execution: AcceptedSubmissionExecutionSettings,
    poll_interval: Duration,
}

impl ProductionWorkerSettings {
    fn from_env() -> Result<Self> {
        let poll_millis = super::settings::bounded_env(
            "PLE_WORKER_POLL_MILLIS",
            DEFAULT_POLL_MILLIS,
            50_u64,
            60_000_u64,
        )?;
        Ok(Self {
            execution: AcceptedSubmissionExecutionSettings::from_env()?,
            poll_interval: Duration::from_millis(poll_millis),
        })
    }
}

/// Starts the complete production registry. Render and generic Import remain
/// reserved and are absent from the broker filter until their implementations
/// have both a handler and an atomic committer.
pub async fn run_production_worker_from_env() -> Result<()> {
    run_worker_from_env(StorageRuntime::worker_from_env()?).await
}

/// Runs one sealed accepted submission through the deterministic exception
/// backend used by the isolated connected recovery journey.
///
/// The feature is compiled into a disposable acceptance image only.  This
/// process owns neither generic queue work nor an API route: it opens the
/// existing recovery capability, claims at most one accepted execution, and
/// delegates the failure commit to the common handler.
#[cfg(feature = "e2e-grader-fault")]
pub async fn run_deterministic_grader_exception_worker_from_env() -> Result<()> {
    let runtime = StorageRuntime::worker_from_env()?;
    if runtime.topology != super::settings::StorageTopology::DisposableLocal {
        bail!("deterministic grader exception worker requires disposable-local storage");
    }
    let settings = ProductionWorkerSettings::from_env()?;
    let recovery_database_url =
        super::accepted_submission_execution::recovery_database_url_from_env()?;
    let recovery_pool = super::local_accepted_submission_recovery_pool(&recovery_database_url)
        .await
        .map_err(|_| {
            anyhow::anyhow!(
                "accepted-submission recovery database connection configuration was rejected"
            )
        })?;
    let store = PostgresAcceptedSubmissionRecoveryStore::from_recovery_pool(recovery_pool);
    let worker = AcceptedSubmissionExecutionWorker::new(
        store,
        crate::accepted_submission_worker::DeterministicGraderExceptionBackend,
        WorkerId::from_uuid(Uuid::new_v4()),
        settings.execution.worker_settings(),
    )
    .context("deterministic grader exception worker settings are incompatible")?;
    let report = worker
        .drain_one()
        .await
        .context("deterministic grader exception worker could not complete its one-claim pass")?;
    eprintln!(
        "peptidyle deterministic grader exception worker completed one sealed pass: no_claim={}, terminal={}",
        report.no_claim, report.terminal
    );
    Ok(())
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
    let worker = Worker::new(
        Arc::clone(&store),
        registry,
        settings.execution.worker_settings(),
    );
    eprintln!("peptidyle public-asset publisher ready");
    runtime::run_until_shutdown(worker, settings.poll_interval, runtime::shutdown_signal())
        .await
        .context("public-asset publisher runtime failed")
}

async fn run_worker_from_env(runtime: StorageRuntime) -> Result<()> {
    if runtime.role != ProcessRole::Worker {
        bail!("production worker composition requires the Worker storage profile");
    }
    let storage = StorageSettings::from_env(runtime)?;
    let settings = ProductionWorkerSettings::from_env()?;
    let grading = GradingBackendSettings::from_env()?;
    let dependencies = LazyStorageDependencies::from_settings(&storage).await?;
    verify_worker_schema(&dependencies.pool).await?;

    let store = dependencies.store;
    let objects = dependencies.objects;
    let recovery_database_url =
        super::accepted_submission_execution::recovery_database_url_from_env()?;
    // ASVS 13.2.2: the recovery login is isolated from both the generic
    // worker store and the separate grader connection capability.
    let recovery_pool = match storage.runtime.topology {
        super::settings::StorageTopology::DisposableLocal => {
            super::local_accepted_submission_recovery_pool(&recovery_database_url).await
        }
        super::settings::StorageTopology::AwsWorkload => {
            super::accepted_submission_recovery_pool(&recovery_database_url).await
        }
    }
    .map_err(|_| {
        anyhow::anyhow!(
            "accepted-submission recovery database connection configuration was rejected"
        )
    })?;
    let sealed_store = PostgresAcceptedSubmissionRecoveryStore::from_recovery_pool(recovery_pool);
    let grader = connect_production_grader(&grading, storage.runtime.topology).await?;
    let backend = build_production_grading_backend(
        Arc::clone(&store),
        Arc::clone(&objects),
        grader,
        &grading,
    );
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
    let execution = WorkerExecutionPlan::new(
        settings.execution.worker_settings(),
        WorkerId::from_uuid(Uuid::new_v4()),
    );
    let generic_worker = Worker::new(store, registry, execution.generic_settings());
    let accepted_worker = AcceptedSubmissionExecutionWorker::new(
        sealed_store,
        backend,
        execution.worker_id(),
        execution.sealed_settings(),
    )
    .context("accepted-submission worker settings are incompatible")?;
    let dispatcher = Arc::new(FairWorkerDispatcher::new(generic_worker, accepted_worker));
    eprintln!(
        "peptidyle worker ready with {SUPPORTED_WORKER_FAMILY_COUNT} supported job families: RecalculateAssignment, RecalculateCourseItemAnalysis, AutoSubmitAttempt, Retention, Export, QtiImport, GradeAcceptedSubmission"
    );
    runtime::run_bounded_dispatch_until_shutdown(
        dispatcher,
        settings.poll_interval,
        runtime::shutdown_signal(),
    )
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
    use learning_data_access::JobClaimFilter;

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

    #[test]
    fn production_worker_keeps_six_generic_families_and_seals_accepted_execution() {
        assert_eq!(
            GENERIC_WORKER_FAMILIES,
            [
                JobKind::RecalculateAssignment,
                JobKind::RecalculateCourseItemAnalysis,
                JobKind::AutoSubmitAttempt,
                JobKind::Retention,
                JobKind::Export,
                JobKind::QtiImport,
            ]
        );
        let filter = JobClaimFilter::new(GENERIC_WORKER_FAMILIES)
            .expect("the production generic worker families are closed");
        assert!(!filter.contains(SEALED_ACCEPTED_SUBMISSION_FAMILY));
    }

    #[test]
    fn worker_execution_plan_preserves_shared_settings_and_stable_identity() {
        let settings = WorkerSettings::new(120, Duration::from_secs(90), 1)
            .expect("valid shared worker settings");
        let worker_id = WorkerId::from_uuid(Uuid::from_u128(0xC3));
        let execution = WorkerExecutionPlan::new(settings, worker_id);

        assert_eq!(
            execution.generic_settings().lease(),
            execution.sealed_settings().lease()
        );
        assert_eq!(
            execution.generic_settings().execution_deadline(),
            execution.sealed_settings().execution_deadline()
        );
        assert_eq!(execution.worker_id(), worker_id);
    }
}

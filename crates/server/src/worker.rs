//! Narrow deadline worker for authoritative Assessment Attempt expiry.
//!
//! This process serves readiness and finalizes abandoned expired Attempts. It
//! does not own a native or WeBWorK grading queue: ordinary submissions grade
//! immediately, and a future backend-specific completion path belongs at that
//! backend boundary rather than in a generic worker loop.

use anyhow::{Context, Result};
use std::sync::Arc;
use tokio::io::AsyncWriteExt;

use adapter_webwork::{HttpWebworkRenderer, WebworkAdapter};
use learning_data_access::{
    AssessmentAttemptExpirySweepStore, ExpiredAssessmentAttemptFinalizationPreparation,
    LibraryWatchNotificationStore, StoreError, StudentAssessmentAttemptFinalizationEvaluation,
    StudentAssessmentAttemptFinalizationPreparation,
};
use objects::s3::S3ObjectStore;

use crate::assessment_delivery::direct_finalization;

/// Composes two closed worker capabilities without granting either raw tables.
pub struct WorkerStores<E, W> {
    pub expiry: E,
    pub watches: W,
}

#[async_trait::async_trait]
impl<E: AssessmentAttemptExpirySweepStore + Send + Sync, W: Send + Sync>
    AssessmentAttemptExpirySweepStore for WorkerStores<E, W>
{
    async fn prepare_expired_assessment_attempt_finalizations(
        &self,
        limit: u32,
    ) -> Result<Vec<ExpiredAssessmentAttemptFinalizationPreparation>, StoreError> {
        self.expiry
            .prepare_expired_assessment_attempt_finalizations(limit)
            .await
    }
    async fn commit_expired_assessment_attempt_finalization(
        &self,
        id: uuid::Uuid,
        preparation: StudentAssessmentAttemptFinalizationPreparation,
        evaluations: Vec<StudentAssessmentAttemptFinalizationEvaluation>,
    ) -> Result<(), StoreError> {
        self.expiry
            .commit_expired_assessment_attempt_finalization(id, preparation, evaluations)
            .await
    }
}

#[async_trait::async_trait]
impl<E: Send + Sync, W: LibraryWatchNotificationStore> LibraryWatchNotificationStore
    for WorkerStores<E, W>
{
    async fn materialize_library_watch_notifications(&self, limit: u16) -> Result<u32, StoreError> {
        self.watches
            .materialize_library_watch_notifications(limit)
            .await
    }
}

const WORKER_READINESS_BIND_ADDRESS: &str = "0.0.0.0:3001";
const ATTEMPT_EXPIRY_SWEEP_LIMIT: u32 = 100;
const LIBRARY_WATCH_NOTIFICATION_LIMIT: u16 = 100;
// The Student-visible expiry window is intentionally about a minute. This
// also bounds retries of an unavailable backend without per-Attempt state.
const ATTEMPT_EXPIRY_SWEEP_INTERVAL: std::time::Duration = std::time::Duration::from_secs(60);

/// Sweep expired Assessment Attempts while serving the private readiness socket.
// ASVS 2.3.1 and 2.3.3: expiry follows the same server-owned submission order
// and PostgreSQL atomically rechecks the captured snapshot before committing it.
pub async fn run_until_shutdown<
    S: AssessmentAttemptExpirySweepStore + LibraryWatchNotificationStore,
>(
    store: S,
    objects: S3ObjectStore,
    webwork: Arc<WebworkAdapter<HttpWebworkRenderer>>,
) -> Result<()> {
    let listener = tokio::net::TcpListener::bind(WORKER_READINESS_BIND_ADDRESS)
        .await
        .context("could not bind the private worker readiness socket")?;
    tracing::info!(
        event = "worker_ready",
        readiness_address = WORKER_READINESS_BIND_ADDRESS
    );
    tokio::select! {
        () = shutdown_signal() => {
            tracing::info!(event = "worker_shutdown_requested");
            Ok(())
        }
        result = serve_readiness(listener) => result,
        result = run_attempt_expiry_sweep(store, objects, webwork) => result,
    }
}

async fn run_attempt_expiry_sweep<
    S: AssessmentAttemptExpirySweepStore + LibraryWatchNotificationStore,
>(
    store: S,
    objects: S3ObjectStore,
    webwork: Arc<WebworkAdapter<HttpWebworkRenderer>>,
) -> Result<()> {
    loop {
        run_attempt_expiry_sweep_iteration(&store, &objects, webwork.as_ref()).await?;
        materialize_library_watch_notifications_iteration(&store).await;
        tokio::time::sleep(ATTEMPT_EXPIRY_SWEEP_INTERVAL).await;
    }
}

/// Watch delivery is retryable background work. It must not take down the
/// expiry worker when one bounded materialization attempt is unavailable.
async fn materialize_library_watch_notifications_iteration<S: LibraryWatchNotificationStore>(
    store: &S,
) {
    if let Err(error) = store
        .materialize_library_watch_notifications(LIBRARY_WATCH_NOTIFICATION_LIMIT)
        .await
    {
        tracing::warn!(event = "library_watch_materialization_failed", error = %error);
    }
}

/// Runs one bounded expiry auto-submission batch.
async fn run_attempt_expiry_sweep_iteration<S: AssessmentAttemptExpirySweepStore>(
    store: &S,
    objects: &S3ObjectStore,
    webwork: &WebworkAdapter<HttpWebworkRenderer>,
) -> Result<()> {
    let preparations = store
        .prepare_expired_assessment_attempt_finalizations(ATTEMPT_EXPIRY_SWEEP_LIMIT)
        .await
        .map_err(store_unavailable)?;
    for expired in preparations {
        let evaluations = direct_finalization::evaluate_saved_responses(
            objects,
            webwork,
            &expired.preparation.saved_responses,
        )
        .await;
        finalize_prepared_expired_attempt(store, expired, evaluations).await;
    }
    Ok(())
}

/// Finalizes one prepared Attempt without allowing its failure to starve the
/// other candidates already selected for this bounded pass.
async fn finalize_prepared_expired_attempt<S: AssessmentAttemptExpirySweepStore>(
    store: &S,
    expired: ExpiredAssessmentAttemptFinalizationPreparation,
    evaluations: Result<Vec<StudentAssessmentAttemptFinalizationEvaluation>, StoreError>,
) {
    let assessment_attempt_id = expired.assessment_attempt_id;
    let evaluations = match evaluations {
        Ok(evaluations) => evaluations,
        Err(error) => {
            log_expiry_error(assessment_attempt_id, "evaluation", &error);
            return;
        }
    };
    if let Err(error) = store
        .commit_expired_assessment_attempt_finalization(
            assessment_attempt_id,
            expired.preparation,
            evaluations,
        )
        .await
    {
        log_expiry_error(assessment_attempt_id, "commit", &error);
    }
}

fn log_expiry_error(assessment_attempt_id: uuid::Uuid, stage: &'static str, error: &StoreError) {
    tracing::warn!(
        event = "attempt_expiry_finalization_failed",
        assessment_attempt_id = %assessment_attempt_id,
        stage,
        error = %error,
    );
}

fn store_unavailable(error: StoreError) -> anyhow::Error {
    anyhow::Error::msg(format!("attempt expiry sweep unavailable: {error}"))
}

async fn serve_readiness(listener: tokio::net::TcpListener) -> Result<()> {
    loop {
        let (mut stream, _) = listener
            .accept()
            .await
            .context("could not accept worker readiness connection")?;
        tokio::spawn(async move {
            let _ = stream.write_all(b"ready\n").await;
        });
    }
}

async fn shutdown_signal() {
    #[cfg(unix)]
    {
        let mut terminate =
            tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
                .expect("SIGTERM signal handler installs");
        tokio::select! {
            _ = tokio::signal::ctrl_c() => {},
            _ = terminate.recv() => {},
        }
    }
    #[cfg(not(unix))]
    if let Err(error) = tokio::signal::ctrl_c().await {
        tracing::error!(event = "worker_shutdown_signal_unavailable", signal = "SIGINT", error = %error);
    }
}

#[cfg(test)]
#[path = "worker_tests.rs"]
mod worker_tests;

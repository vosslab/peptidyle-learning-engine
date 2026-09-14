//! Narrow deadline worker for authoritative Assignment Attempt expiry.
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
    AssignmentAttemptExpirySweepStore, ExpiredAssignmentAttemptFinalizationPreparation, StoreError,
    StudentAssignmentAttemptFinalizationEvaluation,
};
use objects::s3::S3ObjectStore;

use crate::assignment_delivery::direct_finalization;

const WORKER_READINESS_BIND_ADDRESS: &str = "0.0.0.0:3001";
const ATTEMPT_EXPIRY_SWEEP_LIMIT: u32 = 100;
// The Student-visible expiry window is intentionally about a minute. This
// also bounds retries of an unavailable backend without per-Attempt state.
const ATTEMPT_EXPIRY_SWEEP_INTERVAL: std::time::Duration = std::time::Duration::from_secs(60);

/// Sweep expired Assignment Attempts while serving the private readiness socket.
// ASVS 2.3.1 and 2.3.3: expiry follows the same server-owned submission order
// and PostgreSQL atomically rechecks the captured snapshot before committing it.
pub async fn run_until_shutdown<S: AssignmentAttemptExpirySweepStore>(
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

async fn run_attempt_expiry_sweep<S: AssignmentAttemptExpirySweepStore>(
    store: S,
    objects: S3ObjectStore,
    webwork: Arc<WebworkAdapter<HttpWebworkRenderer>>,
) -> Result<()> {
    loop {
        run_attempt_expiry_sweep_iteration(&store, &objects, webwork.as_ref()).await?;
        tokio::time::sleep(ATTEMPT_EXPIRY_SWEEP_INTERVAL).await;
    }
}

/// Runs one bounded expiry auto-submission batch.
async fn run_attempt_expiry_sweep_iteration<S: AssignmentAttemptExpirySweepStore>(
    store: &S,
    objects: &S3ObjectStore,
    webwork: &WebworkAdapter<HttpWebworkRenderer>,
) -> Result<()> {
    let preparations = store
        .prepare_expired_assignment_attempt_finalizations(ATTEMPT_EXPIRY_SWEEP_LIMIT)
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
async fn finalize_prepared_expired_attempt<S: AssignmentAttemptExpirySweepStore>(
    store: &S,
    expired: ExpiredAssignmentAttemptFinalizationPreparation,
    evaluations: Result<Vec<StudentAssignmentAttemptFinalizationEvaluation>, StoreError>,
) {
    let assignment_attempt_id = expired.assignment_attempt_id;
    let evaluations = match evaluations {
        Ok(evaluations) => evaluations,
        Err(error) => {
            log_expiry_error(assignment_attempt_id, "evaluation", &error);
            return;
        }
    };
    if let Err(error) = store
        .commit_expired_assignment_attempt_finalization(
            assignment_attempt_id,
            expired.preparation,
            evaluations,
        )
        .await
    {
        log_expiry_error(assignment_attempt_id, "commit", &error);
    }
}

fn log_expiry_error(assignment_attempt_id: uuid::Uuid, stage: &'static str, error: &StoreError) {
    tracing::warn!(
        event = "attempt_expiry_finalization_failed",
        assignment_attempt_id = %assignment_attempt_id,
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

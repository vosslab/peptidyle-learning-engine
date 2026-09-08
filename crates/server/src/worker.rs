//! Typed native PLE and WebWork grading worker lifecycle.
//!
//! The process owns no HTTP listener and receives only its distinct database
//! Service Identity. Claim-and-commit operations must not add direct
//! protected-table access here.

use anyhow::{Context, Result};
use std::sync::Arc;
use tokio::io::AsyncWriteExt;

use adapter_ple::{PleQuestionBackend, ResolvedPleQuestionJsonSource};
use adapter_webwork::{
    HttpWebworkRenderer, ResolvedWebworkQuestionSource, WebworkAdapter,
    WebworkQuestionSourceBinding,
};
use learning_data_access::{NativePleGradingStore, StoreError, WebworkGradingStore};
use objects::s3::S3ObjectStore;
use question_model::generation::QuestionSeed;
use question_model::{
    ObjectId, QuestionRevisionNumber, QuestionRevisionReference, SourceObjectChecksum,
    SourceObjectReference,
};

const WORKER_READINESS_BIND_ADDRESS: &str = "0.0.0.0:3001";
const NATIVE_PLE_LEASE_MILLISECONDS: i64 = 5_000;
const NATIVE_PLE_POST_LEASE_ABORT_WINDOW: std::time::Duration = std::time::Duration::from_secs(3);
// The lease outlives the renderer's configured request deadline, leaving the
// current worker time to record one bounded renderer failure.
const WEBWORK_LEASE_MILLISECONDS: i64 = 30_000;

/// Wait for process termination after the worker login has been attested.
// ASVS 8.3.1 and 15.3.1: a worker starts with only a verified, least-privilege
// capability and acknowledges termination before its container grace period.
pub async fn run_until_shutdown() -> Result<()> {
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
    }
}

/// Claims and commits only typed native PLE jobs. The loop deliberately owns
/// no HTTP listener and stops between claims, so controller replacement leaves
/// an expired lease for the replacement worker to recover.
pub async fn run_native_ple_until_shutdown<S: NativePleGradingStore>(
    store: S,
    objects: S3ObjectStore,
) -> Result<()> {
    loop {
        let now = unix_millis()?;
        let Some(lease) = store
            .claim_native_ple_grading_job(now + NATIVE_PLE_LEASE_MILLISECONDS)
            .await
            .map_err(store_failure)?
        else {
            tokio::select! { () = shutdown_signal() => return Ok(()), () = tokio::time::sleep(std::time::Duration::from_millis(250)) => continue }
        };
        // A worker that has received termination must not begin evaluation after
        // leasing a Job. This bounded handoff window makes that concrete: the
        // replacement reclaims the short lease and remains the only committer.
        if !await_native_ple_post_lease_abort_window().await {
            tracing::info!(event = "native_ple_worker_shutdown_after_lease");
            return Ok(());
        }
        let revision = QuestionRevisionReference {
            question_id: lease.question_id.clone(),
            revision_number: QuestionRevisionNumber::new(lease.revision_number)
                .map_err(|_| anyhow::anyhow!("native PLE revision is invalid"))?,
        };
        let object = uuid::Uuid::parse_str(&lease.source_object_id)
            .map_err(|_| anyhow::anyhow!("native PLE source object is invalid"))?;
        let source = ResolvedPleQuestionJsonSource::resolve(
            &objects,
            revision,
            SourceObjectReference {
                object: ObjectId::from_uuid(object),
            },
            SourceObjectChecksum::parse(lease.source_object_checksum.clone())
                .map_err(|_| anyhow::anyhow!("native PLE source checksum is invalid"))?,
        )
        .await
        .map_err(|_| anyhow::anyhow!("native PLE source is unavailable"))?;
        let response = serde_json::from_value(lease.student_response.clone())
            .map_err(|_| anyhow::anyhow!("native PLE stored response is invalid"))?;
        let evaluation = PleQuestionBackend::new()
            .grade_question_json(&source, &response)
            .map_err(|_| anyhow::anyhow!("native PLE grading source is invalid"))?;
        store
            .commit_native_ple_grading(
                &lease,
                evaluation.evaluation.correct(),
                evaluation.evaluation.normalized_credit(),
                unix_millis()?,
            )
            .await
            .map_err(store_failure)?;
    }
}

/// Claims only persisted WeBWorK submissions. A renderer outage finalizes the
/// affected Job as failed; it does not retry against another backend or alter
/// the accepted Student Response.
pub async fn run_webwork_until_shutdown<S: WebworkGradingStore>(
    store: S,
    objects: S3ObjectStore,
    webwork: Arc<WebworkAdapter<S3ObjectStore, HttpWebworkRenderer>>,
) -> Result<()> {
    loop {
        let Some(lease) = store
            .claim_webwork_grading_job(unix_millis()? + WEBWORK_LEASE_MILLISECONDS)
            .await
            .map_err(store_failure)?
        else {
            tokio::select! { () = shutdown_signal() => return Ok(()), () = tokio::time::sleep(std::time::Duration::from_millis(250)) => continue }
        };
        let revision = QuestionRevisionReference {
            question_id: lease.question_id.clone(),
            revision_number: QuestionRevisionNumber::new(lease.revision_number)
                .map_err(|_| anyhow::anyhow!("WeBWorK revision is invalid"))?,
        };
        let object = uuid::Uuid::parse_str(&lease.source_object_id)
            .map_err(|_| anyhow::anyhow!("WeBWorK source object is invalid"))?;
        let binding = WebworkQuestionSourceBinding::new(revision, lease.webwork_pg_path.clone())
            .map_err(|_| anyhow::anyhow!("WeBWorK source binding is invalid"))?;
        let source = ResolvedWebworkQuestionSource::resolve(
            &objects,
            binding,
            SourceObjectReference {
                object: ObjectId::from_uuid(object),
            },
            SourceObjectChecksum::parse(lease.source_object_checksum.clone())
                .map_err(|_| anyhow::anyhow!("WeBWorK source checksum is invalid"))?,
        )
        .await
        .map_err(|_| anyhow::anyhow!("WeBWorK source is unavailable"))?;
        let response = serde_json::from_value(lease.student_response.clone())
            .map_err(|_| anyhow::anyhow!("stored WeBWorK response is invalid"))?;
        let replay = serde_json::from_value(lease.replay_details.clone())
            .map_err(|_| anyhow::anyhow!("stored WeBWorK replay is invalid"))?;
        match webwork
            .grade(
                QuestionSeed::new(lease.question_seed),
                &source,
                &response,
                &replay,
            )
            .await
        {
            Ok(grading::QuestionGradingOutcome::Evaluated(evaluation)) => store
                .commit_webwork_grading(
                    &lease,
                    evaluation.correct(),
                    evaluation.normalized_credit(),
                    unix_millis()?,
                )
                .await
                .map_err(store_failure)?,
            Ok(grading::QuestionGradingOutcome::Ungraded) | Err(_) => store
                .fail_webwork_grading(&lease, unix_millis()?)
                .await
                .map_err(store_failure)?,
        }
    }
}

/// Wait briefly after a claim so a terminating worker leaves its lease durable
/// and uncommitted for the exact replacement-worker recovery path.
async fn await_native_ple_post_lease_abort_window() -> bool {
    tokio::select! {
        () = shutdown_signal() => false,
        () = tokio::time::sleep(NATIVE_PLE_POST_LEASE_ABORT_WINDOW) => true,
    }
}

fn unix_millis() -> Result<i64> {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|_| anyhow::anyhow!("system clock is invalid"))
        .and_then(|value| {
            i64::try_from(value.as_millis()).map_err(|_| anyhow::anyhow!("system clock is invalid"))
        })
}

fn store_failure(error: StoreError) -> anyhow::Error {
    anyhow::anyhow!("native PLE grading store unavailable: {error}")
}

async fn serve_readiness(listener: tokio::net::TcpListener) -> Result<()> {
    loop {
        let (mut stream, _peer) = listener
            .accept()
            .await
            .context("private worker readiness socket could not accept")?;
        // This is not an HTTP or job protocol. It accepts no request bytes and
        // returns one fixed liveness marker on the internal-only default network.
        stream
            .write_all(b"PLE-WORKER-READY\n")
            .await
            .context("private worker readiness socket could not respond")?;
    }
}

async fn shutdown_signal() {
    #[cfg(unix)]
    {
        let mut terminate = match tokio::signal::unix::signal(
            tokio::signal::unix::SignalKind::terminate(),
        ) {
            Ok(signal) => signal,
            Err(error) => {
                tracing::error!(event = "worker_shutdown_signal_unavailable", signal = "SIGTERM", error = %error);
                let _ = tokio::signal::ctrl_c().await;
                return;
            }
        };
        tokio::select! {
            result = tokio::signal::ctrl_c() => {
                if let Err(error) = result {
                    tracing::error!(event = "worker_shutdown_signal_unavailable", signal = "SIGINT", error = %error);
                }
            }
            _ = terminate.recv() => {}
        }
    }

    #[cfg(not(unix))]
    if let Err(error) = tokio::signal::ctrl_c().await {
        tracing::error!(event = "worker_shutdown_signal_unavailable", signal = "SIGINT", error = %error);
    }
}

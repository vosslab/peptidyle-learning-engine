//! Long-lived production polling around one bounded worker pass.

use std::{future::Future, sync::Arc, time::Duration};

use async_trait::async_trait;
use learning_data_access::{JobStore, StoreError};

use super::{DrainReport, Worker};

/// Drains complete batches and observes shutdown only between them. Dropping a
/// pass mid-preparation would detach its cancellation-owned task, so shutdown
/// deliberately waits for the current bounded pass to settle.
pub(crate) async fn run_until_shutdown<S, F>(
    worker: Worker<S>,
    poll_interval: Duration,
    shutdown: F,
) -> Result<(), StoreError>
where
    S: JobStore + 'static,
    F: Future<Output = ()>,
{
    tokio::pin!(shutdown);
    loop {
        // Poll once before claiming so Tokio has installed the process signal
        // listener. A signal received during the bounded pass is then retained
        // until the next boundary; the pass itself is never dropped midway.
        tokio::select! {
            biased;
            () = &mut shutdown => return Ok(()),
            () = tokio::task::yield_now() => {}
        }
        let report = worker.drain_once().await;
        let worked = match report {
            Ok(report) => {
                log_report(&worker, report).await;
                processed_count(report) > 0
            }
            Err(error) => {
                eprintln!("worker pass failed: {}", safe_error_kind(&error));
                false
            }
        };
        let delay = if worked {
            Duration::ZERO
        } else {
            poll_interval
        };
        tokio::select! {
            () = &mut shutdown => return Ok(()),
            () = tokio::time::sleep(delay) => {}
        }
    }
}

/// A bounded, server-only polling family outside browser-controllable jobs.
#[async_trait]
pub(crate) trait BoundedWorkerDispatch: Send + Sync {
    async fn drain_once(&self) -> Result<u32, StoreError>;
}

/// Runs one dedicated dispatch process. Each pass completes before shutdown is
/// observed, so no external call is detached midway.
pub(crate) async fn run_bounded_dispatch_until_shutdown<F>(
    dispatch: Arc<dyn BoundedWorkerDispatch>,
    poll_interval: Duration,
    shutdown: F,
) -> Result<(), StoreError>
where
    F: Future<Output = ()>,
{
    tokio::pin!(shutdown);
    loop {
        // Poll once before claiming so Tokio has installed the process signal
        // listener. A signal received during the bounded pass is then retained
        // until the next boundary; the pass itself is never dropped midway.
        tokio::select! {
            biased;
            () = &mut shutdown => return Ok(()),
            () = tokio::task::yield_now() => {}
        }
        let worked = match dispatch.drain_once().await {
            Ok(processed) => processed > 0,
            Err(error) => {
                eprintln!("worker dispatch pass failed: {}", safe_error_kind(&error));
                false
            }
        };
        let delay = if worked {
            Duration::ZERO
        } else {
            poll_interval
        };

        tokio::select! {
            () = &mut shutdown => return Ok(()),
            () = tokio::time::sleep(delay) => {}
        }
    }
}

async fn log_report<S>(worker: &Worker<S>, report: DrainReport)
where
    S: JobStore + 'static,
{
    let processed = processed_count(report);
    if processed == 0 {
        return;
    }
    let ready = worker
        .ready_queue_depth()
        .await
        .map(|depth| depth.ready.to_string())
        .unwrap_or_else(|_| "unavailable".to_string());
    eprintln!(
        "worker pass completed={} rescheduled={} retrying={} dead={} finalization_failed={} supported_ready={ready}",
        report.completed,
        report.rescheduled,
        report.retrying,
        report.dead,
        report.finalization_failed,
    );
}

fn processed_count(report: DrainReport) -> u32 {
    report.completed
        + report.rescheduled
        + report.retrying
        + report.dead
        + report.finalization_failed
}

fn safe_error_kind(error: &StoreError) -> &'static str {
    match error {
        StoreError::RetryableTransaction => "retryable_transaction",
        StoreError::Unavailable(_) => "unavailable",
        StoreError::TimedOut => "timed_out",
        StoreError::Conflict => "conflict",
        StoreError::Forbidden => "forbidden",
        StoreError::NotFound => "not_found",
        StoreError::AlreadyExists => "already_exists",
        StoreError::OwnershipMismatch => "ownership_mismatch",
        StoreError::InvalidRecord(_) => "invalid_record",
        StoreError::RunModel(_) => "run_model",
    }
}

/// Waits for the normal process stop signals without interrupting an active
/// bounded drain pass.
pub(crate) async fn shutdown_signal() {
    #[cfg(unix)]
    {
        let mut terminate =
            match tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate()) {
                Ok(signal) => signal,
                Err(error) => {
                    eprintln!("worker SIGTERM listener unavailable: {error}");
                    let _ = tokio::signal::ctrl_c().await;
                    return;
                }
            };
        tokio::select! {
            result = tokio::signal::ctrl_c() => {
                if let Err(error) = result {
                    eprintln!("worker interrupt listener failed: {error}");
                }
            }
            _ = terminate.recv() => {}
        }
    }

    #[cfg(not(unix))]
    if let Err(error) = tokio::signal::ctrl_c().await {
        eprintln!("worker interrupt listener failed: {error}");
    }
}

#[cfg(test)]
mod tests {
    use std::sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    };

    use async_trait::async_trait;
    use learning_data_access::{
        EnqueueJob, JobKind, JobPayload, JobStore, TenantContext, in_memory::MemoryStore,
    };
    use question_model::{ProblemId, ProblemVersionRef, TenantId, VersionId};
    use uuid::Uuid;

    use super::*;
    use crate::worker::{
        EffectCommitOutcome, EffectCommitter, JobCommitClaim, JobExecution, JobHandler,
        JobRegistry, JobRegistryEntry, PreparedJobEffect, WorkerSettings, sealed,
    };

    struct SettlingHandler {
        started: Arc<tokio::sync::Notify>,
    }

    #[async_trait]
    impl JobHandler for SettlingHandler {
        async fn prepare(
            &self,
            _: JobPayload,
            _: JobExecution,
        ) -> Result<PreparedJobEffect, learning_data_access::JobFailureKind> {
            self.started.notify_one();
            tokio::time::sleep(Duration::from_millis(20)).await;
            Ok(PreparedJobEffect::Test)
        }
    }

    struct CompletingCommitter(Arc<MemoryStore>);
    impl sealed::EffectCommitter for CompletingCommitter {}

    #[async_trait]
    impl EffectCommitter for CompletingCommitter {
        async fn commit(
            &self,
            claim: JobCommitClaim,
            _: PreparedJobEffect,
        ) -> Result<EffectCommitOutcome, StoreError> {
            self.0
                .complete_job(claim.job_id(), claim.lease_token())
                .await?;
            Ok(EffectCommitOutcome::Committed)
        }
    }

    struct SettlingDispatch {
        started: Arc<tokio::sync::Notify>,
        settled: Arc<AtomicBool>,
    }

    #[async_trait]
    impl BoundedWorkerDispatch for SettlingDispatch {
        async fn drain_once(&self) -> Result<u32, StoreError> {
            self.started.notify_one();
            tokio::time::sleep(Duration::from_millis(20)).await;
            self.settled.store(true, Ordering::Release);
            Ok(1)
        }
    }

    #[tokio::test(start_paused = true)]
    async fn shutdown_waits_for_the_active_bounded_pass_to_settle() {
        let store = Arc::new(MemoryStore::default());
        let tenant = TenantId::from_uuid(Uuid::from_u128(1));
        let context = TenantContext::from_authenticated_session(tenant);
        let job = store
            .enqueue_job(
                context,
                EnqueueJob {
                    payload: JobPayload::Render {
                        reference: ProblemVersionRef {
                            problem: ProblemId::from_uuid(Uuid::from_u128(2)),
                            version: VersionId::from_uuid(Uuid::from_u128(3)),
                        },
                        seed: 4,
                    },
                    max_attempts: 2,
                },
            )
            .await
            .expect("enqueue");
        let started = Arc::new(tokio::sync::Notify::new());
        let handler: Arc<dyn JobHandler> = Arc::new(SettlingHandler {
            started: Arc::clone(&started),
        });
        let committer: Arc<dyn EffectCommitter> = Arc::new(CompletingCommitter(Arc::clone(&store)));
        let registry =
            JobRegistry::new([JobRegistryEntry::new(JobKind::Render, handler, committer)])
                .expect("registry");
        let worker = Worker::new(
            Arc::clone(&store),
            registry,
            WorkerSettings::new(2, Duration::from_secs(1), 1).expect("settings"),
        );

        run_until_shutdown(worker, Duration::from_secs(60), started.notified())
            .await
            .expect("shutdown");
        assert_eq!(
            store.get_job(job).await.expect("view").expect("job").state,
            learning_data_access::JobState::Completed
        );
    }

    #[tokio::test(start_paused = true)]
    async fn bounded_dispatch_shutdown_waits_for_a_settled_fair_pass() {
        let started = Arc::new(tokio::sync::Notify::new());
        let settled = Arc::new(AtomicBool::new(false));
        let dispatch: Arc<dyn BoundedWorkerDispatch> = Arc::new(SettlingDispatch {
            started: Arc::clone(&started),
            settled: Arc::clone(&settled),
        });
        let shutdown = Arc::new(tokio::sync::Notify::new());
        let shutdown_wait = Arc::clone(&shutdown);
        let runtime = tokio::spawn(run_bounded_dispatch_until_shutdown(
            dispatch,
            Duration::from_secs(60),
            async move { shutdown_wait.notified().await },
        ));

        started.notified().await;
        shutdown.notify_one();
        tokio::time::advance(Duration::from_millis(20)).await;

        runtime
            .await
            .expect("runtime task")
            .expect("bounded dispatch shutdown");
        assert!(settled.load(Ordering::Acquire));
    }
}

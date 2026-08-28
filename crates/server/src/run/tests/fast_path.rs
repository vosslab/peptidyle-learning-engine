//! Fast-path route composition used by deterministic run-route tests.

use std::sync::Arc;
use std::time::Duration;

use learning_data_access::in_memory::MemoryStore;
use uuid::Uuid;

use super::super::RunBackend;
use crate::accepted_submission_worker::{
    AcceptedSubmissionExecutionWorker, AcceptedSubmissionFastPath,
};
use crate::worker::WorkerSettings;

/// Supplies the same exact-claim worker facade used by the server route,
/// without exposing a MemoryStore as grader authority.
pub(super) fn accepted_submission_fast_path<B>(
    store: &Arc<MemoryStore>,
    backend: Arc<B>,
) -> Arc<dyn AcceptedSubmissionFastPath>
where
    B: RunBackend + Send + Sync + 'static,
{
    let settings = WorkerSettings::new(60, Duration::from_secs(5), 1)
        .expect("bounded accepted-submission fast-path settings");
    Arc::new(
        AcceptedSubmissionExecutionWorker::new(
            (**store).clone(),
            backend,
            learning_data_access::WorkerId::from_uuid(Uuid::from_u128(70_002)),
            settings,
        )
        .expect("accepted-submission fast path"),
    )
}

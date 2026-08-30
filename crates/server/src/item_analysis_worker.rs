//! Private course item-analysis rebuild worker.
//!
//! Analytics is a distinct durable queue family. Its preparation and commit
//! never participate in scoring publication, so a failed analysis rebuild
//! cannot delay or roll back a grade.

use std::sync::Arc;

use async_trait::async_trait;
use learning_data_access::{
    CourseItemAnalysisCommitOutcome, CourseItemAnalysisWorkerCommand,
    CourseItemAnalysisWorkerStore, JobFailureKind, JobPayload, StoreError,
};

use crate::worker::{
    self, EffectCommitOutcome, EffectCommitter, JobCommitClaim, JobExecution, JobHandler,
    PreparedJobEffect,
};

/// Stages one still-current course analysis generation.
pub(crate) struct CourseItemAnalysisHandler<S> {
    store: Arc<S>,
}

impl<S> CourseItemAnalysisHandler<S> {
    pub(crate) fn new(store: Arc<S>) -> Self {
        Self { store }
    }
}

#[async_trait]
impl<S> JobHandler for CourseItemAnalysisHandler<S>
where
    S: CourseItemAnalysisWorkerStore + Send + Sync + 'static,
{
    async fn prepare(
        &self,
        payload: JobPayload,
        execution: JobExecution,
    ) -> Result<PreparedJobEffect, JobFailureKind> {
        let JobPayload::RecalculateCourseItemAnalysis {
            assignment,
            generation,
        } = payload
        else {
            return Err(JobFailureKind::Permanent);
        };
        let claim = execution.claim().ok_or(JobFailureKind::Permanent)?;
        if execution.cancellation_requested() {
            return Err(JobFailureKind::TimedOut);
        }
        self.store
            .prepare_course_item_analysis(CourseItemAnalysisWorkerCommand {
                job: claim.job_id(),
                lease: claim.lease_token(),
                assignment,
                generation,
            })
            .await
            .map_err(item_analysis_failure)?;
        if execution.cancellation_requested() {
            return Err(JobFailureKind::TimedOut);
        }
        Ok(PreparedJobEffect::CourseItemAnalysis {
            assignment,
            generation,
        })
    }
}

fn item_analysis_failure(error: StoreError) -> JobFailureKind {
    match error {
        StoreError::Unavailable(_) | StoreError::RetryableTransaction | StoreError::Conflict => {
            JobFailureKind::Transient
        }
        _ => JobFailureKind::Permanent,
    }
}

/// Sole visibility boundary for a staged course analysis generation.
pub(crate) struct CourseItemAnalysisCommitter<S> {
    store: Arc<S>,
}

impl<S> CourseItemAnalysisCommitter<S> {
    pub(crate) fn new(store: Arc<S>) -> Self {
        Self { store }
    }
}

impl<S> worker::sealed::EffectCommitter for CourseItemAnalysisCommitter<S> where
    S: Send + Sync + 'static
{
}

#[async_trait]
impl<S> EffectCommitter for CourseItemAnalysisCommitter<S>
where
    S: CourseItemAnalysisWorkerStore + Send + Sync + 'static,
{
    async fn commit(
        &self,
        claim: JobCommitClaim,
        effect: PreparedJobEffect,
    ) -> Result<EffectCommitOutcome, StoreError> {
        let PreparedJobEffect::CourseItemAnalysis {
            assignment,
            generation,
        } = effect
        else {
            return Err(StoreError::InvalidRecord(
                "course item-analysis committer received another effect family".to_string(),
            ));
        };
        match self
            .store
            .commit_course_item_analysis(CourseItemAnalysisWorkerCommand {
                job: claim.job_id(),
                lease: claim.lease_token(),
                assignment,
                generation,
            })
            .await?
        {
            CourseItemAnalysisCommitOutcome::Committed
            | CourseItemAnalysisCommitOutcome::Superseded => Ok(EffectCommitOutcome::Committed),
            CourseItemAnalysisCommitOutcome::ClaimNoLongerActive => {
                Ok(EffectCommitOutcome::ClaimNoLongerActive)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Mutex;

    use learning_data_access::{JobId, JobLeaseToken};
    use question_model::{AssignmentId, ScoringGeneration};
    use uuid::Uuid;

    use super::*;

    #[derive(Clone, Copy)]
    enum CommitBehavior {
        Committed,
        Superseded,
        ClaimNoLongerActive,
    }

    struct RecordingStore {
        prepared: Mutex<Vec<CourseItemAnalysisWorkerCommand>>,
        committed: Mutex<Vec<CourseItemAnalysisWorkerCommand>>,
        behavior: CommitBehavior,
    }

    #[async_trait]
    impl CourseItemAnalysisWorkerStore for RecordingStore {
        async fn prepare_course_item_analysis(
            &self,
            command: CourseItemAnalysisWorkerCommand,
        ) -> Result<(), StoreError> {
            self.prepared.lock().expect("test lock").push(command);
            Ok(())
        }

        async fn commit_course_item_analysis(
            &self,
            command: CourseItemAnalysisWorkerCommand,
        ) -> Result<CourseItemAnalysisCommitOutcome, StoreError> {
            self.committed.lock().expect("test lock").push(command);
            Ok(match self.behavior {
                CommitBehavior::Committed => CourseItemAnalysisCommitOutcome::Committed,
                CommitBehavior::Superseded => CourseItemAnalysisCommitOutcome::Superseded,
                CommitBehavior::ClaimNoLongerActive => {
                    CourseItemAnalysisCommitOutcome::ClaimNoLongerActive
                }
            })
        }
    }

    fn uuid(value: u128) -> Uuid {
        Uuid::from_u128(value)
    }

    fn claim() -> JobCommitClaim {
        JobCommitClaim::new(
            JobId::from_uuid(uuid(1)),
            JobLeaseToken::generate().expect("lease token"),
        )
    }

    fn payload() -> JobPayload {
        JobPayload::RecalculateCourseItemAnalysis {
            assignment: AssignmentId::from_uuid(uuid(2)),
            generation: ScoringGeneration::new(3).expect("positive generation"),
        }
    }

    #[tokio::test]
    async fn handler_prepares_only_the_closed_course_item_analysis_payload() {
        let store = Arc::new(RecordingStore {
            prepared: Mutex::new(Vec::new()),
            committed: Mutex::new(Vec::new()),
            behavior: CommitBehavior::Committed,
        });
        let handler = CourseItemAnalysisHandler::new(Arc::clone(&store));
        let claim = claim();
        let effect = handler
            .prepare(payload(), JobExecution::new().with_test_claim(claim))
            .await
            .expect("prepare");

        assert_eq!(
            effect,
            PreparedJobEffect::CourseItemAnalysis {
                assignment: AssignmentId::from_uuid(uuid(2)),
                generation: ScoringGeneration::new(3).expect("positive generation"),
            }
        );
        {
            let prepared = store.prepared.lock().expect("test lock");
            assert_eq!(prepared.len(), 1);
            assert_eq!(prepared[0].job, claim.job_id());
            assert_eq!(prepared[0].assignment, AssignmentId::from_uuid(uuid(2)));
        }

        assert_eq!(
            handler
                .prepare(
                    JobPayload::RecalculateAssignment {
                        assignment: AssignmentId::from_uuid(uuid(2)),
                        generation: ScoringGeneration::new(3).expect("positive generation"),
                    },
                    JobExecution::new().with_test_claim(claim),
                )
                .await,
            Err(JobFailureKind::Permanent)
        );
    }

    #[tokio::test]
    async fn committer_maps_visible_and_stale_outcomes_without_touching_scoring() {
        for (behavior, expected) in [
            (CommitBehavior::Committed, EffectCommitOutcome::Committed),
            (CommitBehavior::Superseded, EffectCommitOutcome::Committed),
            (
                CommitBehavior::ClaimNoLongerActive,
                EffectCommitOutcome::ClaimNoLongerActive,
            ),
        ] {
            let store = Arc::new(RecordingStore {
                prepared: Mutex::new(Vec::new()),
                committed: Mutex::new(Vec::new()),
                behavior,
            });
            let committer = CourseItemAnalysisCommitter::new(Arc::clone(&store));
            let claim = claim();
            let outcome = committer
                .commit(
                    claim,
                    PreparedJobEffect::CourseItemAnalysis {
                        assignment: AssignmentId::from_uuid(uuid(2)),
                        generation: ScoringGeneration::new(3).expect("positive generation"),
                    },
                )
                .await
                .expect("commit");
            assert_eq!(outcome, expected);
            assert_eq!(store.committed.lock().expect("test lock").len(), 1);
        }
    }
}

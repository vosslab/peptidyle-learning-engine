//! Server-private execution authority for committed rehearsal deliveries.
//!
//! This module deliberately sits between route-shaped rehearsal mutations and
//! family backends. It is not a browser DTO boundary; its only private
//! persistence input is the grader-only delivery facade after dispatch.

use std::sync::Arc;

use async_trait::async_trait;
use learning_data_access::{
    DispatchedRehearsalDelivery, RehearsalIdempotencyKey, RehearsalIssuedExecutionArtifactV1,
    RehearsalRouteIdentity, RehearsalRouteMutationStore, RehearsalSubmissionReceipt,
    SealedRehearsalDeliveryExecutionStore, SealedRehearsalDeliveryIssuePreparation,
    SealedRehearsalDeliveryIssueWork, SealedRehearsalGradingParts,
    SealedRehearsalSubmissionExecutionPreparation, SealedRehearsalSubmissionExecutionStore,
    StoreError, TenantContext,
};
use question_model::{RehearsalActiveScreenV1, RehearsalPrivateGradingResult};

use crate::run::RunBackendError;

/// Trusted family capability for issuing frozen rehearsal material.
///
/// It deliberately receives only Store-minted work: a route cannot replace
/// its snapshot, private execution contract, seed, or commit capability.
#[async_trait]
#[allow(dead_code)] // delivery routes call this capability after their route broker dispatches.
pub(crate) trait RehearsalIssueBackend: Send + Sync {
    async fn issue_frozen_rehearsal(
        &self,
        work: &SealedRehearsalDeliveryIssueWork,
    ) -> Result<RehearsalIssuedExecutionArtifactV1, RunBackendError>;
}

/// Trusted family capability for grading one sealed rehearsal submission.
///
/// The sealed work owns response translation and immutable execution material;
/// family backends receive no route DTO, browser key, or catalog lookup seam.
#[async_trait]
#[allow(dead_code)] // submission routes wire this sealed coordinator next.
pub(crate) trait RehearsalGradeBackend: Send + Sync {
    async fn grade_frozen_rehearsal(
        &self,
        work: SealedRehearsalGradingParts,
    ) -> Result<crate::run::GradeReceipt, RunBackendError>;
}

/// Closed private error contract for rehearsal delivery execution.
///
/// Route handlers can distinguish a supported-but-invalid frozen generation,
/// an intentionally unsupported family, and temporary execution failure
/// without parsing backend text.  Store protocol outcomes remain typed for
/// the route's existing authorization and concurrency mapping.
#[derive(Debug)]
#[allow(dead_code)] // route handlers consume every category as delivery endpoints land.
pub(crate) enum RehearsalExecutionError {
    Unsupported(String),
    Invalid(String),
    Unavailable(String),
    Store(StoreError),
}

impl From<StoreError> for RehearsalExecutionError {
    fn from(error: StoreError) -> Self {
        match error {
            StoreError::Unavailable(message) => Self::Unavailable(message),
            StoreError::InvalidRecord(message) => Self::Invalid(message),
            StoreError::RunModel(error) => Self::Invalid(error.to_string()),
            other => Self::Store(other),
        }
    }
}

/// Separately injected private authority for rehearsal delivery execution.
///
/// The backend registry and rehearsal sealed-execution capability are
/// intentionally distinct. In particular, the ordinary learner-run
/// `SealedPrivateExecutionStore` is not represented here.
pub(crate) struct RehearsalExecutionCoordinator<B> {
    backends: Arc<B>,
    sealed_delivery: Arc<dyn SealedRehearsalDeliveryExecutionStore>,
    #[allow(dead_code)] // consumed by the submission endpoint under construction.
    sealed_submission: Option<Arc<dyn SealedRehearsalSubmissionExecutionStore>>,
    #[allow(dead_code)] // consumed by the submission endpoint under construction.
    route_mutations: Option<Arc<dyn RehearsalRouteMutationStore>>,
}

impl<B> RehearsalExecutionCoordinator<B>
where
    B: RehearsalIssueBackend + 'static,
{
    pub(crate) fn new(
        backends: Arc<B>,
        sealed_delivery: Arc<dyn SealedRehearsalDeliveryExecutionStore>,
    ) -> Self {
        Self {
            backends,
            sealed_delivery,
            sealed_submission: None,
            route_mutations: None,
        }
    }

    /// Adds the separately injected recovery and completion authorities for
    /// deterministic rehearsal grading. Keeping these distinct from delivery
    /// sealing prevents a route from obtaining grader-only material.
    #[allow(dead_code)] // called by the production composition once submission routes land.
    pub(crate) fn with_submission_execution(
        backends: Arc<B>,
        sealed_delivery: Arc<dyn SealedRehearsalDeliveryExecutionStore>,
        sealed_submission: Arc<dyn SealedRehearsalSubmissionExecutionStore>,
        route_mutations: Arc<dyn RehearsalRouteMutationStore>,
    ) -> Self {
        Self {
            backends,
            sealed_delivery,
            sealed_submission: Some(sealed_submission),
            route_mutations: Some(route_mutations),
        }
    }

    /// The deterministic backend registry, available only to the execution
    /// coordinator's sibling handler implementation.
    #[allow(dead_code)] // called only by the deterministic delivery/submission coordinator.
    pub(crate) fn backends(&self) -> &Arc<B> {
        &self.backends
    }

    /// Hydrates only a committed, Store-minted delivery generation.  The
    /// erased facade prevents rehearsal handlers from gaining ordinary learner
    /// attempt execution authority.
    #[allow(dead_code)] // called only after a Store-minted dispatch commits.
    pub(crate) fn sealed_delivery(&self) -> &Arc<dyn SealedRehearsalDeliveryExecutionStore> {
        &self.sealed_delivery
    }

    /// Issues or resumes one already-dispatched generation and returns only
    /// the canonical public screen from its committed artifact.
    ///
    /// Store preparation is the sole resume authority.  Existing canonical
    /// bytes are projected directly, so a process crash after commit cannot
    /// cause a second generator invocation.  This is the server-side
    /// state-machine boundary required by ASVS 2.3.1 and 2.3.6.
    #[allow(dead_code)] // wired by the next route-owned delivery handler.
    pub(crate) async fn issue_or_resume(
        &self,
        context: TenantContext,
        dispatched: &DispatchedRehearsalDelivery,
    ) -> Result<RehearsalActiveScreenV1, RehearsalExecutionError> {
        match self
            .sealed_delivery
            .prepare_or_resume_issued_execution(context, dispatched)
            .await
            .map_err(RehearsalExecutionError::from)?
        {
            SealedRehearsalDeliveryIssuePreparation::ExistingArtifact(execution) => execution
                .active_screen()
                .map_err(RehearsalExecutionError::from),
            SealedRehearsalDeliveryIssuePreparation::IssueWork(work) => {
                let artifact = self
                    .backends
                    .issue_frozen_rehearsal(&work)
                    .await
                    .map_err(RehearsalExecutionError::from)?;
                self.sealed_delivery
                    .commit_issued_execution(context, *work, artifact)
                    .await
                    .map_err(RehearsalExecutionError::from)?
                    .active_screen()
                    .map_err(RehearsalExecutionError::from)
            }
        }
    }
}

impl<B> RehearsalExecutionCoordinator<B>
where
    B: RehearsalIssueBackend + RehearsalGradeBackend + 'static,
{
    /// Recovers and completes one route-authorized submission exactly once.
    ///
    /// A durable dispatched claim is the only permission to invoke a grader.
    /// Pending work is first dispatched by the route-keyed Store broker, then
    /// re-prepared through the sealed facade. This preserves sequential,
    /// atomic workflow authority (ASVS 2.3.1, 2.3.3) while allowing a process
    /// restart between every durable transition.
    #[allow(dead_code)] // called by the route-owned submission handler next.
    pub(crate) async fn grade_or_resume_submission(
        &self,
        context: TenantContext,
        route: RehearsalRouteIdentity,
        idempotency_key: RehearsalIdempotencyKey,
    ) -> Result<RehearsalSubmissionReceipt, RehearsalExecutionError> {
        let sealed_submission = self.sealed_submission.as_ref().ok_or_else(|| {
            RehearsalExecutionError::Unavailable(
                "sealed rehearsal submission execution is unavailable".into(),
            )
        })?;
        let route_mutations = self.route_mutations.as_ref().ok_or_else(|| {
            RehearsalExecutionError::Unavailable(
                "rehearsal submission route authority is unavailable".into(),
            )
        })?;
        let preparation = sealed_submission
            .prepare_or_resume_sealed_rehearsal_submission_execution(
                context,
                route,
                idempotency_key.clone(),
            )
            .await
            .map_err(RehearsalExecutionError::from)?;
        let preparation = match preparation {
            SealedRehearsalSubmissionExecutionPreparation::Receipt(receipt) => return Ok(receipt),
            SealedRehearsalSubmissionExecutionPreparation::Work(work) => work,
            SealedRehearsalSubmissionExecutionPreparation::PendingPreparation => {
                route_mutations
                    .dispatch_rehearsal_submission_from_route(
                        context,
                        route,
                        idempotency_key.clone(),
                    )
                    .await
                    .map_err(RehearsalExecutionError::from)?;
                match sealed_submission
                    .prepare_or_resume_sealed_rehearsal_submission_execution(
                        context,
                        route,
                        idempotency_key,
                    )
                    .await
                    .map_err(RehearsalExecutionError::from)?
                {
                    SealedRehearsalSubmissionExecutionPreparation::Receipt(receipt) => {
                        return Ok(receipt);
                    }
                    SealedRehearsalSubmissionExecutionPreparation::Work(work) => work,
                    SealedRehearsalSubmissionExecutionPreparation::PendingPreparation => {
                        return Err(RehearsalExecutionError::Unavailable(
                            "rehearsal submission dispatch did not make sealed grading work available".into(),
                        ));
                    }
                }
            }
        };
        // The sealed Store has already authenticated the exact route claim,
        // verified the committed artifact, and translated rendered browser
        // IDs. The server sees only this one deterministic grading input.
        let (parts, completion) = preparation.into_grading_and_completion();
        let grade = self
            .backends
            .grade_frozen_rehearsal(parts)
            .await
            .map_err(RehearsalExecutionError::from)?;
        let reference = completion
            .backend_receipt_reference()
            .map_err(RehearsalExecutionError::from)?;
        sealed_submission
            .complete_sealed_rehearsal_submission_execution(
                context,
                completion,
                RehearsalPrivateGradingResult::Graded {
                    result: grade.result,
                    feedback: instructor_rehearsal_feedback(grade.result, grade.feedback),
                    backend_receipt_reference: reference,
                },
            )
            .await
            .map_err(RehearsalExecutionError::from)
    }
}

/// An Instructor's own rehearsal is an instructor-safe trusted projection.
/// It retains only the bounded result and sanitized feedback supplied by the
/// family adapter, never private execution material or answer keys.
#[allow(dead_code)] // used by the route-owned submission handler next.
fn instructor_rehearsal_feedback(
    result: question_model::AttemptResult,
    content: question_model::FeedbackContent,
) -> question_model::DisclosedFeedback {
    question_model::DisclosedFeedback {
        correctness: Some(result.correct),
        points_earned: Some(result.points_earned),
        points_possible: Some(result.points_possible),
        hint: content.hint,
        correct_response: content.correct_response,
        rationale: content.rationale,
    }
}

impl From<RunBackendError> for RehearsalExecutionError {
    fn from(error: RunBackendError) -> Self {
        match error {
            RunBackendError::Unsupported(message) => Self::Unsupported(message),
            RunBackendError::Invalid(message) => Self::Invalid(message),
            RunBackendError::Unavailable(message) => Self::Unavailable(message),
        }
    }
}

#[cfg(test)]
#[path = "execution/tests.rs"]
mod tests;

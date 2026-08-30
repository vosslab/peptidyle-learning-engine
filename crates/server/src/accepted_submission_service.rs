//! Shared server-owned acceptance boundary for browser and deterministic seed input.

use learning_data_access::{
    AutomatedGradingStore, Store, StoreError, StudentWorkRoutingBinding, SubmissionPreparation,
    TenantContext,
};
use question_model::{QuestionAttemptId, StudentResponse, UserId};

use crate::accepted_submission_worker::{
    AcceptedSubmissionFastPath, AcceptedSubmissionHandlerResult,
};

/// Server-private submission input shared by HTTP delivery and host-only seeding.
#[derive(Clone)]
pub struct AcceptedSubmissionRequest {
    pub context: TenantContext,
    pub actor: UserId,
    pub binding: StudentWorkRoutingBinding,
    pub attempt: QuestionAttemptId,
    pub response: StudentResponse,
    pub idempotency_key: learning_data_access::SubmissionIdempotencyKey,
}

/// Answer-free acceptance outcome.  Routes project it to HTTP; host tools use
/// it only to decide whether their deterministic lifecycle may continue.
pub enum AcceptedSubmissionApplicationOutcome {
    Replay(Box<learning_data_access::SubmissionRecord>),
    Pending {
        attempt: QuestionAttemptId,
        reason: AcceptedSubmissionPendingReason,
    },
    Executed {
        attempt: QuestionAttemptId,
        result: AcceptedSubmissionHandlerResult,
        /// Exact follow-on scoring job created only by a committed grading
        /// result. Host compositions may execute this through the canonical
        /// scoring worker path when they require synchronous convergence.
        scoring_recalculation: Option<learning_data_access::JobId>,
    },
}

/// Server-internal reason durable acceptance still needs the recovery worker.
///
/// Browser routes intentionally project both variants to the same answer-free
/// pending response. Host-only installation can retain the safe Store error
/// needed to diagnose a failed synchronous execution composition.
pub enum AcceptedSubmissionPendingReason {
    AlreadyAccepted,
    FastPathFailed(StoreError),
}

/// Performs the sole transition from a validated response to accepted v2 work.
pub async fn accept_and_execute<S>(
    store: &S,
    automated_grading: &dyn AutomatedGradingStore,
    fast_path: &dyn AcceptedSubmissionFastPath,
    request: AcceptedSubmissionRequest,
) -> Result<AcceptedSubmissionApplicationOutcome, StoreError>
where
    S: Store,
{
    let preparation = store
        .prepare_question_submission(
            request.context,
            request.actor,
            request.binding,
            request.attempt,
            &request.response,
            &request.idempotency_key,
        )
        .await?;
    let intent = match preparation {
        SubmissionPreparation::Replay(record) => {
            return Ok(AcceptedSubmissionApplicationOutcome::Replay(record));
        }
        SubmissionPreparation::AcceptedPending(pending) => {
            return Ok(AcceptedSubmissionApplicationOutcome::Pending {
                attempt: pending.attempt(),
                reason: AcceptedSubmissionPendingReason::AlreadyAccepted,
            });
        }
        SubmissionPreparation::FirstEffect(intent) => intent,
    };
    let question = intent.issued_question_snapshot.question();
    if matches!(
        question.response,
        question_model::ResponseDefinition::FileUpload { .. }
    ) {
        return Err(StoreError::InvalidRecord(
            "graded file upload requires a deterministic server-owned grader".to_string(),
        ));
    }
    if matches!(
        question.response,
        question_model::ResponseDefinition::ExternalTool {}
    ) {
        return Err(StoreError::InvalidRecord(
            "external-tool submissions require the launch route".to_string(),
        ));
    }
    let valid = match intent.presentation.as_ref() {
        Some(snapshot) => domain::validation::validate_presentation_response_format(
            &snapshot.envelope.response,
            &request.response,
        )
        .is_valid(),
        None => domain::validation::validate_response_format(&question.response, &request.response)
            .is_valid(),
    };
    if !valid {
        return Err(StoreError::InvalidRecord(
            "response format is invalid".to_string(),
        ));
    }
    let execution_job = learning_data_access::JobId::generate()?;
    let accepted = automated_grading
        .accept_automated_submission(
            request.context,
            learning_data_access::AcceptedSubmissionCommand {
                actor: request.actor,
                course: request.binding.course,
                assignment: request.binding.assignment,
                attempt: request.attempt,
                idempotency_key: request.idempotency_key,
                response: request.response,
                execution_job,
            },
        )
        .await?;
    let result = match fast_path
        .execute_accepted_submission(learning_data_access::AcceptedSubmissionExecutionTarget {
            attempt: accepted.attempt,
            submission: accepted.submission,
            job: execution_job,
        })
        .await
    {
        Ok(result) => result,
        Err(error) => {
            // Acceptance and its recovery job are already durable. The
            // optional low-latency executor cannot turn that committed work
            // into a failed browser request; the ordinary worker now owns it.
            return Ok(AcceptedSubmissionApplicationOutcome::Pending {
                attempt: accepted.attempt,
                reason: AcceptedSubmissionPendingReason::FastPathFailed(error),
            });
        }
    };
    let scoring_recalculation = (result == AcceptedSubmissionHandlerResult::Committed)
        .then(|| learning_data_access::accepted_submission_recalculation_job(accepted.submission));
    Ok(AcceptedSubmissionApplicationOutcome::Executed {
        attempt: accepted.attempt,
        result,
        scoring_recalculation,
    })
}

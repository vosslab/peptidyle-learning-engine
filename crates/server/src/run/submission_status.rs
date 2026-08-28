//! Route-bound learner submission-status projection.

use super::contracts::RunBackend;
use super::submission::submission_response;
use super::support::*;

/// Returns the current answer-free automated-grading projection for one
/// learner-owned attempt.
///
/// The store receives the full route binding with the authenticated subject,
/// so an opaque attempt ID cannot select a record in another course or
/// assignment. The status response is deliberately a read-only projection:
/// it never retries a response or invokes a grader. ASVS 2.2.1, 2.3.1,
/// 8.2.1-8.2.2, and 8.3.1.
pub(super) async fn get_submission_status<S, B>(
    State(state): State<RunRouteState<S, B>>,
    headers: HeaderMap,
    Path((course, assignment, attempt_id)): Path<(CourseId, AssignmentId, QuestionAttemptId)>,
) -> Response
where
    S: Store + SessionStore + 'static,
    B: RunBackend + 'static,
{
    let authenticated = match resolve_request_session(state.store.as_ref(), &headers).await {
        Ok(authenticated) => authenticated,
        Err(error) => return auth_error_response(error),
    };
    let binding = LearnerWorkRoutingBinding::new(course, assignment);
    match learner_submission_status_projection(&state, &authenticated, binding, attempt_id).await {
        Ok(response) => response,
        Err(error) => store_error_response(error),
    }
}

/// Projects the one route-bound learner submission state shared by the status
/// GET and the synchronous post-acceptance fast path. It is a status reader,
/// never a receipt writer or grader invocation.
pub(super) async fn learner_submission_status_projection<S, B>(
    state: &RunRouteState<S, B>,
    authenticated: &AuthenticatedSession,
    binding: LearnerWorkRoutingBinding,
    attempt_id: QuestionAttemptId,
) -> Result<Response, StoreError>
where
    S: Store,
    B: RunBackend,
{
    match state
        .learner_submission_status
        .learner_submission_status(
            authenticated.tenant_context,
            authenticated.record.subject.user(),
            binding,
            attempt_id,
        )
        .await?
    {
        learning_data_access::LearnerSubmissionStatusRead::Completed {
            record,
            next_pending,
        } => {
            let scoring_status =
                learner_scoring_status(state.store.as_ref(), authenticated, record.run.enrollment)
                    .await;
            // A status read does not create successor work. The route-bound
            // store supplies only the immutable eligibility truth;
            // `start_or_resume_run` owns the later delivery transition.
            Ok(submission_response(
                *record,
                None,
                next_pending,
                scoring_status,
            ))
        }
        learning_data_access::LearnerSubmissionStatusRead::AcceptedPending(pending) => {
            Ok(accepted_pending_response(pending.attempt()))
        }
        learning_data_access::LearnerSubmissionStatusRead::InstructorAttention(pending) => {
            Ok(automated_submission_status_response(
                pending.attempt(),
                "instructor_attention",
                question_model::AutomatedGradingStatus::InstructorAttention,
            ))
        }
    }
}

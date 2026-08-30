//! Shared, private route state, browser DTOs, and error projections for runs.

pub(super) use std::sync::Arc;

pub(super) use crate::http_refusal::HttpResult;
pub(super) use axum::Json;
pub(super) use axum::body::to_bytes;
pub(super) use axum::extract::{Path, Query, State};
pub(super) use axum::http::{HeaderMap, StatusCode};
pub(super) use axum::response::{IntoResponse, Response};
pub(super) use learning_data_access::{
    AuthoritativeTimeStore, AutomatedGradingStore, CatalogStore, CourseAppearanceStore,
    CourseItemAnalysisStore, Cursor, IssueQuestionAttemptCommand, IssuedAttemptRead, PageRequest,
    PageSize, PaginationError, PresentationCapability, ReceiptNextAttempt,
    ReceiptPresentationSnapshot, ResolveEffectivePolicyCommand, SealedPrivateExecutionStore,
    SessionStore, Store, StoreError, StudentAssignmentSummarySnapshot,
    StudentSubmissionStatusStore, StudentWorkRoutingBinding, SubmissionIdempotencyKey,
    SubmissionRecord, TenantContext,
};
#[cfg(test)]
pub(super) use question_model::UserRole;
pub(super) use question_model::generation::Seed;
pub(super) use question_model::presentation::{
    AssetBindingV1, PresentationV1, build_presentation_v1,
};
pub(super) use question_model::{
    AssignmentEnrollment, AssignmentId, AssignmentRun, AssignmentRunItem, AttemptResult,
    CourseAppearance, CourseId, CourseSummary, DisclosedFeedback, FeedbackContent,
    PresentationBindingV1, ProblemVersionRef, QuestionAttempt, QuestionAttemptId,
    QuestionDefinition, QuestionEnvelope, RunId, StudentAssignmentProgress,
    StudentAssignmentSummary, StudentResponse,
};
pub(super) use serde::{Deserialize, Serialize};

pub(super) use crate::accepted_submission_worker::AcceptedSubmissionFastPath;
pub(super) use crate::auth::{
    AuthenticatedSession, auth_error_response, no_store, resolve_request_session,
};
pub(super) use domain::disclosure_policy::{project_disclosed_feedback, score_current_disclosure};

use super::contracts::RunBackendError;

pub(super) const DEFAULT_PAGE_SIZE: u16 = 50;
pub(super) const INTERNAL_ATTEMPT_PAGE_SIZE: u16 = PageSize::MAX;
pub(super) const MAX_SUBMISSION_BODY_BYTES: usize = 64 * 1024;
const IDEMPOTENCY_HEADER: &str = "idempotency-key";
pub(super) const MAX_JSON_SAFE_INTEGER: u64 = (1_u64 << 53) - 1;

/// Reads the atomic learner summary/status snapshot. A transient absence is
/// deliberately treated as failed for learner projections, never as current.
pub(super) async fn student_scoring_status<S: Store>(
    store: &S,
    authenticated: &AuthenticatedSession,
    enrollment: question_model::EnrollmentId,
) -> question_model::ScoringStatus {
    store
        .student_get_summary(
            authenticated.tenant_context,
            authenticated.record.subject.user(),
            enrollment,
        )
        .await
        .ok()
        .flatten()
        .map(|snapshot| snapshot.scoring_status)
        .unwrap_or(question_model::ScoringStatus::Failed)
}

pub(super) struct RunRouteState<S, B> {
    pub(super) store: Arc<S>,
    pub(super) backend: Arc<B>,
    /// The separately injected, capability-gated first-grade authority.
    ///
    /// It is deliberately not obtained through `Store`: ordinary route
    /// preparation is answer-free, while this facade alone can release the
    /// attempt-bound private grading contracts after the replay fence.
    pub(super) sealed_execution: Arc<dyn SealedPrivateExecutionStore>,
    /// Route-bound, answer-free learner recovery projection. Composition
    /// injects this narrower capability so the status read cannot grow into a
    /// second broad persistence dependency on browser-adjacent routes.
    pub(super) student_submission_status: Arc<dyn StudentSubmissionStatusStore>,
    /// First-effect acceptance capability. Submission owns its use; sibling
    /// route code receives no broad grading-operation authority.
    pub(super) automated_grading: Arc<dyn AutomatedGradingStore>,
    /// The one exact-claim execution capability used after durable acceptance.
    /// It exposes no private input, grader, result, or receipt-writer surface.
    pub(super) accepted_submission_fast_path: Arc<dyn AcceptedSubmissionFastPath>,
}

impl<S, B> Clone for RunRouteState<S, B> {
    fn clone(&self) -> Self {
        Self {
            store: Arc::clone(&self.store),
            backend: Arc::clone(&self.backend),
            sealed_execution: Arc::clone(&self.sealed_execution),
            student_submission_status: Arc::clone(&self.student_submission_status),
            automated_grading: Arc::clone(&self.automated_grading),
            accepted_submission_fast_path: Arc::clone(&self.accepted_submission_fast_path),
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct RunQuery {
    pub(super) cursor: Option<String>,
    pub(super) page_size: Option<u16>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct SubmitResponseRequest {
    pub(super) response: StudentResponse,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct SubmissionReceipt {
    /// Closed browser union discriminant for an immutable completed receipt.
    pub(super) kind: &'static str,
    pub(super) accepted: bool,
    pub(super) attempt: QuestionAttempt,
    pub(super) feedback: Option<DisclosedFeedback>,
    pub(super) scoring_status: question_model::ScoringStatus,
    /// Authoritative persisted run completion, independent of successor availability.
    pub(super) run_completion_status: question_model::RunCompletionStatus,
    pub(super) next_issued: Option<NextIssuedAttempt>,
    /// The durable receipt exists, but a successor has not yet been issued or
    /// delivered. The learner keeps feedback rather than retrying the answer.
    pub(super) next_pending: bool,
}

/// Answer-free replay projection while a server-owned grading execution is pending.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct AcceptedPendingResponse {
    kind: &'static str,
    accepted: bool,
    attempt_id: QuestionAttemptId,
    automated_grading_status: question_model::AutomatedGradingStatus,
    next_action: &'static str,
}

/// Learner attempt routes carry freshness separately from the attempt itself;
/// `QuestionAttempt` remains the shared raw record used by instructor paths.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct StudentAttemptProjection {
    #[serde(flatten)]
    pub(super) attempt: QuestionAttempt,
    pub(super) scoring_status: question_model::ScoringStatus,
    /// Visible placement inside the pool that supplied this issued item.
    pub(super) pool_selection: Option<PoolSelection>,
}

/// Answer-free learner-visible placement within one immutable pool draw.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct PoolSelection {
    /// One-based position in the ordered selected items from this pool.
    pub(super) item_number: u32,
    /// Number of issued items selected from the same pool.
    pub(super) item_count: u32,
}

/// Projects only the ordinal and size of the immutable selection group that
/// supplied an issued position. Group identities, candidates, and draw seeds
/// remain server-only evidence.
pub(super) fn pool_selection_for_position(
    run_items: &[AssignmentRunItem],
    issued_position: u32,
) -> Option<PoolSelection> {
    let group = run_items
        .iter()
        .find(|item| item.issued_position == issued_position)?
        .selection_group?;
    let mut selected = run_items
        .iter()
        .filter(|item| item.selection_group == Some(group))
        .collect::<Vec<_>>();
    selected.sort_by_key(|item| item.issued_position);
    let item_number = selected
        .iter()
        .position(|item| item.issued_position == issued_position)
        .and_then(|index| u32::try_from(index).ok())?
        .checked_add(1)?;
    let item_count = u32::try_from(selected.len()).ok()?;
    Some(PoolSelection {
        item_number,
        item_count,
    })
}

/// Browser-safe identity binding for a just-issued next attempt.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct NextIssuedAttempt {
    pub(super) id: QuestionAttemptId,
    pub(super) run: RunId,
    pub(super) question_version: question_model::VersionId,
    pub(super) seed: Seed,
    pub(super) deadline: Option<question_model::ActivityTimestamp>,
    pub(super) assignment_position: u32,
    pub(super) rendered_question_sha256: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct PrefetchedNextQuestion {
    pub(super) predecessor: QuestionAttemptId,
    pub(super) run: RunId,
    pub(super) assignment_position: u32,
    pub(super) question_version: question_model::VersionId,
    pub(super) seed: Seed,
    pub(super) rendered_question_sha256: String,
    /// Visible placement inside the pool that supplied this next question.
    pub(super) pool_selection: Option<PoolSelection>,
    pub(super) envelope: QuestionEnvelope,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct RunSummaryOutcome {
    pub(super) attempt: QuestionAttemptId,
    pub(super) assignment_position: u32,
    pub(super) submitted_at: Option<question_model::ActivityTimestamp>,
    pub(super) response: Option<StudentResponse>,
    pub(super) feedback: Option<DisclosedFeedback>,
    pub(super) scoring_status: question_model::ScoringStatus,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct RunSummaryResponse {
    pub(super) course: CourseRouteData,
    pub(super) run: AssignmentRun,
    pub(super) summary: StudentAssignmentProgress,
    pub(super) practice_allowed: bool,
    pub(super) outcomes: learning_data_access::Page<RunSummaryOutcome>,
}

/// Historical course-instructor projection for a learner's run.
///
/// This route intentionally keeps the raw, instructor-authorized aggregate
/// and run score separate from the learner DTO. Attempt feedback remains the
/// same bounded presentation projection, so this response never carries an
/// answer key, checker, source, or other private question material.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct InstructorRunSummaryResponse {
    pub(super) course: CourseRouteData,
    pub(super) run: AssignmentRun,
    pub(super) summary: StudentAssignmentSummary,
    pub(super) scoring_status: question_model::ScoringStatus,
    pub(super) practice_allowed: bool,
    pub(super) outcomes: learning_data_access::Page<RunSummaryOutcome>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct CourseRouteData {
    pub(super) summary: CourseSummary,
    pub(super) appearance: CourseAppearance,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct FeedbackReleaseResponse {
    pub(super) released: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct EnrollmentView {
    pub(super) enrollment: AssignmentEnrollment,
    pub(super) summary: StudentAssignmentProgress,
}

/// Projects aggregate progress only after S5 entitlement and S3's current
/// effective-policy resolver have accepted the learner's access.
pub(crate) async fn student_assignment_progress<
    S: Store + AuthoritativeTimeStore + CourseItemAnalysisStore,
>(
    store: &S,
    authenticated: &AuthenticatedSession,
    assignment_id: question_model::AssignmentId,
    snapshot: Option<&StudentAssignmentSummarySnapshot>,
) -> HttpResult<(StudentAssignmentProgress, bool)> {
    let assignment = store
        .get_assignment_for_edit(authenticated.tenant_context, assignment_id)
        .await
        .map_err(store_error_response)?
        .ok_or_else(|| error_response(StatusCode::NOT_FOUND, "assignment not found"))?;
    let entitlement = store
        .evaluate_assignment_entitlement(
            authenticated.tenant_context,
            authenticated.record.subject.user(),
            assignment.record.course_id,
            assignment.record.id,
        )
        .await
        .map_err(store_error_response)?;
    if !matches!(
        entitlement,
        domain::entitlement::EntitlementDecision::Granted(_)
    ) {
        return Err(error_response(StatusCode::NOT_FOUND, "assignment not found").into());
    }
    // Even an empty summary must pass both S5 and S3: DuringAttempt may
    // disclose class statistics before the learner has submitted work.
    let now = store
        .authoritative_time()
        .await
        .map_err(store_error_response)?;
    let resolution = store
        .resolve_effective_policy(
            authenticated.tenant_context,
            ResolveEffectivePolicyCommand {
                assignment: assignment.record.id,
                entitlement,
                authorization: domain::effective_assignment_policy::AuthorizationGate::Authorized,
                now,
                // S3 uses this input only for its start verdict; the resolved
                // due/close policy consumed below is independent of it. The
                // compact summary avoids an unbounded learner route scan.
                prior_run_count: snapshot
                    .map(|snapshot| snapshot.summary.completed_run_count)
                    .unwrap_or(0),
            },
        )
        .await
        .map_err(store_error_response)?
        .ok_or_else(|| error_response(StatusCode::NOT_FOUND, "assignment not found"))?;
    let decision = domain::disclosure_policy::evaluate_student_disclosure(
        assignment.record.disclosure_policy,
        &resolution.decision,
        now,
        // Starting a run updates aggregate activity, but AfterSubmit needs
        // evidence of an actual submitted response. The compact summary has
        // that evidence without scanning attempts.
        snapshot.and_then(|snapshot| {
            (snapshot.summary.total_question_attempts > 0)
                .then_some(snapshot.summary.last_activity_at)
                .flatten()
        }),
    )
    .ok_or_else(|| error_response(StatusCode::NOT_FOUND, "assignment not found"))?;
    let mut progress = match snapshot {
        Some(snapshot) => StudentAssignmentProgress::from_summary(
            &snapshot.summary,
            decision.score,
            snapshot.scoring_status,
        ),
        None => StudentAssignmentProgress::no_activity(assignment.scoring_status),
    };
    if decision.class_statistics {
        progress.class_statistics = Some(
            store
                .student_class_statistics(
                    authenticated.tenant_context,
                    authenticated.record.subject.user(),
                    assignment.record.course_id,
                    assignment.record.id,
                )
                .await
                .map_err(store_error_response)?,
        );
    }
    Ok((progress, decision.score))
}

pub(super) fn fresh_seed() -> Result<u64, RunBackendError> {
    let mut bytes = [0_u8; 8];
    getrandom::fill(&mut bytes).map_err(|error| RunBackendError::Unavailable(error.to_string()))?;
    Ok(u64::from_le_bytes(bytes) & MAX_JSON_SAFE_INTEGER)
}

pub(super) fn submission_key(
    headers: &HeaderMap,
) -> Result<SubmissionIdempotencyKey, &'static str> {
    let value = headers
        .get(IDEMPOTENCY_HEADER)
        .ok_or("idempotency-key is required")?
        .to_str()
        .map_err(|_| "idempotency-key is invalid")?;
    SubmissionIdempotencyKey::parse(value).map_err(|_| "idempotency-key is invalid")
}

pub(super) fn page_request(query: RunQuery) -> Result<PageRequest, PaginationError> {
    let size = PageSize::new(query.page_size.unwrap_or(DEFAULT_PAGE_SIZE))?;
    match query.cursor {
        Some(cursor) => Ok(PageRequest::after(Cursor::parse(cursor)?, size)),
        None => Ok(PageRequest::first(size)),
    }
}

pub(super) fn backend_error_response(error: RunBackendError) -> Response {
    match error {
        RunBackendError::Unsupported(message) | RunBackendError::Invalid(message) => {
            error_response(StatusCode::UNPROCESSABLE_ENTITY, &message)
        }
        RunBackendError::Unavailable(_) | RunBackendError::Deterministic(_) => error_response(
            StatusCode::SERVICE_UNAVAILABLE,
            "question backend unavailable",
        ),
    }
}

pub(super) fn accepted_pending_response(attempt_id: QuestionAttemptId) -> Response {
    automated_submission_status_response(
        attempt_id,
        "accepted_pending",
        question_model::AutomatedGradingStatus::Pending,
    )
}

/// Projects a durable non-completed automated-grading state without exposing
/// the accepted response, receipt, execution, feedback, or score.
///
/// The exact learner route witness has already been validated by the caller.
/// Keeping this closed projection in one helper prevents POST replay and the
/// status GET from drifting into separate browser contracts.
pub(super) fn automated_submission_status_response(
    attempt_id: QuestionAttemptId,
    kind: &'static str,
    automated_grading_status: question_model::AutomatedGradingStatus,
) -> Response {
    no_store(
        (
            StatusCode::ACCEPTED,
            Json(AcceptedPendingResponse {
                kind,
                accepted: true,
                attempt_id,
                automated_grading_status,
                next_action: "check_status",
            }),
        )
            .into_response(),
    )
}

pub(super) fn store_error_response(error: StoreError) -> Response {
    match error {
        StoreError::NotFound => error_response(StatusCode::NOT_FOUND, "record not found"),
        StoreError::AlreadyExists | StoreError::Conflict => {
            error_response(StatusCode::CONFLICT, "record changed or already exists")
        }
        StoreError::OwnershipMismatch | StoreError::Forbidden => {
            error_response(StatusCode::FORBIDDEN, "operation is not authorized")
        }
        StoreError::InvalidRecord(message) => {
            error_response(StatusCode::UNPROCESSABLE_ENTITY, &message)
        }
        StoreError::RunModel(error) => {
            error_response(StatusCode::UNPROCESSABLE_ENTITY, &error.to_string())
        }
        StoreError::TimedOut => error_response(StatusCode::CONFLICT, "question attempt timed out"),
        StoreError::RetryableTransaction | StoreError::Unavailable(_) => {
            error_response(StatusCode::SERVICE_UNAVAILABLE, "run storage unavailable")
        }
    }
}

pub(super) fn error_response(status: StatusCode, message: &str) -> Response {
    no_store((status, Json(serde_json::json!({ "error": message }))).into_response())
}

pub(super) async fn no_store_response(response: Response) -> Response {
    no_store(response)
}

#[cfg(test)]
mod pool_selection_tests {
    use super::*;
    use crate::run::contracts::DeterministicGraderFailure;
    use question_model::{AssignmentItemId, AssignmentSelectionGroupId, ProblemId, VersionId};
    use uuid::Uuid;

    fn id(value: u128) -> Uuid {
        Uuid::from_u128(value)
    }

    fn item(position: u32, group: Option<u128>) -> AssignmentRunItem {
        AssignmentRunItem {
            run: RunId::from_uuid(id(1)),
            assignment_item: AssignmentItemId::from_uuid(id(10 + u128::from(position))),
            source_position: position,
            issued_position: position,
            reference: ProblemVersionRef {
                problem: ProblemId::from_uuid(id(20 + u128::from(position))),
                version: VersionId::from_uuid(id(30 + u128::from(position))),
            },
            statistics_eligible: true,
            selection_group: group.map(|value| AssignmentSelectionGroupId::from_uuid(id(value))),
            selection_seed: group.map(|_| 42),
        }
    }

    #[test]
    fn fixed_items_have_no_pool_selection() {
        assert_eq!(pool_selection_for_position(&[item(0, None)], 0), None);
    }

    #[test]
    fn pooled_items_report_one_based_issued_order_and_group_size() {
        let items = vec![item(2, Some(90)), item(0, None), item(1, Some(90))];

        assert_eq!(
            pool_selection_for_position(&items, 2),
            Some(PoolSelection {
                item_number: 2,
                item_count: 2,
            })
        );
    }

    #[test]
    fn pool_selection_serialization_exposes_counts_without_private_draw_evidence() {
        let json = serde_json::to_value(PoolSelection {
            item_number: 1,
            item_count: 2,
        })
        .expect("pool selection serializes");

        assert_eq!(json, serde_json::json!({"itemNumber": 1, "itemCount": 2}));
        assert!(
            !json.to_string().contains("selectionGroup")
                && !json.to_string().contains("selectionSeed")
                && !json.to_string().contains("candidate")
        );
    }

    #[tokio::test]
    async fn deterministic_grader_error_is_generic_and_no_store() {
        let response = backend_error_response(RunBackendError::Deterministic(
            DeterministicGraderFailure::IssuedEvidenceIntegrity,
        ));

        assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
        assert_eq!(response.headers()["cache-control"], "no-store");
        let body = to_bytes(response.into_body(), 1_024)
            .await
            .expect("error response body");
        assert_eq!(
            serde_json::from_slice::<serde_json::Value>(&body).expect("error response JSON"),
            serde_json::json!({ "error": "question backend unavailable" })
        );
    }
}

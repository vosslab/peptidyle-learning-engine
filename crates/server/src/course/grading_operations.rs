//! Instructor-only, assignment-local automated-grading recovery routes.
//!
//! This boundary accepts route coordinates, closed query values, and guarded
//! action headers. It never accepts an actor, score, learner response, or
//! operation state from the browser. The store receives the resolved session
//! hash and repeats Instructor authority inside its transaction.

use axum::Json;
use axum::body::to_bytes;
use axum::extract::{Path, RawQuery, Request, State};
use axum::http::header::{ETAG, IF_MATCH};
use axum::http::{HeaderMap, HeaderValue, StatusCode};
use axum::response::{IntoResponse, Response};
use learning_data_access::{
    AssignmentRevision, Cursor, GradingOperationActionId, GradingOperationActionReceipt,
    GradingOperationGroup, GradingOperationGroupBy, GradingOperationRevision,
    GradingOperationStore, GradingOperationTrustGeneration, InstructorGradingOperationRow,
    ListInstructorGradingOperationsCommand, Page, PageRequest, PageSize,
    RecalculateAssignmentCommand, RetryGradingOperationCommand, SessionStore, Store, StoreError,
};
use question_model::{
    AssignmentId, CourseId, CourseMembershipRole, GradingOperationReference, UserRole,
};
use serde::Serialize;

use super::projection::{error_response, store_error_response};
use super::routing::{CourseRouteState, DEFAULT_PAGE_SIZE};
use crate::auth::{auth_error_response, no_store, resolve_request_session};
use crate::http_refusal::HttpResult;

const IDEMPOTENCY_KEY: &str = "idempotency-key";
const MAX_EMPTY_ACTION_BODY_BYTES: usize = 1;

pub(in crate::course) async fn list_grading_operations<S>(
    State(state): State<CourseRouteState<S>>,
    Path((course, assignment)): Path<(CourseId, AssignmentId)>,
    headers: HeaderMap,
    RawQuery(raw_query): RawQuery,
) -> Response
where
    S: Store
        + learning_data_access::CourseRecordsAccessStore
        + SessionStore
        + GradingOperationStore
        + 'static,
{
    let authenticated = match resolve_request_session(state.store.as_ref(), &headers).await {
        Ok(value) => value,
        Err(error) => return auth_error_response(error),
    };
    if let Err(response) =
        require_direct_instructor(state.store.as_ref(), &authenticated, course).await
    {
        return response.into_response();
    }
    let (group_by, page) = match list_request(raw_query.as_deref()) {
        Ok(value) => value,
        Err(message) => return error_response(StatusCode::BAD_REQUEST, message),
    };
    match state
        .store
        .list_instructor_grading_operations(
            authenticated.tenant_context,
            ListInstructorGradingOperationsCommand {
                session: authenticated.session_hash,
                course,
                assignment,
                group_by,
                page,
            },
        )
        .await
    {
        Ok(page) => no_store(Json(GradingOperationsPageView::from(page)).into_response()),
        Err(error) => grading_operation_store_error(error),
    }
}

pub(in crate::course) async fn retry_grading_operation<S>(
    State(state): State<CourseRouteState<S>>,
    Path((course, assignment, operation)): Path<(
        CourseId,
        AssignmentId,
        GradingOperationReference,
    )>,
    request: Request,
) -> Response
where
    S: Store
        + learning_data_access::CourseRecordsAccessStore
        + SessionStore
        + GradingOperationStore
        + 'static,
{
    let authenticated = match resolve_request_session(state.store.as_ref(), request.headers()).await
    {
        Ok(value) => value,
        Err(error) => return auth_error_response(error),
    };
    if let Err(response) =
        require_direct_instructor(state.store.as_ref(), &authenticated, course).await
    {
        return response.into_response();
    }
    let expected_revision = match required_operation_revision(request.headers()) {
        Ok(value) => value,
        Err(response) => return response.into_response(),
    };
    let action = match required_action_id(request.headers()) {
        Ok(value) => value,
        Err(response) => return response.into_response(),
    };
    if let Err(response) = require_empty_action_body(request).await {
        return response.into_response();
    }
    match state
        .store
        .retry_instructor_grading_operation(
            authenticated.tenant_context,
            RetryGradingOperationCommand {
                session: authenticated.session_hash,
                course,
                assignment,
                operation,
                action,
                expected_revision,
            },
        )
        .await
    {
        Ok(receipt) => action_receipt_response(receipt),
        Err(error) => grading_operation_store_error(error),
    }
}

pub(in crate::course) async fn recalculate_assignment<S>(
    State(state): State<CourseRouteState<S>>,
    Path((course, assignment)): Path<(CourseId, AssignmentId)>,
    request: Request,
) -> Response
where
    S: Store
        + learning_data_access::CourseRecordsAccessStore
        + SessionStore
        + GradingOperationStore
        + 'static,
{
    let authenticated = match resolve_request_session(state.store.as_ref(), request.headers()).await
    {
        Ok(value) => value,
        Err(error) => return auth_error_response(error),
    };
    if let Err(response) =
        require_direct_instructor(state.store.as_ref(), &authenticated, course).await
    {
        return response.into_response();
    }
    let expected_assignment_revision = match required_assignment_revision(request.headers()) {
        Ok(value) => value,
        Err(response) => return response.into_response(),
    };
    let action = match required_action_id(request.headers()) {
        Ok(value) => value,
        Err(response) => return response.into_response(),
    };
    if let Err(response) = require_empty_action_body(request).await {
        return response.into_response();
    }
    match state
        .store
        .recalculate_instructor_assignment(
            authenticated.tenant_context,
            RecalculateAssignmentCommand {
                session: authenticated.session_hash,
                course,
                assignment,
                action,
                expected_assignment_revision,
            },
        )
        .await
    {
        Ok(receipt) => action_receipt_response(receipt),
        Err(error) => grading_operation_store_error(error),
    }
}

/// Route-level concealment protects the operation surface before controlled
/// browser inputs are interpreted. The store repeats this session-bound check.
async fn require_direct_instructor<S>(
    store: &S,
    authenticated: &crate::auth::AuthenticatedSession,
    course: CourseId,
) -> HttpResult<()>
where
    S: Store + learning_data_access::CourseRecordsAccessStore,
{
    // ASVS 8.2.1: require explicit Instructor permission at the trusted route layer.
    if authenticated.record.subject.role() != UserRole::Instructor {
        return Err(error_response(StatusCode::NOT_FOUND, "grading operations not found").into());
    }
    match store
        .get_current_course_membership(
            authenticated.tenant_context,
            course,
            authenticated.record.subject.user(),
        )
        .await
    {
        Ok(Some(membership)) if membership.role == CourseMembershipRole::Instructor => {
            match store.course_records_accessible(course).await {
                Ok(true) => Ok(()),
                Ok(false) | Err(StoreError::NotFound) | Err(StoreError::Forbidden) => Err(
                    error_response(StatusCode::NOT_FOUND, "grading operations not found").into(),
                ),
                Err(error) => Err(store_error_response(error).into()),
            }
        }
        Ok(Some(_)) | Ok(None) | Err(StoreError::NotFound) | Err(StoreError::Forbidden) => {
            Err(error_response(StatusCode::NOT_FOUND, "grading operations not found").into())
        }
        Err(error) => Err(store_error_response(error).into()),
    }
}

fn list_request(
    raw_query: Option<&str>,
) -> Result<(GradingOperationGroupBy, PageRequest), &'static str> {
    let mut group_by = None;
    let mut cursor = None;
    let mut page_size = None;
    for (key, value) in url::form_urlencoded::parse(raw_query.unwrap_or_default().as_bytes()) {
        let slot = match key.as_ref() {
            "groupBy" => &mut group_by,
            "cursor" => &mut cursor,
            "pageSize" => &mut page_size,
            _ => return Err("grading operations query is invalid"),
        };
        if slot.is_some() {
            return Err("grading operations query is invalid");
        }
        *slot = Some(value.into_owned());
    }
    let group_by = match group_by.as_deref().unwrap_or("question") {
        "question" => GradingOperationGroupBy::Question,
        "learner" => GradingOperationGroupBy::Learner,
        _ => return Err("groupBy must be question or learner"),
    };
    let size = match page_size {
        Some(value) => value
            .parse::<u16>()
            .ok()
            .and_then(|value| PageSize::new(value).ok())
            .ok_or("pageSize must be between 1 and 100")?,
        None => PageSize::new(DEFAULT_PAGE_SIZE).expect("default page size is bounded"),
    };
    let page = match cursor {
        Some(value) => PageRequest::after(
            Cursor::parse(value).map_err(|_| "cursor must not be empty")?,
            size,
        ),
        None => PageRequest::first(size),
    };
    Ok((group_by, page))
}

fn required_operation_revision(headers: &HeaderMap) -> HttpResult<GradingOperationRevision> {
    GradingOperationRevision::from_u64(required_strong_revision(headers)?)
        .ok_or_else(|| error_response(StatusCode::BAD_REQUEST, "If-Match is malformed").into())
}

fn required_assignment_revision(headers: &HeaderMap) -> HttpResult<AssignmentRevision> {
    AssignmentRevision::new(required_strong_revision(headers)?)
        .ok_or_else(|| error_response(StatusCode::BAD_REQUEST, "If-Match is malformed").into())
}

fn required_strong_revision(headers: &HeaderMap) -> HttpResult<u64> {
    let mut values = headers.get_all(IF_MATCH).iter();
    let Some(value) = values.next() else {
        return Err(
            error_response(StatusCode::PRECONDITION_REQUIRED, "If-Match is required").into(),
        );
    };
    if values.next().is_some() {
        return Err(error_response(StatusCode::BAD_REQUEST, "If-Match is malformed").into());
    }
    let value = value
        .to_str()
        .ok()
        .and_then(|value| {
            value
                .strip_prefix('"')
                .and_then(|value| value.strip_suffix('"'))
        })
        .filter(|value| {
            !value.is_empty()
                && !value.starts_with('0')
                && value.bytes().all(|byte| byte.is_ascii_digit())
        })
        .and_then(|value| value.parse::<u64>().ok());
    value.ok_or_else(|| error_response(StatusCode::BAD_REQUEST, "If-Match is malformed").into())
}

fn required_action_id(headers: &HeaderMap) -> HttpResult<GradingOperationActionId> {
    let mut values = headers.get_all(IDEMPOTENCY_KEY).iter();
    let Some(value) = values.next() else {
        return Err(error_response(StatusCode::BAD_REQUEST, "Idempotency-Key is required").into());
    };
    if values.next().is_some() {
        return Err(error_response(StatusCode::BAD_REQUEST, "Idempotency-Key is malformed").into());
    }
    value
        .to_str()
        .ok()
        .and_then(|value| uuid::Uuid::parse_str(value).ok())
        .map(GradingOperationActionId::from_uuid)
        .ok_or_else(|| {
            error_response(StatusCode::BAD_REQUEST, "Idempotency-Key is malformed").into()
        })
}

async fn require_empty_action_body(request: Request) -> HttpResult<()> {
    match to_bytes(request.into_body(), MAX_EMPTY_ACTION_BODY_BYTES).await {
        Ok(body) if body.is_empty() => Ok(()),
        Ok(_) | Err(_) => Err(error_response(
            StatusCode::BAD_REQUEST,
            "grading operation actions require an empty body",
        )
        .into()),
    }
}

fn grading_operation_store_error(error: StoreError) -> Response {
    match error {
        StoreError::NotFound | StoreError::Forbidden | StoreError::OwnershipMismatch => {
            error_response(StatusCode::NOT_FOUND, "grading operations not found")
        }
        error => store_error_response(error),
    }
}

fn action_receipt_response(receipt: GradingOperationActionReceipt) -> Response {
    let (etag, view) = match receipt {
        GradingOperationActionReceipt::Retry {
            action,
            operation,
            resulting_operation_revision,
            safe_category: _,
            occurred_at,
        } => (
            format!("\"{}\"", resulting_operation_revision.as_u64()),
            GradingOperationActionReceiptView::Retry {
                action: action.as_uuid(),
                operation,
                resulting_operation_revision: resulting_operation_revision.as_u64(),
                occurred_at,
            },
        ),
        GradingOperationActionReceipt::Recalculation {
            action,
            operation,
            resulting_operation_revision,
            assignment_revision,
            scoring_generation,
            safe_category: _,
            occurred_at,
        } => (
            format!("\"{}\"", assignment_revision.value()),
            GradingOperationActionReceiptView::Recalculation {
                action: action.as_uuid(),
                operation,
                resulting_operation_revision: resulting_operation_revision.as_u64(),
                assignment_revision: assignment_revision.value(),
                scoring_generation: scoring_generation.value(),
                occurred_at,
            },
        ),
    };
    let mut response = Json(view).into_response();
    response.headers_mut().insert(
        ETAG,
        HeaderValue::from_str(&etag).expect("positive resource revision is a strong ETag"),
    );
    no_store(response)
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct GradingOperationsPageView {
    items: Vec<InstructorGradingOperationRowView>,
    next_cursor: Option<String>,
}

impl From<Page<InstructorGradingOperationRow>> for GradingOperationsPageView {
    fn from(value: Page<InstructorGradingOperationRow>) -> Self {
        Self {
            items: value.items.into_iter().map(Into::into).collect(),
            next_cursor: value.next_cursor.map(|value| value.as_str().to_owned()),
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct InstructorGradingOperationRowView {
    operation: InstructorGradingOperationView,
    group: GradingOperationGroupView,
    affected_learner_count: u32,
    trust_generation: GradingOperationTrustGenerationView,
}

impl From<InstructorGradingOperationRow> for InstructorGradingOperationRowView {
    fn from(value: InstructorGradingOperationRow) -> Self {
        Self {
            operation: InstructorGradingOperationView {
                reference: value.operation.reference,
                reason: value.operation.reason,
                state: value.operation.state,
                revision: value.operation.revision.as_u64(),
                next_action: value.operation.next_action,
            },
            group: value.group.into(),
            affected_learner_count: value.affected_learner_count,
            trust_generation: value.trust_generation.into(),
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct InstructorGradingOperationView {
    reference: GradingOperationReference,
    reason: question_model::GradingOperationReason,
    state: question_model::GradingOperationState,
    revision: u64,
    next_action: Option<question_model::GradingOperationAction>,
}

#[derive(Debug, Serialize)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
enum GradingOperationGroupView {
    Question {
        question_id: question_model::QuestionId,
        title: String,
    },
    Learner {
        membership: question_model::CourseMembershipReference,
        display_name: question_model::TeachingDisplayLabel,
    },
    Assignment,
}

impl From<GradingOperationGroup> for GradingOperationGroupView {
    fn from(value: GradingOperationGroup) -> Self {
        match value {
            GradingOperationGroup::Question { question_id, title } => {
                Self::Question { question_id, title }
            }
            GradingOperationGroup::Learner {
                membership,
                display_name,
            } => Self::Learner {
                membership,
                display_name,
            },
            GradingOperationGroup::Assignment => Self::Assignment,
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
enum GradingOperationTrustGenerationView {
    Execution { generation: u64 },
    AssignmentScoring { generation: u64 },
}

impl From<GradingOperationTrustGeneration> for GradingOperationTrustGenerationView {
    fn from(value: GradingOperationTrustGeneration) -> Self {
        match value {
            GradingOperationTrustGeneration::Execution(value) => Self::Execution {
                generation: value.as_u64(),
            },
            GradingOperationTrustGeneration::AssignmentScoring(value) => Self::AssignmentScoring {
                generation: value.value(),
            },
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
enum GradingOperationActionReceiptView {
    Retry {
        action: uuid::Uuid,
        operation: GradingOperationReference,
        resulting_operation_revision: u64,
        occurred_at: question_model::ActivityTimestamp,
    },
    Recalculation {
        action: uuid::Uuid,
        operation: GradingOperationReference,
        resulting_operation_revision: u64,
        assignment_revision: u64,
        scoring_generation: u64,
        occurred_at: question_model::ActivityTimestamp,
    },
}

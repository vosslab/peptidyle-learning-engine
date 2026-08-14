use axum::Json;
use axum::extract::{Path, State};
use axum::http::header::{ETAG, IF_MATCH};
use axum::http::{HeaderMap, HeaderValue, StatusCode};
use axum::response::{IntoResponse, Response};
use learning_data_access::{
    AssignmentRecord, AssignmentRevision, CatalogStore, CourseRecordsAccessStore, SessionStore,
    Store, StoreError, StoredAssignment,
};
use question_model::{
    AssignmentDeliveryState, AssignmentId, AssignmentItem, AssignmentScoringMode, Capability,
    CourseId, PointValue, ProblemVersionRef,
};
use serde::Serialize;

use crate::auth::{auth_error_response, no_store, resolve_request_session};

use super::policy::require_course_access;
use super::projection::{error_response, store_error_response};
use super::routing::{
    CourseRouteState, CreateAssignmentRequest, UpdateAssignmentRequest, strict_assignment_request,
};

pub(super) async fn create_assignment<S>(
    State(state): State<CourseRouteState<S>>,
    headers: HeaderMap,
    Path(course): Path<CourseId>,
    Json(value): Json<serde_json::Value>,
) -> Response
where
    S: Store + CatalogStore + CourseRecordsAccessStore + SessionStore + 'static,
{
    let authenticated = match resolve_request_session(state.store.as_ref(), &headers).await {
        Ok(authenticated) => authenticated,
        Err(error) => return auth_error_response(error),
    };
    if let Err(response) =
        require_course_access(state.store.as_ref(), &authenticated, course, true).await
    {
        return response;
    }
    let request = match strict_assignment_request::<CreateAssignmentRequest>(value) {
        Ok(request) => request,
        Err(()) => {
            return error_response(
                StatusCode::UNPROCESSABLE_ENTITY,
                "assignment request is invalid",
            );
        }
    };
    let assignment = AssignmentRecord {
        id: AssignmentId::generate(),
        tenant: authenticated.tenant_context.tenant_id(),
        course_id: course,
        title: request.title,
        items: assignment_items(request.problems, None),
        selection_groups: Vec::new(),
        policies: request.policies,
    };
    if let Err(response) =
        validate_assignment_request(&state, authenticated.tenant_context, &assignment).await
    {
        return response;
    }
    match state
        .store
        .create_assignment_with_timing(
            authenticated.tenant_context,
            assignment,
            request.assignment_timing,
        )
        .await
    {
        Ok(assignment) => {
            let public_id = match state
                .store
                .assignment_public_id(
                    authenticated.tenant_context,
                    authenticated.record.subject.user(),
                    assignment.record.id,
                )
                .await
            {
                Ok(Some(public_id)) => public_id,
                Ok(None) | Err(_) => {
                    return error_response(
                        StatusCode::SERVICE_UNAVAILABLE,
                        "assignment navigation reference is unavailable",
                    );
                }
            };
            assignment_response(StatusCode::CREATED, assignment, public_id)
        }
        Err(error) => store_error_response(error),
    }
}

pub(super) async fn get_assignment<S>(
    State(state): State<CourseRouteState<S>>,
    headers: HeaderMap,
    Path(assignment): Path<AssignmentId>,
) -> Response
where
    S: Store + CatalogStore + CourseRecordsAccessStore + SessionStore + 'static,
{
    let authenticated = match resolve_request_session(state.store.as_ref(), &headers).await {
        Ok(authenticated) => authenticated,
        Err(error) => return auth_error_response(error),
    };
    let assignment = match state
        .store
        .get_assignment_for_edit(authenticated.tenant_context, assignment)
        .await
    {
        Ok(Some(assignment)) => assignment,
        Ok(None) => return error_response(StatusCode::NOT_FOUND, "assignment not found"),
        Err(error) => return store_error_response(error),
    };
    if let Err(response) = require_course_access(
        state.store.as_ref(),
        &authenticated,
        assignment.record.course_id,
        false,
    )
    .await
    {
        return response;
    }
    let public_id = match state
        .store
        .assignment_public_id(
            authenticated.tenant_context,
            authenticated.record.subject.user(),
            assignment.record.id,
        )
        .await
    {
        Ok(Some(public_id)) => public_id,
        Ok(None) => return error_response(StatusCode::NOT_FOUND, "assignment not found"),
        Err(error) => return store_error_response(error),
    };
    assignment_response(StatusCode::OK, assignment, public_id)
}

pub(super) async fn update_assignment<S>(
    State(state): State<CourseRouteState<S>>,
    headers: HeaderMap,
    Path((course, assignment)): Path<(CourseId, AssignmentId)>,
    Json(value): Json<serde_json::Value>,
) -> Response
where
    S: Store + CatalogStore + CourseRecordsAccessStore + SessionStore + 'static,
{
    let authenticated = match resolve_request_session(state.store.as_ref(), &headers).await {
        Ok(authenticated) => authenticated,
        Err(error) => return auth_error_response(error),
    };
    if let Err(response) =
        require_course_access(state.store.as_ref(), &authenticated, course, true).await
    {
        return response;
    }
    let expected_revision = match required_assignment_revision(&headers) {
        Ok(revision) => revision,
        Err(AssignmentRevisionHeaderError::Missing) => {
            return error_response(
                StatusCode::PRECONDITION_REQUIRED,
                "If-Match assignment revision is required",
            );
        }
        Err(AssignmentRevisionHeaderError::Malformed) => {
            return error_response(
                StatusCode::UNPROCESSABLE_ENTITY,
                "If-Match assignment revision is invalid",
            );
        }
    };
    let request = match strict_assignment_request::<UpdateAssignmentRequest>(value) {
        Ok(request) => request,
        Err(()) => {
            return error_response(
                StatusCode::UNPROCESSABLE_ENTITY,
                "assignment request is invalid",
            );
        }
    };
    let current = match state
        .store
        .get_assignment_for_edit(authenticated.tenant_context, assignment)
        .await
    {
        Ok(Some(current)) if current.record.course_id == course => current,
        Ok(Some(_)) | Ok(None) => {
            return error_response(StatusCode::NOT_FOUND, "assignment not found");
        }
        Err(error) => return store_error_response(error),
    };
    let items = assignment_items(request.problems, Some(&current.record.items));
    let replacement = AssignmentRecord {
        id: assignment,
        tenant: authenticated.tenant_context.tenant_id(),
        course_id: course,
        title: request.title.clone(),
        items: items.clone(),
        selection_groups: current.record.selection_groups.clone(),
        policies: request.policies,
    };
    if let Err(response) =
        validate_assignment_request(&state, authenticated.tenant_context, &replacement).await
    {
        return response;
    }
    match state
        .store
        .replace_assignment_with_timing(
            authenticated.tenant_context,
            course,
            assignment,
            expected_revision,
            learning_data_access::AssignmentEditorUpdate {
                assignment: learning_data_access::AssignmentUpdate {
                    title: request.title,
                    items,
                    selection_groups: current.record.selection_groups,
                    policies: request.policies,
                },
                assignment_timing: request.assignment_timing,
            },
        )
        .await
    {
        Ok(assignment) => {
            let public_id = match state
                .store
                .assignment_public_id(
                    authenticated.tenant_context,
                    authenticated.record.subject.user(),
                    assignment.record.id,
                )
                .await
            {
                Ok(Some(public_id)) => public_id,
                Ok(None) => return error_response(StatusCode::NOT_FOUND, "assignment not found"),
                Err(error) => return store_error_response(error),
            };
            assignment_response(StatusCode::OK, assignment, public_id)
        }
        Err(StoreError::Conflict) => {
            error_response(StatusCode::CONFLICT, "assignment changed; reload it")
        }
        Err(error) => store_error_response(error),
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum AssignmentRevisionHeaderError {
    Missing,
    Malformed,
}

pub(super) fn required_assignment_revision(
    headers: &HeaderMap,
) -> Result<AssignmentRevision, AssignmentRevisionHeaderError> {
    let mut values = headers.get_all(IF_MATCH).iter();
    let Some(value) = values.next() else {
        return Err(AssignmentRevisionHeaderError::Missing);
    };
    if values.next().is_some() {
        return Err(AssignmentRevisionHeaderError::Malformed);
    }
    let value = value
        .to_str()
        .map_err(|_| AssignmentRevisionHeaderError::Malformed)?;
    let Some(value) = value
        .strip_prefix('"')
        .and_then(|value| value.strip_suffix('"'))
    else {
        return Err(AssignmentRevisionHeaderError::Malformed);
    };
    if value.is_empty() || !value.bytes().all(|byte| byte.is_ascii_digit()) {
        return Err(AssignmentRevisionHeaderError::Malformed);
    }
    let numeric = value
        .parse::<u64>()
        .map_err(|_| AssignmentRevisionHeaderError::Malformed)?;
    if numeric == 0 || numeric > i64::MAX as u64 {
        return Err(AssignmentRevisionHeaderError::Malformed);
    }
    serde_json::from_str(value).map_err(|_| AssignmentRevisionHeaderError::Malformed)
}

fn assignment_response(
    status: StatusCode,
    assignment: StoredAssignment,
    public_id: question_model::AssignmentPublicId,
) -> Response {
    #[derive(Serialize)]
    #[serde(rename_all = "camelCase")]
    struct AssignmentEditorResponse {
        #[serde(flatten)]
        assignment: question_model::AssignmentSummary,
        assignment_timing: question_model::AssignmentRunTiming,
    }
    let mut response = (
        status,
        Json(AssignmentEditorResponse {
            assignment: assignment.record.summary(public_id),
            assignment_timing: assignment.assignment_timing,
        }),
    )
        .into_response();
    let etag = format!("\"{}\"", assignment.revision.value());
    let header = HeaderValue::from_str(&etag).expect("positive revision produces a valid ETag");
    response.headers_mut().insert(ETAG, header);
    no_store(response)
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct AssignmentCapabilityViolation {
    title: String,
    reference: ProblemVersionRef,
    capability: Capability,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct AssignmentValidationFailure {
    error: &'static str,
    violations: Vec<AssignmentCapabilityViolation>,
}

async fn validate_assignment_request<S>(
    state: &CourseRouteState<S>,
    context: learning_data_access::TenantContext,
    assignment: &AssignmentRecord,
) -> Result<(), Response>
where
    S: Store + CatalogStore + SessionStore + 'static,
{
    let references = assignment.references().collect::<Vec<_>>();
    let mut selected = Vec::with_capacity(references.len());
    let mut titles = std::collections::BTreeMap::new();
    for reference in &references {
        let Some(published) = state
            .store
            .get_catalog_problem(context, *reference)
            .await
            .map_err(store_error_response)?
        else {
            return Err(error_response(
                StatusCode::UNPROCESSABLE_ENTITY,
                "assignment references a missing or hidden published version",
            ));
        };
        if !published.lifecycle.is_assignable() {
            return Err(error_response(
                StatusCode::UNPROCESSABLE_ENTITY,
                "assignment references a nonassignable published version",
            ));
        }
        titles.insert(*reference, published.question.metadata.title.clone());
        selected.push(domain::policy::AssignmentQuestionConfig {
            question: published.question,
            backend_capabilities: published.capabilities,
        });
    }
    let violations =
        domain::policy::validate_assignment_config(&domain::policy::AssignmentConfig {
            questions: selected,
            required_capabilities: Vec::new(),
        })
        .into_iter()
        .map(|violation| {
            let reference = assignment
                .references()
                .find(|reference| reference.version == violation.question)
                .expect("domain validation only reports a selected question version");
            let title = titles
                .get(&reference)
                .expect("every selected question has its immutable title")
                .clone();
            AssignmentCapabilityViolation {
                title,
                reference,
                capability: violation.capability,
            }
        })
        .collect::<Vec<_>>();
    if violations.is_empty() {
        Ok(())
    } else {
        Err(no_store(
            (
                StatusCode::UNPROCESSABLE_ENTITY,
                Json(AssignmentValidationFailure {
                    error: "assignment configuration is not supported",
                    violations,
                }),
            )
                .into_response(),
        ))
    }
}

fn assignment_items(
    references: Vec<ProblemVersionRef>,
    existing: Option<&[AssignmentItem]>,
) -> Vec<AssignmentItem> {
    let mut claimed = std::collections::BTreeSet::new();
    references
        .into_iter()
        .enumerate()
        .map(|(position, reference)| {
            let prior = existing.and_then(|items| {
                items.iter().find(|item| {
                    item.reference == reference
                        && item.delivery_state == AssignmentDeliveryState::Active
                        && !claimed.contains(&item.id)
                })
            });
            let id = prior
                .map(|item| item.id)
                .unwrap_or_else(question_model::AssignmentItemId::generate);
            claimed.insert(id);
            AssignmentItem {
                id,
                reference,
                position: u32::try_from(position).expect("bounded request body fits u32"),
                points_possible: prior
                    .map(|item| item.points_possible)
                    .unwrap_or_else(|| PointValue::from_whole(1)),
                delivery_state: AssignmentDeliveryState::Active,
                scoring_mode: prior
                    .map(|item| item.scoring_mode)
                    .unwrap_or(AssignmentScoringMode::Normal),
            }
        })
        .collect()
}

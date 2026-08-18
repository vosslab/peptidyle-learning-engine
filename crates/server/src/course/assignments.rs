use axum::Json;
use axum::body::Bytes;
use axum::extract::{Path, State};
use axum::http::header::{ETAG, IF_MATCH};
use axum::http::{HeaderMap, HeaderValue, StatusCode};
use axum::response::{IntoResponse, Response};
use learning_data_access::{
    AddAssignmentFixedItemCommand, AssignmentRecord, AssignmentRevision, CatalogStore,
    CourseRecordsAccessStore, RemoveAssignmentFixedItemCommand, ReplaceAssignmentFixedItemCommand,
    SessionStore, Store, StoreError, StoredAssignment,
};
use question_model::{
    AssignmentDeliveryState, AssignmentId, AssignmentItem, AssignmentItemId, AssignmentScoringMode,
    Capability, CourseId, PointValue, ProblemVersionRef, QuestionId,
};
use serde::Serialize;

use crate::auth::{auth_error_response, no_store, resolve_request_session};

use super::policy::require_course_access;
use super::projection::{error_response, no_store, store_error_response};
use super::routing::{
    AddAssignmentItemRequest, AssignmentItemUpdateRequest, CourseRouteState,
    CreateAssignmentRequest, ReplaceAssignmentItemQuestionRequest, UpdateAssignmentRequest,
    strict_assignment_request,
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
    let publications = match resolve_assignable_question_ids(
        &state,
        authenticated.tenant_context,
        &request.question_ids,
    )
    .await
    {
        Ok(publications) => publications,
        Err(response) => return response,
    };
    let assignment = AssignmentRecord {
        id: AssignmentId::generate(),
        tenant: authenticated.tenant_context.tenant_id(),
        course_id: course,
        title: request.title,
        items: assignment_items(publications, None),
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
            assignment_response(&state, &authenticated, StatusCode::CREATED, assignment).await
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
    assignment_response(&state, &authenticated, StatusCode::OK, assignment).await
}

pub(super) async fn get_assignment_summary<S>(
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
    let enrollment = match state
        .store
        .learner_get_enrollment_for_assignment(
            authenticated.tenant_context,
            authenticated.record.subject.user(),
            assignment,
        )
        .await
    {
        Ok(Some(enrollment)) => enrollment,
        Ok(None) => return error_response(StatusCode::NOT_FOUND, "assignment summary not found"),
        Err(error) => return store_error_response(error),
    };
    match state
        .store
        .learner_get_summary(
            authenticated.tenant_context,
            authenticated.record.subject.user(),
            enrollment.id,
        )
        .await
    {
        Ok(Some(summary)) => no_store(Json(summary).into_response()),
        Ok(None) => error_response(StatusCode::NOT_FOUND, "summary not found"),
        Err(error) => store_error_response(error),
    }
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
    let items = match preserved_assignment_items(
        &state,
        authenticated.tenant_context,
        &current.record.items,
        request.items,
    )
    .await
    {
        Ok(items) => items,
        Err(response) => return response,
    };
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
            assignment_response(&state, &authenticated, StatusCode::OK, assignment).await
        }
        Err(StoreError::Conflict) => {
            error_response(StatusCode::CONFLICT, "assignment changed; reload it")
        }
        Err(error) => store_error_response(error),
    }
}

pub(super) async fn add_assignment_item<S>(
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
        Ok(value) => value,
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
    let request = match strict_assignment_request::<AddAssignmentItemRequest>(value) {
        Ok(value) => value,
        Err(()) => {
            return error_response(
                StatusCode::UNPROCESSABLE_ENTITY,
                "assignment item request is invalid",
            );
        }
    };
    let references = match resolve_assignable_question_ids(
        &state,
        authenticated.tenant_context,
        &[request.question_id],
    )
    .await
    {
        Ok(value) => value,
        Err(response) => return response,
    };
    let item = AssignmentItem {
        id: AssignmentItemId::generate(),
        reference: references[0],
        position: request.position,
        points_possible: PointValue::from_whole(1),
        delivery_state: AssignmentDeliveryState::Active,
        scoring_mode: AssignmentScoringMode::Normal,
    };
    match state
        .store
        .add_assignment_fixed_item(
            authenticated.tenant_context,
            AddAssignmentFixedItemCommand {
                course,
                assignment,
                expected_revision,
                item,
            },
        )
        .await
    {
        Ok(assignment) => {
            assignment_response(&state, &authenticated, StatusCode::OK, assignment).await
        }
        Err(StoreError::Conflict) => {
            error_response(StatusCode::CONFLICT, "assignment changed; reload it")
        }
        Err(error) => store_error_response(error),
    }
}

pub(super) async fn remove_assignment_item<S>(
    State(state): State<CourseRouteState<S>>,
    headers: HeaderMap,
    Path((course, assignment, item)): Path<(CourseId, AssignmentId, AssignmentItemId)>,
    body: Bytes,
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
    if !body.is_empty() {
        return error_response(
            StatusCode::BAD_REQUEST,
            "assignment item removal does not accept a request body",
        );
    }
    let expected_revision = match required_assignment_revision(&headers) {
        Ok(value) => value,
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
    match state
        .store
        .remove_assignment_fixed_item(
            authenticated.tenant_context,
            RemoveAssignmentFixedItemCommand {
                course,
                assignment,
                item,
                expected_revision,
            },
        )
        .await
    {
        Ok(assignment) => {
            assignment_response(&state, &authenticated, StatusCode::OK, assignment).await
        }
        Err(StoreError::Conflict) => {
            error_response(StatusCode::CONFLICT, "assignment changed; reload it")
        }
        Err(error) => store_error_response(error),
    }
}

pub(super) async fn replace_assignment_item_question<S>(
    State(state): State<CourseRouteState<S>>,
    headers: HeaderMap,
    Path((course, assignment, item)): Path<(CourseId, AssignmentId, AssignmentItemId)>,
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
        Ok(value) => value,
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
    let request = match strict_assignment_request::<ReplaceAssignmentItemQuestionRequest>(value) {
        Ok(value) => value,
        Err(()) => {
            return error_response(
                StatusCode::UNPROCESSABLE_ENTITY,
                "assignment item request is invalid",
            );
        }
    };
    let references = match resolve_assignable_question_ids(
        &state,
        authenticated.tenant_context,
        &[request.question_id],
    )
    .await
    {
        Ok(value) => value,
        Err(response) => return response,
    };
    match state
        .store
        .replace_assignment_fixed_item(
            authenticated.tenant_context,
            ReplaceAssignmentFixedItemCommand {
                course,
                assignment,
                current_item: item,
                expected_revision,
                replacement: references[0],
            },
        )
        .await
    {
        Ok(assignment) => {
            assignment_response(&state, &authenticated, StatusCode::OK, assignment).await
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

async fn assignment_response<S>(
    state: &CourseRouteState<S>,
    authenticated: &crate::auth::AuthenticatedSession,
    status: StatusCode,
    assignment: StoredAssignment,
) -> Response
where
    S: Store + CatalogStore + SessionStore + 'static,
{
    #[derive(Serialize)]
    #[serde(rename_all = "camelCase")]
    struct AssignmentEditorResponse {
        #[serde(flatten)]
        assignment: question_model::AssignmentSummary,
        assignment_timing: question_model::AssignmentRunTiming,
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
    let (items, selection_groups) =
        match assignment_summary_items(state, authenticated.tenant_context, &assignment.record)
            .await
        {
            Ok(value) => value,
            Err(response) => return response,
        };
    let mut response = (
        status,
        Json(AssignmentEditorResponse {
            assignment: assignment
                .record
                .summary(public_id, items, selection_groups),
            assignment_timing: assignment.assignment_timing,
        }),
    )
        .into_response();
    let etag = format!("\"{}\"", assignment.revision.value());
    let header = HeaderValue::from_str(&etag).expect("positive revision produces a valid ETag");
    response.headers_mut().insert(ETAG, header);
    no_store(response)
}

pub(super) async fn assignment_summary_items<S>(
    state: &CourseRouteState<S>,
    context: learning_data_access::TenantContext,
    assignment: &AssignmentRecord,
) -> Result<
    (
        Vec<question_model::AssignmentItemSummary>,
        Vec<question_model::AssignmentSelectionGroupSummary>,
    ),
    Response,
>
where
    S: Store + CatalogStore + SessionStore + 'static,
{
    let mut summaries = std::collections::BTreeMap::new();
    for reference in assignment.references() {
        let Some(record) = state
            .store
            .get_catalog_problem(context, reference)
            .await
            .map_err(store_error_response)?
        else {
            return Err(error_response(
                StatusCode::UNPROCESSABLE_ENTITY,
                "assignment contains an unavailable published question",
            ));
        };
        summaries.insert(reference, record);
    }
    let fixed = assignment
        .items
        .iter()
        .map(|item| {
            let record = summaries
                .get(&item.reference)
                .expect("every fixed item was resolved above");
            question_model::AssignmentItemSummary {
                id: item.id,
                question_id: record.question_id.clone(),
                title: record.question.metadata.title.clone(),
                backend: question_model::QuestionBackend::from(&record.question.source),
                capabilities: record.capabilities.clone(),
                position: item.position,
                points_possible: item.points_possible,
                delivery_state: item.delivery_state,
                scoring_mode: item.scoring_mode,
            }
        })
        .collect();
    let groups = assignment
        .selection_groups
        .iter()
        .map(|group| question_model::AssignmentSelectionGroupSummary {
            id: group.id,
            position: group.position,
            draw_count: group.draw_count,
            points_per_item: group.points_per_item,
            ordering: group.ordering,
            algorithm_version: group.algorithm_version,
            candidates: group
                .candidates
                .iter()
                .map(|candidate| {
                    let record = summaries
                        .get(&candidate.reference)
                        .expect("every candidate was resolved above");
                    question_model::AssignmentSelectionCandidateSummary {
                        id: candidate.id,
                        question_id: record.question_id.clone(),
                        title: record.question.metadata.title.clone(),
                        backend: question_model::QuestionBackend::from(&record.question.source),
                        capabilities: record.capabilities.clone(),
                        position: candidate.position,
                        delivery_state: candidate.delivery_state,
                    }
                })
                .collect(),
        })
        .collect();
    Ok((fixed, groups))
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct AssignmentCapabilityViolation {
    title: String,
    question_id: QuestionId,
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
    let mut display = std::collections::BTreeMap::new();
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
        display.insert(
            *reference,
            (
                published.question.metadata.title.clone(),
                published.question_id.clone(),
            ),
        );
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
            let (title, question_id) = display
                .get(&reference)
                .expect("every selected question has its immutable title")
                .clone();
            AssignmentCapabilityViolation {
                title,
                question_id,
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

async fn resolve_assignable_question_ids<S>(
    state: &CourseRouteState<S>,
    context: learning_data_access::TenantContext,
    question_ids: &[QuestionId],
) -> Result<Vec<ProblemVersionRef>, Response>
where
    S: Store + CatalogStore + SessionStore + 'static,
{
    let mut seen = std::collections::BTreeSet::new();
    let mut references = Vec::with_capacity(question_ids.len());
    for question_id in question_ids {
        if !seen.insert(question_id.clone()) {
            return Err(error_response(
                StatusCode::UNPROCESSABLE_ENTITY,
                "assignment question IDs must be unique",
            ));
        }
        let Some(record) = state
            .store
            .resolve_catalog_problem(
                context,
                question_model::ProblemDisplayRef {
                    question_id: question_id.clone(),
                },
            )
            .await
            .map_err(store_error_response)?
        else {
            return Err(error_response(
                StatusCode::UNPROCESSABLE_ENTITY,
                "assignment question ID is unavailable",
            ));
        };
        if !record.lifecycle.is_assignable() {
            return Err(error_response(
                StatusCode::UNPROCESSABLE_ENTITY,
                "assignment question is not assignable",
            ));
        }
        references.push(ProblemVersionRef {
            problem: record.problem,
            version: record.version,
        });
    }
    Ok(references)
}

async fn preserved_assignment_items<S>(
    state: &CourseRouteState<S>,
    context: learning_data_access::TenantContext,
    current: &[AssignmentItem],
    requests: Vec<AssignmentItemUpdateRequest>,
) -> Result<Vec<AssignmentItem>, Response>
where
    S: Store + CatalogStore + SessionStore + 'static,
{
    if requests.len() != current.len() {
        return Err(error_response(
            StatusCode::UNPROCESSABLE_ENTITY,
            "ordinary assignment save preserves every fixed item",
        ));
    }
    let mut current_by_id = current
        .iter()
        .map(|item| (item.id, item))
        .collect::<std::collections::BTreeMap<_, _>>();
    let mut items = Vec::with_capacity(requests.len());
    for request in requests {
        let Some(prior) = current_by_id.remove(&request.id) else {
            return Err(error_response(
                StatusCode::UNPROCESSABLE_ENTITY,
                "ordinary assignment save uses existing item IDs",
            ));
        };
        let references =
            resolve_assignable_question_ids(state, context, &[request.question_id]).await?;
        if references[0] != prior.reference {
            return Err(error_response(
                StatusCode::UNPROCESSABLE_ENTITY,
                "ordinary assignment save preserves each item question",
            ));
        }
        items.push(AssignmentItem {
            id: prior.id,
            reference: prior.reference,
            position: request.position,
            points_possible: request.points_possible,
            delivery_state: request.delivery_state,
            scoring_mode: request.scoring_mode,
        });
    }
    Ok(items)
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

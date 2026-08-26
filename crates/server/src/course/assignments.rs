use axum::Json;
use axum::body::{Bytes, to_bytes};
use axum::extract::{Path, Request, State};
use axum::http::header::{CONTENT_TYPE, ETAG, IF_MATCH};
use axum::http::{HeaderMap, HeaderValue, StatusCode};
use axum::response::{IntoResponse, Response};
use learning_data_access::{
    AddAssignmentFixedItemCommand, AssignmentRecord, AssignmentRevision, AuthoritativeTimeStore,
    CatalogStore, CourseRecordsAccessStore, CreateAssignmentCommand,
    RemoveAssignmentFixedItemCommand, ReplaceAssignmentCommand, ReplaceAssignmentFixedItemCommand,
    ReplaceUnissuedAssignmentDefinitionCommand, ReplaceUnissuedAssignmentDefinitionOutcome,
    SessionStore, Store, StoreError, StoredAssignment,
};
use question_model::{
    AssignmentDeliveryState, AssignmentId, AssignmentInstructions, AssignmentItem,
    AssignmentItemId, AssignmentLifecycle, AssignmentScoringMode, AssignmentSelectionCandidate,
    AssignmentSelectionGroup, AssignmentTeachingSettings, BaseAssignmentPolicy, CourseId,
    PointValue,
};
use serde::Serialize;

use super::policy::require_course_access;
use super::projection::{error_response, store_error_response};
use super::routing::{
    AddAssignmentItemRequest, AssignmentTeachingSettingsRequest, CourseRouteState,
    CreateAssignmentRequest, ReplaceAssignmentItemQuestionRequest, UpdateAssignmentRequest,
    strict_assignment_request,
};
use crate::auth::{auth_error_response, no_store, resolve_request_session};
use crate::http_refusal::HttpResult;

mod definition_request;
mod learner;
mod teaching_settings;

pub(super) use teaching_settings::put_teaching_settings;

pub(super) use learner::{get_assignment_summary, get_learner_assignment};

pub(super) async fn create_assignment<S>(
    State(state): State<CourseRouteState<S>>,
    Path(course): Path<CourseId>,
    request: Request,
) -> Response
where
    S: Store
        + AuthoritativeTimeStore
        + CatalogStore
        + CourseRecordsAccessStore
        + SessionStore
        + 'static,
{
    // ASVS 8.2.1 and 8.3.1: establish session-derived course authority before
    // consuming untrusted authoring JSON.
    let authenticated = match resolve_request_session(state.store.as_ref(), request.headers()).await
    {
        Ok(authenticated) => authenticated,
        Err(error) => return auth_error_response(error),
    };
    if let Err(response) =
        require_course_access(state.store.as_ref(), &authenticated, course, true).await
    {
        return response.into_response();
    }
    let value = match definition_request::assignment_json_body(request).await {
        Ok(value) => value,
        Err(response) => return response.into_response(),
    };
    let request = match strict_assignment_request::<CreateAssignmentRequest>(value) {
        Ok(request) => request,
        Err(()) => {
            return error_response(
                StatusCode::UNPROCESSABLE_ENTITY,
                "Use the assignment editor to send a complete valid assignment definition.",
            );
        }
    };
    let (items, selection_groups) = match definition_request::resolve_assignment_entries(
        &state,
        authenticated.tenant_context,
        request.entries,
        None,
    )
    .await
    {
        Ok(publications) => publications,
        Err(response) => return response.into_response(),
    };
    let assignment = AssignmentRecord {
        id: AssignmentId::generate(),
        tenant: authenticated.tenant_context.tenant_id(),
        course_id: course,
        title: request.title,
        lifecycle: AssignmentLifecycle::Draft,
        instructions: AssignmentInstructions::default(),
        audience: question_model::AssignmentAudience::CourseWide,
        items,
        selection_groups,
        disclosure_policy: request.disclosure_policy,
        policies: request.policies,
    };
    if let Err(response) = definition_request::validate_assignment_request(
        &state,
        authenticated.tenant_context,
        &assignment,
    )
    .await
    {
        return response.into_response();
    }
    match state
        .store
        .create_assignment(
            authenticated.tenant_context,
            CreateAssignmentCommand {
                actor: authenticated.record.subject.user(),
                assignment,
                base_policy: BaseAssignmentPolicy::default(),
            },
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
    S: Store
        + AuthoritativeTimeStore
        + CatalogStore
        + CourseRecordsAccessStore
        + SessionStore
        + 'static,
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
        true,
    )
    .await
    {
        return response.into_response();
    }
    assignment_response(&state, &authenticated, StatusCode::OK, assignment).await
}

pub(super) async fn update_assignment<S>(
    State(state): State<CourseRouteState<S>>,
    Path((course, assignment)): Path<(CourseId, AssignmentId)>,
    request: Request,
) -> Response
where
    S: Store
        + AuthoritativeTimeStore
        + CatalogStore
        + CourseRecordsAccessStore
        + SessionStore
        + 'static,
{
    // ASVS 8.2.1 and 8.3.1: establish session-derived course authority before
    // consuming untrusted authoring JSON.
    let authenticated = match resolve_request_session(state.store.as_ref(), request.headers()).await
    {
        Ok(authenticated) => authenticated,
        Err(error) => return auth_error_response(error),
    };
    if let Err(response) =
        require_course_access(state.store.as_ref(), &authenticated, course, true).await
    {
        return response.into_response();
    }
    let expected_revision = match required_assignment_revision(request.headers()) {
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
    let value = match definition_request::assignment_json_body(request).await {
        Ok(value) => value,
        Err(response) => return response.into_response(),
    };
    let request = match strict_assignment_request::<UpdateAssignmentRequest>(value) {
        Ok(request) => request,
        Err(()) => {
            return error_response(
                StatusCode::UNPROCESSABLE_ENTITY,
                "Use the assignment editor to send a complete valid assignment definition.",
            );
        }
    };
    let (items, selection_groups) = match definition_request::resolve_assignment_entries(
        &state,
        authenticated.tenant_context,
        request.entries,
        Some(&current.record),
    )
    .await
    {
        Ok(items) => items,
        Err(response) => return response.into_response(),
    };
    let replacement = AssignmentRecord {
        id: assignment,
        tenant: authenticated.tenant_context.tenant_id(),
        course_id: course,
        title: request.title.clone(),
        lifecycle: current.record.lifecycle,
        instructions: current.record.instructions.clone(),
        audience: current.record.audience.clone(),
        items: items.clone(),
        selection_groups: selection_groups.clone(),
        disclosure_policy: request.disclosure_policy,
        policies: request.policies,
    };
    if let Err(response) = definition_request::validate_assignment_request(
        &state,
        authenticated.tenant_context,
        &replacement,
    )
    .await
    {
        return response.into_response();
    }
    let structure_changed =
        !assignment_definition_structure_unchanged(&current.record, &replacement);
    let result: Result<StoredAssignment, StoreError> = if structure_changed {
        match state
            .store
            .replace_unissued_assignment_definition(
                authenticated.tenant_context,
                ReplaceUnissuedAssignmentDefinitionCommand {
                    actor: authenticated.record.subject.user(),
                    course,
                    assignment,
                    expected_revision,
                    definition: replacement,
                    base_policy: current.base_policy,
                },
            )
            .await
        {
            Ok(ReplaceUnissuedAssignmentDefinitionOutcome::Replaced(assignment)) => Ok(*assignment),
            Ok(ReplaceUnissuedAssignmentDefinitionOutcome::Issued) => {
                return error_response(
                    StatusCode::CONFLICT,
                    "This assignment already has learner work. Create a new assignment for this structural pool change.",
                );
            }
            Err(error) => Err(error),
        }
    } else {
        state
            .store
            .replace_assignment(
                authenticated.tenant_context,
                ReplaceAssignmentCommand {
                    actor: authenticated.record.subject.user(),
                    course,
                    assignment,
                    expected_revision,
                    update: learning_data_access::AssignmentUpdate {
                        title: request.title,
                        audience: current.record.audience,
                        items,
                        selection_groups,
                        disclosure_policy: request.disclosure_policy,
                        policies: request.policies,
                    },
                },
            )
            .await
    };
    match result {
        Ok(assignment) => {
            assignment_response(&state, &authenticated, StatusCode::OK, assignment).await
        }
        Err(StoreError::Conflict) => {
            error_response(StatusCode::CONFLICT, "assignment changed; reload it")
        }
        Err(error) => store_error_response(error),
    }
}

/// Separates the immutable future-run definition from ordinary assignment
/// teaching and scoring settings. A pool's presence is not structural by
/// itself: after learner work exists, an instructor can still change title,
/// disclosure, policies, points, and fixed-item delivery/scoring through the
/// established revisioned replacement path. Only a change that could alter
/// which positions or immutable publications a future run receives uses the
/// pre-issue structural capability.
fn assignment_definition_structure_unchanged(
    current: &AssignmentRecord,
    replacement: &AssignmentRecord,
) -> bool {
    let fixed_unchanged = |before: &AssignmentItem, after: &AssignmentItem| {
        before.id == after.id
            && before.reference == after.reference
            && before.position == after.position
    };
    let candidate_unchanged = |before: &AssignmentSelectionCandidate,
                               after: &AssignmentSelectionCandidate| {
        before.id == after.id
            && before.reference == after.reference
            && before.position == after.position
            && before.delivery_state == after.delivery_state
    };
    let group_unchanged = |before: &AssignmentSelectionGroup, after: &AssignmentSelectionGroup| {
        before.id == after.id
            && before.position == after.position
            && before.draw_count == after.draw_count
            && before.ordering == after.ordering
            && before.algorithm == after.algorithm
            && before.candidates.len() == after.candidates.len()
            && before.candidates.iter().all(|candidate| {
                after
                    .candidates
                    .iter()
                    .find(|other| other.id == candidate.id)
                    .is_some_and(|other| candidate_unchanged(candidate, other))
            })
    };
    current.items.len() == replacement.items.len()
        && current.items.iter().all(|item| {
            replacement
                .items
                .iter()
                .find(|other| other.id == item.id)
                .is_some_and(|other| fixed_unchanged(item, other))
        })
        && current.selection_groups.len() == replacement.selection_groups.len()
        && current.selection_groups.iter().all(|group| {
            replacement
                .selection_groups
                .iter()
                .find(|other| other.id == group.id)
                .is_some_and(|other| group_unchanged(group, other))
        })
}

pub(super) async fn add_assignment_item<S>(
    State(state): State<CourseRouteState<S>>,
    headers: HeaderMap,
    Path((course, assignment)): Path<(CourseId, AssignmentId)>,
    Json(value): Json<serde_json::Value>,
) -> Response
where
    S: Store
        + AuthoritativeTimeStore
        + CatalogStore
        + CourseRecordsAccessStore
        + SessionStore
        + 'static,
{
    let authenticated = match resolve_request_session(state.store.as_ref(), &headers).await {
        Ok(authenticated) => authenticated,
        Err(error) => return auth_error_response(error),
    };
    if let Err(response) =
        require_course_access(state.store.as_ref(), &authenticated, course, true).await
    {
        return response.into_response();
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
    let references = match definition_request::resolve_assignable_question_ids(
        &state,
        authenticated.tenant_context,
        &[request.question_id],
    )
    .await
    {
        Ok(value) => value,
        Err(response) => return response.into_response(),
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
                actor: authenticated.record.subject.user(),
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
    S: Store
        + AuthoritativeTimeStore
        + CatalogStore
        + CourseRecordsAccessStore
        + SessionStore
        + 'static,
{
    let authenticated = match resolve_request_session(state.store.as_ref(), &headers).await {
        Ok(authenticated) => authenticated,
        Err(error) => return auth_error_response(error),
    };
    if let Err(response) =
        require_course_access(state.store.as_ref(), &authenticated, course, true).await
    {
        return response.into_response();
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
                actor: authenticated.record.subject.user(),
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
    S: Store
        + AuthoritativeTimeStore
        + CatalogStore
        + CourseRecordsAccessStore
        + SessionStore
        + 'static,
{
    let authenticated = match resolve_request_session(state.store.as_ref(), &headers).await {
        Ok(authenticated) => authenticated,
        Err(error) => return auth_error_response(error),
    };
    if let Err(response) =
        require_course_access(state.store.as_ref(), &authenticated, course, true).await
    {
        return response.into_response();
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
    let references = match definition_request::resolve_assignable_question_ids(
        &state,
        authenticated.tenant_context,
        &[request.question_id],
    )
    .await
    {
        Ok(value) => value,
        Err(response) => return response.into_response(),
    };
    match state
        .store
        .replace_assignment_fixed_item(
            authenticated.tenant_context,
            ReplaceAssignmentFixedItemCommand {
                actor: authenticated.record.subject.user(),
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
    value
        .parse()
        .map_err(|_| AssignmentRevisionHeaderError::Malformed)
}

async fn assignment_response<S>(
    state: &CourseRouteState<S>,
    authenticated: &crate::auth::AuthenticatedSession,
    status: StatusCode,
    assignment: StoredAssignment,
) -> Response
where
    S: Store + AuthoritativeTimeStore + CatalogStore + SessionStore + 'static,
{
    #[derive(Serialize)]
    #[serde(rename_all = "camelCase")]
    struct AssignmentEditorResponse {
        #[serde(flatten)]
        assignment: question_model::AssignmentSummary,
        teaching_settings: question_model::assignment::InstructorAssignmentTeachingSettingsLocal,
        current_state: question_model::InstructorAssignmentCurrentState,
    }
    let public_id = match state
        .store
        .assignment_reference(
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
    let course = match state
        .store
        .get_course(authenticated.tenant_context, assignment.record.course_id)
        .await
    {
        Ok(Some(course)) => course,
        Ok(None) => return error_response(StatusCode::NOT_FOUND, "course not found"),
        Err(error) => return store_error_response(error),
    };
    let (items, selection_groups) =
        match assignment_summary_items(state, authenticated.tenant_context, &assignment.record)
            .await
        {
            Ok(value) => value,
            Err(response) => return response.into_response(),
        };
    let settings = AssignmentTeachingSettings {
        lifecycle: assignment.record.lifecycle,
        instructions: assignment.record.instructions.clone(),
        base_policy: assignment.base_policy,
    };
    let now = match state
        .store
        .authoritative_time(authenticated.tenant_context)
        .await
    {
        Ok(now) => now,
        Err(error) => return store_error_response(error),
    };
    let mut response = (
        status,
        Json(AssignmentEditorResponse {
            assignment: assignment
                .record
                .summary(public_id, items, selection_groups),
        teaching_settings: match question_model::assignment::InstructorAssignmentTeachingSettingsLocal::from_absolute(&course.term, &settings) {
            Ok(settings) => settings,
            Err(_) => return error_response(StatusCode::SERVICE_UNAVAILABLE, "assignment teaching settings are invalid"),
        },
        current_state: match question_model::derive_instructor_assignment_current_state(&course.term, &settings, now) {
            Ok(value) => value,
            Err(_) => return error_response(StatusCode::SERVICE_UNAVAILABLE, "assignment teaching settings are invalid"),
        },
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
) -> HttpResult<(
    Vec<question_model::AssignmentItemSummary>,
    Vec<question_model::AssignmentSelectionGroupSummary>,
)>
where
    S: Store + CatalogStore + SessionStore + 'static,
{
    let mut summaries = std::collections::BTreeMap::new();
    for reference in assignment.references() {
        let Some(record) = state
            .store
            .get_catalog_problem(context, reference)
            .await
            .map_err(|error| crate::http_refusal::HttpRefusal::from(store_error_response(error)))?
        else {
            return Err(error_response(
                StatusCode::UNPROCESSABLE_ENTITY,
                "assignment contains an unavailable published question",
            )
            .into());
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
            algorithm_version: group.algorithm.storage_version(),
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

#[cfg(test)]
mod structural_update_tests {
    use super::*;
    use question_model::{AssignmentSelectionGroupId, PoolDrawAlgorithm, ProblemVersionRef};

    fn id(value: u128) -> uuid::Uuid {
        uuid::Uuid::from_u128(value)
    }

    fn assignment() -> AssignmentRecord {
        AssignmentRecord {
            id: AssignmentId::from_uuid(id(1)),
            tenant: question_model::TenantId::from_uuid(id(2)),
            course_id: CourseId::from_uuid(id(3)),
            title: "Original title".to_string(),
            lifecycle: AssignmentLifecycle::Draft,
            instructions: AssignmentInstructions::default(),
            audience: question_model::AssignmentAudience::CourseWide,
            items: vec![AssignmentItem {
                id: AssignmentItemId::from_uuid(id(4)),
                reference: ProblemVersionRef {
                    problem: question_model::ProblemId::from_uuid(id(5)),
                    version: question_model::VersionId::from_uuid(id(6)),
                },
                position: 0,
                points_possible: PointValue::from_whole(1),
                delivery_state: AssignmentDeliveryState::Active,
                scoring_mode: AssignmentScoringMode::Normal,
            }],
            selection_groups: vec![AssignmentSelectionGroup {
                id: AssignmentSelectionGroupId::from_uuid(id(7)),
                position: 1,
                draw_count: 1,
                points_per_item: PointValue::from_whole(1),
                ordering: question_model::SelectionOrdering::CandidateOrder,
                algorithm: PoolDrawAlgorithm::V1,
                candidates: vec![AssignmentSelectionCandidate {
                    id: AssignmentItemId::from_uuid(id(8)),
                    position: 0,
                    reference: ProblemVersionRef {
                        problem: question_model::ProblemId::from_uuid(id(9)),
                        version: question_model::VersionId::from_uuid(id(10)),
                    },
                    delivery_state: AssignmentDeliveryState::Active,
                }],
            }],
            disclosure_policy: question_model::LearnerDisclosurePolicy::default(),
            policies: question_model::RunPolicies {
                completion: question_model::CompletionRequirement::AllCorrect,
                grade: question_model::GradePolicy::Highest,
                continued_practice: question_model::ContinuedPractice::Unlimited,
                variation: question_model::VariationPolicy::NewSeeds,
            },
        }
    }

    #[test]
    fn pool_points_and_teaching_settings_are_ordinary_updates() {
        let current = assignment();
        let mut replacement = current.clone();
        replacement.title = "Revised title".to_string();
        replacement.items[0].points_possible = PointValue::from_whole(3);
        replacement.selection_groups[0].points_per_item = PointValue::from_whole(2);
        replacement.disclosure_policy.score = question_model::LearnerDisclosureTiming::AfterDue;
        replacement.policies.grade = question_model::GradePolicy::First;

        assert!(assignment_definition_structure_unchanged(
            &current,
            &replacement
        ));
    }

    #[test]
    fn pool_draw_membership_and_order_changes_are_structural() {
        let current = assignment();
        let mut draw = current.clone();
        draw.selection_groups[0].draw_count = 2;
        assert!(!assignment_definition_structure_unchanged(&current, &draw));

        let mut membership = current.clone();
        membership.selection_groups[0].candidates[0].delivery_state =
            AssignmentDeliveryState::Retired;
        assert!(!assignment_definition_structure_unchanged(
            &current,
            &membership
        ));

        let mut reorder = current.clone();
        reorder.items[0].position = 1;
        reorder.selection_groups[0].position = 0;
        assert!(!assignment_definition_structure_unchanged(
            &current, &reorder
        ));
    }
}

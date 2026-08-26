//! Focused Instructor assignment-workspace HTTP operations.

use axum::extract::{Path, Request, State};
use axum::http::header::LOCATION;
use axum::http::{HeaderMap, HeaderValue, StatusCode};
use axum::response::{IntoResponse, Response};
use learning_data_access::{
    AssignmentContentUpdate, AssignmentPoliciesUpdate, AuthoritativeTimeStore, CatalogStore,
    CourseGroupManagementStore, CourseRecordsAccessStore, CreateAssignmentDraftCommand,
    ReplaceAssignmentContentCommand, ReplaceAssignmentContentOutcome,
    ReplaceAssignmentPoliciesCommand, ReplaceAssignmentPoliciesOutcome, SessionStore, Store,
    StoreError,
};
use question_model::{
    AssignmentAudience, AssignmentAudienceRequest, AssignmentId, CourseId,
    CreateAssignmentDraftRequest, InstructorStudentView, ReplaceAssignmentContentRequest,
    ReplaceAssignmentPoliciesRequest,
};

use super::super::policy::require_course_access;
use super::super::projection::{error_response, store_error_response};
use super::super::routing::{CourseRouteState, strict_assignment_request};
use super::{
    AssignmentRevisionHeaderError, assignment_response, definition_request,
    instructor_student_view_delivery, required_assignment_revision,
};
use crate::auth::{auth_error_response, no_store, resolve_request_session};
use crate::http_refusal::HttpResult;

/// Persists a deliberately incomplete draft after the authenticated course
/// authority has been established.  The browser names only the title.
pub(in crate::course) async fn create_assignment_draft<S>(
    State(state): State<CourseRouteState<S>>,
    Path(course): Path<CourseId>,
    request: Request,
) -> Response
where
    S: Store
        + AuthoritativeTimeStore
        + CatalogStore
        + CourseGroupManagementStore
        + CourseRecordsAccessStore
        + SessionStore
        + 'static,
{
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
    let request = match strict_assignment_request::<CreateAssignmentDraftRequest>(value) {
        Ok(request) if !request.title.trim().is_empty() => request,
        _ => {
            return error_response(
                StatusCode::UNPROCESSABLE_ENTITY,
                "Enter an assignment title.",
            );
        }
    };
    let assignment_id = AssignmentId::generate();
    match state
        .store
        .create_assignment_draft(
            authenticated.tenant_context,
            CreateAssignmentDraftCommand {
                actor: authenticated.record.subject.user(),
                course,
                assignment: assignment_id,
                title: request.title,
            },
        )
        .await
    {
        Ok(assignment) => {
            let mut response =
                assignment_response(&state, &authenticated, StatusCode::CREATED, assignment).await;
            let location = format!("/api/courses/{course}/assignments/{assignment_id}");
            let value =
                HeaderValue::from_str(&location).expect("UUID path is a valid Location header");
            response.headers_mut().insert(LOCATION, value);
            response
        }
        Err(error) => store_error_response(error),
    }
}

/// Reads the complete Instructor workspace representation under an exact
/// nested course/assignment route.
pub(in crate::course) async fn get_assignment_workspace<S>(
    State(state): State<CourseRouteState<S>>,
    headers: HeaderMap,
    Path((course, assignment)): Path<(CourseId, AssignmentId)>,
) -> Response
where
    S: Store
        + AuthoritativeTimeStore
        + CatalogStore
        + CourseGroupManagementStore
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
    let stored =
        match exact_assignment(&state, authenticated.tenant_context, course, assignment).await {
            Ok(stored) => stored,
            Err(response) => return response.into_response(),
        };
    assignment_response(&state, &authenticated, StatusCode::OK, stored).await
}

/// Replaces exactly the Questions-owned content slice.
pub(in crate::course) async fn replace_assignment_content<S>(
    State(state): State<CourseRouteState<S>>,
    Path((course, assignment)): Path<(CourseId, AssignmentId)>,
    request: Request,
) -> Response
where
    S: Store
        + AuthoritativeTimeStore
        + CatalogStore
        + CourseGroupManagementStore
        + CourseRecordsAccessStore
        + SessionStore
        + 'static,
{
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
    let expected_revision = match revision_or_response(request.headers()) {
        Ok(revision) => revision,
        Err(response) => return response,
    };
    let current =
        match exact_assignment(&state, authenticated.tenant_context, course, assignment).await {
            Ok(stored) => stored,
            Err(response) => return response.into_response(),
        };
    if current.revision != expected_revision {
        return error_response(
            StatusCode::PRECONDITION_FAILED,
            "assignment changed; reload it",
        );
    }
    let value = match definition_request::assignment_json_body(request).await {
        Ok(value) => value,
        Err(response) => return response.into_response(),
    };
    let request = match strict_assignment_request::<ReplaceAssignmentContentRequest>(value) {
        Ok(request) if !request.title.trim().is_empty() => request,
        _ => {
            return error_response(
                StatusCode::UNPROCESSABLE_ENTITY,
                "Use the Questions workspace to send a valid title and ordered content.",
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
        Ok(value) => value,
        Err(response) => return response.into_response(),
    };
    let candidate = current.record.with_content_update(AssignmentContentUpdate {
        title: request.title.clone(),
        items,
        selection_groups,
    });
    if let Err(response) = definition_request::validate_assignment_request(
        &state,
        authenticated.tenant_context,
        &candidate,
    )
    .await
    {
        return response.into_response();
    }
    match state
        .store
        .replace_assignment_content(
            authenticated.tenant_context,
            ReplaceAssignmentContentCommand {
                actor: authenticated.record.subject.user(),
                course,
                assignment,
                expected_revision,
                update: AssignmentContentUpdate {
                    title: request.title,
                    items: candidate.items,
                    selection_groups: candidate.selection_groups,
                },
            },
        )
        .await
    {
        Ok(ReplaceAssignmentContentOutcome::Replaced(stored)) => {
            assignment_response(&state, &authenticated, StatusCode::OK, *stored).await
        }
        Ok(ReplaceAssignmentContentOutcome::RevisionConflict) => error_response(
            StatusCode::PRECONDITION_FAILED,
            "assignment changed; reload it",
        ),
        Ok(ReplaceAssignmentContentOutcome::Issued) => error_response(
            StatusCode::CONFLICT,
            "This assignment already has learner work. Create a new assignment for this structural change.",
        ),
        Err(StoreError::Conflict) => error_response(
            StatusCode::CONFLICT,
            "assignment content could not be changed in its current state",
        ),
        Err(error) => store_error_response(error),
    }
}

/// Replaces exactly the Policies-owned slice, resolving group locators and
/// course-local times before it reaches the Store aggregate boundary.
pub(in crate::course) async fn replace_assignment_policies<S>(
    State(state): State<CourseRouteState<S>>,
    Path((course, assignment)): Path<(CourseId, AssignmentId)>,
    request: Request,
) -> Response
where
    S: Store
        + AuthoritativeTimeStore
        + CatalogStore
        + CourseGroupManagementStore
        + CourseRecordsAccessStore
        + SessionStore
        + 'static,
{
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
    let expected_revision = match revision_or_response(request.headers()) {
        Ok(revision) => revision,
        Err(response) => return response,
    };
    let current =
        match exact_assignment(&state, authenticated.tenant_context, course, assignment).await {
            Ok(stored) => stored,
            Err(response) => return response.into_response(),
        };
    if current.revision != expected_revision {
        return error_response(
            StatusCode::PRECONDITION_FAILED,
            "assignment changed; reload it",
        );
    }
    let value = match definition_request::assignment_json_body(request).await {
        Ok(value) => value,
        Err(response) => return response.into_response(),
    };
    let request = match strict_assignment_request::<ReplaceAssignmentPoliciesRequest>(value) {
        Ok(request) => request,
        Err(()) => {
            return error_response(
                StatusCode::UNPROCESSABLE_ENTITY,
                "Use the Policies workspace to send complete valid settings.",
            );
        }
    };
    let course_record = match state
        .store
        .get_course(authenticated.tenant_context, course)
        .await
    {
        Ok(Some(course_record)) => course_record,
        Ok(None) => return error_response(StatusCode::NOT_FOUND, "course not found"),
        Err(error) => return store_error_response(error),
    };
    let teaching_settings = match request.teaching_settings.into_absolute(&course_record.term) {
        Ok(settings) => settings,
        Err(error) => {
            return super::teaching_settings::teaching_settings_validation_response(
                error.field(),
                error.reason(),
            );
        }
    };
    if !domain::effective_assignment_policy::is_legal_assignment_lifecycle_transition(
        current.record.lifecycle,
        teaching_settings.lifecycle,
    ) {
        return super::teaching_settings::teaching_settings_validation_response(
            question_model::AssignmentTeachingSettingsField::Lifecycle,
            question_model::AssignmentTeachingSettingsFailureReason::IllegalLifecycleTransition,
        );
    }
    let audience = match resolve_audience(&state, &authenticated, course, request.audience).await {
        Ok(audience) => audience,
        Err(response) => return response.into_response(),
    };
    let candidate = current
        .record
        .with_policies_update(AssignmentPoliciesUpdate {
            audience: audience.clone(),
            disclosure_policy: request.disclosure_policy,
            policies: request.policies,
            teaching_settings: teaching_settings.clone(),
        });
    if let Err(response) = definition_request::validate_assignment_request(
        &state,
        authenticated.tenant_context,
        &candidate,
    )
    .await
    {
        return response.into_response();
    }
    match state
        .store
        .replace_assignment_policies(
            authenticated.tenant_context,
            ReplaceAssignmentPoliciesCommand {
                actor: authenticated.record.subject.user(),
                course,
                assignment,
                expected_revision,
                update: AssignmentPoliciesUpdate {
                    audience,
                    disclosure_policy: request.disclosure_policy,
                    policies: request.policies,
                    teaching_settings,
                },
            },
        )
        .await
    {
        Ok(ReplaceAssignmentPoliciesOutcome::Replaced(stored)) => {
            assignment_response(&state, &authenticated, StatusCode::OK, *stored).await
        }
        Ok(ReplaceAssignmentPoliciesOutcome::RevisionConflict) => error_response(
            StatusCode::PRECONDITION_FAILED,
            "assignment changed; reload it",
        ),
        Err(StoreError::Conflict) => error_response(
            StatusCode::CONFLICT,
            "assignment policies could not be changed in their current state",
        ),
        Err(error) => store_error_response(error),
    }
}

/// Reads an answer-free stable-identity Student view without evaluating a
/// Student entitlement or creating an enrollment, run, attempt, or receipt.
pub(in crate::course) async fn get_instructor_student_view<S>(
    State(state): State<CourseRouteState<S>>,
    headers: HeaderMap,
    Path((course, assignment)): Path<(CourseId, AssignmentId)>,
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
    let stored =
        match exact_assignment(&state, authenticated.tenant_context, course, assignment).await {
            Ok(stored) => stored,
            Err(response) => return response.into_response(),
        };
    let course_record = match state
        .store
        .get_course(authenticated.tenant_context, course)
        .await
    {
        Ok(Some(course_record)) => course_record,
        Ok(None) => return error_response(StatusCode::NOT_FOUND, "course not found"),
        Err(error) => return store_error_response(error),
    };
    let delivery = instructor_student_view_delivery(stored.base_policy);
    let questions_per_run = questions_per_run(&stored.record);
    let variation = stored.record.policies.variation;
    let disclosure_policy = stored.record.disclosure_policy;
    no_store(
        axum::Json(InstructorStudentView {
            title: stored.record.title,
            instructions: stored.record.instructions,
            time_zone: course_record.term.time_zone().clone(),
            delivery,
            questions_per_run,
            variation,
            disclosure_policy,
        })
        .into_response(),
    )
}

fn questions_per_run(record: &learning_data_access::AssignmentRecord) -> u32 {
    let fixed = record
        .items
        .iter()
        .filter(|item| item.delivery_state == question_model::AssignmentDeliveryState::Active)
        .count();
    let selected = record
        .selection_groups
        .iter()
        .map(|group| usize::try_from(group.draw_count).expect("draw count fits usize"))
        .sum::<usize>();
    u32::try_from(fixed + selected).expect("validated assignment count fits u32")
}

fn revision_or_response(
    headers: &HeaderMap,
) -> Result<question_model::AssignmentRevision, Response> {
    required_assignment_revision(headers).map_err(|error| match error {
        AssignmentRevisionHeaderError::Missing => error_response(
            StatusCode::PRECONDITION_REQUIRED,
            "If-Match assignment revision is required",
        ),
        AssignmentRevisionHeaderError::Malformed => error_response(
            StatusCode::UNPROCESSABLE_ENTITY,
            "If-Match assignment revision is invalid",
        ),
    })
}

async fn exact_assignment<S>(
    state: &CourseRouteState<S>,
    context: learning_data_access::TenantContext,
    course: CourseId,
    assignment: AssignmentId,
) -> HttpResult<learning_data_access::StoredAssignment>
where
    S: Store + 'static,
{
    match state
        .store
        .get_assignment_for_edit(context, assignment)
        .await
    {
        Ok(Some(stored)) if stored.record.course_id == course => Ok(stored),
        Ok(Some(_)) | Ok(None) => {
            Err(error_response(StatusCode::NOT_FOUND, "assignment not found").into())
        }
        Err(error) => Err(store_error_response(error).into()),
    }
}

async fn resolve_audience<S>(
    state: &CourseRouteState<S>,
    authenticated: &crate::auth::AuthenticatedSession,
    course: CourseId,
    request: AssignmentAudienceRequest,
) -> HttpResult<AssignmentAudience>
where
    S: CourseGroupManagementStore + 'static,
{
    match request {
        AssignmentAudienceRequest::CourseWide => Ok(AssignmentAudience::CourseWide),
        AssignmentAudienceRequest::AnyOfGroups { groups } => {
            if groups.is_empty() {
                return Err(error_response(
                    StatusCode::UNPROCESSABLE_ENTITY,
                    "Choose at least one course group.",
                )
                .into());
            }
            let mut ids = Vec::with_capacity(groups.len());
            for reference in groups {
                let group = match state
                    .store
                    .get_course_group_by_reference(
                        authenticated.tenant_context,
                        authenticated.record.subject.user(),
                        course,
                        reference,
                    )
                    .await
                {
                    Ok(Some(group)) => group,
                    Ok(None) => {
                        return Err(error_response(StatusCode::NOT_FOUND, "group not found").into());
                    }
                    Err(error) => return Err(store_error_response(error).into()),
                };
                ids.push(group.group.record.id);
            }
            AssignmentAudience::any_of_groups(ids).map_err(|_| {
                error_response(
                    StatusCode::UNPROCESSABLE_ENTITY,
                    "Choose distinct course groups.",
                )
                .into()
            })
        }
    }
}

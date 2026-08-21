//! Instructor-only M2--M4 policy mutation routes.

mod support;

use std::sync::Arc;

use axum::body::to_bytes;
use axum::extract::{Path, Request, State};
use axum::http::header::{CONTENT_TYPE, ETAG};
use axum::http::{HeaderValue, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, put};
use axum::{Json, Router};
use domain::effective_assignment_policy::{
    AuthorizationGate, EffectivePolicyDecision, GroupAccommodation, GroupScheduleOffset,
    IndividualPolicyException, LateVerdict, PolicyModificationMode, PolicyPatch, PolicyPatchSet,
    PolicySource, ResolvedField, ScheduleOffsetSeconds, StartVerdict,
};
use learning_data_access::{
    AuthoritativeTimeStore, CourseGroupManagementStore, CourseRecordsAccessStore,
    DeleteGroupAccommodationCommand, DeleteGroupScheduleOffsetCommand,
    DeleteIndividualPolicyExceptionCommand, PutGroupAccommodationCommand,
    PutGroupScheduleOffsetCommand, PutIndividualPolicyExceptionCommand,
    ResolveEffectivePolicyCommand, SessionStore, Store, StoreError,
    TeachingAuthorityReferenceStore,
};
use question_model::{
    AssignmentId, AssignmentPolicyExceptionId, AssignmentPolicyPatchUpdateRequest,
    AssignmentTeachingSettingsField, CourseGroupReference, CourseId, CourseMembershipReference,
    GroupScheduleOffsetUpdateRequest, IndividualPolicyPatchUpdateRequest,
    TeachingAttemptLimitFieldPatch, TeachingLateVerdict, TeachingLimitFieldPatch,
    TeachingOperationRevision, TeachingOperationRevisionResponse,
    TeachingPreviewDeadlineBehaviorField, TeachingPreviewDenialReason, TeachingPreviewFieldSource,
    TeachingPreviewGroupSource, TeachingPreviewGroupSources, TeachingPreviewLateSubmissionField,
    TeachingPreviewLimitField, TeachingPreviewTimeField, TeachingPreviewView, TeachingStartVerdict,
    TeachingTimeFieldPatch,
};

use super::super::assignments::{AssignmentRevisionHeaderError, required_assignment_revision};
use super::super::policy::require_course_access;
use super::super::projection::{error_response, store_error_response};
use super::super::routing::{CourseRouteState, MAX_COURSE_BODY_BYTES};
use crate::auth::{auth_error_response, no_store, resolve_request_session};

use support::{hypothetical_source_response, label};

/// Builds the empty modifier and preview route group.
pub(super) fn router<S>(store: Arc<S>) -> Router
where
    S: Store
        + CourseRecordsAccessStore
        + AuthoritativeTimeStore
        + SessionStore
        + CourseGroupManagementStore
        + TeachingAuthorityReferenceStore
        + 'static,
{
    Router::new()
        .route(
            "/api/courses/{course}/assignments/{assignment}/group-schedule-offsets/{group}",
            put(put_group_schedule_offset::<S>).delete(delete_group_schedule_offset::<S>),
        )
        .route(
            "/api/courses/{course}/assignments/{assignment}/group-accommodations/{group}",
            put(put_group_accommodation::<S>).delete(delete_group_accommodation::<S>),
        )
        .route(
            "/api/courses/{course}/assignments/{assignment}/individual-policy-exceptions/{student}",
            put(put_individual_exception::<S>).delete(delete_individual_exception::<S>),
        )
        .route(
            "/api/courses/{course}/assignments/{assignment}/policy-preview/{student}",
            get(preview_effective_policy::<S>),
        )
        .with_state(CourseRouteState { store })
}
async fn put_group_schedule_offset<S>(
    State(state): State<CourseRouteState<S>>,
    Path((course, assignment, group)): Path<(CourseId, AssignmentId, CourseGroupReference)>,
    request: Request,
) -> Response
where
    S: Store + CourseRecordsAccessStore + CourseGroupManagementStore + SessionStore + 'static,
{
    let authenticated = match resolve_request_session(state.store.as_ref(), request.headers()).await
    {
        Ok(value) => value,
        Err(error) => return auth_error_response(error),
    };
    if let Err(response) =
        require_course_access(state.store.as_ref(), &authenticated, course, true).await
    {
        return response;
    }
    let expected_revision = match required_assignment_revision(request.headers()) {
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
    let assignment_record = match state
        .store
        .get_assignment_for_edit(authenticated.tenant_context, assignment)
        .await
    {
        Ok(Some(value)) if value.record.course_id == course => value,
        Ok(_) => return error_response(StatusCode::NOT_FOUND, "assignment not found"),
        Err(error) => return store_error_response(error),
    };
    if assignment_record.revision != expected_revision {
        return error_response(
            StatusCode::PRECONDITION_FAILED,
            "assignment changed; reload it",
        );
    }
    let group = match state
        .store
        .get_course_group_by_reference(
            authenticated.tenant_context,
            authenticated.record.subject.user(),
            course,
            group,
        )
        .await
    {
        Ok(Some(value)) => value,
        Ok(None) => return error_response(StatusCode::NOT_FOUND, "group not found"),
        Err(error) => return store_error_response(error),
    };
    if !request
        .headers()
        .get(CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .is_some_and(|value| {
            value
                .split(';')
                .next()
                .is_some_and(|value| value.trim() == "application/json")
        })
    {
        return error_response(StatusCode::UNSUPPORTED_MEDIA_TYPE, "request must be JSON");
    }
    let bytes = match to_bytes(request.into_body(), MAX_COURSE_BODY_BYTES + 1).await {
        Ok(value) if value.len() <= MAX_COURSE_BODY_BYTES => value,
        _ => return error_response(StatusCode::PAYLOAD_TOO_LARGE, "request is too large"),
    };
    let body: GroupScheduleOffsetUpdateRequest = match serde_json::from_slice(&bytes) {
        Ok(value) => value,
        Err(_) => return error_response(StatusCode::UNPROCESSABLE_ENTITY, "request is invalid"),
    };
    let offset_seconds = match ScheduleOffsetSeconds::try_new(body.offset_seconds.get()) {
        Ok(value) => value,
        Err(_) => {
            return error_response(
                StatusCode::UNPROCESSABLE_ENTITY,
                "schedule offset is invalid",
            );
        }
    };
    match state
        .store
        .put_group_schedule_offset(
            authenticated.tenant_context,
            PutGroupScheduleOffsetCommand {
                actor: authenticated.record.subject.user(),
                course,
                assignment,
                expected_revision,
                offset: GroupScheduleOffset {
                    group: group.group.record.id,
                    offset_seconds,
                },
            },
        )
        .await
    {
        Ok(revision) => revision_response(revision),
        Err(StoreError::Conflict) => error_response(
            StatusCode::PRECONDITION_FAILED,
            "assignment changed; reload it",
        ),
        Err(error) => store_error_response(error),
    }
}

fn revision_response(revision: learning_data_access::AssignmentRevision) -> Response {
    let revision =
        TeachingOperationRevision::new(revision.value()).expect("stored revision is valid");
    let mut response = Json(TeachingOperationRevisionResponse { revision }).into_response();
    response.headers_mut().insert(
        ETAG,
        HeaderValue::from_str(&format!("\"{revision}\"")).expect("valid ETag"),
    );
    no_store(response)
}
async fn delete_group_schedule_offset<S>(
    State(state): State<CourseRouteState<S>>,
    Path((course, assignment, group)): Path<(CourseId, AssignmentId, CourseGroupReference)>,
    request: Request,
) -> Response
where
    S: Store + CourseRecordsAccessStore + CourseGroupManagementStore + SessionStore + 'static,
{
    let (auth, revision) =
        match authorize_assignment(&state, course, assignment, request.headers()).await {
            Ok(value) => value,
            Err(response) => return response,
        };
    let group = match group_id(&state, &auth, course, group).await {
        Ok(value) => value,
        Err(response) => return response,
    };
    match state
        .store
        .delete_group_schedule_offset(
            auth.tenant_context,
            DeleteGroupScheduleOffsetCommand {
                actor: auth.record.subject.user(),
                course,
                assignment,
                expected_revision: revision,
                group,
            },
        )
        .await
    {
        Ok(value) => revision_response(value),
        Err(StoreError::Conflict) => error_response(
            StatusCode::PRECONDITION_FAILED,
            "assignment changed; reload it",
        ),
        Err(StoreError::NotFound) => {
            error_response(StatusCode::NOT_FOUND, "schedule offset not found")
        }
        Err(error) => store_error_response(error),
    }
}

async fn put_group_accommodation<S>(
    State(state): State<CourseRouteState<S>>,
    Path((course, assignment, group)): Path<(CourseId, AssignmentId, CourseGroupReference)>,
    request: Request,
) -> Response
where
    S: Store + CourseRecordsAccessStore + CourseGroupManagementStore + SessionStore + 'static,
{
    let (auth, revision) =
        match authorize_assignment(&state, course, assignment, request.headers()).await {
            Ok(value) => value,
            Err(response) => return response,
        };
    let group = match group_id(&state, &auth, course, group).await {
        Ok(value) => value,
        Err(response) => return response,
    };
    let term = match course_term(&state, auth.tenant_context, course).await {
        Ok(value) => value,
        Err(response) => return response,
    };
    let bytes = match json_body(request).await {
        Ok(value) => value,
        Err(response) => return response,
    };
    let body: AssignmentPolicyPatchUpdateRequest = match serde_json::from_slice(&bytes) {
        Ok(value) => value,
        Err(_) => return error_response(StatusCode::UNPROCESSABLE_ENTITY, "request is invalid"),
    };
    let patch = match patch(body.patch, &term) {
        Ok(value) => value,
        Err(response) => return response,
    };
    match state
        .store
        .put_group_accommodation(
            auth.tenant_context,
            PutGroupAccommodationCommand {
                actor: auth.record.subject.user(),
                course,
                assignment,
                expected_revision: revision,
                accommodation: GroupAccommodation {
                    group,
                    mode: mode(body.mode),
                    patch,
                },
            },
        )
        .await
    {
        Ok(value) => revision_response(value),
        Err(StoreError::Conflict) => error_response(
            StatusCode::PRECONDITION_FAILED,
            "assignment changed; reload it",
        ),
        Err(error) => store_error_response(error),
    }
}

async fn delete_group_accommodation<S>(
    State(state): State<CourseRouteState<S>>,
    Path((course, assignment, group)): Path<(CourseId, AssignmentId, CourseGroupReference)>,
    request: Request,
) -> Response
where
    S: Store + CourseRecordsAccessStore + CourseGroupManagementStore + SessionStore + 'static,
{
    let (auth, revision) =
        match authorize_assignment(&state, course, assignment, request.headers()).await {
            Ok(value) => value,
            Err(response) => return response,
        };
    let group = match group_id(&state, &auth, course, group).await {
        Ok(value) => value,
        Err(response) => return response,
    };
    match state
        .store
        .delete_group_accommodation(
            auth.tenant_context,
            DeleteGroupAccommodationCommand {
                actor: auth.record.subject.user(),
                course,
                assignment,
                expected_revision: revision,
                group,
            },
        )
        .await
    {
        Ok(value) => revision_response(value),
        Err(StoreError::Conflict) => error_response(
            StatusCode::PRECONDITION_FAILED,
            "assignment changed; reload it",
        ),
        Err(StoreError::NotFound) => {
            error_response(StatusCode::NOT_FOUND, "accommodation not found")
        }
        Err(error) => store_error_response(error),
    }
}

async fn put_individual_exception<S>(
    State(state): State<CourseRouteState<S>>,
    Path((course, assignment, student)): Path<(CourseId, AssignmentId, CourseMembershipReference)>,
    request: Request,
) -> Response
where
    S: Store
        + CourseRecordsAccessStore
        + CourseGroupManagementStore
        + SessionStore
        + TeachingAuthorityReferenceStore
        + 'static,
{
    let (auth, revision) =
        match authorize_assignment(&state, course, assignment, request.headers()).await {
            Ok(value) => value,
            Err(response) => return response,
        };
    let target = match student_target(&state, &auth, course, student).await {
        Ok(value) => value,
        Err(response) => return response,
    };
    let term = match course_term(&state, auth.tenant_context, course).await {
        Ok(value) => value,
        Err(response) => return response,
    };
    let bytes = match json_body(request).await {
        Ok(value) => value,
        Err(response) => return response,
    };
    let body: IndividualPolicyPatchUpdateRequest = match serde_json::from_slice(&bytes) {
        Ok(value) => value,
        Err(_) => return error_response(StatusCode::UNPROCESSABLE_ENTITY, "request is invalid"),
    };
    let patch = match patch(body.patch, &term) {
        Ok(value) => value,
        Err(response) => return response,
    };
    match state
        .store
        .put_individual_policy_exception(
            auth.tenant_context,
            PutIndividualPolicyExceptionCommand {
                actor: auth.record.subject.user(),
                course,
                assignment,
                expected_revision: revision,
                exception: learning_data_access::StoredIndividualPolicyException {
                    id: AssignmentPolicyExceptionId::generate(),
                    exception: IndividualPolicyException {
                        student: target.student,
                        mode: mode(body.mode),
                        patch,
                    },
                },
            },
        )
        .await
    {
        Ok(value) => revision_response(value),
        Err(StoreError::Conflict) => error_response(
            StatusCode::PRECONDITION_FAILED,
            "assignment changed; reload it",
        ),
        Err(error) => store_error_response(error),
    }
}

async fn delete_individual_exception<S>(
    State(state): State<CourseRouteState<S>>,
    Path((course, assignment, student)): Path<(CourseId, AssignmentId, CourseMembershipReference)>,
    request: Request,
) -> Response
where
    S: Store
        + CourseRecordsAccessStore
        + CourseGroupManagementStore
        + SessionStore
        + TeachingAuthorityReferenceStore
        + 'static,
{
    let (auth, revision) =
        match authorize_assignment(&state, course, assignment, request.headers()).await {
            Ok(value) => value,
            Err(response) => return response,
        };
    let target = match student_target(&state, &auth, course, student).await {
        Ok(value) => value,
        Err(response) => return response,
    };
    match state
        .store
        .delete_individual_policy_exception(
            auth.tenant_context,
            DeleteIndividualPolicyExceptionCommand {
                actor: auth.record.subject.user(),
                course,
                assignment,
                expected_revision: revision,
                student: target.student,
            },
        )
        .await
    {
        Ok(value) => revision_response(value),
        Err(StoreError::Conflict) => error_response(
            StatusCode::PRECONDITION_FAILED,
            "assignment changed; reload it",
        ),
        Err(StoreError::NotFound) => {
            error_response(StatusCode::NOT_FOUND, "individual exception not found")
        }
        Err(error) => store_error_response(error),
    }
}

async fn preview_effective_policy<S>(
    State(state): State<CourseRouteState<S>>,
    Path((course, assignment, student)): Path<(CourseId, AssignmentId, CourseMembershipReference)>,
    request: Request,
) -> Response
where
    S: Store
        + CourseRecordsAccessStore
        + AuthoritativeTimeStore
        + CourseGroupManagementStore
        + SessionStore
        + TeachingAuthorityReferenceStore
        + 'static,
{
    let auth = match resolve_request_session(state.store.as_ref(), request.headers()).await {
        Ok(value) => value,
        Err(error) => return auth_error_response(error),
    };
    if let Err(response) = require_course_access(state.store.as_ref(), &auth, course, true).await {
        return response;
    }
    let assignment_record = match state
        .store
        .get_assignment_for_edit(auth.tenant_context, assignment)
        .await
    {
        Ok(Some(value)) if value.record.course_id == course => value,
        Ok(_) => return error_response(StatusCode::NOT_FOUND, "assignment not found"),
        Err(error) => return store_error_response(error),
    };
    let target = match student_target(&state, &auth, course, student).await {
        Ok(value) => value,
        Err(response) => return response,
    };
    let term = match course_term(&state, auth.tenant_context, course).await {
        Ok(value) => value,
        Err(response) => return response,
    };
    let entitlement = match state
        .store
        .evaluate_assignment_entitlement(auth.tenant_context, target.user, course, assignment)
        .await
    {
        Ok(value) => value,
        Err(error) => return store_error_response(error),
    };
    let domain::entitlement::EntitlementDecision::Granted(grant) = entitlement else {
        return no_store(
            Json(TeachingPreviewView::Denied {
                reason: TeachingPreviewDenialReason::NotEntitled,
            })
            .into_response(),
        );
    };
    let now = match state.store.authoritative_time(auth.tenant_context).await {
        Ok(value) => value,
        Err(error) => return store_error_response(error),
    };
    let prior_run_count = match state
        .store
        .learner_get_enrollment_for_assignment(auth.tenant_context, target.user, assignment)
        .await
    {
        Ok(Some(enrollment)) => match state
            .store
            .learner_get_summary(auth.tenant_context, target.user, enrollment.id)
            .await
        {
            Ok(Some(summary)) => summary.summary.completed_run_count,
            Ok(None) => 0,
            Err(error) => return store_error_response(error),
        },
        Ok(None) => 0,
        Err(error) => return store_error_response(error),
    };
    let resolved = match state
        .store
        .resolve_effective_policy(
            auth.tenant_context,
            ResolveEffectivePolicyCommand {
                assignment,
                entitlement: domain::entitlement::EntitlementDecision::Granted(grant),
                authorization: AuthorizationGate::Authorized,
                now,
                prior_run_count,
            },
        )
        .await
    {
        Ok(Some(value)) => value,
        Ok(None) => return error_response(StatusCode::NOT_FOUND, "assignment not found"),
        Err(error) => return store_error_response(error),
    };
    let EffectivePolicyDecision::Allowed { policy, start } = resolved.decision else {
        return no_store(
            Json(TeachingPreviewView::Denied {
                reason: TeachingPreviewDenialReason::NotEntitled,
            })
            .into_response(),
        );
    };
    let view =
        match preview_view(&state, &auth, course, target.student, &term, *policy, start).await {
            Ok(value) => value,
            Err(response) => return response,
        };
    let _ = assignment_record;
    no_store(Json(view).into_response())
}
async fn authorize_assignment<S>(
    state: &CourseRouteState<S>,
    course: CourseId,
    assignment: AssignmentId,
    headers: &axum::http::HeaderMap,
) -> Result<
    (
        crate::auth::AuthenticatedSession,
        learning_data_access::AssignmentRevision,
    ),
    Response,
>
where
    S: Store + CourseRecordsAccessStore + SessionStore + 'static,
{
    let auth = resolve_request_session(state.store.as_ref(), headers)
        .await
        .map_err(auth_error_response)?;
    require_course_access(state.store.as_ref(), &auth, course, true).await?;
    let revision = match required_assignment_revision(headers) {
        Ok(value) => value,
        Err(AssignmentRevisionHeaderError::Missing) => {
            return Err(error_response(
                StatusCode::PRECONDITION_REQUIRED,
                "If-Match assignment revision is required",
            ));
        }
        Err(AssignmentRevisionHeaderError::Malformed) => {
            return Err(error_response(
                StatusCode::UNPROCESSABLE_ENTITY,
                "If-Match assignment revision is invalid",
            ));
        }
    };
    match state
        .store
        .get_assignment_for_edit(auth.tenant_context, assignment)
        .await
    {
        Ok(Some(value)) if value.record.course_id == course && value.revision == revision => {
            Ok((auth, revision))
        }
        Ok(Some(value)) if value.record.course_id == course => Err(error_response(
            StatusCode::PRECONDITION_FAILED,
            "assignment changed; reload it",
        )),
        Ok(Some(_)) => Err(error_response(
            StatusCode::NOT_FOUND,
            "assignment not found",
        )),
        Ok(None) => Err(error_response(
            StatusCode::NOT_FOUND,
            "assignment not found",
        )),
        Err(error) => Err(store_error_response(error)),
    }
}
async fn course_term<S>(
    state: &CourseRouteState<S>,
    context: learning_data_access::TenantContext,
    course: CourseId,
) -> Result<question_model::CourseTerm, Response>
where
    S: Store + 'static,
{
    match state.store.get_course(context, course).await {
        Ok(Some(value)) => Ok(value.term),
        Ok(None) => Err(error_response(StatusCode::NOT_FOUND, "course not found")),
        Err(error) => Err(store_error_response(error)),
    }
}
async fn group_id<S>(
    state: &CourseRouteState<S>,
    auth: &crate::auth::AuthenticatedSession,
    course: CourseId,
    reference: CourseGroupReference,
) -> Result<question_model::CourseGroupId, Response>
where
    S: CourseGroupManagementStore + 'static,
{
    match state
        .store
        .get_course_group_by_reference(
            auth.tenant_context,
            auth.record.subject.user(),
            course,
            reference,
        )
        .await
    {
        Ok(Some(value)) => Ok(value.group.record.id),
        Ok(None) => Err(error_response(StatusCode::NOT_FOUND, "group not found")),
        Err(error) => Err(store_error_response(error)),
    }
}
async fn student_target<S>(
    state: &CourseRouteState<S>,
    auth: &crate::auth::AuthenticatedSession,
    course: CourseId,
    reference: CourseMembershipReference,
) -> Result<learning_data_access::InstructorStudentTargetView, Response>
where
    S: TeachingAuthorityReferenceStore + 'static,
{
    match state
        .store
        .resolve_active_student_target_reference(
            auth.tenant_context,
            auth.record.subject.user(),
            course,
            reference,
        )
        .await
    {
        Ok(Some(value)) => Ok(value),
        Ok(None) => Err(error_response(StatusCode::NOT_FOUND, "student not found")),
        Err(error) => Err(store_error_response(error)),
    }
}
async fn json_body(request: Request) -> Result<axum::body::Bytes, Response> {
    if !request
        .headers()
        .get(CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .is_some_and(|value| {
            value
                .split(';')
                .next()
                .is_some_and(|value| value.trim() == "application/json")
        })
    {
        return Err(error_response(
            StatusCode::UNSUPPORTED_MEDIA_TYPE,
            "request must be JSON",
        ));
    }
    match to_bytes(request.into_body(), MAX_COURSE_BODY_BYTES + 1).await {
        Ok(value) if value.len() <= MAX_COURSE_BODY_BYTES => Ok(value),
        _ => Err(error_response(
            StatusCode::PAYLOAD_TOO_LARGE,
            "request is too large",
        )),
    }
}
fn mode(value: question_model::PolicyModificationModeView) -> PolicyModificationMode {
    match value {
        question_model::PolicyModificationModeView::ExtendOnly => {
            PolicyModificationMode::ExtendOnly
        }
        question_model::PolicyModificationModeView::Override => PolicyModificationMode::Override,
    }
}

#[allow(clippy::result_large_err)]
fn patch(
    value: question_model::PolicyPatchView,
    term: &question_model::CourseTerm,
) -> Result<PolicyPatchSet, Response> {
    Ok(PolicyPatchSet {
        available_at: time_patch(
            value.available_at,
            term,
            AssignmentTeachingSettingsField::AvailableAt,
        )?,
        due_at: time_patch(value.due_at, term, AssignmentTeachingSettingsField::DueAt)?,
        closes_at: time_patch(
            value.closes_at,
            term,
            AssignmentTeachingSettingsField::ClosesAt,
        )?,
        time_limit_seconds: limit_patch(value.time_limit_seconds),
        attempt_limit: attempt_patch(value.attempt_limit),
    })
}
#[allow(clippy::result_large_err)]
fn time_patch(
    value: TeachingTimeFieldPatch,
    term: &question_model::CourseTerm,
    field: AssignmentTeachingSettingsField,
) -> Result<PolicyPatch<question_model::ActivityTimestamp>, Response> {
    match value {
        TeachingTimeFieldPatch::Inherit => Ok(PolicyPatch::Inherit),
        TeachingTimeFieldPatch::Set { value } => {
            question_model::resolve_teaching_local_time(&value, term, field)
                .map(PolicyPatch::Set)
                .map_err(teaching_local_time_error)
        }
        TeachingTimeFieldPatch::Unrestricted => Ok(PolicyPatch::Unrestricted),
    }
}
fn teaching_local_time_error(
    error: question_model::AssignmentTeachingSettingsLocalError,
) -> Response {
    use question_model::AssignmentTeachingSettingsFailureCode;
    use question_model::AssignmentTeachingSettingsValidationFailure;

    no_store(
        (
            StatusCode::UNPROCESSABLE_ENTITY,
            Json(AssignmentTeachingSettingsValidationFailure {
                error: AssignmentTeachingSettingsFailureCode::AssignmentTeachingSettingsInvalid,
                field: error.field(),
                reason: error.reason(),
                message: "Correct the course-local schedule field.".to_owned(),
            }),
        )
            .into_response(),
    )
}
fn limit_patch(value: TeachingLimitFieldPatch) -> PolicyPatch<std::num::NonZeroU32> {
    match value {
        TeachingLimitFieldPatch::Inherit => PolicyPatch::Inherit,
        TeachingLimitFieldPatch::Set { value } => {
            PolicyPatch::Set(std::num::NonZeroU32::new(value.into()).expect("validated limit"))
        }
        TeachingLimitFieldPatch::Unrestricted => PolicyPatch::Unrestricted,
    }
}
fn attempt_patch(value: TeachingAttemptLimitFieldPatch) -> PolicyPatch<std::num::NonZeroU32> {
    match value {
        TeachingAttemptLimitFieldPatch::Inherit => PolicyPatch::Inherit,
        TeachingAttemptLimitFieldPatch::Set { value } => {
            PolicyPatch::Set(std::num::NonZeroU32::new(value.into()).expect("validated limit"))
        }
        TeachingAttemptLimitFieldPatch::Unrestricted => PolicyPatch::Unrestricted,
    }
}
async fn preview_view<S>(
    state: &CourseRouteState<S>,
    auth: &crate::auth::AuthenticatedSession,
    course: CourseId,
    student: question_model::StudentId,
    term: &question_model::CourseTerm,
    policy: domain::effective_assignment_policy::EffectiveAssignmentPolicy,
    start: StartVerdict,
) -> Result<TeachingPreviewView, Response>
where
    S: CourseGroupManagementStore + TeachingAuthorityReferenceStore + 'static,
{
    Ok(TeachingPreviewView::Allowed {
        time_zone: term.time_zone().clone(),
        start: start_view(start),
        available_at: preview_time_field(
            state,
            auth,
            course,
            student,
            term,
            policy.available_at,
            AssignmentTeachingSettingsField::AvailableAt,
        )
        .await?,
        due_at: preview_time_field(
            state,
            auth,
            course,
            student,
            term,
            policy.due_at,
            AssignmentTeachingSettingsField::DueAt,
        )
        .await?,
        closes_at: preview_time_field(
            state,
            auth,
            course,
            student,
            term,
            policy.closes_at,
            AssignmentTeachingSettingsField::ClosesAt,
        )
        .await?,
        time_limit_seconds: TeachingPreviewLimitField {
            value: policy.time_limit_seconds.value,
            source: source(
                state,
                auth,
                course,
                student,
                policy.time_limit_seconds.source,
            )
            .await?,
        },
        attempt_limit: TeachingPreviewLimitField {
            value: policy.attempt_limit.value,
            source: source(state, auth, course, student, policy.attempt_limit.source).await?,
        },
        late_submission: TeachingPreviewLateSubmissionField {
            value: policy.late_submission.value,
            source: source(state, auth, course, student, policy.late_submission.source).await?,
        },
        deadline_behavior: TeachingPreviewDeadlineBehaviorField {
            value: policy.deadline_behavior.value,
            source: source(
                state,
                auth,
                course,
                student,
                policy.deadline_behavior.source,
            )
            .await?,
        },
    })
}

async fn preview_time_field<S>(
    state: &CourseRouteState<S>,
    auth: &crate::auth::AuthenticatedSession,
    course: CourseId,
    student: question_model::StudentId,
    term: &question_model::CourseTerm,
    field: ResolvedField<Option<question_model::ActivityTimestamp>>,
    settings_field: AssignmentTeachingSettingsField,
) -> Result<TeachingPreviewTimeField, Response>
where
    S: CourseGroupManagementStore + TeachingAuthorityReferenceStore + 'static,
{
    let source = source(state, auth, course, student, field.source).await?;
    question_model::project_teaching_preview_time_field(field.value, source, term, settings_field)
        .map_err(|_| error_response(StatusCode::SERVICE_UNAVAILABLE, "preview time is invalid"))
}
fn start_view(value: StartVerdict) -> TeachingStartVerdict {
    match value {
        StartVerdict::MayStart { late } => TeachingStartVerdict::MayStart {
            late: match late {
                LateVerdict::OnTime => TeachingLateVerdict::OnTime,
                LateVerdict::AcceptedLate => TeachingLateVerdict::AcceptedLate,
                LateVerdict::MarkedLate => TeachingLateVerdict::MarkedLate,
                LateVerdict::RejectedLate => TeachingLateVerdict::MarkedLate,
            },
        },
        StartVerdict::NotYetAvailable => TeachingStartVerdict::NotYetAvailable,
        StartVerdict::Closed => TeachingStartVerdict::Closed,
        StartVerdict::AttemptLimitReached => TeachingStartVerdict::AttemptLimitReached,
        StartVerdict::DueDateRejectsNewRun => TeachingStartVerdict::DueDateRejectsNewRun,
    }
}
async fn source<S>(
    state: &CourseRouteState<S>,
    auth: &crate::auth::AuthenticatedSession,
    course: CourseId,
    student: question_model::StudentId,
    value: PolicySource,
) -> Result<TeachingPreviewFieldSource, Response>
where
    S: CourseGroupManagementStore + TeachingAuthorityReferenceStore + 'static,
{
    match value {
        PolicySource::Base => Ok(TeachingPreviewFieldSource::Base {
            label: label("Assignment policy")?,
        }),
        PolicySource::GroupScheduleOffsets(groups) => {
            Ok(TeachingPreviewFieldSource::GroupScheduleOffsets {
                groups: groups_view(state, auth, course, groups).await?,
            })
        }
        PolicySource::GroupAccommodations(groups) => {
            Ok(TeachingPreviewFieldSource::GroupAccommodations {
                groups: groups_view(state, auth, course, groups).await?,
            })
        }
        PolicySource::IndividualException(_) => {
            let view = state
                .store
                .active_student_membership_reference_view(
                    auth.tenant_context,
                    auth.record.subject.user(),
                    course,
                    student,
                )
                .await
                .map_err(store_error_response)?;
            let Some(view) = view else {
                return Err(error_response(StatusCode::NOT_FOUND, "student not found"));
            };
            Ok(TeachingPreviewFieldSource::Membership {
                membership: view.reference,
                label: label(&view.display_name)?,
            })
        }
        PolicySource::HypotheticalIndividualException => Err(hypothetical_source_response()),
    }
}
async fn groups_view<S>(
    state: &CourseRouteState<S>,
    auth: &crate::auth::AuthenticatedSession,
    course: CourseId,
    groups: Vec<question_model::CourseGroupId>,
) -> Result<TeachingPreviewGroupSources, Response>
where
    S: CourseGroupManagementStore + 'static,
{
    let mut views = Vec::with_capacity(groups.len());
    for group in groups {
        let view = state
            .store
            .get_course_group_by_id_for_instructor(
                auth.tenant_context,
                auth.record.subject.user(),
                course,
                group,
            )
            .await
            .map_err(store_error_response)?;
        let Some(view) = view else {
            return Err(error_response(StatusCode::NOT_FOUND, "group not found"));
        };
        views.push(TeachingPreviewGroupSource {
            group: view.reference,
            label: label(&view.group.record.title)?,
        });
    }
    TeachingPreviewGroupSources::try_from(views).map_err(|_| {
        error_response(
            StatusCode::SERVICE_UNAVAILABLE,
            "preview provenance is invalid",
        )
    })
}
#[cfg(test)]
#[path = "modifiers_preview/tests.rs"]
mod tests;

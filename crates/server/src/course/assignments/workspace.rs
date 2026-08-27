//! Focused Instructor assignment-workspace HTTP operations.

use axum::Json;
use axum::extract::{Path, Request, State};
use axum::http::header::LOCATION;
use axum::http::{HeaderMap, HeaderValue, StatusCode};
use axum::response::{IntoResponse, Response};
use learning_data_access::{
    AssignmentContentUpdate, AssignmentPoliciesUpdate, AuthoritativeTimeStore, CatalogStore,
    CourseGroupManagementStore, CourseRecordsAccessStore, CreateAssignmentDraftCommand,
    ReplaceAssignmentContentCommand, ReplaceAssignmentContentOutcome,
    ReplaceAssignmentFixedItemCommand, ReplaceAssignmentPoliciesCommand,
    ReplaceAssignmentPoliciesOutcome, SessionStore, Store, StoreError,
};
use question_model::{
    AssignmentAudience, AssignmentAudienceRequest, AssignmentAudienceValidationReason,
    AssignmentContentIssuedWorkConflict, AssignmentContentIssuedWorkConflictKind, AssignmentId,
    AssignmentItemId, AssignmentPoliciesValidationFailure, AssignmentPoliciesValidationFailureCode,
    AssignmentPoliciesValidationIssue, AssignmentPolicyConfigurationReason, CourseId,
    CreateAssignmentDraftRequest, InstructorStudentView, ReplaceAssignmentContentRequest,
    ReplaceAssignmentFixedItemRequest, ReplaceAssignmentPoliciesRequest,
};

use super::super::policy::require_course_access;
use super::super::projection::{error_response, store_error_response};
use super::super::routing::{CourseRouteState, strict_assignment_request};
use super::{
    AssignmentRevisionHeaderError, assignment_landing_presentation, assignment_response,
    definition_request, instructor_student_view_delivery, required_assignment_revision,
};
use crate::auth::{auth_error_response, no_store, resolve_request_session};
use crate::http_refusal::{HttpRefusal, HttpResult};

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
        Err(response) => return response.into_response(),
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
        Ok(ReplaceAssignmentContentOutcome::Issued) => issued_assignment_content_response(),
        Err(StoreError::Conflict) => error_response(
            StatusCode::CONFLICT,
            "assignment content could not be changed in its current state",
        ),
        Err(error) => store_error_response(error),
    }
}

/// Replaces one assignment-owned fixed slot's immutable publication for
/// future runs while preserving any issued learner evidence.
pub(in crate::course) async fn replace_assignment_fixed_item<S>(
    State(state): State<CourseRouteState<S>>,
    Path((course, assignment, item)): Path<(CourseId, AssignmentId, AssignmentItemId)>,
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
    // ASVS 8.2.1, 8.2.2, 8.3.1, and 8.4.1: the trusted service derives the
    // instructor and tenant, authorizes the exact course, and binds the
    // nested assignment/item rather than accepting browser authority claims.
    if let Err(response) =
        require_course_access(state.store.as_ref(), &authenticated, course, true).await
    {
        return response.into_response();
    }
    let current =
        match exact_assignment(&state, authenticated.tenant_context, course, assignment).await {
            Ok(stored) => stored,
            Err(response) => return response.into_response(),
        };
    if !current.record.items.iter().any(|fixed| fixed.id == item) {
        return error_response(StatusCode::NOT_FOUND, "assignment not found");
    }
    let expected_revision = match revision_or_response(request.headers()) {
        Ok(revision) => revision,
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
    let request = match strict_assignment_request::<ReplaceAssignmentFixedItemRequest>(value) {
        Ok(request) => request,
        Err(()) => {
            return error_response(
                StatusCode::UNPROCESSABLE_ENTITY,
                "Use the Questions workspace to send one valid Question ID.",
            );
        }
    };
    let replacement = match definition_request::resolve_assignable_question_id(
        &state,
        authenticated.tenant_context,
        request.question_id,
    )
    .await
    {
        Ok(replacement) => replacement,
        Err(response) => return response.into_response(),
    };
    let candidate = learning_data_access::AssignmentRecord {
        items: current
            .record
            .items
            .iter()
            .map(|fixed| {
                let mut fixed = fixed.clone();
                if fixed.id == item {
                    fixed.reference = replacement;
                }
                fixed
            })
            .collect(),
        ..current.record.clone()
    };
    // ASVS 2.2.3: replacement retains the complete aggregate's documented
    // structural invariants, including unique immutable publications, before
    // the focused Store command changes the future-run definition.
    let mut unique_references = std::collections::BTreeSet::new();
    if !candidate
        .references()
        .all(|reference| unique_references.insert(reference))
    {
        return error_response(
            StatusCode::UNPROCESSABLE_ENTITY,
            "assignment Question ID is already used in this definition",
        );
    }
    if let Err(response) = definition_request::validate_assignment_request(
        &state,
        authenticated.tenant_context,
        &candidate,
    )
    .await
    {
        return response.into_response();
    }
    // ASVS 2.3.3 and 15.4.2: the Store performs the assignment-locked
    // compare-and-swap with the mutation, so a race cannot overwrite a newer
    // aggregate revision after this handler's preflight comparison.
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
                replacement,
            },
        )
        .await
    {
        Ok(stored) => assignment_response(&state, &authenticated, StatusCode::OK, stored).await,
        Err(StoreError::Conflict) => error_response(
            StatusCode::PRECONDITION_FAILED,
            "assignment changed; reload it",
        ),
        // ASVS 16.5.1: store_error_response keeps internal Store diagnostics
        // out of the browser-safe no-store error envelope.
        Err(error) => store_error_response(error),
    }
}

/// Gives the Questions client one closed, semantic recovery discriminant.
///
/// This response identifies the durable case where immutable learner evidence
/// makes a structural definition change unavailable on this assignment. Other
/// `409` responses retain their own route-specific conflict semantics. The
/// browser maps this discriminator to visible guidance without presenting route
/// details or internal identifiers from an error body.
fn issued_assignment_content_response() -> Response {
    no_store(
        (
            StatusCode::CONFLICT,
            Json(AssignmentContentIssuedWorkConflict {
                kind: AssignmentContentIssuedWorkConflictKind::IssuedLearnerWork,
            }),
        )
            .into_response(),
    )
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
        Err(response) => return response.into_response(),
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
            return policy_validation_response(vec![
                AssignmentPoliciesValidationIssue::TeachingSettings {
                    correction: teaching_settings_validation_failure(error.field(), error.reason()),
                },
            ]);
        }
    };
    if !domain::effective_assignment_policy::is_legal_assignment_lifecycle_transition(
        current.record.lifecycle,
        teaching_settings.lifecycle,
    ) {
        return policy_validation_response(vec![
            AssignmentPoliciesValidationIssue::TeachingSettings {
                correction: teaching_settings_validation_failure(
                    question_model::AssignmentTeachingSettingsField::Lifecycle,
                    question_model::AssignmentTeachingSettingsFailureReason::IllegalLifecycleTransition,
                ),
            },
        ]);
    }
    // Audience resolution is one independently correctable part of a valid
    // teaching-state candidate.  A rejected browser audience falls back only
    // while deriving the other candidate facts; the write is still refused
    // with its closed audience correction.
    let (audience, audience_issue) =
        match resolve_audience(&state, &authenticated, course, request.audience).await {
            Ok(AudienceResolution::Resolved(audience)) => (audience, None),
            Ok(AudienceResolution::Issue(reason)) => {
                (current.record.audience.clone(), Some(reason))
            }
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
    let facts = match definition_request::collect_assignment_policy_validation_facts(
        &state,
        authenticated.tenant_context,
        &candidate,
    )
    .await
    {
        Ok(facts) => facts,
        Err(response) => return response.into_response(),
    };
    let mut issues = audience_issue
        .into_iter()
        .map(|reason| AssignmentPoliciesValidationIssue::Audience { reason })
        .collect::<Vec<_>>();
    issues.extend(facts
        .into_iter()
        .map(|fact| match fact {
            definition_request::AssignmentPolicyValidationFact::SelectedProblemVariantsWithSelectionGroups => {
                AssignmentPoliciesValidationIssue::Configuration {
                    reason:
                        AssignmentPolicyConfigurationReason::SelectedProblemVariantsWithSelectionGroups,
                }
            }
            definition_request::AssignmentPolicyValidationFact::Capability {
                title,
                question_id,
                capability,
            } => AssignmentPoliciesValidationIssue::Capability {
                title,
                question_id,
                capability,
            },
        }));
    let readiness = candidate.publication_readiness();
    if teaching_settings.lifecycle == question_model::AssignmentLifecycle::Published
        && !readiness.is_ready()
    {
        issues.push(AssignmentPoliciesValidationIssue::PublicationReadiness {
            blocking_issues: readiness.blocking_issues,
        });
    }
    if !issues.is_empty() {
        return policy_validation_response(issues);
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
    let landing =
        assignment_landing_presentation(&stored.record, course_record.term.time_zone().clone());
    no_store(Json(InstructorStudentView::from_landing(landing, delivery)).into_response())
}

fn revision_or_response(headers: &HeaderMap) -> HttpResult<question_model::AssignmentRevision> {
    required_assignment_revision(headers).map_err(|error| match error {
        AssignmentRevisionHeaderError::Missing => HttpRefusal::from(error_response(
            StatusCode::PRECONDITION_REQUIRED,
            "If-Match assignment revision is required",
        )),
        AssignmentRevisionHeaderError::Malformed => HttpRefusal::from(error_response(
            StatusCode::UNPROCESSABLE_ENTITY,
            "If-Match assignment revision is invalid",
        )),
    })
}

fn policy_validation_response(issues: Vec<AssignmentPoliciesValidationIssue>) -> Response {
    // ASVS 2.2.1, 2.2.2, and 2.3.3: the authenticated service returns a
    // closed correction envelope before the aggregate write can advance its
    // revision. The envelope contains only browser-safe teaching facts.
    no_store(
        (
            StatusCode::UNPROCESSABLE_ENTITY,
            Json(AssignmentPoliciesValidationFailure {
                error: AssignmentPoliciesValidationFailureCode::AssignmentPoliciesInvalid,
                issues,
            }),
        )
            .into_response(),
    )
}

fn teaching_settings_validation_failure(
    field: question_model::AssignmentTeachingSettingsField,
    reason: question_model::AssignmentTeachingSettingsFailureReason,
) -> question_model::AssignmentTeachingSettingsValidationFailure {
    use question_model::AssignmentTeachingSettingsFailureReason as Reason;

    let message = match reason {
        Reason::InvalidInput => "Enter complete teaching settings using the form fields.",
        Reason::CourseTimeZoneMismatch => "Use the course time zone shown with this form.",
        Reason::OutsideCourseTerm => "Choose a time inside the course term.",
        Reason::NonexistentLocalTime => {
            "Choose a local time that exists on this daylight-saving date."
        }
        Reason::AmbiguousLocalTime => {
            "Choose a local time outside the daylight-saving repeat hour."
        }
        Reason::TimestampOutOfRange => "Choose a supported calendar time.",
        Reason::ScheduleOutOfOrder => {
            "Keep available, due, and close times in chronological order."
        }
        Reason::TimeLimitOutOfRange => "Choose a supported whole-run time limit.",
        Reason::AttemptLimitOutOfRange => "Choose a supported attempt limit.",
        Reason::IllegalLifecycleTransition => "Choose a permitted assignment lifecycle change.",
        Reason::InvalidInstructions => "Enter plain-text instructions within the supported length.",
    };
    question_model::AssignmentTeachingSettingsValidationFailure {
        error:
            question_model::AssignmentTeachingSettingsFailureCode::AssignmentTeachingSettingsInvalid,
        field,
        reason,
        message: message.to_string(),
    }
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

enum AudienceResolution {
    Resolved(AssignmentAudience),
    Issue(AssignmentAudienceValidationReason),
}

async fn resolve_audience<S>(
    state: &CourseRouteState<S>,
    authenticated: &crate::auth::AuthenticatedSession,
    course: CourseId,
    request: AssignmentAudienceRequest,
) -> HttpResult<AudienceResolution>
where
    S: CourseGroupManagementStore + 'static,
{
    match request {
        AssignmentAudienceRequest::CourseWide => {
            Ok(AudienceResolution::Resolved(AssignmentAudience::CourseWide))
        }
        AssignmentAudienceRequest::AnyOfGroups { groups } => {
            if groups.is_empty() {
                return Ok(AudienceResolution::Issue(
                    AssignmentAudienceValidationReason::GroupRequired,
                ));
            }
            let mut ids = Vec::with_capacity(groups.len());
            let mut references = std::collections::BTreeSet::new();
            for reference in groups {
                if !references.insert(reference) {
                    return Ok(AudienceResolution::Issue(
                        AssignmentAudienceValidationReason::GroupsMustBeDistinct,
                    ));
                }
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
                        return Ok(AudienceResolution::Issue(
                            AssignmentAudienceValidationReason::GroupUnavailable,
                        ));
                    }
                    Err(error) => return Err(store_error_response(error).into()),
                };
                ids.push(group.group.record.id);
            }
            match AssignmentAudience::any_of_groups(ids) {
                Ok(audience) => Ok(AudienceResolution::Resolved(audience)),
                Err(_) => Ok(AudienceResolution::Issue(
                    AssignmentAudienceValidationReason::GroupsMustBeDistinct,
                )),
            }
        }
    }
}

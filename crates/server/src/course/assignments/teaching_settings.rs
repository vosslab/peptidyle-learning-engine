//! Teaching-settings mutation and browser-safe validation responses.

use super::super::routing;
use super::*;

/// Atomically saves lifecycle, learner instructions, and the assignment base
/// policy. This route authorizes before it reads or decodes the request body,
/// so an unauthorized caller cannot use malformed input as an oracle.
pub(in crate::course) async fn put_teaching_settings<S>(
    State(state): State<CourseRouteState<S>>,
    Path((course, assignment)): Path<(CourseId, AssignmentId)>,
    request: Request,
) -> Response
where
    S: Store
        + learning_data_access::AuthoritativeTimeStore
        + CatalogStore
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
        return response;
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
    if current.revision != expected_revision {
        return error_response(
            StatusCode::PRECONDITION_FAILED,
            "assignment changed; reload it",
        );
    }
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
        return error_response(
            StatusCode::UNSUPPORTED_MEDIA_TYPE,
            "teaching settings must be JSON",
        );
    }
    let body = match to_bytes(request.into_body(), routing::MAX_COURSE_BODY_BYTES + 1).await {
        Ok(value) if value.len() <= routing::MAX_COURSE_BODY_BYTES => value,
        Ok(_) | Err(_) => {
            return error_response(
                StatusCode::PAYLOAD_TOO_LARGE,
                "teaching settings are too large",
            );
        }
    };
    let value: serde_json::Value = match serde_json::from_slice(&body) {
        Ok(value) => value,
        Err(_) => {
            return teaching_settings_validation_response(
                question_model::AssignmentTeachingSettingsField::TeachingSettings,
                question_model::AssignmentTeachingSettingsFailureReason::InvalidInput,
            );
        }
    };
    if let Some(instructions) = value.get("instructions") {
        let Some(instructions) = instructions.as_str() else {
            return teaching_settings_validation_response(
                question_model::AssignmentTeachingSettingsField::Instructions,
                question_model::AssignmentTeachingSettingsFailureReason::InvalidInstructions,
            );
        };
        if AssignmentInstructions::try_new(instructions.to_string()).is_err() {
            return teaching_settings_validation_response(
                question_model::AssignmentTeachingSettingsField::Instructions,
                question_model::AssignmentTeachingSettingsFailureReason::InvalidInstructions,
            );
        }
    }
    let request = match strict_assignment_request::<AssignmentTeachingSettingsRequest>(value) {
        Ok(request) => request,
        Err(()) => {
            return teaching_settings_validation_response(
                question_model::AssignmentTeachingSettingsField::TeachingSettings,
                question_model::AssignmentTeachingSettingsFailureReason::InvalidInput,
            );
        }
    };
    if !domain::effective_assignment_policy::is_legal_assignment_lifecycle_transition(
        current.record.lifecycle,
        request.lifecycle,
    ) {
        return teaching_settings_validation_response(
            question_model::AssignmentTeachingSettingsField::Lifecycle,
            question_model::AssignmentTeachingSettingsFailureReason::IllegalLifecycleTransition,
        );
    }
    let course_record = match state
        .store
        .get_course(authenticated.tenant_context, course)
        .await
    {
        Ok(Some(course_record)) => course_record,
        Ok(None) => return error_response(StatusCode::NOT_FOUND, "course not found"),
        Err(error) => return store_error_response(error),
    };
    let settings = match request.into_absolute(&course_record.term) {
        Ok(settings) => settings,
        Err(error) => {
            return teaching_settings_validation_response(error.field(), error.reason());
        }
    };
    let command = learning_data_access::PutAssignmentTeachingSettingsCommand {
        actor: authenticated.record.subject.user(),
        course,
        assignment,
        expected_revision,
        settings,
    };
    match state
        .store
        .put_assignment_teaching_settings(authenticated.tenant_context, command)
        .await
    {
        Ok(_) => match state
            .store
            .get_assignment_for_edit(authenticated.tenant_context, assignment)
            .await
        {
            Ok(Some(updated)) => {
                assignment_response(&state, &authenticated, StatusCode::OK, updated).await
            }
            Ok(None) => error_response(StatusCode::NOT_FOUND, "assignment not found"),
            Err(error) => store_error_response(error),
        },
        Err(StoreError::Conflict) => error_response(
            StatusCode::PRECONDITION_FAILED,
            "assignment changed; reload it",
        ),
        Err(error) => store_error_response(error),
    }
}

pub(super) fn teaching_settings_validation_response(
    field: question_model::AssignmentTeachingSettingsField,
    reason: question_model::AssignmentTeachingSettingsFailureReason,
) -> Response {
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
    no_store(
        (
            StatusCode::UNPROCESSABLE_ENTITY,
            Json(question_model::AssignmentTeachingSettingsValidationFailure {
                error:
                    question_model::AssignmentTeachingSettingsFailureCode::AssignmentTeachingSettingsInvalid,
                field,
                reason,
                message: message.to_string(),
            }),
        )
            .into_response(),
    )
}

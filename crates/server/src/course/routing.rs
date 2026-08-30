use std::sync::Arc;

use axum::Router;
use axum::extract::DefaultBodyLimit;
use axum::routing::{get, post, put};
use learning_data_access::{
    AuthoritativeTimeStore, CatalogStore, CourseGradebookStore, CourseGroupManagementStore,
    CourseInvitationDeliveryStore, CourseItemAnalysisStore, CourseRecordsAccessStore,
    CourseRosterStore, GradingOperationStore, NavigationReferenceStore,
    PoolPreviewStore, PreviewPlaneStore, SessionStore, Store, StudentWorkInspectionStore,
    TeachingAuthorityReferenceStore, TeachingAuthorityStore,
};
use serde::{Deserialize, Serialize};

use super::assignments::{
    create_assignment_draft, get_assignment_summary, get_assignment_workspace,
    get_instructor_student_view, get_student_assignment, replace_assignment_content,
    replace_assignment_fixed_item, replace_assignment_policies,
};
use super::grading_operations::{
    list_grading_operations, recalculate_assignment, retry_grading_operation,
};
use super::invitation_capability::CourseInvitationIssuer;
use super::queries::{create_course, get_course, list_assignments, list_courses};
use super::roster::roster_router;

pub(super) const DEFAULT_PAGE_SIZE: u16 = 50;
pub(super) const MAX_COURSE_BODY_BYTES: usize = 64 * 1_024;

/// Builds the authenticated course and assignment route group.
pub fn router<S>(store: Arc<S>) -> Router
where
    S: Store
        + CatalogStore
        + CourseItemAnalysisStore
        + CourseRecordsAccessStore
        + CourseRosterStore
        + CourseInvitationDeliveryStore
        + CourseGradebookStore
        + CourseGroupManagementStore
        + GradingOperationStore
        + SessionStore
        + TeachingAuthorityStore
        + TeachingAuthorityReferenceStore
        + AuthoritativeTimeStore
        + NavigationReferenceStore
        + PoolPreviewStore
        + PreviewPlaneStore
        + StudentWorkInspectionStore
        + 'static,
{
    router_with_invitations(store, CourseInvitationIssuer::unavailable())
}

/// Builds course routes with a configured server-only invitation issuer.
pub fn router_with_invitations<S>(store: Arc<S>, issuer: CourseInvitationIssuer) -> Router
where
    S: Store
        + CatalogStore
        + CourseItemAnalysisStore
        + CourseRecordsAccessStore
        + CourseRosterStore
        + CourseInvitationDeliveryStore
        + CourseGradebookStore
        + CourseGroupManagementStore
        + GradingOperationStore
        + SessionStore
        + TeachingAuthorityStore
        + TeachingAuthorityReferenceStore
        + AuthoritativeTimeStore
        + NavigationReferenceStore
        + PoolPreviewStore
        + PreviewPlaneStore
        + StudentWorkInspectionStore
        + 'static,
{
    let course_routes = Router::new()
        .route(
            "/api/courses",
            get(list_courses::<S>).post(create_course::<S>),
        )
        .route(
            "/api/courses/{course}/assignments",
            get(list_assignments::<S>),
        )
        .route(
            "/api/courses/{course}/assignments/drafts",
            post(create_assignment_draft::<S>),
        )
        .route(
            "/api/courses/{course}/gradebook",
            get(super::gradebook::get_calculated_gradebook::<S>),
        )
        .route(
            "/api/courses/{course}/gradebook/selection",
            get(super::gradebook::get_gradebook_selection::<S>),
        )
        .route(
            "/api/courses/{course}/gradebook/students/{membership}/assignments/{assignment}/runs",
            get(super::gradebook::get_submitted_run_choices::<S>),
        )
        .route(
            "/api/courses/{course}/gradebook/students/{membership}/assignments/{assignment}/runs/{run}",
            get(super::gradebook::get_student_work::<S>),
        )
        .route(
            "/api/courses/{course}/grade-scheme",
            get(super::gradebook::get_scheme::<S>).put(super::gradebook::put_scheme::<S>),
        )
        .route(
            "/api/courses/{course}/gradebook-totals",
            get(super::gradebook::get_totals::<S>),
        )
        .route(
            "/api/courses/{course}/grade-export.csv",
            post(super::gradebook::create_export::<S>),
        )
        .route("/api/courses/{course}", get(get_course::<S>))
        .route(
            "/api/assignments/{assignment}/student",
            get(get_student_assignment::<S>),
        )
        .route(
            "/api/assignments/{assignment}/summary",
            get(get_assignment_summary::<S>),
        )
        .route(
            "/api/courses/{course}/assignments/{assignment}",
            get(get_assignment_workspace::<S>),
        )
        .route(
            "/api/courses/{course}/assignments/{assignment}/content",
            put(replace_assignment_content::<S>),
        )
        .route(
            "/api/courses/{course}/assignments/{assignment}/fixed-items/{itemId}",
            put(replace_assignment_fixed_item::<S>),
        )
        .route(
            "/api/courses/{course}/assignments/{assignment}/policies",
            put(replace_assignment_policies::<S>),
        )
        .route(
            "/api/courses/{course}/assignments/{assignment}/student-view",
            get(get_instructor_student_view::<S>),
        )
        .route(
            "/api/courses/{course}/assignments/{assignment}/grading-operations",
            get(list_grading_operations::<S>),
        )
        .route(
            "/api/courses/{course}/assignments/{assignment}/grading-operations/{operation}/retry",
            post(retry_grading_operation::<S>),
        )
        .route(
            "/api/courses/{course}/assignments/{assignment}/grading-operations/recalculate",
            post(recalculate_assignment::<S>),
        )
        .layer(DefaultBodyLimit::max(MAX_COURSE_BODY_BYTES))
        .with_state(CourseRouteState {
            store: Arc::clone(&store),
        });
    course_routes
        .merge(roster_router(Arc::clone(&store), issuer))
        .merge(super::teaching_operations::router(store))
}

pub(super) struct CourseRouteState<S> {
    pub(super) store: Arc<S>,
}

impl<S> Clone for CourseRouteState<S> {
    fn clone(&self) -> Self {
        Self {
            store: Arc::clone(&self.store),
        }
    }
}

pub(super) use question_model::AssignmentEntryRequest;

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[serde(deny_unknown_fields)]
pub(super) struct CourseQuery {
    pub(super) cursor: Option<String>,
    pub(super) page_size: Option<u16>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct CreateCourseRequest {
    pub(super) title: String,
    pub(super) term: question_model::CourseTerm,
}

pub(super) enum CreateCourseDecodeError {
    Invalid,
    Term(question_model::CourseTermValidationFailure),
}

pub(super) fn decode_course_create_request(
    value: serde_json::Value,
) -> Result<CreateCourseRequest, CreateCourseDecodeError> {
    let record = value.as_object().ok_or(CreateCourseDecodeError::Invalid)?;
    if record
        .keys()
        .any(|field| field != "title" && field != "term")
    {
        return Err(CreateCourseDecodeError::Invalid);
    }
    let term_value = record.get("term").ok_or_else(|| {
        CreateCourseDecodeError::Term(course_term_failure(
            question_model::CourseTermField::Term,
            question_model::CourseTermFailureReason::Required,
        ))
    })?;
    let term_record = term_value.as_object().ok_or_else(|| {
        CreateCourseDecodeError::Term(course_term_failure(
            question_model::CourseTermField::Term,
            question_model::CourseTermFailureReason::Required,
        ))
    })?;
    if term_record
        .keys()
        .any(|field| field != "startDate" && field != "endDate" && field != "timeZone")
    {
        return Err(CreateCourseDecodeError::Invalid);
    }
    let title = record
        .get("title")
        .and_then(serde_json::Value::as_str)
        .ok_or(CreateCourseDecodeError::Invalid)?;
    if title.trim().is_empty() {
        return Err(CreateCourseDecodeError::Invalid);
    }
    let start_date = required_term_string(
        term_record,
        "startDate",
        question_model::CourseTermField::StartDate,
    )?;
    let end_date = required_term_string(
        term_record,
        "endDate",
        question_model::CourseTermField::EndDate,
    )?;
    let time_zone = required_term_string(
        term_record,
        "timeZone",
        question_model::CourseTermField::TimeZone,
    )?;
    let term = question_model::CourseTerm::from_parts(start_date, end_date, time_zone).map_err(
        |error| {
            let (field, reason) = match error {
                question_model::CourseTermError::StartDate => (
                    question_model::CourseTermField::StartDate,
                    question_model::CourseTermFailureReason::InvalidCalendarDate,
                ),
                question_model::CourseTermError::EndDate => (
                    question_model::CourseTermField::EndDate,
                    question_model::CourseTermFailureReason::InvalidCalendarDate,
                ),
                question_model::CourseTermError::EndBeforeStart => (
                    question_model::CourseTermField::EndDate,
                    question_model::CourseTermFailureReason::EndBeforeStart,
                ),
                question_model::CourseTermError::TimeZone => (
                    question_model::CourseTermField::TimeZone,
                    question_model::CourseTermFailureReason::UnknownIanaTimeZone,
                ),
            };
            CreateCourseDecodeError::Term(course_term_failure(field, reason))
        },
    )?;
    Ok(CreateCourseRequest {
        title: title.to_string(),
        term,
    })
}

fn required_term_string<'a>(
    record: &'a serde_json::Map<String, serde_json::Value>,
    name: &str,
    field: question_model::CourseTermField,
) -> Result<&'a str, CreateCourseDecodeError> {
    let Some(value) = record.get(name) else {
        return Err(CreateCourseDecodeError::Term(course_term_failure(
            field,
            question_model::CourseTermFailureReason::Required,
        )));
    };
    value.as_str().ok_or_else(|| {
        let reason = match field {
            question_model::CourseTermField::StartDate
            | question_model::CourseTermField::EndDate => {
                question_model::CourseTermFailureReason::InvalidCalendarDate
            }
            question_model::CourseTermField::TimeZone => {
                question_model::CourseTermFailureReason::UnknownIanaTimeZone
            }
            question_model::CourseTermField::Term => {
                question_model::CourseTermFailureReason::Required
            }
        };
        CreateCourseDecodeError::Term(course_term_failure(field, reason))
    })
}

fn course_term_failure(
    field: question_model::CourseTermField,
    reason: question_model::CourseTermFailureReason,
) -> question_model::CourseTermValidationFailure {
    let message = match (field, reason) {
        (question_model::CourseTermField::Term, _) => "Enter the course term dates and time zone.",
        (_, question_model::CourseTermFailureReason::Required) => match field {
            question_model::CourseTermField::StartDate => "Enter a course start date.",
            question_model::CourseTermField::EndDate => "Enter a course end date.",
            question_model::CourseTermField::TimeZone => "Enter an IANA time zone.",
            question_model::CourseTermField::Term => "Enter the course term dates and time zone.",
        },
        (_, question_model::CourseTermFailureReason::InvalidCalendarDate) => {
            "Enter a valid date in YYYY-MM-DD format."
        }
        (_, question_model::CourseTermFailureReason::EndBeforeStart) => {
            "Choose an end date on or after the start date."
        }
        (_, question_model::CourseTermFailureReason::UnknownIanaTimeZone) => {
            "Choose a valid IANA time zone such as America/Chicago."
        }
    };
    question_model::CourseTermValidationFailure {
        error: question_model::CourseTermFailureCode::CourseTermInvalid,
        field,
        reason,
        message: message.to_string(),
    }
}

/// Rejects unknown fields at every level by comparing the request to the
/// canonical wire form of the typed model, mirroring the workspace boundary.
pub(super) fn strict_assignment_request<T>(value: serde_json::Value) -> Result<T, ()>
where
    T: serde::de::DeserializeOwned + Serialize,
{
    let request = serde_json::from_value(value.clone()).map_err(|_| ())?;
    if serde_json::to_value(&request).map_err(|_| ())? == value {
        Ok(request)
    } else {
        Err(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn workspace_requests_are_closed_and_canonical() {
        let valid = serde_json::json!({"title": "Mastery"});
        assert!(
            strict_assignment_request::<question_model::CreateAssignmentDraftRequest>(valid)
                .is_ok()
        );

        let mut unknown = serde_json::json!({"title": "Mastery"});
        unknown["unexpected"] = serde_json::json!(true);
        assert!(
            strict_assignment_request::<question_model::CreateAssignmentDraftRequest>(unknown)
                .is_err()
        );

        let content = serde_json::json!({
            "title": "Mastery",
            "entries": [{
                "kind": "selectionGroup",
                "candidateQuestionIds": ["ABC-DEF1"],
                "position": 0,
                "drawCount": 1,
                "pointsPerItem": "1",
                "ordering": "candidateOrder"
            }]
        });
        assert!(
            strict_assignment_request::<question_model::ReplaceAssignmentContentRequest>(
                content.clone()
            )
            .is_ok()
        );
        let mut internal_identity = content;
        internal_identity["entries"][0]["algorithmVersion"] = serde_json::json!(1);
        assert!(
            strict_assignment_request::<question_model::ReplaceAssignmentContentRequest>(
                internal_identity
            )
            .is_err()
        );
    }
}

//! Fetch-Metadata-gated, audit-recorded Student-work delivery.

use axum::Json;
use axum::extract::{Path, RawQuery, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use learning_data_access::{
    CourseGradebookStore, GradebookOperationSelection, InspectStudentWorkRequest,
    InspectedStudentSubmissionV1, InspectedStudentWorkDetailV1, InspectedSubmissionEvidenceV1,
    NavigationReferenceStore, Store, StoreError, StudentWorkInspectionFocusTarget,
    StudentWorkInspectionReturnContext, StudentWorkInspectionStore,
};
use question_model::{
    ActivityTimestamp, AssignmentReference, CourseId, CourseMembershipReference, CourseReference,
    GradingOperationReference, InspectedStudentScoreFeedbackV1, RunReference, ScoringStatus,
    TeachingDisplayLabel, presentation::InspectedStudentResponseV1,
};
use serde::Serialize;

use super::super::policy::require_direct_instructor_course;
use super::super::projection::{error_response, store_error_response};
use super::super::routing::CourseRouteState;
use crate::auth::{auth_error_response, no_store, resolve_request_session};

const FETCH_SITE: &str = "sec-fetch-site";
const FETCH_MODE: &str = "sec-fetch-mode";
const FETCH_DEST: &str = "sec-fetch-dest";
const FETCH_USER: &str = "sec-fetch-user";

/// Returns one immutable, solution-free Student-work detail after writing its audits.
pub(in crate::course) async fn get_student_work<S>(
    State(state): State<CourseRouteState<S>>,
    Path((course, membership, assignment, run)): Path<(
        CourseId,
        CourseMembershipReference,
        AssignmentReference,
        RunReference,
    )>,
    headers: HeaderMap,
    RawQuery(raw_query): RawQuery,
) -> Response
where
    S: Store
        + learning_data_access::CourseRecordsAccessStore
        + CourseGradebookStore
        + NavigationReferenceStore
        + learning_data_access::SessionStore
        + StudentWorkInspectionStore
        + 'static,
{
    if !accepted_fetch_metadata(&headers) {
        inspection_refused("fetch_metadata");
        return inspection_unavailable();
    }
    let authenticated = match resolve_request_session(state.store.as_ref(), &headers).await {
        Ok(value) => value,
        Err(error) => return auth_error_response(error),
    };
    if require_direct_instructor_course(state.store.as_ref(), &authenticated, course)
        .await
        .is_err()
    {
        inspection_refused("authorization");
        return inspection_unavailable();
    }
    let course_reference = match state
        .store
        .course_reference(
            authenticated.tenant_context,
            authenticated.record.subject.user(),
            course,
        )
        .await
    {
        Ok(Some(value)) => value,
        Ok(None)
        | Err(StoreError::NotFound | StoreError::Forbidden | StoreError::TenantMismatch) => {
            inspection_refused("course_reference");
            return inspection_unavailable();
        }
        Err(error) => return store_error_response(error),
    };
    let operation = match inspection_operation_query(raw_query.as_deref()) {
        Ok(value) => value,
        Err(()) => {
            inspection_refused("operation_context");
            return inspection_unavailable();
        }
    };
    let return_context = match operation {
        Some(operation) => {
            let selection = match state
                .store
                .resolve_gradebook_operation(
                    authenticated.tenant_context,
                    authenticated.session_hash,
                    course,
                    operation,
                )
                .await
            {
                Ok(value) => value,
                Err(_) => {
                    inspection_refused("operation_context");
                    return inspection_unavailable();
                }
            };
            match grading_operation_return_context(
                course_reference,
                membership,
                assignment,
                operation,
                selection,
            ) {
                Some(value) => value,
                None => {
                    inspection_refused("operation_context");
                    return inspection_unavailable();
                }
            }
        }
        None => gradebook_return_context(course_reference, membership, assignment),
    };
    let request = InspectStudentWorkRequest {
        course: course_reference,
        membership,
        assignment,
        run,
        return_context,
    };
    match state
        .store
        .inspect_student_work(
            authenticated.tenant_context,
            authenticated.session_hash,
            request,
        )
        .await
    {
        Ok(detail) => no_store(Json(InspectedStudentWorkDetailView::from(detail)).into_response()),
        Err(StoreError::NotFound | StoreError::Forbidden | StoreError::TenantMismatch) => {
            inspection_refused("evidence");
            inspection_unavailable()
        }
        Err(error) => {
            inspection_refused("store");
            store_error_response(error)
        }
    }
}

fn gradebook_return_context(
    course: CourseReference,
    membership: CourseMembershipReference,
    assignment: AssignmentReference,
) -> StudentWorkInspectionReturnContext {
    StudentWorkInspectionReturnContext::Gradebook {
        course,
        membership,
        assignment,
        focus: StudentWorkInspectionFocusTarget::GradebookCell {
            membership,
            assignment,
        },
    }
}

fn inspection_operation_query(
    raw_query: Option<&str>,
) -> Result<Option<GradingOperationReference>, ()> {
    let Some(raw_query) = raw_query.filter(|value| !value.is_empty()) else {
        return Ok(None);
    };
    let Some(value) = raw_query.strip_prefix("operationRef=") else {
        return Err(());
    };
    if value.is_empty() || value.contains('&') {
        return Err(());
    }
    let operation = value.parse::<GradingOperationReference>().map_err(|_| ())?;
    if operation.to_string() != value {
        return Err(());
    }
    Ok(Some(operation))
}

fn grading_operation_return_context(
    course: CourseReference,
    membership: CourseMembershipReference,
    assignment: AssignmentReference,
    operation: GradingOperationReference,
    selection: GradebookOperationSelection,
) -> Option<StudentWorkInspectionReturnContext> {
    let matches_route = match selection {
        GradebookOperationSelection::SingleStudent {
            membership: selected_membership,
            assignment: selected_assignment,
        } => selected_membership == membership && selected_assignment == assignment,
        GradebookOperationSelection::Assignment {
            assignment: selected_assignment,
        } => selected_assignment == assignment,
    };
    matches_route.then_some(StudentWorkInspectionReturnContext::GradingOperation {
        course,
        membership,
        assignment,
        operation,
        focus: StudentWorkInspectionFocusTarget::GradingOperationControl {
            membership,
            assignment,
            operation,
        },
    })
}

/// Accepts the two browser request profiles permitted for protected
/// Gradebook reads: same-origin fetch/navigation and explicit top-level
/// user navigation.  Callers return a concealed response before any Student
/// record Store read when this table rejects the request.
pub(super) fn accepted_fetch_metadata(headers: &HeaderMap) -> bool {
    let Some(site) = exact_header(headers, FETCH_SITE) else {
        return false;
    };
    let Some(mode) = exact_header(headers, FETCH_MODE) else {
        return false;
    };
    let Some(destination) = exact_header(headers, FETCH_DEST) else {
        return false;
    };
    let user = optional_exact_header(headers, FETCH_USER);
    matches!(
        (site, mode, destination, user),
        ("same-origin", "cors" | "same-origin", "empty", Some(None))
            | (
                "same-origin",
                "navigate",
                "document",
                Some(None | Some("?1"))
            )
            | ("none", "navigate", "document", Some(Some("?1")))
    )
}

fn exact_header<'a>(headers: &'a HeaderMap, name: &str) -> Option<&'a str> {
    optional_exact_header(headers, name)?
}

fn optional_exact_header<'a>(headers: &'a HeaderMap, name: &str) -> Option<Option<&'a str>> {
    let mut values = headers.get_all(name).iter();
    let first = values.next();
    if values.next().is_some() {
        return None;
    }
    match first {
        Some(value) => value.to_str().ok().map(Some),
        None => Some(None),
    }
}

fn inspection_refused(stage: &'static str) {
    tracing::warn!(event = "student_work_inspection_refused", stage);
}

fn inspection_unavailable() -> Response {
    error_response(StatusCode::NOT_FOUND, "student work not found")
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct InspectedStudentWorkDetailView {
    course: CourseReference,
    membership: CourseMembershipReference,
    assignment: AssignmentReference,
    run: RunReference,
    student_display_label: TeachingDisplayLabel,
    assignment_title: String,
    submissions: Vec<InspectedStudentSubmissionView>,
    return_context: StudentWorkReturnContextView,
}

impl From<InspectedStudentWorkDetailV1> for InspectedStudentWorkDetailView {
    fn from(detail: InspectedStudentWorkDetailV1) -> Self {
        Self {
            course: detail.course,
            membership: detail.membership,
            assignment: detail.assignment,
            run: detail.run,
            student_display_label: detail.student_display_label,
            assignment_title: detail.assignment_title,
            submissions: detail
                .submissions
                .into_iter()
                .map(InspectedStudentSubmissionView::from)
                .collect(),
            return_context: detail.return_context.into(),
        }
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct InspectedStudentSubmissionView {
    submitted_at: ActivityTimestamp,
    evidence: InspectedSubmissionEvidenceView,
    scoring_generation: u64,
    feedback: InspectedStudentScoreFeedbackV1,
    response: InspectedStudentResponseV1,
    scoring_status: ScoringStatus,
}

impl From<InspectedStudentSubmissionV1> for InspectedStudentSubmissionView {
    fn from(submission: InspectedStudentSubmissionV1) -> Self {
        Self {
            submitted_at: submission.submitted_at,
            evidence: submission.evidence.into(),
            scoring_generation: submission.scoring_generation.value(),
            feedback: submission.feedback,
            response: submission.response,
            scoring_status: submission.scoring_status,
        }
    }
}

#[derive(Serialize)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
enum InspectedSubmissionEvidenceView {
    IssuedPresentation {
        presentation: Box<learning_data_access::ReceiptPresentationSnapshot>,
        issued_presentation_digest: String,
    },
    PresentationNotApplicable,
}

impl From<InspectedSubmissionEvidenceV1> for InspectedSubmissionEvidenceView {
    fn from(evidence: InspectedSubmissionEvidenceV1) -> Self {
        match evidence {
            InspectedSubmissionEvidenceV1::IssuedPresentation {
                presentation,
                issued_presentation_digest,
            } => Self::IssuedPresentation {
                presentation,
                issued_presentation_digest: issued_presentation_digest.to_hex(),
            },
            InspectedSubmissionEvidenceV1::PresentationNotApplicable => {
                Self::PresentationNotApplicable
            }
        }
    }
}

#[derive(Serialize)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
enum StudentWorkReturnContextView {
    Gradebook {
        course: CourseReference,
        membership: CourseMembershipReference,
        assignment: AssignmentReference,
        focus: StudentWorkFocusTargetView,
    },
    GradingOperation {
        course: CourseReference,
        membership: CourseMembershipReference,
        assignment: AssignmentReference,
        operation: question_model::GradingOperationReference,
        focus: StudentWorkFocusTargetView,
    },
}

impl From<StudentWorkInspectionReturnContext> for StudentWorkReturnContextView {
    fn from(context: StudentWorkInspectionReturnContext) -> Self {
        match context {
            StudentWorkInspectionReturnContext::Gradebook {
                course,
                membership,
                assignment,
                focus,
            } => Self::Gradebook {
                course,
                membership,
                assignment,
                focus: focus.into(),
            },
            StudentWorkInspectionReturnContext::GradingOperation {
                course,
                membership,
                assignment,
                operation,
                focus,
            } => Self::GradingOperation {
                course,
                membership,
                assignment,
                operation,
                focus: focus.into(),
            },
        }
    }
}

#[derive(Serialize)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
enum StudentWorkFocusTargetView {
    GradebookCell {
        membership: CourseMembershipReference,
        assignment: AssignmentReference,
    },
    GradingOperationControl {
        membership: CourseMembershipReference,
        assignment: AssignmentReference,
        operation: question_model::GradingOperationReference,
    },
}

impl From<StudentWorkInspectionFocusTarget> for StudentWorkFocusTargetView {
    fn from(focus: StudentWorkInspectionFocusTarget) -> Self {
        match focus {
            StudentWorkInspectionFocusTarget::GradebookCell {
                membership,
                assignment,
            } => Self::GradebookCell {
                membership,
                assignment,
            },
            StudentWorkInspectionFocusTarget::GradingOperationControl {
                membership,
                assignment,
                operation,
            } => Self::GradingOperationControl {
                membership,
                assignment,
                operation,
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::{HeaderName, HeaderValue};

    fn headers(values: &[(&str, &str)]) -> HeaderMap {
        let mut headers = HeaderMap::new();
        for (name, value) in values {
            headers.insert(
                HeaderName::from_bytes(name.as_bytes()).expect("valid header name"),
                HeaderValue::from_str(value).expect("valid header value"),
            );
        }
        headers
    }

    #[test]
    fn fetch_metadata_accepts_same_origin_fetch_and_explicit_navigation() {
        assert!(accepted_fetch_metadata(&headers(&[
            (FETCH_SITE, "same-origin"),
            (FETCH_MODE, "cors"),
            (FETCH_DEST, "empty"),
        ])));
        assert!(accepted_fetch_metadata(&headers(&[
            (FETCH_SITE, "none"),
            (FETCH_MODE, "navigate"),
            (FETCH_DEST, "document"),
            (FETCH_USER, "?1"),
        ])));
    }

    #[test]
    fn fetch_metadata_rejects_cross_site_and_non_user_navigation() {
        assert!(!accepted_fetch_metadata(&headers(&[
            (FETCH_SITE, "cross-site"),
            (FETCH_MODE, "cors"),
            (FETCH_DEST, "empty"),
        ])));
        assert!(!accepted_fetch_metadata(&headers(&[
            (FETCH_SITE, "none"),
            (FETCH_MODE, "navigate"),
            (FETCH_DEST, "document"),
        ])));
    }

    #[test]
    fn inspection_operation_query_accepts_only_one_canonical_operation_reference() {
        assert_eq!(inspection_operation_query(None), Ok(None));
        assert_eq!(inspection_operation_query(Some("")), Ok(None));
        assert_eq!(
            inspection_operation_query(Some("operationRef=GO-17")),
            Ok(Some(
                GradingOperationReference::new(17).expect("positive operation reference")
            ))
        );
        for query in [
            "operationRef=",
            "operationRef=GO-017",
            "operationRef=GO%2D17",
            "operationRef=GO-17&operationRef=GO-18",
            "operationRef=GO-17&",
            "unknown=GO-17",
            "operationRef=GO-17&unknown=1",
        ] {
            assert_eq!(inspection_operation_query(Some(query)), Err(()));
        }
    }

    #[test]
    fn grading_operation_context_requires_exact_resolved_scope() {
        let course = CourseReference::new(1).expect("positive course reference");
        let membership = CourseMembershipReference::new(2).expect("positive membership reference");
        let assignment = AssignmentReference::new(3).expect("positive assignment reference");
        let operation = GradingOperationReference::new(4).expect("positive operation reference");
        let expected = StudentWorkInspectionReturnContext::GradingOperation {
            course,
            membership,
            assignment,
            operation,
            focus: StudentWorkInspectionFocusTarget::GradingOperationControl {
                membership,
                assignment,
                operation,
            },
        };

        assert_eq!(
            grading_operation_return_context(
                course,
                membership,
                assignment,
                operation,
                GradebookOperationSelection::SingleStudent {
                    membership,
                    assignment,
                },
            ),
            Some(expected)
        );
        assert_eq!(
            grading_operation_return_context(
                course,
                membership,
                assignment,
                operation,
                GradebookOperationSelection::Assignment { assignment },
            ),
            Some(expected)
        );
        assert_eq!(
            grading_operation_return_context(
                course,
                membership,
                assignment,
                operation,
                GradebookOperationSelection::SingleStudent {
                    membership: CourseMembershipReference::new(5)
                        .expect("positive membership reference"),
                    assignment,
                },
            ),
            None
        );
        assert_eq!(
            grading_operation_return_context(
                course,
                membership,
                assignment,
                operation,
                GradebookOperationSelection::SingleStudent {
                    membership,
                    assignment: AssignmentReference::new(6).expect("positive assignment reference"),
                },
            ),
            None
        );
        assert_eq!(
            grading_operation_return_context(
                course,
                membership,
                assignment,
                operation,
                GradebookOperationSelection::Assignment {
                    assignment: AssignmentReference::new(6).expect("positive assignment reference"),
                },
            ),
            None
        );
    }
}

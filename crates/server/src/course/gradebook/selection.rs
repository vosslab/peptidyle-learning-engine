//! Explicit, answer-free Student and submitted-run choices for the Gradebook.

use axum::Json;
use axum::extract::{Path, RawQuery, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use learning_data_access::{
    AssignmentInspectionChoice, AssignmentRunSelectionBasis, CourseGradebookStore, Cursor,
    GradebookFilterRequest, GradebookSelectionRequest, GradebookSelectionResult, PageRequest,
    PageSize, Store, StoreError, SubmittedRunChoice, SubmittedRunChoicesPage,
    SubmittedRunChoicesRequest,
};
use question_model::{
    ActivityTimestamp, AssignmentReference, CourseId, CourseMembershipReference,
    GradingOperationReference, RunReference,
};
use serde::Serialize;

use super::super::policy::require_direct_instructor_course;
use super::super::projection::{error_response, store_error_response};
use super::super::routing::{CourseRouteState, DEFAULT_PAGE_SIZE};
use super::inspection::accepted_fetch_metadata;
use crate::auth::{auth_error_response, no_store, resolve_request_session};

/// Returns one explicit named-Student selection before opening Student work.
pub(in crate::course) async fn get_gradebook_selection<S>(
    State(state): State<CourseRouteState<S>>,
    Path(course): Path<CourseId>,
    headers: HeaderMap,
    RawQuery(raw_query): RawQuery,
) -> Response
where
    S: Store
        + learning_data_access::CourseRecordsAccessStore
        + CourseGradebookStore
        + learning_data_access::SessionStore
        + 'static,
{
    if !accepted_fetch_metadata(&headers) {
        return gradebook_selection_unavailable();
    }
    let authenticated = match resolve_request_session(state.store.as_ref(), &headers).await {
        Ok(value) => value,
        Err(error) => return auth_error_response(error),
    };
    if require_direct_instructor_course(state.store.as_ref(), &authenticated, course)
        .await
        .is_err()
    {
        return gradebook_selection_unavailable();
    }
    let request = match gradebook_selection_request(raw_query.as_deref()) {
        Ok(value) => value,
        Err(message) => return error_response(StatusCode::BAD_REQUEST, message),
    };
    if let GradebookFilterRequest::Operation(operation) = request.filter
        && let Err(error) = state
            .store
            .resolve_gradebook_operation(
                authenticated.tenant_context,
                authenticated.session_hash,
                course,
                operation,
            )
            .await
    {
        return gradebook_selection_store_error(error);
    }
    match state
        .store
        .gradebook_selection(
            authenticated.tenant_context,
            authenticated.session_hash,
            course,
            request,
        )
        .await
    {
        Ok(result) => no_store(Json(GradebookSelectionView::from(result)).into_response()),
        Err(error) => gradebook_selection_store_error(error),
    }
}

/// Returns bounded public submitted-run locators after a named Student is selected.
pub(in crate::course) async fn get_submitted_run_choices<S>(
    State(state): State<CourseRouteState<S>>,
    Path((course, membership, assignment)): Path<(
        CourseId,
        CourseMembershipReference,
        AssignmentReference,
    )>,
    headers: HeaderMap,
    RawQuery(raw_query): RawQuery,
) -> Response
where
    S: Store
        + learning_data_access::CourseRecordsAccessStore
        + CourseGradebookStore
        + learning_data_access::SessionStore
        + 'static,
{
    if !accepted_fetch_metadata(&headers) {
        return gradebook_selection_unavailable();
    }
    let authenticated = match resolve_request_session(state.store.as_ref(), &headers).await {
        Ok(value) => value,
        Err(error) => return auth_error_response(error),
    };
    if require_direct_instructor_course(state.store.as_ref(), &authenticated, course)
        .await
        .is_err()
    {
        return gradebook_selection_unavailable();
    }
    let (operation, page) = match submitted_run_choices_request(raw_query.as_deref()) {
        Ok(value) => value,
        Err(message) => return error_response(StatusCode::BAD_REQUEST, message),
    };
    if let Some(operation) = operation
        && let Err(error) = state
            .store
            .resolve_gradebook_operation(
                authenticated.tenant_context,
                authenticated.session_hash,
                course,
                operation,
            )
            .await
    {
        return gradebook_selection_store_error(error);
    }
    let request = SubmittedRunChoicesRequest {
        membership,
        assignment,
        operation,
        page,
    };
    match state
        .store
        .submitted_run_choices(
            authenticated.tenant_context,
            authenticated.session_hash,
            course,
            request,
        )
        .await
    {
        Ok(page) => no_store(Json(SubmittedRunChoicesView::from(page)).into_response()),
        Err(error) => gradebook_selection_store_error(error),
    }
}

fn gradebook_selection_request(
    raw_query: Option<&str>,
) -> Result<GradebookSelectionRequest, &'static str> {
    let (filter, page) = gradebook_filter_page_request(raw_query, false)?;
    match filter {
        GradebookFilterRequest::Assignment(_) | GradebookFilterRequest::Operation(_) => {
            Ok(GradebookSelectionRequest { filter, page })
        }
        GradebookFilterRequest::All | GradebookFilterRequest::Student(_) => {
            Err("gradebook selection requires assignmentRef or operationRef")
        }
    }
}

fn submitted_run_choices_request(
    raw_query: Option<&str>,
) -> Result<(Option<GradingOperationReference>, PageRequest), &'static str> {
    let mut cursor = None;
    let mut page_size = None;
    let mut operation = None;
    for (key, value) in url::form_urlencoded::parse(raw_query.unwrap_or_default().as_bytes()) {
        let slot = match key.as_ref() {
            "cursor" => &mut cursor,
            "pageSize" => &mut page_size,
            "operationRef" => &mut operation,
            _ => return Err("submitted-run query is invalid"),
        };
        if slot.is_some() {
            return Err("submitted-run query is invalid");
        }
        *slot = Some(value.into_owned());
    }
    let operation = operation
        .map(|value| value.parse().map_err(|_| "operationRef is invalid"))
        .transpose()?;
    let page = bounded_page_request(cursor, page_size)?;
    Ok((operation, page))
}

fn gradebook_filter_page_request(
    raw_query: Option<&str>,
    allow_unfiltered: bool,
) -> Result<(GradebookFilterRequest, PageRequest), &'static str> {
    let mut cursor = None;
    let mut page_size = None;
    let mut assignment = None;
    let mut membership = None;
    let mut operation = None;
    for (key, value) in url::form_urlencoded::parse(raw_query.unwrap_or_default().as_bytes()) {
        let slot = match key.as_ref() {
            "cursor" => &mut cursor,
            "pageSize" => &mut page_size,
            "assignmentRef" => &mut assignment,
            "membershipRef" => &mut membership,
            "operationRef" => &mut operation,
            _ => return Err("gradebook selection query is invalid"),
        };
        if slot.is_some() {
            return Err("gradebook selection query is invalid");
        }
        *slot = Some(value.into_owned());
    }
    let filter = match (assignment, membership, operation) {
        (None, None, None) if allow_unfiltered => GradebookFilterRequest::All,
        (Some(value), None, None) => GradebookFilterRequest::Assignment(
            value.parse().map_err(|_| "assignmentRef is invalid")?,
        ),
        (None, Some(value), None) => {
            GradebookFilterRequest::Student(value.parse().map_err(|_| "membershipRef is invalid")?)
        }
        (None, None, Some(value)) => {
            GradebookFilterRequest::Operation(value.parse().map_err(|_| "operationRef is invalid")?)
        }
        _ => return Err("gradebook selection accepts one filter"),
    };
    let page = bounded_page_request(cursor, page_size)?;
    Ok((filter, page))
}

fn bounded_page_request(
    cursor: Option<String>,
    page_size: Option<String>,
) -> Result<PageRequest, &'static str> {
    let size = match page_size {
        Some(value) => value
            .parse::<u16>()
            .ok()
            .and_then(|value| PageSize::new(value).ok())
            .ok_or("pageSize must be between 1 and 100")?,
        None => PageSize::new(DEFAULT_PAGE_SIZE).expect("default page size is bounded"),
    };
    match cursor {
        Some(value) => Ok(PageRequest::after(
            Cursor::parse(value).map_err(|_| "cursor must not be empty")?,
            size,
        )),
        None => Ok(PageRequest::first(size)),
    }
}

fn gradebook_selection_store_error(error: StoreError) -> Response {
    match error {
        StoreError::NotFound
        | StoreError::Forbidden
        | StoreError::OwnershipMismatch
        | StoreError::Unavailable(_) => gradebook_selection_unavailable(),
        error => store_error_response(error),
    }
}

fn gradebook_selection_unavailable() -> Response {
    error_response(StatusCode::NOT_FOUND, "gradebook selection not found")
}

#[derive(Serialize)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
enum GradebookSelectionView {
    SingleStudent {
        membership: CourseMembershipReference,
        assignment: AssignmentReference,
        inspection_choice: AssignmentInspectionChoiceView,
    },
    StudentSelection {
        rows: Vec<StudentSelectionRowView>,
        #[serde(skip_serializing_if = "Option::is_none")]
        next_cursor: Option<Cursor>,
    },
}

impl From<GradebookSelectionResult> for GradebookSelectionView {
    fn from(value: GradebookSelectionResult) -> Self {
        match value {
            GradebookSelectionResult::SingleStudent {
                membership,
                assignment,
                inspection_choice,
            } => Self::SingleStudent {
                membership,
                assignment,
                inspection_choice: inspection_choice.into(),
            },
            GradebookSelectionResult::StudentSelection { rows, next_cursor } => {
                Self::StudentSelection {
                    rows: rows
                        .into_iter()
                        .map(|row| StudentSelectionRowView {
                            membership: row.membership,
                            display_label: row.display_label,
                            assignment: row.assignment,
                            inspection_choice: row.inspection_choice.into(),
                        })
                        .collect(),
                    next_cursor,
                }
            }
        }
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct StudentSelectionRowView {
    membership: CourseMembershipReference,
    display_label: String,
    assignment: AssignmentReference,
    inspection_choice: AssignmentInspectionChoiceView,
}

#[derive(Serialize)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
enum AssignmentInspectionChoiceView {
    #[serde(rename = "selectedRun")]
    Selected {
        basis: AssignmentRunSelectionBasisView,
        run: RunReference,
        submitted_at: ActivityTimestamp,
    },
    #[serde(rename = "chooseRun")]
    Choose { completed_run_count: u32 },
    #[serde(rename = "noSubmittedRun")]
    None,
}

impl From<AssignmentInspectionChoice> for AssignmentInspectionChoiceView {
    fn from(value: AssignmentInspectionChoice) -> Self {
        match value {
            AssignmentInspectionChoice::SelectedRun {
                basis,
                run,
                submitted_at,
            } => Self::Selected {
                basis: basis.into(),
                run,
                submitted_at,
            },
            AssignmentInspectionChoice::ChooseRun {
                completed_run_count,
            } => Self::Choose {
                completed_run_count,
            },
            AssignmentInspectionChoice::NoSubmittedRun => Self::None,
        }
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
enum AssignmentRunSelectionBasisView {
    First,
    Latest,
    Highest,
    InstructorSelected,
}

impl From<AssignmentRunSelectionBasis> for AssignmentRunSelectionBasisView {
    fn from(value: AssignmentRunSelectionBasis) -> Self {
        match value {
            AssignmentRunSelectionBasis::First => Self::First,
            AssignmentRunSelectionBasis::Latest => Self::Latest,
            AssignmentRunSelectionBasis::Highest => Self::Highest,
            AssignmentRunSelectionBasis::InstructorSelected => Self::InstructorSelected,
        }
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct SubmittedRunChoicesView {
    roster_revision: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    next_cursor: Option<Cursor>,
    rows: Vec<SubmittedRunChoiceView>,
}

impl From<SubmittedRunChoicesPage> for SubmittedRunChoicesView {
    fn from(page: SubmittedRunChoicesPage) -> Self {
        Self {
            roster_revision: page.roster_revision.value(),
            next_cursor: page.next_cursor,
            rows: page
                .rows
                .into_iter()
                .map(SubmittedRunChoiceView::from)
                .collect(),
        }
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct SubmittedRunChoiceView {
    run: RunReference,
    submitted_at: ActivityTimestamp,
    score_selected: bool,
}

impl From<SubmittedRunChoice> for SubmittedRunChoiceView {
    fn from(choice: SubmittedRunChoice) -> Self {
        Self {
            run: choice.run,
            submitted_at: choice.submitted_at,
            score_selected: choice.score_selected,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn selection_query_requires_one_assignment_bearing_scope() {
        assert!(gradebook_selection_request(Some("assignmentRef=A-7&pageSize=25")).is_ok());
        assert!(gradebook_selection_request(Some("operationRef=GO-7")).is_ok());
        assert!(gradebook_selection_request(None).is_err());
        assert!(gradebook_selection_request(Some("membershipRef=M-7")).is_err());
    }

    #[test]
    fn run_chooser_query_is_bounded_and_closed() {
        assert!(submitted_run_choices_request(Some("operationRef=GO-7&pageSize=1")).is_ok());
        assert!(submitted_run_choices_request(Some("pageSize=101")).is_err());
        assert!(submitted_run_choices_request(Some("membershipRef=M-7")).is_err());
    }
}

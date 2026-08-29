//! Canonical roster-first calculated Gradebook delivery.

use axum::Json;
use axum::extract::{Path, RawQuery, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use learning_data_access::{
    AssignmentInspectionChoice, AssignmentRunSelectionBasis, CalculatedAssignmentCell,
    CalculatedAssignmentCellAvailability, CalculatedGradebookPage, CalculatedGradebookRequest,
    CalculatedGradebookResult, CalculatedGradebookRow, CourseGradebookStore, Cursor,
    GradebookFilter, GradebookFilterRequest, GradebookOperationSelection, GradebookReloadReason,
    PageRequest, PageSize, Store,
};
use question_model::{
    ActivityTimestamp, AssignmentReference, CourseGradeMode, CourseGradeRoundingRule, CourseId,
    CourseMembershipReference, GradeCategoryId, RunReference, ScoringStatus,
};
use serde::Serialize;

use super::super::policy::require_direct_instructor_course;
use super::super::projection::{error_response, store_error_response};
use super::super::routing::{CourseRouteState, DEFAULT_PAGE_SIZE};
use super::inspection::accepted_fetch_metadata;
use crate::auth::{auth_error_response, no_store, resolve_request_session};

/// Returns one server-calculated Gradebook page or an explicit structural reload.
pub(in crate::course) async fn get_calculated_gradebook<S>(
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
        return error_response(StatusCode::NOT_FOUND, "gradebook not found");
    }
    let authenticated = match resolve_request_session(state.store.as_ref(), &headers).await {
        Ok(value) => value,
        Err(error) => return auth_error_response(error),
    };
    if require_direct_instructor_course(state.store.as_ref(), &authenticated, course)
        .await
        .is_err()
    {
        return gradebook_unavailable();
    }
    let request = match gradebook_request(raw_query.as_deref()) {
        Ok(value) => value,
        Err(message) => return error_response(StatusCode::BAD_REQUEST, message),
    };
    let filter = match normalize_gradebook_filter(
        state.store.as_ref(),
        authenticated.tenant_context,
        authenticated.session_hash,
        course,
        request.filter,
    )
    .await
    {
        Ok(value) => value,
        Err(error) => return gradebook_store_error(error),
    };
    match state
        .store
        .calculated_gradebook_page(
            authenticated.tenant_context,
            authenticated.session_hash,
            course,
            CalculatedGradebookRequest {
                filter,
                page: request.page,
            },
        )
        .await
    {
        Ok(result) => no_store(Json(CalculatedGradebookView::from(result)).into_response()),
        Err(error) => gradebook_store_error(error),
    }
}

async fn normalize_gradebook_filter<S>(
    store: &S,
    context: learning_data_access::TenantContext,
    session: learning_data_access::SessionTokenHash,
    course: CourseId,
    filter: GradebookFilterRequest,
) -> Result<GradebookFilter, learning_data_access::StoreError>
where
    S: CourseGradebookStore,
{
    match filter {
        GradebookFilterRequest::All => Ok(GradebookFilter::All),
        GradebookFilterRequest::Assignment(assignment) => {
            Ok(GradebookFilter::Assignment(assignment))
        }
        GradebookFilterRequest::Student(membership) => Ok(GradebookFilter::Student(membership)),
        GradebookFilterRequest::Operation(operation) => {
            match store
                .resolve_gradebook_operation(context, session, course, operation)
                .await?
            {
                GradebookOperationSelection::Assignment { assignment } => {
                    Ok(GradebookFilter::Assignment(assignment))
                }
                GradebookOperationSelection::SingleStudent { membership, .. } => {
                    Ok(GradebookFilter::Student(membership))
                }
            }
        }
    }
}

struct GradebookRequest {
    filter: GradebookFilterRequest,
    page: PageRequest,
}

fn gradebook_request(raw_query: Option<&str>) -> Result<GradebookRequest, &'static str> {
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
            _ => return Err("gradebook query is invalid"),
        };
        if slot.is_some() {
            return Err("gradebook query is invalid");
        }
        *slot = Some(value.into_owned());
    }
    let filter = match (assignment, membership, operation) {
        (None, None, None) => GradebookFilterRequest::All,
        (Some(value), None, None) => GradebookFilterRequest::Assignment(
            value.parse().map_err(|_| "assignmentRef is invalid")?,
        ),
        (None, Some(value), None) => {
            GradebookFilterRequest::Student(value.parse().map_err(|_| "membershipRef is invalid")?)
        }
        (None, None, Some(value)) => {
            GradebookFilterRequest::Operation(value.parse().map_err(|_| "operationRef is invalid")?)
        }
        _ => return Err("gradebook accepts one filter"),
    };
    let size = match page_size {
        Some(value) => value
            .parse::<u16>()
            .ok()
            .and_then(|value| PageSize::new(value).ok())
            .ok_or("pageSize must be between 1 and 100")?,
        None => PageSize::new(DEFAULT_PAGE_SIZE).expect("default page size is bounded"),
    };
    let page = match cursor {
        Some(value) => PageRequest::after(
            Cursor::parse(value).map_err(|_| "cursor must not be empty")?,
            size,
        ),
        None => PageRequest::first(size),
    };
    Ok(GradebookRequest { filter, page })
}

fn gradebook_store_error(error: learning_data_access::StoreError) -> Response {
    match error {
        learning_data_access::StoreError::NotFound
        | learning_data_access::StoreError::Forbidden
        | learning_data_access::StoreError::TenantMismatch
        | learning_data_access::StoreError::Unavailable(_) => gradebook_unavailable(),
        error => store_error_response(error),
    }
}

fn gradebook_unavailable() -> Response {
    error_response(StatusCode::NOT_FOUND, "gradebook not found")
}

#[derive(Serialize)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
enum CalculatedGradebookView {
    Page {
        scheme_revision: u64,
        roster_revision: u64,
        mode: CourseGradeMode,
        rounding: CourseGradeRoundingRule,
        observation_time: ActivityTimestamp,
        scoring_witnesses: Vec<AssignmentScoringWitnessView>,
        #[serde(skip_serializing_if = "Option::is_none")]
        next_cursor: Option<Cursor>,
        rows: Vec<CalculatedGradebookRowView>,
    },
    ReloadRequired {
        reason: GradebookReloadReasonView,
    },
}

impl From<CalculatedGradebookResult> for CalculatedGradebookView {
    fn from(value: CalculatedGradebookResult) -> Self {
        match value {
            CalculatedGradebookResult::Page(page) => Self::from(page),
            CalculatedGradebookResult::ReloadRequired { reason } => Self::ReloadRequired {
                reason: reason.into(),
            },
        }
    }
}

impl From<CalculatedGradebookPage> for CalculatedGradebookView {
    fn from(page: CalculatedGradebookPage) -> Self {
        Self::Page {
            scheme_revision: page.scheme_revision.value(),
            roster_revision: page.roster_revision.value(),
            mode: page.mode,
            rounding: page.rounding,
            observation_time: page.observation_time,
            scoring_witnesses: page
                .scoring_witnesses
                .into_iter()
                .map(|witness| AssignmentScoringWitnessView {
                    assignment: witness.assignment,
                    generation: witness.generation.value(),
                    status: witness.status,
                })
                .collect(),
            next_cursor: page.next_cursor,
            rows: page
                .rows
                .into_iter()
                .map(CalculatedGradebookRowView::from)
                .collect(),
        }
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct AssignmentScoringWitnessView {
    assignment: AssignmentReference,
    generation: u64,
    status: ScoringStatus,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct CalculatedGradebookRowView {
    membership: CourseMembershipReference,
    display_label: String,
    outcome: CalculatedCourseGradeOutcomeView,
    assignment_cells: Vec<CalculatedAssignmentCellView>,
}

impl From<CalculatedGradebookRow> for CalculatedGradebookRowView {
    fn from(row: CalculatedGradebookRow) -> Self {
        let outcome =
            CalculatedCourseGradeOutcomeView::from_parts(row.outcome, row.dropped_assignments);
        Self {
            membership: row.membership,
            display_label: row.display_label,
            outcome,
            assignment_cells: row
                .assignment_cells
                .into_iter()
                .map(CalculatedAssignmentCellView::from)
                .collect(),
        }
    }
}

#[derive(Serialize)]
#[serde(
    tag = "status",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
enum CalculatedCourseGradeOutcomeView {
    Available {
        score: f64,
        letter: Option<String>,
        dropped_assignments: Vec<AssignmentReference>,
        total_earned: Option<f64>,
        total_possible: Option<f64>,
    },
    Unavailable {
        reason: CalculatedCourseGradeUnavailableReasonView,
    },
}

impl CalculatedCourseGradeOutcomeView {
    fn from_parts(
        outcome: domain::course_grade::CourseGradeOutcome,
        dropped_assignments: Vec<AssignmentReference>,
    ) -> Self {
        match outcome.unavailable_reason {
            Some(reason) => Self::Unavailable {
                reason: reason.into(),
            },
            None => Self::Available {
                score: outcome.rounded_score.expect("available grade has score"),
                letter: outcome.letter,
                dropped_assignments,
                total_earned: outcome.total_earned,
                total_possible: outcome.total_possible,
            },
        }
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
enum CalculatedCourseGradeUnavailableReasonView {
    NoIncludedAssignments,
    Recalculating,
    Failed,
    EmptyAfterDrop,
    ZeroPossiblePoints,
}

impl From<domain::course_grade::CourseGradeUnavailableReason>
    for CalculatedCourseGradeUnavailableReasonView
{
    fn from(value: domain::course_grade::CourseGradeUnavailableReason) -> Self {
        use domain::course_grade::CourseGradeUnavailableReason;
        match value {
            CourseGradeUnavailableReason::NoIncludedAssignments => Self::NoIncludedAssignments,
            CourseGradeUnavailableReason::Recalculating => Self::Recalculating,
            CourseGradeUnavailableReason::Failed => Self::Failed,
            CourseGradeUnavailableReason::EmptyAfterDrop => Self::EmptyAfterDrop,
            CourseGradeUnavailableReason::ZeroPossiblePoints => Self::ZeroPossiblePoints,
        }
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct CalculatedAssignmentCellView {
    assignment: AssignmentReference,
    title: String,
    included: bool,
    category: Option<GradeCategoryId>,
    availability: CalculatedAssignmentCellAvailabilityView,
    selected_score: Option<f64>,
    scoring_status: ScoringStatus,
    inspection_choice: AssignmentInspectionChoiceView,
}

impl From<CalculatedAssignmentCell> for CalculatedAssignmentCellView {
    fn from(cell: CalculatedAssignmentCell) -> Self {
        Self {
            assignment: cell.assignment,
            title: cell.title,
            included: cell.included,
            category: cell.category,
            availability: cell.availability.into(),
            selected_score: cell.selected_score,
            scoring_status: cell.scoring_status,
            inspection_choice: cell.inspection_choice.into(),
        }
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
enum CalculatedAssignmentCellAvailabilityView {
    Available,
    Unavailable,
}

impl From<CalculatedAssignmentCellAvailability> for CalculatedAssignmentCellAvailabilityView {
    fn from(value: CalculatedAssignmentCellAvailability) -> Self {
        match value {
            CalculatedAssignmentCellAvailability::Available => Self::Available,
            CalculatedAssignmentCellAvailability::Unavailable => Self::Unavailable,
        }
    }
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
enum GradebookReloadReasonView {
    #[serde(rename = "schemeChanged")]
    Scheme,
    #[serde(rename = "rosterChanged")]
    Roster,
    #[serde(rename = "filterChanged")]
    Filter,
}

impl From<GradebookReloadReason> for GradebookReloadReasonView {
    fn from(value: GradebookReloadReason) -> Self {
        match value {
            GradebookReloadReason::SchemeChanged => Self::Scheme,
            GradebookReloadReason::RosterChanged => Self::Roster,
            GradebookReloadReason::FilterChanged => Self::Filter,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn query_accepts_one_typed_filter_and_bounded_page() {
        let request = gradebook_request(Some("assignmentRef=A-7&pageSize=25"))
            .expect("valid gradebook query");
        assert_eq!(
            request.filter,
            GradebookFilterRequest::Assignment(AssignmentReference::new(7).expect("reference"))
        );
        assert_eq!(request.page.size.get(), 25);
    }

    #[test]
    fn query_rejects_ambiguous_or_duplicate_filters() {
        for query in [
            "assignmentRef=A-7&membershipRef=M-8",
            "assignmentRef=A-7&assignmentRef=A-8",
            "operationRef=GO-1&assignmentRef=A-7",
        ] {
            assert!(gradebook_request(Some(query)).is_err(), "{query}");
        }
    }
}

//! Instructor-only calculated Gradebook, inspection, configuration, and export.

mod calculated;
mod inspection;
mod selection;

pub(super) use calculated::get_calculated_gradebook;
pub(super) use inspection::get_student_work;
pub(super) use selection::{get_gradebook_selection, get_submitted_run_choices};

use axum::Json;
use axum::body::to_bytes;
use axum::extract::{Path, Request, State};
use axum::http::header::{CONTENT_DISPOSITION, CONTENT_TYPE, ETAG, IF_MATCH};
use axum::http::{HeaderMap, HeaderValue, StatusCode};
use axum::response::{IntoResponse, Response};
use domain::course_grade::{CourseGradeOutcome, CourseGradeUnavailableReason};
use learning_data_access::{
    AuthenticationEmail, CourseGradeAssignmentMembership, CourseGradeSchemeRevision,
    CourseGradebookStore, CourseRosterId, StoreError, UpdateCourseGradeScheme,
};
use question_model::{
    CourseGradeAssignmentSetting, CourseGradeAssignmentView, CourseGradeOutcomeView,
    CourseGradeSchemeUpdateView, CourseGradeSchemeView, CourseGradeUnavailableReasonView,
    CourseGradebookTotalViewRow, CourseGradebookTotalsView, CourseId,
};

use super::policy::require_course_access;
use super::projection::{error_response, store_error_response};
use super::routing::CourseRouteState;
use crate::auth::{auth_error_response, no_store, resolve_request_session};

const MAX_GRADE_SCHEME_JSON_BYTES: usize = 64 * 1_024;
const MAX_EMPTY_EXPORT_BODY_BYTES: usize = 64;
const EXPORT_ID_HEADER: &str = "x-ple-course-grade-export-id";

/// Returns the instructor's editable scheme and current assignment settings.
pub(super) async fn get_scheme<S>(
    State(state): State<CourseRouteState<S>>,
    Path(course): Path<CourseId>,
    headers: HeaderMap,
) -> Response
where
    S: learning_data_access::Store
        + learning_data_access::CourseRecordsAccessStore
        + CourseGradebookStore
        + learning_data_access::SessionStore
        + 'static,
{
    let authenticated = match resolve_request_session(state.store.as_ref(), &headers).await {
        Ok(value) => value,
        Err(error) => return auth_error_response(error),
    };
    if let Err(response) =
        require_course_access(state.store.as_ref(), &authenticated, course, true).await
    {
        return response.into_response();
    }
    match state
        .store
        .course_grade_scheme(
            authenticated.tenant_context,
            authenticated.record.token_hash,
            course,
        )
        .await
    {
        Ok(record) => scheme_response(StatusCode::OK, record),
        Err(error) => store_error_response(error),
    }
}

/// Replaces the whole course-grade scheme with a strong revision precondition.
pub(super) async fn put_scheme<S>(
    State(state): State<CourseRouteState<S>>,
    Path(course): Path<CourseId>,
    request: Request,
) -> Response
where
    S: learning_data_access::Store
        + learning_data_access::CourseRecordsAccessStore
        + CourseGradebookStore
        + learning_data_access::SessionStore
        + 'static,
{
    let authenticated = match resolve_request_session(state.store.as_ref(), request.headers()).await
    {
        Ok(value) => value,
        Err(error) => return auth_error_response(error),
    };
    if let Err(response) =
        require_course_access(state.store.as_ref(), &authenticated, course, true).await
    {
        return response.into_response();
    }
    let expected_revision = match required_revision(request.headers()) {
        Ok(value) => value,
        Err(RevisionHeaderError::Missing) => {
            return error_response(StatusCode::PRECONDITION_REQUIRED, "If-Match is required");
        }
        Err(RevisionHeaderError::Malformed) => {
            return error_response(StatusCode::BAD_REQUEST, "If-Match is malformed");
        }
    };
    if !has_json_content_type(request.headers()) {
        return error_response(
            StatusCode::UNSUPPORTED_MEDIA_TYPE,
            "course grade update must be JSON",
        );
    }
    let body = match to_bytes(request.into_body(), MAX_GRADE_SCHEME_JSON_BYTES + 1).await {
        Ok(value) if value.len() <= MAX_GRADE_SCHEME_JSON_BYTES => value,
        Ok(_) | Err(_) => {
            return error_response(
                StatusCode::PAYLOAD_TOO_LARGE,
                "course grade update is too large",
            );
        }
    };
    let view: CourseGradeSchemeUpdateView = match serde_json::from_slice(&body) {
        Ok(value) => value,
        _ => {
            return error_response(
                StatusCode::UNPROCESSABLE_ENTITY,
                "course grade update is invalid",
            );
        }
    };
    let command = UpdateCourseGradeScheme {
        course,
        expected_revision,
        scheme: view.scheme,
        assignments: view.assignments.into_iter().map(to_membership).collect(),
    };
    match state
        .store
        .update_course_grade_scheme(
            authenticated.tenant_context,
            authenticated.record.token_hash,
            command,
        )
        .await
    {
        Ok(record) => scheme_response(StatusCode::OK, record),
        Err(StoreError::Conflict) => error_response(
            StatusCode::PRECONDITION_FAILED,
            "course grade scheme changed; reload current settings",
        ),
        Err(error) => store_error_response(error),
    }
}

/// Returns server-calculated, protected instructor totals.
pub(super) async fn get_totals<S>(
    State(state): State<CourseRouteState<S>>,
    Path(course): Path<CourseId>,
    headers: HeaderMap,
) -> Response
where
    S: learning_data_access::Store
        + learning_data_access::CourseRecordsAccessStore
        + CourseGradebookStore
        + learning_data_access::SessionStore
        + 'static,
{
    let authenticated = match resolve_request_session(state.store.as_ref(), &headers).await {
        Ok(value) => value,
        Err(error) => return auth_error_response(error),
    };
    if let Err(response) =
        require_course_access(state.store.as_ref(), &authenticated, course, true).await
    {
        return response.into_response();
    }
    match state
        .store
        .course_gradebook_totals(
            authenticated.tenant_context,
            authenticated.record.token_hash,
            course,
        )
        .await
    {
        Ok(totals) => no_store(
            Json(CourseGradebookTotalsView {
                mode: totals.mode,
                rounding: totals.rounding,
                rows: totals
                    .rows
                    .into_iter()
                    .map(|row| CourseGradebookTotalViewRow {
                        display_name: row.display_name,
                        outcome: outcome_view(row.outcome),
                    })
                    .collect(),
            })
            .into_response(),
        ),
        Err(error) => store_error_response(error),
    }
}

/// Generates one synchronous, audited, instructor-only course export.
pub(super) async fn create_export<S>(
    State(state): State<CourseRouteState<S>>,
    Path(course): Path<CourseId>,
    request: Request,
) -> Response
where
    S: learning_data_access::Store
        + learning_data_access::CourseRecordsAccessStore
        + CourseGradebookStore
        + learning_data_access::SessionStore
        + 'static,
{
    let authenticated = match resolve_request_session(state.store.as_ref(), request.headers()).await
    {
        Ok(value) => value,
        Err(error) => return auth_error_response(error),
    };
    if let Err(response) =
        require_course_access(state.store.as_ref(), &authenticated, course, true).await
    {
        return response.into_response();
    }
    let body = match to_bytes(request.into_body(), MAX_EMPTY_EXPORT_BODY_BYTES).await {
        Ok(value) => value,
        Err(_) => {
            return error_response(
                StatusCode::PAYLOAD_TOO_LARGE,
                "course grade export request is invalid",
            );
        }
    };
    if !body.is_empty() {
        return error_response(
            StatusCode::UNPROCESSABLE_ENTITY,
            "course grade export request body must be empty",
        );
    }
    let export = match state
        .store
        .create_course_grade_export(
            authenticated.tenant_context,
            authenticated.record.token_hash,
            course,
        )
        .await
    {
        Ok(value) => value,
        Err(error) => return store_error_response(error),
    };
    let mut writer = csv::WriterBuilder::new()
        .terminator(csv::Terminator::CRLF)
        .from_writer(Vec::new());
    let mode = match export.audit.mode {
        question_model::CourseGradeMode::TotalPoints => "totalPoints",
        question_model::CourseGradeMode::WeightedCategories => "weightedCategories",
    };
    let rounding = "fourDecimalPlacesHalfAwayFromZero";
    if writer
        .write_record([
            "record_type",
            "aggregation_mode",
            "rounding_rule",
            "roster_id",
            "email",
            "display_name",
            "course_total",
            "letter",
            "unavailable_status",
        ])
        .is_err()
        || writer
            .write_record(["metadata", mode, rounding, "", "", "", "", "", ""])
            .is_err()
    {
        return export_unavailable();
    }
    for row in &export.rows {
        let (score, letter, unavailable) = match &row.outcome.unavailable_reason {
            Some(reason) => (
                String::new(),
                String::new(),
                unavailable_reason_name(*reason).to_owned(),
            ),
            None => (
                row.outcome
                    .rounded_score
                    .map(|value| value.to_string())
                    .unwrap_or_default(),
                row.outcome.letter.clone().unwrap_or_default(),
                String::new(),
            ),
        };
        let email = csv_text(
            row.roster_email
                .as_ref()
                .map(AuthenticationEmail::delivery)
                .unwrap_or_default(),
        );
        let roster_id = row
            .roster_id
            .as_ref()
            .map(CourseRosterId::as_str)
            .unwrap_or_default();
        let display_name = csv_text(row.display_name.as_str());
        let letter = csv_text(&letter);
        if writer
            .write_record([
                "student",
                mode,
                rounding,
                roster_id,
                email.as_str(),
                display_name.as_str(),
                &score,
                letter.as_str(),
                &unavailable,
            ])
            .is_err()
        {
            return export_unavailable();
        }
    }
    let body = match writer.into_inner() {
        Ok(value) => value,
        Err(_) => return export_unavailable(),
    };
    let mut response = body.into_response();
    response.headers_mut().insert(
        CONTENT_TYPE,
        HeaderValue::from_static("text/csv; charset=utf-8"),
    );
    response.headers_mut().insert(
        CONTENT_DISPOSITION,
        HeaderValue::from_static("attachment; filename=ple-course-grades.csv"),
    );
    response.headers_mut().insert(
        EXPORT_ID_HEADER,
        HeaderValue::from_str(&export.audit.id.as_uuid().to_string())
            .expect("UUID is valid header"),
    );
    no_store(response)
}

fn to_membership(value: CourseGradeAssignmentSetting) -> CourseGradeAssignmentMembership {
    CourseGradeAssignmentMembership {
        assignment: value.assignment,
        included: value.included,
        category: value.category,
        position: value.position,
    }
}

fn scheme_response(
    status: StatusCode,
    record: learning_data_access::CourseGradeSchemeRecord,
) -> Response {
    let revision = record.revision.value();
    let view = CourseGradeSchemeView {
        scheme: record.scheme,
        assignments: record
            .assignments
            .into_iter()
            .map(|value| CourseGradeAssignmentView {
                assignment: value.assignment,
                title: value.title,
                included: value.included,
                category: value.category,
                position: value.position,
            })
            .collect(),
    };
    let mut response = (status, Json(view)).into_response();
    response.headers_mut().insert(
        ETAG,
        HeaderValue::from_str(&format!("\"{revision}\""))
            .expect("positive revision is a valid ETag"),
    );
    no_store(response)
}

fn outcome_view(value: CourseGradeOutcome) -> CourseGradeOutcomeView {
    match value.unavailable_reason {
        Some(reason) => CourseGradeOutcomeView::Unavailable {
            reason: unavailable_reason_view(reason),
        },
        None => CourseGradeOutcomeView::Available {
            score: value.rounded_score.expect("available grade has score"),
            letter: value.letter,
            dropped_assignment_ids: value.dropped_assignment_ids,
            total_earned: value.total_earned,
            total_possible: value.total_possible,
        },
    }
}

fn unavailable_reason_view(
    value: CourseGradeUnavailableReason,
) -> CourseGradeUnavailableReasonView {
    match value {
        CourseGradeUnavailableReason::NoIncludedAssignments => {
            CourseGradeUnavailableReasonView::NoIncludedAssignments
        }
        CourseGradeUnavailableReason::Recalculating => {
            CourseGradeUnavailableReasonView::Recalculating
        }
        CourseGradeUnavailableReason::Failed => CourseGradeUnavailableReasonView::Failed,
        CourseGradeUnavailableReason::EmptyAfterDrop => {
            CourseGradeUnavailableReasonView::EmptyAfterDrop
        }
        CourseGradeUnavailableReason::ZeroPossiblePoints => {
            CourseGradeUnavailableReasonView::ZeroPossiblePoints
        }
    }
}
fn unavailable_reason_name(value: CourseGradeUnavailableReason) -> &'static str {
    match value {
        CourseGradeUnavailableReason::NoIncludedAssignments => "noIncludedAssignments",
        CourseGradeUnavailableReason::Recalculating => "recalculating",
        CourseGradeUnavailableReason::Failed => "failed",
        CourseGradeUnavailableReason::EmptyAfterDrop => "emptyAfterDrop",
        CourseGradeUnavailableReason::ZeroPossiblePoints => "zeroPossiblePoints",
    }
}

/// Makes a text cell inert for spreadsheet programs while leaving ordinary
/// course data untouched. RFC4180 quoting remains csv::Writer's responsibility.
fn csv_text(value: &str) -> String {
    if matches!(value.as_bytes().first(), Some(b'=' | b'+' | b'-' | b'@')) {
        format!("'{value}")
    } else {
        value.to_owned()
    }
}

fn export_unavailable() -> Response {
    error_response(
        StatusCode::SERVICE_UNAVAILABLE,
        "course grade export is unavailable",
    )
}
fn has_json_content_type(headers: &HeaderMap) -> bool {
    let mut values = headers.get_all(CONTENT_TYPE).iter();
    let Some(value) = values.next().and_then(|value| value.to_str().ok()) else {
        return false;
    };
    values.next().is_none()
        && value
            .split_once(';')
            .map_or(value, |(media_type, _)| media_type)
            .trim()
            .eq_ignore_ascii_case("application/json")
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RevisionHeaderError {
    Missing,
    Malformed,
}
fn required_revision(
    headers: &HeaderMap,
) -> Result<CourseGradeSchemeRevision, RevisionHeaderError> {
    let mut values = headers.get_all(IF_MATCH).iter();
    let Some(value) = values.next() else {
        return Err(RevisionHeaderError::Missing);
    };
    if values.next().is_some() {
        return Err(RevisionHeaderError::Malformed);
    }
    let value = value.to_str().map_err(|_| RevisionHeaderError::Malformed)?;
    let value = value
        .strip_prefix('"')
        .and_then(|value| value.strip_suffix('"'))
        .ok_or(RevisionHeaderError::Malformed)?;
    if value.contains('"') || value.starts_with("W/") {
        return Err(RevisionHeaderError::Malformed);
    }
    let value = value
        .parse::<u64>()
        .map_err(|_| RevisionHeaderError::Malformed)?;
    let revision =
        CourseGradeSchemeRevision::from_u64(value).map_err(|_| RevisionHeaderError::Malformed)?;
    revision
        .to_i64()
        .map_err(|_| RevisionHeaderError::Malformed)?;
    Ok(revision)
}

#[cfg(test)]
mod tests {
    use super::csv_text;

    #[test]
    fn csv_text_neutralizes_spreadsheet_formula_prefixes_without_changing_ordinary_text() {
        for value in ["=SUM(A1:A2)", "+1", "-1", "@command"] {
            assert_eq!(csv_text(value), format!("'{value}"));
        }
        assert_eq!(
            csv_text("Student, \"Example\"\nName"),
            "Student, \"Example\"\nName"
        );
    }
}

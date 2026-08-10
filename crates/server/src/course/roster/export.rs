//! Manager-authorized ephemeral CSV grade export.

use axum::body::to_bytes;
use axum::extract::{Path, Request, State};
use axum::http::header::{CONTENT_DISPOSITION, CONTENT_TYPE};
use axum::http::{HeaderValue, StatusCode};
use axum::response::{IntoResponse, Response};
use learning_data_access::{CreateManualGradeExport, ManualGradeExportStore};
use question_model::{AssignmentId, CourseId};

use super::CourseRosterRouteState;
use crate::auth::{auth_error_response, no_store, resolve_request_session};
use crate::course::policy::require_course_access;
use crate::course::projection::{error_response, store_error_response};

const MAX_EMPTY_EXPORT_BODY_BYTES: usize = 64;
const EXPORT_ID_HEADER: &str = "x-ple-export-id";

pub(super) async fn create<S>(
    State(state): State<CourseRosterRouteState<S>>,
    Path((course, assignment)): Path<(CourseId, AssignmentId)>,
    request: Request,
) -> Response
where
    S: learning_data_access::Store
        + learning_data_access::CourseRecordsAccessStore
        + learning_data_access::CourseRosterStore
        + ManualGradeExportStore
        + learning_data_access::SessionStore
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
    let body = match to_bytes(request.into_body(), MAX_EMPTY_EXPORT_BODY_BYTES).await {
        Ok(body) => body,
        Err(_) => {
            return error_response(
                StatusCode::PAYLOAD_TOO_LARGE,
                "grade export request is invalid",
            );
        }
    };
    if !body.is_empty() {
        return error_response(
            StatusCode::UNPROCESSABLE_ENTITY,
            "grade export request body must be empty",
        );
    }
    let export = match state
        .store
        .create_manual_grade_export(
            authenticated.tenant_context,
            authenticated.record.token_hash,
            CreateManualGradeExport { course, assignment },
        )
        .await
    {
        Ok(export) => export,
        Err(error) => return store_error_response(error),
    };
    let mut writer = csv::WriterBuilder::new()
        .terminator(csv::Terminator::CRLF)
        .from_writer(Vec::new());
    if writer
        .write_record(["roster_id", "email", "display_name", "score"])
        .is_err()
    {
        return export_unavailable();
    }
    for row in &export.rows {
        let score = row.current_score.map(|value| value.to_string());
        if writer
            .write_record([
                row.roster_id.as_str(),
                row.roster_email.delivery(),
                row.display_name.as_str(),
                score.as_deref().unwrap_or(""),
            ])
            .is_err()
        {
            return export_unavailable();
        }
    }
    let body = match writer.into_inner() {
        Ok(body) => body,
        Err(_) => return export_unavailable(),
    };
    let mut response = body.into_response();
    response.headers_mut().insert(
        CONTENT_TYPE,
        HeaderValue::from_static("text/csv; charset=utf-8"),
    );
    response.headers_mut().insert(
        CONTENT_DISPOSITION,
        HeaderValue::from_str(&format!(
            "attachment; filename=ple-grade-export-{}.csv",
            assignment.as_uuid()
        ))
        .expect("UUID filename is a valid header value"),
    );
    response.headers_mut().insert(
        EXPORT_ID_HEADER,
        HeaderValue::from_str(&export.id.as_uuid().to_string())
            .expect("UUID export ID is a valid header value"),
    );
    no_store(response)
}

fn export_unavailable() -> Response {
    error_response(
        StatusCode::SERVICE_UNAVAILABLE,
        "grade export is unavailable",
    )
}

use axum::Json;
use axum::http::header::{ETAG, IF_MATCH};
use axum::http::{HeaderMap, HeaderValue, StatusCode};
use axum::response::{IntoResponse, Response};
use learning_data_access::{
    Cursor, PageRequest, PageSize, PaginationError, StoreError, WorkspaceDraft,
    WorkspaceDraftRevision,
};
use question_model::{DraftQuestionDefinition, UserRole};
use serde::{Deserialize, Serialize};

use crate::auth::no_store;

pub(super) const MAX_WORKSPACE_BODY_BYTES: usize = 64 * 1_024;
const DEFAULT_PAGE_SIZE: u16 = 50;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct WorkspaceQuery {
    cursor: Option<String>,
    page_size: Option<u16>,
}

pub(super) fn may_author_workspaces(roles: &[UserRole]) -> bool {
    roles.iter().any(|role| {
        matches!(
            role,
            UserRole::Instructor | UserRole::Publisher | UserRole::Administrator
        )
    })
}

pub(super) fn page_request(query: WorkspaceQuery) -> Result<PageRequest, PaginationError> {
    let size = PageSize::new(query.page_size.unwrap_or(DEFAULT_PAGE_SIZE))?;
    match query.cursor {
        Some(cursor) => Ok(PageRequest::after(Cursor::parse(cursor)?, size)),
        None => Ok(PageRequest::first(size)),
    }
}

pub(super) fn store_error_response(error: StoreError) -> Response {
    match error {
        StoreError::NotFound => error_response(StatusCode::NOT_FOUND, "workspace not found"),
        StoreError::AlreadyExists => {
            error_response(StatusCode::CONFLICT, "workspace already exists")
        }
        StoreError::Conflict => {
            error_response(StatusCode::CONFLICT, "workspace changed; reload it")
        }
        // Workspace visibility is governed by persisted owner/collaborator
        // bindings. Returning not-found keeps an unshared same-tenant draft
        // indistinguishable from an absent or foreign draft.
        StoreError::TenantMismatch | StoreError::Forbidden => {
            error_response(StatusCode::NOT_FOUND, "workspace not found")
        }
        StoreError::InvalidRecord(message) => {
            error_response(StatusCode::UNPROCESSABLE_ENTITY, &message)
        }
        StoreError::RunModel(error) => {
            error_response(StatusCode::UNPROCESSABLE_ENTITY, &error.to_string())
        }
        StoreError::TimedOut => {
            error_response(StatusCode::CONFLICT, "workspace operation timed out")
        }
        StoreError::RetryableTransaction | StoreError::Unavailable(_) => error_response(
            StatusCode::SERVICE_UNAVAILABLE,
            "workspace storage unavailable",
        ),
    }
}

pub(super) fn error_response(status: StatusCode, message: &str) -> Response {
    no_store((status, Json(serde_json::json!({ "error": message }))).into_response())
}

/// Returns the browser-safe draft while keeping the concurrency token in a
/// standard response header rather than authored JSON. A later PUT must echo
/// this exact strong ETag in `If-Match` to replace an existing draft.
pub(super) fn draft_response(draft: WorkspaceDraft) -> Response {
    revisioned_response(draft.revision, draft.record.question)
}

/// Attaches the exact private-draft revision represented by an authoring
/// response. Validation and publication diff are snapshots too, so their
/// strong ETag must be treated with the same freshness semantics as detail.
pub(super) fn revisioned_response<T>(revision: WorkspaceDraftRevision, body: T) -> Response
where
    T: Serialize,
{
    let revision = HeaderValue::from_str(&format!("\"{}\"", revision.value()))
        .expect("a decimal workspace revision is always a valid ETag");
    let mut response = Json(body).into_response();
    response.headers_mut().insert(ETAG, revision);
    no_store(response)
}

/// Parses the single strong ETag accepted by workspace PUT.
///
/// Omitting the precondition asks storage to create a new draft. Storage
/// rejects that request with 409 when the workspace already exists, so this
/// never becomes a last-writer-wins update path.
pub(super) fn expected_revision(headers: &HeaderMap) -> Result<Option<WorkspaceDraftRevision>, ()> {
    let mut values = headers.get_all(IF_MATCH).iter();
    let Some(value) = values.next() else {
        return Ok(None);
    };
    if values.next().is_some() {
        return Err(());
    }
    let value = value.to_str().map_err(|_| ())?;
    let Some(value) = value
        .strip_prefix('"')
        .and_then(|value| value.strip_suffix('"'))
    else {
        return Err(());
    };
    if value.is_empty() || !value.bytes().all(|byte| byte.is_ascii_digit()) {
        return Err(());
    }
    let numeric = value.parse::<u64>().map_err(|_| ())?;
    if numeric == 0 || numeric > i64::MAX as u64 {
        return Err(());
    }
    serde_json::from_str(value).map(Some).map_err(|_| ())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum RequiredRevisionError {
    Missing,
    Malformed,
}

/// Parses the one current strong ETag that must accompany destructive draft
/// deletion. A missing precondition is distinguishable from malformed input
/// so the browser can refresh rather than mistaking a stale tab for a valid
/// delete request.
pub(super) fn required_revision(
    headers: &HeaderMap,
) -> Result<WorkspaceDraftRevision, RequiredRevisionError> {
    match expected_revision(headers) {
        Ok(Some(revision)) => Ok(revision),
        Ok(None) => Err(RequiredRevisionError::Missing),
        Err(()) => Err(RequiredRevisionError::Malformed),
    }
}

pub(super) async fn no_store_response(response: Response) -> Response {
    no_store(response)
}

/// Decodes exactly the browser workspace contract. Serde's ordinary model
/// deserialization tolerates additional fields for storage evolution; this
/// HTTP boundary compares the typed canonical form to received JSON so
/// unknown fields are rejected at every nested level.
pub(super) fn strict_draft_definition(
    value: serde_json::Value,
) -> Result<DraftQuestionDefinition, ()> {
    let question: DraftQuestionDefinition =
        serde_json::from_value(value.clone()).map_err(|_| ())?;
    let canonical = serde_json::to_value(&question).map_err(|_| ())?;
    if value == canonical {
        Ok(question)
    } else {
        Err(())
    }
}

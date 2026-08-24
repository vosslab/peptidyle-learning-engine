//! Shared transport boundary for live Instructor rehearsal operations.
//!
//! The concrete handlers and execution coordinator live beside this module as
//! the durable rehearsal operation protocol lands.  This boundary deliberately
//! keeps HTTP parsing small: authentication and direct-Instructor route
//! binding must complete before a handler calls [`json_body_after_authorization`].

use std::sync::Arc;

use axum::Json;
use axum::Router;
use axum::body::to_bytes;
use axum::extract::{Path, Request, State};
use axum::http::header::{CONTENT_TYPE, IF_MATCH};
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use learning_data_access::{
    CourseRecordsAccessStore, NavigationReferenceStore, ReadRehearsalRouteCommand,
    RehearsalIdempotencyKey, RehearsalOperationDigest, RehearsalRouteMutationStore, RehearsalStore,
    SessionStore, StartRehearsalRouteCommand, Store, StoreError,
};
use question_model::{RehearsalReference, RehearsalStartRequest, TeachingOperationRevision};
use serde::de::DeserializeOwned;

use crate::auth::no_store;
use crate::run::RunBackend;

mod execution;
mod routes;

pub(crate) use execution::{
    RehearsalExecutionCoordinator, RehearsalGradeBackend, RehearsalIssueBackend,
};

struct RehearsalRouteState<S, B> {
    store: Arc<S>,
    coordinator: Arc<RehearsalExecutionCoordinator<B>>,
}

impl<S, B> Clone for RehearsalRouteState<S, B> {
    fn clone(&self) -> Self {
        Self {
            store: Arc::clone(&self.store),
            coordinator: Arc::clone(&self.coordinator),
        }
    }
}

/// Builds the live Instructor rehearsal route group.  Every handler binds the
/// public course/assignment pair before decoding a rehearsal body or RH-
/// reference, then delegates authorization-sensitive lookup to `RehearsalStore`.
pub(crate) fn router<S, B>(
    store: Arc<S>,
    coordinator: Arc<RehearsalExecutionCoordinator<B>>,
) -> Router
where
    S: Store
        + CourseRecordsAccessStore
        + NavigationReferenceStore
        + RehearsalRouteMutationStore
        + RehearsalStore
        + SessionStore
        + 'static,
    B: RunBackend + 'static,
{
    Router::new()
        .route(
            "/api/courses/{course}/assignments/{assignment}/rehearsals",
            post(start_rehearsal::<S, B>),
        )
        .route(
            "/api/courses/{course}/assignments/{assignment}/rehearsals/{rehearsal}",
            get(get_rehearsal::<S, B>),
        )
        .with_state(RehearsalRouteState { store, coordinator })
}

async fn start_rehearsal<S, B>(
    State(state): State<RehearsalRouteState<S, B>>,
    Path((course, assignment)): Path<(String, String)>,
    request: Request,
) -> Response
where
    S: Store
        + CourseRecordsAccessStore
        + NavigationReferenceStore
        + RehearsalRouteMutationStore
        + RehearsalStore
        + SessionStore
        + 'static,
    B: RunBackend + 'static,
{
    let bound = match routes::authorize_bound(&state.store, &course, &assignment, request.headers())
        .await
    {
        Ok(bound) => bound,
        Err(response) => return response,
    };
    let headers = match mutation_headers(request.headers()) {
        Ok(headers) => headers,
        Err(response) => return *response,
    };
    let body = match json_body_after_authorization::<RehearsalStartRequest>(request).await {
        Ok(body) => body,
        Err(response) => return response,
    };
    let fingerprint = match digest_start_request(&body, headers.revision) {
        Ok(value) => value,
        Err(response) => return *response,
    };
    match state
        .store
        .start_rehearsal_from_route(
            bound.authenticated.tenant_context,
            StartRehearsalRouteCommand {
                actor: bound.authenticated.record.subject.user(),
                course: bound.course,
                assignment: bound.assignment,
                expected_revision: headers.revision,
                subject: body.subject,
                start_new_after_completion: body.start_new_after_completion,
                idempotency_key: headers.idempotency_key,
                request_fingerprint: fingerprint,
            },
        )
        .await
    {
        Ok(result) => no_store(
            (
                if result.replayed {
                    StatusCode::OK
                } else {
                    StatusCode::CREATED
                },
                Json(result.receipt),
            )
                .into_response(),
        ),
        Err(error) => rehearsal_store_error(error),
    }
}

async fn get_rehearsal<S, B>(
    State(state): State<RehearsalRouteState<S, B>>,
    Path((course, assignment, rehearsal)): Path<(String, String, String)>,
    request: Request,
) -> Response
where
    S: Store
        + CourseRecordsAccessStore
        + NavigationReferenceStore
        + RehearsalRouteMutationStore
        + RehearsalStore
        + SessionStore
        + 'static,
    B: RunBackend + 'static,
{
    let bound = match routes::authorize_bound(&state.store, &course, &assignment, request.headers())
        .await
    {
        Ok(bound) => bound,
        Err(response) => return response,
    };
    let rehearsal = match rehearsal.parse::<RehearsalReference>() {
        Ok(reference) => reference,
        Err(_) => return routes::concealed_route_response(),
    };
    match state
        .store
        .read_rehearsal_from_route(
            bound.authenticated.tenant_context,
            ReadRehearsalRouteCommand {
                actor: bound.authenticated.record.subject.user(),
                course: bound.course,
                assignment: bound.assignment,
                rehearsal,
            },
        )
        .await
    {
        Ok(receipt) => no_store(Json(receipt).into_response()),
        Err(error) => rehearsal_store_error(error),
    }
}

fn digest_start_request(
    request: &RehearsalStartRequest,
    revision: TeachingOperationRevision,
) -> Result<RehearsalOperationDigest, Box<Response>> {
    let bytes =
        serde_json::to_vec(&("rehearsal-start-v1", revision.value(), request)).map_err(|_| {
            Box::new(error_response(
                StatusCode::UNPROCESSABLE_ENTITY,
                "rehearsal request is invalid",
            ))
        })?;
    Ok(RehearsalOperationDigest::from_bytes(
        *objects::Sha256Digest::compute(&bytes).as_bytes(),
    ))
}

fn rehearsal_store_error(error: StoreError) -> Response {
    match error {
        StoreError::NotFound | StoreError::Forbidden | StoreError::TenantMismatch => {
            routes::concealed_route_response()
        }
        StoreError::Conflict => error_response(
            StatusCode::PRECONDITION_FAILED,
            "assignment changed; reload it",
        ),
        StoreError::InvalidRecord(_) | StoreError::RunModel(_) => error_response(
            StatusCode::UNPROCESSABLE_ENTITY,
            "rehearsal request is invalid",
        ),
        StoreError::AlreadyExists => error_response(
            StatusCode::CONFLICT,
            "rehearsal request conflicts with an existing operation",
        ),
        StoreError::TimedOut => error_response(StatusCode::CONFLICT, "rehearsal attempt expired"),
        StoreError::RetryableTransaction | StoreError::Unavailable(_) => error_response(
            StatusCode::SERVICE_UNAVAILABLE,
            "rehearsal storage is unavailable",
        ),
    }
}

/// One rehearsal command is intentionally no larger than a normal server-side
/// automated-grading response.  The bound is applied before JSON decoding.
pub(crate) const MAX_REHEARSAL_BODY_BYTES: usize = 64 * 1024;

/// Header material required by every mutating rehearsal command.
///
/// The browser retries the same key for the same logical transition.  The
/// server turns both values into typed, bounded values before they can reach a
/// Store idempotency or revision fence.
#[derive(Debug)]
pub(crate) struct RehearsalMutationHeaders {
    pub(crate) idempotency_key: RehearsalIdempotencyKey,
    pub(crate) revision: TeachingOperationRevision,
}

/// Parse one mutation's concurrency and idempotency headers.
///
/// A route invokes this only after it has authenticated and bound its C-/A-
/// path to a direct Instructor.  Header parsing itself has no data access and
/// therefore cannot disclose a protected assignment.
pub(crate) fn mutation_headers(
    headers: &HeaderMap,
) -> Result<RehearsalMutationHeaders, Box<Response>> {
    let mut values = headers.get_all(IF_MATCH).iter();
    let Some(value) = values.next() else {
        return Err(Box::new(error_response(
            StatusCode::PRECONDITION_REQUIRED,
            "If-Match assignment revision is required",
        )));
    };
    if values.next().is_some() {
        return Err(Box::new(error_response(
            StatusCode::UNPROCESSABLE_ENTITY,
            "If-Match assignment revision is invalid",
        )));
    }
    let revision = value
        .to_str()
        .ok()
        .and_then(|raw| raw.strip_prefix('"').and_then(|raw| raw.strip_suffix('"')))
        .filter(|raw| !raw.is_empty() && raw.bytes().all(|byte| byte.is_ascii_digit()))
        .and_then(|raw| raw.parse::<u64>().ok())
        .and_then(TeachingOperationRevision::new)
        .ok_or_else(|| {
            Box::new(error_response(
                StatusCode::UNPROCESSABLE_ENTITY,
                "If-Match assignment revision is invalid",
            ))
        })?;
    let mut keys = headers.get_all("idempotency-key").iter();
    let Some(key) = keys.next() else {
        return Err(Box::new(error_response(
            StatusCode::BAD_REQUEST,
            "Idempotency-Key is required",
        )));
    };
    if keys.next().is_some() {
        return Err(Box::new(error_response(
            StatusCode::BAD_REQUEST,
            "Idempotency-Key is invalid",
        )));
    }
    let idempotency_key = key
        .to_str()
        .ok()
        .and_then(|raw| RehearsalIdempotencyKey::new(raw.to_owned()).ok())
        .ok_or_else(|| {
            Box::new(error_response(
                StatusCode::BAD_REQUEST,
                "Idempotency-Key is invalid",
            ))
        })?;
    Ok(RehearsalMutationHeaders {
        idempotency_key,
        revision,
    })
}

/// Decode a bounded JSON request after authorization has already succeeded.
pub(crate) async fn json_body_after_authorization<T>(request: Request) -> Result<T, Response>
where
    T: DeserializeOwned,
{
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
            "rehearsal request must be JSON",
        ));
    }
    let body = to_bytes(request.into_body(), MAX_REHEARSAL_BODY_BYTES + 1)
        .await
        .map_err(|_| {
            error_response(
                StatusCode::PAYLOAD_TOO_LARGE,
                "rehearsal request is too large",
            )
        })?;
    if body.len() > MAX_REHEARSAL_BODY_BYTES {
        return Err(error_response(
            StatusCode::PAYLOAD_TOO_LARGE,
            "rehearsal request is too large",
        ));
    }
    serde_json::from_slice(&body).map_err(|_| {
        error_response(
            StatusCode::UNPROCESSABLE_ENTITY,
            "rehearsal request is invalid",
        )
    })
}

pub(crate) fn error_response(status: StatusCode, message: &str) -> Response {
    no_store((status, Json(serde_json::json!({ "error": message }))).into_response())
}

#[cfg(test)]
mod tests {
    use axum::http::{HeaderMap, HeaderValue};

    use super::*;

    #[test]
    fn mutation_headers_require_one_quoted_revision_and_one_bounded_key() {
        let mut headers = HeaderMap::new();
        headers.insert(IF_MATCH, HeaderValue::from_static("\"7\""));
        headers.insert(
            "idempotency-key",
            HeaderValue::from_static("rehearsal-start-1"),
        );

        let parsed = mutation_headers(&headers).expect("valid mutation headers");
        assert_eq!(parsed.revision.value(), 7);
        assert_eq!(parsed.idempotency_key.as_str(), "rehearsal-start-1");
    }

    #[test]
    fn mutation_headers_refuse_duplicate_idempotency_keys() {
        let mut headers = HeaderMap::new();
        headers.insert(IF_MATCH, HeaderValue::from_static("\"7\""));
        headers.append("idempotency-key", HeaderValue::from_static("first"));
        headers.append("idempotency-key", HeaderValue::from_static("second"));

        assert_eq!(
            mutation_headers(&headers)
                .expect_err("duplicate key is invalid")
                .status(),
            StatusCode::BAD_REQUEST
        );
    }
}

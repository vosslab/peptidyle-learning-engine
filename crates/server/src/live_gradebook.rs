//! Current-Course Instructor Gradebook evidence route.

mod export;

#[cfg(test)]
#[path = "live_gradebook/export_http_tests.rs"]
mod export_http_tests;

use std::{str::FromStr, sync::Arc};

use axum::{
    Json, Router,
    extract::{Path, Query, State, rejection::QueryRejection},
    http::{
        HeaderMap, StatusCode,
        header::{CONTENT_DISPOSITION, CONTENT_TYPE, COOKIE, X_CONTENT_TYPE_OPTIONS},
    },
    middleware,
    response::{IntoResponse, Response},
    routing::get,
};
use learning_data_access::{
    CourseGradebookStore, SessionTokenHash, StoreError,
    postgres::{PostgresCourseGradebookStore, PostgresSessionStore},
};
use question_model::{CourseInstanceId, ProductRole};
use serde::Deserialize;

use crate::auth::{AuthError, resolve_session};

#[derive(Clone)]
struct RouteState {
    sessions: Arc<PostgresSessionStore>,
    gradebook: PostgresCourseGradebookStore,
}

pub fn live_gradebook_router(
    sessions: Arc<PostgresSessionStore>,
    gradebook: PostgresCourseGradebookStore,
) -> Router {
    Router::new()
        .route(
            "/api/course-instances/{course_instance_id}/gradebook",
            get(read_gradebook),
        )
        .route(
            "/api/course-instances/{course_instance_id}/gradebook/export",
            get(download_gradebook),
        )
        // ASVS 14.3.2: include extractor and method rejections in no-store handling.
        .layer(middleware::map_response(async |response: Response| {
            crate::auth::no_store(response)
        }))
        .with_state(RouteState {
            sessions,
            gradebook,
        })
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ExportQuery {
    format: export::Format,
}

async fn download_gradebook(
    State(state): State<RouteState>,
    headers: HeaderMap,
    Path(reference): Path<String>,
    query: Result<Query<ExportQuery>, QueryRejection>,
) -> Response {
    // ASVS 2.2.1: a required closed query rejects duplicate and unknown fields.
    let format = match query {
        Ok(Query(query)) => query.format,
        Err(_) => return route_error(StatusCode::BAD_REQUEST, "Invalid Gradebook export format"),
    };
    let course = match CourseInstanceId::from_str(&reference) {
        Ok(value) => value,
        Err(_) => return concealed(),
    };
    let token = match instructor_session_hash(&state, &headers).await {
        Ok(value) => value,
        Err(response) => return *response,
    };
    // ASVS 8.2.1-8.2.3/8.3.1: repeat ordinary exact-Course authorization on every read.
    let gradebook = match state
        .gradebook
        .course_gradebook(token, course.clone())
        .await
    {
        Ok(value) => value,
        Err(error) => return store_error_response(error),
    };
    let bytes = match export::encode(&gradebook, format) {
        Ok(value) => value,
        Err(_) => {
            return route_error(
                StatusCode::SERVICE_UNAVAILABLE,
                "Gradebook export unavailable",
            );
        }
    };
    // ASVS 3.2.1/3.4.4/5.4.1-5.4.2: fixed media types and canonical-only filenames.
    (
        [
            (CONTENT_TYPE, format.media_type().to_owned()),
            (
                CONTENT_DISPOSITION,
                format!(
                    "attachment; filename=\"ple_{course}_grades.{}\"",
                    format.extension()
                ),
            ),
            (X_CONTENT_TYPE_OPTIONS, "nosniff".to_owned()),
        ],
        bytes,
    )
        .into_response()
}

async fn read_gradebook(
    State(state): State<RouteState>,
    headers: HeaderMap,
    Path(reference): Path<String>,
) -> Response {
    let course = match CourseInstanceId::from_str(&reference) {
        Ok(value) => value,
        Err(_) => return concealed(),
    };
    let token = match instructor_session_hash(&state, &headers).await {
        Ok(value) => value,
        Err(response) => return *response,
    };
    match state.gradebook.course_gradebook(token, course).await {
        // ASVS 4.1.3/8.2.1: the Store procedure repeats exact current Course
        // Instructor authorization; this response exposes only answer-free
        // immutable Grading Result aggregates.
        Ok(gradebook) => crate::auth::no_store(Json(gradebook).into_response()),
        Err(error) => store_error_response(error),
    }
}

async fn instructor_session_hash(
    state: &RouteState,
    headers: &HeaderMap,
) -> Result<SessionTokenHash, Box<Response>> {
    match resolve_session(
        state.sessions.as_ref(),
        joined_cookie_header(headers).as_deref(),
    )
    .await
    {
        Ok(session) if session.record.product_role == ProductRole::Instructor => {
            Ok(session.session_hash)
        }
        Ok(_) | Err(AuthError::Unauthenticated) => Err(Box::new(concealed())),
        Err(AuthError::Unavailable(_) | AuthError::Randomness(_)) => Err(Box::new(route_error(
            StatusCode::SERVICE_UNAVAILABLE,
            "Gradebook authentication unavailable",
        ))),
    }
}

fn joined_cookie_header(headers: &HeaderMap) -> Option<String> {
    let values = headers
        .get_all(COOKIE)
        .iter()
        .map(|value| value.to_str().ok())
        .collect::<Option<Vec<_>>>()?;
    (!values.is_empty()).then(|| values.join("; "))
}

fn store_error_response(error: StoreError) -> Response {
    match error {
        StoreError::NotFound | StoreError::Forbidden | StoreError::OwnershipMismatch => concealed(),
        StoreError::Conflict | StoreError::RetryableTransaction => {
            route_error(StatusCode::PRECONDITION_FAILED, "Gradebook changed")
        }
        StoreError::LifecycleConflict => {
            route_error(StatusCode::CONFLICT, "Gradebook lifecycle conflict")
        }
        StoreError::InvalidRecord(_) => {
            route_error(StatusCode::UNPROCESSABLE_ENTITY, "Gradebook is invalid")
        }
        StoreError::AlreadyExists => route_error(StatusCode::CONFLICT, "Gradebook conflict"),
        StoreError::AssessmentActivity(_)
        | StoreError::TimedOut
        | StoreError::LeaseLost
        | StoreError::Unavailable(_) => {
            route_error(StatusCode::SERVICE_UNAVAILABLE, "Gradebook unavailable")
        }
    }
}

fn concealed() -> Response {
    route_error(StatusCode::NOT_FOUND, "Gradebook not found")
}

fn route_error(status: StatusCode, message: &'static str) -> Response {
    crate::auth::no_store((status, message).into_response())
}

//! Live Course Instance creation, member summary, and teaching-team Server Routes.
//!
//! This module exposes the smallest live-teaching boundary: an exact Blueprint Revision
//! is adopted into a Course Instance with all its Assignments and one Assigned Instructor.
//! Assignments start Unreleased; Student Work is never copied.

use std::{str::FromStr, sync::Arc};

use axum::{
    Json, Router,
    extract::{Path, State},
    http::{
        HeaderMap, HeaderValue, StatusCode,
        header::{COOKIE, ETAG, IF_MATCH},
    },
    response::{IntoResponse, Response},
    routing::{get, put},
};
use learning_data_access::{
    CourseInstancePoolIdIssuer, CourseInstanceStore, CreateCourseInstanceInput, SessionTokenHash,
    StoreError,
    postgres::{PostgresCourseInstanceStore, PostgresSessionStore},
};
use question_model::{CourseInstanceReference, CourseInstanceRouteSummary, ProductRole};
use serde::Serialize;

use crate::{
    auth::{AuthError, resolve_session},
    question_publication::{QuestionIdIssuer, RandomQuestionIdIssuer},
};

impl CourseInstancePoolIdIssuer for RandomQuestionIdIssuer {
    fn issue_question_pool_id(&self) -> Result<question_model::QuestionId, StoreError> {
        self.issue_question_id().map_err(|_| {
            StoreError::Unavailable(
                "Question Pool fork identity issuance is unavailable".to_string(),
            )
        })
    }
}

#[derive(Clone)]
struct CourseInstanceRouteState {
    sessions: Arc<PostgresSessionStore>,
    courses: PostgresCourseInstanceStore,
}

/// Registers the active Course Instance creation, member summary, and teaching-team routes.
pub fn course_instance_router(
    sessions: Arc<PostgresSessionStore>,
    courses: PostgresCourseInstanceStore,
) -> Router {
    Router::new()
        .route(
            "/api/course-instances",
            get(list_course_instances).post(create_course_instance),
        )
        .route(
            "/api/course-instances/{reference}",
            get(load_course_instance),
        )
        .route(
            "/api/course-instances/{reference}/summary",
            get(read_course_summary),
        )
        .route(
            "/api/course-instances/{reference}/classification",
            put(update_classification),
        )
        .route(
            "/api/course-instance-creation/instructors",
            get(list_course_creation_instructors),
        )
        .with_state(CourseInstanceRouteState { sessions, courses })
}

#[derive(Serialize)]
struct CourseInstanceListResponse<T> {
    items: Vec<T>,
    #[serde(rename = "nextCursor")]
    next_cursor: Option<String>,
}

async fn list_course_instances(
    State(state): State<CourseInstanceRouteState>,
    headers: HeaderMap,
) -> Response {
    let session_hash = match instructor_session_hash(&state, &headers).await {
        Ok(value) => value,
        Err(response) => return *response,
    };
    match state.courses.list_course_instances(session_hash).await {
        Ok(items) => crate::auth::no_store(
            Json(CourseInstanceListResponse {
                items,
                next_cursor: None,
            })
            .into_response(),
        ),
        Err(error) => store_error_response(error),
    }
}

async fn create_course_instance(
    State(state): State<CourseInstanceRouteState>,
    headers: HeaderMap,
    Json(input): Json<CreateCourseInstanceInput>,
) -> Response {
    let session_hash = match course_creator_session_hash(&state, &headers).await {
        Ok(value) => value,
        Err(response) => return *response,
    };
    match state
        .courses
        .create_course_instance(session_hash, input, Default::default())
        .await
    {
        Ok(created) => crate::auth::no_store((StatusCode::CREATED, Json(created)).into_response()),
        Err(error) => store_error_response(error),
    }
}

async fn load_course_instance(
    State(state): State<CourseInstanceRouteState>,
    headers: HeaderMap,
    Path(reference): Path<String>,
) -> Response {
    let reference = match CourseInstanceReference::from_str(&reference) {
        Ok(value) => value,
        Err(_) => return concealed(),
    };
    let session_hash = match instructor_session_hash(&state, &headers).await {
        Ok(value) => value,
        Err(response) => return *response,
    };
    match state
        .courses
        .load_course_instance(session_hash, reference)
        .await
    {
        Ok(view) => crate::auth::no_store(Json(view).into_response()),
        Err(error) => store_error_response(error),
    }
}

async fn read_course_summary(
    State(state): State<CourseInstanceRouteState>,
    headers: HeaderMap,
    Path(reference): Path<String>,
) -> Response {
    let reference = match CourseInstanceReference::from_str(&reference) {
        Ok(value) => value,
        Err(_) => return concealed(),
    };
    let session_hash = match authenticated_session_hash(&state, &headers).await {
        Ok(value) => value,
        Err(response) => return *response,
    };
    // ASVS 2.2.1, 8.2.2, and 8.3.1: the positively validated public
    // reference is resolved only through this session's current active Course
    // Membership. The resulting private Course ID never enters the response.
    let course = match state
        .courses
        .resolve_course_navigation(session_hash, reference)
        .await
    {
        Ok(value) => value,
        Err(error) => return store_error_response(error),
    };
    match state
        .courses
        .read_course_summary(session_hash, course)
        .await
    {
        Ok(summary) => crate::auth::no_store(
            Json(CourseInstanceRouteSummary {
                classification: summary.classification,
                reference: summary.reference,
                short_name: summary.short_name,
                long_name: summary.long_name,
                term: summary.term,
                role: summary.role,
            })
            .into_response(),
        ),
        Err(error) => store_error_response(error),
    }
}

async fn update_classification(
    State(state): State<CourseInstanceRouteState>,
    headers: HeaderMap,
    Path(reference): Path<String>,
    Json(classification): Json<question_model::CourseClassification>,
) -> Response {
    let reference = match reference.parse::<CourseInstanceReference>() {
        Ok(value) => value,
        Err(_) => return concealed(),
    };
    let Some(raw) = headers.get(IF_MATCH).and_then(|value| value.to_str().ok()) else {
        return route_error(
            StatusCode::PRECONDITION_REQUIRED,
            "Course metadata precondition is required",
        );
    };
    let expected = match raw
        .strip_prefix('"')
        .and_then(|value| value.strip_suffix('"'))
        .and_then(|value| value.parse::<question_model::CourseMetadataEtag>().ok())
    {
        Some(value) => value,
        None => return route_error(StatusCode::BAD_REQUEST, "Course metadata ETag is invalid"),
    };
    let session = match instructor_session_hash(&state, &headers).await {
        Ok(value) => value,
        Err(response) => return *response,
    };
    match state
        .courses
        .update_course_classification(session, reference, expected, classification)
        .await
    {
        Ok(value) => {
            let etag = match HeaderValue::from_str(&format!("\"{}\"", value.metadata_etag)) {
                Ok(value) => value,
                Err(_) => {
                    return route_error(
                        StatusCode::SERVICE_UNAVAILABLE,
                        "Course metadata unavailable",
                    );
                }
            };
            let mut response = crate::auth::no_store(Json(value).into_response());
            response.headers_mut().insert(ETAG, etag);
            response
        }
        Err(error) => store_error_response(error),
    }
}

async fn list_course_creation_instructors(
    State(state): State<CourseInstanceRouteState>,
    headers: HeaderMap,
) -> Response {
    let session_hash = match sysadmin_session_hash(&state, &headers).await {
        Ok(value) => value,
        Err(response) => return *response,
    };
    match state
        .courses
        .list_course_creation_instructors(session_hash)
        .await
    {
        Ok(items) => crate::auth::no_store(
            Json(CourseInstanceListResponse {
                items,
                next_cursor: None,
            })
            .into_response(),
        ),
        Err(error) => store_error_response(error),
    }
}

async fn instructor_session_hash(
    state: &CourseInstanceRouteState,
    headers: &HeaderMap,
) -> Result<SessionTokenHash, Box<Response>> {
    required_session_hash(state, headers, |role| role == ProductRole::Instructor).await
}

async fn course_creator_session_hash(
    state: &CourseInstanceRouteState,
    headers: &HeaderMap,
) -> Result<SessionTokenHash, Box<Response>> {
    required_session_hash(state, headers, |role| {
        matches!(role, ProductRole::Instructor | ProductRole::Sysadmin)
    })
    .await
}

async fn sysadmin_session_hash(
    state: &CourseInstanceRouteState,
    headers: &HeaderMap,
) -> Result<SessionTokenHash, Box<Response>> {
    required_session_hash(state, headers, |role| role == ProductRole::Sysadmin).await
}

async fn required_session_hash(
    state: &CourseInstanceRouteState,
    headers: &HeaderMap,
    permitted: impl FnOnce(ProductRole) -> bool,
) -> Result<SessionTokenHash, Box<Response>> {
    match resolve_session(
        state.sessions.as_ref(),
        joined_cookie_header(headers).as_deref(),
    )
    .await
    {
        Ok(session) if permitted(session.record.product_role) => Ok(session.session_hash),
        Ok(_) | Err(AuthError::Unauthenticated) => Err(Box::new(concealed())),
        Err(AuthError::Unavailable(_) | AuthError::Randomness(_)) => Err(Box::new(route_error(
            StatusCode::SERVICE_UNAVAILABLE,
            "Course Instance authentication unavailable",
        ))),
    }
}

async fn authenticated_session_hash(
    state: &CourseInstanceRouteState,
    headers: &HeaderMap,
) -> Result<SessionTokenHash, Box<Response>> {
    match resolve_session(
        state.sessions.as_ref(),
        joined_cookie_header(headers).as_deref(),
    )
    .await
    {
        // ASVS 8.2.2 and 8.3.1: Product Role is not Course Membership
        // authority. The Store applies the exact active-membership check.
        Ok(session) => Ok(session.session_hash),
        Err(AuthError::Unauthenticated) => Err(Box::new(concealed())),
        Err(AuthError::Unavailable(_) | AuthError::Randomness(_)) => Err(Box::new(route_error(
            StatusCode::SERVICE_UNAVAILABLE,
            "Course Summary authentication unavailable",
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
            route_error(StatusCode::PRECONDITION_FAILED, "Course Instance changed")
        }
        StoreError::LifecycleConflict => {
            route_error(StatusCode::CONFLICT, "Course Instance lifecycle conflict")
        }
        StoreError::InvalidRecord(_) => route_error(
            StatusCode::UNPROCESSABLE_ENTITY,
            "Course Instance is invalid",
        ),
        StoreError::AlreadyExists => route_error(StatusCode::CONFLICT, "Course Instance conflict"),
        StoreError::AssessmentActivity(_)
        | StoreError::TimedOut
        | StoreError::LeaseLost
        | StoreError::Unavailable(_) => route_error(
            StatusCode::SERVICE_UNAVAILABLE,
            "Course Instance unavailable",
        ),
    }
}

fn concealed() -> Response {
    route_error(StatusCode::NOT_FOUND, "Course Instance not found")
}

fn route_error(status: StatusCode, message: &'static str) -> Response {
    crate::auth::no_store((status, message).into_response())
}

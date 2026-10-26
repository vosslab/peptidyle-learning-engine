//! Live Course Instance creation, member summary, and teaching-team Server Routes.
//!
//! This module exposes the smallest live-teaching boundary: an exact Blueprint Revision
//! is adopted into a Course Instance with all its Assignments and one Assigned Instructor.
//! Assignments start Unreleased; Student Work is never copied.

use std::{str::FromStr, sync::Arc};

use axum::{
    Json, Router,
    extract::{Path, Query, State},
    http::{
        HeaderMap, HeaderValue, StatusCode,
        header::{ETAG, IF_MATCH},
    },
    response::{IntoResponse, Response},
    routing::{get, post, put},
};
use learning_data_access::{
    CourseInstanceStore, CreateCourseInstanceInput, DiscoveryPageSize,
    NumberedDiscoveryPageRequest, PageNumber, SessionTokenHash, StoreError,
    postgres::{PostgresCourseInstanceStore, PostgresSessionStore},
};
use question_model::{CourseInstanceId, CourseInstanceRouteSummary, UserRole};
use serde::{Deserialize, Serialize};

use crate::auth::{AuthError, resolve_session};

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
            "/api/course-instances/{course_instance_id}",
            get(load_course_instance),
        )
        .route(
            "/api/course-instances/{course_instance_id}/summary",
            get(read_course_summary),
        )
        .route(
            "/api/course-instances/{course_instance_id}/classification",
            put(update_classification),
        )
        .route(
            "/api/course-instance-creation/instructors",
            get(list_course_creation_instructors),
        )
        .route("/api/sysadmin/courses", get(list_installation_courses))
        .route(
            "/api/sysadmin/courses/search",
            post(search_installation_courses),
        )
        .route(
            "/api/sysadmin/courses/{course_instance_id}",
            get(load_installation_course),
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
        .create_course_instance(session_hash, input)
        .await
    {
        Ok(created) => crate::auth::no_store((StatusCode::CREATED, Json(created)).into_response()),
        Err(error) => store_error_response(error),
    }
}

async fn load_course_instance(
    State(state): State<CourseInstanceRouteState>,
    headers: HeaderMap,
    Path(course_instance_id): Path<String>,
) -> Response {
    let course_instance_id = match CourseInstanceId::from_str(&course_instance_id) {
        Ok(value) => value,
        Err(_) => return concealed(),
    };
    let session_hash = match instructor_session_hash(&state, &headers).await {
        Ok(value) => value,
        Err(response) => return *response,
    };
    match state
        .courses
        .load_course_instance(session_hash, course_instance_id)
        .await
    {
        Ok(view) => crate::auth::no_store(Json(view).into_response()),
        Err(error) => store_error_response(error),
    }
}

async fn read_course_summary(
    State(state): State<CourseInstanceRouteState>,
    headers: HeaderMap,
    Path(course_instance_id): Path<String>,
) -> Response {
    let course_instance_id = match CourseInstanceId::from_str(&course_instance_id) {
        Ok(value) => value,
        Err(_) => return concealed(),
    };
    let session_hash = match authenticated_session_hash(&state, &headers).await {
        Ok(value) => value,
        Err(response) => return *response,
    };
    // ASVS 2.2.1, 8.2.2, and 8.3.1: the positively validated public
    // Course Instance ID is resolved only through this session's current active Course
    // Membership. The resulting private Course ID never enters the response.
    let course = match state
        .courses
        .resolve_course_navigation(session_hash, course_instance_id)
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
                id: summary.id,
                short_name: summary.short_name,
                long_name: summary.long_name,
                term: summary.term,
                role: summary.role,
                lifecycle_state: summary.lifecycle_state,
            })
            .into_response(),
        ),
        Err(error) => store_error_response(error),
    }
}

async fn update_classification(
    State(state): State<CourseInstanceRouteState>,
    headers: HeaderMap,
    Path(course_instance_id): Path<String>,
    Json(classification): Json<question_model::CourseClassification>,
) -> Response {
    let course_instance_id = match course_instance_id.parse::<CourseInstanceId>() {
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
        .and_then(|value| value.parse::<question_model::CourseEditNumber>().ok())
    {
        Some(value) => value,
        None => {
            return route_error(
                StatusCode::BAD_REQUEST,
                "Course metadata Edit Number is invalid",
            );
        }
    };
    let session = match instructor_session_hash(&state, &headers).await {
        Ok(value) => value,
        Err(response) => return *response,
    };
    match state
        .courses
        .update_course_classification(session, course_instance_id, expected, classification)
        .await
    {
        Ok(value) => {
            let etag = match HeaderValue::from_str(&format!("\"{}\"", value.course_edit_number)) {
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

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct InstallationCourseSearch {
    query: String,
    page_number: Option<u32>,
    #[serde(default)]
    page_size: Option<u16>,
    sort: String,
    activity: Vec<String>,
    retention: Vec<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct InstallationCourseListQuery {
    page_number: Option<u32>,
    page_size: Option<u16>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct InstallationCourseListResponse {
    courses: Vec<learning_data_access::InstallationCourseInspection>,
    page_number: PageNumber,
    page_size: DiscoveryPageSize,
    total_count: u64,
    activity_choices: Vec<InstallationCourseChoice>,
    retention_choices: Vec<InstallationCourseChoice>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct InstallationCourseChoice {
    key: String,
    count: i64,
}

async fn list_installation_courses(
    State(state): State<CourseInstanceRouteState>,
    headers: HeaderMap,
    query: Result<Query<InstallationCourseListQuery>, axum::extract::rejection::QueryRejection>,
) -> Response {
    let (page_number, page_size) = match query {
        Ok(Query(query)) => (query.page_number, query.page_size.unwrap_or(50)),
        Err(_) => {
            return route_error(
                StatusCode::BAD_REQUEST,
                "installation Course page is invalid",
            );
        }
    };
    // ASVS 14.2.1: the unfiltered page carries no search text in the URL.
    installation_courses_response(
        &state,
        &headers,
        InstallationCourseSearch {
            query: String::new(),
            page_number,
            page_size: Some(page_size),
            sort: "nameAscending".to_owned(),
            activity: vec![],
            retention: vec![],
        },
    )
    .await
}

async fn search_installation_courses(
    State(state): State<CourseInstanceRouteState>,
    headers: HeaderMap,
    Json(input): Json<InstallationCourseSearch>,
) -> Response {
    // ASVS 14.2.1: Instructor and Course search text stays in the request body.
    installation_courses_response(&state, &headers, input).await
}

async fn installation_courses_response(
    state: &CourseInstanceRouteState,
    headers: &HeaderMap,
    input: InstallationCourseSearch,
) -> Response {
    let session_hash = match sysadmin_session_hash(state, headers).await {
        Ok(value) => value,
        Err(response) => return *response,
    };
    // ASVS 2.2.1/2.2.2: bound the search before it reaches the store.
    let query = match bounded_installation_query(&input.query) {
        Ok(value) => value,
        Err(response) => return *response,
    };
    let mut activity = input.activity;
    let mut retention = input.retention;
    activity.sort();
    activity.dedup();
    retention.sort();
    retention.dedup();
    if !matches!(input.sort.as_str(), "nameAscending" | "nameDescending")
        || activity.len() > 2
        || retention.len() > 3
        || activity
            .iter()
            .any(|value| !matches!(value.as_str(), "active" | "inactive"))
        || retention
            .iter()
            .any(|value| !matches!(value.as_str(), "active" | "archived" | "deleted"))
    {
        return route_error(
            StatusCode::BAD_REQUEST,
            "installation Course filter is invalid",
        );
    }
    let size = match DiscoveryPageSize::new(input.page_size.unwrap_or(50)) {
        Ok(size) => size,
        Err(_) => {
            return route_error(
                StatusCode::BAD_REQUEST,
                "installation Course page size is invalid",
            );
        }
    };
    let page_number = match PageNumber::new(input.page_number.unwrap_or(1)) {
        Ok(page_number) => page_number,
        Err(_) => {
            return route_error(
                StatusCode::BAD_REQUEST,
                "installation Course page number is invalid",
            );
        }
    };
    let page = NumberedDiscoveryPageRequest {
        page_number,
        page_size: size,
    };
    match state
        .courses
        .list_installation_courses(
            session_hash,
            &query,
            &input.sort,
            &activity,
            &retention,
            page,
        )
        .await
    {
        Ok(page) => {
            let facets = match state
                .courses
                .list_installation_course_facets(session_hash, &query, &activity, &retention)
                .await
            {
                Ok(value) => value,
                Err(error) => return store_error_response(error),
            };
            let choices = |field: &str| {
                facets
                    .iter()
                    .filter(|choice| choice.field_name == field)
                    .map(|choice| InstallationCourseChoice {
                        key: choice.choice_key.clone(),
                        count: choice.matching_count,
                    })
                    .collect()
            };
            crate::auth::no_store(
                Json(InstallationCourseListResponse {
                    courses: page.items,
                    page_number: page.page_number,
                    page_size: page.page_size,
                    total_count: page.total_count,
                    activity_choices: choices("activity"),
                    retention_choices: choices("retention"),
                })
                .into_response(),
            )
        }
        Err(error) => store_error_response(error),
    }
}

async fn load_installation_course(
    State(state): State<CourseInstanceRouteState>,
    headers: HeaderMap,
    Path(course_instance_id): Path<String>,
) -> Response {
    // ASVS 2.2.1/8.2.2/16.5: an invalid public id is the same concealed 404 as a missing Course.
    let course_instance_id = match CourseInstanceId::from_str(&course_instance_id) {
        Ok(value) => value,
        Err(_) => return concealed(),
    };
    let session_hash = match sysadmin_session_hash(&state, &headers).await {
        Ok(value) => value,
        Err(response) => return *response,
    };
    match state
        .courses
        .load_installation_course(session_hash, course_instance_id)
        .await
    {
        Ok(course) => crate::auth::no_store(Json(course).into_response()),
        Err(error) => store_error_response(error),
    }
}

fn bounded_installation_query(value: &str) -> Result<String, Box<Response>> {
    let query = value.trim();
    if query.chars().count() > 200 || query.chars().any(char::is_control) {
        return Err(Box::new(route_error(
            StatusCode::UNPROCESSABLE_ENTITY,
            "Course Instance is invalid",
        )));
    }
    Ok(query.to_string())
}

async fn instructor_session_hash(
    state: &CourseInstanceRouteState,
    headers: &HeaderMap,
) -> Result<SessionTokenHash, Box<Response>> {
    required_session_hash(state, headers, |role| role == UserRole::Instructor).await
}

async fn course_creator_session_hash(
    state: &CourseInstanceRouteState,
    headers: &HeaderMap,
) -> Result<SessionTokenHash, Box<Response>> {
    required_session_hash(state, headers, |role| {
        matches!(role, UserRole::Instructor | UserRole::Sysadmin)
    })
    .await
}

async fn sysadmin_session_hash(
    state: &CourseInstanceRouteState,
    headers: &HeaderMap,
) -> Result<SessionTokenHash, Box<Response>> {
    required_session_hash(state, headers, |role| role == UserRole::Sysadmin).await
}

async fn required_session_hash(
    state: &CourseInstanceRouteState,
    headers: &HeaderMap,
    permitted: impl FnOnce(UserRole) -> bool,
) -> Result<SessionTokenHash, Box<Response>> {
    match resolve_session(
        state.sessions.as_ref(),
        crate::auth::joined_cookie_header(headers).as_deref(),
    )
    .await
    {
        Ok(session) if permitted(session.record.user_role) => Ok(session.session_hash),
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
        crate::auth::joined_cookie_header(headers).as_deref(),
    )
    .await
    {
        // ASVS 8.2.2 and 8.3.1: User Role is not Course Membership
        // authority. The Store applies the exact active-membership check.
        Ok(session) => Ok(session.session_hash),
        Err(AuthError::Unauthenticated) => Err(Box::new(concealed())),
        Err(AuthError::Unavailable(_) | AuthError::Randomness(_)) => Err(Box::new(route_error(
            StatusCode::SERVICE_UNAVAILABLE,
            "Course Summary authentication unavailable",
        ))),
    }
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

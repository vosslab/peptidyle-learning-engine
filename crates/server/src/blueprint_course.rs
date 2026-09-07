//! Live Blueprint Course browse, creation, and immutable revision Server Routes.
//!
//! The Store preserves exact Question Revision pins. This route assembles only
//! current answer-free Question Library rows for an authorized Instructor.

use std::{collections::BTreeMap, str::FromStr, sync::Arc};

use axum::{
    Json, Router,
    extract::{Path, Query, State},
    http::{HeaderMap, HeaderValue, StatusCode, header::COOKIE},
    response::{IntoResponse, Response},
    routing::get,
};
use learning_data_access::{
    BlueprintCourseStore, QuestionLibraryStore, SessionTokenHash, StoreError,
    StoredBlueprintAssignmentContent, StoredBlueprintAssignmentEntry, StoredBlueprintCourse,
    postgres::{PostgresBlueprintCourseStore, PostgresQuestionLibraryStore, PostgresSessionStore},
};
use objects::s3::S3ObjectStore;
use question_model::{
    BlueprintAssignmentContentView, BlueprintAssignmentEntryView,
    BlueprintCourseAssignmentContentView, BlueprintCourseReference, BlueprintCourseSummaryView,
    BlueprintCourseView, BlueprintModuleView, BlueprintRevision, CreateBlueprintCourseContentInput,
    QuestionId, QuestionSearchResult, ReplaceBlueprintCourseContentInput, ReusablePoolView,
    ReusableQuestionPoolItemView, ReusableQuestionView, ReusableSelectionAvailability,
};
use serde::{Deserialize, Serialize};

use crate::auth::{AuthError, resolve_session};

const MAX_PAGE_SIZE: u16 = 100;

#[derive(Clone)]
struct BlueprintCourseRouteState {
    sessions: Arc<PostgresSessionStore>,
    blueprints: PostgresBlueprintCourseStore,
    question_library: PostgresQuestionLibraryStore,
    objects: S3ObjectStore,
}

/// Registers the active-Instructor Blueprint Course lifecycle routes.
pub fn blueprint_course_router(
    sessions: Arc<PostgresSessionStore>,
    blueprints: PostgresBlueprintCourseStore,
    question_library: PostgresQuestionLibraryStore,
    objects: S3ObjectStore,
) -> Router {
    Router::new()
        .route(
            "/api/course-blueprints",
            get(list_blueprints).post(create_blueprint),
        )
        .route(
            "/api/course-blueprints/{reference}",
            get(load_blueprint).put(replace_blueprint),
        )
        .with_state(BlueprintCourseRouteState {
            sessions,
            blueprints,
            question_library,
            objects,
        })
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct BlueprintCourseListQuery {
    #[serde(default)]
    cursor: Option<String>,
    #[serde(default, rename = "pageSize")]
    page_size: Option<u16>,
}

#[derive(Debug, Serialize)]
struct BlueprintCourseListResponse {
    items: Vec<BlueprintCourseSummaryView>,
    #[serde(rename = "nextCursor")]
    next_cursor: Option<String>,
}

async fn list_blueprints(
    State(state): State<BlueprintCourseRouteState>,
    headers: HeaderMap,
    Query(query): Query<BlueprintCourseListQuery>,
) -> Response {
    if query.cursor.is_some()
        || query
            .page_size
            .is_some_and(|size| size == 0 || size > MAX_PAGE_SIZE)
    {
        return route_error(StatusCode::BAD_REQUEST, "Blueprint Course page is invalid");
    }
    let session_hash = match instructor_session_hash(&state, &headers).await {
        Ok(value) => value,
        Err(response) => return *response,
    };
    match state.blueprints.list_blueprint_courses(session_hash).await {
        Ok(records) => crate::auth::no_store(
            Json(BlueprintCourseListResponse {
                items: records
                    .into_iter()
                    .map(|record| BlueprintCourseSummaryView {
                        reference: record.reference,
                        title: record.title,
                        revision: record.revision,
                        read_access: record.read_access,
                    })
                    .collect(),
                next_cursor: None,
            })
            .into_response(),
        ),
        Err(error) => store_error_response(error),
    }
}

async fn load_blueprint(
    State(state): State<BlueprintCourseRouteState>,
    headers: HeaderMap,
    Path(reference): Path<String>,
) -> Response {
    let reference = match reference.parse::<BlueprintCourseReference>() {
        Ok(value) => value,
        Err(_) => return concealed(),
    };
    let session_hash = match instructor_session_hash(&state, &headers).await {
        Ok(value) => value,
        Err(response) => return *response,
    };
    match load_view(&state, session_hash, reference).await {
        Ok(view) => revision_response(StatusCode::OK, view),
        Err(RouteLoadError::Store(error)) => store_error_response(error),
        Err(RouteLoadError::Unavailable) => route_error(
            StatusCode::SERVICE_UNAVAILABLE,
            "Blueprint Course unavailable",
        ),
    }
}

async fn create_blueprint(
    State(state): State<BlueprintCourseRouteState>,
    headers: HeaderMap,
    Json(input): Json<CreateBlueprintCourseContentInput>,
) -> Response {
    let session_hash = match instructor_session_hash(&state, &headers).await {
        Ok(value) => value,
        Err(response) => return *response,
    };
    let record = match state
        .blueprints
        .create_blueprint_course(session_hash, input)
        .await
    {
        Ok(value) => value,
        Err(error) => return store_error_response(error),
    };
    match view_from_record(&state, session_hash, record).await {
        Ok(view) => revision_response(StatusCode::CREATED, view),
        Err(RouteLoadError::Store(error)) => store_error_response(error),
        Err(RouteLoadError::Unavailable) => route_error(
            StatusCode::SERVICE_UNAVAILABLE,
            "Blueprint Course unavailable",
        ),
    }
}

async fn replace_blueprint(
    State(state): State<BlueprintCourseRouteState>,
    headers: HeaderMap,
    Path(reference): Path<String>,
    Json(input): Json<ReplaceBlueprintCourseContentInput>,
) -> Response {
    let reference = match reference.parse::<BlueprintCourseReference>() {
        Ok(value) => value,
        Err(_) => return concealed(),
    };
    let expected_revision = match expected_revision(&headers) {
        Ok(value) => value,
        Err(response) => return *response,
    };
    let session_hash = match instructor_session_hash(&state, &headers).await {
        Ok(value) => value,
        Err(response) => return *response,
    };
    let record = match state
        .blueprints
        .replace_blueprint_course(session_hash, reference, expected_revision, input)
        .await
    {
        Ok(value) => value,
        Err(StoreError::Conflict | StoreError::RetryableTransaction) => {
            return route_error(StatusCode::PRECONDITION_FAILED, "Blueprint Course changed");
        }
        Err(error) => return store_error_response(error),
    };
    match view_from_record(&state, session_hash, record).await {
        Ok(view) => revision_response(StatusCode::OK, view),
        Err(RouteLoadError::Store(error)) => store_error_response(error),
        Err(RouteLoadError::Unavailable) => route_error(
            StatusCode::SERVICE_UNAVAILABLE,
            "Blueprint Course unavailable",
        ),
    }
}

enum RouteLoadError {
    Store(StoreError),
    Unavailable,
}

async fn load_view(
    state: &BlueprintCourseRouteState,
    session_hash: SessionTokenHash,
    reference: BlueprintCourseReference,
) -> Result<BlueprintCourseView, RouteLoadError> {
    let record = state
        .blueprints
        .load_blueprint_course(session_hash, reference)
        .await
        .map_err(RouteLoadError::Store)?;
    view_from_record(state, session_hash, record).await
}

async fn view_from_record(
    state: &BlueprintCourseRouteState,
    session_hash: SessionTokenHash,
    record: StoredBlueprintCourse,
) -> Result<BlueprintCourseView, RouteLoadError> {
    let entries = state
        .question_library
        .list_published_question_library_entries(session_hash)
        .await
        .map_err(RouteLoadError::Store)?;
    let questions =
        crate::question_library::answer_free_question_search_results(&state.objects, entries)
            .await
            .map_err(|_| RouteLoadError::Unavailable)?;
    let modules = record
        .content
        .modules
        .into_iter()
        .map(|module| {
            let assignments = module
                .assignments
                .into_iter()
                .map(|assignment| {
                    let content = assignment_content_view(&assignment.content, &questions)?;
                    Ok(BlueprintCourseAssignmentContentView {
                        blueprint_assignment_reference: assignment.blueprint_assignment_reference,
                        content,
                    })
                })
                .collect::<Result<Vec<_>, RouteLoadError>>()?;
            Ok(BlueprintModuleView {
                blueprint_module_reference: module.blueprint_module_reference,
                label: module.label,
                assignments,
            })
        })
        .collect::<Result<Vec<_>, RouteLoadError>>()?;
    Ok(BlueprintCourseView {
        reference: record.reference,
        title: record.content.title,
        revision: record.revision,
        read_access: record.read_access,
        modules,
    })
}

fn assignment_content_view(
    content: &StoredBlueprintAssignmentContent,
    questions: &BTreeMap<QuestionId, QuestionSearchResult>,
) -> Result<BlueprintAssignmentContentView, RouteLoadError> {
    let entries = content
        .entries
        .iter()
        .map(|entry| match entry {
            StoredBlueprintAssignmentEntry::Fixed {
                question_revision,
                points_possible,
                scoring_rule,
                question_attempt_limit,
                question_attempt_time_limit,
            } => Ok(BlueprintAssignmentEntryView::Fixed {
                question: Box::new(question_view(question_revision, questions)?),
                points_possible: *points_possible,
                scoring_rule: *scoring_rule,
                question_attempt_limit: *question_attempt_limit,
                question_attempt_time_limit: *question_attempt_time_limit,
            }),
            StoredBlueprintAssignmentEntry::Pool {
                question_revisions,
                selection_count,
                points_per_item,
                scoring_rule,
                selection_rule,
                question_attempt_limit,
                question_attempt_time_limit,
            } => Ok(BlueprintAssignmentEntryView::Pool(ReusablePoolView {
                items: question_revisions
                    .iter()
                    .map(|reference| {
                        Ok(ReusableQuestionPoolItemView {
                            question_library: question_search_result(reference, questions)?,
                            selection_availability: selection_availability(reference, questions)?,
                        })
                    })
                    .collect::<Result<Vec<_>, RouteLoadError>>()?,
                selection_count: *selection_count,
                points_per_item: *points_per_item,
                scoring_rule: *scoring_rule,
                selection_rule: *selection_rule,
                question_attempt_limit: *question_attempt_limit,
                question_attempt_time_limit: *question_attempt_time_limit,
            })),
        })
        .collect::<Result<Vec<_>, RouteLoadError>>()?;
    Ok(BlueprintAssignmentContentView {
        title: content.title.clone(),
        instructions: content.instructions.clone(),
        entries,
        defaults: content.defaults.clone(),
        schedule: content.schedule.clone(),
    })
}

fn question_view(
    reference: &question_model::QuestionRevisionReference,
    questions: &BTreeMap<QuestionId, QuestionSearchResult>,
) -> Result<ReusableQuestionView, RouteLoadError> {
    Ok(ReusableQuestionView {
        question_library: question_search_result(reference, questions)?,
        selection_availability: selection_availability(reference, questions)?,
    })
}

fn question_search_result(
    reference: &question_model::QuestionRevisionReference,
    questions: &BTreeMap<QuestionId, QuestionSearchResult>,
) -> Result<QuestionSearchResult, RouteLoadError> {
    questions
        .get(&reference.question_id)
        .cloned()
        .ok_or(RouteLoadError::Unavailable)
}

fn selection_availability(
    reference: &question_model::QuestionRevisionReference,
    questions: &BTreeMap<QuestionId, QuestionSearchResult>,
) -> Result<ReusableSelectionAvailability, RouteLoadError> {
    let result = question_search_result(reference, questions)?;
    if result.summary.latest_question_revision == *reference
        && result
            .summary
            .availability
            .is_eligible_for_ordinary_new_selection()
    {
        Ok(ReusableSelectionAvailability::Available)
    } else {
        Ok(ReusableSelectionAvailability::Retained)
    }
}

fn expected_revision(headers: &HeaderMap) -> Result<BlueprintRevision, Box<Response>> {
    let Some(value) = headers
        .get(axum::http::header::IF_MATCH)
        .and_then(|value| value.to_str().ok())
    else {
        return Err(Box::new(route_error(
            StatusCode::PRECONDITION_REQUIRED,
            "Blueprint Revision is required",
        )));
    };
    let Some(number) = value
        .strip_prefix('"')
        .and_then(|candidate| candidate.strip_suffix('"'))
    else {
        return Err(Box::new(route_error(
            StatusCode::BAD_REQUEST,
            "Blueprint Revision is invalid",
        )));
    };
    BlueprintRevision::from_str(number).map_err(|_| {
        Box::new(route_error(
            StatusCode::BAD_REQUEST,
            "Blueprint Revision is invalid",
        ))
    })
}

fn revision_response(status: StatusCode, view: BlueprintCourseView) -> Response {
    let mut response = crate::auth::no_store((status, Json(view.clone())).into_response());
    let value = HeaderValue::from_str(&format!("\"{}\"", view.revision)).map_err(|_| ());
    match value {
        Ok(value) => {
            response
                .headers_mut()
                .insert(axum::http::header::ETAG, value);
            response
        }
        Err(()) => route_error(
            StatusCode::SERVICE_UNAVAILABLE,
            "Blueprint Course unavailable",
        ),
    }
}

async fn instructor_session_hash(
    state: &BlueprintCourseRouteState,
    headers: &HeaderMap,
) -> Result<SessionTokenHash, Box<Response>> {
    match resolve_session(
        state.sessions.as_ref(),
        joined_cookie_header(headers).as_deref(),
    )
    .await
    {
        Ok(session) if session.record.product_role == question_model::ProductRole::Instructor => {
            Ok(session.session_hash)
        }
        Ok(_) | Err(AuthError::Unauthenticated) => Err(Box::new(concealed())),
        Err(AuthError::Unavailable(_) | AuthError::Randomness(_)) => Err(Box::new(route_error(
            StatusCode::SERVICE_UNAVAILABLE,
            "Blueprint Course authentication unavailable",
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
            route_error(StatusCode::PRECONDITION_FAILED, "Blueprint Course changed")
        }
        StoreError::InvalidRecord(_) => route_error(
            StatusCode::UNPROCESSABLE_ENTITY,
            "Blueprint Course is invalid",
        ),
        StoreError::AlreadyExists => route_error(StatusCode::CONFLICT, "Blueprint Course conflict"),
        StoreError::AssignmentActivity(_) | StoreError::TimedOut | StoreError::Unavailable(_) => {
            route_error(
                StatusCode::SERVICE_UNAVAILABLE,
                "Blueprint Course unavailable",
            )
        }
    }
}

fn concealed() -> Response {
    route_error(StatusCode::NOT_FOUND, "Blueprint Course not found")
}

fn route_error(status: StatusCode, message: &'static str) -> Response {
    crate::auth::no_store((status, message).into_response())
}

//! Active-Instructor Question Star HTTP boundary.
//!
//! This route exposes only an Instructor's own Star state plus an aggregate
//! count and public display identities. It has no Watch read path and
//! serializes no account identifier, email, credential, source, or Student fact.

use std::sync::Arc;

use axum::{
    Json, Router,
    body::to_bytes,
    extract::{Path, RawQuery, Request, State},
    http::{HeaderMap, StatusCode, Uri},
    response::{IntoResponse, Response},
    routing::get,
};
use axum_extra::extract::Query;
use learning_data_access::{
    QuestionLibrarySearchScope, QuestionLibrarySearchSort, QuestionLibraryStore,
    QuestionStarProjection, QuestionStarStore, QuestionStarredInstructor, SessionTokenHash,
    StoreError,
    postgres::{PostgresQuestionLibraryStore, PostgresQuestionStarStore, PostgresSessionStore},
};
use objects::s3::S3ObjectStore;
use question_model::{
    LibraryObjectSearchRequest, LibrarySearchClassification, LibrarySearchKind,
    PublishedQuestionFilter, PublishedQuestionId, QuestionSearchFacets, QuestionSearchResult,
    UserRole,
};
use serde::{Deserialize, Serialize};

use crate::auth::{AuthError, resolve_session};

const MAX_STAR_UPDATE_BYTES: usize = 128;

#[derive(Clone)]
struct RouteState {
    sessions: Arc<PostgresSessionStore>,
    stars: PostgresQuestionStarStore,
    library: PostgresQuestionLibraryStore,
    objects: S3ObjectStore,
}

/// Registers the Question Star surface. It is intentionally separate from
/// Question Library source/detail delivery and has no Watch route.
pub fn question_stewardship_router(
    sessions: Arc<PostgresSessionStore>,
    stars: PostgresQuestionStarStore,
    library: PostgresQuestionLibraryStore,
    objects: S3ObjectStore,
) -> Router {
    Router::new()
        .route("/api/questions/stewardship/stars", get(list_stars))
        .route(
            "/api/questions/stewardship/stars/facet-choices/{field}",
            get(facet_choices::list_starred_facet_choices),
        )
        .route(
            "/api/questions/by-id/{question_id}/stewardship/star",
            get(read_star).put(set_star),
        )
        .with_state(RouteState {
            sessions,
            stars,
            library,
            objects,
        })
}

mod facet_choices;

/// Closed browser payload. The caller may choose only its own desired state.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct SetStarInput {
    starred: bool,
}

/// Browser-safe Star state. This is a closed projection: only active
/// Instructor display names can be included, never durable Account identity.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct StarResponse {
    star_count: u64,
    viewer_has_starred: bool,
    starred_instructors: Vec<StarredInstructorResponse>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct StarredInstructorResponse {
    display_name: String,
    account_id: String,
}

impl From<QuestionStarredInstructor> for StarredInstructorResponse {
    fn from(value: QuestionStarredInstructor) -> Self {
        Self {
            display_name: value.display_name,
            account_id: value.account_id.to_string(),
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct StarredQuestionListResponse {
    items: Vec<StarredQuestionItemResponse>,
    page_number: u32,
    page_size: u16,
    total_count: u64,
    facets: QuestionSearchFacets,
}

struct StarredQuestionListQuery {
    library: crate::question_library::query::LibraryObjectSearchQuery,
    starred_sort: StarredSort,
}

fn parse_starred_query(raw_query: Option<&str>) -> Result<StarredQuestionListQuery, ()> {
    let mut library_query = url::form_urlencoded::Serializer::new(String::new());
    let mut starred_sort = None;
    for (name, value) in url::form_urlencoded::parse(raw_query.unwrap_or_default().as_bytes()) {
        if name == "starredSort" {
            if starred_sort.is_some() {
                return Err(());
            }
            starred_sort = Some(
                serde_json::from_value::<StarredSort>(serde_json::Value::String(
                    value.into_owned(),
                ))
                .map_err(|_| ())?,
            );
        } else {
            library_query.append_pair(&name, &value);
        }
    }
    let uri: Uri = format!("/?{}", library_query.finish())
        .parse()
        .map_err(|_| ())?;
    let Query(library) = Query::try_from_uri(&uri).map_err(|_| ())?;
    Ok(StarredQuestionListQuery {
        library,
        starred_sort: starred_sort.unwrap_or_default(),
    })
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct StarredQuestionItemResponse {
    question: QuestionSearchResult,
    owner_account_id: question_model::AccountId,
    classification: LibrarySearchClassification,
    starred_at_micros: i64,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
enum StarredSort {
    #[default]
    StarredDateDescending,
    StarredDateAscending,
    TitleAscending,
    TitleDescending,
    DisciplineAscending,
    DisciplineDescending,
    QuestionTypeAscending,
    QuestionTypeDescending,
    PublishedNewest,
}

impl From<QuestionStarProjection> for StarResponse {
    fn from(value: QuestionStarProjection) -> Self {
        Self {
            star_count: value.star_count,
            viewer_has_starred: value.viewer_has_starred,
            starred_instructors: value
                .starred_instructors
                .into_iter()
                .map(Into::into)
                .collect(),
        }
    }
}

fn store_sort(sort: StarredSort) -> QuestionLibrarySearchSort {
    match sort {
        StarredSort::StarredDateAscending => QuestionLibrarySearchSort::StarredDateAscending,
        StarredSort::StarredDateDescending => QuestionLibrarySearchSort::StarredDateDescending,
        StarredSort::TitleAscending => QuestionLibrarySearchSort::TitleAscending,
        StarredSort::TitleDescending => QuestionLibrarySearchSort::TitleDescending,
        StarredSort::DisciplineAscending => QuestionLibrarySearchSort::DisciplineAscending,
        StarredSort::DisciplineDescending => QuestionLibrarySearchSort::DisciplineDescending,
        StarredSort::QuestionTypeAscending => QuestionLibrarySearchSort::QuestionTypeAscending,
        StarredSort::QuestionTypeDescending => QuestionLibrarySearchSort::QuestionTypeDescending,
        StarredSort::PublishedNewest => QuestionLibrarySearchSort::PublishedNewest,
    }
}

async fn list_stars(
    State(state): State<RouteState>,
    headers: HeaderMap,
    RawQuery(raw_query): RawQuery,
) -> Response {
    let Ok(query) = parse_starred_query(raw_query.as_deref()) else {
        return route_error(StatusCode::BAD_REQUEST, "Starred Question query is invalid");
    };
    let sort = query.starred_sort;
    let mut public_query = match LibraryObjectSearchRequest::try_from(query.library) {
        Ok(value) => value,
        Err(_) => return route_error(StatusCode::BAD_REQUEST, "Starred Question query is invalid"),
    };
    public_query.kind = LibrarySearchKind::Questions;
    public_query.questions = PublishedQuestionFilter::All;
    let (session, _) = match instructor_session(&state, &headers).await {
        Ok(value) => value,
        Err(response) => return *response,
    };
    let mut request = crate::question_library::mixed_search::store_search_request(&public_query);
    request.scope = QuestionLibrarySearchScope::CurrentStarred;
    request.sort = store_sort(sort);
    let page = match state
        .library
        .search_published_question_library_entries(session, request)
        .await
    {
        Ok(value) => value,
        Err(error) => return store_error_response(error),
    };
    let starred_dates = page
        .page
        .items
        .iter()
        .map(|entry| entry.starred_at_micros())
        .collect::<Vec<_>>();
    let page_number = page.page.page_number.get();
    let page_size = page.page.page_size.get();
    let total_count = page.page.total_count;
    let (results, facets) = match crate::question_library::mixed_search::materialize_search_page(
        &state.library,
        &state.objects,
        session,
        true,
        page,
    )
    .await
    {
        Ok(value) => value,
        Err(response) => return *response,
    };
    let mut items = Vec::with_capacity(results.len());
    for (result, starred_at_micros) in results.into_iter().zip(starred_dates) {
        let (question, owner_account_id, classification) = match result {
            question_model::LibrarySearchResult::Question {
                question,
                owner_account_id,
                classification,
            } => (question, owner_account_id, classification),
            question_model::LibrarySearchResult::Pool { .. } => {
                return route_error(
                    StatusCode::SERVICE_UNAVAILABLE,
                    "Starred Questions unavailable",
                );
            }
        };
        let Some(starred_at_micros) = starred_at_micros else {
            return route_error(
                StatusCode::SERVICE_UNAVAILABLE,
                "Starred Questions unavailable",
            );
        };
        items.push(StarredQuestionItemResponse {
            question: *question,
            owner_account_id,
            classification,
            starred_at_micros,
        });
    }
    crate::auth::no_store(
        Json(StarredQuestionListResponse {
            items,
            page_number,
            page_size,
            total_count,
            facets,
        })
        .into_response(),
    )
}

async fn read_star(
    State(state): State<RouteState>,
    headers: HeaderMap,
    Path(raw_question_id): Path<String>,
) -> Response {
    let question_id = match verified_question_id(&raw_question_id) {
        Some(value) => value,
        None => return concealed(),
    };
    let session = match instructor_session_hash(&state, &headers).await {
        Ok(value) => value,
        Err(response) => return *response,
    };
    match state
        .stars
        .question_star_projection(session, &question_id)
        .await
    {
        Ok(projection) => {
            crate::auth::no_store(Json(StarResponse::from(projection)).into_response())
        }
        Err(error) => store_error_response(error),
    }
}

async fn set_star(
    State(state): State<RouteState>,
    Path(raw_question_id): Path<String>,
    request: Request,
) -> Response {
    let question_id = match verified_question_id(&raw_question_id) {
        Some(value) => value,
        None => return concealed(),
    };
    let session = match instructor_session_hash(&state, request.headers()).await {
        Ok(value) => value,
        Err(response) => return *response,
    };
    if !has_content_type(request.headers(), "application/json") {
        return route_error(
            StatusCode::UNSUPPORTED_MEDIA_TYPE,
            "Question Star is invalid",
        );
    }
    let input = match to_bytes(request.into_body(), MAX_STAR_UPDATE_BYTES).await {
        Ok(bytes) => match serde_json::from_slice::<SetStarInput>(&bytes) {
            Ok(value) => value,
            Err(_) => {
                return route_error(StatusCode::UNPROCESSABLE_ENTITY, "Question Star is invalid");
            }
        },
        Err(_) => return route_error(StatusCode::PAYLOAD_TOO_LARGE, "Question Star is too large"),
    };
    match state
        .stars
        .set_current_question_star(session, &question_id, input.starred)
        .await
    {
        Ok(projection) => {
            crate::auth::no_store(Json(StarResponse::from(projection)).into_response())
        }
        Err(error) => store_error_response(error),
    }
}

/// Parses the exact checksum-bearing ID before any persistence lookup.
/// ASVS 2.2.1--2.2.2: only an exact canonical Question identity reaches the
/// Star Store.
fn verified_question_id(value: &str) -> Option<PublishedQuestionId> {
    value.parse::<PublishedQuestionId>().ok()
}

async fn instructor_session_hash(
    state: &RouteState,
    headers: &HeaderMap,
) -> Result<SessionTokenHash, Box<Response>> {
    instructor_session(state, headers)
        .await
        .map(|(hash, _)| hash)
}

async fn instructor_session(
    state: &RouteState,
    headers: &HeaderMap,
) -> Result<(SessionTokenHash, String), Box<Response>> {
    match resolve_session(
        state.sessions.as_ref(),
        crate::auth::joined_cookie_header(headers).as_deref(),
    )
    .await
    {
        // ASVS 8.2.1 and 8.3.1: role authorization is server-derived. The
        // subsequent database procedure independently rechecks active status.
        Ok(session) if session.record.user_role == UserRole::Instructor => {
            Ok((session.session_hash, session.record.account.to_string()))
        }
        Ok(_) | Err(AuthError::Unauthenticated) => Err(Box::new(concealed())),
        Err(AuthError::Unavailable(_) | AuthError::Randomness(_)) => Err(Box::new(route_error(
            StatusCode::SERVICE_UNAVAILABLE,
            "Question Star authentication unavailable",
        ))),
    }
}

fn has_content_type(headers: &HeaderMap, expected: &str) -> bool {
    headers
        .get("content-type")
        .and_then(|value| value.to_str().ok())
        .is_some_and(|value| {
            value
                .split(';')
                .next()
                .is_some_and(|media_type| media_type.trim().eq_ignore_ascii_case(expected))
        })
}

fn store_error_response(error: StoreError) -> Response {
    match error {
        StoreError::NotFound | StoreError::Forbidden | StoreError::OwnershipMismatch => concealed(),
        StoreError::InvalidRecord(_) => {
            route_error(StatusCode::UNPROCESSABLE_ENTITY, "Question Star is invalid")
        }
        StoreError::Conflict | StoreError::RetryableTransaction => {
            route_error(StatusCode::PRECONDITION_FAILED, "Question Star changed")
        }
        StoreError::LifecycleConflict | StoreError::AlreadyExists => {
            route_error(StatusCode::CONFLICT, "Question Star conflict")
        }
        StoreError::AssessmentActivity(_)
        | StoreError::TimedOut
        | StoreError::LeaseLost
        | StoreError::Unavailable(_) => {
            route_error(StatusCode::SERVICE_UNAVAILABLE, "Question Star unavailable")
        }
    }
}

fn concealed() -> Response {
    route_error(StatusCode::NOT_FOUND, "Question Star not found")
}

fn route_error(status: StatusCode, message: &'static str) -> Response {
    crate::auth::no_store((status, message).into_response())
}

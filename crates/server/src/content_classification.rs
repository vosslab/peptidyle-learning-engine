//! Installed-session classification selectors and Sysadmin Discipline lifecycle routes.

use std::sync::Arc;

use axum::{
    Json, Router,
    extract::{Path, Query, State, rejection::QueryRejection},
    http::{HeaderMap, StatusCode, header::COOKIE},
    response::{IntoResponse, Response},
    routing::{get, post},
};
use learning_data_access::{
    ContentClassificationItem, ContentClassificationStore, ContentDiscipline,
    ContentDisciplineAdministrationStore, SessionTokenHash, StoreError,
    postgres::{PostgresContentClassificationStore, PostgresSessionStore},
};
use question_model::ProductRole;
use serde::Deserialize;
use uuid::Uuid;

use crate::auth::{AuthError, resolve_session};

#[derive(Clone)]
struct RouteState {
    sessions: Arc<PostgresSessionStore>,
    selections: Arc<dyn ContentClassificationStore>,
    disciplines: Arc<dyn ContentDisciplineAdministrationStore>,
}

pub fn content_classification_router(
    sessions: Arc<PostgresSessionStore>,
    store: PostgresContentClassificationStore,
) -> Router {
    Router::new()
        .route(
            "/api/content-classification/disciplines",
            get(disciplines).post(create_discipline),
        )
        .route(
            "/api/content-classification/disciplines/discovery",
            get(disciplines_including_retired),
        )
        .route(
            "/api/content-classification/disciplines/{discipline_uuid}/rename",
            post(rename_discipline),
        )
        .route(
            "/api/content-classification/disciplines/{discipline_uuid}/retire",
            post(retire_discipline),
        )
        .route(
            "/api/content-classification/disciplines/{discipline_uuid}/restore",
            post(restore_discipline),
        )
        .route("/api/content-classification/subjects", get(subjects))
        .route("/api/content-classification/topics", get(topics))
        .route("/api/content-classification/subtopics", get(subtopics))
        .with_state(RouteState {
            sessions,
            selections: Arc::new(store.clone()),
            disciplines: Arc::new(store),
        })
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct EmptyQuery {}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct SubjectQuery {
    discipline_uuid: String,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct TopicQuery {
    subject_uuid: String,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct SubtopicQuery {
    topic_uuid: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct DisciplineNameInput {
    name: String,
}

async fn disciplines(
    State(state): State<RouteState>,
    headers: HeaderMap,
    query: Result<Query<EmptyQuery>, QueryRejection>,
) -> Response {
    let token = match reader_token(&state, &headers).await {
        Ok(token) => token,
        Err(response) => return *response,
    };
    if query.is_err() {
        return invalid();
    }
    list_response(
        "disciplines",
        state.selections.list_disciplines(token).await,
    )
}

async fn disciplines_including_retired(
    State(state): State<RouteState>,
    headers: HeaderMap,
    query: Result<Query<EmptyQuery>, QueryRejection>,
) -> Response {
    let token = match reader_token(&state, &headers).await {
        Ok(token) => token,
        Err(response) => return *response,
    };
    if query.is_err() {
        return invalid();
    }
    discipline_list_response(
        "disciplines",
        state
            .disciplines
            .list_disciplines_including_retired(token)
            .await,
    )
}

async fn create_discipline(
    State(state): State<RouteState>,
    headers: HeaderMap,
    Json(input): Json<DisciplineNameInput>,
) -> Response {
    let token = match sysadmin_token(&state, &headers).await {
        Ok(token) => token,
        Err(response) => return *response,
    };
    discipline_response(
        state.disciplines.create_discipline(token, input.name).await,
        StatusCode::CREATED,
    )
}

async fn rename_discipline(
    State(state): State<RouteState>,
    headers: HeaderMap,
    Path(discipline_uuid): Path<String>,
    Json(input): Json<DisciplineNameInput>,
) -> Response {
    let Some(discipline_uuid) = canonical_uuid(&discipline_uuid) else {
        return concealed();
    };
    let token = match sysadmin_token(&state, &headers).await {
        Ok(token) => token,
        Err(response) => return *response,
    };
    discipline_response(
        state
            .disciplines
            .rename_discipline(token, discipline_uuid, input.name)
            .await,
        StatusCode::OK,
    )
}

async fn retire_discipline(
    State(state): State<RouteState>,
    headers: HeaderMap,
    Path(discipline_uuid): Path<String>,
) -> Response {
    discipline_state_response(state, headers, discipline_uuid, true).await
}

async fn restore_discipline(
    State(state): State<RouteState>,
    headers: HeaderMap,
    Path(discipline_uuid): Path<String>,
) -> Response {
    discipline_state_response(state, headers, discipline_uuid, false).await
}

async fn discipline_state_response(
    state: RouteState,
    headers: HeaderMap,
    discipline_uuid: String,
    retire: bool,
) -> Response {
    let Some(discipline_uuid) = canonical_uuid(&discipline_uuid) else {
        return concealed();
    };
    let token = match sysadmin_token(&state, &headers).await {
        Ok(token) => token,
        Err(response) => return *response,
    };
    let result = if retire {
        state
            .disciplines
            .retire_discipline(token, discipline_uuid)
            .await
    } else {
        state
            .disciplines
            .restore_discipline(token, discipline_uuid)
            .await
    };
    discipline_response(result, StatusCode::OK)
}

async fn subjects(
    State(state): State<RouteState>,
    headers: HeaderMap,
    query: Result<Query<SubjectQuery>, QueryRejection>,
) -> Response {
    let token = match reader_token(&state, &headers).await {
        Ok(token) => token,
        Err(response) => return *response,
    };
    let parent = query
        .ok()
        .and_then(|Query(query)| canonical_uuid(&query.discipline_uuid));
    let Some(parent) = parent else {
        return invalid();
    };
    list_response(
        "subjects",
        state.selections.list_subjects(token, parent).await,
    )
}

async fn topics(
    State(state): State<RouteState>,
    headers: HeaderMap,
    query: Result<Query<TopicQuery>, QueryRejection>,
) -> Response {
    let token = match reader_token(&state, &headers).await {
        Ok(token) => token,
        Err(response) => return *response,
    };
    let parent = query
        .ok()
        .and_then(|Query(query)| canonical_uuid(&query.subject_uuid));
    let Some(parent) = parent else {
        return invalid();
    };
    list_response("topics", state.selections.list_topics(token, parent).await)
}

async fn subtopics(
    State(state): State<RouteState>,
    headers: HeaderMap,
    query: Result<Query<SubtopicQuery>, QueryRejection>,
) -> Response {
    let token = match reader_token(&state, &headers).await {
        Ok(token) => token,
        Err(response) => return *response,
    };
    let parent = query
        .ok()
        .and_then(|Query(query)| canonical_uuid(&query.topic_uuid));
    let Some(parent) = parent else {
        return invalid();
    };
    list_response(
        "subtopics",
        state.selections.list_subtopics(token, parent).await,
    )
}

fn canonical_uuid(value: &str) -> Option<Uuid> {
    // ASVS 2.2.1: accept only canonical lowercase-hyphenated UUID inputs.
    let uuid = Uuid::parse_str(value).ok()?;
    (uuid.to_string() == value).then_some(uuid)
}

async fn reader_token(
    state: &RouteState,
    headers: &HeaderMap,
) -> Result<SessionTokenHash, Box<Response>> {
    let cookies = headers
        .get_all(COOKIE)
        .iter()
        .map(|value| value.to_str().ok())
        .collect::<Option<Vec<_>>>()
        .map(|values| values.join("; "));
    // ASVS 8.2.1: trusted session role only; SQL rechecks active/vetted authority.
    match resolve_session(state.sessions.as_ref(), cookies.as_deref()).await {
        Ok(session)
            if matches!(
                session.record.product_role,
                ProductRole::Instructor | ProductRole::Sysadmin
            ) =>
        {
            Ok(session.session_hash)
        }
        Ok(_) | Err(AuthError::Unauthenticated) => Err(Box::new(concealed())),
        Err(AuthError::Unavailable(_) | AuthError::Randomness(_)) => Err(Box::new(unavailable())),
    }
}

async fn sysadmin_token(
    state: &RouteState,
    headers: &HeaderMap,
) -> Result<SessionTokenHash, Box<Response>> {
    let cookies = headers
        .get_all(COOKIE)
        .iter()
        .map(|value| value.to_str().ok())
        .collect::<Option<Vec<_>>>()
        .map(|values| values.join("; "));
    match resolve_session(state.sessions.as_ref(), cookies.as_deref()).await {
        Ok(session) if session.record.product_role == ProductRole::Sysadmin => {
            Ok(session.session_hash)
        }
        Ok(_) | Err(AuthError::Unauthenticated) => Err(Box::new(concealed())),
        Err(AuthError::Unavailable(_) | AuthError::Randomness(_)) => Err(Box::new(unavailable())),
    }
}

fn list_response(
    key: &'static str,
    result: Result<Vec<ContentClassificationItem>, StoreError>,
) -> Response {
    match result {
        Ok(items) => {
            let items = items
                .into_iter()
                .map(|item| item_json(item, false))
                .collect::<Vec<_>>();
            crate::auth::no_store(Json(serde_json::json!({ (key): items })).into_response())
        }
        Err(StoreError::NotFound | StoreError::Forbidden | StoreError::OwnershipMismatch) => {
            concealed()
        }
        Err(_) => unavailable(),
    }
}
fn discipline_list_response(
    key: &'static str,
    result: Result<Vec<ContentDiscipline>, StoreError>,
) -> Response {
    match result {
        Ok(items) => {
            let items = items.into_iter().map(discipline_json).collect::<Vec<_>>();
            crate::auth::no_store(Json(serde_json::json!({ (key): items })).into_response())
        }
        Err(StoreError::NotFound | StoreError::Forbidden | StoreError::OwnershipMismatch) => {
            concealed()
        }
        Err(_) => unavailable(),
    }
}
fn discipline_response(
    result: Result<ContentDiscipline, StoreError>,
    status: StatusCode,
) -> Response {
    match result {
        Ok(item) => crate::auth::no_store((status, Json(discipline_json(item))).into_response()),
        Err(StoreError::NotFound | StoreError::Forbidden | StoreError::OwnershipMismatch) => {
            concealed()
        }
        Err(StoreError::AlreadyExists) => {
            error(StatusCode::CONFLICT, "Content Discipline conflicts")
        }
        Err(StoreError::InvalidRecord(_)) => invalid(),
        Err(_) => unavailable(),
    }
}
fn item_json(item: ContentClassificationItem, is_retired: bool) -> serde_json::Value {
    serde_json::json!({
        "uuid": item.uuid.to_string(),
        "name": item.name,
        "isRetired": is_retired,
    })
}
fn discipline_json(item: ContentDiscipline) -> serde_json::Value {
    serde_json::json!({
        "uuid": item.uuid.to_string(),
        "name": item.name,
        "isRetired": item.is_retired,
    })
}
fn invalid() -> Response {
    error(
        StatusCode::UNPROCESSABLE_ENTITY,
        "Content classification query is invalid",
    )
}
fn concealed() -> Response {
    error(StatusCode::NOT_FOUND, "Content classification not found")
}
fn unavailable() -> Response {
    error(
        StatusCode::SERVICE_UNAVAILABLE,
        "Content classification is unavailable",
    )
}
fn error(status: StatusCode, message: &'static str) -> Response {
    // ASVS 16.5.1: database failures never expose queries, tokens, or internals.
    crate::auth::no_store((status, Json(serde_json::json!({"error": message}))).into_response())
}

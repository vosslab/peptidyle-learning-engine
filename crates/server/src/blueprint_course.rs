//! Blueprint Revision, lineage metadata, and availability routes.
//!
//! Browser Question IDs are checksum-validated before Store resolution. The Store
//! alone resolves the exact immutable Question Revision pins.

use std::sync::Arc;

use axum::{
    Json, Router,
    extract::{Path, State},
    http::{
        HeaderMap, StatusCode,
        header::{COOKIE, IF_MATCH},
    },
    response::{IntoResponse, Response},
    routing::{get, post, put},
};
use browser_api_contract::blueprint_course::BlueprintRevisionView;
use learning_data_access::{
    BlueprintCourseStore, SessionTokenHash, StoreError,
    postgres::{
        PostgresBlueprintCourseStore, PostgresBlueprintLineageStore,
        PostgresCourseBlueprintPublicationStore, PostgresQuestionLibraryStore,
        PostgresSessionStore,
    },
};
use objects::s3::S3ObjectStore;
use question_model::{
    BlueprintCourseId, BlueprintCourseView, BlueprintEditNumber, BlueprintRevisionNumber,
    BlueprintRevisionTuple, CreateBlueprintCourseInput, RenameBlueprintCourseInput,
    ReplaceBlueprintCourseContentInput, RequestChecksum,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::auth::{AuthError, resolve_session};

mod change_proposal_view;
mod change_proposals;
mod course_publication;
mod exchange;
mod fork;
mod fork_apply;
mod fork_review;
mod history;
mod known_forks;
mod list;
mod pool_members;
mod promotion;
mod responses;
mod views;

use responses::{
    blueprint_response, blueprint_save_response, concealed, metadata_response, route_error,
    store_error_response, unavailable,
};
use views::{
    content_modules, summary_view, valid_create_question_ids, valid_replace_question_ids,
    view_from_record,
};

use fork::fork_blueprint;

const MAX_IDEMPOTENCY_KEY_BYTES: usize = 128;

#[derive(Clone)]
pub(super) struct BlueprintCourseRouteState {
    pub(super) sessions: Arc<PostgresSessionStore>,
    pub(super) blueprints: PostgresBlueprintCourseStore,
    pub(super) course_publication: PostgresCourseBlueprintPublicationStore,
    pub(super) lineage: PostgresBlueprintLineageStore,
    pub(super) question_library: PostgresQuestionLibraryStore,
    pub(super) objects: S3ObjectStore,
}

/// Registers the Instructor Blueprint lifecycle.
pub fn blueprint_course_router(
    sessions: Arc<PostgresSessionStore>,
    blueprints: PostgresBlueprintCourseStore,
    course_publication: PostgresCourseBlueprintPublicationStore,
    lineage: PostgresBlueprintLineageStore,
    question_library: PostgresQuestionLibraryStore,
    objects: S3ObjectStore,
) -> Router {
    Router::new()
        .route(
            "/api/course-blueprints/{blueprint_course_id}/change-proposals",
            get(change_proposals::list_target).post(change_proposals::create),
        )
        .route(
            "/api/blueprint-change-proposals",
            get(change_proposals::list_mine),
        )
        .route(
            "/api/blueprint-change-proposals/{proposal_id}",
            get(change_proposals::detail),
        )
        .route(
            "/api/blueprint-change-proposals/{proposal_id}/acceptance",
            post(change_proposals::accept),
        )
        .route(
            "/api/sysadmin/course-blueprints/{blueprint_course_id}/promotion",
            get(promotion::load_promotion).put(promotion::set_promotion),
        )
        .route(
            "/api/course-blueprints",
            get(list::list_blueprints).post(create_blueprint),
        )
        .route(
            "/api/course-instances/{course_instance_id}/course-blueprints",
            post(course_publication::create),
        )
        .route("/api/course-blueprints/import", post(exchange::import))
        .route(
            "/api/course-blueprints/{blueprint_course_id}",
            get(load_blueprint).put(save_blueprint),
        )
        .route(
            "/api/course-blueprints/{blueprint_course_id}/export",
            get(exchange::export),
        )
        .route(
            "/api/course-blueprints/{left_blueprint_course_id}/compare/{right_blueprint_course_id}",
            get(fork_review::load_comparison),
        )
        .route(
            "/api/course-blueprints/{blueprint_course_id}/fork-update",
            post(fork_apply::apply_fork_update),
        )
        .route(
            "/api/course-blueprints/{blueprint_course_id}/forks",
            get(known_forks::list_known_forks),
        )
        .route(
            "/api/course-blueprints/{blueprint_course_id}/history",
            get(history::list_history),
        )
        .route(
            "/api/course-blueprints/{blueprint_course_id}/assessments/{assessment_id}/pools/{question_pool_id}/members",
            get(pool_members::load_pool_members),
        )
        .route(
            "/api/course-blueprints/{blueprint_course_id}/metadata",
            put(rename_blueprint),
        )
        .route(
            "/api/course-blueprints/{blueprint_course_id}/classification",
            put(update_classification),
        )
        .route(
            "/api/course-blueprints/{blueprint_course_id}/publish",
            post(publish_blueprint),
        )
        .route(
            "/api/course-blueprints/{blueprint_course_id}/revisions/{revision_number}",
            get(load_revision),
        )
        .route(
            "/api/course-blueprints/{blueprint_course_id}/revisions/{revision_number}/fork",
            post(fork_blueprint),
        )
        .route(
            "/api/course-blueprints/{blueprint_course_id}/archive",
            post(archive_blueprint),
        )
        .route(
            "/api/course-blueprints/{blueprint_course_id}/restore",
            post(restore_blueprint),
        )
        .route(
            "/api/course-blueprints/{blueprint_course_id}/return-to-private",
            post(return_blueprint_to_private),
        )
        .with_state(BlueprintCourseRouteState {
            sessions,
            blueprints,
            course_publication,
            lineage,
            question_library,
            objects,
        })
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct ArchiveBlueprintRequest {
    confirmation_long_name: String,
}

async fn load_blueprint(
    State(state): State<BlueprintCourseRouteState>,
    headers: HeaderMap,
    Path(blueprint_course_id): Path<String>,
) -> Response {
    let blueprint_course_id = match parse_blueprint_course_id(&blueprint_course_id) {
        Ok(value) => value,
        Err(response) => return *response,
    };
    let session = match instructor_session_hash(&state, &headers).await {
        Ok(value) => value,
        Err(response) => return *response,
    };
    match load_view(&state, session, blueprint_course_id).await {
        Ok(view) => blueprint_response(StatusCode::OK, view),
        Err(RouteLoadError::Store(error)) => store_error_response(error),
        Err(RouteLoadError::Unavailable) => unavailable(),
    }
}

async fn update_classification(
    State(state): State<BlueprintCourseRouteState>,
    headers: HeaderMap,
    Path(blueprint_course_id): Path<String>,
    Json(classification): Json<question_model::CourseClassification>,
) -> Response {
    let blueprint_course_id = match parse_blueprint_course_id(&blueprint_course_id) {
        Ok(value) => value,
        Err(response) => return *response,
    };
    let expected = match expected_edit_number(&headers) {
        Ok(value) => value,
        Err(response) => return *response,
    };
    let session = match instructor_session_hash(&state, &headers).await {
        Ok(value) => value,
        Err(response) => return *response,
    };
    match state
        .blueprints
        .update_blueprint_classification(session, blueprint_course_id, expected, classification)
        .await
    {
        Ok(value) => metadata_response(value),
        Err(error) => store_error_response(error),
    }
}

async fn create_blueprint(
    State(state): State<BlueprintCourseRouteState>,
    headers: HeaderMap,
    Json(input): Json<CreateBlueprintCourseInput>,
) -> Response {
    if !valid_create_question_ids(&input) {
        return concealed();
    }
    let checksum = match request_checksum("create-blueprint-course", &headers, &input) {
        Ok(value) => value,
        Err(response) => return *response,
    };
    let session = match instructor_session_hash(&state, &headers).await {
        Ok(value) => value,
        Err(response) => return *response,
    };
    let receipt = match state
        .blueprints
        .create_blueprint_course(session, checksum, input, Default::default())
        .await
    {
        Ok(value) => value,
        Err(error) => return store_error_response(error),
    };
    match load_view(
        &state,
        session,
        receipt.blueprint_revision_tuple.blueprint_course_id,
    )
    .await
    {
        Ok(view) => blueprint_response(StatusCode::CREATED, view),
        Err(RouteLoadError::Store(error)) => store_error_response(error),
        Err(RouteLoadError::Unavailable) => unavailable(),
    }
}

async fn save_blueprint(
    State(state): State<BlueprintCourseRouteState>,
    headers: HeaderMap,
    Path(blueprint_course_id): Path<String>,
    Json(input): Json<ReplaceBlueprintCourseContentInput>,
) -> Response {
    let blueprint_course_id = match parse_blueprint_course_id(&blueprint_course_id) {
        Ok(value) => value,
        Err(response) => return *response,
    };
    if !valid_replace_question_ids(&input) {
        return concealed();
    }
    let expected = match expected_revision_number(&headers) {
        Ok(value) => value,
        Err(response) => return *response,
    };
    let checksum = match request_checksum(
        "save-blueprint-course",
        &headers,
        &(blueprint_course_id.clone(), expected, &input),
    ) {
        Ok(value) => value,
        Err(response) => return *response,
    };
    let session = match instructor_session_hash(&state, &headers).await {
        Ok(value) => value,
        Err(response) => return *response,
    };
    match state
        .blueprints
        .save_blueprint_course(
            session,
            blueprint_course_id.clone(),
            expected,
            checksum,
            input,
            Default::default(),
        )
        .await
    {
        Ok(receipt) => match load_view(&state, session, blueprint_course_id).await {
            Ok(view) => blueprint_save_response(view, receipt.changed),
            Err(RouteLoadError::Store(error)) => store_error_response(error),
            Err(RouteLoadError::Unavailable) => unavailable(),
        },
        Err(error) => store_error_response(error),
    }
}

async fn rename_blueprint(
    State(state): State<BlueprintCourseRouteState>,
    headers: HeaderMap,
    Path(blueprint_course_id): Path<String>,
    Json(input): Json<RenameBlueprintCourseInput>,
) -> Response {
    let blueprint_course_id = match parse_blueprint_course_id(&blueprint_course_id) {
        Ok(value) => value,
        Err(response) => return *response,
    };
    let expected = match expected_edit_number(&headers) {
        Ok(value) => value,
        Err(response) => return *response,
    };
    let session = match instructor_session_hash(&state, &headers).await {
        Ok(value) => value,
        Err(response) => return *response,
    };
    match state
        .blueprints
        .rename_blueprint_course(session, blueprint_course_id, expected, input)
        .await
    {
        Ok(value) => metadata_response(value),
        Err(error) => store_error_response(error),
    }
}

async fn load_revision(
    State(state): State<BlueprintCourseRouteState>,
    headers: HeaderMap,
    Path((blueprint_course_id, revision_number)): Path<(String, String)>,
) -> Response {
    let blueprint_course_id = match parse_blueprint_course_id(&blueprint_course_id) {
        Ok(value) => value,
        Err(response) => return *response,
    };
    let revision_number = match revision_number.parse::<BlueprintRevisionNumber>() {
        Ok(value) => value,
        Err(_) => return concealed(),
    };
    let session = match instructor_session_hash(&state, &headers).await {
        Ok(value) => value,
        Err(response) => return *response,
    };
    let blueprint_revision_tuple = BlueprintRevisionTuple {
        blueprint_course_id,
        revision_number,
    };
    let record = match state
        .blueprints
        .load_blueprint_revision(session, blueprint_revision_tuple.clone())
        .await
    {
        Ok(value) => value,
        Err(error) => return store_error_response(error),
    };
    match content_modules(&state, session, &record.content).await {
        Ok(modules) => crate::auth::no_store(
            Json(BlueprintRevisionView {
                blueprint_revision_tuple,
                modules,
            })
            .into_response(),
        ),
        Err(RouteLoadError::Store(error)) => store_error_response(error),
        Err(RouteLoadError::Unavailable) => unavailable(),
    }
}

async fn archive_blueprint(
    State(state): State<BlueprintCourseRouteState>,
    headers: HeaderMap,
    Path(blueprint_course_id): Path<String>,
    Json(input): Json<ArchiveBlueprintRequest>,
) -> Response {
    transition_availability(
        &state,
        &headers,
        blueprint_course_id,
        BlueprintLifecycleAction::Archive {
            confirmation_long_name: input.confirmation_long_name,
        },
    )
    .await
}
async fn publish_blueprint(
    State(state): State<BlueprintCourseRouteState>,
    headers: HeaderMap,
    Path(blueprint_course_id): Path<String>,
) -> Response {
    transition_availability(
        &state,
        &headers,
        blueprint_course_id,
        BlueprintLifecycleAction::Publish,
    )
    .await
}
async fn restore_blueprint(
    State(state): State<BlueprintCourseRouteState>,
    headers: HeaderMap,
    Path(blueprint_course_id): Path<String>,
) -> Response {
    transition_availability(
        &state,
        &headers,
        blueprint_course_id,
        BlueprintLifecycleAction::Restore,
    )
    .await
}
async fn return_blueprint_to_private(
    State(state): State<BlueprintCourseRouteState>,
    headers: HeaderMap,
    Path(blueprint_course_id): Path<String>,
) -> Response {
    transition_availability(
        &state,
        &headers,
        blueprint_course_id,
        BlueprintLifecycleAction::ReturnToPrivate,
    )
    .await
}

/// The HTTP surface names the only four lifecycle commands.  It deliberately
/// does not accept a client-selected availability string: C49 is the complete
/// state machine, and its Store transaction owns the owner/adoption predicates
/// (ASVS 2.2.1, 2.3.1, 8.2.2, 8.3.1).
enum BlueprintLifecycleAction {
    Publish,
    Archive { confirmation_long_name: String },
    Restore,
    ReturnToPrivate,
}

async fn transition_availability(
    state: &BlueprintCourseRouteState,
    headers: &HeaderMap,
    value: String,
    action: BlueprintLifecycleAction,
) -> Response {
    let blueprint_course_id = match parse_blueprint_course_id(&value) {
        Ok(value) => value,
        Err(response) => return *response,
    };
    let expected = match expected_edit_number(headers) {
        Ok(value) => value,
        Err(response) => return *response,
    };
    let session = match instructor_session_hash(state, headers).await {
        Ok(value) => value,
        Err(response) => return *response,
    };
    let result = match action {
        BlueprintLifecycleAction::Publish => {
            state
                .blueprints
                .publish_blueprint(session, blueprint_course_id, expected)
                .await
        }
        BlueprintLifecycleAction::Archive {
            confirmation_long_name,
        } => {
            state
                .blueprints
                .archive_blueprint(
                    session,
                    blueprint_course_id,
                    expected,
                    &confirmation_long_name,
                )
                .await
        }
        BlueprintLifecycleAction::Restore => {
            state
                .blueprints
                .restore_blueprint(session, blueprint_course_id, expected)
                .await
        }
        BlueprintLifecycleAction::ReturnToPrivate => {
            state
                .blueprints
                .return_blueprint_to_private(session, blueprint_course_id, expected)
                .await
        }
    };
    match result {
        Ok(value) => metadata_response(value),
        Err(error) => store_error_response(error),
    }
}

pub(super) enum RouteLoadError {
    Store(StoreError),
    Unavailable,
}
pub(super) async fn load_view(
    state: &BlueprintCourseRouteState,
    session: SessionTokenHash,
    blueprint_course_id: BlueprintCourseId,
) -> Result<BlueprintCourseView, RouteLoadError> {
    view_from_record(
        state,
        session,
        state
            .blueprints
            .load_blueprint_course(session, blueprint_course_id)
            .await
            .map_err(RouteLoadError::Store)?,
    )
    .await
}

pub(super) fn parse_blueprint_course_id(value: &str) -> Result<BlueprintCourseId, Box<Response>> {
    value.parse().map_err(|_| Box::new(concealed()))
}
fn quoted_if_match(headers: &HeaderMap) -> Result<&str, Box<Response>> {
    let Some(value) = headers.get(IF_MATCH).and_then(|value| value.to_str().ok()) else {
        return Err(Box::new(route_error(
            StatusCode::PRECONDITION_REQUIRED,
            "Blueprint precondition is required",
        )));
    };
    let Some(number) = value
        .strip_prefix('"')
        .and_then(|candidate| candidate.strip_suffix('"'))
    else {
        return Err(Box::new(route_error(
            StatusCode::BAD_REQUEST,
            "Blueprint precondition is invalid",
        )));
    };
    Ok(number)
}
fn expected_revision_number(headers: &HeaderMap) -> Result<BlueprintRevisionNumber, Box<Response>> {
    quoted_if_match(headers)?.parse().map_err(|_| {
        Box::new(route_error(
            StatusCode::BAD_REQUEST,
            "Blueprint Revision ETag is invalid",
        ))
    })
}
fn expected_edit_number(headers: &HeaderMap) -> Result<BlueprintEditNumber, Box<Response>> {
    quoted_if_match(headers)?.parse().map_err(|_| {
        Box::new(route_error(
            StatusCode::BAD_REQUEST,
            "Blueprint metadata ETag is invalid",
        ))
    })
}
// The operation name, standard transport key, and exact serialized command
// form one receipt key; a new deliberate operation uses a new key (ASVS 2.3.1).
pub(super) fn request_checksum<T: Serialize>(
    operation: &'static str,
    headers: &HeaderMap,
    payload: &T,
) -> Result<RequestChecksum, Box<Response>> {
    let key = headers
        .get("idempotency-key")
        .and_then(|value| value.to_str().ok())
        .filter(|value| valid_idempotency_key(value))
        .ok_or_else(|| {
            Box::new(route_error(
                StatusCode::BAD_REQUEST,
                "Idempotency-Key is required",
            ))
        })?;
    let payload = serde_json::to_vec(payload).map_err(|_| Box::new(unavailable()))?;
    let mut digest = Sha256::new();
    digest.update(operation.as_bytes());
    digest.update([0]);
    digest.update(key.as_bytes());
    digest.update([0]);
    digest.update(payload);
    Ok(RequestChecksum::from_bytes(digest.finalize().into()))
}
fn valid_idempotency_key(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= MAX_IDEMPOTENCY_KEY_BYTES
        && value.bytes().all(|byte| byte.is_ascii_graphic())
}
pub(super) async fn instructor_session_hash(
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::question_publication::{QuestionIdIssuer, RandomQuestionIdIssuer};
    use axum::http::{HeaderValue, StatusCode};
    use learning_data_access::StoreError;
    use question_model::{QuestionId, QuestionRevisionTuple, ReusableSelectionAvailability};
    use std::collections::BTreeMap;
    #[test]
    fn rejects_id_with_wrong_checksum() {
        let issuer = RandomQuestionIdIssuer::new();
        let valid = issuer.issue_question_id().expect("ID");
        let wrong = "0000-5000".parse::<QuestionId>();
        assert!(valid.as_str().parse::<QuestionId>().is_ok());
        assert!(wrong.is_err());
        assert!(!valid_idempotency_key(""));
        assert!(valid_idempotency_key("publish-1"));
    }

    #[test]
    fn retained_older_question_pin_uses_its_exact_revision_and_is_not_selectable() {
        let question_id = QuestionId::from_random_identifier("0000000").expect("Question ID");
        let older = QuestionRevisionTuple {
            question_id: question_id.clone(),
            revision_number: question_model::QuestionRevisionNumber::new(1).expect("revision one"),
        };
        let current = QuestionRevisionTuple {
            question_id: question_id.clone(),
            revision_number: question_model::QuestionRevisionNumber::new(2).expect("revision two"),
        };
        let values = BTreeMap::from([(older.clone(), "older"), (current.clone(), "current")]);
        let current_question_revision_tuples = BTreeMap::from([(question_id, current)]);

        assert_eq!(views::exact_revision_value(&older, &values), Some(&"older"));
        assert_eq!(
            views::selection_availability(&older, &current_question_revision_tuples),
            ReusableSelectionAvailability::Retained
        );
    }

    #[test]
    fn revision_and_metadata_preconditions_keep_mutations_qualified() {
        let mut headers = HeaderMap::new();
        headers.insert(IF_MATCH, HeaderValue::from_static("\"7\""));
        headers.insert("idempotency-key", HeaderValue::from_static("save-7"));
        let revision = expected_revision_number(&headers).expect("Revision CAS ETag");
        assert_eq!(revision.value(), 7);

        let first = request_checksum("save-blueprint-course", &headers, &("BP-1", revision))
            .expect("receipt checksum");
        let replay = request_checksum("save-blueprint-course", &headers, &("BP-1", revision))
            .expect("same receipt checksum");
        headers.insert("idempotency-key", HeaderValue::from_static("save-8"));
        let deliberate_later =
            request_checksum("save-blueprint-course", &headers, &("BP-1", revision))
                .expect("new receipt checksum");
        assert_eq!(first, replay);
        assert_ne!(first, deliberate_later);

        headers.insert(IF_MATCH, HeaderValue::from_static("\"7\""));
        assert_eq!(
            expected_edit_number(&headers)
                .expect("metadata Edit Number")
                .to_string(),
            "7"
        );
    }

    #[test]
    fn hidden_store_failures_remain_concealed() {
        for error in [
            StoreError::NotFound,
            StoreError::Forbidden,
            StoreError::OwnershipMismatch,
        ] {
            assert_eq!(store_error_response(error).status(), StatusCode::NOT_FOUND);
        }
    }

    #[test]
    fn lifecycle_and_edit_number_conflicts_have_distinct_statuses() {
        assert_eq!(
            super::store_error_response(StoreError::Conflict).status(),
            StatusCode::PRECONDITION_FAILED
        );
        assert_eq!(
            super::store_error_response(StoreError::LifecycleConflict).status(),
            StatusCode::CONFLICT
        );
    }
}

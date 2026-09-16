//! Blueprint Revision, lineage metadata, and availability routes.
//!
//! Browser Question IDs are HMAC-validated before Store resolution. The Store
//! alone resolves the exact immutable Question Revision pins.

use std::{collections::BTreeMap, sync::Arc};

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
    BlueprintCourseStore, QuestionLibraryStore, SessionTokenHash, StoreError,
    StoredBlueprintAssessmentContent, StoredBlueprintAssessmentEntry, StoredBlueprintCourse,
    StoredBlueprintCourseContent,
    postgres::{
        PostgresBlueprintCourseStore, PostgresBlueprintLineageStore, PostgresQuestionLibraryStore,
        PostgresSessionStore,
    },
};
use objects::s3::S3ObjectStore;
use question_model::{
    BlueprintAssessmentContentView, BlueprintAssessmentEntryView,
    BlueprintCourseAssessmentContentView, BlueprintCourseReference, BlueprintCourseSummaryView,
    BlueprintCourseView, BlueprintMetadataEtag, BlueprintModuleView, BlueprintRevision,
    BlueprintRevisionReference, CreateBlueprintCourseInput, QuestionId, QuestionRevisionReference,
    QuestionSearchResult, RenameBlueprintCourseInput, ReplaceBlueprintCourseContentInput,
    RequestChecksum, ReusablePoolView, ReusableQuestionView, ReusableSelectionAvailability,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::{
    auth::{AuthError, resolve_session},
    question_publication::HmacQuestionIdIssuer,
};

mod fork;
mod fork_apply;
mod fork_review;
mod history;
mod known_forks;
mod list;
mod pool_members;
mod responses;

use responses::{
    blueprint_response, blueprint_save_response, concealed, metadata_response, route_error,
    store_error_response, unavailable,
};

use fork::fork_blueprint;

const MAX_IDEMPOTENCY_KEY_BYTES: usize = 128;

#[derive(Clone)]
pub(super) struct BlueprintCourseRouteState {
    pub(super) sessions: Arc<PostgresSessionStore>,
    pub(super) blueprints: PostgresBlueprintCourseStore,
    pub(super) lineage: PostgresBlueprintLineageStore,
    pub(super) question_library: PostgresQuestionLibraryStore,
    pub(super) objects: S3ObjectStore,
    pub(super) question_id_issuer: HmacQuestionIdIssuer,
}

/// Registers the Instructor Blueprint lifecycle. Composition supplies the
/// deployment-only issuer so input validation precedes Store access.
pub fn blueprint_course_router(
    sessions: Arc<PostgresSessionStore>,
    blueprints: PostgresBlueprintCourseStore,
    lineage: PostgresBlueprintLineageStore,
    question_library: PostgresQuestionLibraryStore,
    objects: S3ObjectStore,
    question_id_issuer: HmacQuestionIdIssuer,
) -> Router {
    Router::new()
        .route(
            "/api/course-blueprints",
            get(list::list_blueprints).post(create_blueprint),
        )
        .route(
            "/api/course-blueprints/{reference}",
            get(load_blueprint).put(save_blueprint),
        )
        .route(
            "/api/course-blueprints/{left}/compare/{right}",
            get(fork_review::load_comparison),
        )
        .route(
            "/api/course-blueprints/{reference}/fork-update",
            post(fork_apply::apply_fork_update),
        )
        .route(
            "/api/course-blueprints/{reference}/forks",
            get(known_forks::list_known_forks),
        )
        .route(
            "/api/course-blueprints/{reference}/history",
            get(history::list_history),
        )
        .route(
            "/api/course-blueprints/{reference}/assessments/{assessment}/pools/{pool}/members",
            get(pool_members::load_pool_members),
        )
        .route(
            "/api/course-blueprints/{reference}/metadata",
            put(rename_blueprint),
        )
        .route(
            "/api/course-blueprints/{reference}/publish",
            post(publish_blueprint),
        )
        .route(
            "/api/course-blueprints/{reference}/revisions/{revision}",
            get(load_revision),
        )
        .route(
            "/api/course-blueprints/{reference}/revisions/{revision}/fork",
            post(fork_blueprint),
        )
        .route(
            "/api/course-blueprints/{reference}/archive",
            post(archive_blueprint),
        )
        .route(
            "/api/course-blueprints/{reference}/restore",
            post(restore_blueprint),
        )
        .route(
            "/api/course-blueprints/{reference}/return-to-private",
            post(return_blueprint_to_private),
        )
        .with_state(BlueprintCourseRouteState {
            sessions,
            blueprints,
            lineage,
            question_library,
            objects,
            question_id_issuer,
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
    Path(reference): Path<String>,
) -> Response {
    let reference = match parse_reference(&reference) {
        Ok(value) => value,
        Err(response) => return *response,
    };
    let session = match instructor_session_hash(&state, &headers).await {
        Ok(value) => value,
        Err(response) => return *response,
    };
    match load_view(&state, session, reference).await {
        Ok(view) => blueprint_response(StatusCode::OK, view),
        Err(RouteLoadError::Store(error)) => store_error_response(error),
        Err(RouteLoadError::Unavailable) => unavailable(),
    }
}

async fn create_blueprint(
    State(state): State<BlueprintCourseRouteState>,
    headers: HeaderMap,
    Json(input): Json<CreateBlueprintCourseInput>,
) -> Response {
    if !valid_create_question_ids(&state.question_id_issuer, &input) {
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
        .create_blueprint_course(session, checksum, input)
        .await
    {
        Ok(value) => value,
        Err(error) => return store_error_response(error),
    };
    match load_view(&state, session, receipt.blueprint_revision.reference).await {
        Ok(view) => blueprint_response(StatusCode::CREATED, view),
        Err(RouteLoadError::Store(error)) => store_error_response(error),
        Err(RouteLoadError::Unavailable) => unavailable(),
    }
}

async fn save_blueprint(
    State(state): State<BlueprintCourseRouteState>,
    headers: HeaderMap,
    Path(reference): Path<String>,
    Json(input): Json<ReplaceBlueprintCourseContentInput>,
) -> Response {
    let reference = match parse_reference(&reference) {
        Ok(value) => value,
        Err(response) => return *response,
    };
    if !valid_replace_question_ids(&state.question_id_issuer, &input) {
        return concealed();
    }
    let expected = match expected_revision(&headers) {
        Ok(value) => value,
        Err(response) => return *response,
    };
    let checksum = match request_checksum(
        "save-blueprint-course",
        &headers,
        &(reference, expected, &input),
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
        .save_blueprint_course(session, reference, expected, checksum, input)
        .await
    {
        Ok(receipt) => match load_view(&state, session, reference).await {
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
    Path(reference): Path<String>,
    Json(input): Json<RenameBlueprintCourseInput>,
) -> Response {
    let reference = match parse_reference(&reference) {
        Ok(value) => value,
        Err(response) => return *response,
    };
    let expected = match expected_metadata_etag(&headers) {
        Ok(value) => value,
        Err(response) => return *response,
    };
    let session = match instructor_session_hash(&state, &headers).await {
        Ok(value) => value,
        Err(response) => return *response,
    };
    match state
        .blueprints
        .rename_blueprint_course(session, reference, expected, input)
        .await
    {
        Ok(value) => metadata_response(value),
        Err(error) => store_error_response(error),
    }
}

async fn load_revision(
    State(state): State<BlueprintCourseRouteState>,
    headers: HeaderMap,
    Path((reference, revision)): Path<(String, String)>,
) -> Response {
    let reference = match parse_reference(&reference) {
        Ok(value) => value,
        Err(response) => return *response,
    };
    let revision = match revision.parse::<BlueprintRevision>() {
        Ok(value) => value,
        Err(_) => return concealed(),
    };
    let session = match instructor_session_hash(&state, &headers).await {
        Ok(value) => value,
        Err(response) => return *response,
    };
    let blueprint_revision = BlueprintRevisionReference {
        reference,
        revision,
    };
    let record = match state
        .blueprints
        .load_blueprint_revision(session, blueprint_revision)
        .await
    {
        Ok(value) => value,
        Err(error) => return store_error_response(error),
    };
    match content_modules(&state, session, &record.content).await {
        Ok(modules) => crate::auth::no_store(
            Json(BlueprintRevisionView {
                blueprint_revision,
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
    Path(reference): Path<String>,
    Json(input): Json<ArchiveBlueprintRequest>,
) -> Response {
    transition_availability(
        &state,
        &headers,
        reference,
        BlueprintLifecycleAction::Archive {
            confirmation_long_name: input.confirmation_long_name,
        },
    )
    .await
}
async fn publish_blueprint(
    State(state): State<BlueprintCourseRouteState>,
    headers: HeaderMap,
    Path(reference): Path<String>,
) -> Response {
    transition_availability(
        &state,
        &headers,
        reference,
        BlueprintLifecycleAction::Publish,
    )
    .await
}
async fn restore_blueprint(
    State(state): State<BlueprintCourseRouteState>,
    headers: HeaderMap,
    Path(reference): Path<String>,
) -> Response {
    transition_availability(
        &state,
        &headers,
        reference,
        BlueprintLifecycleAction::Restore,
    )
    .await
}
async fn return_blueprint_to_private(
    State(state): State<BlueprintCourseRouteState>,
    headers: HeaderMap,
    Path(reference): Path<String>,
) -> Response {
    transition_availability(
        &state,
        &headers,
        reference,
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
    let reference = match parse_reference(&value) {
        Ok(value) => value,
        Err(response) => return *response,
    };
    let expected = match expected_metadata_etag(headers) {
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
                .publish_blueprint(session, reference, expected)
                .await
        }
        BlueprintLifecycleAction::Archive {
            confirmation_long_name,
        } => {
            state
                .blueprints
                .archive_blueprint(session, reference, expected, &confirmation_long_name)
                .await
        }
        BlueprintLifecycleAction::Restore => {
            state
                .blueprints
                .restore_blueprint(session, reference, expected)
                .await
        }
        BlueprintLifecycleAction::ReturnToPrivate => {
            state
                .blueprints
                .return_blueprint_to_private(session, reference, expected)
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
    reference: BlueprintCourseReference,
) -> Result<BlueprintCourseView, RouteLoadError> {
    view_from_record(
        state,
        session,
        state
            .blueprints
            .load_blueprint_course(session, reference)
            .await
            .map_err(RouteLoadError::Store)?,
    )
    .await
}
async fn view_from_record(
    state: &BlueprintCourseRouteState,
    session: SessionTokenHash,
    record: StoredBlueprintCourse,
) -> Result<BlueprintCourseView, RouteLoadError> {
    Ok(BlueprintCourseView {
        reference: record.reference,
        short_name: record.short_name,
        long_name: record.long_name,
        availability: record.availability,
        metadata_etag: record.metadata_etag,
        current_revision: BlueprintRevisionReference {
            reference: record.reference,
            revision: record.current_revision,
        },
        read_access: record.read_access,
        fork_source: record.fork_source,
        modules: content_modules(state, session, &record.content).await?,
    })
}
fn summary_view(
    record: learning_data_access::StoredBlueprintCourseSummary,
) -> BlueprintCourseSummaryView {
    BlueprintCourseSummaryView {
        total_adoptions: record.total_adoptions,
        total_students_ever_enrolled: record.total_students_ever_enrolled,
        reference: record.reference,
        short_name: record.short_name,
        long_name: record.long_name,
        availability: record.availability,
        metadata_etag: record.metadata_etag,
        current_revision: BlueprintRevisionReference {
            reference: record.reference,
            revision: record.current_revision,
        },
        read_access: record.read_access,
    }
}
async fn content_modules(
    state: &BlueprintCourseRouteState,
    session: SessionTokenHash,
    content: &StoredBlueprintCourseContent,
) -> Result<Vec<BlueprintModuleView>, RouteLoadError> {
    let mut entries = state
        .question_library
        .list_published_question_library_entries(session)
        .await
        .map_err(RouteLoadError::Store)?;
    let current_question_revisions = entries
        .iter()
        .map(|entry| {
            (
                entry.question_revision.question_id.clone(),
                entry.question_revision.clone(),
            )
        })
        .collect::<BTreeMap<_, _>>();
    // Discovery excludes archived lineages.  Retained Blueprint pins use the
    // exact historical Store path so an archive never breaks immutable
    // Blueprint Revision interpretation.
    for reference in content_question_revisions(content) {
        if !entries
            .iter()
            .any(|entry| entry.question_revision == reference)
        {
            entries.push(
                state
                    .question_library
                    .load_published_question_revision_library_entry(session, &reference)
                    .await
                    .map_err(RouteLoadError::Store)?,
            );
        }
    }
    let questions =
        crate::question_library::answer_free_question_search_results(&state.objects, entries)
            .await
            .map_err(|_| RouteLoadError::Unavailable)?;
    content
        .modules
        .iter()
        .map(|module| {
            Ok(BlueprintModuleView {
                blueprint_module_reference: module.blueprint_module_reference,
                label: module.label.clone(),
                assessments: module
                    .assessments
                    .iter()
                    .map(|assessment| {
                        Ok(BlueprintCourseAssessmentContentView {
                            blueprint_assessment_reference: assessment
                                .blueprint_assessment_reference,
                            content: assessment_content_view(
                                &assessment.content,
                                &questions,
                                &current_question_revisions,
                            )?,
                        })
                    })
                    .collect::<Result<Vec<_>, RouteLoadError>>()?,
            })
        })
        .collect()
}

fn content_question_revisions(
    content: &StoredBlueprintCourseContent,
) -> Vec<question_model::QuestionRevisionReference> {
    content
        .modules
        .iter()
        .flat_map(|module| module.assessments.iter())
        .flat_map(|assessment| assessment.content.entries.iter())
        .filter_map(|entry| match entry {
            StoredBlueprintAssessmentEntry::Fixed {
                question_revision, ..
            } => Some(question_revision),
            StoredBlueprintAssessmentEntry::Pool { .. } => None,
        })
        .cloned()
        .collect()
}
fn assessment_content_view(
    content: &StoredBlueprintAssessmentContent,
    questions: &BTreeMap<QuestionRevisionReference, QuestionSearchResult>,
    current_question_revisions: &BTreeMap<QuestionId, QuestionRevisionReference>,
) -> Result<BlueprintAssessmentContentView, RouteLoadError> {
    let entries = content
        .entries
        .iter()
        .map(|entry| match entry {
            StoredBlueprintAssessmentEntry::Fixed {
                question_revision,
                points_possible,
                scoring_rule,
                question_attempt_limit,
                question_attempt_time_limit,
            } => Ok(BlueprintAssessmentEntryView::Fixed {
                question: Box::new(question_view(
                    question_revision,
                    questions,
                    current_question_revisions,
                )?),
                points_possible: *points_possible,
                scoring_rule: *scoring_rule,
                question_attempt_limit: *question_attempt_limit,
                question_attempt_time_limit: *question_attempt_time_limit,
            }),
            StoredBlueprintAssessmentEntry::Pool {
                question_pool_revision,
                selection_count,
                points_per_item,
                scoring_rule,
                selection_rule,
                question_attempt_limit,
                question_attempt_time_limit,
            } => Ok(BlueprintAssessmentEntryView::Pool(ReusablePoolView {
                question_pool_revision: question_pool_revision.clone(),
                selection_count: *selection_count,
                points_per_item: *points_per_item,
                scoring_rule: *scoring_rule,
                selection_rule: *selection_rule,
                question_attempt_limit: *question_attempt_limit,
                question_attempt_time_limit: *question_attempt_time_limit,
            })),
        })
        .collect::<Result<Vec<_>, RouteLoadError>>()?;
    Ok(BlueprintAssessmentContentView {
        assessment_type: content.assessment_type,
        title: content.title.clone(),
        instructions: content.instructions.clone(),
        entries,
        defaults: content.defaults.clone(),
    })
}
fn question_view(
    reference: &question_model::QuestionRevisionReference,
    questions: &BTreeMap<QuestionRevisionReference, QuestionSearchResult>,
    current_question_revisions: &BTreeMap<QuestionId, QuestionRevisionReference>,
) -> Result<ReusableQuestionView, RouteLoadError> {
    Ok(ReusableQuestionView {
        reference: reference.clone(),
        question_library: question_search_result(reference, questions)?,
        selection_availability: selection_availability(reference, current_question_revisions),
    })
}
fn question_search_result(
    reference: &question_model::QuestionRevisionReference,
    questions: &BTreeMap<QuestionRevisionReference, QuestionSearchResult>,
) -> Result<QuestionSearchResult, RouteLoadError> {
    exact_revision_value(reference, questions)
        .cloned()
        .ok_or(RouteLoadError::Unavailable)
}

fn exact_revision_value<'a, T>(
    reference: &QuestionRevisionReference,
    values: &'a BTreeMap<QuestionRevisionReference, T>,
) -> Option<&'a T> {
    values.get(reference)
}
fn selection_availability(
    reference: &question_model::QuestionRevisionReference,
    current_question_revisions: &BTreeMap<QuestionId, QuestionRevisionReference>,
) -> ReusableSelectionAvailability {
    if current_question_revisions.get(&reference.question_id) == Some(reference) {
        ReusableSelectionAvailability::Available
    } else {
        ReusableSelectionAvailability::Retained
    }
}

// ASVS 2.2.1/2.2.2: reject a syntactically plausible but deployment-invalid ID
// before a persistence resolver can disclose whether a Question exists.
fn valid_create_question_ids(
    issuer: &HmacQuestionIdIssuer,
    input: &CreateBlueprintCourseInput,
) -> bool {
    input
        .modules
        .iter()
        .flat_map(|module| module.assessments.iter())
        .all(|assessment| valid_assessment_question_ids(issuer, assessment))
}
fn valid_replace_question_ids(
    issuer: &HmacQuestionIdIssuer,
    input: &ReplaceBlueprintCourseContentInput,
) -> bool {
    input
        .modules
        .iter()
        .flat_map(|module| module.assessments.iter())
        .all(|assessment| valid_assessment_question_ids(issuer, &assessment.content))
}
fn valid_assessment_question_ids(
    issuer: &HmacQuestionIdIssuer,
    input: &question_model::BlueprintAssessmentContentInput,
) -> bool {
    input.entries.iter().all(|entry| match entry {
        question_model::BlueprintAssessmentEntryInput::Fixed(value) => {
            issuer.validates_question_id(&value.published_question.question_id)
        }
        // ASVS 2.2.1/2.2.2: validate the exact Pool and any newly authored
        // member Question IDs before the Store resolves private state.
        question_model::BlueprintAssessmentEntryInput::Pool(value) => match &value.pool {
            question_model::BlueprintPoolInputChoice::Import {
                question_pool_revision,
            } => issuer.validates_question_id(&question_pool_revision.question_pool_id),
            question_model::BlueprintPoolInputChoice::Retained {
                question_pool_revision,
                members,
                ..
            } => {
                issuer.validates_question_id(&question_pool_revision.question_pool_id)
                    && members.as_ref().is_none_or(|members| {
                        members
                            .iter()
                            .all(|member| issuer.validates_question_id(&member.question_id))
                    })
            }
        },
    })
}

pub(super) fn parse_reference(value: &str) -> Result<BlueprintCourseReference, Box<Response>> {
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
fn expected_revision(headers: &HeaderMap) -> Result<BlueprintRevision, Box<Response>> {
    quoted_if_match(headers)?.parse().map_err(|_| {
        Box::new(route_error(
            StatusCode::BAD_REQUEST,
            "Blueprint Revision ETag is invalid",
        ))
    })
}
fn expected_metadata_etag(headers: &HeaderMap) -> Result<BlueprintMetadataEtag, Box<Response>> {
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
    use crate::question_publication::{QuestionIdIssuer, QuestionIdSecret};
    use axum::http::{HeaderValue, StatusCode};
    use learning_data_access::StoreError;
    #[test]
    fn rejects_syntactically_valid_id_with_wrong_hmac() {
        let issuer = HmacQuestionIdIssuer::new(QuestionIdSecret::from_bytes([5; 32]));
        let valid = issuer.issue_question_id().expect("ID");
        let wrong = QuestionId::from_canonical_parts(
            valid.identifier_compact(),
            if valid.validation_character() == '0' {
                '1'
            } else {
                '0'
            },
        )
        .expect("shape");
        assert!(issuer.validates_question_id(&valid));
        assert!(!issuer.validates_question_id(&wrong));
        assert!(!valid_idempotency_key(""));
        assert!(valid_idempotency_key("publish-1"));
    }

    #[test]
    fn retained_older_question_pin_uses_its_exact_revision_and_is_not_selectable() {
        let question_id: QuestionId = "0000-X000".parse().expect("Question ID");
        let older = QuestionRevisionReference {
            question_id: question_id.clone(),
            revision_number: question_model::QuestionRevisionNumber::new(1).expect("revision one"),
        };
        let current = QuestionRevisionReference {
            question_id: question_id.clone(),
            revision_number: question_model::QuestionRevisionNumber::new(2).expect("revision two"),
        };
        let values = BTreeMap::from([(older.clone(), "older"), (current.clone(), "current")]);
        let current_revisions = BTreeMap::from([(question_id, current)]);

        assert_eq!(exact_revision_value(&older, &values), Some(&"older"));
        assert_eq!(
            selection_availability(&older, &current_revisions),
            ReusableSelectionAvailability::Retained
        );
    }

    #[test]
    fn revision_and_metadata_preconditions_keep_mutations_qualified() {
        let mut headers = HeaderMap::new();
        headers.insert(IF_MATCH, HeaderValue::from_static("\"7\""));
        headers.insert("idempotency-key", HeaderValue::from_static("save-7"));
        let revision = expected_revision(&headers).expect("Revision CAS ETag");
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

        headers.insert(
            IF_MATCH,
            HeaderValue::from_static("\"00000000-0000-0000-0000-000000000007\""),
        );
        assert_eq!(
            expected_metadata_etag(&headers)
                .expect("metadata ETag")
                .to_string(),
            "00000000-0000-0000-0000-000000000007"
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

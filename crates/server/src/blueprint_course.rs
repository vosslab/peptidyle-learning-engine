//! Blueprint lineage, private Draft, publication, and availability routes.
//!
//! Browser Question IDs are HMAC-validated before Store resolution. The Store
//! alone resolves the exact immutable Question Revision pins.

use std::{collections::BTreeMap, str::FromStr, sync::Arc};

use axum::{
    Json, Router,
    extract::{Path, Query, State},
    http::{
        HeaderMap, HeaderValue, StatusCode,
        header::{COOKIE, ETAG, IF_MATCH},
    },
    response::{IntoResponse, Response},
    routing::{get, post, put},
};
use browser_api_contract::blueprint_course::BlueprintRevisionView;
use learning_data_access::{
    BlueprintCourseStore, QuestionLibraryStore, SessionTokenHash, StoreError,
    StoredBlueprintAssignmentContent, StoredBlueprintAssignmentEntry, StoredBlueprintCourse,
    StoredBlueprintCourseContent,
    postgres::{PostgresBlueprintCourseStore, PostgresQuestionLibraryStore, PostgresSessionStore},
};
use objects::s3::S3ObjectStore;
use question_model::{
    BlueprintAssignmentContentView, BlueprintAssignmentEntryView, BlueprintAvailability,
    BlueprintAvailabilityEditNumber, BlueprintCourseAssignmentContentView,
    BlueprintCourseReference, BlueprintCourseSummaryView, BlueprintCourseView,
    BlueprintDraftEditNumber, BlueprintDraftView, BlueprintModuleView, BlueprintRevision,
    BlueprintRevisionReference, CreateBlueprintCourseContentInput, QuestionId,
    QuestionRevisionReference, QuestionSearchResult, ReplaceBlueprintCourseContentInput,
    RequestChecksum, ReusablePoolView, ReusableQuestionPoolItemView, ReusableQuestionView,
    ReusableSelectionAvailability,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::{
    auth::{AuthError, resolve_session},
    question_publication::HmacQuestionIdIssuer,
};

const MAX_PAGE_SIZE: u16 = 100;
const MAX_IDEMPOTENCY_KEY_BYTES: usize = 128;

#[derive(Clone)]
struct BlueprintCourseRouteState {
    sessions: Arc<PostgresSessionStore>,
    blueprints: PostgresBlueprintCourseStore,
    question_library: PostgresQuestionLibraryStore,
    objects: S3ObjectStore,
    question_id_issuer: HmacQuestionIdIssuer,
}

/// Registers the instructor Blueprint lifecycle. Composition supplies the
/// deployment-only issuer so input validation precedes Store access.
pub fn blueprint_course_router(
    sessions: Arc<PostgresSessionStore>,
    blueprints: PostgresBlueprintCourseStore,
    question_library: PostgresQuestionLibraryStore,
    objects: S3ObjectStore,
    question_id_issuer: HmacQuestionIdIssuer,
) -> Router {
    Router::new()
        .route(
            "/api/course-blueprints",
            get(list_blueprints).post(create_blueprint),
        )
        .route("/api/course-blueprints/{reference}", get(load_blueprint))
        .route("/api/course-blueprints/{reference}/draft", put(save_draft))
        .route(
            "/api/course-blueprints/{reference}/publish",
            post(publish_draft),
        )
        .route(
            "/api/course-blueprints/{reference}/revisions/{revision}",
            get(load_revision),
        )
        .route(
            "/api/course-blueprints/{reference}/archive",
            post(archive_blueprint),
        )
        .route(
            "/api/course-blueprints/{reference}/restore",
            post(restore_blueprint),
        )
        .with_state(BlueprintCourseRouteState {
            sessions,
            blueprints,
            question_library,
            objects,
            question_id_issuer,
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
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct ArchiveBlueprintRequest {
    confirmation_title: String,
}
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct BlueprintPublicationResponse {
    blueprint_revision: BlueprintRevisionReference,
}
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct BlueprintAvailabilityResponse {
    availability: BlueprintAvailability,
    edit_number: BlueprintAvailabilityEditNumber,
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
    let session = match instructor_session_hash(&state, &headers).await {
        Ok(value) => value,
        Err(response) => return *response,
    };
    match state.blueprints.list_blueprint_courses(session).await {
        Ok(records) => crate::auth::no_store(
            Json(BlueprintCourseListResponse {
                items: records.into_iter().map(summary_view).collect(),
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
    Json(input): Json<CreateBlueprintCourseContentInput>,
) -> Response {
    if !valid_create_question_ids(&state.question_id_issuer, &input) {
        return concealed();
    }
    let checksum = match request_checksum("create-blueprint-draft", &headers, &input) {
        Ok(value) => value,
        Err(response) => return *response,
    };
    let session = match instructor_session_hash(&state, &headers).await {
        Ok(value) => value,
        Err(response) => return *response,
    };
    let receipt = match state
        .blueprints
        .create_blueprint_draft(session, checksum, input)
        .await
    {
        Ok(value) => value,
        Err(error) => return store_error_response(error),
    };
    match load_view(&state, session, receipt.blueprint).await {
        Ok(view) => blueprint_response(StatusCode::CREATED, view),
        Err(RouteLoadError::Store(error)) => store_error_response(error),
        Err(RouteLoadError::Unavailable) => unavailable(),
    }
}

async fn save_draft(
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
    let expected = match expected_edit_number::<BlueprintDraftEditNumber>(&headers, "Draft") {
        Ok(value) => value,
        Err(response) => return *response,
    };
    let checksum = match request_checksum(
        "save-blueprint-draft",
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
        .save_blueprint_draft(session, reference, expected, checksum, input)
        .await
    {
        Ok(_) => match load_view(&state, session, reference).await {
            Ok(view) => blueprint_response(StatusCode::OK, view),
            Err(RouteLoadError::Store(error)) => store_error_response(error),
            Err(RouteLoadError::Unavailable) => unavailable(),
        },
        Err(error) => store_error_response(error),
    }
}

async fn publish_draft(
    State(state): State<BlueprintCourseRouteState>,
    headers: HeaderMap,
    Path(reference): Path<String>,
) -> Response {
    let reference = match parse_reference(&reference) {
        Ok(value) => value,
        Err(response) => return *response,
    };
    let expected = match expected_edit_number::<BlueprintDraftEditNumber>(&headers, "Draft") {
        Ok(value) => value,
        Err(response) => return *response,
    };
    let checksum =
        match request_checksum("publish-blueprint-draft", &headers, &(reference, expected)) {
            Ok(value) => value,
            Err(response) => return *response,
        };
    let session = match instructor_session_hash(&state, &headers).await {
        Ok(value) => value,
        Err(response) => return *response,
    };
    match state
        .blueprints
        .publish_blueprint_draft(session, reference, expected, checksum)
        .await
    {
        Ok(receipt) => crate::auth::no_store(
            (
                StatusCode::OK,
                Json(BlueprintPublicationResponse {
                    blueprint_revision: receipt.blueprint_revision,
                }),
            )
                .into_response(),
        ),
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
                title: record.content.title,
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
        Some(input.confirmation_title),
        true,
    )
    .await
}
async fn restore_blueprint(
    State(state): State<BlueprintCourseRouteState>,
    headers: HeaderMap,
    Path(reference): Path<String>,
) -> Response {
    transition_availability(&state, &headers, reference, None, false).await
}
async fn transition_availability(
    state: &BlueprintCourseRouteState,
    headers: &HeaderMap,
    value: String,
    confirmation: Option<String>,
    archive: bool,
) -> Response {
    let reference = match parse_reference(&value) {
        Ok(value) => value,
        Err(response) => return *response,
    };
    let expected =
        match expected_edit_number::<BlueprintAvailabilityEditNumber>(headers, "Availability") {
            Ok(value) => value,
            Err(response) => return *response,
        };
    let session = match instructor_session_hash(state, headers).await {
        Ok(value) => value,
        Err(response) => return *response,
    };
    let result = if archive {
        state
            .blueprints
            .archive_blueprint(
                session,
                reference,
                expected,
                confirmation.as_deref().unwrap_or_default(),
            )
            .await
    } else {
        state
            .blueprints
            .restore_blueprint(session, reference, expected)
            .await
    };
    match result {
        Ok(value) => availability_response(value.availability, value.edit_number),
        Err(error) => store_error_response(error),
    }
}

enum RouteLoadError {
    Store(StoreError),
    Unavailable,
}
async fn load_view(
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
    let draft = match record.draft_edit_number {
        Some(edit_number) => Some(BlueprintDraftView {
            edit_number,
            modules: content_modules(state, session, &record.content).await?,
        }),
        None => None,
    };
    Ok(BlueprintCourseView {
        reference: record.reference,
        title: record.title,
        availability: record.availability,
        availability_edit_number: record.availability_edit_number,
        latest_published_revision: record.latest_published_revision.map(|revision| {
            BlueprintRevisionReference {
                reference: record.reference,
                revision,
            }
        }),
        read_access: record.read_access,
        draft,
    })
}
fn summary_view(
    record: learning_data_access::StoredBlueprintCourseSummary,
) -> BlueprintCourseSummaryView {
    BlueprintCourseSummaryView {
        reference: record.reference,
        title: record.title,
        availability: record.availability,
        availability_edit_number: record.availability_edit_number,
        latest_published_revision: record.latest_published_revision.map(|revision| {
            BlueprintRevisionReference {
                reference: record.reference,
                revision,
            }
        }),
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
                assignments: module
                    .assignments
                    .iter()
                    .map(|assignment| {
                        Ok(BlueprintCourseAssignmentContentView {
                            blueprint_assignment_reference: assignment
                                .blueprint_assignment_reference,
                            content: assignment_content_view(
                                &assignment.content,
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
        .flat_map(|module| module.assignments.iter())
        .flat_map(|assignment| assignment.content.entries.iter())
        .flat_map(|entry| match entry {
            StoredBlueprintAssignmentEntry::Fixed {
                question_revision, ..
            } => std::slice::from_ref(question_revision).iter(),
            StoredBlueprintAssignmentEntry::Pool {
                question_revisions, ..
            } => question_revisions.iter(),
        })
        .cloned()
        .collect()
}
fn assignment_content_view(
    content: &StoredBlueprintAssignmentContent,
    questions: &BTreeMap<QuestionRevisionReference, QuestionSearchResult>,
    current_question_revisions: &BTreeMap<QuestionId, QuestionRevisionReference>,
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
                            selection_availability: selection_availability(
                                reference,
                                current_question_revisions,
                            ),
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
    questions: &BTreeMap<QuestionRevisionReference, QuestionSearchResult>,
    current_question_revisions: &BTreeMap<QuestionId, QuestionRevisionReference>,
) -> Result<ReusableQuestionView, RouteLoadError> {
    Ok(ReusableQuestionView {
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
    input: &CreateBlueprintCourseContentInput,
) -> bool {
    input
        .modules
        .iter()
        .flat_map(|module| module.assignments.iter())
        .all(|assignment| valid_assignment_question_ids(issuer, assignment))
}
fn valid_replace_question_ids(
    issuer: &HmacQuestionIdIssuer,
    input: &ReplaceBlueprintCourseContentInput,
) -> bool {
    input
        .modules
        .iter()
        .flat_map(|module| module.assignments.iter())
        .all(|assignment| valid_assignment_question_ids(issuer, &assignment.content))
}
fn valid_assignment_question_ids(
    issuer: &HmacQuestionIdIssuer,
    input: &question_model::BlueprintAssignmentContentInput,
) -> bool {
    input
        .entries
        .iter()
        .flat_map(|entry| match entry {
            question_model::BlueprintAssignmentEntryInput::Fixed(value) => {
                std::slice::from_ref(&value.question_id).iter()
            }
            question_model::BlueprintAssignmentEntryInput::Pool(value) => value.items.iter(),
        })
        .all(|question_id| issuer.validates_question_id(question_id))
}

fn parse_reference(value: &str) -> Result<BlueprintCourseReference, Box<Response>> {
    value.parse().map_err(|_| Box::new(concealed()))
}
fn expected_edit_number<T: FromStr>(
    headers: &HeaderMap,
    kind: &'static str,
) -> Result<T, Box<Response>> {
    let Some(value) = headers.get(IF_MATCH).and_then(|value| value.to_str().ok()) else {
        return Err(Box::new(route_error(
            StatusCode::PRECONDITION_REQUIRED,
            "Blueprint Edit Number is required",
        )));
    };
    let Some(number) = value
        .strip_prefix('"')
        .and_then(|candidate| candidate.strip_suffix('"'))
    else {
        return Err(Box::new(route_error(
            StatusCode::BAD_REQUEST,
            "Blueprint Edit Number is invalid",
        )));
    };
    number.parse().map_err(|_| {
        Box::new(route_error(
            StatusCode::BAD_REQUEST,
            if kind == "Draft" {
                "Blueprint Draft Edit Number is invalid"
            } else {
                "Blueprint Availability Edit Number is invalid"
            },
        ))
    })
}
// The operation name, standard transport key, and exact serialized command
// form one receipt key; a new deliberate publish uses a new key (ASVS 2.3.1).
fn request_checksum<T: Serialize>(
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
fn blueprint_response(status: StatusCode, view: BlueprintCourseView) -> Response {
    let edit = view
        .draft
        .as_ref()
        .map(|draft| draft.edit_number.to_string());
    let mut response = crate::auth::no_store((status, Json(view)).into_response());
    match edit {
        Some(edit) => match HeaderValue::from_str(&format!("\"{edit}\"")) {
            Ok(value) => {
                response.headers_mut().insert(ETAG, value);
                response
            }
            Err(_) => unavailable(),
        },
        None => response,
    }
}
fn availability_response(
    availability: BlueprintAvailability,
    edit_number: BlueprintAvailabilityEditNumber,
) -> Response {
    let mut response = crate::auth::no_store(
        Json(BlueprintAvailabilityResponse {
            availability,
            edit_number,
        })
        .into_response(),
    );
    match HeaderValue::from_str(&format!("\"{edit_number}\"")) {
        Ok(value) => {
            response.headers_mut().insert(ETAG, value);
            response
        }
        Err(_) => unavailable(),
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
        StoreError::LifecycleConflict => {
            route_error(StatusCode::CONFLICT, "Blueprint Course lifecycle conflict")
        }
        StoreError::InvalidRecord(_) => route_error(
            StatusCode::UNPROCESSABLE_ENTITY,
            "Blueprint Course is invalid",
        ),
        StoreError::AlreadyExists => route_error(StatusCode::CONFLICT, "Blueprint Course conflict"),
        StoreError::AssignmentActivity(_) | StoreError::TimedOut | StoreError::Unavailable(_) => {
            unavailable()
        }
    }
}
fn concealed() -> Response {
    route_error(StatusCode::NOT_FOUND, "Blueprint Course not found")
}
fn unavailable() -> Response {
    route_error(
        StatusCode::SERVICE_UNAVAILABLE,
        "Blueprint Course unavailable",
    )
}
fn route_error(status: StatusCode, message: &'static str) -> Response {
    crate::auth::no_store((status, message).into_response())
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
        let question_id: QuestionId = "000-0000".parse().expect("Question ID");
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
    fn separate_edit_numbers_and_request_receipts_keep_mutations_qualified() {
        let mut headers = HeaderMap::new();
        headers.insert(IF_MATCH, HeaderValue::from_static("\"7\""));
        headers.insert("idempotency-key", HeaderValue::from_static("publish-7"));
        let draft = expected_edit_number::<BlueprintDraftEditNumber>(&headers, "Draft")
            .expect("Draft CAS ETag");
        let availability =
            expected_edit_number::<BlueprintAvailabilityEditNumber>(&headers, "Availability")
                .expect("availability CAS ETag");
        assert_eq!(draft.value(), availability.value());

        let first = request_checksum("publish-blueprint-draft", &headers, &("BP-1", draft))
            .expect("receipt checksum");
        let replay = request_checksum("publish-blueprint-draft", &headers, &("BP-1", draft))
            .expect("same receipt checksum");
        headers.insert("idempotency-key", HeaderValue::from_static("publish-8"));
        let deliberate_later =
            request_checksum("publish-blueprint-draft", &headers, &("BP-1", draft))
                .expect("new receipt checksum");
        assert_eq!(first, replay);
        assert_ne!(first, deliberate_later);
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

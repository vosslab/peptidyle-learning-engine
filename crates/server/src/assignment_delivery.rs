//! Student Assignment Access and answer-free Assignment start routes.

use std::{str::FromStr, sync::Arc};

use adapter_ple::{PleQuestionBackend, ResolvedPleQuestionJsonSource};
use adapter_webwork::{
    HttpWebworkRenderer, ResolvedWebworkQuestionSource, WebworkAdapter,
    WebworkQuestionSourceBinding,
};
use axum::{
    Json, Router,
    extract::{FromRef, Path, Query, State},
    http::{HeaderMap, StatusCode, header::COOKIE},
    response::{IntoResponse, Response},
    routing::{get, post, put},
};
use browser_api_contract::assignment_delivery::StudentQuestionPresentation;
use learning_data_access::{
    LiveAssignmentAttempt, LiveAssignmentDeliveryStore, NativeAssignmentIssuanceBatch,
    NativePleIssuanceSource, NativePresentationInput, NativeWebworkIssuanceSource,
    SessionTokenHash, StoreError,
    postgres::{
        PostgresLiveAssignmentDeliveryStore, PostgresNativePleSubmissionStore, PostgresSessionStore,
    },
};
use objects::s3::S3ObjectStore;
use question_model::{
    AssignmentAttemptReference, AssignmentReference, CourseInstanceReference, ObjectId,
    ProductRole, QuestionPresentation, QuestionPresentationChecksum, QuestionRevisionNumber,
    QuestionRevisionReference, SourceObjectChecksum, SourceObjectReference, StudentResponse,
    Timestamp,
};
use question_model::{generation::QuestionSeed, presentation::build_question_presentation};
use serde::Serialize;

use crate::auth::{AuthError, resolve_session};

mod context;
mod history;
mod history_response;
mod presentation_assets;
mod submission;

pub(super) use presentation_assets::{
    question_asset_renditions, question_asset_renditions_from_ready,
};

#[derive(serde::Deserialize)]
struct PositionQuery {
    position: u32,
}

#[derive(Clone)]
pub(super) struct StateData {
    pub(super) sessions: Arc<PostgresSessionStore>,
    pub(super) delivery: PostgresLiveAssignmentDeliveryStore,
    pub(super) submissions: PostgresNativePleSubmissionStore,
    pub(super) objects: S3ObjectStore,
    pub(super) webwork: Arc<WebworkAdapter<S3ObjectStore, HttpWebworkRenderer>>,
}

/// The retained status read needs only authentication and its dedicated
/// question-submission projection, not the mutable Assignment delivery state.
#[derive(Clone)]
pub(super) struct NativePleSubmissionStatusState {
    pub(super) sessions: Arc<PostgresSessionStore>,
    pub(super) submissions: PostgresNativePleSubmissionStore,
}

impl FromRef<StateData> for NativePleSubmissionStatusState {
    fn from_ref(state: &StateData) -> Self {
        Self {
            sessions: Arc::clone(&state.sessions),
            submissions: state.submissions.clone(),
        }
    }
}

/// Registers Student-only Assignment Access and initial start routes.
pub fn assignment_delivery_router(
    sessions: Arc<PostgresSessionStore>,
    delivery: PostgresLiveAssignmentDeliveryStore,
    submissions: PostgresNativePleSubmissionStore,
    objects: S3ObjectStore,
    webwork: Arc<WebworkAdapter<S3ObjectStore, HttpWebworkRenderer>>,
) -> Router {
    Router::new()
        .route(
            "/api/course-instances/{course}/assignments/{assignment}/access",
            get(access),
        )
        .route(
            "/api/course-instances/{course}/assignments/{assignment}/start",
            post(start),
        )
        .route(
            "/api/course-instances/{course}/assignments/{assignment}/presentations/{presentation_nonce}/submissions",
            submission::native_ple_submission_status_route::<StateData>(),
        )
        .route(
            "/api/assignment-attempts/{assignment_attempt}/student-progress",
            get(student_progress),
        )
        .route(
            "/api/assignment-attempts/{assignment_attempt}/history",
            get(history::student_history),
        )
        .route(
            "/api/assignment-attempts/{assignment_attempt}/context",
            get(context::student_context),
        )
        .route(
            "/api/assignment-attempts/{assignment_attempt}/student-question",
            get(student_question),
        )
        .route(
            "/api/assignment-attempts/{assignment_attempt}/responses/{position}",
            put(submission::save_selected_response),
        )
        .route(
            "/api/assignment-attempts/{assignment_attempt}/submission",
            post(submission::finalize_assignment_attempt),
        )
        .with_state(StateData {
            sessions,
            delivery,
            submissions,
            objects,
            webwork,
        })
}

async fn student_progress(
    State(state): State<StateData>,
    headers: HeaderMap,
    Path(assignment_attempt): Path<String>,
) -> Response {
    let assignment_attempt = match AssignmentAttemptReference::from_str(&assignment_attempt) {
        Ok(value) => value,
        Err(_) => return concealed(),
    };
    let token = match student(&state, &headers).await {
        Ok(value) => value,
        Err(value) => return *value,
    };
    match state
        .delivery
        .student_assignment_attempt_progress(token, assignment_attempt)
        .await
    {
        Ok(value) => crate::auth::no_store(Json(value).into_response()),
        Err(value) => store_error(value),
    }
}

async fn student_question(
    State(state): State<StateData>,
    headers: HeaderMap,
    Path(assignment_attempt): Path<String>,
    Query(query): Query<PositionQuery>,
) -> Response {
    let assignment_attempt = match AssignmentAttemptReference::from_str(&assignment_attempt) {
        Ok(value) => value,
        Err(_) => return concealed(),
    };
    if query.position == 0 {
        return concealed();
    }
    let token = match student(&state, &headers).await {
        Ok(value) => value,
        Err(value) => return *value,
    };
    let source = match state
        .delivery
        .student_assignment_attempt_presentation_evidence(token, assignment_attempt, query.position)
        .await
    {
        Ok(value) => value,
        Err(value) => return store_error(value),
    };
    let question_revision = source.question_revision.clone();
    let issued = match reproduce_selected_issued_presentation(source) {
        Ok(value) => value,
        Err(StartError::Store(value)) => return store_error(value),
        Err(StartError::Unavailable) => {
            return error(
                StatusCode::SERVICE_UNAVAILABLE,
                "Student Assignment delivery unavailable",
            );
        }
        Err(StartError::Invalid) => return concealed(),
    };
    let saved_response = match state
        .delivery
        .student_assignment_attempt_saved_response(token, assignment_attempt, query.position)
        .await
    {
        Ok(Some(value)) => match submission::restore_saved_response(&value, &issued) {
            Ok(value) => Some(value),
            Err(()) => return concealed(),
        },
        Ok(None) => None,
        Err(value) => return store_error(value),
    };
    // ASVS 2.2.1/2.2.2 and 14.1.1: the trusted presentation is the only
    // boundary that can translate durable response identifiers back into the
    // opaque identifiers rendered to this Student.
    crate::auth::no_store(
        Json(SelectedPresentationResponse {
            position: query.position,
            presentation: StudentQuestionPresentation {
                question_revision,
                prompt: issued.presentation.prompt,
                response: issued.presentation.response,
            },
            saved_response,
        })
        .into_response(),
    )
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct SelectedPresentationResponse {
    position: u32,
    presentation: StudentQuestionPresentation,
    saved_response: Option<StudentResponse>,
}

pub(super) fn reproduce_selected_issued_presentation(
    evidence: learning_data_access::StudentAssignmentAttemptPresentationEvidence,
) -> Result<question_model::presentation::IssuedQuestionPresentation, StartError> {
    let presentation = serde_json::from_value::<QuestionPresentation>(evidence.presentation)
        .map_err(|_| StartError::Invalid)?;
    if presentation.question_revision != evidence.question_revision
        || presentation.question_seed.value() != evidence.question_seed
        || presentation.presentation_nonce.to_hex() != evidence.presentation_nonce
    {
        return Err(StartError::Invalid);
    }
    let checksum = QuestionPresentationChecksum::parse_hex(&evidence.presentation_checksum)
        .map_err(|_| StartError::Invalid)?;
    let issued = question_model::presentation::rebuild_public_question_presentation(
        &presentation,
        &question_asset_renditions_from_ready(&evidence.question_asset_renditions),
    )
    .map_err(|_| StartError::Invalid)?;
    if issued.checksum != checksum {
        return Err(StartError::Invalid);
    }
    question_model::presentation::rebind_durable_response_item_bindings(
        issued,
        &evidence.response_item_bindings,
    )
    .map_err(|_| StartError::Invalid)
}

async fn access(
    State(state): State<StateData>,
    headers: HeaderMap,
    Path((course, assignment)): Path<(String, String)>,
) -> Response {
    let (course, assignment) = match refs(&course, &assignment) {
        Ok(v) => v,
        Err(v) => return v,
    };
    let token = match student(&state, &headers).await {
        Ok(v) => v,
        Err(v) => return *v,
    };
    match state
        .delivery
        .live_assignment_access(token, course, assignment)
        .await
    {
        Ok(value) => crate::auth::no_store(Json(value).into_response()),
        Err(value) => store_error(value),
    }
}

async fn start(
    State(state): State<StateData>,
    headers: HeaderMap,
    Path((course, assignment)): Path<(String, String)>,
) -> Response {
    let (course, assignment) = match refs(&course, &assignment) {
        Ok(v) => v,
        Err(v) => return v,
    };
    let token = match student(&state, &headers).await {
        Ok(v) => v,
        Err(v) => return *v,
    };
    // ASVS 2.3.1/2.3.3 and 8.2.1/8.2.2: only the Student-authorized store
    // seam exposes immutable source pins; this route resolves and persists a
    // presentation before serializing its answer-free public form.
    match issue_native_ple_presentation(&state, token, course, assignment).await {
        Ok(value) => crate::auth::no_store((StatusCode::CREATED, Json(value)).into_response()),
        Err(StartError::Store(value)) => store_error(value),
        Err(StartError::Unavailable) => error(
            StatusCode::SERVICE_UNAVAILABLE,
            "Student Assignment delivery unavailable",
        ),
        Err(StartError::Invalid) => error(
            StatusCode::UNPROCESSABLE_ENTITY,
            "Assignment start is invalid",
        ),
    }
}

/// Browser response: all presentation fields are intentionally answer-free;
/// the store's private attempt ID, source, binding checksum, and reproduction
/// evidence never cross this type boundary.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct LiveAssignmentAttemptResponse {
    assignment_attempt: AssignmentAttemptReference,
    assignment: AssignmentReference,
    attempt_number: u32,
    resumed: bool,
    title: String,
    instructions: String,
    questions: Vec<QuestionPresentation>,
}

pub(super) enum StartError {
    Store(StoreError),
    Invalid,
    Unavailable,
}

async fn issue_native_ple_presentation(
    state: &StateData,
    token: SessionTokenHash,
    course: CourseInstanceReference,
    assignment: AssignmentReference,
) -> Result<LiveAssignmentAttemptResponse, StartError> {
    let batch = state
        .delivery
        .prepare_native_assignment_issuance(token, course, assignment)
        .await
        .map_err(StartError::Store)?;
    issue_native_assignment_batch(state, token, batch).await
}

async fn issue_native_assignment_batch(
    state: &StateData,
    token: SessionTokenHash,
    batch: NativeAssignmentIssuanceBatch,
) -> Result<LiveAssignmentAttemptResponse, StartError> {
    if let Some(attempt) = batch.committed_attempt {
        if batch.retained_presentations.len() != attempt.questions.len() {
            return Err(StartError::Invalid);
        }
        let questions = batch
            .retained_presentations
            .into_iter()
            .zip(&attempt.questions)
            .map(|(evidence, issued)| {
                if evidence.question_revision.question_id != issued.question_id
                    || evidence.question_revision.revision_number.get() != issued.revision_number
                    || evidence.question_seed != issued.question_seed
                    || evidence.presentation_nonce != issued.presentation_nonce
                    || evidence.presentation_checksum != issued.presentation_checksum
                {
                    return Err(StartError::Invalid);
                }
                Ok(reproduce_selected_issued_presentation(evidence)?.presentation)
            })
            .collect::<Result<Vec<_>, _>>()?;
        return Ok(LiveAssignmentAttemptResponse {
            assignment_attempt: attempt.assignment_attempt,
            assignment: attempt.assignment,
            attempt_number: attempt.attempt_number,
            resumed: batch.attempt_was_resumed,
            title: attempt.title,
            instructions: attempt.instructions,
            questions,
        });
    }
    let sources = &batch.ple_sources;
    let webwork_sources = &batch.webwork_sources;
    if sources.is_empty() && webwork_sources.is_empty() {
        return Err(StartError::Invalid);
    }

    let prepared = if !batch.requires_presentation_commit() {
        Vec::new()
    } else {
        let mut values = issue_new_presentations(&state.objects, sources).await?;
        values.extend(issue_new_webwork_presentations(state, webwork_sources).await?);
        values.sort_by_key(|value| value.position);
        values
    };
    let attempt = state
        .delivery
        .commit_native_assignment_issuance(token, batch.assignment_attempt_id, prepared)
        .await
        .map_err(StartError::Store)?;
    let questions = rebuild_committed_attempt_presentations(state, token, &attempt).await?;
    Ok(LiveAssignmentAttemptResponse {
        assignment_attempt: attempt.assignment_attempt,
        assignment: attempt.assignment,
        attempt_number: attempt.attempt_number,
        resumed: batch.attempt_was_resumed,
        title: attempt.title,
        instructions: attempt.instructions,
        questions,
    })
}

async fn rebuild_committed_attempt_presentations(
    state: &StateData,
    token: SessionTokenHash,
    attempt: &LiveAssignmentAttempt,
) -> Result<Vec<QuestionPresentation>, StartError> {
    let mut presentations = Vec::with_capacity(attempt.questions.len());
    for issued in &attempt.questions {
        let evidence = state
            .delivery
            .student_assignment_attempt_presentation_evidence(
                token,
                attempt.assignment_attempt,
                issued.position,
            )
            .await
            .map_err(StartError::Store)?;
        if evidence.question_revision.question_id != issued.question_id
            || evidence.question_revision.revision_number.get() != issued.revision_number
            || evidence.question_seed != issued.question_seed
            || evidence.presentation_nonce != issued.presentation_nonce
            || evidence.presentation_checksum != issued.presentation_checksum
        {
            return Err(StartError::Invalid);
        }
        presentations.push(reproduce_selected_issued_presentation(evidence)?.presentation);
    }
    Ok(presentations)
}

async fn issue_new_presentations(
    objects: &S3ObjectStore,
    sources: &[NativePleIssuanceSource],
) -> Result<Vec<NativePresentationInput>, StartError> {
    let backend = PleQuestionBackend::new();
    let mut inputs = Vec::with_capacity(sources.len());
    for source in sources {
        let seed = QuestionSeed::new(source.question_seed.ok_or(StartError::Invalid)?);
        let resolved = resolve_source(objects, source).await?;
        let issued = backend
            .issue_question_json(&resolved, seed)
            .map_err(|_| StartError::Invalid)?;
        let assets = question_asset_renditions(source);
        let presentation = build_question_presentation(&issued.presentation, &assets)
            .map_err(|_| StartError::Unavailable)?;
        inputs.push(NativePresentationInput {
            issued_question_id: source
                .issued_question_id
                .ok_or(StartError::Invalid)?
                .to_string(),
            assignment_entry_id: source.assignment_entry_id.clone(),
            position: source.position,
            question_id: source.question_id.clone(),
            revision_number: source.revision_number,
            question_seed: seed.value(),
            parameter_hash: issued.parameter_hash,
            reproduction_details: serde_json::to_value(issued.reproduction_details)
                .map_err(|_| StartError::Invalid)?,
            presentation: serde_json::to_value(&presentation.presentation)
                .map_err(|_| StartError::Invalid)?,
            presentation_nonce: presentation.presentation.presentation_nonce.to_hex(),
            presentation_checksum: presentation.checksum.to_hex(),
            response_item_bindings:
                question_model::presentation::extract_durable_response_item_bindings(&presentation)
                    .map_err(|_| StartError::Invalid)?,
            question_asset_renditions: source.question_asset_renditions.clone(),
            replay_details: None,
        });
    }
    Ok(inputs)
}

async fn issue_new_webwork_presentations(
    state: &StateData,
    sources: &[NativeWebworkIssuanceSource],
) -> Result<Vec<NativePresentationInput>, StartError> {
    let mut inputs = Vec::with_capacity(sources.len());
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|_| StartError::Unavailable)?;
    let created_at = Timestamp::from_unix_millis(
        i64::try_from(now.as_millis()).map_err(|_| StartError::Unavailable)?,
    );
    for source in sources {
        let seed = QuestionSeed::new(source.question_seed);
        let issued = state
            .webwork
            .issue(
                seed,
                &resolve_webwork_source(&state.objects, source).await?,
                created_at,
            )
            .await
            .map_err(|_| StartError::Unavailable)?;
        let replay = issued.replay.ok_or(StartError::Invalid)?;
        let presentation = build_question_presentation(
            &issued.presentation,
            &question_asset_renditions_from_ready(&source.question_asset_renditions),
        )
        .map_err(|_| StartError::Unavailable)?;
        inputs.push(NativePresentationInput {
            issued_question_id: source
                .issued_question_id
                .ok_or(StartError::Invalid)?
                .to_string(),
            assignment_entry_id: source.assignment_entry_id.clone(),
            position: source.position,
            question_id: source.question_id.clone(),
            revision_number: source.revision_number,
            question_seed: seed.value(),
            parameter_hash: issued.parameter_hash,
            reproduction_details: serde_json::to_value(issued.reproduction_details)
                .map_err(|_| StartError::Invalid)?,
            presentation: serde_json::to_value(&presentation.presentation)
                .map_err(|_| StartError::Invalid)?,
            presentation_nonce: presentation.presentation.presentation_nonce.to_hex(),
            presentation_checksum: presentation.checksum.to_hex(),
            response_item_bindings:
                question_model::presentation::extract_durable_response_item_bindings(&presentation)
                    .map_err(|_| StartError::Invalid)?,
            question_asset_renditions: source.question_asset_renditions.clone(),
            replay_details: Some(serde_json::to_value(replay).map_err(|_| StartError::Invalid)?),
        });
    }
    Ok(inputs)
}

async fn resolve_webwork_source(
    objects: &S3ObjectStore,
    source: &NativeWebworkIssuanceSource,
) -> Result<ResolvedWebworkQuestionSource, StartError> {
    let object =
        uuid::Uuid::parse_str(&source.source_object_id).map_err(|_| StartError::Invalid)?;
    let revision = QuestionRevisionReference {
        question_id: source.question_id.clone(),
        revision_number: QuestionRevisionNumber::new(source.revision_number)
            .map_err(|_| StartError::Invalid)?,
    };
    let binding = WebworkQuestionSourceBinding::new(revision, source.webwork_pg_path.clone())
        .map_err(|_| StartError::Invalid)?;
    ResolvedWebworkQuestionSource::resolve(
        objects,
        binding,
        SourceObjectReference {
            object: ObjectId::from_uuid(object),
        },
        SourceObjectChecksum::parse(source.source_object_checksum.clone())
            .map_err(|_| StartError::Invalid)?,
    )
    .await
    .map_err(|_| StartError::Unavailable)
}

pub(super) async fn resolve_source(
    objects: &S3ObjectStore,
    source: &NativePleIssuanceSource,
) -> Result<ResolvedPleQuestionJsonSource, StartError> {
    let object =
        uuid::Uuid::parse_str(&source.source_object_id).map_err(|_| StartError::Invalid)?;
    let revision = QuestionRevisionReference {
        question_id: source.question_id.clone(),
        revision_number: QuestionRevisionNumber::new(source.revision_number)
            .map_err(|_| StartError::Invalid)?,
    };
    let checksum = SourceObjectChecksum::parse(source.source_object_checksum.clone())
        .map_err(|_| StartError::Invalid)?;
    ResolvedPleQuestionJsonSource::resolve(
        objects,
        revision,
        SourceObjectReference {
            object: ObjectId::from_uuid(object),
        },
        checksum,
    )
    .await
    .map_err(|_| StartError::Unavailable)
}

// Route handlers return this response immediately; boxing it would add an
// allocation and require every handler, including the submission boundary, to
// unwrap it solely to preserve Axum's `Response` return type.
#[allow(clippy::result_large_err)]
pub(super) fn refs(
    course: &str,
    assignment: &str,
) -> Result<(CourseInstanceReference, AssignmentReference), Response> {
    Ok((
        CourseInstanceReference::from_str(course).map_err(|_| concealed())?,
        AssignmentReference::from_str(assignment).map_err(|_| concealed())?,
    ))
}

pub(super) async fn student(
    state: &StateData,
    headers: &HeaderMap,
) -> Result<SessionTokenHash, Box<Response>> {
    student_with_sessions(state.sessions.as_ref(), headers).await
}

pub(super) async fn student_with_sessions(
    sessions: &PostgresSessionStore,
    headers: &HeaderMap,
) -> Result<SessionTokenHash, Box<Response>> {
    match resolve_session(sessions, cookie(headers).as_deref()).await {
        Ok(value) if value.record.product_role == ProductRole::Student => Ok(value.session_hash),
        Ok(_) | Err(AuthError::Unauthenticated) => Err(Box::new(concealed())),
        Err(AuthError::Unavailable(_) | AuthError::Randomness(_)) => Err(Box::new(error(
            StatusCode::SERVICE_UNAVAILABLE,
            "Student Assignment delivery authentication unavailable",
        ))),
    }
}

fn cookie(headers: &HeaderMap) -> Option<String> {
    let values = headers
        .get_all(COOKIE)
        .iter()
        .map(|value| value.to_str().ok())
        .collect::<Option<Vec<_>>>()?;
    (!values.is_empty()).then(|| values.join("; "))
}

pub(super) fn store_error(value: StoreError) -> Response {
    match value {
        StoreError::NotFound | StoreError::Forbidden | StoreError::OwnershipMismatch => concealed(),
        StoreError::Conflict | StoreError::RetryableTransaction => {
            error(StatusCode::PRECONDITION_FAILED, "Assignment start changed")
        }
        StoreError::LifecycleConflict => error(
            StatusCode::CONFLICT,
            "Assignment is not available for a new Attempt",
        ),
        StoreError::InvalidRecord(_) => error(
            StatusCode::UNPROCESSABLE_ENTITY,
            "Assignment start is invalid",
        ),
        StoreError::AlreadyExists => error(StatusCode::CONFLICT, "Assignment start conflict"),
        StoreError::AssignmentActivity(_) | StoreError::TimedOut | StoreError::Unavailable(_) => {
            error(
                StatusCode::SERVICE_UNAVAILABLE,
                "Student Assignment delivery unavailable",
            )
        }
    }
}

pub(super) fn submission_store_error(value: StoreError) -> Response {
    match value {
        StoreError::NotFound | StoreError::Forbidden | StoreError::OwnershipMismatch => concealed(),
        StoreError::Conflict | StoreError::AlreadyExists | StoreError::RetryableTransaction => {
            error(
                StatusCode::PRECONDITION_FAILED,
                "Question Submission changed",
            )
        }
        StoreError::LifecycleConflict => error(
            StatusCode::CONFLICT,
            "Question Submission lifecycle conflict",
        ),
        StoreError::InvalidRecord(_) => error(
            StatusCode::UNPROCESSABLE_ENTITY,
            "Student Response is invalid",
        ),
        StoreError::AssignmentActivity(_) | StoreError::TimedOut | StoreError::Unavailable(_) => {
            error(
                StatusCode::SERVICE_UNAVAILABLE,
                "Question Submission unavailable",
            )
        }
    }
}

pub(super) fn concealed() -> Response {
    error(StatusCode::NOT_FOUND, "Assignment not found")
}
pub(super) fn error(status: StatusCode, message: &'static str) -> Response {
    crate::auth::no_store((status, message).into_response())
}

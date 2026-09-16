//! Student Assessment Access and answer-free Assessment start routes.

use std::{str::FromStr, sync::Arc};

use adapter_ple::ResolvedPleQuestionJsonSource;
use adapter_webwork::{
    HttpWebworkRenderer, ResolvedWebworkQuestionSource, WebworkAdapter,
    WebworkQuestionSourceBinding,
};
use axum::{
    Json, Router,
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode, header::COOKIE},
    response::{IntoResponse, Response},
    routing::{get, post, put},
};
use browser_api_contract::assessment_delivery::StudentQuestionPresentation;
use learning_data_access::{
    LiveAssessmentAttempt, LiveAssessmentDeliveryStore, NativeAssessmentIssuanceBatch,
    NativePleIssuanceSource, NativePresentationInput, NativeWebworkIssuanceSource,
    QuestionIssuanceReproductionInput, SessionTokenHash, StoreError,
    postgres::{PostgresLiveAssessmentDeliveryStore, PostgresSessionStore},
};
use objects::s3::S3ObjectStore;
use question_model::presentation::build_question_presentation;
use question_model::question_library::QuestionBackendInterface;
use question_model::{
    AssessmentAttemptReference, AssessmentReference, CourseInstanceReference, ObjectId,
    ProductRole, QuestionBackend, QuestionBackendCapabilities, QuestionPresentation,
    QuestionPresentationChecksum, QuestionRevisionNumber, QuestionRevisionReference,
    SourceObjectChecksum, SourceObjectReference, StudentResponse,
};
use serde::Serialize;

use crate::auth::{AuthError, resolve_session};

mod context;
pub(crate) mod direct_finalization;
mod history;
mod history_response;
mod ple_shell;
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
pub(crate) struct StateData {
    pub(crate) sessions: Arc<PostgresSessionStore>,
    pub(crate) delivery: PostgresLiveAssessmentDeliveryStore,
    pub(crate) objects: S3ObjectStore,
    pub(crate) webwork: Arc<WebworkAdapter<HttpWebworkRenderer>>,
    /// Validated deployment-owned origin used to scope C858's exact WASM CSP
    /// source. Request headers never select a CSP authority.
    pub(crate) browser_origin: Arc<str>,
}

/// Registers Student-only Assessment Access and initial start routes.
pub fn assessment_delivery_router(
    sessions: Arc<PostgresSessionStore>,
    delivery: PostgresLiveAssessmentDeliveryStore,
    objects: S3ObjectStore,
    webwork: Arc<WebworkAdapter<HttpWebworkRenderer>>,
    browser_origin: Arc<str>,
) -> Router {
    Router::new()
        .route(
            "/api/course-instances/{course}/assessments/{assessment}/access",
            get(access),
        )
        .route(
            "/api/course-instances/{course}/assessments/{assessment}/start",
            post(start),
        )
        .route(
            "/api/assessment-attempts/{assessment_attempt}/student-progress",
            get(student_progress),
        )
        .route(
            "/api/assessment-attempts/{assessment_attempt}/history",
            get(history::student_history),
        )
        .route(
            "/api/assessment-attempts/{assessment_attempt}/context",
            get(context::student_context),
        )
        .route(
            "/api/assessment-attempts/{assessment_attempt}/student-question",
            get(student_question),
        )
        .route(
            "/api/assessment-attempts/{assessment_attempt}/questions/{position}/document",
            get(crate::webwork_document_route::document),
        )
        .route(
            "/api/assessment-attempts/{assessment_attempt}/questions/{position}/author-content-document",
            get(crate::author_content_document_route::document),
        )
        .route(
            "/api/assessment-attempts/{assessment_attempt}/responses/{position}",
            put(submission::save_selected_response),
        )
        .route(
            "/api/assessment-attempts/{assessment_attempt}/submission",
            post(submission::finalize_assessment_attempt),
        )
        .with_state(StateData {
            sessions,
            delivery,
            objects,
            webwork,
            browser_origin,
        })
}

async fn student_progress(
    State(state): State<StateData>,
    headers: HeaderMap,
    Path(assessment_attempt): Path<String>,
) -> Response {
    let assessment_attempt = match AssessmentAttemptReference::from_str(&assessment_attempt) {
        Ok(value) => value,
        Err(_) => return concealed(),
    };
    let token = match student(&state, &headers).await {
        Ok(value) => value,
        Err(value) => return *value,
    };
    match state
        .delivery
        .student_assessment_attempt_progress(token, assessment_attempt)
        .await
    {
        Ok(value) => crate::auth::no_store(Json(value).into_response()),
        Err(value) => store_error(value),
    }
}

async fn student_question(
    State(state): State<StateData>,
    headers: HeaderMap,
    Path(assessment_attempt): Path<String>,
    Query(query): Query<PositionQuery>,
) -> Response {
    let assessment_attempt = match AssessmentAttemptReference::from_str(&assessment_attempt) {
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
        .student_assessment_attempt_presentation_evidence(token, assessment_attempt, query.position)
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
                "Student Assessment delivery unavailable",
            );
        }
        Err(StartError::Invalid) => return concealed(),
    };
    let saved_response = match state
        .delivery
        .student_assessment_attempt_saved_response(token, assessment_attempt, query.position)
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
                author_content_digest: issued.presentation.author_content_digest,
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
    evidence: learning_data_access::StudentAssessmentAttemptPresentationEvidence,
) -> Result<question_model::presentation::IssuedQuestionPresentation, StartError> {
    let presentation = serde_json::from_value::<QuestionPresentation>(evidence.presentation)
        .map_err(|_| StartError::Invalid)?;
    match (
        presentation.author_content_digest.as_deref(),
        evidence.author_content.as_ref(),
    ) {
        (None, None) => {}
        (Some(expected), Some(author_content)) if expected == author_content.digest() => {}
        _ => return Err(StartError::Invalid),
    }
    if presentation.question_revision != evidence.question_revision
        || presentation.presentation_nonce.to_hex() != evidence.presentation_nonce
    {
        return Err(StartError::Invalid);
    }
    let checksum = QuestionPresentationChecksum::parse_hex(&evidence.presentation_checksum)
        .map_err(|_| StartError::Invalid)?;
    let issued = question_model::presentation::rebuild_question_presentation_with_reproduction_and_author_content(
        &presentation,
        &question_asset_renditions_from_ready(&evidence.question_asset_renditions),
        evidence.reproduction,
        evidence.author_content,
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
    Path((course, assessment)): Path<(String, String)>,
) -> Response {
    let (course, assessment) = match refs(&course, &assessment) {
        Ok(v) => v,
        Err(v) => return v,
    };
    let token = match student(&state, &headers).await {
        Ok(v) => v,
        Err(v) => return *v,
    };
    match state
        .delivery
        .live_assessment_access(token, course, assessment)
        .await
    {
        // ASVS 4.1.1 and 14.2.6: the Store value is the closed, answer-free
        // current-Student projection; Json supplies its matching media type.
        Ok(value) => crate::auth::no_store(Json(value).into_response()),
        Err(value) => store_error(value),
    }
}

async fn start(
    State(state): State<StateData>,
    headers: HeaderMap,
    Path((course, assessment)): Path<(String, String)>,
) -> Response {
    let (course, assessment) = match refs(&course, &assessment) {
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
    match issue_native_ple_presentation(&state, token, course, assessment).await {
        Ok(value) => crate::auth::no_store((StatusCode::CREATED, Json(value)).into_response()),
        Err(StartError::Store(value)) => store_error(value),
        Err(StartError::Unavailable) => error(
            StatusCode::SERVICE_UNAVAILABLE,
            "Student Assessment delivery unavailable",
        ),
        Err(StartError::Invalid) => error(
            StatusCode::UNPROCESSABLE_ENTITY,
            "Assessment start is invalid",
        ),
    }
}

/// Browser response: all presentation fields are intentionally answer-free;
/// the store's private attempt ID, source, binding checksum, and reproduction
/// evidence never cross this type boundary.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct LiveAssessmentAttemptResponse {
    assessment_attempt: AssessmentAttemptReference,
    assessment: AssessmentReference,
    attempt_number: u32,
    resumed: bool,
    title: String,
    instructions: String,
    questions: Vec<QuestionPresentation>,
}

pub(crate) enum StartError {
    Store(StoreError),
    Invalid,
    Unavailable,
}

async fn issue_native_ple_presentation(
    state: &StateData,
    token: SessionTokenHash,
    course: CourseInstanceReference,
    assessment: AssessmentReference,
) -> Result<LiveAssessmentAttemptResponse, StartError> {
    let batch = state
        .delivery
        .prepare_native_assessment_issuance(token, course, assessment)
        .await
        .map_err(StartError::Store)?;
    issue_native_assessment_batch(state, token, batch).await
}

async fn issue_native_assessment_batch(
    state: &StateData,
    token: SessionTokenHash,
    batch: NativeAssessmentIssuanceBatch,
) -> Result<LiveAssessmentAttemptResponse, StartError> {
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
                    || evidence.reproduction != issued.reproduction
                    || evidence.presentation_nonce != issued.presentation_nonce
                    || evidence.presentation_checksum != issued.presentation_checksum
                {
                    return Err(StartError::Invalid);
                }
                Ok(reproduce_selected_issued_presentation(evidence)?.presentation)
            })
            .collect::<Result<Vec<_>, _>>()?;
        return Ok(LiveAssessmentAttemptResponse {
            assessment_attempt: attempt.assessment_attempt,
            assessment: attempt.assessment,
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
        .commit_native_assessment_issuance(token, batch.assessment_attempt_id, prepared)
        .await
        .map_err(StartError::Store)?;
    let questions = rebuild_committed_attempt_presentations(state, token, &attempt).await?;
    Ok(LiveAssessmentAttemptResponse {
        assessment_attempt: attempt.assessment_attempt,
        assessment: attempt.assessment,
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
    attempt: &LiveAssessmentAttempt,
) -> Result<Vec<QuestionPresentation>, StartError> {
    let mut presentations = Vec::with_capacity(attempt.questions.len());
    for issued in &attempt.questions {
        let evidence = state
            .delivery
            .student_assessment_attempt_presentation_evidence(
                token,
                attempt.assessment_attempt,
                issued.position,
            )
            .await
            .map_err(StartError::Store)?;
        if evidence.question_revision.question_id != issued.question_id
            || evidence.question_revision.revision_number.get() != issued.revision_number
            || evidence.reproduction != issued.reproduction
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
    // The common interface makes this an explicit PLE-owned lifecycle path;
    // it is not a cue to interpret another adapter's private controls.
    let backend = ple_shell::native_ple_backend(QuestionBackendInterface::new(
        QuestionBackend::Ple,
        QuestionBackendCapabilities::none(),
    ))?;
    let mut inputs = Vec::with_capacity(sources.len());
    for source in sources {
        // ASVS 2.2.1/2.2.2: only the trusted server/store boundary may
        // classify native source reproduction; a static source never gains a seed.
        if source.reproduction != QuestionIssuanceReproductionInput::Static {
            return Err(StartError::Invalid);
        }
        let resolved = resolve_source(objects, source).await?;
        let issued = backend
            .issue_question_json(&resolved)
            .map_err(|_| StartError::Invalid)?;
        let assets = question_asset_renditions(source);
        let presentation = build_question_presentation(&issued.presentation, &assets)
            .map_err(|_| StartError::Unavailable)?;
        if presentation.reproduction != question_model::QuestionReproduction::Static {
            return Err(StartError::Invalid);
        }
        let response_item_bindings =
            question_model::presentation::extract_durable_response_item_bindings(&presentation)
                .map_err(|_| StartError::Invalid)?;
        inputs.push(NativePresentationInput {
            issued_question_id: source
                .issued_question_id
                .ok_or(StartError::Invalid)?
                .to_string(),
            assessment_entry_id: source.assessment_entry_id.clone(),
            position: source.position,
            question_id: source.question_id.clone(),
            revision_number: source.revision_number,
            reproduction: presentation.reproduction,
            reproduction_details: serde_json::to_value(issued.reproduction_details)
                .map_err(|_| StartError::Invalid)?,
            presentation: serde_json::to_value(&presentation.presentation)
                .map_err(|_| StartError::Invalid)?,
            presentation_nonce: presentation.presentation.presentation_nonce.to_hex(),
            presentation_checksum: presentation.checksum.to_hex(),
            author_content: presentation.author_content.clone(),
            response_item_bindings,
            question_asset_renditions: source.question_asset_renditions.clone(),
            issued_capability: "ple_question_json_presentation".to_string(),
            backend_document: None,
        });
    }
    Ok(inputs)
}

async fn issue_new_webwork_presentations(
    state: &StateData,
    sources: &[NativeWebworkIssuanceSource],
) -> Result<Vec<NativePresentationInput>, StartError> {
    let mut inputs = Vec::with_capacity(sources.len());
    for source in sources {
        // ASVS 2.2.1/2.2.2: renderer issuance consumes only a trusted
        // Seeded input, then retains the renderer-produced paired evidence.
        let seed = match &source.reproduction {
            QuestionIssuanceReproductionInput::Seeded { question_seed } => *question_seed,
            QuestionIssuanceReproductionInput::Static => return Err(StartError::Invalid),
        };
        let issued = state
            .webwork
            .issue(seed, &resolve_webwork_source(&state.objects, source).await?)
            .await
            .map_err(|_| StartError::Unavailable)?;
        let document = String::from_utf8(issued.document).map_err(|_| StartError::Invalid)?;
        let presentation = build_question_presentation(
            &issued.presentation,
            &question_asset_renditions_from_ready(&source.question_asset_renditions),
        )
        .map_err(|_| StartError::Unavailable)?;
        if presentation.reproduction.question_seed() != Some(seed) {
            return Err(StartError::Invalid);
        }
        let response_item_bindings =
            question_model::presentation::extract_durable_response_item_bindings(&presentation)
                .map_err(|_| StartError::Invalid)?;
        inputs.push(NativePresentationInput {
            issued_question_id: source
                .issued_question_id
                .ok_or(StartError::Invalid)?
                .to_string(),
            assessment_entry_id: source.assessment_entry_id.clone(),
            position: source.position,
            question_id: source.question_id.clone(),
            revision_number: source.revision_number,
            reproduction: presentation.reproduction,
            reproduction_details: serde_json::to_value(issued.reproduction_details)
                .map_err(|_| StartError::Invalid)?,
            presentation: serde_json::to_value(&presentation.presentation)
                .map_err(|_| StartError::Invalid)?,
            presentation_nonce: presentation.presentation.presentation_nonce.to_hex(),
            presentation_checksum: presentation.checksum.to_hex(),
            author_content: None,
            response_item_bindings,
            question_asset_renditions: source.question_asset_renditions.clone(),
            issued_capability: "webwork_presentation".to_string(),
            backend_document: Some(document),
        });
    }
    Ok(inputs)
}

pub(crate) async fn resolve_webwork_source(
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
    assessment: &str,
) -> Result<(CourseInstanceReference, AssessmentReference), Response> {
    Ok((
        CourseInstanceReference::from_str(course).map_err(|_| concealed())?,
        AssessmentReference::from_str(assessment).map_err(|_| concealed())?,
    ))
}

pub(crate) async fn student(
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
            "Student Assessment delivery authentication unavailable",
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
            error(StatusCode::PRECONDITION_FAILED, "Assessment start changed")
        }
        StoreError::LifecycleConflict => error(
            StatusCode::CONFLICT,
            "Assessment is not available for a new Attempt",
        ),
        StoreError::InvalidRecord(_) => error(
            StatusCode::UNPROCESSABLE_ENTITY,
            "Assessment start is invalid",
        ),
        StoreError::AlreadyExists => error(StatusCode::CONFLICT, "Assessment start conflict"),
        StoreError::AssessmentActivity(_)
        | StoreError::TimedOut
        | StoreError::LeaseLost
        | StoreError::Unavailable(_) => error(
            StatusCode::SERVICE_UNAVAILABLE,
            "Student Assessment delivery unavailable",
        ),
    }
}

pub(super) fn submission_store_error(value: StoreError) -> Response {
    match value {
        StoreError::NotFound | StoreError::Forbidden | StoreError::OwnershipMismatch => concealed(),
        StoreError::Conflict | StoreError::AlreadyExists | StoreError::RetryableTransaction => {
            error(
                StatusCode::PRECONDITION_FAILED,
                "Question response changed",
            )
        }
        StoreError::LifecycleConflict => error(
            StatusCode::CONFLICT,
            "Question response lifecycle conflict",
        ),
        StoreError::InvalidRecord(_) => error(
            StatusCode::UNPROCESSABLE_ENTITY,
            "Student Response is invalid",
        ),
        StoreError::AssessmentActivity(_)
        | StoreError::TimedOut
        | StoreError::LeaseLost
        | StoreError::Unavailable(_) => error(
            StatusCode::SERVICE_UNAVAILABLE,
            "Question response unavailable",
        ),
    }
}

pub(crate) fn concealed() -> Response {
    error(StatusCode::NOT_FOUND, "Assessment not found")
}
pub(super) fn error(status: StatusCode, message: &'static str) -> Response {
    crate::auth::no_store((status, message).into_response())
}

//! Student Assignment Access and answer-free Assignment start routes.

use std::{collections::BTreeMap, str::FromStr, sync::Arc};

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
use learning_data_access::{
    LiveAssignmentAttempt, LiveAssignmentDeliveryStore, NativePleIssuanceSource,
    NativePlePresentationInput, NativeWebworkIssuanceSource, NativeWebworkPresentationInput,
    SessionTokenHash, StoreError,
    postgres::{
        PostgresLiveAssignmentDeliveryStore, PostgresNativePleSubmissionStore, PostgresSessionStore,
    },
};
use objects::s3::S3ObjectStore;
use question_model::{
    AssignmentAttemptReference, AssignmentReference, CourseInstanceReference, ObjectId,
    ProductRole, QuestionAssetReference, QuestionAssetRendition, QuestionContentBlock, QuestionId,
    QuestionPresentation, QuestionPresentationBinding, QuestionPresentationChecksum,
    QuestionPresentationResponseFormat, QuestionRevisionNumber, QuestionRevisionReference,
    SourceObjectChecksum, SourceObjectReference, StudentResponse, Timestamp,
};
use question_model::{
    generation::QuestionSeed,
    presentation::{build_question_presentation, reproduce_question_presentation},
};
use serde::Serialize;

use crate::auth::{AuthError, resolve_session};

mod context;
mod history;
mod history_response;
mod submission;

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
        .student_assignment_attempt_presentation_source(token, assignment_attempt, query.position)
        .await
    {
        Ok(value) => value,
        Err(value) => return store_error(value),
    };
    let issued = match reproduce_selected_issued_presentation(&state, source).await {
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
            presentation: issued.presentation.into(),
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

/// Browser-safe selected Question facts. Pinned source, seed, nonce, checksum,
/// revision, and authored title remain on the server-side replay boundary.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct StudentQuestionPresentation {
    prompt: Vec<QuestionContentBlock>,
    response: QuestionPresentationResponseFormat,
}

impl From<QuestionPresentation> for StudentQuestionPresentation {
    fn from(value: QuestionPresentation) -> Self {
        Self {
            prompt: value.prompt,
            response: value.response,
        }
    }
}

pub(super) async fn reproduce_selected_issued_presentation(
    state: &StateData,
    source: learning_data_access::StudentAssignmentAttemptPresentationSource,
) -> Result<question_model::presentation::IssuedQuestionPresentation, StartError> {
    match source {
        learning_data_access::StudentAssignmentAttemptPresentationSource::Ple {
            source, ..
        } => {
            let resolved = resolve_source(&state.objects, &source).await?;
            history::reproduce_ple_issued_presentation(&source, &resolved)
        }
        learning_data_access::StudentAssignmentAttemptPresentationSource::Webwork {
            source,
            question_seed,
            presentation_nonce,
            presentation_checksum,
            ..
        } => {
            let issued = state
                .webwork
                .reproduce(
                    QuestionSeed::new(question_seed),
                    &resolve_webwork_source(&state.objects, &source).await?,
                )
                .await
                .map_err(|_| StartError::Unavailable)?;
            let nonce = question_model::QuestionPresentationNonce::parse(&presentation_nonce)
                .map_err(|_| StartError::Invalid)?;
            let checksum = QuestionPresentationChecksum::parse_hex(&presentation_checksum)
                .map_err(|_| StartError::Invalid)?;
            reproduce_question_presentation(
                &issued.presentation,
                &[],
                QuestionPresentationBinding::new(nonce, checksum),
            )
            .map_err(|_| StartError::Invalid)
        }
    }
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
    let sources = state
        .delivery
        .prepare_native_ple_issuance(token, course, assignment)
        .await
        .map_err(StartError::Store)?;
    let webwork_sources = state
        .delivery
        .prepare_native_webwork_issuance(token, course, assignment)
        .await
        .map_err(StartError::Store)?;
    if !sources.is_empty() && !webwork_sources.is_empty() {
        return issue_mixed_native_presentation(
            state,
            token,
            course,
            assignment,
            &sources,
            &webwork_sources,
        )
        .await;
    }
    if sources.is_empty() {
        return issue_native_webwork_presentation_from_sources(
            state,
            token,
            course,
            assignment,
            &webwork_sources,
        )
        .await;
    }
    let resumed = sources.first().ok_or(StartError::Invalid)?.resumed;
    if sources.iter().any(|source| source.resumed != resumed) {
        return Err(StartError::Invalid);
    }

    let source_by_entry = sources
        .iter()
        .map(|source| {
            (
                (
                    source.assignment_entry_id.as_str(),
                    &source.question_id,
                    source.revision_number,
                ),
                source,
            )
        })
        .collect::<BTreeMap<_, _>>();
    if source_by_entry.len() != sources.len() {
        return Err(StartError::Invalid);
    }

    let prepared = if resumed {
        Vec::new()
    } else {
        issue_new_presentations(&state.objects, &sources).await?
    };
    let attempt = state
        .delivery
        .commit_native_ple_issuance(token, course, assignment, prepared)
        .await
        .map_err(StartError::Store)?;
    let questions = reproduce_presentations(&state.objects, &source_by_entry, &attempt).await?;
    Ok(LiveAssignmentAttemptResponse {
        assignment_attempt: attempt.assignment_attempt,
        assignment: attempt.assignment,
        attempt_number: attempt.attempt_number,
        resumed: attempt.resumed,
        title: attempt.title,
        instructions: attempt.instructions,
        questions,
    })
}

async fn issue_new_presentations(
    objects: &S3ObjectStore,
    sources: &[NativePleIssuanceSource],
) -> Result<Vec<NativePlePresentationInput>, StartError> {
    let backend = PleQuestionBackend::new();
    let mut inputs = Vec::with_capacity(sources.len());
    for source in sources {
        let mut seed = [0_u8; 8];
        getrandom::fill(&mut seed).map_err(|_| StartError::Unavailable)?;
        // Question Presentation exposes the exact seed, so retain only the
        // 53 bits every browser Number can represent without rounding.
        let seed = QuestionSeed::new(u64::from_be_bytes(seed) & ((1_u64 << 53) - 1));
        let resolved = resolve_source(objects, source).await?;
        let issued = backend
            .issue_question_json(&resolved, seed)
            .map_err(|_| StartError::Invalid)?;
        let assets = question_asset_renditions(source);
        let presentation = build_question_presentation(&issued.presentation, &assets)
            .map_err(|_| StartError::Unavailable)?;
        inputs.push(NativePlePresentationInput {
            issued_question_id: uuid::Uuid::now_v7().to_string(),
            assignment_entry_id: source.assignment_entry_id.clone(),
            position: source.position,
            question_id: source.question_id.clone(),
            revision_number: source.revision_number,
            question_seed: seed.value(),
            parameter_hash: issued.parameter_hash,
            reproduction_details: serde_json::to_value(issued.reproduction_details)
                .map_err(|_| StartError::Invalid)?,
            presentation_nonce: presentation.presentation.presentation_nonce.to_hex(),
            presentation_checksum: presentation.checksum.to_hex(),
            question_asset_renditions: source.question_asset_renditions.clone(),
        });
    }
    Ok(inputs)
}

async fn reproduce_presentations(
    objects: &S3ObjectStore,
    source_by_entry: &BTreeMap<(&str, &QuestionId, u32), &NativePleIssuanceSource>,
    attempt: &LiveAssignmentAttempt,
) -> Result<Vec<QuestionPresentation>, StartError> {
    let backend = PleQuestionBackend::new();
    let mut questions = Vec::with_capacity(attempt.questions.len());
    for issued_attempt in &attempt.questions {
        let source = source_by_entry
            .get(&(
                issued_attempt.assignment_entry_id.as_str(),
                &issued_attempt.question_id,
                issued_attempt.revision_number,
            ))
            .copied()
            .ok_or(StartError::Invalid)?;
        if source.question_id != issued_attempt.question_id
            || source.revision_number != issued_attempt.revision_number
        {
            return Err(StartError::Invalid);
        }
        let resolved = resolve_source(objects, source).await?;
        let issued = backend
            .issue_question_json(&resolved, QuestionSeed::new(issued_attempt.question_seed))
            .map_err(|_| StartError::Invalid)?;
        let nonce =
            question_model::QuestionPresentationNonce::parse(&issued_attempt.presentation_nonce)
                .map_err(|_| StartError::Invalid)?;
        let checksum =
            QuestionPresentationChecksum::parse_hex(&issued_attempt.presentation_checksum)
                .map_err(|_| StartError::Invalid)?;
        let assets = question_asset_renditions(source);
        let presentation = reproduce_question_presentation(
            &issued.presentation,
            &assets,
            QuestionPresentationBinding::new(nonce, checksum),
        )
        .map_err(|_| StartError::Invalid)?;
        questions.push(presentation.presentation);
    }
    Ok(questions)
}

/// Issues or reproduces an immutable WeBWorK render.  Renderer form mappings
/// are persisted only by the store's private replay table and never reach the
/// browser response. ASVS 8.2.1/8.2.2.
async fn issue_native_webwork_presentation_from_sources(
    state: &StateData,
    token: SessionTokenHash,
    course: CourseInstanceReference,
    assignment: AssignmentReference,
    sources: &[NativeWebworkIssuanceSource],
) -> Result<LiveAssignmentAttemptResponse, StartError> {
    let resumed = sources.first().ok_or(StartError::Invalid)?.resumed;
    if sources.iter().any(|source| source.resumed != resumed) {
        return Err(StartError::Invalid);
    }
    let by_entry = sources
        .iter()
        .map(|source| {
            (
                (
                    source.assignment_entry_id.as_str(),
                    &source.question_id,
                    source.revision_number,
                ),
                source,
            )
        })
        .collect::<BTreeMap<_, _>>();
    if by_entry.len() != sources.len() {
        return Err(StartError::Invalid);
    }
    let prepared = if resumed {
        Vec::new()
    } else {
        issue_new_webwork_presentations(state, sources).await?
    };
    let attempt = state
        .delivery
        .commit_native_webwork_issuance(token, course, assignment, prepared)
        .await
        .map_err(StartError::Store)?;
    let mut questions = Vec::with_capacity(attempt.questions.len());
    for issued_attempt in &attempt.questions {
        let source = by_entry
            .get(&(
                issued_attempt.assignment_entry_id.as_str(),
                &issued_attempt.question_id,
                issued_attempt.revision_number,
            ))
            .copied()
            .ok_or(StartError::Invalid)?;
        if source.question_id != issued_attempt.question_id
            || source.revision_number != issued_attempt.revision_number
        {
            return Err(StartError::Invalid);
        }
        let issued = state
            .webwork
            .reproduce(
                QuestionSeed::new(issued_attempt.question_seed),
                &resolve_webwork_source(&state.objects, source).await?,
            )
            .await
            .map_err(|_| StartError::Unavailable)?;
        let nonce =
            question_model::QuestionPresentationNonce::parse(&issued_attempt.presentation_nonce)
                .map_err(|_| StartError::Invalid)?;
        let checksum =
            QuestionPresentationChecksum::parse_hex(&issued_attempt.presentation_checksum)
                .map_err(|_| StartError::Invalid)?;
        questions.push(
            reproduce_question_presentation(
                &issued.presentation,
                &[],
                QuestionPresentationBinding::new(nonce, checksum),
            )
            .map_err(|_| StartError::Invalid)?
            .presentation,
        );
    }
    Ok(LiveAssignmentAttemptResponse {
        assignment_attempt: attempt.assignment_attempt,
        assignment: attempt.assignment,
        attempt_number: attempt.attempt_number,
        resumed: attempt.resumed,
        title: attempt.title,
        instructions: attempt.instructions,
        questions,
    })
}

/// A released Assignment may contain both supported native source backends.
/// Build all exact pins before the single locked commit; the commit's
/// set-equality check then prevents issuing a partial Assignment Attempt.
async fn issue_mixed_native_presentation(
    state: &StateData,
    token: SessionTokenHash,
    course: CourseInstanceReference,
    assignment: AssignmentReference,
    ple_sources: &[NativePleIssuanceSource],
    webwork_sources: &[NativeWebworkIssuanceSource],
) -> Result<LiveAssignmentAttemptResponse, StartError> {
    let resumed = ple_sources.first().ok_or(StartError::Invalid)?.resumed;
    if ple_sources.iter().any(|source| source.resumed != resumed)
        || webwork_sources
            .iter()
            .any(|source| source.resumed != resumed)
    {
        return Err(StartError::Invalid);
    }
    let ple_by_entry = ple_sources
        .iter()
        .map(|source| {
            (
                (
                    source.assignment_entry_id.as_str(),
                    &source.question_id,
                    source.revision_number,
                ),
                source,
            )
        })
        .collect::<BTreeMap<_, _>>();
    let webwork_by_entry = webwork_sources
        .iter()
        .map(|source| {
            (
                (
                    source.assignment_entry_id.as_str(),
                    &source.question_id,
                    source.revision_number,
                ),
                source,
            )
        })
        .collect::<BTreeMap<_, _>>();
    if ple_by_entry.len() != ple_sources.len()
        || webwork_by_entry.len() != webwork_sources.len()
        || ple_by_entry
            .keys()
            .any(|entry| webwork_by_entry.contains_key(entry))
    {
        return Err(StartError::Invalid);
    }
    let prepared = if resumed {
        Vec::new()
    } else {
        let mut values = issue_new_presentations(&state.objects, ple_sources)
            .await?
            .into_iter()
            .map(|value| NativeWebworkPresentationInput {
                issued_question_id: value.issued_question_id,
                assignment_entry_id: value.assignment_entry_id,
                position: value.position,
                question_id: value.question_id,
                revision_number: value.revision_number,
                question_seed: value.question_seed,
                parameter_hash: value.parameter_hash,
                reproduction_details: value.reproduction_details,
                presentation_nonce: value.presentation_nonce,
                presentation_checksum: value.presentation_checksum,
                question_asset_renditions: value.question_asset_renditions,
                replay_details: None,
            })
            .collect::<Vec<_>>();
        values.extend(issue_new_webwork_presentations(state, webwork_sources).await?);
        values.sort_by_key(|value| value.position);
        values
    };
    let attempt = state
        .delivery
        .commit_native_webwork_issuance(token, course, assignment, prepared)
        .await
        .map_err(StartError::Store)?;
    let mut questions = Vec::with_capacity(attempt.questions.len());
    for issued_attempt in &attempt.questions {
        let nonce =
            question_model::QuestionPresentationNonce::parse(&issued_attempt.presentation_nonce)
                .map_err(|_| StartError::Invalid)?;
        let checksum =
            QuestionPresentationChecksum::parse_hex(&issued_attempt.presentation_checksum)
                .map_err(|_| StartError::Invalid)?;
        let binding = QuestionPresentationBinding::new(nonce, checksum);
        let presentation = if let Some(source) = ple_by_entry
            .get(&(
                issued_attempt.assignment_entry_id.as_str(),
                &issued_attempt.question_id,
                issued_attempt.revision_number,
            ))
            .copied()
        {
            if source.question_id != issued_attempt.question_id
                || source.revision_number != issued_attempt.revision_number
            {
                return Err(StartError::Invalid);
            }
            let issued = PleQuestionBackend::new()
                .issue_question_json(
                    &resolve_source(&state.objects, source).await?,
                    QuestionSeed::new(issued_attempt.question_seed),
                )
                .map_err(|_| StartError::Invalid)?;
            reproduce_question_presentation(
                &issued.presentation,
                &question_asset_renditions(source),
                binding,
            )
            .map_err(|_| StartError::Invalid)?
            .presentation
        } else if let Some(source) = webwork_by_entry
            .get(&(
                issued_attempt.assignment_entry_id.as_str(),
                &issued_attempt.question_id,
                issued_attempt.revision_number,
            ))
            .copied()
        {
            if source.question_id != issued_attempt.question_id
                || source.revision_number != issued_attempt.revision_number
            {
                return Err(StartError::Invalid);
            }
            let issued = state
                .webwork
                .reproduce(
                    QuestionSeed::new(issued_attempt.question_seed),
                    &resolve_webwork_source(&state.objects, source).await?,
                )
                .await
                .map_err(|_| StartError::Unavailable)?;
            reproduce_question_presentation(&issued.presentation, &[], binding)
                .map_err(|_| StartError::Invalid)?
                .presentation
        } else {
            return Err(StartError::Invalid);
        };
        questions.push(presentation);
    }
    Ok(LiveAssignmentAttemptResponse {
        assignment_attempt: attempt.assignment_attempt,
        assignment: attempt.assignment,
        attempt_number: attempt.attempt_number,
        resumed: attempt.resumed,
        title: attempt.title,
        instructions: attempt.instructions,
        questions,
    })
}

async fn issue_new_webwork_presentations(
    state: &StateData,
    sources: &[NativeWebworkIssuanceSource],
) -> Result<Vec<NativeWebworkPresentationInput>, StartError> {
    let mut inputs = Vec::with_capacity(sources.len());
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|_| StartError::Unavailable)?;
    let created_at = Timestamp::from_unix_millis(
        i64::try_from(now.as_millis()).map_err(|_| StartError::Unavailable)?,
    );
    for source in sources {
        let mut bytes = [0_u8; 8];
        getrandom::fill(&mut bytes).map_err(|_| StartError::Unavailable)?;
        let seed = QuestionSeed::new(u64::from_be_bytes(bytes) & ((1_u64 << 53) - 1));
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
        let presentation = build_question_presentation(&issued.presentation, &[])
            .map_err(|_| StartError::Unavailable)?;
        inputs.push(NativeWebworkPresentationInput {
            issued_question_id: uuid::Uuid::now_v7().to_string(),
            assignment_entry_id: source.assignment_entry_id.clone(),
            position: source.position,
            question_id: source.question_id.clone(),
            revision_number: source.revision_number,
            question_seed: seed.value(),
            parameter_hash: issued.parameter_hash,
            reproduction_details: serde_json::to_value(issued.reproduction_details)
                .map_err(|_| StartError::Invalid)?,
            presentation_nonce: presentation.presentation.presentation_nonce.to_hex(),
            presentation_checksum: presentation.checksum.to_hex(),
            question_asset_renditions: Vec::new(),
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

pub(super) fn question_asset_renditions(
    source: &NativePleIssuanceSource,
) -> Vec<QuestionAssetRendition> {
    question_asset_renditions_from_ready(&source.question_asset_renditions)
}

pub(super) fn question_asset_renditions_from_ready(
    renditions: &[learning_data_access::ReadyQuestionAssetRendition],
) -> Vec<QuestionAssetRendition> {
    renditions
        .iter()
        .map(|rendition| QuestionAssetRendition {
            question_asset: QuestionAssetReference {
                question_asset: rendition.question_asset,
                checksum: rendition.question_asset_checksum.clone(),
            },
            rendition_checksum: rendition.rendition_checksum.clone(),
            intrinsic_width: Some(rendition.intrinsic_width),
            intrinsic_height: Some(rendition.intrinsic_height),
        })
        .collect()
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

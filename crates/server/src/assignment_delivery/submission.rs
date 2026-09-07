//! Private M13/M14 response submission and status handlers.

use axum::{
    Json,
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
};
use learning_data_access::{
    AcceptNativePleSubmission, NativePleSubmissionStatus, NativePleSubmissionStore,
    ResolvedNativePleSubmission, ResolvedWebworkSubmission, SessionTokenHash, StoreError,
    WebworkSubmissionStore,
};
use question_model::{
    AssignmentReference, CourseInstanceReference, ObjectId, QuestionPresentationBinding,
    QuestionPresentationChecksum, QuestionRevisionNumber, QuestionRevisionReference,
    SourceObjectChecksum, SourceObjectReference, StudentResponse,
    presentation::{
        QuestionPresentationNonce, reproduce_question_presentation,
        translate_presentation_response_item_references,
    },
};
use serde::{Deserialize, Serialize};

use super::{
    StartError, StateData, concealed, error, question_asset_renditions_from_ready, refs, student,
    submission_store_error,
};
use adapter_ple::{PleQuestionBackend, ResolvedPleQuestionJsonSource};
use adapter_webwork::{ResolvedWebworkQuestionSource, WebworkQuestionSourceBinding};
use question_model::generation::QuestionSeed;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct NativePleSubmissionRequest {
    response: StudentResponse,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct NativePleSubmissionAcknowledgement {
    presentation_nonce: String,
    grading_state: &'static str,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct NativePleSubmissionStatusResponse {
    grading_state: &'static str,
    correct: Option<bool>,
    points_earned: Option<f64>,
    points_possible: Option<f64>,
}

pub(super) async fn native_ple_submission_status(
    State(state): State<StateData>,
    headers: HeaderMap,
    Path((course, assignment, presentation_nonce)): Path<(String, String, String)>,
) -> Response {
    let (course, assignment) = match refs(&course, &assignment) {
        Ok(value) => value,
        Err(value) => return value,
    };
    let nonce = match QuestionPresentationNonce::parse(&presentation_nonce) {
        Ok(value) => value,
        Err(_) => return concealed(),
    };
    let token = match student(&state, &headers).await {
        Ok(value) => value,
        Err(value) => return *value,
    };
    let response = match state
        .submissions
        .native_ple_submission_status(
            token,
            u64::from(course.number()),
            u64::from(assignment.number()),
            &nonce.to_hex(),
        )
        .await
    {
        Ok(NativePleSubmissionStatus::Pending) => NativePleSubmissionStatusResponse {
            grading_state: "pending",
            correct: None,
            points_earned: None,
            points_possible: None,
        },
        Ok(NativePleSubmissionStatus::InstructorAttention) => NativePleSubmissionStatusResponse {
            grading_state: "instructorAttention",
            correct: None,
            points_earned: None,
            points_possible: None,
        },
        Ok(NativePleSubmissionStatus::Graded {
            correct,
            points_earned,
            points_possible,
        }) => NativePleSubmissionStatusResponse {
            grading_state: "graded",
            correct: Some(correct),
            points_earned: Some(points_earned),
            points_possible: Some(points_possible),
        },
        Err(value) => return submission_store_error(value),
    };
    crate::auth::no_store(Json(response).into_response())
}

pub(super) async fn submit_native_ple_response(
    State(state): State<StateData>,
    headers: HeaderMap,
    Path((course, assignment, presentation_nonce)): Path<(String, String, String)>,
    Json(request): Json<NativePleSubmissionRequest>,
) -> Response {
    let (course, assignment) = match refs(&course, &assignment) {
        Ok(value) => value,
        Err(value) => return value,
    };
    let nonce = match QuestionPresentationNonce::parse(&presentation_nonce) {
        Ok(value) => value,
        Err(_) => return concealed(),
    };
    let token = match student(&state, &headers).await {
        Ok(value) => value,
        Err(value) => return *value,
    };
    let resolved = match state
        .submissions
        .resolve_native_ple_submission(
            token.clone(),
            u64::from(course.number()),
            u64::from(assignment.number()),
            &nonce.to_hex(),
        )
        .await
    {
        Ok(value) => value,
        Err(StoreError::Forbidden) => {
            return submit_webwork_response(
                state,
                token,
                course,
                assignment,
                nonce,
                request.response,
            )
            .await;
        }
        Err(value) => return submission_store_error(value),
    };
    let issued = match reproduce_submission_presentation(&state.objects, &resolved).await {
        Ok(value) => value,
        Err(StartError::Unavailable) => {
            return error(
                StatusCode::SERVICE_UNAVAILABLE,
                "Question Submission unavailable",
            );
        }
        Err(StartError::Invalid | StartError::Store(_)) => return invalid_response(),
    };
    if !domain::validation::validate_presentation_response_format(
        &issued.presentation.response,
        &request.response,
    )
    .is_valid()
    {
        return invalid_response();
    }
    let response = match translate_presentation_response_item_references(&request.response, &issued)
    {
        Ok(value) => value,
        Err(_) => return invalid_response(),
    };
    let student_response = match serde_json::to_value(response) {
        Ok(value) => value,
        Err(_) => return invalid_response(),
    };
    match state
        .submissions
        .accept_native_ple_submission(
            token,
            AcceptNativePleSubmission {
                question_attempt: resolved.question_attempt,
                student_response,
            },
        )
        .await
    {
        Ok(learning_data_access::StudentQuestionSubmissionGradingState::Pending) => {
            acknowledgement(nonce)
        }
        Ok(_) => error(
            StatusCode::SERVICE_UNAVAILABLE,
            "Question Submission unavailable",
        ),
        Err(value) => submission_store_error(value),
    }
}

async fn submit_webwork_response(
    state: StateData,
    token: SessionTokenHash,
    course: CourseInstanceReference,
    assignment: AssignmentReference,
    nonce: QuestionPresentationNonce,
    response: StudentResponse,
) -> Response {
    let resolved = match state
        .webwork_submissions
        .resolve_webwork_submission(
            token.clone(),
            u64::from(course.number()),
            u64::from(assignment.number()),
            &nonce.to_hex(),
        )
        .await
    {
        Ok(value) => value,
        Err(value) => return submission_store_error(value),
    };
    let issued = match reproduce_webwork_submission_presentation(&state, &resolved).await {
        Ok(value) => value,
        Err(StartError::Unavailable) => {
            return error(
                StatusCode::SERVICE_UNAVAILABLE,
                "Question Submission unavailable",
            );
        }
        Err(StartError::Invalid | StartError::Store(_)) => return invalid_response(),
    };
    if !domain::validation::validate_presentation_response_format(
        &issued.presentation.response,
        &response,
    )
    .is_valid()
    {
        return invalid_response();
    }
    let response = match translate_presentation_response_item_references(&response, &issued) {
        Ok(value) => value,
        Err(_) => return invalid_response(),
    };
    let student_response = match serde_json::to_value(response) {
        Ok(value) => value,
        Err(_) => return invalid_response(),
    };
    match state
        .webwork_submissions
        .accept_webwork_submission(
            token,
            AcceptNativePleSubmission {
                question_attempt: resolved.question_attempt,
                student_response,
            },
        )
        .await
    {
        Ok(()) => acknowledgement(nonce),
        Err(value) => submission_store_error(value),
    }
}

fn invalid_response() -> Response {
    error(
        StatusCode::UNPROCESSABLE_ENTITY,
        "Student Response is invalid",
    )
}

fn acknowledgement(nonce: QuestionPresentationNonce) -> Response {
    crate::auth::no_store(
        (
            StatusCode::CREATED,
            Json(NativePleSubmissionAcknowledgement {
                presentation_nonce: nonce.to_hex(),
                grading_state: "pending",
            }),
        )
            .into_response(),
    )
}

async fn reproduce_submission_presentation(
    objects: &objects::s3::S3ObjectStore,
    source: &ResolvedNativePleSubmission,
) -> Result<question_model::presentation::IssuedQuestionPresentation, StartError> {
    let object =
        uuid::Uuid::parse_str(&source.source_object_id).map_err(|_| StartError::Invalid)?;
    let revision = QuestionRevisionReference {
        question_id: source.question_id.clone(),
        revision_number: QuestionRevisionNumber::new(source.revision_number)
            .map_err(|_| StartError::Invalid)?,
    };
    let resolved = ResolvedPleQuestionJsonSource::resolve(
        objects,
        revision,
        SourceObjectReference {
            object: ObjectId::from_uuid(object),
        },
        SourceObjectChecksum::parse(source.source_object_checksum.clone())
            .map_err(|_| StartError::Invalid)?,
    )
    .await
    .map_err(|_| StartError::Unavailable)?;
    let issued = PleQuestionBackend::new()
        .issue_question_json(&resolved, QuestionSeed::new(source.question_seed))
        .map_err(|_| StartError::Invalid)?;
    let nonce = QuestionPresentationNonce::parse(&source.presentation_nonce)
        .map_err(|_| StartError::Invalid)?;
    let checksum = QuestionPresentationChecksum::parse_hex(&source.presentation_checksum)
        .map_err(|_| StartError::Invalid)?;
    reproduce_question_presentation(
        &issued.presentation,
        &question_asset_renditions_from_ready(&source.question_asset_renditions),
        QuestionPresentationBinding::new(nonce, checksum),
    )
    .map_err(|_| StartError::Invalid)
}

async fn reproduce_webwork_submission_presentation(
    state: &StateData,
    source: &ResolvedWebworkSubmission,
) -> Result<question_model::presentation::IssuedQuestionPresentation, StartError> {
    let object =
        uuid::Uuid::parse_str(&source.source_object_id).map_err(|_| StartError::Invalid)?;
    let revision = QuestionRevisionReference {
        question_id: source.question_id.clone(),
        revision_number: QuestionRevisionNumber::new(source.revision_number)
            .map_err(|_| StartError::Invalid)?,
    };
    let binding = WebworkQuestionSourceBinding::new(revision, source.webwork_pg_path.clone())
        .map_err(|_| StartError::Invalid)?;
    let resolved = ResolvedWebworkQuestionSource::resolve(
        &state.objects,
        binding,
        SourceObjectReference {
            object: ObjectId::from_uuid(object),
        },
        SourceObjectChecksum::parse(source.source_object_checksum.clone())
            .map_err(|_| StartError::Invalid)?,
    )
    .await
    .map_err(|_| StartError::Unavailable)?;
    let issued = state
        .webwork
        .reproduce(QuestionSeed::new(source.question_seed), &resolved)
        .await
        .map_err(|_| StartError::Unavailable)?;
    let nonce = QuestionPresentationNonce::parse(&source.presentation_nonce)
        .map_err(|_| StartError::Invalid)?;
    let checksum = QuestionPresentationChecksum::parse_hex(&source.presentation_checksum)
        .map_err(|_| StartError::Invalid)?;
    reproduce_question_presentation(
        &issued.presentation,
        &[],
        QuestionPresentationBinding::new(nonce, checksum),
    )
    .map_err(|_| StartError::Invalid)
}

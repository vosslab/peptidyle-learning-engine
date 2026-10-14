//! Student-authorized delivery of one immutable backend-owned WeBWorK document.

use std::str::FromStr;

use axum::{
    extract::{Path, Request, State},
    http::{HeaderMap, HeaderName, HeaderValue, StatusCode, header},
    response::{IntoResponse, Response},
};
use learning_data_access::{
    QuestionIssuanceReproductionInput, StudentAssessmentAttemptHistoryEvidence,
    StudentAssessmentAttemptPresentationSource,
};
use question_model::{AssessmentAttemptId, PublishedQuestionRevisionTuple};

use crate::assessment_delivery::{StateData, concealed, student};

const DOCUMENT_CSP: &str = "default-src 'self'; script-src 'self'; style-src 'self' 'unsafe-inline'; img-src 'self' data:; base-uri 'none'; object-src 'none'; frame-ancestors 'self'; form-action 'none'";
const PREVIEW_DOCUMENT_CSP: &str = "sandbox allow-scripts; default-src 'self'; script-src 'self'; style-src 'self' 'unsafe-inline'; img-src 'self' data:; base-uri 'none'; object-src 'none'; frame-ancestors 'self'; form-action 'none'";
const PARENT_TELEMETRY_SCRIPT_PATH: &str =
    "/api/webwork-assets/webwork2_files/js/apps/Problem/problem.";

/// Re-renders one Student's recorded WeBWorK Question and response without
/// grading or correct-answer disclosure.
pub(crate) async fn recorded_review(
    State(state): State<StateData>,
    Path((assessment_attempt, position)): Path<(String, String)>,
    request: Request,
) -> Response {
    let (parts, body) = request.into_parts();
    // This fixed GET route has no caller-selected renderer inputs.
    if parts.method != axum::http::Method::GET
        || parts.uri.query().is_some()
        || axum::body::to_bytes(body, 0).await.is_err()
    {
        return concealed();
    }
    let (assessment_attempt, position) = match (
        AssessmentAttemptId::from_str(&assessment_attempt),
        position.parse::<u32>(),
    ) {
        (Ok(assessment_attempt_id), Ok(position)) if position > 0 => {
            (assessment_attempt_id, position)
        }
        _ => return concealed(),
    };
    let token = match student(&state, &parts.headers).await {
        Ok(value) => value,
        Err(value) => return *value,
    };
    let history = match state
        .delivery
        .student_assessment_attempt_history(token, assessment_attempt)
        .await
    {
        Ok(value) => value,
        Err(_) => return concealed(),
    };
    let Some(revision) = permitted_recorded_review_revision_tuple(&history, position) else {
        return concealed();
    };
    let sources = match state
        .delivery
        .student_assessment_attempt_history_response_sources(token, assessment_attempt)
        .await
    {
        Ok(value) => value,
        Err(
            learning_data_access::StoreError::NotFound
            | learning_data_access::StoreError::Forbidden
            | learning_data_access::StoreError::OwnershipMismatch,
        ) => return concealed(),
        Err(_) => return recorded_review_unavailable(),
    };
    let Some(source) = sources
        .into_iter()
        .find(|source| source.position == position)
    else {
        return concealed();
    };
    let learning_data_access::StudentAssessmentAttemptHistoryResponseSource {
        response,
        backend_document,
        presentation_source,
        ..
    } = source;
    let Some(StudentAssessmentAttemptPresentationSource::Webwork { source, .. }) =
        presentation_source
    else {
        return concealed();
    };
    if source.question_id != revision.published_question_id
        || source.revision_number != revision.revision_number.get()
    {
        return recorded_review_unavailable();
    }
    let (document, response_available) = match response {
        None => (backend_document, true),
        Some(response) => {
            let replay = async {
                let seed = match &source.reproduction {
                    QuestionIssuanceReproductionInput::Seeded { question_seed } => *question_seed,
                    QuestionIssuanceReproductionInput::Static => return None,
                };
                let source =
                    crate::assessment_delivery::resolve_webwork_source(&state.objects, &source)
                        .await
                        .ok()?;
                let document = state
                    .webwork
                    .resume_document(seed, &source, &response)
                    .await
                    .ok()?;
                String::from_utf8(document).ok()
            }
            .await;
            match replay {
                Some(document) => (Some(document), true),
                None => (backend_document, false),
            }
        }
    };
    let Some(document) = document else {
        return recorded_review_unavailable();
    };
    // The final history read is the release instant. Renderer I/O never holds
    // a transaction or turns a revoked review permission into a document.
    let final_history = match state
        .delivery
        .student_assessment_attempt_history(token, assessment_attempt)
        .await
    {
        Ok(value) => value,
        Err(_) => return concealed(),
    };
    if permitted_recorded_review_revision_tuple(&final_history, position) != Some(revision) {
        return concealed();
    }
    let response = recorded_review_document_response(document, response_available);
    if response.status() == StatusCode::OK {
        response
    } else {
        recorded_review_unavailable()
    }
}

/// Renders one correct answer only while completed-history disclosure remains authorized.
pub(crate) async fn answer_review(
    State(state): State<StateData>,
    Path((assessment_attempt, position)): Path<(String, String)>,
    request: Request,
) -> Response {
    let (parts, body) = request.into_parts();
    // ASVS 2.2.1-2.2.2: the fixed route accepts no caller-selected render inputs.
    if parts.method != axum::http::Method::GET
        || parts.uri.query().is_some()
        || axum::body::to_bytes(body, 0).await.is_err()
    {
        return concealed();
    }
    let (assessment_attempt, position) = match (
        AssessmentAttemptId::from_str(&assessment_attempt),
        position.parse::<u32>(),
    ) {
        (Ok(assessment_attempt_id), Ok(position)) if position > 0 => {
            (assessment_attempt_id, position)
        }
        _ => return concealed(),
    };
    let token = match student(&state, &parts.headers).await {
        Ok(value) => value,
        Err(value) => return *value,
    };
    let history = match state
        .delivery
        .student_assessment_attempt_history(token, assessment_attempt)
        .await
    {
        Ok(value) => value,
        Err(_) => return concealed(),
    };
    let Some(revision) = permitted_answer_revision_tuple(&history, position) else {
        return concealed();
    };
    let sources = match state
        .delivery
        .student_assessment_attempt_history_response_sources(token, assessment_attempt)
        .await
    {
        Ok(value) => value,
        Err(
            learning_data_access::StoreError::NotFound
            | learning_data_access::StoreError::Forbidden
            | learning_data_access::StoreError::OwnershipMismatch,
        ) => return concealed(),
        Err(_) => return answer_review_unavailable(),
    };
    let Some(StudentAssessmentAttemptPresentationSource::Webwork { source, .. }) = sources
        .into_iter()
        .find(|source| source.position == position)
        .and_then(|source| source.presentation_source)
    else {
        return concealed();
    };
    if source.question_id != revision.published_question_id
        || source.revision_number != revision.revision_number.get()
    {
        return answer_review_unavailable();
    }
    let seed = match source.reproduction {
        QuestionIssuanceReproductionInput::Seeded { question_seed } => question_seed,
        QuestionIssuanceReproductionInput::Static => return answer_review_unavailable(),
    };
    let source =
        match crate::assessment_delivery::resolve_webwork_source(&state.objects, &source).await {
            Ok(value) => value,
            Err(_) => return answer_review_unavailable(),
        };
    let rendered = state.webwork.answer_review_document(seed, &source).await;
    // ASVS 8.2.2-8.2.3, 8.3.1: the final database read is the release instant.
    // No transaction or disclosure latch spans renderer I/O. Revocation discards the output.
    let final_history = match state
        .delivery
        .student_assessment_attempt_history(token, assessment_attempt)
        .await
    {
        Ok(value) => value,
        Err(_) => return concealed(),
    };
    if permitted_answer_revision_tuple(&final_history, position) != Some(revision) {
        return concealed();
    }
    let document = match rendered
        .ok()
        .and_then(|bytes| String::from_utf8(bytes).ok())
    {
        Some(value) => value,
        None => return answer_review_unavailable(),
    };
    let response = preview_backend_document_response(document);
    if response.status() == StatusCode::OK {
        response
    } else {
        answer_review_unavailable()
    }
}

fn permitted_answer_revision_tuple(
    evidence: &StudentAssessmentAttemptHistoryEvidence,
    position: u32,
) -> Option<&PublishedQuestionRevisionTuple> {
    if evidence.submitted_at.is_none()
        || !crate::assessment_delivery::history::history_decision(evidence).question_answer
    {
        return None;
    }
    evidence
        .history
        .questions
        .iter()
        .find(|question| question.position == position)
        .map(|question| &question.published_question_revision_tuple)
}

fn permitted_recorded_review_revision_tuple(
    evidence: &StudentAssessmentAttemptHistoryEvidence,
    position: u32,
) -> Option<&PublishedQuestionRevisionTuple> {
    evidence
        .submitted_at
        .is_some()
        .then(|| {
            evidence
                .history
                .questions
                .iter()
                .find(|question| question.position == position)
                .map(|question| &question.published_question_revision_tuple)
        })
        .flatten()
}

fn answer_review_unavailable() -> Response {
    // ASVS 16.5.1-16.5.3: no partial answer, backend diagnostics, or source on failure.
    let mut response = preview_backend_document_response(
        "<!doctype html><html lang=\"en\"><meta charset=\"utf-8\"><title>Correct answer unavailable</title><p>The correct answer could not be loaded. Use Retry correct answer to try again.</p></html>".into(),
    );
    *response.status_mut() = StatusCode::SERVICE_UNAVAILABLE;
    response
}

fn recorded_review_unavailable() -> Response {
    let mut response = preview_backend_document_response(
        "<!doctype html><html lang=\"en\"><meta charset=\"utf-8\"><title>Recorded question unavailable</title><p>Your recorded Question could not be loaded. Use Retry recorded Question to try again.</p></html>".into(),
    );
    *response.status_mut() = StatusCode::SERVICE_UNAVAILABLE;
    response
}

/// Serves the immutable retained renderer document, or an ephemeral
/// backend-authored resume render for its saved opaque response, for one
/// Student-owned issued WeBWorK position. The data-access boundary conceals
/// every unavailable, foreign, native, or incomplete position before document
/// bytes are returned.
pub(crate) async fn document(
    State(state): State<StateData>,
    headers: HeaderMap,
    Path((assessment_attempt, position)): Path<(String, u32)>,
) -> Response {
    let assessment_attempt = match AssessmentAttemptId::from_str(&assessment_attempt) {
        Ok(value) if position > 0 => value,
        _ => return concealed(),
    };
    let token = match student(&state, &headers).await {
        Ok(value) => value,
        Err(value) => return *value,
    };
    let document = match state
        .delivery
        .student_assessment_attempt_backend_document(token, assessment_attempt, position)
        .await
    {
        Ok(value) => match value.resume {
            None => value.backend_document,
            Some(resume) => {
                let source = match crate::assessment_delivery::resolve_webwork_source(
                    &state.objects,
                    &resume.source,
                )
                .await
                {
                    Ok(value) => value,
                    // ASVS 16.5.1: renderer/source failures stay concealed.
                    Err(_) => return concealed(),
                };
                let seed = match &resume.source.reproduction {
                    QuestionIssuanceReproductionInput::Seeded { question_seed } => *question_seed,
                    QuestionIssuanceReproductionInput::Static => return concealed(),
                };
                let document = match state
                    .webwork
                    .resume_document(seed, &source, &resume.saved_response)
                    .await
                {
                    Ok(value) => value,
                    Err(_) => return concealed(),
                };
                match String::from_utf8(document) {
                    Ok(value) => value,
                    Err(_) => return concealed(),
                }
            }
        },
        Err(_) => return concealed(),
    };

    backend_document_response(document)
}

/// Applies the isolated browser-document policy to renderer-owned HTML.
///
/// Callers must authorize and resolve the exact immutable Question source
/// before passing backend output to this response-only helper.
pub(crate) fn backend_document_response(document: String) -> Response {
    document_response(document, DOCUMENT_CSP)
}

/// Applies a response-level script-only sandbox to a no-write preview document.
pub(crate) fn preview_backend_document_response(document: String) -> Response {
    match without_preview_parent_telemetry(document) {
        Some(document) => document_response(document, PREVIEW_DOCUMENT_CSP),
        None => concealed(),
    }
}

/// Gives the isolated renderer document one narrow, server-owned review mode.
/// The shared bridge recognizes this marker and prevents form editing or
/// submission; the opaque response sandbox remains the enforcement boundary.
fn recorded_review_document_response(document: String, response_available: bool) -> Response {
    let Some(document) = without_preview_parent_telemetry(document).and_then(|document| {
        document.find("<head>").map(|index| {
            let marker = if response_available {
                "<meta name=\"ple-review\" content=\"recorded\">"
            } else {
                "<meta name=\"ple-review\" content=\"prompt\">"
            };
            format!(
                "{}{}{}",
                &document[..index + "<head>".len()],
                marker,
                &document[index + "<head>".len()..]
            )
        })
    }) else {
        return concealed();
    };
    document_response(document, PREVIEW_DOCUMENT_CSP)
}

fn without_preview_parent_telemetry(mut document: String) -> Option<String> {
    // The renderer always includes this parent-frame telemetry/result-popover
    // loader. Its frameElement access cannot run in an opaque preview, which has
    // neither Student interaction reporting nor grading results. Omit only that
    // exact loader; do not interpret Question controls or change Student bytes.
    const OPEN: &str = "<script defer src=\"";
    const CLOSE: &str = "\"></script>";
    let Some(path_start) = document.find(PARENT_TELEMETRY_SCRIPT_PATH) else {
        return Some(document);
    };
    if document[path_start + PARENT_TELEMETRY_SCRIPT_PATH.len()..]
        .contains(PARENT_TELEMETRY_SCRIPT_PATH)
    {
        return None;
    }
    let start = document[..path_start].rfind(OPEN)?;
    let source_end = path_start + document[path_start..].find(CLOSE)?;
    let source = url::Url::parse(&document[start + OPEN.len()..source_end]).ok()?;
    let filename = source.path().strip_prefix(PARENT_TELEMETRY_SCRIPT_PATH)?;
    let recognized_filename = filename == "js"
        || filename.strip_suffix(".min.js").is_some_and(|hash| {
            !hash.is_empty() && hash.bytes().all(|byte| byte.is_ascii_hexdigit())
        });
    if !recognized_filename
        || !matches!(source.scheme(), "http" | "https")
        || !source.username().is_empty()
        || source.password().is_some()
        || source.query().is_some()
        || source.fragment().is_some()
    {
        return None;
    }
    document.replace_range(start..source_end + CLOSE.len(), "");
    Some(document)
}

fn document_response(document: String, content_security_policy: &'static str) -> Response {
    // ASVS 3.2.1, 3.4.3-3.4.6, and 4.1.1: preserve the existing isolated
    // document CSP and declare its exact media, cache, origin, and referrer policy.
    let mut response = (StatusCode::OK, document).into_response();
    let response_headers = response.headers_mut();
    response_headers.insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static("text/html; charset=utf-8"),
    );
    response_headers.insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
    response_headers.insert(
        header::CONTENT_SECURITY_POLICY,
        HeaderValue::from_static(content_security_policy),
    );
    response_headers.insert(
        HeaderName::from_static("cross-origin-resource-policy"),
        HeaderValue::from_static("same-origin"),
    );
    response_headers.insert(
        header::REFERRER_POLICY,
        HeaderValue::from_static("no-referrer"),
    );
    response_headers.insert(
        header::X_CONTENT_TYPE_OPTIONS,
        HeaderValue::from_static("nosniff"),
    );
    response
}

#[cfg(test)]
mod tests {
    use axum::{body::to_bytes, http::header};

    use super::{PREVIEW_DOCUMENT_CSP, recorded_review_document_response};

    #[tokio::test]
    async fn recorded_review_marks_the_renderer_document_for_read_only_bridge_behavior() {
        let response = recorded_review_document_response(
            "<!doctype html><html><head></head><body><form><input value=\"saved\"></form></body></html>"
                .to_string(),
            true,
        );
        assert!(response.status().is_success());
        assert_eq!(
            response.headers().get(header::CONTENT_SECURITY_POLICY),
            Some(&header::HeaderValue::from_static(PREVIEW_DOCUMENT_CSP))
        );
        let body = to_bytes(response.into_body(), 4096)
            .await
            .expect("review document body");
        let body = std::str::from_utf8(&body).expect("UTF-8 review document");
        assert!(body.contains("<meta name=\"ple-review\" content=\"recorded\">"));
    }
}

//! Student-authorized delivery of one immutable backend-owned WeBWorK document.

use std::str::FromStr;

use axum::{
    extract::{Path, Request, State},
    http::{HeaderMap, HeaderName, HeaderValue, StatusCode, header},
    response::{IntoResponse, Response},
};
use learning_data_access::{
    LiveAssessmentDeliveryStore, QuestionIssuanceReproductionInput,
    StudentAssessmentAttemptHistoryEvidence, StudentAssessmentAttemptPresentationSource,
};
use question_model::{AssessmentAttemptId, QuestionRevisionTuple};

use crate::assessment_delivery::{StateData, concealed, student};

const DOCUMENT_CSP: &str = "default-src 'self'; script-src 'self'; style-src 'self' 'unsafe-inline'; img-src 'self' data:; base-uri 'none'; object-src 'none'; frame-ancestors 'self'; form-action 'none'";
const PREVIEW_DOCUMENT_CSP: &str = "sandbox allow-scripts; default-src 'self'; script-src 'self'; style-src 'self' 'unsafe-inline'; img-src 'self' data:; base-uri 'none'; object-src 'none'; frame-ancestors 'self'; form-action 'none'";
const PARENT_TELEMETRY_SCRIPT_PATH: &str =
    "/api/webwork-assets/webwork2_files/js/apps/Problem/problem.";

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
    let Some(revision) = permitted_answer_revision(&history, position) else {
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
    if source.question_id != revision.question_id
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
    if permitted_answer_revision(&final_history, position) != Some(revision) {
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

fn permitted_answer_revision(
    evidence: &StudentAssessmentAttemptHistoryEvidence,
    position: u32,
) -> Option<&QuestionRevisionTuple> {
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
        .map(|question| &question.question_revision_tuple)
}

fn answer_review_unavailable() -> Response {
    // ASVS 16.5.1-16.5.3: no partial answer, backend diagnostics, or source on failure.
    let mut response = preview_backend_document_response(
        "<!doctype html><html lang=\"en\"><meta charset=\"utf-8\"><title>Correct answer unavailable</title><p>The correct answer could not be loaded. Use Retry correct answer to try again.</p></html>".into(),
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
    use super::*;

    fn review_evidence() -> StudentAssessmentAttemptHistoryEvidence {
        use learning_data_access::{
            LiveAssessmentPreviousAttemptState, StudentAssessmentAttemptHistory,
            StudentAssessmentAttemptHistoryAssessment, StudentAssessmentAttemptHistoryCourse,
            StudentAssessmentAttemptHistoryQuestion,
        };
        use question_model::{
            AssessmentType, StudentFeedback, StudentFeedbackReleaseRule, Timestamp,
        };
        StudentAssessmentAttemptHistoryEvidence {
            history: StudentAssessmentAttemptHistory {
                assessment_attempt: AssessmentAttemptId::from_uuid(uuid::Uuid::from_u128(12)),
                attempt_number: 1,
                course: StudentAssessmentAttemptHistoryCourse {
                    id: "CIABCDEFGS".parse().unwrap(),
                    short_name: "Course".into(),
                    long_name: "Course".into(),
                    theme: question_model::CourseTheme::Forest,
                },
                assessment: StudentAssessmentAttemptHistoryAssessment {
                    id: "AABCDEFG8".parse().unwrap(),
                    title: "Practice".into(),
                },
                state: LiveAssessmentPreviousAttemptState::Submitted,
                score: None,
                questions: vec![StudentAssessmentAttemptHistoryQuestion {
                    position: 1,
                    question_revision_tuple: QuestionRevisionTuple {
                        question_id: question_model::QuestionId::from_random_identifier("ABCDEF1")
                            .unwrap(),
                        revision_number: question_model::QuestionRevisionNumber::new(3).unwrap(),
                    },
                    response_state: LiveAssessmentPreviousAttemptState::Closed,
                    response: None,
                    backend_answer_review: None,
                    feedback: StudentFeedback::empty(),
                }],
            },
            assessment_type: AssessmentType::PracticeQuestionAssignment,
            feedback_rule: StudentFeedbackReleaseRule {
                question_answer: question_model::StudentFeedbackReleaseTiming::AfterSubmit,
                ..StudentFeedbackReleaseRule::default()
            },
            due_at: None,
            closes_at: Some(Timestamp::from_unix_millis(1)),
            submitted_at: Some(Timestamp::from_unix_millis(1)),
            evaluated_at: Timestamp::from_unix_millis(2),
            all_students_completed: false,
            grading_is_current: false,
            grading_results: vec![None],
        }
    }

    #[test]
    fn answer_review_requires_committed_submission_and_current_answer_permission() {
        let mut evidence = review_evidence();
        assert!(permitted_answer_revision(&evidence, 1).is_some());
        assert!(permitted_answer_revision(&evidence, 0).is_none());
        assert!(permitted_answer_revision(&evidence, 2).is_none());
        evidence.submitted_at = None;
        assert!(permitted_answer_revision(&evidence, 1).is_none());
        evidence.submitted_at = Some(question_model::Timestamp::from_unix_millis(1));
        for assessment_type in [
            question_model::AssessmentType::Quiz,
            question_model::AssessmentType::Exam,
        ] {
            evidence.assessment_type = assessment_type;
            evidence.all_students_completed = false;
            assert!(permitted_answer_revision(&evidence, 1).is_none());
            evidence.all_students_completed = true;
            assert!(permitted_answer_revision(&evidence, 1).is_some());
            // A newly current Student recloses the same decision used after rendering.
            evidence.all_students_completed = false;
            assert!(permitted_answer_revision(&evidence, 1).is_none());
        }
        evidence.all_students_completed = true;
        evidence.feedback_rule.question_answer =
            question_model::StudentFeedbackReleaseTiming::Never;
        assert!(permitted_answer_revision(&evidence, 1).is_none());
    }

    #[test]
    fn review_failure_is_bounded_and_has_the_same_opaque_document_headers() {
        let response = answer_review_unavailable();
        assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
        for (name, value) in [
            (header::CONTENT_TYPE, "text/html; charset=utf-8"),
            (header::CACHE_CONTROL, "no-store"),
            (header::CONTENT_SECURITY_POLICY, PREVIEW_DOCUMENT_CSP),
            (header::X_CONTENT_TYPE_OPTIONS, "nosniff"),
            (header::REFERRER_POLICY, "no-referrer"),
        ] {
            assert_eq!(response.headers().get(name).unwrap(), value);
        }
    }

    #[tokio::test]
    async fn preview_omits_parent_telemetry_and_keeps_student_document_unchanged() {
        let script = format!(
            "<script defer src=\"https://ple.example{PARENT_TELEMETRY_SCRIPT_PATH}da1d2ec5.min.js\"></script>"
        );
        let content =
            "<form><input name=\"backend-owned\"></form><script src=\"/ple_bridge.js\"></script>";
        let document = format!("<!doctype html>{script}{content}");
        let response = preview_backend_document_response(document.clone());

        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(
            response.headers().get(header::CONTENT_SECURITY_POLICY),
            Some(&HeaderValue::from_static(PREVIEW_DOCUMENT_CSP))
        );
        assert_eq!(
            response.headers().get(header::CACHE_CONTROL),
            Some(&HeaderValue::from_static("no-store"))
        );
        assert!(PREVIEW_DOCUMENT_CSP.starts_with("sandbox allow-scripts;"));
        assert!(!PREVIEW_DOCUMENT_CSP.contains("allow-forms"));
        assert!(!PREVIEW_DOCUMENT_CSP.contains("allow-same-origin"));
        let preview = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        assert_eq!(preview, format!("<!doctype html>{content}"));

        let student = backend_document_response(document.clone());
        let student = axum::body::to_bytes(student.into_body(), usize::MAX)
            .await
            .unwrap();
        assert_eq!(student, document);
    }

    #[test]
    fn preview_refuses_an_unrecognized_parent_telemetry_loader() {
        let document = format!(
            "<script async src=\"https://ple.example{PARENT_TELEMETRY_SCRIPT_PATH}js\"></script>"
        );
        assert_eq!(
            preview_backend_document_response(document).status(),
            StatusCode::NOT_FOUND
        );
    }
}

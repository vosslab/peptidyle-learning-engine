//! Student-authorized delivery of one immutable backend-owned WeBWorK document.

use std::str::FromStr;

use axum::{
    extract::{Path, State},
    http::{HeaderMap, HeaderName, HeaderValue, StatusCode, header},
    response::{IntoResponse, Response},
};
use learning_data_access::{LiveAssessmentDeliveryStore, QuestionIssuanceReproductionInput};
use question_model::AssessmentAttemptReference;

use crate::assessment_delivery::{StateData, concealed, student};

const DOCUMENT_CSP: &str = "default-src 'self'; script-src 'self'; style-src 'self' 'unsafe-inline'; img-src 'self' data:; base-uri 'none'; object-src 'none'; frame-ancestors 'self'; form-action 'none'";
const PREVIEW_DOCUMENT_CSP: &str = "sandbox allow-scripts; default-src 'self'; script-src 'self'; style-src 'self' 'unsafe-inline'; img-src 'self' data:; base-uri 'none'; object-src 'none'; frame-ancestors 'self'; form-action 'none'";
const PARENT_TELEMETRY_SCRIPT_PATH: &str =
    "/api/webwork-assets/webwork2_files/js/apps/Problem/problem.";

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
    let assessment_attempt = match AssessmentAttemptReference::from_str(&assessment_attempt) {
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

//! Isolated, Student-authorized delivery of retained author content.
//!
//! This is deliberately an HTML-document boundary, not a browser API.  The
//! generic Question presentation never carries source, and this route neither
//! saves, submits, grades, nor exposes a reusable source/object endpoint.

use std::str::FromStr;

use axum::{
    extract::{Path, State},
    http::{HeaderMap, HeaderValue, StatusCode, header},
    response::{IntoResponse, Response},
};
use base64::{Engine as _, engine::general_purpose::STANDARD};
use learning_data_access::LiveAssessmentDeliveryStore;
use question_model::{AssessmentAttemptReference, AuthorContentLibraryId};

use crate::{
    assessment_delivery::{StateData, concealed, reproduce_selected_issued_presentation, student},
    author_content_dependency_assets::reviewed_rdkit_runtime,
};

const AUTHOR_CONTENT_DOCUMENT_CSP_PREFIX: &str =
    "sandbox allow-scripts; default-src 'none'; base-uri 'none'; object-src 'none';";
const AUTHOR_CONTENT_DOCUMENT_CSP_SUFFIX: &str = "img-src 'none'; media-src 'none'; font-src 'none'; frame-src 'none'; worker-src 'none'; form-action 'none'; frame-ancestors 'self';";

/// Serves exactly one retained author-content document for an owning Student's
/// issued position. Missing, foreign, native, malformed, stale, or runtime-
/// unavailable positions stay concealed.
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
    let evidence = match state
        .delivery
        .student_assessment_attempt_presentation_evidence(token, assessment_attempt, position)
        .await
    {
        Ok(value) => value,
        Err(_) => return concealed(),
    };
    let issued = match reproduce_selected_issued_presentation(evidence) {
        Ok(value) => value,
        Err(_) => return concealed(),
    };
    let author_content = match issued.author_content {
        Some(value) => value,
        _ => return concealed(),
    };
    let runtime = match author_content.library_ids() {
        [] => None,
        [AuthorContentLibraryId::Rdkit] => match reviewed_rdkit_runtime() {
            Some(value) => Some(value),
            None => return concealed(),
        },
        _ => return concealed(),
    };
    let nonce = match document_nonce() {
        Ok(value) => value,
        Err(()) => return StatusCode::SERVICE_UNAVAILABLE.into_response(),
    };
    let html = document_html(author_content.source(), runtime.as_ref(), &nonce);
    let csp = document_csp(state.browser_origin.as_ref(), runtime.as_ref(), &nonce);
    let mut response = (StatusCode::OK, html).into_response();
    let response_headers = response.headers_mut();
    response_headers.insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static("text/html; charset=utf-8"),
    );
    response_headers.insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
    response_headers.insert(
        header::REFERRER_POLICY,
        HeaderValue::from_static("no-referrer"),
    );
    response_headers.insert(
        header::CONTENT_SECURITY_POLICY,
        HeaderValue::from_str(&csp).expect("server-owned CSP is valid"),
    );
    response_headers.insert(
        header::X_CONTENT_TYPE_OPTIONS,
        HeaderValue::from_static("nosniff"),
    );
    response
}

fn document_nonce() -> Result<String, ()> {
    let mut bytes = [0_u8; 16];
    getrandom::fill(&mut bytes).map_err(|_| ())?;
    Ok(STANDARD.encode(bytes))
}

fn document_csp(
    browser_origin: &str,
    runtime: Option<&crate::author_content_dependency_assets::ReviewedRdkitRuntime>,
    nonce: &str,
) -> String {
    match runtime {
        Some(runtime) => format!(
            "{AUTHOR_CONTENT_DOCUMENT_CSP_PREFIX} connect-src {browser_origin}{}; {AUTHOR_CONTENT_DOCUMENT_CSP_SUFFIX} script-src '{}' 'nonce-{nonce}' 'unsafe-eval'",
            runtime.wasm_path(),
            runtime.javascript_sri(),
        ),
        None => format!(
            "{AUTHOR_CONTENT_DOCUMENT_CSP_PREFIX} connect-src 'none'; {AUTHOR_CONTENT_DOCUMENT_CSP_SUFFIX} script-src 'nonce-{nonce}'"
        ),
    }
}

fn document_html(
    source: &str,
    runtime: Option<&crate::author_content_dependency_assets::ReviewedRdkitRuntime>,
    nonce: &str,
) -> String {
    // Source must not enter HTML parsing: base64 carries its UTF-8 bytes to a
    // nonce-authorized bootstrap, which creates a nonce-authorized script DOM
    // node only after the reviewed runtime has loaded.
    let encoded_source = STANDARD.encode(source.as_bytes());
    let runtime_head = match runtime {
        Some(runtime) => format!(
            "<script nonce=\"{nonce}\">window.Module = {{ locateFile: (name) => name === \"RDKit_minimal.wasm\" ? {:?} : \"\" }};</script><script src=\"{}\" integrity=\"{}\" crossorigin=\"anonymous\"></script>",
            runtime.wasm_path(),
            runtime.javascript_path(),
            runtime.javascript_sri(),
        ),
        None => String::new(),
    };
    format!(
        "<!doctype html><html><head><meta charset=\"utf-8\">{runtime_head}</head><body><div id=\"author-content-root\"></div><script nonce=\"{nonce}\">(() => {{ const bytes = Uint8Array.from(atob({encoded_source:?}), (value) => value.charCodeAt(0)); const author = document.createElement(\"script\"); author.setAttribute(\"nonce\", {nonce:?}); author.textContent = new TextDecoder().decode(bytes); document.body.append(author); }})();</script></body></html>"
    )
}

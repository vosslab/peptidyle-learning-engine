//! Isolated, Student-authorized delivery of retained author content.
//!
//! This is deliberately an HTML-document boundary, not a browser API.  The
//! generic Question presentation never carries source, and this route neither
//! saves, submits, grades, nor exposes a reusable source/object endpoint.

use std::str::FromStr;

use axum::{
    extract::{Path, State},
    http::{HeaderMap, HeaderName, HeaderValue, StatusCode, header},
    response::{IntoResponse, Response},
};
use base64::{Engine as _, engine::general_purpose::STANDARD};
use question_model::{AssessmentAttemptId, AuthorContentLibraryId};

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
    accepted: Option<axum::extract::Extension<crate::auth::AcceptedBrowserOrigin>>,
) -> Response {
    let assessment_attempt = match AssessmentAttemptId::from_str(&assessment_attempt) {
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
    let browser_origin = crate::auth::document_parent_origin(&state.browser_origin, accepted);
    author_content_document_response(&author_content, browser_origin.as_ref())
}

/// Builds one isolated author-content document after the caller has authorized
/// and reproduced the exact immutable native Question source.
pub(crate) fn author_content_document_response(
    author_content: &question_model::AuthorContentPresentation,
    browser_origin: &str,
) -> Response {
    // ASVS 1.1.2, 1.2.1, 3.2.1, 3.4.3-3.4.6, and 4.1.1: source enters only
    // the existing base64/nonce document builder under its isolated response policy.
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
        Err(()) => {
            return crate::auth::no_store(StatusCode::SERVICE_UNAVAILABLE.into_response());
        }
    };
    let html = document_html(
        author_content.source(),
        runtime.as_ref(),
        &nonce,
        browser_origin,
    );
    let csp = document_csp(browser_origin, runtime.as_ref(), &nonce);
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
    response_headers.insert(
        HeaderName::from_static("cross-origin-resource-policy"),
        HeaderValue::from_static("same-origin"),
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
    browser_origin: &str,
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
    let appearance_bootstrap = format!(
        r#"<script nonce="{nonce}">(() => {{
const parentOrigin = {browser_origin:?};
const colorKeys = ["background", "foreground", "surface", "secondary", "accent", "highlight", "muted", "border", "onAccent", "link"];
const properties = [["--ple-document-background", "background"], ["--ple-document-foreground", "foreground"], ["--ple-document-surface", "surface"], ["--ple-document-secondary", "secondary"], ["--ple-document-accent", "accent"], ["--ple-document-highlight", "highlight"], ["--ple-document-muted", "muted"], ["--ple-document-border", "border"], ["--ple-document-on-accent", "onAccent"], ["--ple-document-link", "link"], ["--bs-body-bg", "background"], ["--bs-body-color", "foreground"], ["--bs-secondary-bg", "secondary"], ["--bs-tertiary-bg", "secondary"], ["--bs-border-color", "border"], ["--bs-primary", "accent"], ["--bs-link-color", "link"], ["--bs-link-hover-color", "link"]];
const isRecord = (value) => value !== null && typeof value === "object" && !Array.isArray(value);
const hasExactKeys = (value, keys) => Object.keys(value).length === keys.length && keys.every((key) => Object.keys(value).includes(key));
const isAppearance = (value) => isRecord(value) && hasExactKeys(value, ["kind", "version", "mode", "colors"]) && value.kind === "ple.embed.appearance" && value.version === 1 && (value.mode === "light" || value.mode === "dark") && isRecord(value.colors) && hasExactKeys(value.colors, colorKeys) && colorKeys.every((key) => typeof value.colors[key] === "string" && /^#[0-9a-f]{{6}}$/i.test(value.colors[key]));
const appearancePrefix = "ple.embed.appearance:";
const decodeAppearance = (value) => {{
  if (typeof value !== "string" || !value.startsWith(appearancePrefix)) return undefined;
  try {{
    const appearance = JSON.parse(value.slice(appearancePrefix.length));
    return isAppearance(appearance) ? appearance : undefined;
  }} catch {{ return undefined; }}
}};
window.addEventListener("message", (event) => {{
  // ASVS 1.2.3, 2.2.1, 3.5.5: fixed output properties accept only the
  // canonical parent's closed, validated cosmetic record.
  const appearance = decodeAppearance(event.data);
  if (event.origin !== parentOrigin || event.source !== window.parent || appearance === undefined) return;
  const root = document.documentElement;
  root.style.colorScheme = appearance.mode;
  for (const [property, color] of properties) root.style.setProperty(property, appearance.colors[color]);
  document.body.style.setProperty("background-color", appearance.colors.background);
  document.body.style.setProperty("color", appearance.colors.foreground);
}});
}})();</script>"#
    );
    format!(
        "<!doctype html><html><head><meta charset=\"utf-8\">{runtime_head}</head><body><div id=\"author-content-root\"></div>{appearance_bootstrap}<script nonce=\"{nonce}\">(() => {{ const bytes = Uint8Array.from(atob({encoded_source:?}), (value) => value.charCodeAt(0)); const author = document.createElement(\"script\"); author.setAttribute(\"nonce\", {nonce:?}); author.textContent = new TextDecoder().decode(bytes); document.body.append(author); }})();</script></body></html>"
    )
}

#[cfg(test)]
mod tests {
    use axum::body::{Body, to_bytes};
    use axum::http::{Request, StatusCode, header};
    use base64::{Engine as _, engine::general_purpose::STANDARD};
    use question_model::{AuthorContentLibraryId, AuthorContentPresentation};
    use tower::ServiceExt;

    use super::{author_content_document_response, document_csp, document_html};
    use crate::author_content_dependency_assets::{
        author_content_dependency_asset_router, reviewed_rdkit_runtime,
    };

    #[test]
    fn author_document_installs_the_closed_appearance_receiver_before_author_source() {
        let html = document_html(
            "window.authorRan = true;",
            None,
            "test-nonce",
            "https://ple.test",
        );
        let receiver = html.find("ple.embed.appearance").unwrap();
        let author = html.find("author.textContent").unwrap();

        assert!(receiver < author);
        assert!(html.contains("const parentOrigin = \"https://ple.test\";"));
        assert!(html.contains("event.source !== window.parent"));
        assert!(html.contains("/^#[0-9a-f]{6}$/i"));
        assert!(html.contains("\"onAccent\", \"link\""));
        assert!(html.contains("[\"--ple-document-link\", \"link\"]"));
        assert!(html.contains("[\"--bs-link-color\", \"link\"]"));
        assert!(html.contains("root.style.setProperty(property, appearance.colors[color])"));
        assert!(!html.contains("style.cssText"));
    }

    #[test]
    fn author_document_keeps_its_existing_nonce_only_content_policy() {
        let csp = document_csp("https://ple.test", None, "test-nonce");
        assert!(csp.contains("script-src 'nonce-test-nonce'"));
        assert!(csp.contains("connect-src 'none'"));
        assert!(!csp.contains("style-src"));
    }

    #[tokio::test]
    async fn author_supplied_javascript_may_provide_client_side_rendering_or_interaction_without_access_to_a_random_seed()
     {
        let seed = "question-seed-9f3c";
        let source = "document.getElementById('author-content-root').textContent = 'rendered';";
        let author_content =
            AuthorContentPresentation::new(source.to_string(), Vec::new()).expect("author content");
        let response = author_content_document_response(&author_content, "https://ple.test");
        let (parts, body) = response.into_parts();
        let html = String::from_utf8(
            to_bytes(body, usize::MAX)
                .await
                .expect("document body")
                .to_vec(),
        )
        .expect("document text");
        let headers = format!("{:?}", parts.headers);

        assert!(html.contains(&STANDARD.encode(source.as_bytes())));
        assert!(html.contains("author.textContent"));
        assert!(html.contains("id=\"author-content-root\""));
        assert!(!html.contains(seed));
        assert!(!headers.contains(seed));
    }

    #[tokio::test]
    async fn author_supplied_javascript_runs_in_an_isolated_browser_environment() {
        let source = "document.getElementById('author-content-root').textContent = 'rendered';";
        let author_content =
            AuthorContentPresentation::new(source.to_string(), Vec::new()).expect("author content");
        let response = author_content_document_response(&author_content, "https://ple.test");
        let (parts, body) = response.into_parts();
        let csp = parts
            .headers
            .get(header::CONTENT_SECURITY_POLICY)
            .expect("content security policy")
            .to_str()
            .expect("content security policy text");
        let html = String::from_utf8(
            to_bytes(body, usize::MAX)
                .await
                .expect("document body")
                .to_vec(),
        )
        .expect("document text");

        assert!(csp.starts_with("sandbox allow-scripts"));
        assert!(!csp.contains("allow-same-origin"));
        assert!(csp.contains("default-src 'none'"));
        assert!(csp.contains("frame-ancestors 'self'"));
        assert_eq!(
            parts
                .headers
                .get(header::REFERRER_POLICY)
                .expect("referrer policy")
                .to_str()
                .expect("referrer policy text"),
            "no-referrer"
        );
        assert!(html.contains(&STANDARD.encode(source.as_bytes())));
        assert!(html.contains("author.textContent"));
    }

    #[tokio::test]
    async fn author_supplied_javascript_is_treated_as_untrusted_content() {
        // ASVS 1.2.1: untrusted source is carried as data, not parsed as HTML.
        let source = "</script><img src=x onerror=alert(1)>";
        let author_content =
            AuthorContentPresentation::new(source.to_string(), Vec::new()).expect("author content");
        let response = author_content_document_response(&author_content, "https://ple.test");
        let html = String::from_utf8(
            to_bytes(response.into_body(), usize::MAX)
                .await
                .expect("document body")
                .to_vec(),
        )
        .expect("document text");

        assert!(html.contains(&STANDARD.encode(source.as_bytes())));
        assert!(html.contains("author.textContent"));
        assert!(!html.contains(source));
        assert!(!html.contains("onerror=alert(1)"));
    }

    #[tokio::test]
    async fn author_supplied_javascript_is_isolated_from_ple_application_state_credentials_and_privileged_browser_context()
     {
        // ASVS 3.4.5 and 3.5.1: the document origin stays opaque and receives no
        // credentials or application state.
        let source = "document.getElementById('author-content-root').textContent = 'rendered';";
        let credential = "ple-session-credential";
        let author_content =
            AuthorContentPresentation::new(source.to_string(), Vec::new()).expect("author content");
        let response = author_content_document_response(&author_content, "https://ple.test");
        let (parts, body) = response.into_parts();
        let csp = parts
            .headers
            .get(header::CONTENT_SECURITY_POLICY)
            .expect("content security policy")
            .to_str()
            .expect("content security policy text");
        let html = String::from_utf8(
            to_bytes(body, usize::MAX)
                .await
                .expect("document body")
                .to_vec(),
        )
        .expect("document text");

        assert_eq!(
            csp.split(';').next().expect("sandbox directive").trim(),
            "sandbox allow-scripts"
        );
        assert!(csp.contains("connect-src 'none'"));
        assert!(parts.headers.get(header::SET_COOKIE).is_none());
        assert!(parts.headers.get(header::COOKIE).is_none());
        assert!(parts.headers.get(header::AUTHORIZATION).is_none());
        assert!(html.contains(&STANDARD.encode(source.as_bytes())));
        for absent in [
            "localStorage",
            "sessionStorage",
            "document.cookie",
            "indexedDB",
            "Authorization",
            credential,
        ] {
            assert!(!html.contains(absent), "{absent}");
        }
    }

    #[tokio::test]
    async fn author_supplied_javascript_is_limited_to_client_side_rendering_and_interaction() {
        // ASVS 3.4.3 and 15.2.5: the author script may render in its document
        // and receives no network, form, frame, or worker capability.
        let source = "document.getElementById('author-content-root').textContent = 'rendered';";
        let author_content =
            AuthorContentPresentation::new(source.to_string(), Vec::new()).expect("author content");
        let response = author_content_document_response(&author_content, "https://ple.test");
        let (parts, body) = response.into_parts();
        let csp = parts
            .headers
            .get(header::CONTENT_SECURITY_POLICY)
            .expect("content security policy")
            .to_str()
            .expect("content security policy text");
        let html = String::from_utf8(
            to_bytes(body, usize::MAX)
                .await
                .expect("document body")
                .to_vec(),
        )
        .expect("document text");

        assert_eq!(
            csp.split(';').next().expect("sandbox directive").trim(),
            "sandbox allow-scripts"
        );
        for required in [
            "default-src 'none'",
            "base-uri 'none'",
            "object-src 'none'",
            "connect-src 'none'",
            "img-src 'none'",
            "media-src 'none'",
            "font-src 'none'",
            "frame-src 'none'",
            "worker-src 'none'",
            "form-action 'none'",
        ] {
            assert!(csp.contains(required), "{required}");
        }
        for absent in [
            "allow-same-origin",
            "allow-forms",
            "allow-popups",
            "allow-top-navigation",
            "allow-downloads",
            "allow-modals",
        ] {
            assert!(!csp.contains(absent), "{absent}");
        }
        assert!(html.contains(&STANDARD.encode(source.as_bytes())));
        assert!(html.contains("author.textContent"));
        for absent in ["fetch(", "XMLHttpRequest", "WebSocket", "/api/"] {
            assert!(!html.contains(absent), "{absent}");
        }
    }

    #[tokio::test]
    async fn author_supplied_javascript_operates_independently_of_ple_application_apis_and_privileged_state()
     {
        // ASVS 15.2.5: the author script is installed from its own source.
        // The document builder takes no API client, session, or privileged state.
        let source = "document.getElementById('author-content-root').textContent = 'rendered';";
        let credential = "ple-session-credential";
        let author_content =
            AuthorContentPresentation::new(source.to_string(), Vec::new()).expect("author content");
        let response = author_content_document_response(&author_content, "https://ple.test");
        let (parts, body) = response.into_parts();
        let csp = parts
            .headers
            .get(header::CONTENT_SECURITY_POLICY)
            .expect("content security policy")
            .to_str()
            .expect("content security policy text");
        let html = String::from_utf8(
            to_bytes(body, usize::MAX)
                .await
                .expect("document body")
                .to_vec(),
        )
        .expect("document text");

        assert!(csp.contains("connect-src 'none'"));
        assert!(parts.headers.get(header::AUTHORIZATION).is_none());
        assert!(parts.headers.get(header::COOKIE).is_none());
        assert!(parts.headers.get(header::SET_COOKIE).is_none());
        assert!(html.contains(&STANDARD.encode(source.as_bytes())));
        assert!(html.contains("author.textContent"));
        for absent in [
            "/api/",
            credential,
            "Authorization",
            "localStorage",
            "document.cookie",
        ] {
            assert!(!html.contains(absent), "{absent}");
        }
    }

    #[tokio::test]
    async fn supported_external_dependencies_should_eventually_become_ple_owned_and_served_locally()
    {
        // ASVS 3.6.1: the reviewed runtime is PLE-owned local bytes. The author
        // document names those routes and no external CDN host.
        let source = "document.getElementById('author-content-root').textContent = 'rendered';";
        let author_content =
            AuthorContentPresentation::new(source.to_string(), vec![AuthorContentLibraryId::Rdkit])
                .expect("author content");
        let response = author_content_document_response(&author_content, "https://ple.test");
        let (parts, body) = response.into_parts();
        let csp = parts
            .headers
            .get(header::CONTENT_SECURITY_POLICY)
            .expect("content security policy")
            .to_str()
            .expect("content security policy text");
        let html = String::from_utf8(
            to_bytes(body, usize::MAX)
                .await
                .expect("document body")
                .to_vec(),
        )
        .expect("document text");
        let runtime = reviewed_rdkit_runtime().expect("reviewed current runtime");
        let javascript = runtime.javascript_path();
        let wasm = runtime.wasm_path();

        assert_eq!(
            javascript,
            "/api/author-content-dependencies/rdkit/RDKit_minimal.js"
        );
        assert_eq!(
            wasm,
            "/api/author-content-dependencies/rdkit/RDKit_minimal.wasm"
        );
        assert!(html.contains(&format!("src=\"{javascript}\"")));
        assert!(html.contains(wasm));
        assert!(csp.contains(&format!("'{sri}'", sri = runtime.javascript_sri())));
        let connect = csp
            .split(';')
            .map(str::trim)
            .find(|directive| directive.starts_with("connect-src "))
            .expect("connect-src");
        assert_eq!(connect, format!("connect-src https://ple.test{wasm}"));
        for absent in ["cdn.", "unpkg.com", "jsdelivr.net", "cdnjs.cloudflare.com"] {
            assert!(!html.contains(absent), "{absent}");
            assert!(!csp.contains(absent), "{absent}");
        }

        let app = author_content_dependency_asset_router();
        for path in [javascript, wasm] {
            let served = app
                .clone()
                .oneshot(Request::get(path).body(Body::empty()).expect("request"))
                .await
                .expect("dependency response");
            assert_eq!(served.status(), StatusCode::OK);
            assert!(served.headers().get(header::LOCATION).is_none());
            assert!(
                !to_bytes(served.into_body(), usize::MAX)
                    .await
                    .expect("dependency body")
                    .is_empty()
            );
        }
    }
}

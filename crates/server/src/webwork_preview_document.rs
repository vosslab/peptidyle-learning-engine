//! Shared preparation for isolated WeBWorK preview and capture documents.

use base64::{Engine as _, engine::general_purpose::STANDARD};
use scraper::{Html, Selector};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;

const PREVIEW_DOCUMENT_CSP: &str = "sandbox allow-scripts; default-src 'self'; script-src 'self'; style-src 'self' 'unsafe-inline'; img-src 'self' data:; base-uri 'none'; object-src 'none'; frame-ancestors 'self'; form-action 'none'";
const PARENT_TELEMETRY_SCRIPT_PATH: &str =
    "/api/webwork-assets/webwork2_files/js/apps/Problem/problem.";

/// Renderer-owned HTML plus the response policy needed to execute its inline scripts.
///
/// `html` is byte-for-byte renderer output after the one established parent telemetry
/// loader removal. Parsing is used only to compute CSP hashes; it never serializes HTML.
pub(crate) struct PreparedPreviewDocument {
    pub(crate) html: String,
    pub(crate) content_security_policy: String,
}

/// Prepares a renderer document for opaque preview/capture execution.
///
/// Returns `None` if the known parent telemetry loader is malformed or ambiguous.
pub(crate) fn prepare_preview_document(document: String) -> Option<PreparedPreviewDocument> {
    let html = without_preview_parent_telemetry(document)?;
    let parsed = Html::parse_document(&html);
    let scripts = Selector::parse("script:not([src])").ok()?;
    let hashes = parsed
        .select(&scripts)
        .filter(|script| is_executable_inline_script(script.value().attr("type")))
        .map(|script| {
            let text = script.text().collect::<String>();
            let digest = Sha256::digest(text.as_bytes());
            format!("'sha256-{}'", STANDARD.encode(digest))
        })
        .collect::<BTreeSet<_>>();

    let content_security_policy = if hashes.is_empty() {
        PREVIEW_DOCUMENT_CSP.to_owned()
    } else {
        PREVIEW_DOCUMENT_CSP.replacen(
            "script-src 'self'",
            &format!(
                "script-src 'self' {}",
                hashes.into_iter().collect::<Vec<_>>().join(" ")
            ),
            1,
        )
    };

    Some(PreparedPreviewDocument {
        html,
        content_security_policy,
    })
}

fn is_executable_inline_script(script_type: Option<&str>) -> bool {
    let Some(script_type) = script_type else {
        return true;
    };
    let essence = script_type
        .split(';')
        .next()
        .unwrap_or_default()
        .trim()
        .to_ascii_lowercase();
    if essence.is_empty() || essence == "module" {
        return true;
    }
    matches!(
        essence.as_str(),
        "application/ecmascript"
            | "application/javascript"
            | "application/x-ecmascript"
            | "application/x-javascript"
            | "text/ecmascript"
            | "text/javascript"
            | "text/javascript1.0"
            | "text/javascript1.1"
            | "text/javascript1.2"
            | "text/javascript1.3"
            | "text/javascript1.4"
            | "text/javascript1.5"
            | "text/jscript"
            | "text/livescript"
            | "text/x-ecmascript"
            | "text/x-javascript"
    )
}

pub(crate) fn without_preview_parent_telemetry(mut document: String) -> Option<String> {
    // This is the exact recognized renderer loader whose frameElement access
    // cannot run in an opaque preview and has no preview/capture behavior.
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

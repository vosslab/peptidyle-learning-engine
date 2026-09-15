//! Fixed public delivery for the current reviewed author-content runtime.
//!
//! This is not a general static-file service. The two exact paths, local
//! bytes, media types, and integrity records are closed server inputs.

use std::sync::OnceLock;

use axum::{
    Router,
    body::Body,
    extract::{OriginalUri, State},
    http::{
        HeaderValue, StatusCode,
        header::{CACHE_CONTROL, CONTENT_TYPE},
    },
    response::{IntoResponse, Response},
    routing::{MethodFilter, on},
};
use base64::{Engine as _, engine::general_purpose::STANDARD};
use sha2::{Digest, Sha256};

use crate::author_content_dependency_registry_generated::{
    RDKIT_FILES, ReviewedAuthorContentRuntimeFile,
};

const RDKIT_JAVASCRIPT_ROUTE: &str = "/api/author-content-dependencies/rdkit/RDKit_minimal.js";
const RDKIT_WASM_ROUTE: &str = "/api/author-content-dependencies/rdkit/RDKit_minimal.wasm";
const CURRENT_CACHE_CONTROL: &str = "no-cache";

const RDKIT_JAVASCRIPT_BYTES: &[u8] =
    include_bytes!("../../../assets/author-content-dependencies/rdkit/RDKit_minimal.js");
const RDKIT_WASM_BYTES: &[u8] =
    include_bytes!("../../../assets/author-content-dependencies/rdkit/RDKit_minimal.wasm");

static CURRENT_RUNTIME: OnceLock<Result<ReviewedRdkitRuntime, ()>> = OnceLock::new();

/// Server-derived facts for the current local RDKit pair. It contains no
/// package version, durable identity, or historical runtime selection.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ReviewedRdkitRuntime {
    javascript_sri: String,
}

impl ReviewedRdkitRuntime {
    pub(crate) fn javascript_path(&self) -> &'static str {
        RDKIT_JAVASCRIPT_ROUTE
    }

    pub(crate) fn javascript_sri(&self) -> &str {
        &self.javascript_sri
    }

    pub(crate) fn wasm_path(&self) -> &'static str {
        RDKIT_WASM_ROUTE
    }
}

#[derive(Clone, Copy)]
struct AssetRouteState {
    bytes: &'static [u8],
    media_type: &'static str,
}

/// Returns the current reviewed local runtime for C858 document construction.
/// A local-file mismatch fails closed before the document can name its paths.
pub(crate) fn reviewed_rdkit_runtime() -> Option<ReviewedRdkitRuntime> {
    CURRENT_RUNTIME
        .get_or_init(build_runtime)
        .as_ref()
        .ok()
        .cloned()
}

/// Registers exactly the current public JS/WASM pair. There is no wildcard,
/// directory, object-store redirect, request-state branch, or caller path.
pub(crate) fn author_content_dependency_asset_router() -> Router {
    Router::new()
        .merge(asset_router(
            RDKIT_JAVASCRIPT_ROUTE,
            AssetRouteState {
                bytes: RDKIT_JAVASCRIPT_BYTES,
                media_type: "application/javascript; charset=utf-8",
            },
        ))
        .merge(asset_router(
            RDKIT_WASM_ROUTE,
            AssetRouteState {
                bytes: RDKIT_WASM_BYTES,
                media_type: "application/wasm",
            },
        ))
}

fn asset_router(path: &str, state: AssetRouteState) -> Router {
    Router::new()
        .route(
            path,
            on(MethodFilter::GET, get_asset).on(MethodFilter::HEAD, head_asset),
        )
        .with_state(state)
}

async fn get_asset(
    State(state): State<AssetRouteState>,
    OriginalUri(uri): OriginalUri,
) -> Response {
    asset_response(uri.query().is_none(), state, false)
}

async fn head_asset(
    State(state): State<AssetRouteState>,
    OriginalUri(uri): OriginalUri,
) -> Response {
    asset_response(uri.query().is_none(), state, true)
}

fn asset_response(query_free: bool, state: AssetRouteState, head: bool) -> Response {
    if !query_free || reviewed_rdkit_runtime().is_none() {
        return concealed();
    }
    let mut response = if head {
        StatusCode::OK.into_response()
    } else {
        Response::new(Body::from(state.bytes))
    };
    let headers = response.headers_mut();
    headers.insert(CONTENT_TYPE, HeaderValue::from_static(state.media_type));
    // The public URL names current bytes, so caches must revalidate it.
    headers.insert(
        CACHE_CONTROL,
        HeaderValue::from_static(CURRENT_CACHE_CONTROL),
    );
    // ASVS 3.4.4: enforce the registry-selected JavaScript or WASM media type.
    headers.insert(
        "x-content-type-options",
        HeaderValue::from_static("nosniff"),
    );
    // ASVS 3.5.8: the reviewed public pair is the opaque-frame exception.
    headers.insert(
        "cross-origin-resource-policy",
        HeaderValue::from_static("cross-origin"),
    );
    // The opaque-origin frame fetches only these fixed public bytes. This
    // deliberate CORS exception grants neither credentials nor wider routes.
    headers.insert("access-control-allow-origin", HeaderValue::from_static("*"));
    response
}

fn build_runtime() -> Result<ReviewedRdkitRuntime, ()> {
    let javascript = exact_file("RDKit_minimal.js", "application/javascript; charset=utf-8")?;
    let wasm = exact_file("RDKit_minimal.wasm", "application/wasm")?;
    if !hash_matches(RDKIT_JAVASCRIPT_BYTES, javascript.sha256)
        || !hash_matches(RDKIT_WASM_BYTES, wasm.sha256)
    {
        return Err(());
    }
    Ok(ReviewedRdkitRuntime {
        javascript_sri: format!("sha256-{}", STANDARD.encode(hex_digest(javascript.sha256)?)),
    })
}

fn exact_file(
    expected_name: &str,
    expected_media_type: &str,
) -> Result<&'static ReviewedAuthorContentRuntimeFile, ()> {
    if RDKIT_FILES.len() != 2 {
        return Err(());
    }
    let file = RDKIT_FILES
        .iter()
        .find(|file| file.file_name == expected_name)
        .ok_or(())?;
    (file.media_type == expected_media_type)
        .then_some(file)
        .ok_or(())
}

fn hash_matches(bytes: &[u8], expected_hex: &str) -> bool {
    let actual = Sha256::digest(bytes);
    hex_digest(expected_hex).is_ok_and(|expected| actual.as_slice() == expected.as_slice())
}

fn hex_digest(value: &str) -> Result<[u8; 32], ()> {
    if value.len() != 64 {
        return Err(());
    }
    let mut digest = [0_u8; 32];
    for (index, byte) in digest.iter_mut().enumerate() {
        *byte = u8::from_str_radix(&value[index * 2..index * 2 + 2], 16).map_err(|_| ())?;
    }
    Ok(digest)
}

fn concealed() -> Response {
    StatusCode::NOT_FOUND.into_response()
}

#[cfg(test)]
mod tests {
    use axum::{body::to_bytes, http::Request};
    use tower::ServiceExt;

    use super::*;

    #[tokio::test]
    async fn current_rdkit_assets_are_byte_exact_and_have_only_their_public_contract() {
        // Regression prevented: a generic or cache-stale asset endpoint could
        // load unreviewed bytes in the isolated author-content frame. Failure
        // means repair the closed current registry/route and rerun C900/C901.
        let app = author_content_dependency_asset_router();
        let runtime = reviewed_rdkit_runtime().expect("reviewed current runtime");
        for (path, expected_media_type, expected_bytes) in [
            (
                runtime.javascript_path(),
                "application/javascript; charset=utf-8",
                RDKIT_JAVASCRIPT_BYTES,
            ),
            (runtime.wasm_path(), "application/wasm", RDKIT_WASM_BYTES),
        ] {
            let response = app
                .clone()
                .oneshot(Request::get(path).body(Body::empty()).expect("request"))
                .await
                .expect("response");
            assert_eq!(response.status(), StatusCode::OK);
            assert_eq!(response.headers()[CONTENT_TYPE], expected_media_type);
            assert_eq!(response.headers()[CACHE_CONTROL], CURRENT_CACHE_CONTROL);
            assert_eq!(response.headers()["x-content-type-options"], "nosniff");
            assert_eq!(
                response.headers()["cross-origin-resource-policy"],
                "cross-origin"
            );
            assert_eq!(response.headers()["access-control-allow-origin"], "*");
            assert!(
                response
                    .headers()
                    .get("access-control-allow-credentials")
                    .is_none()
            );
            assert_eq!(
                to_bytes(response.into_body(), usize::MAX)
                    .await
                    .expect("body"),
                expected_bytes
            );
        }
        assert!(runtime.javascript_sri().starts_with("sha256-"));
    }
}

//! Exact server-to-renderer `render_rpc` form construction.

use std::collections::BTreeMap;

use base64::Engine as _;

use crate::renderer_contract::RenderRequest;

/// Constructs the fixed, server-owned upstream render form.
pub(super) fn render_fields(request: RenderRequest<'_>) -> BTreeMap<String, String> {
    BTreeMap::from([
        (
            "problemSource".into(),
            base64::engine::general_purpose::STANDARD.encode(request.pg_source),
        ),
        ("fileName".into(), request.pg_path.to_owned()),
        ("problemSeed".into(), request.seed.to_string()),
        ("displayMode".into(), "MathJax".into()),
        ("showSummary".into(), "0".into()),
        ("showHints".into(), "0".into()),
        ("showSolutions".into(), "0".into()),
        ("showPreviewButton".into(), "0".into()),
        ("showCheckAnswersButton".into(), "0".into()),
        ("showCorrectAnswersButton".into(), "0".into()),
        ("showFooter".into(), "0".into()),
    ])
}

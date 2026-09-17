//! Exact server-to-renderer standalone render form construction.

use base64::Engine as _;
use reqwest::Url;

use crate::renderer_contract::RenderRequest;

/// Correct-answer disclosure is an explicit server-only operation.
pub(super) fn answer_review_fields(
    request: RenderRequest<'_>,
    ple_origin: &Url,
    ple_asset_base: &str,
) -> Vec<(String, String)> {
    let mut fields = render_fields(request, ple_origin, ple_asset_base);
    fields.push(("showCorrectAnswers".into(), "1".into()));
    fields.push(("showScoreSummary".into(), "0".into()));
    fields
}

/// Constructs the fixed, server-owned upstream render form.
pub(super) fn render_fields(
    request: RenderRequest<'_>,
    ple_origin: &Url,
    ple_asset_base: &str,
) -> Vec<(String, String)> {
    vec![
        ("_format".into(), "json".into()),
        (
            "problemSource".into(),
            base64::engine::general_purpose::STANDARD.encode(request.pg_source),
        ),
        ("sourceFilePath".into(), request.pg_path.to_owned()),
        ("problemSeed".into(), request.seed.to_string()),
        ("outputFormat".into(), "ple_embed".into()),
        (
            "pleOrigin".into(),
            ple_origin.as_str().trim_end_matches('/').into(),
        ),
        ("pleAssetBase".into(), ple_asset_base.into()),
        ("displayMode".into(), "MathJax".into()),
        ("isInstructor".into(), "0".into()),
        ("showSummary".into(), "0".into()),
        ("showHints".into(), "0".into()),
        ("showSolutions".into(), "0".into()),
        ("hidePreviewButton".into(), "1".into()),
        ("hideCheckAnswersButton".into(), "1".into()),
        ("hideAttemptsTable".into(), "1".into()),
        ("hideMessages".into(), "1".into()),
        ("showCorrectAnswersButton".into(), "0".into()),
        ("showFooter".into(), "0".into()),
    ]
}

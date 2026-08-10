//! MOD-EXPORT: answer-key-free print models and deterministic artifact writers.
//!
//! The worker resolves immutable, published assets before it calls this crate.
//! This crate receives verified bytes only: it never receives an object key,
//! a URL, tenant information, or an answer key.

/// Microsoft Word output.
pub mod docx;
/// PDF output.
pub mod pdf;

use std::collections::BTreeMap;

use objects::Sha256Digest;
use question_model::envelope::{AssetRef, ContentBlock};
use question_model::{
    AssetId, BackendCapabilities, Capability, QuestionDefinition, ResponseDefinition,
};

/// The rendition selected by an instructor or accessibility workflow.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PrintLayout {
    /// A printed exam with the actual published figures.
    Standard,
    /// The same figures plus their required alternatives in reading order.
    Accessible,
}

impl PrintLayout {
    /// Stable filename segment.
    pub fn filename_segment(self) -> &'static str {
        match self {
            Self::Standard => "exam",
            Self::Accessible => "exam-accessible",
        }
    }
}

/// A rendered export artifact.  The caller persists it; this type has no path,
/// URL, object key, tenant, or answer-bearing field.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExportArtifact {
    pub filename: String,
    pub media_type: &'static str,
    pub bytes: Vec<u8>,
}

/// Standard and accessible DOCX/PDF views of the same exam.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExportBundle {
    pub docx: ExportArtifact,
    pub pdf: ExportArtifact,
    pub accessible_docx: ExportArtifact,
    pub accessible_pdf: ExportArtifact,
}

/// One question plus the capability declaration made by its adapter.
#[derive(Debug, Clone)]
pub struct ExportCandidate<'a> {
    pub question: &'a QuestionDefinition,
    pub capabilities: &'a BackendCapabilities,
}

/// Verified bytes for one immutable published asset.  Image support is
/// deliberately narrow rather than silently degrading a visual question.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PrintableAsset {
    pub media_type: String,
    pub bytes: Vec<u8>,
}

/// Server-side asset lookup.  Implementations must return bytes from the exact
/// immutable asset reference; the checksum is verified again below.
pub trait TrustedAssetResolver {
    fn resolve(&self, asset: &AssetRef) -> Result<PrintableAsset, String>;
}

/// One named question refused before any writer starts.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnexportableQuestion {
    pub title: String,
    pub reason: String,
}

/// All questions that prevent an export.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExportabilityError {
    pub questions: Vec<UnexportableQuestion>,
}

impl std::fmt::Display for ExportabilityError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let names = self
            .questions
            .iter()
            .map(|q| q.title.as_str())
            .collect::<Vec<_>>()
            .join(", ");
        write!(formatter, "exam contains unexportable questions: {names}")
    }
}
impl std::error::Error for ExportabilityError {}

/// Validated, answer-key-free print model.
#[derive(Debug, Clone, PartialEq)]
pub struct PrintExam {
    pub title: String,
    pub questions: Vec<PrintQuestion>,
    assets: BTreeMap<AssetId, PrintableAsset>,
}

/// One question in a validated print exam.
#[derive(Debug, Clone, PartialEq)]
pub struct PrintQuestion {
    pub title: String,
    pub prompt: Vec<ContentBlock>,
    pub response: ResponseDefinition,
}

impl PrintExam {
    /// Compatibility entry point for questions without figures.  A figure
    /// requires the server's trusted resolver, so it is refused pre-build.
    pub fn build<'a>(
        title: impl Into<String>,
        candidates: impl IntoIterator<Item = ExportCandidate<'a>>,
    ) -> Result<Self, ExportabilityError> {
        struct NoAssets;
        impl TrustedAssetResolver for NoAssets {
            fn resolve(&self, _: &AssetRef) -> Result<PrintableAsset, String> {
                Err(
                    "a trusted published-asset resolver is required for printable figures"
                        .to_string(),
                )
            }
        }
        Self::build_with_assets(title, candidates, &NoAssets)
    }

    /// Resolves every visual asset and checks every export constraint before a
    /// DOCX/PDF buffer is allocated.  Currently both writers support verified
    /// non-interlaced 8-bit PNG figures; unsupported or corrupt media is an
    /// actionable refusal rather than a text-only replacement.
    pub fn build_with_assets<'a>(
        title: impl Into<String>,
        candidates: impl IntoIterator<Item = ExportCandidate<'a>>,
        resolver: &dyn TrustedAssetResolver,
    ) -> Result<Self, ExportabilityError> {
        let mut questions = Vec::new();
        let mut assets = BTreeMap::new();
        let mut failures = Vec::new();
        for candidate in candidates {
            let question = candidate.question;
            let mut reason = None;
            if !candidate.capabilities.supports(Capability::PrintExport) {
                reason = Some("the backend does not declare printExport".to_string());
            } else if matches!(
                question.response,
                ResponseDefinition::FileUpload { .. } | ResponseDefinition::ExternalTool {}
            ) {
                reason = Some(
                    "this response requires an online interaction and cannot be completed on paper"
                        .to_string(),
                );
            } else if let Err(problem) = resolve_question_assets(question, resolver, &mut assets) {
                reason = Some(problem);
            }
            if let Some(reason) = reason {
                failures.push(UnexportableQuestion {
                    title: question.metadata.title.clone(),
                    reason,
                });
            } else {
                questions.push(PrintQuestion {
                    title: question.metadata.title.clone(),
                    prompt: question.prompt.clone(),
                    response: question.response.clone(),
                });
            }
        }
        if failures.is_empty() {
            Ok(Self {
                title: title.into(),
                questions,
                assets,
            })
        } else {
            Err(ExportabilityError {
                questions: failures,
            })
        }
    }

    /// Produces all four deterministic artifacts.  Pre-build validation means
    /// these writers never need a lossy fallback.
    pub fn render_all(&self) -> ExportBundle {
        ExportBundle {
            docx: docx::write(self, PrintLayout::Standard),
            pdf: pdf::write(self, PrintLayout::Standard),
            accessible_docx: docx::write(self, PrintLayout::Accessible),
            accessible_pdf: pdf::write(self, PrintLayout::Accessible),
        }
    }

    pub(crate) fn asset(&self, id: AssetId) -> Option<&PrintableAsset> {
        self.assets.get(&id)
    }
}

fn resolve_question_assets(
    question: &QuestionDefinition,
    resolver: &dyn TrustedAssetResolver,
    target: &mut BTreeMap<AssetId, PrintableAsset>,
) -> Result<(), String> {
    let mut refs = Vec::new();
    refs.extend(assets_in_blocks(&question.prompt));
    match &question.response {
        ResponseDefinition::MultipleChoice { choices, .. } => {
            for choice in choices {
                refs.extend(assets_in_blocks(&choice.body));
            }
        }
        ResponseDefinition::Ordering { items } => {
            for item in items {
                refs.extend(assets_in_blocks(&item.body));
            }
        }
        _ => {}
    }
    for asset in refs {
        let resolved = resolver
            .resolve(asset)
            .map_err(|e| format!("figure {asset:?} cannot be resolved: {e}"))?;
        if resolved.media_type != "image/png" {
            return Err(format!(
                "figure {} has unsupported printable media type {} (only image/png is currently supported)",
                asset.asset, resolved.media_type
            ));
        }
        if Sha256Digest::compute(&resolved.bytes).to_string() != asset.checksum {
            return Err(format!(
                "figure {} bytes do not match its published checksum",
                asset.asset
            ));
        }
        if !pdf::png_is_supported(&resolved.bytes) {
            return Err(format!(
                "figure {} is not a supported 8-bit non-interlaced PNG",
                asset.asset
            ));
        }
        match target.get(&asset.asset) {
            Some(existing) if existing != &resolved => {
                return Err(format!(
                    "figure {} resolved to conflicting bytes",
                    asset.asset
                ));
            }
            Some(_) => {}
            None => {
                target.insert(asset.asset, resolved);
            }
        }
    }
    Ok(())
}

fn assets_in_blocks(blocks: &[ContentBlock]) -> Vec<&AssetRef> {
    blocks
        .iter()
        .filter_map(|block| match block {
            ContentBlock::Image { asset, .. } => Some(asset),
            _ => None,
        })
        .collect()
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum FlowBlock {
    Text { text: String, keep_with_next: bool },
    Image { asset: AssetId, alternative: String },
}

pub(crate) fn exam_flow(exam: &PrintExam, layout: PrintLayout) -> Vec<Vec<FlowBlock>> {
    let mut questions = vec![vec![FlowBlock::Text {
        text: exam.title.clone(),
        keep_with_next: true,
    }]];
    for (index, question) in exam.questions.iter().enumerate() {
        let mut blocks = vec![FlowBlock::Text {
            text: format!("{}. {}", index + 1, question.title),
            keep_with_next: true,
        }];
        for block in &question.prompt {
            append_block_flow(&mut blocks, block, layout);
        }
        append_response_flow(&mut blocks, &question.response, layout);
        questions.push(blocks);
    }
    questions
}

fn append_block_flow(target: &mut Vec<FlowBlock>, block: &ContentBlock, layout: PrintLayout) {
    match block {
        ContentBlock::Text { markdown } => target.push(FlowBlock::Text {
            text: markdown.clone(),
            keep_with_next: false,
        }),
        ContentBlock::Math { latex, description } => target.push(FlowBlock::Text {
            text: match layout {
                PrintLayout::Standard => format!("Math: {latex} ({description})"),
                PrintLayout::Accessible => format!("Math alternative: {description} [{latex}]"),
            },
            keep_with_next: false,
        }),
        ContentBlock::Image { asset, description } => {
            target.push(FlowBlock::Image {
                asset: asset.asset,
                alternative: description.clone(),
            });
            if layout == PrintLayout::Accessible {
                target.push(FlowBlock::Text {
                    text: format!("Figure alternative: {description}"),
                    keep_with_next: false,
                });
            }
        }
        ContentBlock::Code { language, source } => {
            target.push(FlowBlock::Text {
                text: format!("Code ({language}):"),
                keep_with_next: true,
            });
            for line in source.lines() {
                target.push(FlowBlock::Text {
                    text: line.to_string(),
                    keep_with_next: false,
                });
            }
        }
        ContentBlock::Table {
            headers,
            rows,
            description,
        } => {
            target.push(FlowBlock::Text {
                text: format!("Table: {description}"),
                keep_with_next: true,
            });
            target.push(FlowBlock::Text {
                text: headers.join(" | "),
                keep_with_next: true,
            });
            for row in rows {
                target.push(FlowBlock::Text {
                    text: row.join(" | "),
                    keep_with_next: false,
                });
            }
        }
    }
}

fn append_response_flow(
    target: &mut Vec<FlowBlock>,
    response: &ResponseDefinition,
    layout: PrintLayout,
) {
    match response {
        ResponseDefinition::Numeric { unit, .. } => target.push(FlowBlock::Text {
            text: unit.as_ref().map_or_else(
                || "Answer: ____________________".to_string(),
                |unit| format!("Answer: ____________________ {unit}"),
            ),
            keep_with_next: false,
        }),
        ResponseDefinition::MultipleChoice { choices, .. } => {
            for (index, choice) in choices.iter().enumerate() {
                target.push(FlowBlock::Text {
                    text: format!("   {}.", letters(index)),
                    keep_with_next: true,
                });
                for block in &choice.body {
                    append_block_flow(target, block, layout);
                }
            }
        }
        ResponseDefinition::ShortText { .. } => target.push(FlowBlock::Text {
            text: "Answer: ____________________________________________________________"
                .to_string(),
            keep_with_next: false,
        }),
        ResponseDefinition::MultiBlank { blanks } => {
            for blank in blanks {
                for block in &blank.label {
                    append_block_flow(target, block, layout);
                }
                target.push(FlowBlock::Text {
                    text: "Answer: ______________________________".to_string(),
                    keep_with_next: false,
                });
            }
        }
        ResponseDefinition::Matching { prompts, choices } => {
            target.push(FlowBlock::Text {
                text: "Match each prompt to one choice.".to_string(),
                keep_with_next: true,
            });
            for prompt in prompts {
                target.push(FlowBlock::Text {
                    text: "Prompt: ____".to_string(),
                    keep_with_next: true,
                });
                for block in &prompt.body {
                    append_block_flow(target, block, layout);
                }
            }
            target.push(FlowBlock::Text {
                text: "Choices:".to_string(),
                keep_with_next: true,
            });
            for choice in choices {
                for block in &choice.body {
                    append_block_flow(target, block, layout);
                }
            }
        }
        ResponseDefinition::Ordering { items } => {
            target.push(FlowBlock::Text {
                text: "Write the order: ______________________________".to_string(),
                keep_with_next: true,
            });
            for item in items {
                target.push(FlowBlock::Text {
                    text: "   -".to_string(),
                    keep_with_next: true,
                });
                for block in &item.body {
                    append_block_flow(target, block, layout);
                }
            }
        }
        ResponseDefinition::Hotspot {
            description,
            regions,
            ..
        } => {
            target.push(FlowBlock::Text {
                text: format!("Hotspot surface: {description}"),
                keep_with_next: true,
            });
            target.push(FlowBlock::Text {
                text: "Accessible region choices:".to_string(),
                keep_with_next: true,
            });
            for region in regions {
                for block in &region.label {
                    append_block_flow(target, block, layout);
                }
            }
        }
        ResponseDefinition::FileUpload { .. } | ResponseDefinition::ExternalTool {} => {
            unreachable!("validated before print build")
        }
    }
}

fn letters(index: usize) -> String {
    let mut number = index + 1;
    let mut result = String::new();
    while number > 0 {
        let remainder = (number - 1) % 26;
        result.insert(0, char::from(b'A' + remainder as u8));
        number = (number - 1) / 26;
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::Deserialize;
    use std::collections::BTreeMap;
    use std::fs;
    use std::process::Command;

    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct Fixture {
        published_problem: QuestionDefinition,
    }
    fn fixture_question() -> QuestionDefinition {
        serde_json::from_str::<Fixture>(include_str!(
            "../../../tests/fixtures/published_problem/corpus.json"
        ))
        .expect("fixture")
        .published_problem
    }
    fn printable_capabilities() -> BackendCapabilities {
        BackendCapabilities::from_iter([Capability::PrintExport])
    }
    fn png() -> Vec<u8> {
        vec![
            137, 80, 78, 71, 13, 10, 26, 10, 0, 0, 0, 13, 73, 72, 68, 82, 0, 0, 0, 1, 0, 0, 0, 1,
            8, 2, 0, 0, 0, 144, 119, 83, 222, 0, 0, 0, 12, 73, 68, 65, 84, 120, 156, 99, 248, 207,
            192, 0, 0, 3, 1, 1, 0, 201, 254, 146, 239, 0, 0, 0, 0, 73, 69, 78, 68, 174, 66, 96,
            130,
        ]
    }
    struct Assets(BTreeMap<AssetId, PrintableAsset>);
    impl TrustedAssetResolver for Assets {
        fn resolve(&self, asset: &AssetRef) -> Result<PrintableAsset, String> {
            self.0
                .get(&asset.asset)
                .cloned()
                .ok_or_else(|| "not found".to_string())
        }
    }
    fn assets_for(question: &QuestionDefinition) -> Assets {
        let bytes = png();
        let mut map = BTreeMap::new();
        let mut all = assets_in_blocks(&question.prompt);
        match &question.response {
            ResponseDefinition::MultipleChoice { choices, .. } => {
                for choice in choices {
                    all.extend(assets_in_blocks(&choice.body));
                }
            }
            ResponseDefinition::Ordering { items } => {
                for item in items {
                    all.extend(assets_in_blocks(&item.body));
                }
            }
            _ => {}
        }
        for asset in all {
            map.insert(
                asset.asset,
                PrintableAsset {
                    media_type: "image/png".to_string(),
                    bytes: bytes.clone(),
                },
            );
        }
        Assets(map)
    }
    fn set_asset_checksums(question: &mut QuestionDefinition) {
        let checksum = Sha256Digest::compute(&png()).to_string();
        for block in &mut question.prompt {
            if let ContentBlock::Image { asset, .. } = block {
                asset.checksum = checksum.clone();
            }
        }
        let groups = match &mut question.response {
            ResponseDefinition::MultipleChoice { choices, .. } => choices,
            ResponseDefinition::Ordering { items } => items,
            _ => return,
        };
        for item in groups {
            for block in &mut item.body {
                if let ContentBlock::Image { asset, .. } = block {
                    asset.checksum = checksum.clone();
                }
            }
        }
    }

    #[test]
    fn build_refuses_unavailable_assets_before_writing() {
        let question = fixture_question();
        let error = PrintExam::build(
            "Midterm",
            [ExportCandidate {
                question: &question,
                capabilities: &printable_capabilities(),
            }],
        )
        .expect_err("figure must resolve");
        assert!(
            error.questions[0]
                .reason
                .contains("trusted published-asset")
        );
    }
    #[test]
    fn four_artifacts_embed_figure_and_accessible_alternative() {
        let mut question = fixture_question();
        let checksum = Sha256Digest::compute(&png()).to_string();
        for block in &mut question.prompt {
            if let ContentBlock::Image { asset, .. } = block {
                asset.checksum = checksum.clone();
            }
        }
        let assets = assets_for(&question);
        let exam = PrintExam::build_with_assets(
            "Biochemistry",
            [ExportCandidate {
                question: &question,
                capabilities: &printable_capabilities(),
            }],
            &assets,
        )
        .expect("printable");
        let one = exam.render_all();
        assert_eq!(one, exam.render_all());
        assert!(String::from_utf8_lossy(&one.docx.bytes).contains("word/media/image1.png"));
        assert!(one.pdf.bytes.windows(4).any(|part| part == b"/Im1"));
        assert!(
            String::from_utf8_lossy(&one.accessible_docx.bytes).contains("Figure alternative:")
        );
        assert_eq!(one.docx.filename, "exam.docx");
        assert_eq!(one.accessible_pdf.filename, "exam-accessible.pdf");
    }
    #[test]
    fn unicode_scientific_content_is_embedded_and_extractable() {
        let mut question = fixture_question();
        question.metadata.title =
            "\u{03b2}-sheet: \u{03bc}M at 37\u{00b0}C \u{2192} caf\u{00e9}; x\u{00b2} + H\u{2082}O"
                .to_string();
        let checksum = Sha256Digest::compute(&png()).to_string();
        for block in &mut question.prompt {
            if let ContentBlock::Image { asset, .. } = block {
                asset.checksum = checksum.clone();
            }
        }
        let exam = PrintExam::build_with_assets(
            "Exam",
            [ExportCandidate {
                question: &question,
                capabilities: &printable_capabilities(),
            }],
            &assets_for(&question),
        )
        .expect("ordinary scientific Unicode must be printable");
        let artifact = exam.render_all().pdf;
        let directory = std::env::temp_dir().join(format!("ple-unicode-{}", std::process::id()));
        fs::create_dir_all(&directory).expect("temporary directory");
        let path = directory.join("unicode.pdf");
        fs::write(&path, artifact.bytes).expect("PDF write");
        let output = Command::new("pdftotext")
            .arg(&path)
            .arg("-")
            .output()
            .expect("pdftotext");
        assert!(output.status.success());
        let extracted = String::from_utf8(output.stdout).expect("UTF-8 extracted text");
        for expected in [
            "\u{03b2}-sheet",
            "\u{03bc}M",
            "37\u{00b0}C",
            "\u{2192}",
            "caf\u{00e9}",
            "x\u{00b2}",
            "H\u{2082}O",
        ] {
            assert!(
                extracted.contains(expected),
                "PDF extractor lost {expected:?}: {extracted}"
            );
        }
        fs::remove_dir_all(directory).expect("temporary cleanup");
    }
    #[test]
    fn labels_continue_after_z() {
        assert_eq!(letters(26), "AA");
        assert_eq!(letters(51), "AZ");
    }

    #[test]
    fn choice_and_ordering_figures_are_embedded_with_accessible_alternatives() {
        let mut question = fixture_question();
        set_asset_checksums(&mut question);
        let image = question
            .prompt
            .iter_mut()
            .find_map(|block| match block {
                ContentBlock::Image { asset, description } => {
                    Some((asset.clone(), description.clone()))
                }
                _ => None,
            })
            .expect("fixture image");
        let figure = |description: &str| ContentBlock::Image {
            asset: image.0.clone(),
            description: description.to_string(),
        };
        question.response = ResponseDefinition::MultipleChoice {
            choices: vec![question_model::response::ChoiceOption {
                id: question_model::response::ChoiceId::new("figure"),
                body: vec![figure("choice figure")],
            }],
            selection: question_model::answer::SelectionCardinality::ExactlyOne,
        };
        let exam = PrintExam::build_with_assets(
            "Exam",
            [ExportCandidate {
                question: &question,
                capabilities: &printable_capabilities(),
            }],
            &assets_for(&question),
        )
        .expect("choice figure resolves");
        let bundle = exam.render_all();
        let document = String::from_utf8_lossy(&bundle.docx.bytes);
        assert!(
            document.matches("<wp:docPr").count() >= 2,
            "choice figure must be a drawing"
        );
        assert!(String::from_utf8_lossy(&bundle.accessible_docx.bytes).contains("choice figure"));
        assert!(bundle.pdf.bytes.windows(4).any(|window| window == b"/Im1"));

        question.response = ResponseDefinition::Ordering {
            items: vec![question_model::response::ChoiceOption {
                id: question_model::response::ChoiceId::new("ordering-figure"),
                body: vec![figure("ordering figure")],
            }],
        };
        let ordering = PrintExam::build_with_assets(
            "Exam",
            [ExportCandidate {
                question: &question,
                capabilities: &printable_capabilities(),
            }],
            &assets_for(&question),
        )
        .expect("ordering figure resolves")
        .render_all();
        assert!(
            String::from_utf8_lossy(&ordering.docx.bytes)
                .matches("<wp:docPr")
                .count()
                >= 2,
            "ordering figure must be a drawing"
        );
        assert!(
            String::from_utf8_lossy(&ordering.accessible_docx.bytes).contains("ordering figure")
        );
    }

    #[test]
    fn every_supported_response_shape_builds_four_readable_artifacts() {
        let mut base = fixture_question();
        let checksum = Sha256Digest::compute(&png()).to_string();
        for block in &mut base.prompt {
            if let ContentBlock::Image { asset, .. } = block {
                asset.checksum = checksum.clone();
            }
        }
        let candidates = [
            ResponseDefinition::Numeric {
                tolerance: question_model::answer::NumericTolerance::Absolute { epsilon: 0.1 },
                unit: Some("mM".to_string()),
            },
            ResponseDefinition::MultipleChoice {
                choices: vec![],
                selection: question_model::answer::SelectionCardinality::ExactlyOne,
            },
            ResponseDefinition::ShortText {
                match_mode: question_model::answer::TextMatchMode::CaseInsensitive,
                max_length: 100,
            },
            ResponseDefinition::Ordering { items: vec![] },
        ];
        for response in candidates {
            let mut question = base.clone();
            question.response = response;
            let assets = assets_for(&question);
            let exam = PrintExam::build_with_assets(
                "Response shapes",
                [ExportCandidate {
                    question: &question,
                    capabilities: &printable_capabilities(),
                }],
                &assets,
            )
            .expect("supported response must build");
            let bundle = exam.render_all();
            assert_eq!(bundle.docx.bytes[..4], *b"PK\x03\x04");
            assert_eq!(bundle.pdf.bytes[..8], *b"%PDF-1.4");
        }
    }

    #[test]
    fn independent_readers_accept_all_four_artifacts() {
        let mut question = fixture_question();
        let checksum = Sha256Digest::compute(&png()).to_string();
        for block in &mut question.prompt {
            if let ContentBlock::Image { asset, .. } = block {
                asset.checksum = checksum.clone();
            }
        }
        let assets = assets_for(&question);
        let exam = PrintExam::build_with_assets(
            "Reader validation",
            [ExportCandidate {
                question: &question,
                capabilities: &printable_capabilities(),
            }],
            &assets,
        )
        .expect("fixture builds");
        let directory =
            std::env::temp_dir().join(format!("ple-export-test-{}", std::process::id()));
        fs::create_dir_all(&directory).expect("temporary directory");
        for artifact in [
            exam.render_all().docx,
            exam.render_all().accessible_docx,
            exam.render_all().pdf,
            exam.render_all().accessible_pdf,
        ] {
            let path = directory.join(&artifact.filename);
            fs::write(&path, artifact.bytes).expect("artifact write");
            let output = if artifact.media_type == "application/pdf" {
                let info = Command::new("pdfinfo")
                    .arg(&path)
                    .output()
                    .expect("pdfinfo must be installed");
                assert!(info.status.success(), "pdfinfo rejected {}", path.display());
                Command::new("pdftotext")
                    .arg(&path)
                    .arg("-")
                    .output()
                    .expect("pdftotext must be installed")
            } else {
                let zip = Command::new("unzip")
                    .arg("-t")
                    .arg(&path)
                    .output()
                    .expect("unzip must be installed");
                if std::env::var_os("PLE_DOCX_RENDER_PROBE").is_some() {
                    let converted = Command::new("soffice")
                        .arg("--headless")
                        .arg("--convert-to")
                        .arg("pdf")
                        .arg("--outdir")
                        .arg(&directory)
                        .arg(&path)
                        .output()
                        .expect("soffice must be installed for the requested DOCX render probe");
                    assert!(
                        converted.status.success(),
                        "LibreOffice rejected {}: {}",
                        path.display(),
                        String::from_utf8_lossy(&converted.stderr)
                    );
                    assert!(
                        directory
                            .join(artifact.filename.replace(".docx", ".pdf"))
                            .is_file(),
                        "LibreOffice produced no PDF for {}",
                        path.display()
                    );
                }
                zip
            };
            assert!(
                output.status.success(),
                "reader rejected {}",
                path.display()
            );
            if artifact.media_type == "application/pdf" {
                assert!(
                    String::from_utf8_lossy(&output.stdout).contains("Reader validation"),
                    "PDF extractor lost the exam title"
                );
                let rendered = directory.join(format!("{}-render", artifact.filename));
                let raster = Command::new("pdftoppm")
                    .arg("-png")
                    .arg("-singlefile")
                    .arg(&path)
                    .arg(&rendered)
                    .output()
                    .expect("pdftoppm must be installed");
                assert!(
                    raster.status.success(),
                    "renderer rejected {}",
                    path.display()
                );
                assert!(
                    std::path::PathBuf::from(format!("{}.png", rendered.display())).is_file(),
                    "renderer wrote no image"
                );
            }
        }
        fs::remove_dir_all(directory).expect("temporary cleanup");
    }
}

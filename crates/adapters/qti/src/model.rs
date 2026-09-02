use std::collections::BTreeMap;

use crate::profiles::NormalizedQtiItemFingerprint;
use question_model::QuestionAssetId;
use question_model::QuestionContentBlock;
use question_model::QuestionResponseFormat;
use question_model::response::ResponseItemReference;
use serde::{Deserialize, Serialize};

/// Hard resource limits enforced before extraction or XML parsing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct QtiImportLimits {
    pub max_archive_bytes: usize,
    pub max_expanded_bytes: u64,
    pub max_entries: usize,
    pub max_file_bytes: u64,
    /// Maximum nested XML element depth, including the root element.
    pub max_xml_depth: usize,
    /// Maximum XML tokens accepted from a single package XML document.
    pub max_xml_tokens: usize,
    /// Maximum element nodes retained in a single package XML tree.
    pub max_xml_nodes: usize,
}

impl Default for QtiImportLimits {
    fn default() -> Self {
        Self {
            max_archive_bytes: 32 * 1024 * 1024,
            max_expanded_bytes: 128 * 1024 * 1024,
            max_entries: 256,
            max_file_bytes: 32 * 1024 * 1024,
            max_xml_depth: 128,
            max_xml_tokens: 100_000,
            max_xml_nodes: 25_000,
        }
    }
}

/// Original immutable package bytes, retained verbatim for export/re-import.
#[derive(Clone, PartialEq)]
pub(crate) struct ArchivedQtiPackage {
    pub(crate) bytes: Vec<u8>,
    pub(crate) package_checksum: String,
    pub(crate) size_bytes: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct QtiResource {
    pub identifier: String,
    pub resource_type: Option<String>,
    pub href: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct QtiManifest {
    pub identifier: Option<String>,
    pub resources: Vec<QtiResource>,
}

/// A feature retained in the original package but deliberately not converted.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UnsupportedFeature {
    pub location: String,
    pub feature: String,
    pub detail: String,
}

/// Per-resource import outcome retained even when another item is accepted.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum QtiItemImportStatus {
    Accepted,
    Rejected,
}

/// Answer-free result for one QTI item resource in the source package.
///
/// The normalized checksum includes the private grading binding, but the
/// binding itself remains absent. It can therefore identify exact duplicate
/// content without becoming an answer-disclosure surface.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QtiItemImportResult {
    pub source_identifier: String,
    pub item_id: Option<String>,
    pub normalized_qti_item_fingerprint: Option<NormalizedQtiItemFingerprint>,
    pub status: QtiItemImportStatus,
    pub warnings: Vec<UnsupportedFeature>,
}

/// One immutable media object the import worker must write before publishing.
///
/// `bytes` is intentionally excluded from JSON.  It is an import-worker
/// handoff, not a browser or draft projection.  The worker uses `asset` as the
/// logical ID and writes the bytes under the eventual immutable Object Address.
#[derive(Clone, PartialEq, Eq)]
pub struct QtiAssetObject {
    pub(crate) asset: QuestionAssetId,
    pub(crate) source_path: String,
    pub(crate) sha256: String,
    pub(crate) media_type: String,
    pub(crate) bytes: Vec<u8>,
}

impl std::fmt::Debug for QtiAssetObject {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("QtiAssetObject")
            .field("asset", &self.asset)
            .field("source_path", &"<worker-only>")
            .field("sha256", &self.sha256)
            .field("media_type", &self.media_type)
            .field("bytes", &"<worker-only>")
            .finish()
    }
}

impl QtiAssetObject {
    /// Returns bytes only to the import worker which owns the object write.
    pub fn worker_bytes(&self) -> &[u8] {
        &self.bytes
    }

    /// Logical asset identity the worker carries into its private registry.
    pub fn worker_asset_id(&self) -> QuestionAssetId {
        self.asset
    }

    /// Package-relative source path for the worker's private registry only.
    pub fn worker_source_path(&self) -> &str {
        &self.source_path
    }

    /// Adapter-sniffed media type, never browser-supplied MIME metadata.
    pub fn worker_media_type(&self) -> &str {
        &self.media_type
    }

    /// SHA-256 of the verified extracted bytes.
    pub fn worker_sha256(&self) -> &str {
        &self.sha256
    }
}

/// Public, answer-free QTI question projection.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ImportedQtiQuestion {
    pub item_id: String,
    pub prompt: Vec<QuestionContentBlock>,
    pub response: QuestionResponseFormat,
}

/// Returns each logical item asset with the checksum embedded in its public
/// QTI presentation reference. Conflicting duplicate references are invalid:
/// the same logical asset cannot name two different immutable byte strings.
pub fn qti_question_asset_checksums(
    question: &ImportedQtiQuestion,
) -> Result<BTreeMap<QuestionAssetId, String>, QtiAssetReferenceError> {
    let mut response_blocks: Vec<&QuestionContentBlock> = Vec::new();
    let mut assets = BTreeMap::new();
    match &question.response {
        question_model::QuestionResponseFormat::MultipleChoice { choices, .. } => {
            response_blocks.extend(choices.iter().flat_map(|choice| choice.body.iter()));
        }
        question_model::QuestionResponseFormat::Ordering { items } => {
            response_blocks.extend(items.iter().flat_map(|item| item.body.iter()));
        }
        question_model::QuestionResponseFormat::MultiBlank { blanks } => {
            response_blocks.extend(blanks.iter().flat_map(|blank| blank.label.iter()))
        }
        question_model::QuestionResponseFormat::Matching { prompts, choices } => {
            response_blocks.extend(prompts.iter().flat_map(|prompt| prompt.body.iter()));
            response_blocks.extend(choices.iter().flat_map(|choice| choice.body.iter()));
        }
        question_model::QuestionResponseFormat::Hotspot {
            surface, regions, ..
        } => {
            assets.insert(surface.asset, surface.checksum.clone());
            response_blocks.extend(regions.iter().flat_map(|region| region.label.iter()));
        }
        question_model::QuestionResponseFormat::Numeric { .. }
        | question_model::QuestionResponseFormat::ShortText { .. }
        | question_model::QuestionResponseFormat::ImathasQuestionBackend {} => {}
    }
    for block in question.prompt.iter().chain(response_blocks) {
        if let QuestionContentBlock::Image { asset, .. } = block
            && let Some(previous) = assets.insert(asset.asset, asset.checksum.clone())
            && previous != asset.checksum
        {
            return Err(QtiAssetReferenceError::ConflictingChecksum);
        }
    }
    Ok(assets)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QtiAssetReferenceError {
    ConflictingChecksum,
}

/// Server-only answer handoff. It is intentionally crate-private: public QTI
/// callers receive only a Question Prompt and cannot retrieve or compare the
/// Answer Key.
#[derive(Clone, PartialEq, Eq)]
pub(crate) struct QtiGradingHandoff {
    pub(crate) choices_by_item: BTreeMap<String, ResponseItemReference>,
}

/// Result of a validated import. `grading` and original bytes are deliberately
/// not serializable. Persist their two server-only handoffs independently.
#[derive(Clone, PartialEq)]
pub struct ImportedQtiPackage {
    pub(crate) original: ArchivedQtiPackage,
    pub manifest: QtiManifest,
    pub questions: Vec<ImportedQtiQuestion>,
    /// Import-worker-only object manifest; it is persisted before a public
    /// question projection is made and is never included in that projection.
    pub(crate) assets: Vec<QtiAssetObject>,
    pub unsupported: Vec<UnsupportedFeature>,
    /// Complete answer-free per-item report, including rejected resources.
    pub item_results: Vec<QtiItemImportResult>,
    pub(crate) grading: QtiGradingHandoff,
}

impl std::fmt::Debug for ImportedQtiPackage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Do not include questions here: a choice identifier can happen to be
        // the correct choice, so a generic debug log must not become a route
        // for associating a choice with correctness.
        f.debug_struct("ImportedQtiPackage")
            .field("manifest", &self.manifest)
            .field("question_count", &self.questions.len())
            .field("asset_count", &self.assets.len())
            .field("unsupported", &self.unsupported)
            .field("item_result_count", &self.item_results.len())
            .field("original", &"<server-only>")
            .field("grading", &"<server-only>")
            .finish()
    }
}

impl ImportedQtiPackage {
    /// The authoritative archive bytes for the server-owned import worker.
    ///
    /// This handoff is deliberately Rust-only: neither this package nor this
    /// method's result implements serialization for browser, WASM, or API
    /// delivery. The worker owns durable object writes and must never surface
    /// the bytes in draft or Question Library JSON.
    pub fn worker_original_bytes(&self) -> &[u8] {
        &self.original.bytes
    }

    /// Immutable package checksum used by the import worker before it
    /// records a private workspace source object.
    pub fn worker_original_package_checksum(&self) -> &str {
        &self.original.package_checksum
    }

    /// Exact archive size used by the worker's durable metadata record.
    pub fn worker_original_size_bytes(&self) -> u64 {
        self.original.size_bytes
    }

    /// Verified extracted assets for server-owned workspace persistence.
    pub fn worker_assets(&self) -> &[QtiAssetObject] {
        &self.assets
    }

    /// Returns the private correct-choice mapping only to the server import
    /// worker so it can write the grader-owned record. No browser projection,
    /// generated type, or Debug implementation receives this association.
    pub fn worker_correct_choice(&self, item_id: &str) -> Option<ResponseItemReference> {
        self.grading.choices_by_item.get(item_id).cloned()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum QtiImportError {
    ArchiveTooLarge { actual: usize, limit: usize },
    InvalidArchive(String),
    UnsafeEntry { path: String, reason: String },
    MissingManifest,
    InvalidXml { path: String, reason: String },
    MissingReferencedEntry { path: String },
    UnsupportedMedia { path: String, reason: String },
}

impl std::fmt::Display for QtiImportError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ArchiveTooLarge { actual, limit } => {
                write!(f, "QTI archive is {actual} bytes; limit is {limit} bytes")
            }
            Self::InvalidArchive(reason) => write!(f, "invalid QTI ZIP archive: {reason}"),
            Self::UnsafeEntry { path, reason } => {
                write!(f, "unsafe QTI ZIP entry `{path}`: {reason}")
            }
            Self::MissingManifest => write!(f, "QTI package has no imsmanifest.xml"),
            Self::InvalidXml { path, reason } => write!(f, "invalid QTI XML in `{path}`: {reason}"),
            Self::MissingReferencedEntry { path } => {
                write!(f, "QTI manifest references missing entry `{path}`")
            }
            Self::UnsupportedMedia { path, reason } => {
                write!(f, "unsupported media `{path}`: {reason}")
            }
        }
    }
}
impl std::error::Error for QtiImportError {}

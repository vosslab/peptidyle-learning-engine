//! Bounded QTI ZIP import and archival export.
//!
//! The original ZIP is authoritative and is never reconstructed.  Conversion
//! accepts a deliberately small QTI subset using an XML event parser: DTDs,
//! entity declarations, malformed nesting, and duplicate attributes fail
//! before any model is made.  Asset bytes leave this module only in an
//! immutable worker handoff; student-visible questions contain `AssetId`s and
//! checksums, never archive paths or answer material.

use std::collections::{BTreeMap, BTreeSet};
use std::io::{Cursor, Read};

use objects::Sha256Digest;
use question_model::answer::SelectionCardinality;
use question_model::envelope::{AssetRef, ContentBlock};
use question_model::response::{ChoiceId, ChoiceOption};
use question_model::{AssetId, ResponseDefinition};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use xmlparser::{ElementEnd, Token, Tokenizer};
use zip::ZipArchive;

const MANIFEST_PATH: &str = "imsmanifest.xml";

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
    bytes: Vec<u8>,
    sha256: String,
    size_bytes: u64,
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

/// One immutable media object the import worker must write before publishing.
///
/// `bytes` is intentionally excluded from JSON.  It is an import-worker
/// handoff, not a browser or draft projection.  The worker uses `asset` as the
/// logical ID and writes the bytes under the eventual immutable object key.
#[derive(Clone, PartialEq, Eq)]
pub struct QtiAssetObject {
    asset: AssetId,
    source_path: String,
    sha256: String,
    media_type: String,
    bytes: Vec<u8>,
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
    pub fn worker_asset_id(&self) -> AssetId {
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
    pub prompt: Vec<ContentBlock>,
    pub response: ResponseDefinition,
}

/// Returns each logical item asset with the checksum embedded in its public
/// QTI presentation reference. Conflicting duplicate references are invalid:
/// the same logical asset cannot name two different immutable byte strings.
pub fn qti_question_asset_checksums(
    question: &ImportedQtiQuestion,
) -> Result<BTreeMap<AssetId, String>, QtiAssetReferenceError> {
    let choice_blocks: Box<dyn Iterator<Item = &ContentBlock> + '_> = match &question.response {
        question_model::ResponseDefinition::MultipleChoice { choices, .. }
        | question_model::ResponseDefinition::Ordering { items: choices } => {
            Box::new(choices.iter().flat_map(|choice| choice.body.iter()))
        }
        question_model::ResponseDefinition::Numeric { .. }
        | question_model::ResponseDefinition::ShortText { .. }
        | question_model::ResponseDefinition::FileUpload { .. }
        | question_model::ResponseDefinition::ExternalTool {} => Box::new(std::iter::empty()),
    };
    let mut assets = BTreeMap::new();
    for block in question.prompt.iter().chain(choice_blocks) {
        if let ContentBlock::Image { asset, .. } = block
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
/// callers receive only a question projection and cannot retrieve or compare
/// correctness material.
#[derive(Clone, PartialEq, Eq)]
struct QtiGradingHandoff {
    choices_by_item: BTreeMap<String, ChoiceId>,
}

/// Result of a validated import. `grading` and original bytes are deliberately
/// not serializable. Persist their two server-only handoffs independently.
#[derive(Clone, PartialEq)]
pub struct ImportedQtiPackage {
    original: ArchivedQtiPackage,
    pub manifest: QtiManifest,
    pub questions: Vec<ImportedQtiQuestion>,
    /// Import-worker-only object manifest; it is persisted before a public
    /// question projection is made and is never included in that projection.
    assets: Vec<QtiAssetObject>,
    pub unsupported: Vec<UnsupportedFeature>,
    grading: QtiGradingHandoff,
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
    /// the bytes in draft or catalog JSON.
    pub fn worker_original_bytes(&self) -> &[u8] {
        &self.original.bytes
    }

    /// Checksummed archive metadata used by the import worker before it
    /// records a private workspace source object.
    pub fn worker_original_sha256(&self) -> &str {
        &self.original.sha256
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
    pub fn worker_correct_choice(&self, item_id: &str) -> Option<ChoiceId> {
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

#[derive(Debug, Clone, Copy)]
pub struct QtiImporter {
    limits: QtiImportLimits,
}

impl QtiImporter {
    pub fn new(limits: QtiImportLimits) -> Self {
        Self { limits }
    }

    /// Validates the ZIP before extracting a bounded QTI single-choice subset.
    pub fn import(&self, bytes: &[u8]) -> Result<ImportedQtiPackage, QtiImportError> {
        if bytes.len() > self.limits.max_archive_bytes {
            return Err(QtiImportError::ArchiveTooLarge {
                actual: bytes.len(),
                limit: self.limits.max_archive_bytes,
            });
        }
        let mut archive = ZipArchive::new(Cursor::new(bytes))
            .map_err(|e| QtiImportError::InvalidArchive(e.to_string()))?;
        if archive.len() > self.limits.max_entries {
            return Err(QtiImportError::UnsafeEntry {
                path: "<archive>".into(),
                reason: format!(
                    "contains {} entries; limit is {}",
                    archive.len(),
                    self.limits.max_entries
                ),
            });
        }
        let mut entries = BTreeMap::new();
        let mut expanded = 0_u64;
        for index in 0..archive.len() {
            let mut entry = archive
                .by_index(index)
                .map_err(|e| QtiImportError::InvalidArchive(e.to_string()))?;
            let path = entry.name().to_string();
            validate_entry_path(&path, &entry)?;
            if entry.size() > self.limits.max_file_bytes {
                return Err(QtiImportError::UnsafeEntry {
                    path,
                    reason: format!(
                        "expanded size {} exceeds per-file limit {}",
                        entry.size(),
                        self.limits.max_file_bytes
                    ),
                });
            }
            if entry.is_dir() {
                continue;
            }
            let mut contents = Vec::new();
            entry
                .by_ref()
                .take(self.limits.max_file_bytes.saturating_add(1))
                .read_to_end(&mut contents)
                .map_err(|e| QtiImportError::InvalidArchive(e.to_string()))?;
            let actual =
                u64::try_from(contents.len()).map_err(|_| QtiImportError::UnsafeEntry {
                    path: path.clone(),
                    reason: "expanded entry length overflow".into(),
                })?;
            if actual > self.limits.max_file_bytes {
                return Err(QtiImportError::UnsafeEntry {
                    path,
                    reason: format!(
                        "expanded size {actual} exceeds per-file limit {}",
                        self.limits.max_file_bytes
                    ),
                });
            }
            expanded = expanded
                .checked_add(actual)
                .ok_or_else(|| QtiImportError::UnsafeEntry {
                    path: "<archive>".into(),
                    reason: "expanded size overflow".into(),
                })?;
            if expanded > self.limits.max_expanded_bytes {
                return Err(QtiImportError::UnsafeEntry {
                    path,
                    reason: format!(
                        "expanded archive size {expanded} exceeds limit {}",
                        self.limits.max_expanded_bytes
                    ),
                });
            }
            if entries.insert(path.clone(), contents).is_some() {
                return Err(QtiImportError::UnsafeEntry {
                    path,
                    reason: "duplicate entry path".into(),
                });
            }
        }
        let manifest = parse_manifest(
            MANIFEST_PATH,
            entries
                .get(MANIFEST_PATH)
                .ok_or(QtiImportError::MissingManifest)?,
            self.limits,
        )?;
        let mut unsupported_features = unsupported_manifest_resources(&manifest);
        let mut questions = Vec::new();
        let mut grading = BTreeMap::new();
        let mut assets = BTreeMap::new();
        let mut referenced = BTreeSet::new();
        for resource in &manifest.resources {
            let Some(href) = resource.href.as_deref() else {
                if is_qti_item(resource) {
                    unsupported_features.push(unsupported(
                        &resource.identifier,
                        "missing-resource-href",
                        "QTI item resource has no package entry point",
                    ));
                }
                continue;
            };
            validate_relative_reference(href).map_err(|reason| QtiImportError::InvalidXml {
                path: MANIFEST_PATH.into(),
                reason,
            })?;
            referenced.insert(href.to_string());
            let item = entries
                .get(href)
                .ok_or_else(|| QtiImportError::MissingReferencedEntry { path: href.into() })?;
            if !is_qti_item(resource) {
                continue;
            }
            let node = parse_xml(href, item, self.limits)?;
            match parse_single_choice_item(href, &node, &entries, &mut assets) {
                Ok((question, correct)) => {
                    grading.insert(question.item_id.clone(), correct);
                    questions.push(question);
                }
                Err(feature) => unsupported_features.push(feature),
            }
        }
        for path in entries
            .keys()
            .filter(|p| p.as_str() != MANIFEST_PATH && !referenced.contains(*p))
        {
            if !is_allowed_unreferenced_asset(path) {
                return Err(QtiImportError::UnsafeEntry {
                    path: path.clone(),
                    reason: "unexpected unreferenced package entry".into(),
                });
            }
        }
        Ok(ImportedQtiPackage {
            original: ArchivedQtiPackage {
                bytes: bytes.to_vec(),
                sha256: Sha256Digest::compute(bytes).to_string(),
                size_bytes: u64::try_from(bytes.len()).map_err(|_| {
                    QtiImportError::InvalidArchive("archive length overflow".into())
                })?,
            },
            manifest,
            questions,
            assets: assets.into_values().collect(),
            unsupported: unsupported_features,
            grading: QtiGradingHandoff {
                choices_by_item: grading,
            },
        })
    }
}
impl Default for QtiImporter {
    fn default() -> Self {
        Self::new(QtiImportLimits::default())
    }
}

fn validate_entry_path(
    path: &str,
    entry: &zip::read::ZipFile<'_, Cursor<&[u8]>>,
) -> Result<(), QtiImportError> {
    if path.is_empty()
        || path.starts_with('/')
        || path.starts_with('\\')
        || path.contains('\\')
        || path.contains('\0')
        || path
            .split('/')
            .any(|p| p.is_empty() || p == "." || p == "..")
    {
        return Err(QtiImportError::UnsafeEntry {
            path: path.into(),
            reason: "path must be a nonempty relative slash-separated path".into(),
        });
    }
    if entry.enclosed_name().is_none() {
        return Err(QtiImportError::UnsafeEntry {
            path: path.into(),
            reason: "path escapes extraction root".into(),
        });
    }
    if entry
        .unix_mode()
        .is_some_and(|mode| mode & 0o170000 == 0o120000)
    {
        return Err(QtiImportError::UnsafeEntry {
            path: path.into(),
            reason: "symbolic links are not accepted".into(),
        });
    }
    if !entry.is_dir() && !is_allowed_entry(path) {
        return Err(QtiImportError::UnsafeEntry {
            path: path.into(),
            reason: "entry is outside supported manifest/items/assets layout".into(),
        });
    }
    Ok(())
}
fn is_allowed_entry(path: &str) -> bool {
    path == MANIFEST_PATH || path.starts_with("items/") || path.starts_with("assets/")
}
fn is_allowed_unreferenced_asset(path: &str) -> bool {
    path.starts_with("assets/")
}

#[derive(Debug, Clone)]
struct XmlNode {
    name: String,
    attrs: BTreeMap<String, String>,
    children: Vec<XmlNode>,
    text: String,
}
#[derive(Debug)]
struct NodeStart {
    name: String,
    attrs: BTreeMap<String, String>,
}

/// Parses XML through `xmlparser`, refusing every DTD/entity token and
/// validating balanced nesting ourselves (a documented tokenizer limitation).
fn parse_xml(path: &str, bytes: &[u8], limits: QtiImportLimits) -> Result<XmlNode, QtiImportError> {
    let text = std::str::from_utf8(bytes).map_err(|_| invalid_xml(path, "XML is not UTF-8"))?;
    let mut roots = Vec::new();
    let mut stack: Vec<XmlNode> = Vec::new();
    let mut pending: Option<NodeStart> = None;
    let mut token_count = 0_usize;
    let mut node_count = 0_usize;
    for token in Tokenizer::from(text) {
        token_count = token_count.saturating_add(1);
        if token_count > limits.max_xml_tokens {
            return Err(xml_resource_limit(
                path,
                "token count",
                token_count,
                limits.max_xml_tokens,
            ));
        }
        let token =
            token.map_err(|e| invalid_xml(path, &format!("XML parser rejected document: {e}")))?;
        match token {
            Token::DtdStart { .. }
            | Token::EmptyDtd { .. }
            | Token::EntityDeclaration { .. }
            | Token::DtdEnd { .. } => {
                return Err(invalid_xml(
                    path,
                    "DOCTYPE and entity declarations are forbidden",
                ));
            }
            Token::ElementStart { local, .. } => {
                if pending.is_some() {
                    return Err(invalid_xml(path, "element started before prior tag closed"));
                }
                pending = Some(NodeStart {
                    name: local.as_str().to_string(),
                    attrs: BTreeMap::new(),
                });
            }
            Token::Attribute { local, value, .. } => {
                let Some(start) = pending.as_mut() else {
                    return Err(invalid_xml(path, "attribute outside element start"));
                };
                if start
                    .attrs
                    .insert(local.as_str().to_string(), xml_unescape(value.as_str()))
                    .is_some()
                {
                    return Err(invalid_xml(path, "duplicate attribute"));
                }
            }
            Token::ElementEnd {
                end: ElementEnd::Open,
                ..
            } => {
                node_count = node_count.saturating_add(1);
                ensure_xml_node_limits(path, &stack, node_count, limits)?;
                stack.push(node_from_pending(path, &mut pending)?);
            }
            Token::ElementEnd {
                end: ElementEnd::Empty,
                ..
            } => {
                node_count = node_count.saturating_add(1);
                ensure_xml_node_limits(path, &stack, node_count, limits)?;
                append_node(
                    path,
                    &mut stack,
                    &mut roots,
                    node_from_pending(path, &mut pending)?,
                )?
            }
            Token::ElementEnd {
                end: ElementEnd::Close(_, local),
                ..
            } => {
                if pending.is_some() {
                    return Err(invalid_xml(path, "closing element before prior tag closed"));
                }
                let node = stack
                    .pop()
                    .ok_or_else(|| invalid_xml(path, "closing element without open element"))?;
                if node.name != local.as_str() {
                    return Err(invalid_xml(path, "mismatched closing element"));
                }
                append_node(path, &mut stack, &mut roots, node)?;
            }
            Token::Text { text } | Token::Cdata { text, .. } => {
                if let Some(node) = stack.last_mut() {
                    node.text.push_str(&xml_unescape(text.as_str()));
                } else if !text.as_str().trim().is_empty() {
                    return Err(invalid_xml(path, "text outside root element"));
                }
            }
            Token::Declaration { .. }
            | Token::ProcessingInstruction { .. }
            | Token::Comment { .. } => {}
        }
    }
    if pending.is_some() || !stack.is_empty() || roots.len() != 1 {
        return Err(invalid_xml(
            path,
            "document must have one balanced root element",
        ));
    }
    Ok(roots.remove(0))
}
fn invalid_xml(path: &str, reason: &str) -> QtiImportError {
    QtiImportError::InvalidXml {
        path: path.into(),
        reason: reason.into(),
    }
}
fn xml_resource_limit(path: &str, resource: &str, actual: usize, limit: usize) -> QtiImportError {
    invalid_xml(
        path,
        &format!("XML resource limit exceeded: {resource} {actual} exceeds limit {limit}"),
    )
}
fn ensure_xml_node_limits(
    path: &str,
    stack: &[XmlNode],
    node_count: usize,
    limits: QtiImportLimits,
) -> Result<(), QtiImportError> {
    let depth = stack.len().saturating_add(1);
    if depth > limits.max_xml_depth {
        return Err(xml_resource_limit(
            path,
            "element depth",
            depth,
            limits.max_xml_depth,
        ));
    }
    if node_count > limits.max_xml_nodes {
        return Err(xml_resource_limit(
            path,
            "element node count",
            node_count,
            limits.max_xml_nodes,
        ));
    }
    Ok(())
}
fn node_from_pending(
    path: &str,
    pending: &mut Option<NodeStart>,
) -> Result<XmlNode, QtiImportError> {
    let start = pending
        .take()
        .ok_or_else(|| invalid_xml(path, "element end without start"))?;
    Ok(XmlNode {
        name: start.name,
        attrs: start.attrs,
        children: vec![],
        text: String::new(),
    })
}
fn append_node(
    path: &str,
    stack: &mut [XmlNode],
    roots: &mut Vec<XmlNode>,
    node: XmlNode,
) -> Result<(), QtiImportError> {
    if let Some(parent) = stack.last_mut() {
        parent.children.push(node);
    } else {
        roots.push(node);
        if roots.len() > 1 {
            return Err(invalid_xml(path, "document has multiple root elements"));
        }
    }
    Ok(())
}

fn parse_manifest(
    path: &str,
    bytes: &[u8],
    limits: QtiImportLimits,
) -> Result<QtiManifest, QtiImportError> {
    let root = parse_xml(path, bytes, limits)?;
    if root.name != "manifest" {
        return Err(invalid_xml(path, "root element is not manifest"));
    }
    let mut resources = Vec::new();
    collect_resources(&root, &mut resources)?;
    Ok(QtiManifest {
        identifier: root.attrs.get("identifier").cloned(),
        resources,
    })
}
fn collect_resources(
    node: &XmlNode,
    resources: &mut Vec<QtiResource>,
) -> Result<(), QtiImportError> {
    let mut pending = vec![node];
    while let Some(node) = pending.pop() {
        if node.name != "resource" {
            pending.extend(node.children.iter().rev());
            continue;
        }
        resources.push(QtiResource {
            identifier: node
                .attrs
                .get("identifier")
                .filter(|v| !v.is_empty())
                .cloned()
                .ok_or_else(|| invalid_xml(MANIFEST_PATH, "resource has no identifier"))?,
            resource_type: node.attrs.get("type").cloned(),
            href: node.attrs.get("href").cloned(),
        });
    }
    Ok(())
}
fn unsupported_manifest_resources(manifest: &QtiManifest) -> Vec<UnsupportedFeature> {
    manifest
        .resources
        .iter()
        .filter(|r| !is_qti_item(r))
        .map(|r| {
            unsupported(
                r.href.as_deref().unwrap_or(&r.identifier),
                "manifest-resource-type",
                &format!(
                    "resource type {:?} is retained but not converted",
                    r.resource_type
                ),
            )
        })
        .collect()
}
fn is_qti_item(resource: &QtiResource) -> bool {
    resource
        .resource_type
        .as_deref()
        .is_some_and(|value| value.contains("qti_item"))
}

fn parse_single_choice_item(
    path: &str,
    root: &XmlNode,
    entries: &BTreeMap<String, Vec<u8>>,
    assets: &mut BTreeMap<String, QtiAssetObject>,
) -> Result<(ImportedQtiQuestion, ChoiceId), UnsupportedFeature> {
    if root.name != "assessmentItem" {
        return Err(unsupported(
            path,
            "unsupported-item-root",
            "only QTI assessmentItem resources are supported",
        ));
    }
    if contains_name(root, "math") || contains_name(root, "table") {
        return Err(unsupported(
            path,
            "unsupported-item-markup",
            "MathML and tables are retained unchanged until their internal conversion is supported",
        ));
    }
    let item_id = required_attr(root, "identifier").ok_or_else(|| {
        unsupported(
            path,
            "missing-item-identifier",
            "assessmentItem needs an identifier",
        )
    })?;
    let interactions = descendants(root, "choiceInteraction");
    if interactions.len() != 1 {
        return Err(unsupported(
            path,
            "unsupported-interaction",
            "exactly one choiceInteraction is required",
        ));
    }
    let interaction = interactions[0];
    if interaction.attrs.get("maxChoices").map(String::as_str) != Some("1") {
        return Err(unsupported(
            path,
            "multiple-choice-cardinality",
            "only single-choice QTI interactions are converted",
        ));
    }
    let response_identifier =
        required_attr(interaction, "responseIdentifier").ok_or_else(|| {
            unsupported(
                path,
                "missing-response-identifier",
                "choiceInteraction needs responseIdentifier",
            )
        })?;
    let correct = correct_response(root, &response_identifier).ok_or_else(|| {
        unsupported(
            path,
            "missing-correct-response",
            "single-choice item needs one correctResponse value",
        )
    })?;
    let item_body = descendants(root, "itemBody")
        .into_iter()
        .next()
        .ok_or_else(|| unsupported(path, "missing-prompt", "assessmentItem needs itemBody"))?;
    let mut prompt_nodes: Vec<&XmlNode> = item_body
        .children
        .iter()
        .filter(|n| n.name != "choiceInteraction")
        .collect();
    if prompt_nodes.is_empty() {
        prompt_nodes.push(item_body);
    }
    let prompt = content_blocks(path, &prompt_nodes, entries, assets)?;
    if prompt.is_empty() {
        return Err(unsupported(
            path,
            "missing-prompt",
            "itemBody needs visible prompt content",
        ));
    }
    let mut choices = Vec::new();
    for choice in descendants(interaction, "simpleChoice") {
        let id = required_attr(choice, "identifier").ok_or_else(|| {
            unsupported(
                path,
                "missing-choice-identifier",
                "simpleChoice needs an identifier",
            )
        })?;
        let body = content_blocks(path, &[choice], entries, assets)?;
        if body.is_empty() {
            return Err(unsupported(
                path,
                "invalid-choice-set",
                "simpleChoice cannot be empty",
            ));
        }
        choices.push(ChoiceOption {
            id: ChoiceId::new(id),
            body,
        });
    }
    if choices.len() < 2 || !choices.iter().any(|c| c.id.as_str() == correct) {
        return Err(unsupported(
            path,
            "invalid-choice-set",
            "single-choice item needs two choices including its declared answer",
        ));
    }
    Ok((
        ImportedQtiQuestion {
            item_id,
            prompt,
            response: ResponseDefinition::MultipleChoice {
                choices,
                selection: SelectionCardinality::ExactlyOne,
            },
        },
        ChoiceId::new(correct),
    ))
}
fn required_attr(node: &XmlNode, name: &str) -> Option<String> {
    node.attrs.get(name).filter(|v| !v.is_empty()).cloned()
}
fn descendants<'a>(node: &'a XmlNode, name: &str) -> Vec<&'a XmlNode> {
    let mut found = Vec::new();
    let mut pending = vec![node];
    while let Some(current) = pending.pop() {
        if current.name == name {
            found.push(current);
        }
        pending.extend(current.children.iter().rev());
    }
    found
}
fn contains_name(node: &XmlNode, name: &str) -> bool {
    let mut pending = vec![node];
    while let Some(current) = pending.pop() {
        if current.name == name {
            return true;
        }
        pending.extend(current.children.iter().rev());
    }
    false
}
fn correct_response(root: &XmlNode, response_identifier: &str) -> Option<String> {
    descendants(root, "responseDeclaration")
        .into_iter()
        .find(|n| n.attrs.get("identifier").map(String::as_str) == Some(response_identifier))
        .and_then(|n| descendants(n, "value").into_iter().next())
        .map(node_text)
        .filter(|v| !v.is_empty())
}

fn content_blocks(
    item_path: &str,
    nodes: &[&XmlNode],
    entries: &BTreeMap<String, Vec<u8>>,
    assets: &mut BTreeMap<String, QtiAssetObject>,
) -> Result<Vec<ContentBlock>, UnsupportedFeature> {
    let mut blocks = Vec::new();
    for node in nodes {
        if node.name == "object" || node.name == "audio" || node.name == "video" {
            return Err(unsupported(
                item_path,
                "unsupported-media-kind",
                "only images can be represented by the current internal response model",
            ));
        }
        if node.name == "img" {
            blocks.push(image_block(item_path, node, entries, assets)?);
            continue;
        }
        let text = node_text(node);
        if !text.is_empty() {
            blocks.push(ContentBlock::Text { markdown: text });
        }
        for image in descendants(node, "img") {
            blocks.push(image_block(item_path, image, entries, assets)?);
        }
        if descendants(node, "object").len()
            + descendants(node, "audio").len()
            + descendants(node, "video").len()
            > 0
        {
            return Err(unsupported(
                item_path,
                "unsupported-media-kind",
                "only images can be represented by the current internal response model",
            ));
        }
    }
    Ok(blocks)
}
fn image_block(
    item_path: &str,
    image: &XmlNode,
    entries: &BTreeMap<String, Vec<u8>>,
    assets: &mut BTreeMap<String, QtiAssetObject>,
) -> Result<ContentBlock, UnsupportedFeature> {
    let raw = image
        .attrs
        .get("src")
        .or_else(|| image.attrs.get("data"))
        .ok_or_else(|| {
            unsupported(
                item_path,
                "missing-media-reference",
                "img needs src or data attribute",
            )
        })?;
    let path = resolve_asset_path(item_path, raw)
        .map_err(|detail| unsupported(item_path, "unsafe-media-reference", &detail))?;
    let bytes = entries.get(&path).ok_or_else(|| {
        unsupported(
            item_path,
            "missing-media-entry",
            &format!("referenced asset `{path}` is absent"),
        )
    })?;
    let media_type = sniff_media_type(bytes).ok_or_else(|| {
        unsupported(
            &path,
            "unsupported-media-type",
            "media bytes do not match a supported image signature",
        )
    })?;
    let asset = assets
        .entry(path.clone())
        .or_insert_with(|| asset_object(path.clone(), bytes.clone(), media_type.to_string()));
    Ok(ContentBlock::Image {
        asset: AssetRef {
            asset: asset.asset,
            checksum: asset.sha256.clone(),
        },
        description: image
            .attrs
            .get("alt")
            .filter(|v| !v.is_empty())
            .cloned()
            .unwrap_or_else(|| "Image supplied by imported QTI package".into()),
    })
}
fn asset_object(source_path: String, bytes: Vec<u8>, media_type: String) -> QtiAssetObject {
    let digest = Sha256Digest::compute(&bytes);
    let mut raw = [0_u8; 16];
    raw.copy_from_slice(&digest.as_bytes()[..16]);
    raw[6] = (raw[6] & 0x0f) | 0x40;
    raw[8] = (raw[8] & 0x3f) | 0x80;
    QtiAssetObject {
        asset: AssetId::from_uuid(Uuid::from_bytes(raw)),
        source_path,
        sha256: digest.to_string(),
        media_type,
        bytes,
    }
}
fn resolve_asset_path(item_path: &str, raw: &str) -> Result<String, String> {
    if raw.contains("://")
        || raw.starts_with('/')
        || raw.starts_with('\\')
        || raw.contains('\\')
        || raw.contains('\0')
    {
        return Err("media reference must be a package-relative path".into());
    }
    let mut parts: Vec<&str> = if raw.starts_with("assets/") {
        Vec::new()
    } else {
        item_path
            .rsplit_once('/')
            .map_or_else(Vec::new, |(base, _)| base.split('/').collect())
    };
    for part in raw.split('/') {
        match part {
            "" | "." => return Err("media reference has an empty or ambiguous component".into()),
            ".." => {
                if parts.pop().is_none() {
                    return Err("media reference escapes the package root".into());
                }
            }
            component => parts.push(component),
        }
    }
    let joined = parts.join("/");
    validate_relative_reference(&joined)?;
    if !joined.starts_with("assets/") {
        return Err("media reference must resolve under assets/".into());
    }
    Ok(joined)
}
fn validate_relative_reference(path: &str) -> Result<(), String> {
    if path.is_empty()
        || path.starts_with('/')
        || path.contains('\\')
        || path.contains('\0')
        || path
            .split('/')
            .any(|p| p.is_empty() || p == "." || p == "..")
    {
        Err("reference must be a nonempty relative slash-separated path without traversal".into())
    } else {
        Ok(())
    }
}
fn sniff_media_type(bytes: &[u8]) -> Option<&'static str> {
    if bytes.starts_with(b"\x89PNG\r\n\x1a\n") {
        Some("image/png")
    } else if bytes.starts_with(&[0xff, 0xd8, 0xff]) {
        Some("image/jpeg")
    } else if bytes.starts_with(b"GIF87a") || bytes.starts_with(b"GIF89a") {
        Some("image/gif")
    } else if bytes.starts_with(b"RIFF") && bytes.get(8..12) == Some(b"WEBP") {
        Some("image/webp")
    } else {
        let trimmed = std::str::from_utf8(bytes).ok()?.trim_start();
        (trimmed.starts_with("<svg") || trimmed.starts_with("<?xml") && trimmed.contains("<svg"))
            .then_some("image/svg+xml")
    }
}
fn node_text(node: &XmlNode) -> String {
    let mut pieces = Vec::new();
    let mut pending = vec![node];
    while let Some(current) = pending.pop() {
        if current.name != "img" {
            pieces.push(current.text.as_str());
            pending.extend(current.children.iter().rev());
        }
    }
    pieces
        .join(" ")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}
fn xml_unescape(value: &str) -> String {
    value
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&amp;", "&")
        .replace("&quot;", "\"")
        .replace("&apos;", "'")
}
fn unsupported(path: &str, feature: &str, detail: &str) -> UnsupportedFeature {
    UnsupportedFeature {
        location: path.into(),
        feature: feature.into(),
        detail: detail.into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use base64::Engine;
    const VALID_PACKAGE: &str =
        include_str!("../../../../tests/fixtures/qti/valid-single-choice.zip.b64");
    const TRAVERSAL_PACKAGE: &str =
        include_str!("../../../../tests/fixtures/qti/path-traversal.zip.b64");
    const ABSOLUTE_PACKAGE: &str =
        include_str!("../../../../tests/fixtures/qti/absolute-path.zip.b64");
    const SYMLINK_PACKAGE: &str = include_str!("../../../../tests/fixtures/qti/symlink.zip.b64");
    const UNEXPECTED_PACKAGE: &str =
        include_str!("../../../../tests/fixtures/qti/unexpected-entry.zip.b64");
    fn fixture(s: &str) -> Vec<u8> {
        base64::engine::general_purpose::STANDARD
            .decode(s.trim())
            .expect("base64")
    }
    #[test]
    fn imports_supported_single_choice_with_no_debuggable_answer_or_archive() {
        let imported = QtiImporter::default()
            .import(&fixture(VALID_PACKAGE))
            .expect("valid");
        assert_eq!(imported.questions.len(), 1);
        let debug = format!("{imported:?}");
        assert!(
            !debug.contains("ChoiceId")
                && !debug.contains("correctResponse")
                && !debug.contains("PK\\x03\\x04")
        );
        assert!(debug.contains("<server-only>"));
        assert_eq!(imported.worker_original_bytes(), fixture(VALID_PACKAGE));
        let item_id = &imported.questions[0].item_id;
        assert_eq!(
            imported.worker_correct_choice(item_id),
            Some(ChoiceId::new("b"))
        );
    }
    #[test]
    fn hostile_zip_corpus_is_rejected() {
        for (name, package) in [
            ("traversal", TRAVERSAL_PACKAGE),
            ("absolute", ABSOLUTE_PACKAGE),
            ("symlink", SYMLINK_PACKAGE),
            ("unexpected", UNEXPECTED_PACKAGE),
        ] {
            let error = QtiImporter::default()
                .import(&fixture(package))
                .expect_err(name);
            assert!(
                matches!(error, QtiImportError::UnsafeEntry { .. }),
                "{name}: {error}"
            );
        }
    }
    #[test]
    fn xml_parser_rejects_dtd_malformed_comments_and_cdata_deception() {
        for item in [
            "<!DOCTYPE assessmentItem><assessmentItem/>",
            "<assessmentItem><itemBody></assessmentItem>",
            "<!-- <choiceInteraction maxChoices='1'/> --><assessmentItem identifier='x'><itemBody><![CDATA[<choiceInteraction/>]]></itemBody></assessmentItem>",
        ] {
            let bytes = package(&[
                (MANIFEST_PATH, manifest("items/item.xml")),
                ("items/item.xml", item.into()),
            ]);
            let result = QtiImporter::default().import(&bytes);
            assert!(result.is_err() || result.expect("parsed package").questions.is_empty());
        }
    }
    #[test]
    fn xml_parser_refuses_deep_and_wide_documents_at_resource_limits() {
        let deeply_nested = format!(
            "<assessmentItem identifier='x'>{}</assessmentItem>",
            "<outer>".repeat(8) + &"</outer>".repeat(8)
        );
        let deep_package = package(&[
            (MANIFEST_PATH, manifest("items/item.xml")),
            ("items/item.xml", deeply_nested),
        ]);
        let deep_limits = QtiImportLimits {
            max_xml_depth: 5,
            ..QtiImportLimits::default()
        };
        let deep_error = QtiImporter::new(deep_limits)
            .import(&deep_package)
            .expect_err("deep XML must be refused");
        assert!(matches!(deep_error, QtiImportError::InvalidXml { .. }));
        assert!(
            deep_error
                .to_string()
                .contains("XML resource limit exceeded: element depth")
        );

        let wide_item = format!(
            "<assessmentItem identifier='x'>{}</assessmentItem>",
            "<p>one</p>".repeat(12)
        );
        let wide_package = package(&[
            (MANIFEST_PATH, manifest("items/item.xml")),
            ("items/item.xml", wide_item),
        ]);
        let wide_limits = QtiImportLimits {
            max_xml_nodes: 8,
            ..QtiImportLimits::default()
        };
        let wide_error = QtiImporter::new(wide_limits)
            .import(&wide_package)
            .expect_err("wide XML must be refused");
        assert!(matches!(wide_error, QtiImportError::InvalidXml { .. }));
        assert!(
            wide_error
                .to_string()
                .contains("XML resource limit exceeded: element node count")
        );

        let token_limits = QtiImportLimits {
            max_xml_tokens: 5,
            ..QtiImportLimits::default()
        };
        let token_error = QtiImporter::new(token_limits)
            .import(&wide_package)
            .expect_err("token-heavy XML must be refused");
        assert!(matches!(token_error, QtiImportError::InvalidXml { .. }));
        assert!(
            token_error
                .to_string()
                .contains("XML resource limit exceeded: token count")
        );
    }
    #[test]
    fn extracts_sniffed_image_to_worker_manifest_and_rewrites_prompt() {
        let png = vec![0x89, b'P', b'N', b'G', b'\r', b'\n', 0x1a, b'\n'];
        let item = "<assessmentItem identifier='choice'><responseDeclaration identifier='R'><correctResponse><value>b</value></correctResponse></responseDeclaration><itemBody><p>Look <img src='../assets/p.png' alt='plot'/></p><choiceInteraction responseIdentifier='R' maxChoices='1'><simpleChoice identifier='a'>A</simpleChoice><simpleChoice identifier='b'>B</simpleChoice></choiceInteraction></itemBody></assessmentItem>";
        let bytes = package_bytes(&[
            (MANIFEST_PATH, manifest("items/item.xml").into_bytes()),
            ("items/item.xml", item.as_bytes().to_vec()),
            ("assets/p.png", png.clone()),
        ]);
        let imported = QtiImporter::default()
            .import(&bytes)
            .expect("image imports");
        assert_eq!(
            imported.assets.len(),
            1,
            "unsupported: {:?}",
            imported.unsupported
        );
        assert_eq!(imported.assets[0].worker_bytes(), png.as_slice());
        assert_eq!(imported.assets[0].media_type, "image/png");
        assert!(!format!("{:?}", imported.assets[0]).contains("assets/p.png"));
        assert!(matches!(
            imported.questions[0].prompt.last(),
            Some(ContentBlock::Image { .. })
        ));
    }
    #[test]
    fn asset_collector_includes_images_in_choice_bodies() {
        let item = "<assessmentItem identifier='choice'><responseDeclaration identifier='R'><correctResponse><value>b</value></correctResponse></responseDeclaration><itemBody><p>Choose one.</p><choiceInteraction responseIdentifier='R' maxChoices='1'><simpleChoice identifier='a'>A</simpleChoice><simpleChoice identifier='b'><img src='../assets/choice.png' alt='choice diagram'/>B</simpleChoice></choiceInteraction></itemBody></assessmentItem>";
        let archive = package_bytes(&[
            (MANIFEST_PATH, manifest("items/choice.xml").into_bytes()),
            ("items/choice.xml", item.as_bytes().to_vec()),
            ("assets/choice.png", b"\x89PNG\r\n\x1a\nfixture".to_vec()),
        ]);
        let imported = QtiImporter::default()
            .import(&archive)
            .expect("choice image parses");
        let question = imported.questions.first().expect("one imported question");
        assert_eq!(
            qti_question_asset_checksums(question)
                .expect("one checksum per logical image")
                .len(),
            1
        );
        assert_eq!(question.prompt.len(), 1, "image is not in the prompt");
    }
    #[test]
    fn import_handoff_keeps_archive_assets_and_grading_server_only() {
        let imported = QtiImporter::default()
            .import(&fixture(VALID_PACKAGE))
            .expect("valid");
        assert_eq!(
            imported.worker_original_size_bytes() as usize,
            fixture(VALID_PACKAGE).len()
        );
        assert_eq!(imported.worker_original_sha256().len(), 64);
        assert!(imported.worker_assets().is_empty());
        let item_id = &imported.questions[0].item_id;
        assert_eq!(
            imported.worker_correct_choice(item_id),
            Some(ChoiceId::new("b"))
        );
    }
    fn manifest(choice: &str) -> String {
        format!(
            "<manifest identifier='package'><resources><resource identifier='choice' type='imsqti_item_xmlv2p1' href='{choice}'/></resources></manifest>"
        )
    }
    fn package(files: &[(&str, String)]) -> Vec<u8> {
        package_bytes(
            &files
                .iter()
                .map(|(p, b)| (*p, b.as_bytes().to_vec()))
                .collect::<Vec<_>>(),
        )
    }
    fn package_bytes(files: &[(&str, Vec<u8>)]) -> Vec<u8> {
        let mut writer = zip::ZipWriter::new(Cursor::new(Vec::new()));
        for (path, contents) in files {
            writer
                .start_file(*path, zip::write::SimpleFileOptions::default())
                .expect("start");
            std::io::Write::write_all(&mut writer, contents).expect("write");
        }
        writer.finish().expect("finish").into_inner()
    }
}

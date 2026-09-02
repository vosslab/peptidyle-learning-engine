//! Bounded QTI ZIP import and archival export.
//!
//! The original ZIP is authoritative and is never reconstructed.  Conversion
//! accepts a deliberately small QTI subset using an XML event parser: DTDs,
//! entity declarations, malformed nesting, and duplicate attributes fail
//! before any model is made.  Asset bytes leave this module only in an
//! immutable worker handoff; student-visible questions contain `QuestionAssetId`s and
//! checksums, never archive paths or an Answer Key.

use std::collections::{BTreeMap, BTreeSet};

use crate::profiles::NormalizedQtiItemFingerprint;
use objects::Sha256Checksum;
use objects::image_validation::verify_still_image;
use question_model::answer::ResponseSelectionRule;
use question_model::response::{QuestionChoice, ResponseItemReference};
use question_model::{QuestionAssetId, QuestionResponseFormat};
use question_model::{QuestionAssetReference, QuestionContentBlock};
use uuid::Uuid;

const MANIFEST_PATH: &str = "imsmanifest.xml";

use crate::archive::{BoundedArchiveEntries, read_bounded_archive, validate_relative_reference};
use crate::model::{ArchivedQtiPackage, QtiGradingHandoff};
pub use crate::model::{
    ImportedQtiPackage, ImportedQtiQuestion, QtiAssetObject, QtiAssetReferenceError,
    QtiImportError, QtiImportLimits, QtiItemImportResult, QtiItemImportStatus, QtiManifest,
    QtiResource, UnsupportedFeature, qti_question_asset_checksums,
};
use crate::xml::{XmlNode, parse_xml};

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
        let entries = read_bounded_archive(bytes, self.limits, is_allowed_entry)?;
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
        let mut item_results = Vec::new();
        let mut item_ids = BTreeSet::new();
        let mut exact_digests = BTreeMap::<String, String>::new();
        let mut presentation_digests = BTreeMap::<String, (String, String)>::new();
        for resource in &manifest.resources {
            let qti_item = is_qti_item(resource);
            let Some(href) = resource.href.as_deref() else {
                if qti_item {
                    let warning = unsupported(
                        &resource.identifier,
                        "missing-resource-href",
                        "QTI item resource has no package entry point",
                    );
                    unsupported_features.push(warning.clone());
                    item_results.push(QtiItemImportResult {
                        source_identifier: resource.identifier.clone(),
                        item_id: None,
                        normalized_qti_item_fingerprint: None,
                        status: QtiItemImportStatus::Rejected,
                        warnings: vec![warning],
                    });
                }
                continue;
            };
            validate_relative_reference(href).map_err(|reason| QtiImportError::InvalidXml {
                path: MANIFEST_PATH.into(),
                reason,
            })?;
            referenced.insert(href.to_string());
            let Some(item) = entries.get(href) else {
                if qti_item {
                    let warning = unsupported(
                        &resource.identifier,
                        "missing-referenced-entry",
                        "QTI item resource references a package entry that is absent",
                    );
                    unsupported_features.push(warning.clone());
                    item_results.push(QtiItemImportResult {
                        source_identifier: resource.identifier.clone(),
                        item_id: None,
                        normalized_qti_item_fingerprint: None,
                        status: QtiItemImportStatus::Rejected,
                        warnings: vec![warning],
                    });
                    continue;
                }
                return Err(QtiImportError::MissingReferencedEntry { path: href.into() });
            };
            if !qti_item {
                continue;
            }
            let node = parse_xml(href, item, self.limits)?;
            if let Some(item_id) = required_attr(&node, "identifier")
                && item_ids.contains(&item_id)
            {
                let warning = unsupported(
                    &resource.identifier,
                    "duplicate-item-identifier",
                    "another QTI item in this package already uses this item identifier",
                );
                unsupported_features.push(warning.clone());
                item_results.push(QtiItemImportResult {
                    source_identifier: resource.identifier.clone(),
                    item_id: Some(item_id),
                    normalized_qti_item_fingerprint: None,
                    status: QtiItemImportStatus::Rejected,
                    warnings: vec![warning],
                });
                continue;
            }
            match parse_single_choice_item(href, &node, &entries, &mut assets) {
                Ok((question, correct)) => {
                    let (normalized, presentation) =
                        normalized_item_fingerprints(href, &question, &correct)?;
                    let normalized_text = normalized.to_string();
                    let presentation_text = presentation.to_string();
                    let mut warnings = Vec::new();
                    if let Some(first) = exact_digests.get(&normalized_text) {
                        warnings.push(unsupported(
                            &resource.identifier,
                            "exact-duplicate-item",
                            &format!(
                                "normalized question and grading content duplicates resource `{first}`"
                            ),
                        ));
                    } else if let Some((first_normalized, first)) =
                        presentation_digests.get(&presentation_text)
                        && first_normalized != &normalized_text
                    {
                        warnings.push(unsupported(
                            &resource.identifier,
                            "likely-duplicate-item",
                            &format!(
                                "normalized presentation matches resource `{first}` but grading differs"
                            ),
                        ));
                    }
                    exact_digests
                        .entry(normalized_text.clone())
                        .or_insert_with(|| resource.identifier.clone());
                    presentation_digests
                        .entry(presentation_text)
                        .or_insert_with(|| (normalized_text.clone(), resource.identifier.clone()));
                    item_ids.insert(question.item_id.clone());
                    unsupported_features.extend(warnings.iter().cloned());
                    item_results.push(QtiItemImportResult {
                        source_identifier: resource.identifier.clone(),
                        item_id: Some(question.item_id.clone()),
                        normalized_qti_item_fingerprint: Some(normalized),
                        status: QtiItemImportStatus::Accepted,
                        warnings,
                    });
                    grading.insert(question.item_id.clone(), correct);
                    questions.push(question);
                }
                Err(feature) => {
                    unsupported_features.push(feature.clone());
                    item_results.push(QtiItemImportResult {
                        source_identifier: resource.identifier.clone(),
                        item_id: None,
                        normalized_qti_item_fingerprint: None,
                        status: QtiItemImportStatus::Rejected,
                        warnings: vec![feature],
                    });
                }
            }
        }
        for path in entries
            .paths()
            .filter(|path| *path != MANIFEST_PATH && !referenced.contains(*path))
        {
            if !is_allowed_unreferenced_asset(path) {
                return Err(QtiImportError::UnsafeEntry {
                    path: path.to_owned(),
                    reason: "unexpected unreferenced package entry".into(),
                });
            }
        }
        Ok(ImportedQtiPackage {
            original: ArchivedQtiPackage {
                bytes: bytes.to_vec(),
                package_checksum: Sha256Checksum::compute(bytes).to_string(),
                size_bytes: u64::try_from(bytes.len()).map_err(|_| {
                    QtiImportError::InvalidArchive("archive length overflow".into())
                })?,
            },
            manifest,
            questions,
            assets: assets.into_values().collect(),
            unsupported: unsupported_features,
            item_results,
            grading: QtiGradingHandoff {
                choices_by_item: grading,
            },
        })
    }
}

fn normalized_item_fingerprints(
    path: &str,
    question: &ImportedQtiQuestion,
    correct: &ResponseItemReference,
) -> Result<(NormalizedQtiItemFingerprint, Sha256Checksum), QtiImportError> {
    let presentation =
        serde_json::to_vec(&(&question.prompt, &question.response)).map_err(|_| {
            QtiImportError::InvalidXml {
                path: path.to_string(),
                reason: "normalized QTI presentation cannot be serialized".to_string(),
            }
        })?;
    let exact =
        serde_json::to_vec(&(&question.prompt, &question.response, correct)).map_err(|_| {
            QtiImportError::InvalidXml {
                path: path.to_string(),
                reason: "normalized QTI grading content cannot be serialized".to_string(),
            }
        })?;
    Ok((
        NormalizedQtiItemFingerprint::from_normalized_bytes(&exact),
        Sha256Checksum::compute(&presentation),
    ))
}

impl Default for QtiImporter {
    fn default() -> Self {
        Self::new(QtiImportLimits::default())
    }
}

fn is_allowed_entry(path: &str) -> bool {
    path == MANIFEST_PATH || path.starts_with("items/") || path.starts_with("assets/")
}
fn is_allowed_unreferenced_asset(path: &str) -> bool {
    path.starts_with("assets/")
}

fn invalid_xml(path: &str, reason: &str) -> QtiImportError {
    QtiImportError::InvalidXml {
        path: path.into(),
        reason: reason.into(),
    }
}
fn parse_manifest(
    path: &str,
    bytes: &[u8],
    limits: QtiImportLimits,
) -> Result<QtiManifest, QtiImportError> {
    let root = parse_xml(path, bytes, limits)?;
    if root.name() != "manifest" {
        return Err(invalid_xml(path, "root element is not manifest"));
    }
    let mut resources = Vec::new();
    collect_resources(&root, &mut resources)?;
    Ok(QtiManifest {
        identifier: root.attribute("identifier").map(str::to_owned),
        resources,
    })
}
fn collect_resources(
    node: &XmlNode,
    resources: &mut Vec<QtiResource>,
) -> Result<(), QtiImportError> {
    let mut pending = vec![node];
    while let Some(node) = pending.pop() {
        if node.name() != "resource" {
            pending.extend(node.children().iter().rev());
            continue;
        }
        resources.push(QtiResource {
            identifier: node
                .attribute("identifier")
                .filter(|value| !value.is_empty())
                .map(str::to_owned)
                .ok_or_else(|| invalid_xml(MANIFEST_PATH, "resource has no identifier"))?,
            resource_type: node.attribute("type").map(str::to_owned),
            href: node.attribute("href").map(str::to_owned),
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
    entries: &BoundedArchiveEntries,
    assets: &mut BTreeMap<String, QtiAssetObject>,
) -> Result<(ImportedQtiQuestion, ResponseItemReference), UnsupportedFeature> {
    if root.name() != "assessmentItem" {
        return Err(unsupported(
            path,
            "unsupported-item-root",
            "only QTI assessmentItem resources are supported",
        ));
    }
    if root.contains_named("math") || root.contains_named("table") {
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
    let interactions = root.descendants_named("choiceInteraction");
    if interactions.len() != 1 {
        return Err(unsupported(
            path,
            "unsupported-interaction",
            "exactly one choiceInteraction is required",
        ));
    }
    let interaction = interactions[0];
    if interaction.attribute("maxChoices") != Some("1") {
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
    let item_body = root
        .descendants_named("itemBody")
        .into_iter()
        .next()
        .ok_or_else(|| unsupported(path, "missing-prompt", "assessmentItem needs itemBody"))?;
    let mut prompt_nodes: Vec<&XmlNode> = item_body
        .children()
        .iter()
        .filter(|node| node.name() != "choiceInteraction")
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
    for choice in interaction.descendants_named("simpleChoice") {
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
        choices.push(QuestionChoice {
            id: ResponseItemReference::new(id),
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
            response: QuestionResponseFormat::MultipleChoice {
                choices,
                selection: ResponseSelectionRule::ExactlyOne,
            },
        },
        ResponseItemReference::new(correct),
    ))
}
fn required_attr(node: &XmlNode, name: &str) -> Option<String> {
    node.attribute(name)
        .filter(|value| !value.is_empty())
        .map(str::to_owned)
}
fn correct_response(root: &XmlNode, response_identifier: &str) -> Option<String> {
    root.descendants_named("responseDeclaration")
        .into_iter()
        .find(|node| node.attribute("identifier") == Some(response_identifier))
        .and_then(|node| node.descendants_named("value").into_iter().next())
        .map(|node| node.normalized_text_except("img"))
        .filter(|v| !v.is_empty())
}

fn content_blocks(
    item_path: &str,
    nodes: &[&XmlNode],
    entries: &BoundedArchiveEntries,
    assets: &mut BTreeMap<String, QtiAssetObject>,
) -> Result<Vec<QuestionContentBlock>, UnsupportedFeature> {
    let mut blocks = Vec::new();
    for node in nodes {
        if node.name() == "object" || node.name() == "audio" || node.name() == "video" {
            return Err(unsupported(
                item_path,
                "unsupported-media-kind",
                "only images can be represented by the current internal response model",
            ));
        }
        if node.name() == "img" {
            blocks.push(image_block(item_path, node, entries, assets)?);
            continue;
        }
        let text = node.normalized_text_except("img");
        if !text.is_empty() {
            blocks.push(QuestionContentBlock::Text { markdown: text });
        }
        for image in node.descendants_named("img") {
            blocks.push(image_block(item_path, image, entries, assets)?);
        }
        if node.descendants_named("object").len()
            + node.descendants_named("audio").len()
            + node.descendants_named("video").len()
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
    entries: &BoundedArchiveEntries,
    assets: &mut BTreeMap<String, QtiAssetObject>,
) -> Result<QuestionContentBlock, UnsupportedFeature> {
    let raw = image
        .attribute("src")
        .or_else(|| image.attribute("data"))
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
    let verified = verify_still_image(bytes)
        .map_err(|error| unsupported(&path, "unsafe-image", error.import_detail()))?;
    let media_type = verified.media_type.canonical_media_type();
    let asset = assets
        .entry(path.clone())
        .or_insert_with(|| asset_object(path.clone(), bytes.to_vec(), media_type.to_string()));
    Ok(QuestionContentBlock::Image {
        asset: QuestionAssetReference {
            asset: asset.asset,
            checksum: asset.sha256.clone(),
        },
        description: image
            .attribute("alt")
            .filter(|value| !value.is_empty())
            .map(str::to_owned)
            .unwrap_or_else(|| "Image supplied by imported QTI package".into()),
    })
}
fn asset_object(source_path: String, bytes: Vec<u8>, media_type: String) -> QtiAssetObject {
    let checksum = Sha256Checksum::compute(&bytes);
    let mut raw = [0_u8; 16];
    raw.copy_from_slice(&checksum.as_bytes()[..16]);
    raw[6] = (raw[6] & 0x0f) | 0x40;
    raw[8] = (raw[8] & 0x3f) | 0x80;
    QtiAssetObject {
        asset: QuestionAssetId::from_uuid(Uuid::from_bytes(raw)),
        source_path,
        sha256: checksum.to_string(),
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
fn unsupported(path: &str, feature: &str, detail: &str) -> UnsupportedFeature {
    UnsupportedFeature {
        location: path.into(),
        feature: feature.into(),
        detail: detail.into(),
    }
}

#[cfg(test)]
#[path = "parser/tests.rs"]
mod tests;

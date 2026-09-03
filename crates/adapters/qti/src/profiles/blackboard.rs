//! Strict Blackboard Original QTI 2.1 static-pool import boundary.

use std::collections::BTreeSet;
use std::fmt;

use super::markup::MarkupLimits;
use super::{
    QtiImportResultChecksumInput, QtiMappedItem, QtiMappedPoints, QtiProfileDetection,
    QtiProfileDetectionEvidence, QtiProfileDiagnosticCode, QtiProfileId, QtiProfileItemEvidence,
    QtiPublicChoiceChecksumInput, QtiSafeDiagnostic, QtiSafeDiagnosticLocation,
    QtiSafeDiagnosticTemplate, QtiSafeItemReport, map_qti_choice_ids,
};
use crate::archive::read_bounded_archive;
use crate::model::QtiImportLimits;
use crate::xml::{XmlNode, parse_xml};

mod shape;

const MANIFEST: &str = "imsmanifest.xml";
const META: &str = "qti21_items/assessment_meta.xml";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlackboardQtiImportError {
    Archive,
    Manifest,
    Detection(QtiProfileDiagnosticCode),
}

impl fmt::Display for BlackboardQtiImportError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("Blackboard QTI package does not meet the supported profile")
    }
}

impl std::error::Error for BlackboardQtiImportError {}

/// A package whose mapped items remain private because they bind correct answers.
///
/// ```compile_fail
/// use adapter_qti::profiles::BlackboardQtiPackage;
/// fn needs_debug<T: std::fmt::Debug>() {}
/// needs_debug::<BlackboardQtiPackage>();
/// ```
/// ```compile_fail
/// use adapter_qti::profiles::BlackboardQtiPackage;
/// fn needs_serialize<T: serde::Serialize>() {}
/// needs_serialize::<BlackboardQtiPackage>();
/// ```
#[derive(Clone, PartialEq)]
pub struct BlackboardQtiPackage {
    evidence: QtiProfileDetectionEvidence,
    reports: Vec<QtiSafeItemReport>,
    items: Vec<QtiMappedItem>,
}

impl BlackboardQtiPackage {
    pub fn detection_evidence(&self) -> &QtiProfileDetectionEvidence {
        &self.evidence
    }

    pub fn reports(&self) -> &[QtiSafeItemReport] {
        &self.reports
    }

    pub fn accepted_count(&self) -> usize {
        self.items.len()
    }

    /// Answer-free QTI Import Result Checksum input for this exact mapped package.
    pub fn qti_import_result_checksum_input(
        &self,
    ) -> Result<QtiImportResultChecksumInput, super::QtiProfileContractError> {
        super::checksums::package_import_result_checksum_input(
            QtiProfileId::BLACKBOARD,
            &self.evidence,
            &self.reports,
            &self.items,
        )
    }

    pub fn into_mapped_items(self) -> Vec<QtiMappedItem> {
        self.items
    }
}

/// Parses one bounded Blackboard Original QTI 2.1 pool without treating its
/// test metadata as PLE assignment policy.
pub fn import_blackboard_qti21(
    bytes: &[u8],
    limits: QtiImportLimits,
) -> Result<BlackboardQtiPackage, BlackboardQtiImportError> {
    let entries = read_bounded_archive(bytes, limits, shape::allowed)
        .map_err(|_| BlackboardQtiImportError::Archive)?;
    let manifest = parse_entry(&entries, MANIFEST, limits)?;
    let resources =
        shape::manifest_resources(&manifest).ok_or(BlackboardQtiImportError::Manifest)?;
    let meta = parse_entry(&entries, META, limits)?;
    if !shape::valid_meta(&meta, &resources) {
        return Err(BlackboardQtiImportError::Manifest);
    }

    let mut item_roots = Vec::new();
    for resource in &resources {
        if resource.resource_type.as_deref() == Some("imsqti_item_xmlv2p1") {
            let path = resource
                .href
                .as_deref()
                .ok_or(BlackboardQtiImportError::Manifest)?;
            let parsed = entries
                .get(path)
                .and_then(|body| parse_xml(path, body, limits).ok());
            item_roots.push((resource.identifier.clone(), path.to_string(), parsed));
        }
    }
    let evidence = QtiProfileDetectionEvidence {
        manifest_namespace: manifest.namespace_uri().unwrap_or_default().to_string(),
        manifest_schema: Some("QTIv2.1".to_string()),
        resources,
        items: item_roots
            .iter()
            .filter_map(|(identifier, path, root)| {
                root.as_ref().map(|root| QtiProfileItemEvidence {
                    resource_identifier: identifier.clone(),
                    path: path.clone(),
                    namespace: root.namespace_uri().unwrap_or_default().to_string(),
                    root: root.name().to_string(),
                })
            })
            .collect(),
    };
    match super::detect_qti_profile(&evidence) {
        QtiProfileDetection::Recognized(QtiProfileId::BLACKBOARD) => {}
        QtiProfileDetection::Rejected(code) => {
            return Err(BlackboardQtiImportError::Detection(code));
        }
        _ => {
            return Err(BlackboardQtiImportError::Detection(
                QtiProfileDiagnosticCode::ProfileAmbiguous,
            ));
        }
    }
    let referenced: BTreeSet<_> = evidence
        .resources
        .iter()
        .filter_map(|resource| resource.href.as_deref())
        .collect();
    if entries
        .paths()
        .any(|path| path != MANIFEST && !referenced.contains(path))
    {
        return Err(BlackboardQtiImportError::Detection(
            QtiProfileDiagnosticCode::UnexpectedEntry,
        ));
    }
    let mut ids = BTreeSet::new();
    if item_roots
        .iter()
        .filter_map(|(_, _, root)| root.as_ref())
        .any(|root| !ids.insert(root.attribute("identifier").unwrap_or_default().to_string()))
    {
        return Err(BlackboardQtiImportError::Detection(
            QtiProfileDiagnosticCode::ItemShape,
        ));
    }
    let mut reports = Vec::new();
    let mut items = Vec::new();
    for (ordinal, (resource_id, path, root)) in item_roots.iter().enumerate() {
        let outcome = match root.as_ref() {
            Some(root) => parse_item(path, root, ordinal + 1),
            None => Err(rejected_parse(resource_id, ordinal + 1)),
        };
        match outcome {
            Ok(item) => {
                reports.push(item.safe_report().clone());
                items.push(item);
            }
            Err(report) => reports.push(report),
        }
    }
    Ok(BlackboardQtiPackage {
        evidence,
        reports,
        items,
    })
}

fn rejected_parse(resource_id: &str, ordinal: usize) -> QtiSafeItemReport {
    let diagnostic = QtiSafeDiagnostic::new(
        QtiProfileDiagnosticCode::ItemShape,
        QtiSafeDiagnosticLocation::Item,
        QtiSafeDiagnosticTemplate::UnsupportedItemShape,
    )
    .expect("closed Blackboard diagnostic");
    QtiMappedItem::rejected_safe_report_lossy(
        resource_id,
        &format!("blackboard-item-{ordinal}"),
        None,
        vec![diagnostic],
    )
}

fn parse_entry(
    entries: &crate::archive::BoundedArchiveEntries,
    path: &str,
    limits: QtiImportLimits,
) -> Result<XmlNode, BlackboardQtiImportError> {
    let bytes = entries
        .get(path)
        .ok_or(BlackboardQtiImportError::Manifest)?;
    parse_xml(path, bytes, limits).map_err(|_| BlackboardQtiImportError::Manifest)
}

#[allow(clippy::result_large_err)]
fn parse_item(
    path: &str,
    item: &XmlNode,
    ordinal: usize,
) -> Result<QtiMappedItem, QtiSafeItemReport> {
    let id = item.attribute("identifier").unwrap_or_default();
    let title = item
        .attribute("title")
        .filter(|value| !value.trim().is_empty());
    let reject =
        |code, location, template| reject_item(id, title, ordinal, code, location, template);
    let Some(title) = title else {
        return Err(reject(
            QtiProfileDiagnosticCode::ItemShape,
            QtiSafeDiagnosticLocation::Item,
            QtiSafeDiagnosticTemplate::MissingRequiredField,
        ));
    };
    if !shape::valid_assessment_item_shape(item) {
        return Err(reject(
            QtiProfileDiagnosticCode::ItemShape,
            QtiSafeDiagnosticLocation::Item,
            QtiSafeDiagnosticTemplate::UnsupportedItemShape,
        ));
    }
    let Some((response_id, correct)) = shape::response(item) else {
        return Err(reject(
            QtiProfileDiagnosticCode::CorrectResponse,
            QtiSafeDiagnosticLocation::Response,
            QtiSafeDiagnosticTemplate::UnsupportedItemShape,
        ));
    };
    let Some((body, interaction)) = shape::body_and_interaction(item) else {
        return Err(reject(
            QtiProfileDiagnosticCode::ItemShape,
            QtiSafeDiagnosticLocation::Prompt,
            QtiSafeDiagnosticTemplate::UnsupportedItemShape,
        ));
    };
    if !shape::valid_interaction(interaction, &response_id) {
        let code = if interaction.attribute("shuffle") == Some("true") {
            QtiProfileDiagnosticCode::Shuffle
        } else {
            QtiProfileDiagnosticCode::ResponseCardinality
        };
        return Err(reject(
            code,
            QtiSafeDiagnosticLocation::Response,
            QtiSafeDiagnosticTemplate::UnsupportedItemShape,
        ));
    }
    if !shape::valid_processing(item, &response_id) {
        return Err(reject(
            QtiProfileDiagnosticCode::ResponseProcessing,
            QtiSafeDiagnosticLocation::Response,
            QtiSafeDiagnosticTemplate::UnsupportedResponseProcessing,
        ));
    }
    let prompt =
        super::markup::project_blackboard_item_body(body, MarkupLimits::PROMPT).map_err(|_| {
            reject(
                QtiProfileDiagnosticCode::Markup,
                QtiSafeDiagnosticLocation::Prompt,
                QtiSafeDiagnosticTemplate::UnsupportedMarkup,
            )
        })?;
    let choice_nodes = interaction.children();
    if !(2..=100).contains(&choice_nodes.len()) {
        return Err(reject(
            QtiProfileDiagnosticCode::ChoiceCount,
            QtiSafeDiagnosticLocation::Response,
            QtiSafeDiagnosticTemplate::UnsupportedItemShape,
        ));
    }
    let mut vendor = Vec::new();
    let mut choices = Vec::new();
    for (index, choice) in choice_nodes.iter().enumerate() {
        let Some(choice_id) = choice.attribute("identifier") else {
            return Err(reject(
                QtiProfileDiagnosticCode::DuplicateChoiceId,
                QtiSafeDiagnosticLocation::Choice {
                    index: (index + 1) as u8,
                },
                QtiSafeDiagnosticTemplate::MissingRequiredField,
            ));
        };
        vendor.push(choice_id.to_string());
        let text = super::markup::project_blackboard_choice(choice, MarkupLimits::CHOICE).map_err(
            |_| {
                reject(
                    QtiProfileDiagnosticCode::Markup,
                    QtiSafeDiagnosticLocation::Choice {
                        index: (index + 1) as u8,
                    },
                    QtiSafeDiagnosticTemplate::UnsupportedMarkup,
                )
            },
        )?;
        choices.push(text);
    }
    if vendor.iter().collect::<BTreeSet<_>>().len() != vendor.len() {
        return Err(reject(
            QtiProfileDiagnosticCode::DuplicateChoiceId,
            QtiSafeDiagnosticLocation::Response,
            QtiSafeDiagnosticTemplate::UnsupportedItemShape,
        ));
    }
    if !vendor.iter().any(|choice| choice == &correct) {
        return Err(reject(
            QtiProfileDiagnosticCode::CorrectResponse,
            QtiSafeDiagnosticLocation::Response,
            QtiSafeDiagnosticTemplate::UnsupportedItemShape,
        ));
    }
    let map = map_qti_choice_ids(QtiProfileId::BLACKBOARD, id, &vendor).map_err(|_| {
        reject(
            QtiProfileDiagnosticCode::DuplicateChoiceId,
            QtiSafeDiagnosticLocation::Response,
            QtiSafeDiagnosticTemplate::UnsupportedItemShape,
        )
    })?;
    let choices = choices
        .into_iter()
        .zip(&map)
        .map(|(text_markdown, map)| QtiPublicChoiceChecksumInput {
            ple_choice_id: map.ple_choice_id().to_string(),
            text_markdown,
        })
        .collect();
    QtiMappedItem::new(
        QtiProfileId::BLACKBOARD,
        path.to_string(),
        id.to_string(),
        title.to_string(),
        prompt,
        choices,
        QtiMappedPoints::BlackboardDefaulted,
        map,
        correct,
    )
    .map_err(|_| {
        reject(
            QtiProfileDiagnosticCode::ItemShape,
            QtiSafeDiagnosticLocation::Item,
            QtiSafeDiagnosticTemplate::UnsupportedItemShape,
        )
    })
}

fn reject_item(
    id: &str,
    title: Option<&str>,
    ordinal: usize,
    code: QtiProfileDiagnosticCode,
    location: QtiSafeDiagnosticLocation,
    template: QtiSafeDiagnosticTemplate,
) -> QtiSafeItemReport {
    let diagnostic =
        QtiSafeDiagnostic::new(code, location, template).expect("closed Blackboard diagnostic");
    QtiMappedItem::rejected_safe_report_lossy(
        id,
        &format!("blackboard-item-{ordinal}"),
        title,
        vec![diagnostic],
    )
}

#[cfg(test)]
#[path = "blackboard/tests.rs"]
mod tests;

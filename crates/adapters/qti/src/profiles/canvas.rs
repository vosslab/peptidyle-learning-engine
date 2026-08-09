use super::markup::MarkupLimits;
use super::{
    CANVAS_ITEM_NAMESPACE, IMS_CONTENT_PACKAGING_NAMESPACE, QtiMappedItem, QtiMappedPoints,
    QtiProfileDetection, QtiProfileDetectionEvidence, QtiProfileDiagnosticCode, QtiProfileId,
    QtiProfileItemEvidence, QtiProfileReportDigestInput, QtiProfileResourceEvidence,
    QtiPublicChoiceDigestInput, QtiSafeDiagnostic, QtiSafeDiagnosticLocation,
    QtiSafeDiagnosticTemplate, QtiSafeItemReport, map_qti_choice_ids,
};
use crate::archive::read_bounded_archive;
use crate::model::QtiImportLimits;
use crate::xml::{XmlContentRef, XmlNode, parse_xml};
mod shape;
use std::collections::BTreeSet;
use std::fmt;

const MANIFEST: &str = "imsmanifest.xml";
const META: &str = "canvas_qti12_questions/assessment_meta.xml";
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CanvasQtiImportError {
    Archive,
    Manifest,
    Detection(QtiProfileDiagnosticCode),
}
impl fmt::Display for CanvasQtiImportError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Canvas QTI package does not meet the supported profile")
    }
}
impl std::error::Error for CanvasQtiImportError {}

#[derive(Clone, PartialEq)]
/// ```compile_fail
/// use adapter_qti::profiles::CanvasQtiPackage;
/// fn needs_debug<T: std::fmt::Debug>() {}
/// needs_debug::<CanvasQtiPackage>();
/// ```
/// ```compile_fail
/// use adapter_qti::profiles::CanvasQtiPackage;
/// fn needs_serialize<T: serde::Serialize>() {}
/// needs_serialize::<CanvasQtiPackage>();
/// ```
pub struct CanvasQtiPackage {
    evidence: QtiProfileDetectionEvidence,
    reports: Vec<QtiSafeItemReport>,
    items: Vec<QtiMappedItem>,
}

impl CanvasQtiPackage {
    pub fn detection_evidence(&self) -> &QtiProfileDetectionEvidence {
        &self.evidence
    }
    pub fn reports(&self) -> &[QtiSafeItemReport] {
        &self.reports
    }
    pub fn accepted_count(&self) -> usize {
        self.items.len()
    }
    /// Canonical answer-free report evidence for this exact mapped package.
    pub fn profile_report_digest_input(
        &self,
    ) -> Result<QtiProfileReportDigestInput, super::QtiProfileContractError> {
        super::digests::package_report_digest_input(
            QtiProfileId::CANVAS,
            &self.evidence,
            &self.reports,
            &self.items,
        )
    }
    pub fn into_mapped_items(self) -> Vec<QtiMappedItem> {
        self.items
    }
}

pub fn import_canvas_qti12(
    bytes: &[u8],
    limits: QtiImportLimits,
) -> Result<CanvasQtiPackage, CanvasQtiImportError> {
    let entries = read_bounded_archive(bytes, limits, shape::allowed)
        .map_err(|_| CanvasQtiImportError::Archive)?;
    let manifest = entries
        .get(MANIFEST)
        .ok_or(CanvasQtiImportError::Manifest)?;
    let manifest =
        parse_xml(MANIFEST, manifest, limits).map_err(|_| CanvasQtiImportError::Manifest)?;
    if !no_events(&manifest) {
        return Err(CanvasQtiImportError::Manifest);
    }
    let (schema, resources) =
        shape::manifest_resources(&manifest).ok_or(CanvasQtiImportError::Manifest)?;
    let meta = parse_xml(
        META,
        entries.get(META).ok_or(CanvasQtiImportError::Manifest)?,
        limits,
    )
    .map_err(|_| CanvasQtiImportError::Manifest)?;
    if !no_events(&meta) {
        return Err(CanvasQtiImportError::Manifest);
    }
    if !shape::valid_meta(&meta) {
        return Err(CanvasQtiImportError::Manifest);
    }
    let mut item_roots = Vec::new();
    for resource in &resources {
        if resource.resource_type.as_deref() == Some("imsqti_xmlv1p2") {
            let path = resource
                .href
                .as_deref()
                .ok_or(CanvasQtiImportError::Manifest)?;
            let root = parse_xml(
                path,
                entries.get(path).ok_or(CanvasQtiImportError::Manifest)?,
                limits,
            )
            .map_err(|_| CanvasQtiImportError::Manifest)?;
            item_roots.push((resource.identifier.clone(), path.to_string(), root));
        }
    }
    let evidence = QtiProfileDetectionEvidence {
        manifest_namespace: manifest.namespace_uri().unwrap_or_default().to_string(),
        manifest_schema: schema,
        resources,
        items: item_roots
            .iter()
            .map(|(id, path, root)| QtiProfileItemEvidence {
                resource_identifier: id.clone(),
                path: path.clone(),
                namespace: root.namespace_uri().unwrap_or_default().to_string(),
                root: root.name().to_string(),
            })
            .collect(),
    };
    if let QtiProfileDetection::Rejected(code) = super::detect_qti_profile(&evidence) {
        return Err(CanvasQtiImportError::Detection(code));
    }
    if super::detect_qti_profile(&evidence) != QtiProfileDetection::Recognized(QtiProfileId::CANVAS)
    {
        return Err(CanvasQtiImportError::Detection(
            QtiProfileDiagnosticCode::ProfileAmbiguous,
        ));
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
        return Err(CanvasQtiImportError::Detection(
            QtiProfileDiagnosticCode::UnexpectedEntry,
        ));
    }
    let mut candidates = Vec::new();
    for (_resource, path, root) in &item_roots {
        let item_nodes = shape::canvas_items(root).ok_or(CanvasQtiImportError::Detection(
            QtiProfileDiagnosticCode::ItemShape,
        ))?;
        candidates.extend(item_nodes.into_iter().map(|item| (path.as_str(), item)));
    }
    let mut identifiers = BTreeSet::new();
    if candidates
        .iter()
        .any(|(_, item)| !identifiers.insert(item.attribute("ident").unwrap_or_default()))
    {
        return Err(CanvasQtiImportError::Detection(
            QtiProfileDiagnosticCode::ItemShape,
        ));
    }
    let mut reports = Vec::new();
    let mut items = Vec::new();
    for (ordinal, (path, item)) in candidates.into_iter().enumerate() {
        match parse_item(path, item, ordinal + 1) {
            Ok(mapped) => {
                reports.push(mapped.safe_report().clone());
                items.push(mapped);
            }
            Err(rejected) => reports.push(rejected),
        }
    }
    Ok(CanvasQtiPackage {
        evidence,
        reports,
        items,
    })
}

#[allow(clippy::result_large_err)] // The caller records the bounded report without a second allocation.
fn parse_item(
    path: &str,
    item: &XmlNode,
    ordinal: usize,
) -> Result<QtiMappedItem, QtiSafeItemReport> {
    let id = item.attribute("ident").unwrap_or_default().to_string();
    let title = item
        .attribute("title")
        .filter(|v| !v.trim().is_empty())
        .map(str::to_string);
    let reject = |code, location, template| {
        reject_item(&id, title.clone(), ordinal, code, location, template)
    };
    let Some(title) = title.clone() else {
        return Err(reject(
            QtiProfileDiagnosticCode::ItemShape,
            QtiSafeDiagnosticLocation::Item,
            QtiSafeDiagnosticTemplate::MissingRequiredField,
        ));
    };
    if !all_canvas(item) || !shape::exact_item_tree(item) || forbidden(item) {
        return Err(reject(
            QtiProfileDiagnosticCode::Media,
            QtiSafeDiagnosticLocation::Item,
            QtiSafeDiagnosticTemplate::UnsupportedItemShape,
        ));
    }
    if id.trim().is_empty() {
        return Err(reject(
            QtiProfileDiagnosticCode::ItemShape,
            QtiSafeDiagnosticLocation::Item,
            QtiSafeDiagnosticTemplate::MissingRequiredField,
        ));
    }
    let Some(metadata) = metadata(item) else {
        return Err(reject(
            QtiProfileDiagnosticCode::ItemShape,
            QtiSafeDiagnosticLocation::Item,
            QtiSafeDiagnosticTemplate::UnsupportedItemShape,
        ));
    };
    if metadata.get("question_type") != Some(&"multiple_choice_question".to_string()) {
        return Err(reject(
            QtiProfileDiagnosticCode::QuestionType,
            QtiSafeDiagnosticLocation::Item,
            QtiSafeDiagnosticTemplate::UnsupportedItemShape,
        ));
    }
    let Some(points) = metadata.get("points_possible").cloned() else {
        return Err(reject(
            QtiProfileDiagnosticCode::Points,
            QtiSafeDiagnosticLocation::Points,
            QtiSafeDiagnosticTemplate::MissingRequiredField,
        ));
    };
    if points
        .parse::<f64>()
        .ok()
        .is_none_or(|value| !value.is_finite() || value < 0.0)
    {
        return Err(reject(
            QtiProfileDiagnosticCode::Points,
            QtiSafeDiagnosticLocation::Points,
            QtiSafeDiagnosticTemplate::UnsupportedItemShape,
        ));
    }
    let presentation = exactly(item, "presentation").ok_or_else(|| {
        reject(
            QtiProfileDiagnosticCode::ItemShape,
            QtiSafeDiagnosticLocation::Prompt,
            QtiSafeDiagnosticTemplate::MissingRequiredField,
        )
    })?;
    if presentation.children().len() != 2
        || presentation.children()[0].name() != "material"
        || presentation.children()[1].name() != "response_lid"
    {
        return Err(reject(
            QtiProfileDiagnosticCode::ItemShape,
            QtiSafeDiagnosticLocation::Prompt,
            QtiSafeDiagnosticTemplate::UnsupportedItemShape,
        ));
    }
    let responses: Vec<_> = presentation
        .children()
        .iter()
        .filter(|n| n.name() == "response_lid")
        .collect();
    if responses.len() != 1 || responses[0].attribute("rcardinality") != Some("Single") {
        return Err(reject(
            QtiProfileDiagnosticCode::ResponseCardinality,
            QtiSafeDiagnosticLocation::Response,
            QtiSafeDiagnosticTemplate::UnsupportedItemShape,
        ));
    }
    let response = responses[0];
    let Some(response_id) = response.attribute("ident") else {
        return Err(reject(
            QtiProfileDiagnosticCode::ResponseCardinality,
            QtiSafeDiagnosticLocation::Response,
            QtiSafeDiagnosticTemplate::MissingRequiredField,
        ));
    };
    let render = exactly(response, "render_choice").ok_or_else(|| {
        reject(
            QtiProfileDiagnosticCode::ItemShape,
            QtiSafeDiagnosticLocation::Response,
            QtiSafeDiagnosticTemplate::UnsupportedItemShape,
        )
    })?;
    let prompt_nodes: Vec<_> = presentation
        .children()
        .iter()
        .take_while(|n| n.name() != "response_lid")
        .filter(|n| n.name() == "material")
        .collect();
    if prompt_nodes.len() != 1 {
        return Err(reject(
            QtiProfileDiagnosticCode::ItemShape,
            QtiSafeDiagnosticLocation::Prompt,
            QtiSafeDiagnosticTemplate::MissingRequiredField,
        ));
    }
    let prompt = project_mattext(prompt_nodes[0], MarkupLimits::PROMPT).map_err(|_| {
        reject(
            QtiProfileDiagnosticCode::Markup,
            QtiSafeDiagnosticLocation::Prompt,
            QtiSafeDiagnosticTemplate::UnsupportedMarkup,
        )
    })?;
    let labels: Vec<_> = render
        .children()
        .iter()
        .filter(|n| n.name() == "response_label")
        .collect();
    if !(2..=100).contains(&labels.len()) {
        return Err(reject(
            QtiProfileDiagnosticCode::ChoiceCount,
            QtiSafeDiagnosticLocation::Response,
            QtiSafeDiagnosticTemplate::UnsupportedItemShape,
        ));
    }
    let mut vendor = Vec::new();
    let mut choices = Vec::new();
    for (index, label) in labels.iter().enumerate() {
        let Some(choice_id) = label.attribute("ident") else {
            return Err(reject(
                QtiProfileDiagnosticCode::DuplicateChoiceId,
                QtiSafeDiagnosticLocation::Choice {
                    index: (index + 1) as u8,
                },
                QtiSafeDiagnosticTemplate::MissingRequiredField,
            ));
        };
        vendor.push(choice_id.to_string());
        let choice = project_mattext(
            exactly(label, "material").ok_or_else(|| {
                reject(
                    QtiProfileDiagnosticCode::ItemShape,
                    QtiSafeDiagnosticLocation::Choice {
                        index: (index + 1) as u8,
                    },
                    QtiSafeDiagnosticTemplate::MissingRequiredField,
                )
            })?,
            MarkupLimits::CHOICE,
        )
        .map_err(|_| {
            reject(
                QtiProfileDiagnosticCode::Markup,
                QtiSafeDiagnosticLocation::Choice {
                    index: (index + 1) as u8,
                },
                QtiSafeDiagnosticTemplate::UnsupportedMarkup,
            )
        })?;
        choices.push(choice);
    }
    if metadata.get("original_answer_ids").is_some_and(|v| {
        v.split(',').map(str::trim).collect::<Vec<_>>()
            != vendor.iter().map(String::as_str).collect::<Vec<_>>()
    }) {
        return Err(reject(
            QtiProfileDiagnosticCode::ChoiceCount,
            QtiSafeDiagnosticLocation::Response,
            QtiSafeDiagnosticTemplate::UnsupportedItemShape,
        ));
    }
    let Some(correct) = correct_response(item, response_id) else {
        return Err(reject(
            QtiProfileDiagnosticCode::ResponseProcessing,
            QtiSafeDiagnosticLocation::Response,
            QtiSafeDiagnosticTemplate::UnsupportedResponseProcessing,
        ));
    };
    if !vendor.contains(&correct) {
        return Err(reject(
            QtiProfileDiagnosticCode::CorrectResponse,
            QtiSafeDiagnosticLocation::Response,
            QtiSafeDiagnosticTemplate::UnsupportedItemShape,
        ));
    }
    let map = map_qti_choice_ids(QtiProfileId::CANVAS, &id, &vendor).map_err(|_| {
        reject(
            QtiProfileDiagnosticCode::DuplicateChoiceId,
            QtiSafeDiagnosticLocation::Response,
            QtiSafeDiagnosticTemplate::UnsupportedItemShape,
        )
    })?;
    let mapped_choices = choices
        .into_iter()
        .zip(&map)
        .map(|(text_markdown, map)| QtiPublicChoiceDigestInput {
            ple_choice_id: map.ple_choice_id().to_string(),
            text_markdown,
        })
        .collect();
    QtiMappedItem::new(
        QtiProfileId::CANVAS,
        path.to_string(),
        id.clone(),
        title,
        prompt,
        mapped_choices,
        QtiMappedPoints::Declared(points),
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

fn metadata(item: &XmlNode) -> Option<std::collections::BTreeMap<String, String>> {
    let fields = item.descendants_named("qtimetadatafield");
    let mut result = std::collections::BTreeMap::new();
    for field in fields {
        let label = text(exactly(field, "fieldlabel")?);
        let entry = text(exactly(field, "fieldentry")?);
        if !matches!(
            label.as_str(),
            "question_type" | "points_possible" | "original_answer_ids"
        ) || result.insert(label, entry).is_some()
        {
            return None;
        }
    }
    Some(result)
}
fn project_mattext(material: &XmlNode, limits: MarkupLimits) -> Result<String, ()> {
    let mattexts: Vec<_> = material
        .children()
        .iter()
        .filter(|n| n.name() == "mattext" && n.attribute("texttype") == Some("text/html"))
        .collect();
    if mattexts.len() != 1 {
        return Err(());
    }
    super::markup::project_canvas_mattext(mattexts[0], limits).map_err(|_| ())
}
fn correct_response(item: &XmlNode, response_id: &str) -> Option<String> {
    let processing = exactly(item, "resprocessing")?;
    let condition = exactly(processing, "respcondition")?;
    let conditionvar = exactly(condition, "conditionvar")?;
    let value = exactly(conditionvar, "varequal")?;
    let score = exactly(condition, "setvar")?;
    let answer = text(value);
    (value.attribute("respident") == Some(response_id)
        && !answer.trim().is_empty()
        && score.attribute("varname") == Some("SCORE")
        && score.attribute("action") == Some("Set")
        && text(score).trim() == "100")
        .then_some(answer.trim().to_string())
}
fn forbidden(item: &XmlNode) -> bool {
    !item.descendants_named("itemfeedback").is_empty()
        || [
            "matimage", "matvideo", "mataudio", "table", "style", "outcomes", "decvar",
        ]
        .iter()
        .any(|n| item.contains_named(n))
        || !item.descendants_named("displayfeedback").is_empty()
}
fn all_canvas(item: &XmlNode) -> bool {
    fn valid(node: &XmlNode) -> bool {
        const ALLOWED: &[&str] = &[
            "item",
            "itemmetadata",
            "qtimetadata",
            "qtimetadatafield",
            "fieldlabel",
            "fieldentry",
            "presentation",
            "material",
            "mattext",
            "response_lid",
            "render_choice",
            "response_label",
            "resprocessing",
            "respcondition",
            "conditionvar",
            "varequal",
            "setvar",
        ];
        node.namespace_uri() == Some(CANVAS_ITEM_NAMESPACE)
            && ALLOWED.contains(&node.name())
            && node.attributes().all(|attribute| {
                attribute.namespace_uri().is_none()
                    && match node.name() {
                        "item" => matches!(attribute.local_name(), "ident" | "title"),
                        "response_lid" => {
                            matches!(attribute.local_name(), "ident" | "rcardinality")
                        }
                        "response_label" => attribute.local_name() == "ident",
                        "mattext" => attribute.local_name() == "texttype",
                        "respcondition" => attribute.local_name() == "continue",
                        "varequal" => attribute.local_name() == "respident",
                        "setvar" => matches!(attribute.local_name(), "action" | "varname"),
                        _ => false,
                    }
            })
            && (node.name() != "respcondition" || node.attribute("continue") == Some("No"))
            && node.content().all(|content| {
                !matches!(
                    content,
                    XmlContentRef::Comment(_) | XmlContentRef::ProcessingInstruction { .. }
                )
            })
            && node.children().iter().all(valid)
    }
    valid(item)
}
fn exactly<'a>(node: &'a XmlNode, name: &str) -> Option<&'a XmlNode> {
    let all: Vec<_> = node
        .children()
        .iter()
        .filter(|n| n.name() == name)
        .collect();
    (all.len() == 1).then_some(all[0])
}
fn text(node: &XmlNode) -> String {
    node.normalized_text_except("")
}
fn no_events(node: &XmlNode) -> bool {
    node.content().all(|content| {
        !matches!(
            content,
            XmlContentRef::Comment(_) | XmlContentRef::ProcessingInstruction { .. }
        )
    }) && node.children().iter().all(no_events)
}
fn reject_item(
    id: &str,
    title: Option<String>,
    ordinal: usize,
    code: QtiProfileDiagnosticCode,
    location: QtiSafeDiagnosticLocation,
    template: QtiSafeDiagnosticTemplate,
) -> QtiSafeItemReport {
    let diagnostic =
        QtiSafeDiagnostic::new(code, location, template).expect("closed Canvas diagnostic");
    QtiMappedItem::rejected_safe_report_lossy(
        id,
        &format!("canvas-item-{ordinal}"),
        title.as_deref(),
        vec![diagnostic],
    )
}

#[cfg(test)]
#[path = "canvas/tests.rs"]
mod tests;

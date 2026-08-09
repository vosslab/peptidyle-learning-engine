//! Frozen vendor-profile predicates and fixture references.

use super::{QtiProfileDetectionEvidence, QtiProfileDiagnosticCode, QtiProfileId};

pub const IMS_CONTENT_PACKAGING_NAMESPACE: &str = "http://www.imsglobal.org/xsd/imscp_v1p1";
pub const CANVAS_ITEM_NAMESPACE: &str = "http://www.imsglobal.org/xsd/ims_qtiasiv1p2";
pub const BLACKBOARD_ITEM_NAMESPACE: &str = "http://www.imsglobal.org/xsd/imsqti_v2p1";

/// One reviewable profile row. String fields are XPath-like descriptions, not
/// executable parser instructions; profile parsers must implement them exactly.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct QtiProfileMatrixDetail {
    pub profile: QtiProfileId,
    pub manifest_namespace: &'static str,
    pub manifest_schema: Option<&'static str>,
    pub resource_type: &'static str,
    pub href_prefix: &'static str,
    pub dependency_policy: &'static str,
    pub resource_cardinality: &'static str,
    pub item_cardinality: &'static str,
    pub item_namespace: &'static str,
    pub item_root: &'static str,
    pub title_field: &'static str,
    pub prompt_field: &'static str,
    pub choice_field: &'static str,
    pub correct_field: &'static str,
    pub points_field: &'static str,
    pub normalization: &'static str,
    pub duplicate_policy: &'static str,
    pub missing_policy: &'static str,
    pub rejection_code: QtiProfileDiagnosticCode,
    pub rejection_scope: &'static str,
    pub positive_manifest_fixture: &'static str,
    pub positive_item_fixture: &'static str,
    pub near_miss_manifest_fixture: &'static str,
    pub near_miss_item_fixture: &'static str,
}

pub const QTI_PROFILE_MATRIX: [QtiProfileMatrixDetail; 2] = [
    QtiProfileMatrixDetail {
        profile: QtiProfileId::CANVAS,
        manifest_namespace: IMS_CONTENT_PACKAGING_NAMESPACE,
        manifest_schema: Some("IMS Content"),
        resource_type: "imsqti_xmlv1p2",
        href_prefix: "canvas_qti12_questions/",
        dependency_policy: "each question depends on assessment_meta; assessment_meta depends on every question",
        resource_cardinality: "one assessment_meta plus one or more imsqti_xmlv1p2 resources",
        item_cardinality: "one questestinterop per question resource, with one or more item candidates",
        item_namespace: CANVAS_ITEM_NAMESPACE,
        item_root: "questestinterop",
        title_field: "item/@title (nonblank)",
        prompt_field: "presentation/material/mattext before response_lid",
        choice_field: "response_label/material/mattext in source order",
        correct_field: "sole resprocessing/varequal with SCORE setvar 100",
        points_field: "qtimetadata/points_possible (finite, nonnegative, required)",
        normalization: "entity-decoded UTF-8, LF line endings, allowlisted markup to Markdown",
        duplicate_policy: "duplicate vendor choice ID rejects the item",
        missing_policy: "missing title, prompt, choice, correct response, or points rejects item",
        rejection_code: QtiProfileDiagnosticCode::ItemShape,
        rejection_scope: "resource/item; package continues with siblings",
        positive_manifest_fixture: "tests/fixtures/profiles/canvas_positive_manifest.xml",
        positive_item_fixture: "tests/fixtures/profiles/canvas_positive_item.xml",
        near_miss_manifest_fixture: "tests/fixtures/profiles/canvas_near_miss_manifest.xml",
        near_miss_item_fixture: "tests/fixtures/profiles/canvas_near_miss_item.xml",
    },
    QtiProfileMatrixDetail {
        profile: QtiProfileId::BLACKBOARD,
        manifest_namespace: IMS_CONTENT_PACKAGING_NAMESPACE,
        manifest_schema: Some("QTIv2.1"),
        resource_type: "imsqti_item_xmlv2p1",
        href_prefix: "qti21_items/",
        dependency_policy: "each item depends on assessment_meta; assessment_meta depends on every item",
        resource_cardinality: "one assessment_meta plus one or more imsqti_item_xmlv2p1 resources",
        item_cardinality: "exactly one assessmentItem document per item resource",
        item_namespace: BLACKBOARD_ITEM_NAMESPACE,
        item_root: "assessmentItem",
        title_field: "assessmentItem/@title (nonblank)",
        prompt_field: "itemBody excluding choiceInteraction",
        choice_field: "choiceInteraction/simpleChoice in source order",
        correct_field: "responseDeclaration/correctResponse/value",
        points_field: "PLE default 1.0 with explicit warning",
        normalization: "entity-decoded UTF-8, LF line endings, allowlisted markup to Markdown",
        duplicate_policy: "duplicate simpleChoice identifier rejects the item",
        missing_policy: "missing title, prompt, choice, or correct response rejects item",
        rejection_code: QtiProfileDiagnosticCode::ItemShape,
        rejection_scope: "resource/item; package continues with siblings",
        positive_manifest_fixture: "tests/fixtures/profiles/blackboard_positive_manifest.xml",
        positive_item_fixture: "tests/fixtures/profiles/blackboard_positive_item.xml",
        near_miss_manifest_fixture: "tests/fixtures/profiles/blackboard_near_miss_manifest.xml",
        near_miss_item_fixture: "tests/fixtures/profiles/blackboard_near_miss_item.xml",
    },
];

pub(crate) fn validate_profile_evidence(
    profile: QtiProfileId,
    evidence: &QtiProfileDetectionEvidence,
) -> Result<bool, QtiProfileDiagnosticCode> {
    let Some(row) = QTI_PROFILE_MATRIX.iter().find(|row| row.profile == profile) else {
        return Ok(false);
    };
    if evidence.manifest_namespace != row.manifest_namespace {
        return Ok(false);
    }
    if evidence
        .resources
        .iter()
        .all(|resource| resource.resource_type.as_deref() != Some(row.resource_type))
    {
        return Ok(false);
    }
    if evidence.manifest_schema.as_deref() != row.manifest_schema {
        return Err(QtiProfileDiagnosticCode::ManifestSchema);
    }
    if evidence.resources.is_empty() || evidence.items.is_empty() {
        return Err(QtiProfileDiagnosticCode::ItemShape);
    }
    validate_resource_graph(row, evidence)
}

fn validate_resource_graph(
    row: &QtiProfileMatrixDetail,
    evidence: &QtiProfileDetectionEvidence,
) -> Result<bool, QtiProfileDiagnosticCode> {
    let meta_type = match row.profile {
        QtiProfileId::CANVAS => "associatedcontent/imscc_xmlv1p1/learning-application-resource",
        QtiProfileId::BLACKBOARD => "imsqti_test_xmlv2p1",
        QtiProfileId::GENERIC => return Ok(false),
    };
    let metas = evidence
        .resources
        .iter()
        .filter(|resource| resource.resource_type.as_deref() == Some(meta_type))
        .collect::<Vec<_>>();
    let questions = evidence
        .resources
        .iter()
        .filter(|resource| resource.resource_type.as_deref() == Some(row.resource_type))
        .collect::<Vec<_>>();
    if metas.len() != 1
        || questions.is_empty()
        || metas.len() + questions.len() != evidence.resources.len()
    {
        return Err(QtiProfileDiagnosticCode::UnexpectedEntry);
    }
    let meta = metas[0];
    let expected_meta_href = format!("{}assessment_meta.xml", row.href_prefix);
    if meta.identifier != "assessment_meta"
        || meta.href.as_deref() != Some(expected_meta_href.as_str())
        || !is_normalized_xml_href(meta.href.as_deref())
    {
        return Err(QtiProfileDiagnosticCode::ResourcePath);
    }
    let question_ids = questions
        .iter()
        .map(|resource| resource.identifier.as_str())
        .collect::<Vec<_>>();
    if !same_identifier_set(&meta.dependencies, &question_ids) {
        return Err(QtiProfileDiagnosticCode::UnexpectedEntry);
    }
    for resource in &questions {
        if resource.identifier.trim().is_empty()
            || !is_normalized_item_href(
                resource.href.as_deref().unwrap_or_default(),
                row.href_prefix,
            )
            || !same_identifier_set(&resource.dependencies, &[meta.identifier.as_str()])
        {
            return Err(QtiProfileDiagnosticCode::ResourcePath);
        }
        let matching_items = evidence
            .items
            .iter()
            .filter(|item| {
                item.resource_identifier == resource.identifier
                    && resource.href.as_deref() == Some(item.path.as_str())
                    && item.namespace == row.item_namespace
                    && item.root == row.item_root
            })
            .count();
        let enough_items = if row.profile == QtiProfileId::CANVAS {
            matching_items >= 1
        } else {
            // Blackboard retains a per-resource refusal when a sibling cannot
            // be parsed. A valid sibling still proves the closed package
            // profile; parsed evidence may never be mixed or unowned.
            matching_items <= 1
        };
        if !enough_items {
            return Err(QtiProfileDiagnosticCode::ItemShape);
        }
    }
    if evidence.items.iter().any(|item| {
        !questions.iter().any(|resource| {
            resource.identifier == item.resource_identifier
                && resource.href.as_deref() == Some(item.path.as_str())
                && item.namespace == row.item_namespace
                && item.root == row.item_root
        })
    }) {
        return Err(QtiProfileDiagnosticCode::ItemShape);
    }
    if row.profile == QtiProfileId::BLACKBOARD
        && !evidence.items.iter().any(|item| {
            questions.iter().any(|resource| {
                item.resource_identifier == resource.identifier
                    && resource.href.as_deref() == Some(item.path.as_str())
                    && item.namespace == row.item_namespace
                    && item.root == row.item_root
            })
        })
    {
        return Err(QtiProfileDiagnosticCode::ItemShape);
    }
    Ok(true)
}

fn same_identifier_set(actual: &[String], expected: &[&str]) -> bool {
    actual.len() == expected.len()
        && actual.iter().all(|value| {
            expected
                .iter()
                .filter(|expected| *expected == value)
                .count()
                == 1
        })
}

fn is_normalized_item_href(href: &str, prefix: &str) -> bool {
    href.starts_with(prefix)
        && href.ends_with(".xml")
        && !href.contains('\\')
        && !href.bytes().any(|byte| byte.is_ascii_control())
        && href.split('/').all(|part| {
            !part.is_empty()
                && part != "."
                && part != ".."
                && part
                    .bytes()
                    .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.'))
        })
}

fn is_normalized_xml_href(href: Option<&str>) -> bool {
    href.is_some_and(|href| {
        href.ends_with(".xml")
            && href.split('/').all(|part| {
                !part.is_empty()
                    && part != "."
                    && part != ".."
                    && part.bytes().all(|byte| {
                        byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.')
                    })
            })
    })
}

#[cfg(test)]
#[path = "matrix/tests.rs"]
mod tests;

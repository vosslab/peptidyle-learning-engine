use super::*;
use crate::{QtiProfileDetection, QtiProfileItemEvidence, QtiProfileResourceEvidence};

fn canvas_evidence(href: &str) -> QtiProfileDetectionEvidence {
    QtiProfileDetectionEvidence {
        manifest_namespace: IMS_CONTENT_PACKAGING_NAMESPACE.to_string(),
        manifest_schema: Some("IMS Content".to_string()),
        resources: vec![
            QtiProfileResourceEvidence {
                identifier: "canvas-1".to_string(),
                resource_type: Some("imsqti_xmlv1p2".to_string()),
                href: Some(href.to_string()),
                dependencies: vec!["assessment_meta".to_string()],
            },
            QtiProfileResourceEvidence {
                identifier: "assessment_meta".to_string(),
                resource_type: Some(
                    "associatedcontent/imscc_xmlv1p1/learning-application-resource".to_string(),
                ),
                href: Some("canvas_qti12_questions/assessment_meta.xml".to_string()),
                dependencies: vec!["canvas-1".to_string()],
            },
        ],
        items: vec![QtiProfileItemEvidence {
            resource_identifier: "canvas-1".to_string(),
            path: href.to_string(),
            namespace: CANVAS_ITEM_NAMESPACE.to_string(),
            root: "questestinterop".to_string(),
        }],
    }
}

fn blackboard_evidence(extra_resource: bool) -> QtiProfileDetectionEvidence {
    QtiProfileDetectionEvidence {
        manifest_namespace: IMS_CONTENT_PACKAGING_NAMESPACE.to_string(),
        manifest_schema: Some("QTIv2.1".to_string()),
        resources: vec![
            QtiProfileResourceEvidence {
                identifier: "bb-1".to_string(),
                resource_type: Some("imsqti_item_xmlv2p1".to_string()),
                href: Some("qti21_items/bb-1.xml".to_string()),
                dependencies: vec!["assessment_meta".to_string()],
            },
            QtiProfileResourceEvidence {
                identifier: "assessment_meta".to_string(),
                resource_type: Some("imsqti_test_xmlv2p1".to_string()),
                href: Some("qti21_items/assessment_meta.xml".to_string()),
                dependencies: vec!["bb-1".to_string()],
            },
        ],
        items: vec![QtiProfileItemEvidence {
            resource_identifier: "bb-1".to_string(),
            path: "qti21_items/bb-1.xml".to_string(),
            namespace: BLACKBOARD_ITEM_NAMESPACE.to_string(),
            root: "assessmentItem".to_string(),
        }],
    }
    .with_extra_resource(extra_resource)
}

trait ExtraResource {
    fn with_extra_resource(self, extra: bool) -> Self;
}
impl ExtraResource for QtiProfileDetectionEvidence {
    fn with_extra_resource(mut self, extra: bool) -> Self {
        if extra {
            self.resources.push(QtiProfileResourceEvidence {
                identifier: "asset-1".to_string(),
                resource_type: Some("webcontent".to_string()),
                href: Some("assets/image.png".to_string()),
                dependencies: Vec::new(),
            });
        }
        self
    }
}

#[test]
fn profile_matrix_references_readable_positive_and_near_miss_fixtures() {
    assert_eq!(QTI_PROFILE_MATRIX.len(), 2);
    assert!(
        include_str!("../../../tests/fixtures/profiles/canvas_positive_manifest.xml")
            .contains("canvas_qti12_questions/canvas-1.xml")
    );
    assert!(
        include_str!("../../../tests/fixtures/profiles/canvas_positive_item.xml")
            .contains("questestinterop")
    );
    assert!(
        include_str!("../../../tests/fixtures/profiles/canvas_near_miss_manifest.xml")
            .contains("../")
    );
    assert!(
        include_str!("../../../tests/fixtures/profiles/blackboard_positive_manifest.xml")
            .contains("QTIv2.1")
    );
    assert!(
        include_str!("../../../tests/fixtures/profiles/blackboard_positive_item.xml")
            .contains("assessmentItem")
    );
    assert!(
        include_str!("../../../tests/fixtures/profiles/blackboard_near_miss_manifest.xml")
            .contains("dependency")
    );
}

#[test]
fn canvas_matrix_accepts_its_positive_shape_and_rejects_its_near_miss() {
    assert_eq!(
        crate::detect_qti_profile(&canvas_evidence("canvas_qti12_questions/canvas-1.xml")),
        QtiProfileDetection::Recognized(QtiProfileId::CANVAS)
    );
    assert_eq!(
        crate::detect_qti_profile(&canvas_evidence("canvas_qti12_questions/../escape.xml")),
        QtiProfileDetection::Rejected(QtiProfileDiagnosticCode::ResourcePath)
    );
}

#[test]
fn canvas_requires_observed_metadata_schema_and_meta_path() {
    let mut wrong_schema = canvas_evidence("canvas_qti12_questions/canvas-1.xml");
    wrong_schema.manifest_schema = Some("QTIv2.1".to_string());
    assert_eq!(
        crate::detect_qti_profile(&wrong_schema),
        QtiProfileDetection::Rejected(QtiProfileDiagnosticCode::ManifestSchema)
    );
    let mut cross_directory = canvas_evidence("canvas_qti12_questions/canvas-1.xml");
    cross_directory.resources[1].href = Some("qti21_items/assessment_meta.xml".to_string());
    assert_eq!(
        crate::detect_qti_profile(&cross_directory),
        QtiProfileDetection::Rejected(QtiProfileDiagnosticCode::ResourcePath)
    );
    let mut root_meta = canvas_evidence("canvas_qti12_questions/canvas-1.xml");
    root_meta.resources[1].href = Some("assessment_meta.xml".to_string());
    assert_eq!(
        crate::detect_qti_profile(&root_meta),
        QtiProfileDetection::Rejected(QtiProfileDiagnosticCode::ResourcePath)
    );
}

#[test]
fn blackboard_matrix_accepts_its_positive_shape_and_rejects_dependencies() {
    assert_eq!(
        crate::detect_qti_profile(&blackboard_evidence(false)),
        QtiProfileDetection::Recognized(QtiProfileId::BLACKBOARD)
    );
    assert_eq!(
        crate::detect_qti_profile(&blackboard_evidence(true)),
        QtiProfileDetection::Rejected(QtiProfileDiagnosticCode::UnexpectedEntry)
    );
}

#[test]
fn blackboard_recognizes_a_valid_sibling_when_one_declared_item_has_no_xml_evidence() {
    let mut evidence = blackboard_evidence(false);
    evidence.resources.insert(
        1,
        QtiProfileResourceEvidence {
            identifier: "bb-2".to_string(),
            resource_type: Some("imsqti_item_xmlv2p1".to_string()),
            href: Some("qti21_items/bb-2.xml".to_string()),
            dependencies: vec!["assessment_meta".to_string()],
        },
    );
    evidence.resources[2].dependencies = vec!["bb-1".to_string(), "bb-2".to_string()];
    assert_eq!(
        crate::detect_qti_profile(&evidence),
        QtiProfileDetection::Recognized(QtiProfileId::BLACKBOARD)
    );
    evidence.items.clear();
    assert_eq!(
        crate::detect_qti_profile(&evidence),
        QtiProfileDetection::Rejected(QtiProfileDiagnosticCode::ItemShape)
    );
}

#[test]
fn blackboard_rejects_foreign_or_unowned_parsed_item_evidence() {
    let mut evidence = blackboard_evidence(false);
    evidence.items.push(QtiProfileItemEvidence {
        resource_identifier: "bb-1".to_string(),
        path: "qti21_items/bb-1.xml".to_string(),
        namespace: "urn:foreign".to_string(),
        root: "assessmentItem".to_string(),
    });
    assert_eq!(
        crate::detect_qti_profile(&evidence),
        QtiProfileDetection::Rejected(QtiProfileDiagnosticCode::ItemShape)
    );
}

#[test]
fn ims_content_package_without_a_vendor_resource_remains_generic_compatibility() {
    let evidence = QtiProfileDetectionEvidence {
        manifest_namespace: IMS_CONTENT_PACKAGING_NAMESPACE.to_string(),
        manifest_schema: None,
        resources: vec![QtiProfileResourceEvidence {
            identifier: "other-1".to_string(),
            resource_type: Some("webcontent".to_string()),
            href: Some("content/other.xml".to_string()),
            dependencies: Vec::new(),
        }],
        items: vec![QtiProfileItemEvidence {
            resource_identifier: "other-1".to_string(),
            path: "content/other.xml".to_string(),
            namespace: "urn:other".to_string(),
            root: "otherItem".to_string(),
        }],
    };
    assert_eq!(
        crate::detect_qti_profile(&evidence),
        QtiProfileDetection::GenericCompatibility
    );
}

#[test]
fn vendor_resource_with_the_wrong_schema_is_rejected_not_downgraded() {
    let mut evidence = blackboard_evidence(false);
    evidence.manifest_schema = Some("QTIv2.0".to_string());
    assert_eq!(
        crate::detect_qti_profile(&evidence),
        QtiProfileDetection::Rejected(QtiProfileDiagnosticCode::ManifestSchema)
    );
}

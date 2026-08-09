use super::*;

fn canvas_evidence() -> QtiProfileDetectionEvidence {
    QtiProfileDetectionEvidence {
        manifest_namespace: IMS_CONTENT_PACKAGING_NAMESPACE.to_string(),
        manifest_schema: Some("IMS Content".to_string()),
        resources: vec![
            QtiProfileResourceEvidence {
                identifier: "resource-1".to_string(),
                resource_type: Some("imsqti_xmlv1p2".to_string()),
                href: Some("canvas_qti12_questions/one.xml".to_string()),
                dependencies: vec!["assessment_meta".to_string()],
            },
            QtiProfileResourceEvidence {
                identifier: "assessment_meta".to_string(),
                resource_type: Some(
                    "associatedcontent/imscc_xmlv1p1/learning-application-resource".to_string(),
                ),
                href: Some("canvas_qti12_questions/assessment_meta.xml".to_string()),
                dependencies: vec!["resource-1".to_string()],
            },
        ],
        items: vec![QtiProfileItemEvidence {
            resource_identifier: "resource-1".to_string(),
            path: "canvas_qti12_questions/one.xml".to_string(),
            namespace: CANVAS_ITEM_NAMESPACE.to_string(),
            root: "questestinterop".to_string(),
        }],
    }
}

#[test]
fn profile_ids_are_closed_and_use_the_committed_labels() {
    assert_eq!(
        QtiProfileId::CANVAS.to_string(),
        "canvas-qti-1.2-static-single-choice/v1"
    );
    assert_eq!(
        QtiProfileId::BLACKBOARD.to_string(),
        "blackboard-qti-2.1-static-single-choice-pool/v1"
    );
    assert_eq!(
        QtiProfileId::GENERIC.to_string(),
        "ple-qti-assessment-item-single-choice/v1"
    );
    assert_eq!(
        "qti-1.2-subset".parse::<QtiProfileId>(),
        Err(QtiProfileContractError::UnknownProfile(
            "qti-1.2-subset".to_string()
        ))
    );
}

#[test]
fn profile_and_mapping_versions_serialize_as_their_committed_contract_strings() {
    assert_eq!(
        serde_json::to_string(&QtiProfileId::CANVAS).expect("profile serializes"),
        "\"canvas-qti-1.2-static-single-choice/v1\""
    );
    assert_eq!(
        serde_json::to_string(&QtiProfileVersion::V1).expect("profile version serializes"),
        "\"v1\""
    );
    assert_eq!(
        serde_json::to_string(&QtiMappingVersion::V1).expect("mapping version serializes"),
        "\"v1\""
    );
    assert_eq!(
        serde_json::from_str::<QtiProfileId>("\"blackboard-qti-2.1-static-single-choice-pool/v1\"")
            .expect("profile deserializes"),
        QtiProfileId::BLACKBOARD
    );
}

#[test]
fn detector_requires_correlated_vendor_manifest_and_item_evidence() {
    assert_eq!(
        detect_qti_profile(&canvas_evidence()),
        QtiProfileDetection::Recognized(QtiProfileId::CANVAS)
    );
    let mut near_miss = canvas_evidence();
    near_miss.items[0].root = "assessmentItem".to_string();
    assert_eq!(
        detect_qti_profile(&near_miss),
        QtiProfileDetection::Rejected(QtiProfileDiagnosticCode::ItemShape)
    );
}

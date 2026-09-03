use super::super::{
    CANVAS_ITEM_NAMESPACE, IMS_CONTENT_PACKAGING_NAMESPACE, QtiMappingVersion, QtiProfileDetection,
    QtiProfileDetectionEvidence, QtiProfileId, QtiProfileItemEvidence, QtiProfileResourceEvidence,
    QtiProfileVersion,
};
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

fn public_mapping() -> QtiPublicMappingChecksumInput {
    QtiPublicMappingChecksumInput {
        source_location: "canvas_qti12_questions/one.xml".to_string(),
        source_identifier: "question-1".to_string(),
        title: "Favorite color".to_string(),
        prompt_markdown: "What is your favorite color?".to_string(),
        choices: vec![QtiPublicChoiceChecksumInput {
            ple_choice_id: "blue".to_string(),
            text_markdown: "Blue".to_string(),
        }],
        points: "1".to_string(),
        defaults: Vec::new(),
        warnings: Vec::new(),
    }
}

fn private_mapping() -> QtiPrivateMappingChecksumInput {
    QtiPrivateMappingChecksumInput::new(
        "choice_2".to_string(),
        "blue".to_string(),
        vec![QtiPrivateChoiceMapChecksumInput::new(
            "choice_2".to_string(),
            "blue".to_string(),
        )],
        Vec::new(),
    )
}

fn report(public_mapping: &QtiPublicMappingChecksumInput) -> QtiImportResultChecksumInput {
    QtiImportResultChecksumInput {
        profile: QtiProfileId::CANVAS,
        profile_version: QtiProfileVersion::V1,
        mapping_version: QtiMappingVersion::V1,
        detection: canvas_evidence(),
        detection_outcome: QtiProfileDetection::Recognized(QtiProfileId::CANVAS),
        items: vec![QtiWorkspaceImportItemResult {
            source_identifier: public_mapping.source_identifier.clone(),
            item_id: Some("question-1".to_string()),
            accepted: true,
            public_mapping_checksum: Some(public_mapping.checksum().expect("public checksum")),
            diagnostics: Vec::new(),
        }],
        defaults: Vec::new(),
    }
}

#[test]
fn integrity_checksums_are_deterministic_and_private_debug_is_redacted() {
    let public_mapping = public_mapping();
    let report = report(&public_mapping);
    let private_mapping = private_mapping();
    let first = QtiImportChecksums::compute(&report, &public_mapping, &private_mapping)
        .expect("contract encodes");
    let second = QtiImportChecksums::compute(&report, &public_mapping, &private_mapping)
        .expect("same contract encodes");
    assert_eq!(first, second);
    assert!(!format!("{first:?}").contains("choice_2"));
    assert!(!format!("{first:?}").contains(&first.mapping_checksum.to_string()));
}

#[test]
fn checksum_contract_refuses_import_results_and_private_binding_contradictions() {
    let public_mapping = public_mapping();
    let mut invalid_report = report(&public_mapping);
    invalid_report.items[0].public_mapping_checksum = None;
    assert_eq!(
        QtiImportChecksums::compute(&invalid_report, &public_mapping, &private_mapping()),
        Err(QtiProfileContractError::ItemChecksumImportResult)
    );
    let report = report(&public_mapping);
    let missing_binding = QtiPrivateMappingChecksumInput::new(
        "choice_2".to_string(),
        "blue".to_string(),
        Vec::new(),
        Vec::new(),
    );
    assert_eq!(
        QtiImportChecksums::compute(&report, &public_mapping, &missing_binding),
        Err(QtiProfileContractError::PrivateBindingMissing)
    );
}

#[test]
fn checksum_contract_is_sensitive_to_one_field_and_choice_order() {
    let first = public_mapping();
    let mut changed = public_mapping();
    changed.title = "Other color".to_string();
    assert_ne!(
        first.checksum().expect("checksum"),
        changed.checksum().expect("checksum")
    );
    let mut reordered = public_mapping();
    reordered.choices.push(QtiPublicChoiceChecksumInput {
        ple_choice_id: "red".to_string(),
        text_markdown: "Red".to_string(),
    });
    let ordered = reordered.checksum().expect("checksum");
    reordered.choices.reverse();
    assert_ne!(ordered, reordered.checksum().expect("checksum"));
}

#[test]
fn public_mapping_deterministic_encoding_and_checksum_are_golden() {
    let public_mapping = public_mapping();
    let bytes = serde_json::to_vec(&DeterministicChecksumInput {
        schema: "public-mapping",
        value: &public_mapping,
    })
    .expect("deterministic checksum encoding");
    assert_eq!(
        std::str::from_utf8(&bytes).expect("deterministic JSON is UTF-8"),
        "{\"schema\":\"public-mapping\",\"value\":{\"source_location\":\"canvas_qti12_questions/one.xml\",\"source_identifier\":\"question-1\",\"title\":\"Favorite color\",\"prompt_markdown\":\"What is your favorite color?\",\"choices\":[{\"ple_choice_id\":\"blue\",\"text_markdown\":\"Blue\"}],\"points\":\"1\",\"defaults\":[],\"warnings\":[]}}"
    );
    assert_eq!(
        public_mapping.checksum().expect("checksum").to_string(),
        "e511e37a056973d2f21e3522f6c8362603e0de4fc807c8ccd006b32515db39c0"
    );
}

#[test]
fn generic_profile_uses_generic_compatibility_for_a_valid_checksum_contract() {
    let public_mapping = public_mapping();
    let mut report = report(&public_mapping);
    report.profile = QtiProfileId::GENERIC;
    report.detection = QtiProfileDetectionEvidence {
        manifest_namespace: "unknown".to_string(),
        manifest_schema: None,
        resources: Vec::new(),
        items: Vec::new(),
    };
    report.detection_outcome = QtiProfileDetection::GenericCompatibility;
    assert!(QtiImportChecksums::compute(&report, &public_mapping, &private_mapping()).is_ok());
}

use super::super::report::MAX_SAFE_DIAGNOSTICS;
use super::super::{
    QtiProfileDiagnosticCode, QtiSafeDiagnosticLocation, QtiSafeDiagnosticTemplate,
};
use super::*;
use crate::profiles::map_qti_choice_ids;

fn choices() -> Vec<QtiPublicChoiceChecksumInput> {
    vec![
        QtiPublicChoiceChecksumInput {
            ple_choice_id: "blue".to_string(),
            text_markdown: "Blue".to_string(),
        },
        QtiPublicChoiceChecksumInput {
            ple_choice_id: "red".to_string(),
            text_markdown: "Red".to_string(),
        },
    ]
}

fn mapped_item(points: QtiMappedPoints) -> QtiMappedItem {
    let vendor_choice_ids = vec!["blue".to_string(), "red".to_string()];
    let choice_map = map_qti_choice_ids(QtiProfileId::BLACKBOARD, "item-1", &vendor_choice_ids)
        .expect("choice mapping");
    QtiMappedItem::new(
        QtiProfileId::BLACKBOARD,
        "qti21_items/item-1.xml".to_string(),
        "item-1".to_string(),
        "Favorite color".to_string(),
        "What is my favorite color?".to_string(),
        choices(),
        points,
        choice_map,
        "blue".to_string(),
    )
    .expect("mapped item")
}

#[test]
fn safe_report_is_bounded_exact_and_contains_no_private_binding() {
    let item = mapped_item(QtiMappedPoints::BlackboardDefaulted);
    let report = item.safe_report();
    let encoded = serde_json::to_string(report).expect("safe report serializes");
    assert_eq!(report.status(), QtiSafeItemStatus::Accepted);
    assert_eq!(report.defaults().len(), QtiPleDefault::ALL.len());
    assert_eq!(report.warnings().len(), 1);
    assert!(encoded.contains("Blackboard item points were absent"));
    assert!(!encoded.contains("blue"));
    assert!(!encoded.contains("qti21_items/item-1.xml"));
    assert!(!format!("{report:?}").contains("blue"));
}

#[test]
fn mapped_item_produces_digest_inputs_and_server_parts_without_private_debug() {
    let item = mapped_item(QtiMappedPoints::BlackboardDefaulted);
    assert_eq!(item.public_mapping_checksum_input().points, "1.0");
    let disposition = item
        .accepted_item_disposition()
        .expect("mapped item owns its accepted disposition");
    assert!(disposition.accepted);
    assert!(disposition.public_mapping_checksum.is_some());
    {
        let _private_mapping = item.private_mapping_checksum_input();
    }
    let parts = item.into_server_parts();
    assert_eq!(parts.profile(), QtiProfileId::BLACKBOARD);
    assert_eq!(parts.profile_version(), QtiProfileId::BLACKBOARD.version());
    assert_eq!(parts.mapping_version(), QtiMappingVersion::V1);
    assert_eq!(parts.public_mapping().title, "Favorite color");
    assert_eq!(parts.server_correct_ple_choice_id(), "blue");
    assert_eq!(parts.server_ordered_choice_map().len(), 2);
    assert_eq!(
        parts.server_choice_map_payload().server_sha256(),
        objects::Sha256Checksum::compute(parts.server_choice_map_payload().server_bytes())
    );
}

#[test]
fn normalized_fingerprint_is_owned_by_mapped_item_and_excludes_mapping_transport_fields() {
    let first = QtiMappedItem::new(
        QtiProfileId::CANVAS,
        "canvas/first.xml".to_string(),
        "first-item".to_string(),
        "Favorite color".to_string(),
        "What is my favorite color?".to_string(),
        choices(),
        QtiMappedPoints::Declared("1.0".to_string()),
        vec![
            QtiChoiceIdMap::new("blue-vendor".to_string(), "blue".to_string()),
            QtiChoiceIdMap::new("red-vendor".to_string(), "red".to_string()),
        ],
        "blue-vendor".to_string(),
    )
    .expect("first mapped item");
    let second = QtiMappedItem::new(
        QtiProfileId::CANVAS,
        "canvas/second.xml".to_string(),
        "second-item".to_string(),
        "Favorite color".to_string(),
        "What is my favorite color?".to_string(),
        vec![
            QtiPublicChoiceChecksumInput {
                ple_choice_id: "first_blue".to_string(),
                text_markdown: "Blue".to_string(),
            },
            QtiPublicChoiceChecksumInput {
                ple_choice_id: "second_red".to_string(),
                text_markdown: "Red".to_string(),
            },
        ],
        QtiMappedPoints::Declared("1.0".to_string()),
        vec![
            QtiChoiceIdMap::new("blue-vendor".to_string(), "first_blue".to_string()),
            QtiChoiceIdMap::new("red-vendor".to_string(), "second_red".to_string()),
        ],
        "blue-vendor".to_string(),
    )
    .expect("second mapped item");

    let first_fingerprint = first.normalized_qti_item_fingerprint();
    assert_eq!(first_fingerprint, second.normalized_qti_item_fingerprint());
    assert_eq!(
        first_fingerprint,
        first.into_server_parts().normalized_qti_item_fingerprint()
    );

    let blackboard = mapped_item(QtiMappedPoints::BlackboardDefaulted);
    assert_ne!(
        first_fingerprint,
        blackboard.normalized_qti_item_fingerprint()
    );
}

#[test]
fn points_policy_and_public_private_choice_binding_are_closed() {
    assert!(matches!(
        QtiMappedPoints::BlackboardDefaulted.resolve(QtiProfileId::CANVAS),
        Err(QtiMappedItemError::ProfilePointsPolicy)
    ));
    assert!(matches!(
        QtiMappedPoints::Declared("NaN".to_string()).resolve(QtiProfileId::CANVAS),
        Err(QtiMappedItemError::InvalidPoints)
    ));
    assert!(matches!(
        QtiMappedPoints::Declared("1".to_string()).resolve(QtiProfileId::BLACKBOARD),
        Err(QtiMappedItemError::ProfilePointsPolicy)
    ));
    let negative_zero = QtiMappedPoints::Declared("-0.0".to_string())
        .resolve(QtiProfileId::CANVAS)
        .expect("negative zero normalizes");
    assert_eq!(negative_zero.0, "0.0");

    let mut choice_map = map_qti_choice_ids(
        QtiProfileId::CANVAS,
        "item-1",
        &["blue".to_string(), "red".to_string()],
    )
    .expect("choice mapping");
    choice_map.pop();
    assert!(matches!(
        QtiMappedItem::new(
            QtiProfileId::CANVAS,
            "canvas/item.xml".to_string(),
            "item-1".to_string(),
            "Favorite color".to_string(),
            "Prompt".to_string(),
            choices(),
            QtiMappedPoints::Declared("1".to_string()),
            choice_map,
            "blue".to_string(),
        ),
        Err(QtiMappedItemError::ChoiceMapMismatch)
    ));

    let duplicate_vendor = vec![
        QtiChoiceIdMap::new("vendor".to_string(), "blue".to_string()),
        QtiChoiceIdMap::new("vendor".to_string(), "red".to_string()),
    ];
    assert!(matches!(
        QtiMappedItem::new(
            QtiProfileId::CANVAS,
            "canvas/item.xml".to_string(),
            "item-1".to_string(),
            "Favorite color".to_string(),
            "Prompt".to_string(),
            choices(),
            QtiMappedPoints::Declared("1".to_string()),
            duplicate_vendor,
            "vendor".to_string(),
        ),
        Err(QtiMappedItemError::DuplicateVendorChoiceId)
    ));

    let reversed_map = vec![
        QtiChoiceIdMap::new("red-vendor".to_string(), "red".to_string()),
        QtiChoiceIdMap::new("blue-vendor".to_string(), "blue".to_string()),
    ];
    assert!(matches!(
        QtiMappedItem::new(
            QtiProfileId::CANVAS,
            "canvas/item.xml".to_string(),
            "item-1".to_string(),
            "Favorite color".to_string(),
            "Prompt".to_string(),
            choices(),
            QtiMappedPoints::Declared("1".to_string()),
            reversed_map,
            "blue-vendor".to_string(),
        ),
        Err(QtiMappedItemError::ChoiceMapOrderMismatch)
    ));
}

#[test]
fn safe_report_constructor_refuses_values_beyond_its_visible_bounds() {
    let choice_map = map_qti_choice_ids(
        QtiProfileId::CANVAS,
        "item-1",
        &["blue".to_string(), "red".to_string()],
    )
    .expect("choice mapping");
    assert!(matches!(
        QtiMappedItem::new(
            QtiProfileId::CANVAS,
            "canvas/item.xml".to_string(),
            "item-1".to_string(),
            "e".repeat(MAX_SAFE_TITLE_CHARS + 1),
            "Prompt".to_string(),
            choices(),
            QtiMappedPoints::Declared("1".to_string()),
            choice_map,
            "blue".to_string(),
        ),
        Err(QtiMappedItemError::TitleTooLong)
    ));
}

#[test]
fn integrity_checksums_refuse_a_report_owned_by_another_profile() {
    let choice_map = map_qti_choice_ids(
        QtiProfileId::CANVAS,
        "item-1",
        &["blue".to_string(), "red".to_string()],
    )
    .expect("choice mapping");
    let item = QtiMappedItem::new(
        QtiProfileId::CANVAS,
        "canvas/item.xml".to_string(),
        "item-1".to_string(),
        "Favorite color".to_string(),
        "Prompt".to_string(),
        choices(),
        QtiMappedPoints::Declared("1".to_string()),
        choice_map,
        "blue".to_string(),
    )
    .expect("mapped item");
    let report = QtiImportResultChecksumInput {
        profile: QtiProfileId::BLACKBOARD,
        profile_version: QtiProfileId::BLACKBOARD.version(),
        mapping_version: QtiMappingVersion::V1,
        detection: super::super::QtiProfileDetectionEvidence {
            manifest_namespace: String::new(),
            manifest_schema: None,
            resources: Vec::new(),
            items: Vec::new(),
        },
        detection_outcome: super::super::QtiProfileDetection::Recognized(QtiProfileId::BLACKBOARD),
        items: Vec::new(),
        defaults: Vec::new(),
    };
    assert!(matches!(
        item.compute_import_checksums(&report),
        Err(QtiProfileContractError::MappingOwnerMismatch)
    ));
}

#[test]
fn rejected_reports_are_bounded_and_do_not_echo_source_or_vendor_identifiers() {
    let diagnostic = QtiSafeDiagnostic::new(
        QtiProfileDiagnosticCode::Markup,
        QtiSafeDiagnosticLocation::Prompt,
        QtiSafeDiagnosticTemplate::UnsupportedMarkup,
    )
    .expect("closed safe diagnostic");
    let report = QtiMappedItem::rejected_safe_report(
        "item-1".to_string(),
        Some("Favorite color".to_string()),
        vec![diagnostic],
    )
    .expect("bounded safe report");
    assert_eq!(report.status(), QtiSafeItemStatus::Rejected);
    assert_eq!(report.diagnostics().len(), 1);
    assert!(report.defaults().is_empty());
    assert!(report.warnings().is_empty());

    assert!(matches!(
        QtiSafeDiagnostic::new(
            QtiProfileDiagnosticCode::CorrectResponse,
            QtiSafeDiagnosticLocation::Response,
            QtiSafeDiagnosticTemplate::UnsupportedMarkup,
        ),
        Err(QtiMappedItemError::UnsafeDiagnostic)
    ));

    let too_many = (0..=MAX_SAFE_DIAGNOSTICS)
        .map(|_| {
            QtiSafeDiagnostic::new(
                QtiProfileDiagnosticCode::Markup,
                QtiSafeDiagnosticLocation::Prompt,
                QtiSafeDiagnosticTemplate::UnsupportedMarkup,
            )
            .expect("closed safe diagnostic")
        })
        .collect();
    assert!(matches!(
        QtiMappedItem::rejected_safe_report(
            "item-1".to_string(),
            Some("Favorite color".to_string()),
            too_many,
        ),
        Err(QtiMappedItemError::UnsafeDiagnostic)
    ));

    let missing_title = QtiMappedItem::rejected_safe_report(
        "item-1".to_string(),
        None,
        vec![
            QtiSafeDiagnostic::new(
                QtiProfileDiagnosticCode::ItemShape,
                QtiSafeDiagnosticLocation::Item,
                QtiSafeDiagnosticTemplate::MissingRequiredField,
            )
            .expect("closed safe diagnostic"),
        ],
    )
    .expect("missing title remains reportable");
    assert_eq!(missing_title.title(), None);
}

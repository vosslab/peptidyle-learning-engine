use std::io::{Cursor, Write};

use super::*;

#[path = "tests/negatives.rs"]
mod negatives;

const MANIFEST: &str =
    include_str!("../../../tests/fixtures/profiles/blackboard_positive_manifest.xml");
const META: &str = include_str!("../../../tests/fixtures/profiles/blackboard_assessment_meta.xml");
const ITEM: &str = include_str!("../../../tests/fixtures/profiles/blackboard_positive_item.xml");

fn archive(item: &str) -> Vec<u8> {
    archive_members(MANIFEST, META, [("qti21_items/bb-1.xml", item)])
}

fn archive_members<'a>(
    manifest: &str,
    meta: &str,
    items: impl IntoIterator<Item = (&'a str, &'a str)>,
) -> Vec<u8> {
    let mut zip = zip::ZipWriter::new(Cursor::new(Vec::new()));
    let options = zip::write::SimpleFileOptions::default();
    for (path, body) in [
        ("imsmanifest.xml", manifest),
        ("qti21_items/assessment_meta.xml", meta),
    ] {
        zip.start_file(path, options).expect("fixture entry");
        zip.write_all(body.as_bytes()).expect("fixture body");
    }
    for (path, body) in items {
        zip.start_file(path, options).expect("fixture item");
        zip.write_all(body.as_bytes()).expect("fixture body");
    }
    zip.finish().expect("fixture archive").into_inner()
}

fn two_item_archive(second: &str) -> Vec<u8> {
    let manifest = MANIFEST
        .replacen(
            "    <resource identifier=\"assessment_meta\"",
            "    <resource identifier=\"bb-2\" type=\"imsqti_item_xmlv2p1\" href=\"qti21_items/bb-2.xml\"><file href=\"qti21_items/bb-2.xml\"/><dependency identifierref=\"assessment_meta\"/></resource>\n    <resource identifier=\"assessment_meta\"",
            1,
        )
        .replacen(
            "<dependency identifierref=\"bb-1\"/>",
            "<dependency identifierref=\"bb-1\"/><dependency identifierref=\"bb-2\"/>",
            1,
        );
    let meta = META.replacen(
        "    </assessmentSection>",
        "      <assessmentItemRef identifier=\"bb-2\" href=\"bb-2.xml\"/>\n    </assessmentSection>",
        1,
    );
    archive_members(
        &manifest,
        &meta,
        [
            ("qti21_items/bb-1.xml", ITEM),
            ("qti21_items/bb-2.xml", second),
        ],
    )
}

#[test]
fn preserves_a_valid_item_when_a_declared_sibling_is_malformed() {
    let package = import_blackboard_qti21(&two_item_archive("<broken"), QtiImportLimits::default())
        .expect("one valid root proves the profile");
    assert_eq!(package.accepted_count(), 1);
    assert_eq!(package.reports().len(), 2);
}

#[test]
fn report_digest_input_preserves_accepted_and_rejected_source_order() {
    let package = import_blackboard_qti21(&two_item_archive("<broken"), QtiImportLimits::default())
        .expect("one valid root proves the profile");
    let report = package
        .profile_report_digest_input()
        .expect("package constructs its report digest input");

    assert_eq!(report.profile, QtiProfileId::BLACKBOARD);
    assert!(matches!(
        report.items.as_slice(),
        [accepted, rejected]
            if accepted.source_identifier == "bb-1"
                && accepted.accepted
                && accepted.public_mapping_sha256.is_some()
                && rejected.source_identifier == "bb-2"
                && !rejected.accepted
                && rejected.public_mapping_sha256.is_none()
                && rejected.item_id.is_none()
                && rejected.diagnostics.iter().all(|diagnostic| {
                    !diagnostic.location.trim().is_empty() && !diagnostic.detail.trim().is_empty()
                })
    ));

    let mapped = package
        .into_mapped_items()
        .pop()
        .expect("accepted item remains package-owned after report projection");
    mapped
        .compute_integrity_digests(&report)
        .expect("ordered accepted disposition binds the package mapping");
}

#[test]
fn foreign_semantic_qnames_refuse_only_their_item() {
    let second = ITEM.replacen("identifier=\"bb-1\"", "identifier=\"bb-2\"", 1);
    for invalid in [
        second.replacen(
            "<choiceInteraction",
            "<choiceInteraction xmlns=\"urn:foreign\"",
            1,
        ),
        second.replacen(
            "<correctResponse>",
            "<correctResponse xmlns=\"urn:foreign\">",
            1,
        ),
        second.replacen(
            "<responseCondition>",
            "<responseCondition xmlns=\"urn:foreign\">",
            1,
        ),
    ] {
        let package =
            import_blackboard_qti21(&two_item_archive(&invalid), QtiImportLimits::default())
                .expect("valid sibling establishes the profile");
        assert_eq!(package.accepted_count(), 1);
        assert_eq!(package.reports().len(), 2);
        assert_eq!(
            package.reports()[1].status(),
            super::super::QtiSafeItemStatus::Rejected
        );
    }
}

#[test]
fn maps_the_frozen_blackboard_pool_with_defaulted_points() {
    let package = import_blackboard_qti21(&archive(ITEM), QtiImportLimits::default())
        .expect("frozen Blackboard fixture maps");
    let repeated = import_blackboard_qti21(&archive(ITEM), QtiImportLimits::default())
        .expect("frozen Blackboard fixture maps repeatedly");
    assert_eq!(package.accepted_count(), 1, "{:?}", package.reports());
    assert_eq!(package.reports()[0].source_identifier(), "bb-1");
    assert_eq!(package.reports()[0].warnings().len(), 1);
    let item = package.into_mapped_items().pop().expect("mapped item");
    let parts = item.into_server_parts();
    assert_eq!(parts.public_mapping().points, "1.0");
    assert_eq!(parts.server_correct_ple_choice_id(), "blue");
    assert_eq!(
        parts.normalized_profile_item_sha256(),
        repeated
            .into_mapped_items()
            .pop()
            .expect("repeated mapped item")
            .normalized_profile_item_sha256()
    );
}

#[test]
fn refuses_real_shuffle_without_losing_the_package_report() {
    let invalid = ITEM.replacen("fixed=\"true\"", "fixed=\"false\"", 1);
    let package = import_blackboard_qti21(&archive(&invalid), QtiImportLimits::default())
        .expect("package remains inspectable");
    assert_eq!(package.accepted_count(), 0);
    assert_eq!(
        package.reports()[0].status(),
        super::super::QtiSafeItemStatus::Rejected
    );
}

#[test]
fn accepts_the_observed_inert_score_declaration() {
    let with_score = ITEM.replacen(
        "  <itemBody>",
        "  <outcomeDeclaration identifier=\"SCORE\" cardinality=\"single\" baseType=\"float\"/>\n  <itemBody>",
        1,
    );
    assert_eq!(
        import_blackboard_qti21(&archive(&with_score), QtiImportLimits::default())
            .expect("observed inert score declaration is provenance only")
            .accepted_count(),
        1
    );
}

#[test]
fn safe_reports_do_not_serialize_the_correct_answer() {
    let package =
        import_blackboard_qti21(&archive(ITEM), QtiImportLimits::default()).expect("maps");
    let safe = serde_json::to_string(package.reports()).expect("safe reports serialize");
    assert!(!safe.contains("blue"));
}

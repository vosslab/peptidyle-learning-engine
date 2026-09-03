use std::io::{Cursor, Write};

use super::*;

#[path = "tests/negatives.rs"]
mod negatives;

const MANIFEST_XML: &str =
    include_str!("../../../tests/fixtures/profiles/canvas_positive_manifest.xml");
const ITEM_XML: &str = include_str!("../../../tests/fixtures/profiles/canvas_positive_item.xml");
const META_XML: &str = include_str!("../../../tests/fixtures/profiles/canvas_assessment_meta.xml");

fn archive(item: &str) -> Vec<u8> {
    archive_members(MANIFEST_XML, item)
}

fn archive_members(manifest: &str, item: &str) -> Vec<u8> {
    archive_all(manifest, item, META_XML)
}

fn archive_all(manifest: &str, item: &str, meta: &str) -> Vec<u8> {
    let mut zip = zip::ZipWriter::new(Cursor::new(Vec::new()));
    let options = zip::write::SimpleFileOptions::default();
    for (path, contents) in [
        (MANIFEST, manifest),
        ("canvas_qti12_questions/assessment_meta.xml", meta),
        ("canvas_qti12_questions/canvas-1.xml", item),
    ] {
        zip.start_file(path, options).expect("fixture entry");
        zip.write_all(contents.as_bytes()).expect("fixture bytes");
    }
    zip.finish().expect("fixture archive").into_inner()
}

#[test]
fn maps_the_frozen_canvas_fixture_and_keeps_correct_choice_private() {
    let package = import_canvas_qti12(&archive(ITEM_XML), QtiImportLimits::default())
        .expect("frozen fixture maps");
    let repeated = import_canvas_qti12(&archive(ITEM_XML), QtiImportLimits::default())
        .expect("frozen fixture maps repeatedly");
    assert_eq!(package.accepted_count(), 1);
    assert_eq!(
        package.detection_evidence().manifest_schema.as_deref(),
        Some("IMS Content")
    );
    let report = &package.reports()[0];
    assert_eq!(report.source_identifier(), "canvas-1");
    assert_eq!(report.title(), Some("Favorite color"));
    assert!(
        serde_json::to_string(report)
            .expect("safe report")
            .contains("Favorite color")
    );
    let parts = package
        .into_mapped_items()
        .pop()
        .expect("mapped item")
        .into_server_parts();
    assert_eq!(
        parts.public_mapping().prompt_markdown,
        "What is my favorite color?"
    );
    assert_eq!(parts.public_mapping().choices[0].ple_choice_id, "blue");
    assert_eq!(parts.server_correct_ple_choice_id(), "blue");
    assert_eq!(
        parts.normalized_qti_item_fingerprint(),
        repeated
            .into_mapped_items()
            .pop()
            .expect("repeated mapped item")
            .normalized_qti_item_fingerprint()
    );
}

#[test]
fn package_owns_a_deterministic_answer_free_qti_import_result_checksum_input() {
    let package = import_canvas_qti12(&archive(ITEM_XML), QtiImportLimits::default())
        .expect("frozen fixture maps");
    let repeated = import_canvas_qti12(&archive(ITEM_XML), QtiImportLimits::default())
        .expect("frozen fixture maps repeatedly");

    let report = package
        .qti_import_result_checksum_input()
        .expect("package constructs its import-result checksum input");
    assert_eq!(
        report,
        repeated
            .qti_import_result_checksum_input()
            .expect("same package constructs the same import-result checksum input")
    );
    assert_eq!(report.profile, QtiProfileId::CANVAS);
    assert!(matches!(
        report.items.as_slice(),
        [item_result]
            if item_result.source_identifier == "canvas-1"
                && item_result.accepted
                && item_result.public_mapping_checksum.is_some()
                && item_result.diagnostics.is_empty()
    ));
    assert!(report.defaults.iter().all(|entry| {
        entry.code == QtiProfileDiagnosticCode::Policy && !entry.detail.trim().is_empty()
    }));

    let encoded =
        serde_json::to_string(&report).expect("safe import-result checksum input serializes");
    assert!(!encoded.contains("blue"));
}

#[test]
fn refuses_bad_cardinality_as_a_safe_item_outcome() {
    let invalid = ITEM_XML.replacen("rcardinality=\"Single\"", "rcardinality=\"Multiple\"", 1);
    let package = import_canvas_qti12(&archive(&invalid), QtiImportLimits::default())
        .expect("valid package remains inspectable");
    assert_eq!(package.accepted_count(), 0);
    assert_eq!(
        package.reports()[0].status(),
        super::super::QtiSafeItemStatus::Rejected
    );
}

#[test]
fn zero_accepted_package_has_a_deterministic_qti_import_result_checksum() {
    let invalid = ITEM_XML.replacen("rcardinality=\"Single\"", "rcardinality=\"Multiple\"", 1);
    let first = import_canvas_qti12(&archive(&invalid), QtiImportLimits::default())
        .expect("recognized package remains reportable");
    let second = import_canvas_qti12(&archive(&invalid), QtiImportLimits::default())
        .expect("same recognized package remains reportable");

    let first_report = first
        .qti_import_result_checksum_input()
        .expect("rejected package owns QTI Import Result Checksum input");
    let second_report = second
        .qti_import_result_checksum_input()
        .expect("repeated rejected package owns QTI Import Result Checksum input");
    assert!(first_report.items.iter().all(|item| {
        !item.accepted && item.public_mapping_checksum.is_none() && !item.diagnostics.is_empty()
    }));
    assert_eq!(
        first_report
            .import_result_checksum()
            .expect("zero-accepted report has a QTI Import Result Checksum"),
        second_report
            .import_result_checksum()
            .expect("repeated zero-accepted report has the same QTI Import Result Checksum")
    );
}

#[test]
fn hostile_visible_fields_never_panic_or_echo_their_raw_values() {
    for invalid in [
        ITEM_XML.replacen("ident=\"canvas-1\"", "ident=\"\"", 1),
        ITEM_XML.replacen(
            "ident=\"canvas-1\"",
            &format!("ident=\"{}\"", "x".repeat(1_025)),
            1,
        ),
        ITEM_XML.replacen(
            "title=\"Favorite color\"",
            &format!("title=\"{}\"", "x".repeat(513)),
            1,
        ),
    ] {
        let outcome = std::panic::catch_unwind(|| {
            import_canvas_qti12(&archive(&invalid), QtiImportLimits::default())
        });
        let package = outcome
            .expect("hostile visible values must not panic")
            .expect("package remains inspectable");
        let encoded = serde_json::to_string(&package.reports()[0]).expect("safe report serializes");
        assert!(!encoded.contains(&"x".repeat(513)));
    }
}

#[test]
fn exact_canvas_attributes_reject_shuffle_and_noncanonical_continue() {
    for invalid in [
        ITEM_XML.replacen("<render_choice>", "<render_choice shuffle=\"Yes\">", 1),
        ITEM_XML.replacen("continue=\"No\"", "continue=\"Yes\"", 1),
    ] {
        let package = import_canvas_qti12(&archive(&invalid), QtiImportLimits::default())
            .expect("package parses");
        assert_eq!(package.accepted_count(), 0);
    }
}

#[test]
fn rejects_an_unexpected_package_entry_before_item_parsing() {
    // The archive helper separately owns hostile-entry assertions; this parser's
    // narrow grammar is exercised directly here with a fresh archive.
    let mut zip = zip::ZipWriter::new(Cursor::new(Vec::new()));
    let options = zip::write::SimpleFileOptions::default();
    for (path, contents) in [
        (MANIFEST, MANIFEST_XML),
        (META, META_XML),
        ("canvas_qti12_questions/canvas-1.xml", ITEM_XML),
        ("assets/nope.txt", "nope"),
    ] {
        zip.start_file(path, options).expect("entry");
        zip.write_all(contents.as_bytes()).expect("bytes");
    }
    let bytes = zip.finish().expect("finish").into_inner();
    assert!(matches!(
        import_canvas_qti12(&bytes, QtiImportLimits::default()),
        Err(CanvasQtiImportError::Archive)
    ));
}

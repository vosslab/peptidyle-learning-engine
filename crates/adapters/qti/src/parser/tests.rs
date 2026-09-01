use super::*;
use base64::Engine;
use image::codecs::jpeg::JpegEncoder;
use image::codecs::png::PngEncoder;
use image::codecs::webp::WebPEncoder;
use image::{ExtendedColorType, ImageEncoder, Rgb, RgbImage};
use std::io::Cursor;
const VALID_PACKAGE: &str = concat!(
    "UEsDBBQAAAAIAHS7B13yXbGdXwAAAIsAAAAPAAAAaW1zbWFuaWZlc3QueG1sVY5RDkAwEESv0uwBNHxXryLClg2l",
    "uku4vYoIfiYvM5PJGF9P5JBFUYuTkCOMJYShA2si8rzGBvnFX4sEPSg5Aib2vAhVl1XtftyKkIPqI7q7xvrSLCWg",
    "rdGfZf0csCdQSwMEFAAAAAgAdLsHXcJKi+S6AAAAiwEAAA4AAABpdGVtcy9pdGVtLnhtbH2QSw7CMAxErxLlAETs",
    "XUu0sOgGUDlBCEaN1CZVHH63J7QgKEXsrPEbe2zQzMTckotlpFbYQ6rs0VLIpE2CRAjEnXdMSzKNDjpa70ZYtdpt",
    "N+vdKqHGh0AmVk8Hwlk3J8I9qKEANSHUj/EIj9W5P9wQOixq75lErEk83UI7vlCYgerSztpbQ6WLFLTpw70mlr9C",
    "ilZfi97CmZynzGzbrqFBGt2lJS5Afbb/wHuJ+TesJtGS9r5MjV+Pd1BLAQIUAxQAAAAIAHS7B13yXbGdXwAAAIsA",
    "AAAPAAAAAAAAAAAAAACAAQAAAABpbXNtYW5pZmVzdC54bWxQSwECFAMUAAAACAB0uwddwkqL5LoAAACLAQAADgAA",
    "AAAAAAAAAAAAgAGMAAAAaXRlbXMvaXRlbS54bWxQSwUGAAAAAAIAAgB5AAAAcgEAAAAA",
);
const TRAVERSAL_PACKAGE: &str = concat!(
    "UEsDBBQAAAAIAHS7B13yXbGdXwAAAIsAAAAPAAAAaW1zbWFuaWZlc3QueG1sVY5RDkAwEESv0uwBNHxXryLClg2l",
    "uku4vYoIfiYvM5PJGF9P5JBFUYuTkCOMJYShA2si8rzGBvnFX4sEPSg5Aib2vAhVl1XtftyKkIPqI7q7xvrSLCWg",
    "rdGfZf0csCdQSwMEFAAAAAgAdLsHXfs5K4IFAAAAAwAAAA0AAAAuLi9lc2NhcGUueG1sS0pMAQBQSwECFAMUAAAA",
    "CAB0uwdd8l2xnV8AAACLAAAADwAAAAAAAAAAAAAAgAEAAAAAaW1zbWFuaWZlc3QueG1sUEsBAhQDFAAAAAgAdLsH",
    "Xfs5K4IFAAAAAwAAAA0AAAAAAAAAAAAAAIABjAAAAC4uL2VzY2FwZS54bWxQSwUGAAAAAAIAAgB4AAAAvAAAAAAA",
);
const ABSOLUTE_PACKAGE: &str = concat!(
    "UEsDBBQAAAAIAHS7B13yXbGdXwAAAIsAAAAPAAAAaW1zbWFuaWZlc3QueG1sVY5RDkAwEESv0uwBNHxXryLClg2l",
    "uku4vYoIfiYvM5PJGF9P5JBFUYuTkCOMJYShA2si8rzGBvnFX4sEPSg5Aib2vAhVl1XtftyKkIPqI7q7xvrSLCWg",
    "rdGfZf0csCdQSwMEFAAAAAgAdLsHXfs5K4IFAAAAAwAAAAsAAAAvZXNjYXBlLnhtbEtKTAEAUEsBAhQDFAAAAAgA",
    "dLsHXfJdsZ1fAAAAiwAAAA8AAAAAAAAAAAAAAIABAAAAAGltc21hbmlmZXN0LnhtbFBLAQIUAxQAAAAIAHS7B137",
    "OSuCBQAAAAMAAAALAAAAAAAAAAAAAACAAYwAAAAvZXNjYXBlLnhtbFBLBQYAAAAAAgACAHYAAAC6AAAAAAA=",
);
const SYMLINK_PACKAGE: &str = concat!(
    "UEsDBBQAAAAIAHS7B13yXbGdXwAAAIsAAAAPAAAAaW1zbWFuaWZlc3QueG1sVY5RDkAwEESv0uwBNHxXryLClg2l",
    "uku4vYoIfiYvM5PJGF9P5JBFUYuTkCOMJYShA2si8rzGBvnFX4sEPSg5Aib2vAhVl1XtftyKkIPqI7q7xvrSLCWg",
    "rdGfZf0csCdQSwMEFAAAAAAAAAAhAPwvb0YGAAAABgAAAA4AAABpdGVtcy9saW5rLnhtbHRhcmdldFBLAQIUAxQA",
    "AAAIAHS7B13yXbGdXwAAAIsAAAAPAAAAAAAAAAAAAACAAQAAAABpbXNtYW5pZmVzdC54bWxQSwECFAMUAAAAAAAA",
    "ACEA/C9vRgYAAAAGAAAADgAAAAAAAAAAAAAA/6GMAAAAaXRlbXMvbGluay54bWxQSwUGAAAAAAIAAgB5AAAAvgAA",
    "AAAA",
);
const UNEXPECTED_PACKAGE: &str = concat!(
    "UEsDBBQAAAAIAHS7B13yXbGdXwAAAIsAAAAPAAAAaW1zbWFuaWZlc3QueG1sVY5RDkAwEESv0uwBNHxXryLClg2l",
    "uku4vYoIfiYvM5PJGF9P5JBFUYuTkCOMJYShA2si8rzGBvnFX4sEPSg5Aib2vAhVl1XtftyKkIPqI7q7xvrSLCWg",
    "rdGfZf0csCdQSwMEFAAAAAgAdLsHXfs5K4IFAAAAAwAAAAgAAABldmlsLmV4ZUtKTAEAUEsBAhQDFAAAAAgAdLsH",
    "XfJdsZ1fAAAAiwAAAA8AAAAAAAAAAAAAAIABAAAAAGltc21hbmlmZXN0LnhtbFBLAQIUAxQAAAAIAHS7B137OSuC",
    "BQAAAAMAAAAIAAAAAAAAAAAAAACAAYwAAABldmlsLmV4ZVBLBQYAAAAAAgACAHMAAAC3AAAAAAA=",
);
fn fixture(s: &str) -> Vec<u8> {
    base64::engine::general_purpose::STANDARD
        .decode(s.trim())
        .expect("base64")
}

fn valid_png() -> Vec<u8> {
    let image = RgbImage::from_pixel(2, 1, Rgb([12, 34, 56]));
    let mut bytes = Vec::new();
    PngEncoder::new(&mut bytes)
        .write_image(
            image.as_raw(),
            image.width(),
            image.height(),
            ExtendedColorType::Rgb8,
        )
        .expect("PNG fixture encodes");
    bytes
}

fn valid_jpeg() -> Vec<u8> {
    let image = RgbImage::from_pixel(2, 1, Rgb([12, 34, 56]));
    let mut bytes = Vec::new();
    JpegEncoder::new_with_quality(&mut bytes, 90)
        .write_image(
            image.as_raw(),
            image.width(),
            image.height(),
            ExtendedColorType::Rgb8,
        )
        .expect("JPEG fixture encodes");
    bytes
}

fn valid_webp() -> Vec<u8> {
    let image = RgbImage::from_pixel(2, 1, Rgb([12, 34, 56]));
    let mut bytes = Vec::new();
    WebPEncoder::new_lossless(&mut bytes)
        .write_image(
            image.as_raw(),
            image.width(),
            image.height(),
            ExtendedColorType::Rgb8,
        )
        .expect("WebP fixture encodes");
    bytes
}
#[test]
fn imports_supported_single_choice_with_no_debuggable_answer_or_archive() {
    let imported = QtiImporter::default()
        .import(&fixture(VALID_PACKAGE))
        .expect("valid");
    assert_eq!(
        imported.questions,
        vec![ImportedQtiQuestion {
            item_id: "item".into(),
            prompt: vec![QuestionContentBlock::Text {
                markdown: "Choose the correct answer.".into(),
            }],
            response: QuestionResponseFormat::MultipleChoice {
                choices: vec![
                    QuestionChoice {
                        id: ResponseItemReference::new("a"),
                        body: vec![QuestionContentBlock::Text {
                            markdown: "A".into(),
                        }],
                    },
                    QuestionChoice {
                        id: ResponseItemReference::new("b"),
                        body: vec![QuestionContentBlock::Text {
                            markdown: "B".into(),
                        }],
                    },
                ],
                selection: ResponseSelectionRule::ExactlyOne,
            },
        }]
    );
    let debug = format!("{imported:?}");
    assert!(
        !debug.contains("ResponseItemReference")
            && !debug.contains("correctResponse")
            && !debug.contains("PK\\x03\\x04")
    );
    assert!(debug.contains("<server-only>"));
    assert_eq!(imported.worker_original_bytes(), fixture(VALID_PACKAGE));
    let item_id = &imported.questions[0].item_id;
    assert_eq!(
        imported.worker_correct_choice(item_id),
        Some(ResponseItemReference::new("b"))
    );
}
#[test]
fn hostile_zip_corpus_is_rejected() {
    for (name, package) in [
        ("traversal", TRAVERSAL_PACKAGE),
        ("absolute", ABSOLUTE_PACKAGE),
        ("symlink", SYMLINK_PACKAGE),
        ("unexpected", UNEXPECTED_PACKAGE),
    ] {
        let error = QtiImporter::default()
            .import(&fixture(package))
            .expect_err(name);
        assert!(
            matches!(error, QtiImportError::UnsafeEntry { .. }),
            "{name}: {error}"
        );
    }
}
#[test]
fn xml_parser_rejects_dtd_malformed_comments_and_cdata_deception() {
    for item in [
        "<!DOCTYPE assessmentItem><assessmentItem/>",
        "<assessmentItem><itemBody></assessmentItem>",
        "<!-- <choiceInteraction maxChoices='1'/> --><assessmentItem identifier='x'><itemBody><![CDATA[<choiceInteraction/>]]></itemBody></assessmentItem>",
    ] {
        let bytes = package(&[
            (MANIFEST_PATH, manifest("items/item.xml")),
            ("items/item.xml", item.into()),
        ]);
        let result = QtiImporter::default().import(&bytes);
        assert!(result.is_err() || result.expect("parsed package").questions.is_empty());
    }
}
#[test]
fn xml_parser_refuses_deep_and_wide_documents_at_resource_limits() {
    let deeply_nested = format!(
        "<assessmentItem identifier='x'>{}</assessmentItem>",
        "<outer>".repeat(8) + &"</outer>".repeat(8)
    );
    let deep_package = package(&[
        (MANIFEST_PATH, manifest("items/item.xml")),
        ("items/item.xml", deeply_nested),
    ]);
    let deep_limits = QtiImportLimits {
        max_xml_depth: 5,
        ..QtiImportLimits::default()
    };
    let deep_error = QtiImporter::new(deep_limits)
        .import(&deep_package)
        .expect_err("deep XML must be refused");
    assert!(matches!(deep_error, QtiImportError::InvalidXml { .. }));
    assert!(
        deep_error
            .to_string()
            .contains("XML resource limit exceeded: element depth")
    );

    let wide_item = format!(
        "<assessmentItem identifier='x'>{}</assessmentItem>",
        "<p>one</p>".repeat(12)
    );
    let wide_package = package(&[
        (MANIFEST_PATH, manifest("items/item.xml")),
        ("items/item.xml", wide_item),
    ]);
    let wide_limits = QtiImportLimits {
        max_xml_nodes: 8,
        ..QtiImportLimits::default()
    };
    let wide_error = QtiImporter::new(wide_limits)
        .import(&wide_package)
        .expect_err("wide XML must be refused");
    assert!(matches!(wide_error, QtiImportError::InvalidXml { .. }));
    assert!(
        wide_error
            .to_string()
            .contains("XML resource limit exceeded: element node count")
    );

    let token_limits = QtiImportLimits {
        max_xml_tokens: 5,
        ..QtiImportLimits::default()
    };
    let token_error = QtiImporter::new(token_limits)
        .import(&wide_package)
        .expect_err("token-heavy XML must be refused");
    assert!(matches!(token_error, QtiImportError::InvalidXml { .. }));
    assert!(
        token_error
            .to_string()
            .contains("XML resource limit exceeded: token count")
    );
}
#[test]
fn extracts_verified_image_to_worker_manifest_and_rewrites_prompt() {
    let png = valid_png();
    let item = "<assessmentItem identifier='choice'><responseDeclaration identifier='R'><correctResponse><value>b</value></correctResponse></responseDeclaration><itemBody><p>Look <img src='../assets/p.png' alt='plot'/></p><choiceInteraction responseIdentifier='R' maxChoices='1'><simpleChoice identifier='a'>A</simpleChoice><simpleChoice identifier='b'>B</simpleChoice></choiceInteraction></itemBody></assessmentItem>";
    let bytes = package_bytes(&[
        (MANIFEST_PATH, manifest("items/item.xml").into_bytes()),
        ("items/item.xml", item.as_bytes().to_vec()),
        ("assets/p.png", png.clone()),
    ]);
    let imported = QtiImporter::default()
        .import(&bytes)
        .expect("image imports");
    assert_eq!(
        imported.assets.len(),
        1,
        "unsupported: {:?}",
        imported.unsupported
    );
    assert_eq!(imported.assets[0].worker_bytes(), png.as_slice());
    assert_eq!(imported.assets[0].media_type, "image/png");
    assert!(!format!("{:?}", imported.assets[0]).contains("assets/p.png"));
    assert!(matches!(
        imported.questions[0].prompt.last(),
        Some(QuestionContentBlock::Image { .. })
    ));
}

#[test]
fn qti_assets_share_the_complete_still_raster_boundary() {
    let item = "<assessmentItem identifier='choice'><responseDeclaration identifier='R'><correctResponse><value>b</value></correctResponse></responseDeclaration><itemBody><img src='../assets/asset' alt='plot'/><choiceInteraction responseIdentifier='R' maxChoices='1'><simpleChoice identifier='a'>A</simpleChoice><simpleChoice identifier='b'>B</simpleChoice></choiceInteraction></itemBody></assessmentItem>";
    for (bytes, expected_media_type) in [
        (valid_png(), "image/png"),
        (valid_jpeg(), "image/jpeg"),
        (valid_webp(), "image/webp"),
    ] {
        let archive = package_bytes(&[
            (MANIFEST_PATH, manifest("items/item.xml").into_bytes()),
            ("items/item.xml", item.as_bytes().to_vec()),
            ("assets/asset", bytes),
        ]);
        let imported = QtiImporter::default()
            .import(&archive)
            .expect("safe image imports");
        assert_eq!(imported.worker_assets().len(), 1);
        assert_eq!(
            imported.worker_assets()[0].worker_media_type(),
            expected_media_type
        );
    }
}

#[test]
fn qti_reports_unsafe_image_bytes_without_publishing_an_asset() {
    let item = "<assessmentItem identifier='choice'><responseDeclaration identifier='R'><correctResponse><value>b</value></correctResponse></responseDeclaration><itemBody><img src='../assets/asset' alt='plot'/><choiceInteraction responseIdentifier='R' maxChoices='1'><simpleChoice identifier='a'>A</simpleChoice><simpleChoice identifier='b'>B</simpleChoice></choiceInteraction></itemBody></assessmentItem>";
    let mut polyglot = valid_png();
    polyglot.extend_from_slice(b"PK\\x03\\x04not-an-image");
    for bytes in [
        b"GIF89a".to_vec(),
        vec![0x89, b'P', b'N', b'G', 13, 10, 26, 10],
        polyglot,
    ] {
        let archive = package_bytes(&[
            (MANIFEST_PATH, manifest("items/item.xml").into_bytes()),
            ("items/item.xml", item.as_bytes().to_vec()),
            ("assets/asset", bytes),
        ]);
        let imported = QtiImporter::default()
            .import(&archive)
            .expect("partial report");
        assert!(imported.questions.is_empty());
        assert!(imported.worker_assets().is_empty());
        assert!(
            imported.item_results[0]
                .warnings
                .iter()
                .any(|warning| warning.feature == "unsafe-image")
        );
    }
}

#[test]
fn active_svg_assets_are_rejected_without_disclosing_source_bytes() {
    for (name, svg, secret) in [
        (
            "script",
            "<svg xmlns='http://www.w3.org/2000/svg'><script>svg_script_secret()</script></svg>",
            "svg_script_secret",
        ),
        (
            "event",
            "<svg xmlns='http://www.w3.org/2000/svg' onload='svg_onload_secret()'/>",
            "svg_onload_secret",
        ),
        (
            "foreign-object",
            "<svg xmlns='http://www.w3.org/2000/svg'><foreignObject>svg_foreign_secret</foreignObject></svg>",
            "svg_foreign_secret",
        ),
        (
            "external-reference",
            "<svg xmlns='http://www.w3.org/2000/svg'><image href='https://svg_external_secret.invalid/pixel'/></svg>",
            "svg_external_secret",
        ),
    ] {
        let asset_path = format!("assets/{name}.svg");
        let item = format!(
            "<assessmentItem identifier='choice'><responseDeclaration identifier='R'><correctResponse><value>b</value></correctResponse></responseDeclaration><itemBody><p>Look <img src='../{asset_path}' alt='plot'/></p><choiceInteraction responseIdentifier='R' maxChoices='1'><simpleChoice identifier='a'>A</simpleChoice><simpleChoice identifier='b'>B</simpleChoice></choiceInteraction></itemBody></assessmentItem>"
        );
        let archive = package_bytes(&[
            (MANIFEST_PATH, manifest("items/item.xml").into_bytes()),
            ("items/item.xml", item.into_bytes()),
            (&asset_path, svg.as_bytes().to_vec()),
        ]);

        let imported = QtiImporter::default()
            .import(&archive)
            .expect("a safe item-level refusal keeps the package report available");
        assert!(imported.questions.is_empty(), "{name}");
        assert!(imported.worker_assets().is_empty(), "{name}");
        let result = imported.item_results.first().expect("one item result");
        assert_eq!(result.status, QtiItemImportStatus::Rejected, "{name}");
        assert!(
            result
                .warnings
                .iter()
                .any(|warning| warning.feature == "unsafe-image"),
            "{name}: {:?}",
            result.warnings
        );
        let debug = format!("{imported:?}");
        assert!(!debug.contains(secret), "{name}: {debug}");
    }
}

#[test]
fn asset_collector_includes_images_in_choice_bodies() {
    let item = "<assessmentItem identifier='choice'><responseDeclaration identifier='R'><correctResponse><value>b</value></correctResponse></responseDeclaration><itemBody><p>Choose one.</p><choiceInteraction responseIdentifier='R' maxChoices='1'><simpleChoice identifier='a'>A</simpleChoice><simpleChoice identifier='b'><img src='../assets/choice.png' alt='choice diagram'/>B</simpleChoice></choiceInteraction></itemBody></assessmentItem>";
    let archive = package_bytes(&[
        (MANIFEST_PATH, manifest("items/choice.xml").into_bytes()),
        ("items/choice.xml", item.as_bytes().to_vec()),
        ("assets/choice.png", valid_png()),
    ]);
    let imported = QtiImporter::default()
        .import(&archive)
        .expect("choice image parses");
    let question = imported.questions.first().expect("one imported question");
    assert_eq!(
        qti_question_asset_checksums(question)
            .expect("one checksum per logical image")
            .len(),
        1
    );
    assert_eq!(question.prompt.len(), 1, "image is not in the prompt");
}
#[test]
fn import_handoff_keeps_archive_assets_and_grading_server_only() {
    let imported = QtiImporter::default()
        .import(&fixture(VALID_PACKAGE))
        .expect("valid");
    assert_eq!(
        imported.worker_original_size_bytes() as usize,
        fixture(VALID_PACKAGE).len()
    );
    assert_eq!(imported.worker_original_sha256().len(), 64);
    assert!(imported.worker_assets().is_empty());
    let item_id = &imported.questions[0].item_id;
    assert_eq!(
        imported.worker_correct_choice(item_id),
        Some(ResponseItemReference::new("b"))
    );
}

#[test]
fn reports_partial_success_and_normalized_duplicate_warnings() {
    let manifest = multi_item_manifest(&[
        ("accepted", "items/accepted.xml"),
        ("exact-copy", "items/exact.xml"),
        ("likely-copy", "items/likely.xml"),
        ("unsupported", "items/unsupported.xml"),
        ("missing", "items/missing.xml"),
    ]);
    let accepted = single_choice_item("accepted-item", "b", "1");
    let exact = single_choice_item("exact-item", "b", "1");
    let likely = single_choice_item("likely-item", "a", "1");
    let unsupported = single_choice_item("unsupported-item", "b", "2");
    let bytes = package(&[
        (MANIFEST_PATH, manifest),
        ("items/accepted.xml", accepted),
        ("items/exact.xml", exact),
        ("items/likely.xml", likely),
        ("items/unsupported.xml", unsupported),
    ]);

    let imported = QtiImporter::default()
        .import(&bytes)
        .expect("semantic item failures do not reject a safe archive");
    assert_eq!(imported.questions.len(), 3);
    assert_eq!(imported.item_results.len(), 5);
    assert_eq!(imported.worker_original_bytes(), bytes);

    let result = |source: &str| {
        imported
            .item_results
            .iter()
            .find(|result| result.source_identifier == source)
            .expect("resource result")
    };
    assert_eq!(result("accepted").status, QtiItemImportStatus::Accepted);
    assert!(result("accepted").normalized_sha256.is_some());
    assert!(result("accepted").warnings.is_empty());
    assert_eq!(result("exact-copy").status, QtiItemImportStatus::Accepted);
    assert!(
        result("exact-copy")
            .warnings
            .iter()
            .any(|warning| warning.feature == "exact-duplicate-item")
    );
    assert_eq!(result("likely-copy").status, QtiItemImportStatus::Accepted);
    assert!(
        result("likely-copy")
            .warnings
            .iter()
            .any(|warning| warning.feature == "likely-duplicate-item")
    );
    assert_eq!(result("unsupported").status, QtiItemImportStatus::Rejected);
    assert!(
        result("unsupported")
            .warnings
            .iter()
            .any(|warning| warning.feature == "multiple-choice-cardinality")
    );
    assert_eq!(result("missing").status, QtiItemImportStatus::Rejected);
    assert!(
        result("missing")
            .warnings
            .iter()
            .any(|warning| warning.feature == "missing-referenced-entry")
    );
    assert_eq!(
        imported.worker_correct_choice("accepted-item"),
        Some(ResponseItemReference::new("b"))
    );
    assert_eq!(
        imported.worker_correct_choice("likely-item"),
        Some(ResponseItemReference::new("a"))
    );
}

fn manifest(choice: &str) -> String {
    format!(
        "<manifest identifier='package'><resources><resource identifier='choice' type='imsqti_item_xmlv2p1' href='{choice}'/></resources></manifest>"
    )
}

fn multi_item_manifest(resources: &[(&str, &str)]) -> String {
    let resources = resources
        .iter()
        .map(|(identifier, href)| {
            format!(
                "<resource identifier='{identifier}' type='imsqti_item_xmlv2p1' href='{href}'/>"
            )
        })
        .collect::<String>();
    format!("<manifest identifier='package'><resources>{resources}</resources></manifest>")
}

fn single_choice_item(identifier: &str, correct: &str, max_choices: &str) -> String {
    format!(
        "<assessmentItem identifier='{identifier}'><responseDeclaration identifier='R'><correctResponse><value>{correct}</value></correctResponse></responseDeclaration><itemBody><p>Choose one.</p><choiceInteraction responseIdentifier='R' maxChoices='{max_choices}'><simpleChoice identifier='a'>A</simpleChoice><simpleChoice identifier='b'>B</simpleChoice></choiceInteraction></itemBody></assessmentItem>"
    )
}

fn package(files: &[(&str, String)]) -> Vec<u8> {
    package_bytes(
        &files
            .iter()
            .map(|(p, b)| (*p, b.as_bytes().to_vec()))
            .collect::<Vec<_>>(),
    )
}
fn package_bytes(files: &[(&str, Vec<u8>)]) -> Vec<u8> {
    let mut writer = zip::ZipWriter::new(Cursor::new(Vec::new()));
    for (path, contents) in files {
        writer
            .start_file(*path, zip::write::SimpleFileOptions::default())
            .expect("start");
        std::io::Write::write_all(&mut writer, contents).expect("write");
    }
    writer.finish().expect("finish").into_inner()
}

//! Parser-ready vendor fixture corpus bound to the frozen profile matrix.

mod support;

use adapter_qti::{QTI_PROFILE_MATRIX, QtiProfileId, QtiProfileMatrixDetail};
use support::{
    BLACKBOARD_ITEM, BLACKBOARD_MANIFEST, BLACKBOARD_META, BLACKBOARD_NEAR_MISS_ITEM,
    BLACKBOARD_NEAR_MISS_MANIFEST, BLACKBOARD_UNEXPECTED_ASSET, CANVAS_ITEM, CANVAS_MANIFEST,
    CANVAS_META, CANVAS_NEAR_MISS_ITEM, CANVAS_NEAR_MISS_MANIFEST, FixtureEntry,
    assert_well_formed, build_fixture_archive, is_well_formed, read_fixture_archive,
};

fn matrix_row(profile: QtiProfileId) -> &'static QtiProfileMatrixDetail {
    QTI_PROFILE_MATRIX
        .iter()
        .find(|row| row.profile == profile)
        .expect("frozen matrix row")
}

#[test]
fn every_xml_fixture_is_well_formed_before_any_parser_consumes_it() {
    for (name, xml) in [
        ("Canvas positive manifest", CANVAS_MANIFEST),
        ("Canvas positive item", CANVAS_ITEM),
        ("Canvas assessment metadata", CANVAS_META),
        ("Canvas near-miss manifest", CANVAS_NEAR_MISS_MANIFEST),
        ("Canvas near-miss item", CANVAS_NEAR_MISS_ITEM),
        ("Blackboard positive manifest", BLACKBOARD_MANIFEST),
        ("Blackboard positive item", BLACKBOARD_ITEM),
        ("Blackboard assessment metadata", BLACKBOARD_META),
        (
            "Blackboard near-miss manifest",
            BLACKBOARD_NEAR_MISS_MANIFEST,
        ),
        ("Blackboard near-miss item", BLACKBOARD_NEAR_MISS_ITEM),
    ] {
        assert_well_formed(name, xml);
    }
}

#[test]
fn fixture_xml_validator_rejects_a_mismatched_closing_tag() {
    assert_eq!(
        is_well_formed("<manifest><resource></manifest>"),
        Err("mismatched closing element".to_string())
    );
}

#[test]
fn corpus_traces_the_frozen_profile_matrix_and_real_package_layouts() {
    let canvas = matrix_row(QtiProfileId::CANVAS);
    assert_eq!(
        canvas.positive_manifest_fixture,
        "tests/fixtures/profiles/canvas_positive_manifest.xml"
    );
    assert_eq!(
        canvas.positive_item_fixture,
        "tests/fixtures/profiles/canvas_positive_item.xml"
    );
    assert_eq!(
        canvas.near_miss_manifest_fixture,
        "tests/fixtures/profiles/canvas_near_miss_manifest.xml"
    );
    assert_eq!(
        canvas.near_miss_item_fixture,
        "tests/fixtures/profiles/canvas_near_miss_item.xml"
    );
    assert!(CANVAS_MANIFEST.contains("<schema>IMS Content</schema>"));
    assert!(CANVAS_MANIFEST.contains("canvas_qti12_questions/assessment_meta.xml"));
    assert!(CANVAS_MANIFEST.contains("identifierref=\"assessment_meta\""));
    assert!(CANVAS_MANIFEST.contains("identifierref=\"canvas-1\""));
    assert!(CANVAS_META.contains("canvas.instructure.com/xsd/cccv1p0"));

    let blackboard = matrix_row(QtiProfileId::BLACKBOARD);
    assert_eq!(
        blackboard.positive_manifest_fixture,
        "tests/fixtures/profiles/blackboard_positive_manifest.xml"
    );
    assert_eq!(
        blackboard.positive_item_fixture,
        "tests/fixtures/profiles/blackboard_positive_item.xml"
    );
    assert_eq!(
        blackboard.near_miss_manifest_fixture,
        "tests/fixtures/profiles/blackboard_near_miss_manifest.xml"
    );
    assert_eq!(
        blackboard.near_miss_item_fixture,
        "tests/fixtures/profiles/blackboard_near_miss_item.xml"
    );
    assert!(BLACKBOARD_MANIFEST.contains("<schema>QTIv2.1</schema>"));
    assert!(BLACKBOARD_MANIFEST.contains("qti21_items/assessment_meta.xml"));
    assert!(BLACKBOARD_MANIFEST.contains("identifierref=\"assessment_meta\""));
    assert!(BLACKBOARD_MANIFEST.contains("identifierref=\"bb-1\""));
    assert!(BLACKBOARD_META.contains("<assessmentItemRef identifier=\"bb-1\" href=\"bb-1.xml\""));
}

#[test]
fn positive_items_cover_every_v1_static_single_choice_predicate() {
    assert!(
        CANVAS_ITEM
            .contains("<questestinterop xmlns=\"http://www.imsglobal.org/xsd/ims_qtiasiv1p2\"")
    );
    assert!(CANVAS_ITEM.contains("<assessment") && CANVAS_ITEM.contains("<section"));
    assert!(CANVAS_ITEM.contains("<item ident=\"canvas-1\" title=\"Favorite color\""));
    assert!(CANVAS_ITEM.contains("texttype=\"text/html\""));
    assert!(CANVAS_ITEM.contains(
        "<fieldlabel>question_type</fieldlabel><fieldentry>multiple_choice_question</fieldentry>"
    ));
    assert!(
        CANVAS_ITEM
            .contains("<fieldlabel>points_possible</fieldlabel><fieldentry>1.0</fieldentry>")
    );
    assert!(
        CANVAS_ITEM
            .contains("<response_lid ident=\"response1\" rcardinality=\"Single\"><render_choice>")
    );
    assert_eq!(CANVAS_ITEM.matches("<response_label ident=").count(), 2);
    assert_eq!(CANVAS_ITEM.matches("<varequal ").count(), 1);
    assert!(CANVAS_ITEM.contains("<setvar action=\"Set\" varname=\"SCORE\">100</setvar>"));

    assert!(
        BLACKBOARD_ITEM
            .contains("<assessmentItem xmlns=\"http://www.imsglobal.org/xsd/imsqti_v2p1\"")
    );
    assert!(BLACKBOARD_ITEM.contains(
        "identifier=\"bb-1\" title=\"Favorite color\" adaptive=\"false\" timeDependent=\"false\""
    ));
    assert!(BLACKBOARD_ITEM.contains("<responseDeclaration identifier=\"RESPONSE\" cardinality=\"single\" baseType=\"identifier\"><correctResponse><value>blue</value></correctResponse></responseDeclaration>"));
    assert!(BLACKBOARD_ITEM.contains(
        "<choiceInteraction responseIdentifier=\"RESPONSE\" maxChoices=\"1\" shuffle=\"true\">"
    ));
    assert_eq!(
        BLACKBOARD_ITEM.matches("<simpleChoice identifier=").count(),
        2
    );
    assert_eq!(BLACKBOARD_ITEM.matches("fixed=\"true\"").count(), 2);
    assert!(BLACKBOARD_ITEM.contains(
        "<match><variable identifier=\"RESPONSE\"/><correct identifier=\"RESPONSE\"/></match>"
    ));
}

#[test]
fn near_misses_change_one_bounded_profile_fact() {
    assert!(CANVAS_NEAR_MISS_MANIFEST.contains("canvas_qti12_questions/../canvas-1.xml"));
    assert!(CANVAS_NEAR_MISS_ITEM.contains("rcardinality=\"Multiple\""));
    assert!(!CANVAS_NEAR_MISS_ITEM.contains("rcardinality=\"Single\""));
    assert!(BLACKBOARD_NEAR_MISS_MANIFEST.contains("type=\"webcontent\""));
    assert!(BLACKBOARD_NEAR_MISS_ITEM.contains("shuffle=\"true\""));
    assert_eq!(
        BLACKBOARD_NEAR_MISS_ITEM.matches("fixed=\"true\"").count(),
        1
    );
    assert_eq!(
        BLACKBOARD_NEAR_MISS_ITEM.matches("fixed=\"false\"").count(),
        1
    );
}

#[test]
fn package_builder_preserves_safe_sorted_members_and_complete_near_miss_packages() {
    let canvas = build_fixture_archive(&[
        FixtureEntry {
            path: "canvas_qti12_questions/canvas-1.xml",
            contents: CANVAS_ITEM,
        },
        FixtureEntry {
            path: "imsmanifest.xml",
            contents: CANVAS_MANIFEST,
        },
        FixtureEntry {
            path: "canvas_qti12_questions/assessment_meta.xml",
            contents: CANVAS_META,
        },
    ]);
    let blackboard = build_fixture_archive(&[
        FixtureEntry {
            path: "qti21_items/bb-1.xml",
            contents: BLACKBOARD_ITEM,
        },
        FixtureEntry {
            path: "qti21_items/assessment_meta.xml",
            contents: BLACKBOARD_META,
        },
        FixtureEntry {
            path: "imsmanifest.xml",
            contents: BLACKBOARD_MANIFEST,
        },
    ]);
    let blackboard_near_miss = build_fixture_archive(&[
        FixtureEntry {
            path: "qti21_items/bb-1.xml",
            contents: BLACKBOARD_ITEM,
        },
        FixtureEntry {
            path: "qti21_items/assessment_meta.xml",
            contents: BLACKBOARD_META,
        },
        FixtureEntry {
            path: "assets/fixture.txt",
            contents: BLACKBOARD_UNEXPECTED_ASSET,
        },
        FixtureEntry {
            path: "imsmanifest.xml",
            contents: BLACKBOARD_NEAR_MISS_MANIFEST,
        },
    ]);

    assert_eq!(
        read_fixture_archive(&canvas),
        vec![
            (
                "canvas_qti12_questions/assessment_meta.xml".to_string(),
                CANVAS_META.to_string(),
            ),
            (
                "canvas_qti12_questions/canvas-1.xml".to_string(),
                CANVAS_ITEM.to_string(),
            ),
            ("imsmanifest.xml".to_string(), CANVAS_MANIFEST.to_string()),
        ]
    );
    assert_eq!(
        read_fixture_archive(&blackboard),
        vec![
            (
                "imsmanifest.xml".to_string(),
                BLACKBOARD_MANIFEST.to_string()
            ),
            (
                "qti21_items/assessment_meta.xml".to_string(),
                BLACKBOARD_META.to_string(),
            ),
            (
                "qti21_items/bb-1.xml".to_string(),
                BLACKBOARD_ITEM.to_string(),
            ),
        ]
    );
    assert_eq!(
        read_fixture_archive(&blackboard_near_miss),
        vec![
            (
                "assets/fixture.txt".to_string(),
                BLACKBOARD_UNEXPECTED_ASSET.to_string(),
            ),
            (
                "imsmanifest.xml".to_string(),
                BLACKBOARD_NEAR_MISS_MANIFEST.to_string(),
            ),
            (
                "qti21_items/assessment_meta.xml".to_string(),
                BLACKBOARD_META.to_string(),
            ),
            (
                "qti21_items/bb-1.xml".to_string(),
                BLACKBOARD_ITEM.to_string(),
            ),
        ]
    );
}

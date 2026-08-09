use super::*;

#[test]
fn hostile_identifier_and_title_never_panic_or_echo_source_values() {
    for invalid in [
        ITEM.replacen("identifier=\"bb-1\"", "identifier=\"\"", 1),
        ITEM.replacen(
            "identifier=\"bb-1\"",
            &format!("identifier=\"{}\"", "x".repeat(1_025)),
            1,
        ),
        ITEM.replacen(
            "title=\"Favorite color\"",
            &format!("title=\"{}\"", "x".repeat(513)),
            1,
        ),
    ] {
        let result = std::panic::catch_unwind(|| {
            import_blackboard_qti21(&archive(&invalid), QtiImportLimits::default())
        });
        let package = result
            .expect("hostile visible fields do not panic")
            .expect("safe outcome");
        let safe = serde_json::to_string(package.reports()).expect("safe reports serialize");
        assert!(!safe.contains(&"x".repeat(513)));
    }
}

#[test]
fn manifest_and_meta_shape_are_package_refusals() {
    let reordered = MANIFEST.replacen("<metadata>", "<resources></resources><metadata>", 1);
    let bad_meta = META.replacen("href=\"bb-1.xml\"", "href=\"other.xml\"", 1);
    for bytes in [
        archive_members(&reordered, META, [("qti21_items/bb-1.xml", ITEM)]),
        archive_members(MANIFEST, &bad_meta, [("qti21_items/bb-1.xml", ITEM)]),
    ] {
        assert!(matches!(
            import_blackboard_qti21(&bytes, QtiImportLimits::default()),
            Err(BlackboardQtiImportError::Manifest)
        ));
    }
}

#[test]
fn manifest_and_meta_refuse_foreign_names_unknown_attrs_and_structural_text() {
    for manifest in [
        MANIFEST.replacen("<metadata>", "<metadata extra=\"x\">", 1),
        MANIFEST.replacen("<schema>", "<schema>not-whitespace", 1),
        MANIFEST.replacen("<resources>", "<resources xmlns=\"urn:foreign\">", 1),
    ] {
        assert!(matches!(
            import_blackboard_qti21(
                &archive_members(&manifest, META, [("qti21_items/bb-1.xml", ITEM)]),
                QtiImportLimits::default()
            ),
            Err(BlackboardQtiImportError::Manifest)
        ));
    }
    for meta in [
        META.replacen("<testPart", "<testPart extra=\"x\"", 1),
        META.replacen("<assessmentSection", "text<assessmentSection", 1),
        META.replacen(
            "<assessmentItemRef",
            "<assessmentItemRef xmlns=\"urn:foreign\"",
            1,
        ),
    ] {
        assert!(matches!(
            import_blackboard_qti21(
                &archive_members(MANIFEST, &meta, [("qti21_items/bb-1.xml", ITEM)]),
                QtiImportLimits::default()
            ),
            Err(BlackboardQtiImportError::Manifest)
        ));
    }
}

#[test]
fn schema_and_correct_value_are_exact_text_only_leaves() {
    for manifest in [
        MANIFEST.replacen(
            "<schema>QTIv2.1</schema>",
            "<schema extra=\"x\">QTIv2.1</schema>",
            1,
        ),
        MANIFEST.replacen(
            "<schemaversion>2.0</schemaversion>",
            "<schemaversion><!--x-->2.0</schemaversion>",
            1,
        ),
        MANIFEST.replacen(
            "<schema>QTIv2.1</schema>",
            "<schema><x/>QTIv2.1</schema>",
            1,
        ),
    ] {
        assert!(matches!(
            import_blackboard_qti21(
                &archive_members(&manifest, META, [("qti21_items/bb-1.xml", ITEM)]),
                QtiImportLimits::default()
            ),
            Err(BlackboardQtiImportError::Manifest)
        ));
    }
    for item in [
        ITEM.replacen("<value>blue</value>", "<value extra=\"x\">blue</value>", 1),
        ITEM.replacen("<value>blue</value>", "<value><!--x-->blue</value>", 1),
        ITEM.replacen("<value>blue</value>", "<value><x/>blue</value>", 1),
    ] {
        assert_eq!(
            import_blackboard_qti21(&archive(&item), QtiImportLimits::default())
                .unwrap()
                .accepted_count(),
            0
        );
    }
}

#[test]
fn score_declaration_is_absent_or_exactly_inert() {
    let exact = ITEM.replacen(
        "  <itemBody>",
        "  <outcomeDeclaration identifier=\"SCORE\" cardinality=\"single\" baseType=\"float\"/>\n  <itemBody>",
        1,
    );
    let other = exact.replacen("identifier=\"SCORE\"", "identifier=\"OTHER\"", 1);
    assert_eq!(
        import_blackboard_qti21(&archive(ITEM), QtiImportLimits::default())
            .unwrap()
            .accepted_count(),
        1
    );
    assert_eq!(
        import_blackboard_qti21(&archive(&exact), QtiImportLimits::default())
            .unwrap()
            .accepted_count(),
        1
    );
    assert_eq!(
        import_blackboard_qti21(&archive(&other), QtiImportLimits::default())
            .unwrap()
            .accepted_count(),
        0
    );
}

#[test]
fn choice_and_correct_binding_failures_are_safe_item_refusals() {
    for invalid in [
        ITEM.replacen("<value>blue</value>", "", 1),
        ITEM.replacen(
            "<value>blue</value>",
            "<value>blue</value><value>red</value>",
            1,
        ),
        ITEM.replacen("identifier=\"blue\"", "", 1),
        ITEM.replacen("identifier=\"red\"", "identifier=\"blue\"", 1),
        ITEM.replacen("<value>blue</value>", "<value>not-a-choice</value>", 1),
    ] {
        let package = import_blackboard_qti21(&archive(&invalid), QtiImportLimits::default())
            .expect("package remains inspectable");
        assert_eq!(package.accepted_count(), 0);
    }
}

#[test]
fn processing_absent_or_exact_is_accepted_and_semantic_variants_refuse() {
    let absent = ITEM.replacen(
        "  <responseProcessing><responseCondition><responseIf><match><variable identifier=\"RESPONSE\"/><correct identifier=\"RESPONSE\"/></match></responseIf></responseCondition></responseProcessing>\n",
        "",
        1,
    );
    assert_eq!(
        import_blackboard_qti21(&archive(&absent), QtiImportLimits::default())
            .unwrap()
            .accepted_count(),
        1
    );
    let branch = ITEM
        .replacen("<responseIf>", "<responseElseIf>", 1)
        .replacen("</responseIf>", "</responseElseIf>", 1);
    for invalid in [
        branch,
        ITEM.replacen(
            "</responseIf>",
            "<setOutcomeValue identifier=\"SCORE\"/></responseIf>",
            1,
        ),
    ] {
        assert_eq!(
            import_blackboard_qti21(&archive(&invalid), QtiImportLimits::default())
                .unwrap()
                .accepted_count(),
            0
        );
    }
}

#[test]
fn markup_feedback_media_style_and_table_refuse_only_that_item() {
    for invalid in [
        ITEM.replacen("<p>What", "<table><tr><td>x</td></tr></table><p>What", 1),
        ITEM.replacen("<p>What", "<style>x</style><p>What", 1),
        ITEM.replacen("<p>What", "<img src=\"x\"/><p>What", 1),
        ITEM.replacen("  <itemBody>", "  <modalFeedback/>\n  <itemBody>", 1),
    ] {
        assert_eq!(
            import_blackboard_qti21(&archive(&invalid), QtiImportLimits::default())
                .unwrap()
                .accepted_count(),
            0
        );
    }
}

#[test]
fn shuffle_acceptance_is_limited_to_static_order() {
    for accepted in [
        ITEM.replacen(" shuffle=\"true\"", "", 1),
        ITEM.replacen("shuffle=\"true\"", "shuffle=\"false\"", 1),
    ] {
        assert_eq!(
            import_blackboard_qti21(&archive(&accepted), QtiImportLimits::default())
                .unwrap()
                .accepted_count(),
            1
        );
    }
    let shuffled = ITEM.replacen("fixed=\"true\"", "fixed=\"false\"", 1);
    assert_eq!(
        import_blackboard_qti21(&archive(&shuffled), QtiImportLimits::default())
            .unwrap()
            .accepted_count(),
        0
    );
}

#[test]
fn repeat_import_is_deterministic_and_keeps_server_binding_private() {
    let first = import_blackboard_qti21(&archive(ITEM), QtiImportLimits::default()).unwrap();
    let second = import_blackboard_qti21(&archive(ITEM), QtiImportLimits::default()).unwrap();
    assert_eq!(first.reports(), second.reports());
    let first_parts = first.into_mapped_items().pop().unwrap().into_server_parts();
    let second_parts = second
        .into_mapped_items()
        .pop()
        .unwrap()
        .into_server_parts();
    assert_eq!(first_parts.public_mapping(), second_parts.public_mapping());
    assert_eq!(
        first_parts.server_correct_ple_choice_id(),
        second_parts.server_correct_ple_choice_id()
    );
}

#[test]
fn all_invalid_or_mixed_item_roots_cannot_establish_a_profile() {
    assert!(matches!(
        import_blackboard_qti21(&archive("<broken"), QtiImportLimits::default()),
        Err(BlackboardQtiImportError::Detection(
            QtiProfileDiagnosticCode::ItemShape
        ))
    ));
    let foreign = ITEM.replacen(
        "<assessmentItem xmlns=\"http://www.imsglobal.org/xsd/imsqti_v2p1\"",
        "<assessmentItem xmlns=\"urn:foreign\"",
        1,
    );
    assert!(matches!(
        import_blackboard_qti21(&archive(&foreign), QtiImportLimits::default()),
        Err(BlackboardQtiImportError::Detection(
            QtiProfileDiagnosticCode::ItemShape
        ))
    ));
}

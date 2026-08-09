use super::*;
use crate::profiles::QtiSafeItemStatus;

fn item_refusal(item: String) -> CanvasQtiPackage {
    let package = import_canvas_qti12(&archive(&item), QtiImportLimits::default())
        .expect("package-level graph remains valid");
    assert_eq!(package.accepted_count(), 0);
    assert_eq!(package.reports().len(), 1);
    package
}

#[test]
fn declared_points_and_source_order_consistency_refuse_only_the_item() {
    for item in [
        ITEM_XML.replacen("<presentation>", "<presentation>stray", 1),
        ITEM_XML.replacen(
            "<qtimetadatafield><fieldlabel>points_possible</fieldlabel><fieldentry>1.0</fieldentry></qtimetadatafield>",
            "", 1),
        ITEM_XML.replacen("<fieldentry>1.0</fieldentry>", "<fieldentry>NaN</fieldentry>", 1),
        ITEM_XML.replacen("<fieldentry>1.0</fieldentry>", "<fieldentry>-1</fieldentry>", 1),
        ITEM_XML.replacen("blue,red", "red,blue", 1),
    ] {
        let _ = item_refusal(item);
    }
}

#[test]
fn choice_and_correct_binding_failures_are_item_refusals() {
    for item in [
        ITEM_XML.replacen("ident=\"red\"", "ident=\"blue\"", 1),
        ITEM_XML.replacen(">blue</varequal>", ">green</varequal>", 1),
        ITEM_XML.replacen("<setvar action=\"Set\" varname=\"SCORE\">100</setvar>", "<setvar action=\"Set\" varname=\"SCORE\">50</setvar>", 1),
        ITEM_XML.replacen("</conditionvar>", "</conditionvar><conditionvar><varequal respident=\"response1\">red</varequal></conditionvar>", 1),
    ] {
        let _ = item_refusal(item);
    }
}

#[test]
fn forbidden_feedback_media_and_markup_are_item_refusals() {
    for item in [
        ITEM_XML.replacen("</item>", "<itemfeedback ident=\"f\"/></item>", 1),
        ITEM_XML.replacen("</item>", "<displayfeedback linkrefid=\"f\"/></item>", 1),
        ITEM_XML.replacen(
            "</presentation>",
            "<matimage uri=\"x.png\"/></presentation>",
            1,
        ),
        ITEM_XML.replacen("Blue", "<table><tr><td>Blue</td></tr></table>", 1),
        ITEM_XML.replacen("Blue", "<style>Blue</style>", 1),
        ITEM_XML.replacen("text/html", "text/plain", 1),
    ] {
        let _ = item_refusal(item);
    }
}

#[test]
fn exact_presentation_and_attribute_grammar_refuses_extensions() {
    for item in [
        ITEM_XML.replacen(
            "</presentation>",
            "<material><mattext texttype=\"text/html\">Extra</mattext></material></presentation>",
            1,
        ),
        ITEM_XML.replacen(
            "<response_label ident=\"blue\"",
            "<response_label bad=\"x\" ident=\"blue\"",
            1,
        ),
    ] {
        let _ = item_refusal(item);
    }
    for item in [
        ITEM_XML.replacen("<assessment", "<assessment xmlns=\"urn:foreign\"", 1),
        ITEM_XML.replacen("<section", "<section xmlns=\"urn:foreign\"", 1),
    ] {
        assert!(matches!(
            import_canvas_qti12(&archive(&item), QtiImportLimits::default()),
            Err(CanvasQtiImportError::Detection(
                QtiProfileDiagnosticCode::ItemShape
            ))
        ));
    }
}

#[test]
fn duplicate_item_identifier_is_a_package_refusal() {
    let duplicate = ITEM_XML.replacen(
        "</section>",
        "<item ident=\"canvas-1\" title=\"Second\"><itemmetadata><qtimetadata><qtimetadatafield><fieldlabel>question_type</fieldlabel><fieldentry>multiple_choice_question</fieldentry></qtimetadatafield><qtimetadatafield><fieldlabel>points_possible</fieldlabel><fieldentry>1</fieldentry></qtimetadatafield></qtimetadata></itemmetadata><presentation><material><mattext texttype=\"text/html\">Second?</mattext></material><response_lid ident=\"response2\" rcardinality=\"Single\"><render_choice><response_label ident=\"a\"><material><mattext texttype=\"text/html\">A</mattext></material></response_label><response_label ident=\"b\"><material><mattext texttype=\"text/html\">B</mattext></material></response_label></render_choice></response_lid></presentation><resprocessing><respcondition continue=\"No\"><conditionvar><varequal respident=\"response2\">a</varequal></conditionvar><setvar action=\"Set\" varname=\"SCORE\">100</setvar></respcondition></resprocessing></item></section>", 1);
    assert!(matches!(
        import_canvas_qti12(&archive(&duplicate), QtiImportLimits::default()),
        Err(CanvasQtiImportError::Detection(
            QtiProfileDiagnosticCode::ItemShape
        ))
    ));
}

#[test]
fn repeat_import_is_deterministic_and_safe_report_has_no_answer_association() {
    let first = import_canvas_qti12(&archive(ITEM_XML), QtiImportLimits::default()).expect("first");
    let second =
        import_canvas_qti12(&archive(ITEM_XML), QtiImportLimits::default()).expect("second");
    let first_json = serde_json::to_string(first.reports()).expect("safe json");
    assert_eq!(
        first_json,
        serde_json::to_string(second.reports()).expect("safe json")
    );
    assert!(!first_json.contains("blue"));
    assert!(!first_json.contains("canvas_qti12_questions"));
    let first_parts = first
        .into_mapped_items()
        .pop()
        .expect("mapped")
        .into_server_parts();
    let second_parts = second
        .into_mapped_items()
        .pop()
        .expect("mapped")
        .into_server_parts();
    assert_eq!(first_parts.profile(), second_parts.profile());
    assert_eq!(first_parts.public_mapping(), second_parts.public_mapping());
    assert!(first_parts.private_mapping() == second_parts.private_mapping());
    assert!(first_parts.server_ordered_choice_map() == second_parts.server_ordered_choice_map());
}

#[test]
fn one_valid_sibling_survives_an_invalid_sibling_in_the_same_resource() {
    let body = ITEM_XML
        .split_once("<item ")
        .expect("fixture item")
        .1
        .split_once("</item>")
        .expect("fixture item")
        .0;
    let invalid = format!(
        "<item {}",
        body.replacen("ident=\"canvas-1\"", "ident=\"canvas-2\"", 1)
            .replacen("rcardinality=\"Single\"", "rcardinality=\"Multiple\"", 1)
    );
    let package_xml = ITEM_XML.replacen("</section>", &format!("{invalid}</item></section>"), 1);
    let package = import_canvas_qti12(&archive(&package_xml), QtiImportLimits::default())
        .expect("valid package with one item refusal");
    assert_eq!(package.accepted_count(), 1);
    assert_eq!(package.reports().len(), 2);
    assert_eq!(package.reports()[0].status(), QtiSafeItemStatus::Accepted);
    assert_eq!(package.reports()[1].status(), QtiSafeItemStatus::Rejected);
}

#[test]
fn comments_pis_and_empty_containers_are_package_refusals() {
    let manifest_comment = MANIFEST_XML.replacen("<resources>", "<!-- hostile --><resources>", 1);
    assert!(matches!(
        import_canvas_qti12(
            &archive_members(&manifest_comment, ITEM_XML),
            QtiImportLimits::default()
        ),
        Err(CanvasQtiImportError::Manifest)
    ));
    let manifest_pi = MANIFEST_XML.replacen("<resources>", "<?hostile no?><resources>", 1);
    assert!(matches!(
        import_canvas_qti12(
            &archive_members(&manifest_pi, ITEM_XML),
            QtiImportLimits::default()
        ),
        Err(CanvasQtiImportError::Manifest)
    ));
    let empty = ITEM_XML
        .replacen(
            "<item ident=\"canvas-1\"",
            "<not_item ident=\"canvas-1\"",
            1,
        )
        .replacen("</item>", "</not_item>", 1);
    assert!(matches!(
        import_canvas_qti12(&archive(&empty), QtiImportLimits::default()),
        Err(CanvasQtiImportError::Detection(
            QtiProfileDiagnosticCode::ItemShape
        ))
    ));
}

#[test]
fn manifest_resource_and_meta_extensions_are_package_refusals() {
    for manifest in [
        MANIFEST_XML.replacen("</resources>", "<evil/></resources>", 1),
        MANIFEST_XML.replacen(
            "<file href=\"canvas_qti12_questions/canvas-1.xml\"/>",
            "<file href=\"wrong.xml\"/>",
            1,
        ),
        MANIFEST_XML.replacen(
            "<schema>IMS Content</schema>",
            "<schema>IMS Content</schema><evil/>",
            1,
        ),
    ] {
        assert!(matches!(
            import_canvas_qti12(
                &archive_members(&manifest, ITEM_XML),
                QtiImportLimits::default()
            ),
            Err(CanvasQtiImportError::Manifest)
        ));
    }
    let meta = META_XML.replacen("</quiz>", "<evil/></quiz>", 1);
    assert!(matches!(
        import_canvas_qti12(
            &archive_all(MANIFEST_XML, ITEM_XML, &meta),
            QtiImportLimits::default()
        ),
        Err(CanvasQtiImportError::Manifest)
    ));
}

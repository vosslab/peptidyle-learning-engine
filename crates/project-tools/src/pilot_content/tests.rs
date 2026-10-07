use super::*;

const FIXTURE_QUESTION_IDS: [&str; 8] = [
    "A000-BCDE",
    "A001-0CDE",
    "A002-RCDE",
    "A003-KCDE",
    "A004-GCDE",
    "A005-6CDE",
    "A006-XCDE",
    "A007-GCDE",
];

#[test]
fn pilot_publication_preserves_explicit_source_formats() {
    let manifest_path = tracked_manifest_path().expect("tracked Pilot manifest");
    let mut manifest = read_manifest(&manifest_path).expect("Pilot manifest decodes");
    manifest.chapters[0].classification.validate().unwrap();
    let mut malformed = manifest.chapters[0].classification.clone();
    malformed.subject = " Genetics".to_owned();
    assert!(malformed.validate().is_err());
    malformed.subject = "Genetics".to_owned();
    malformed.subtopic = Some("X-linked crosses".to_owned());
    assert!(malformed.validate().is_err());
    let missing = classification_name_uuid(Vec::new(), "Subject", "Genetics")
        .unwrap_err()
        .to_string();
    assert!(missing.contains("provision the canonical vocabulary"));
    let webwork = &mut manifest.chapters[0].questions[0];
    assert_eq!(
        validated_question_format(webwork).unwrap(),
        QuestionFormat::WebworkPgml
    );
    webwork.source_format = Some(WebworkSourceFormat::Pg);
    assert!(validated_question_format(webwork).is_err());
    webwork.source.set_extension("pg");
    assert_eq!(
        validated_question_format(webwork).unwrap(),
        QuestionFormat::WebworkPg
    );
    webwork.source_format = None;
    assert!(validated_question_format(webwork).is_err());

    let native = &mut manifest.chapters[0].questions[2];
    assert_eq!(
        validated_question_format(native).unwrap(),
        QuestionFormat::PleQuestionJson
    );
    native.source_format = Some(WebworkSourceFormat::Pgml);
    assert!(validated_question_format(native).is_err());

    for source in publication_plan()
        .expect("Pilot publication plan")
        .questions
    {
        assert_eq!(
            source.question_format,
            match source.backend {
                Backend::Webwork => QuestionFormat::WebworkPgml,
                Backend::PleQuestionJson => QuestionFormat::PleQuestionJson,
            }
        );
    }
}

#[test]
fn pilot_manifest_metadata_allows_empty_tags_and_optional_language() {
    let manifest_path = tracked_manifest_path().expect("tracked Pilot manifest");
    let root = manifest_path.parent().expect("manifest parent");
    let manifest_text = std::fs::read_to_string(&manifest_path).expect("Pilot manifest reads");
    let mut manifest_value: serde_yaml_ng::Value =
        serde_yaml_ng::from_str(&manifest_text).expect("Pilot manifest YAML parses");
    manifest_value["chapters"][0]["questions"][2]
        .as_mapping_mut()
        .expect("native manifest entry")
        .remove(serde_yaml_ng::Value::String("language".to_owned()));
    let mut manifest: PilotManifest =
        serde_yaml_ng::from_value(manifest_value).expect("Pilot manifest decodes without language");
    let native = &mut manifest.chapters[0].questions[2];
    assert_eq!(native.language, None);
    native.tags.clear();

    validate_loaded_manifest(&manifest, root).expect("empty tags and absent language are valid");
    let metadata = publication_metadata(
        &manifest.chapters[0].questions[2],
        &manifest.source_project.content_license,
    )
    .expect("metadata plan is valid");
    assert!(metadata.tags.is_empty());
    assert_eq!(
        metadata.question_license,
        Some(question_model::QuestionLicense::CcBy4_0)
    );
    assert_eq!(metadata.question_citation, None);
    assert_eq!(metadata.language, None);
}

#[test]
fn pilot_publication_metadata_carries_supplied_values_into_the_plan() {
    let manifest_path = tracked_manifest_path().expect("tracked Pilot manifest");
    let mut manifest = read_manifest(&manifest_path).expect("Pilot manifest decodes");
    let native = &mut manifest.chapters[0].questions[2];
    native.tags = vec!["reviewed-tag".to_owned(), "chapter-1".to_owned()];
    native.question_citation = Some("https://example.edu/source - Reviewed source".to_owned());
    native.language = Some("fr-CA".to_owned());

    let metadata = publication_metadata(native, &manifest.source_project.content_license)
        .expect("metadata plan is valid");
    assert_eq!(
        metadata.tags,
        vec![
            question_model::Tag::new("reviewed-tag".to_owned()),
            question_model::Tag::new("chapter-1".to_owned()),
        ]
    );
    assert_eq!(
        metadata.question_license,
        Some(question_model::QuestionLicense::CcBy4_0)
    );
    assert_eq!(metadata.question_citation, native.question_citation);
    assert_eq!(metadata.language.as_deref(), Some("fr-CA"));
}

#[test]
fn checksum_validation_requires_lowercase_sha256() {
    assert!(is_lower_hex(&"a".repeat(64), 64));
    assert!(!is_lower_hex(&"A".repeat(64), 64));
    assert!(!is_lower_hex(&"a".repeat(63), 64));
}

#[test]
fn publication_mapping_rejects_a_checksum_that_is_not_the_canonical_source() {
    let plan = publication_plan().expect("tracked Pilot content should load for publication");
    let mut mapping = serde_json::Map::new();
    for (index, question) in plan.questions.iter().enumerate() {
        mapping.insert(
            question.slug.clone(),
            serde_json::json!({
                "sourceSha256": question.source_sha256,
                "publishedQuestionRevisionTuple": {
                    "publishedQuestionId": FIXTURE_QUESTION_IDS[index],
                    "revisionNumber": 1,
                },
            }),
        );
    }
    let valid = serde_json::to_string(&mapping).expect("mapping serializes");
    validate_publication_mapping_json(&valid).expect("plan-derived mapping is accepted");

    mapping
        .get_mut("genetics-disorders-ple-question-json-mc")
        .expect("known Pilot source")
        .as_object_mut()
        .expect("Pilot mapping entry")
        .insert("sourceSha256".to_owned(), serde_json::json!("0".repeat(64)));
    let tampered = serde_json::to_string(&mapping).expect("mapping serializes");
    assert!(validate_publication_mapping_json(&tampered).is_err());
}

#[test]
fn validated_mapping_selects_the_four_ple_question_json_revisions_in_plan_order() {
    let plan = publication_plan().expect("tracked Pilot content should load for publication");
    let mapping = plan
        .questions
        .iter()
        .enumerate()
        .map(|(index, question)| {
            (
                question.slug.clone(),
                serde_json::json!({
                    "sourceSha256": question.source_sha256,
                    "publishedQuestionRevisionTuple": {
                    "publishedQuestionId": FIXTURE_QUESTION_IDS[index],
                        "revisionNumber": 1,
                    },
                }),
            )
        })
        .collect::<serde_json::Map<_, _>>();
    let value = serde_json::to_string(&mapping).expect("mapping serializes");

    let selected = validated_ple_question_json_revisions(&value)
        .expect("approved PLE Question JSON publications resolve");

    assert_eq!(selected.len(), 4);
    assert_eq!(selected[0].published_question_id.to_string(), "A002-RCDE");
    assert_eq!(selected[3].published_question_id.to_string(), "A007-GCDE");
}

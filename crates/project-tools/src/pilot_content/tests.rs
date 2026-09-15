use super::*;

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
                "questionRevision": {
                    "questionId": format!("A{:02}-BCDE", index),
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
                    "questionRevision": {
                        "questionId": format!("A{index:02}-BCDE"),
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
    assert_eq!(selected[0].question_id.to_string(), "A02B-XCDE");
    assert_eq!(selected[3].question_id.to_string(), "A07B-XCDE");
}

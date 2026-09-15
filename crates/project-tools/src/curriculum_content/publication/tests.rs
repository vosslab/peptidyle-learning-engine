use super::*;
use question_model::{QuestionId, QuestionRevisionNumber};

fn accepted_replacement() -> ParameterizedSource {
    ParameterizedSource {
        source_id: "topic-canonical".to_owned(),
        topic_slug: "topic".to_owned(),
        question_title: "Canonical Question".to_owned(),
        question_description: "Canonical Question description".to_owned(),
        question_type: CurriculumQuestionType::MultipleChoice,
        source_format: super::super::WebworkSourceFormat::Pgml,
        replaces_static_bank_slug: Some("bank".to_owned()),
        pg_source: "pg/genetics/parameterized/topic/canonical.pgml".into(),
        pg_sha256: "c".repeat(64),
        webwork_pg_path: "genetics/parameterized/topic/canonical.pgml".to_owned(),
        canonical_author_source_url: "https://github.com/vosslab/example/blob/0123456789abcdef0123456789abcdef01234567/source.pgml".to_owned(),
        canonical_author_source_sha256: "d".repeat(64),
        content_license: "CC-BY-4.0".to_owned(),
        source_code_license: "LGPL-3.0-or-later".to_owned(),
    }
}

fn reference(question_id: &str, revision_number: u32) -> QuestionRevisionReference {
    QuestionRevisionReference {
        question_id: question_id
            .parse::<QuestionId>()
            .expect("valid Question ID"),
        revision_number: QuestionRevisionNumber::new(revision_number)
            .expect("positive Question Revision Number"),
    }
}

fn receipt_manifest() -> Manifest {
    Manifest {
        version: 1,
        course: Course {
            short_name: "Genetics".to_owned(),
            long_name: "Genetics Blueprint".to_owned(),
            module_label: "Genetics".to_owned(),
            source_repository: "owner/repository".to_owned(),
            source_revision: "abc123".to_owned(),
            author: "Author".to_owned(),
            content_license: "CC-BY-4.0".to_owned(),
        },
        parameterized_sources: Vec::new(),
        topics: vec![Topic {
            slug: "topic".to_owned(),
            title: "Topic title".to_owned(),
            instructions: "Instructions".to_owned(),
            banks: vec![super::super::Bank {
                slug: "bank".to_owned(),
                title: "Bank title".to_owned(),
                source: "bank.txt".into(),
                source_sha256: "b".repeat(64),
                selection_count: 1,
                rows: vec![
                    Row {
                        row_id: "first".to_owned(),
                        question_title: "First".to_owned(),
                        question_description: "First description".to_owned(),
                        question_type: CurriculumQuestionType::MultipleChoice,
                        pg_source: "first.pg".into(),
                        pg_sha256: "1".repeat(64),
                        webwork_pg_path: "topic/first.pg".to_owned(),
                    },
                    Row {
                        row_id: "second".to_owned(),
                        question_title: "Second".to_owned(),
                        question_description: "Second description".to_owned(),
                        question_type: CurriculumQuestionType::MultipleChoice,
                        pg_source: "second.pg".into(),
                        pg_sha256: "2".repeat(64),
                        webwork_pg_path: "topic/second.pg".to_owned(),
                    },
                ],
            }],
        }],
    }
}

#[test]
fn receipt_retains_source_provenance_and_exact_pool_pin_order() {
    let manifest = receipt_manifest();
    let first = reference("7K3M-X9QX", 1);
    let second = reference("8K3M-X9QX", 2);
    let published = BTreeMap::from([
        ("topic/bank/first".to_owned(), first.clone()),
        ("topic/bank/second".to_owned(), second.clone()),
    ]);

    let receipt = Receipt::new(
        "BP-TEST".to_owned(),
        1,
        &manifest,
        &published,
        &ReplacementRevisions::new(),
    )
    .expect("receipt from complete source map");
    let bank = &receipt.topics[0].banks[0];
    assert_eq!(bank.pool_question_revisions, vec![first, second]);
    assert_eq!(bank.source_path, "bank.txt");
    assert_eq!(bank.selection_count, 1);
    assert_eq!(bank.rows[0].row_id, "first");
    assert_eq!(bank.rows[0].source_path, "first.pg");
    assert_eq!(bank.rows[0].webwork_pg_path, "topic/first.pg");
    assert_eq!(bank.rows[1].pool_position, 1);
    assert_eq!(receipt.source_repository, "owner/repository");
    assert_eq!(receipt.source_revision, "abc123");
}

#[test]
fn exact_pool_pins_reject_a_newer_revision_of_the_same_question() {
    let revision_one = reference("7K3M-X9QX", 1);
    let revision_two = reference("7K3M-X9QX", 2);

    assert!(
        ensure_exact_pool_pins(
            std::slice::from_ref(&revision_one),
            std::slice::from_ref(&revision_one)
        )
        .is_ok()
    );
    assert!(ensure_exact_pool_pins(&[revision_two], &[revision_one]).is_err());
}

#[test]
fn retained_blueprint_rejects_a_newer_revision_of_the_same_question() {
    let manifest = receipt_manifest();
    let first = reference("7K3M-X9QX", 1);
    let second = reference("8K3M-X9QX", 2);
    let published = BTreeMap::from([
        ("topic/bank/first".to_owned(), first.clone()),
        ("topic/bank/second".to_owned(), second.clone()),
    ]);
    let input = blueprint_input(&manifest, &published, &ReplacementRevisions::new())
        .expect("valid Blueprint input");
    let pins = BTreeMap::from([
        (first.question_id.clone(), first.clone()),
        (second.question_id.clone(), second.clone()),
    ]);
    let mut actual = StoredBlueprintCourseContent::from_create(input.clone(), &pins)
        .expect("stored Blueprint content");
    let StoredBlueprintAssessmentEntry::Pool {
        question_revisions, ..
    } = &mut actual.modules[0].assessments[0].content.entries[0]
    else {
        panic!("fixture retains one Question Pool");
    };
    question_revisions[0] = reference("7K3M-X9QX", 2);

    assert!(
        validate_loaded_content(
            &actual,
            &input,
            Some(&manifest),
            &published,
            &ReplacementRevisions::new(),
            false,
        )
        .is_err()
    );
}

#[test]
fn accepted_replacement_requires_the_exact_static_pool_before_cas() {
    let mut manifest = receipt_manifest();
    let first = reference("7K3M-X9QX", 1);
    let second = reference("8K3M-X9QX", 2);
    let canonical = reference("9K3M-X9QX", 1);
    let published = BTreeMap::from([
        ("topic/bank/first".to_owned(), first.clone()),
        ("topic/bank/second".to_owned(), second.clone()),
    ]);
    let static_input = blueprint_input(&manifest, &published, &ReplacementRevisions::new())
        .expect("valid static Blueprint input");
    let pins = BTreeMap::from([
        (first.question_id.clone(), first.clone()),
        (second.question_id.clone(), second.clone()),
    ]);
    let mut actual = StoredBlueprintCourseContent::from_create(static_input, &pins)
        .expect("stored static Blueprint content");
    manifest.parameterized_sources.push(accepted_replacement());
    let replacements =
        BTreeMap::from([(("topic".to_owned(), "bank".to_owned()), canonical.clone())]);
    let replacement_input = blueprint_input(&manifest, &published, &replacements)
        .expect("valid canonical replacement input");

    assert_eq!(
        validate_loaded_content(
            &actual,
            &replacement_input,
            Some(&manifest),
            &published,
            &replacements,
            true,
        )
        .expect("exact static Pool is eligible for replacement"),
        true
    );
    let converted = StoredBlueprintCourseContent::from_create(
        replacement_input.clone(),
        &BTreeMap::from([(canonical.question_id.clone(), canonical.clone())]),
    )
    .expect("stored canonical replacement Blueprint content");
    assert!(
        !validate_loaded_content(
            &converted,
            &replacement_input,
            Some(&manifest),
            &published,
            &replacements,
            true,
        )
        .expect("already-converted canonical entry is idempotent")
    );
    let StoredBlueprintAssessmentEntry::Pool {
        selection_count, ..
    } = &mut actual.modules[0].assessments[0].content.entries[0]
    else {
        panic!("fixture retains one static Question Pool");
    };
    *selection_count = 2;
    assert!(
        validate_loaded_content(
            &actual,
            &replacement_input,
            Some(&manifest),
            &published,
            &replacements,
            true,
        )
        .is_err()
    );
}

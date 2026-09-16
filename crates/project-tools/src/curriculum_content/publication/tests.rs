use super::*;
use question_model::{QuestionId, QuestionRevisionNumber};

fn reference(question_id: &str, revision_number: u32) -> QuestionRevisionReference {
    QuestionRevisionReference {
        question_id: question_id
            .parse::<QuestionId>()
            .expect("valid Question ID"),
        revision_number: QuestionRevisionNumber::new(revision_number)
            .expect("positive Question Revision Number"),
    }
}

fn source(source_id: &str, topic_slug: &str) -> ParameterizedSource {
    ParameterizedSource {
        source_id: source_id.to_owned(),
        topic_slug: topic_slug.to_owned(),
        question_title: format!("Canonical {source_id}"),
        question_description: format!("Canonical description for {source_id}"),
        question_type: super::super::CurriculumQuestionType::MultipleChoice,
        source_format: super::super::WebworkSourceFormat::Pgml,
        pg_source: format!("pg/{topic_slug}/{source_id}.pgml").into(),
        pg_sha256: "c".repeat(64),
        webwork_pg_path: format!("genetics/{topic_slug}/{source_id}.pgml"),
        canonical_author_source_url: "https://github.com/vosslab/example/blob/0123456789abcdef0123456789abcdef01234567/source.pgml".to_owned(),
        canonical_author_source_sha256: "d".repeat(64),
        content_license: "CC-BY-4.0".to_owned(),
        source_code_license: "LGPL-3.0-or-later".to_owned(),
    }
}

fn manifest() -> Manifest {
    Manifest {
        version: 1,
        course: Course {
            assessment_type: question_model::AssessmentType::PracticeQuestionAssignment,
            short_name: "Genetics".to_owned(),
            long_name: "Genetics Blueprint".to_owned(),
            module_label: "Genetics".to_owned(),
            source_repository: "owner/repository".to_owned(),
            source_revision: "abc123".to_owned(),
            author: "Author".to_owned(),
            content_license: "CC-BY-4.0".to_owned(),
        },
        parameterized_sources: vec![source("first", "topic"), source("second", "topic")],
        topics: vec![super::super::Topic {
            slug: "topic".to_owned(),
            title: "Topic title".to_owned(),
            instructions: "Instructions".to_owned(),
            source_ids: vec!["first".to_owned(), "second".to_owned()],
        }],
    }
}

#[test]
fn canonical_blueprint_uses_ordered_direct_fixed_questions() {
    let manifest = manifest();
    let first = reference("7K3M-X9QX", 1);
    let second = reference("8K3M-X9QX", 1);
    let revisions = BTreeMap::from([
        ("first".to_owned(), first.clone()),
        ("second".to_owned(), second.clone()),
    ]);
    let input = blueprint_input(&manifest, &revisions).expect("valid direct Fixed Blueprint");
    let entries = &input.modules[0].assessments[0].entries;
    assert_eq!(entries.len(), 2);
    assert!(
        entries
            .iter()
            .all(|entry| matches!(entry, BlueprintAssessmentEntryInput::Fixed(_)))
    );
    let BlueprintAssessmentEntryInput::Fixed(first_entry) = &entries[0] else {
        unreachable!();
    };
    let BlueprintAssessmentEntryInput::Fixed(second_entry) = &entries[1] else {
        unreachable!();
    };
    assert_eq!(first_entry.question_id, first.question_id);
    assert_eq!(second_entry.question_id, second.question_id);
}

#[test]
fn exact_replay_rejects_semantic_drift() {
    let manifest = manifest();
    let first = reference("7K3M-X9QX", 1);
    let second = reference("8K3M-X9QX", 1);
    let revisions = BTreeMap::from([
        ("first".to_owned(), first.clone()),
        ("second".to_owned(), second.clone()),
    ]);
    let input = blueprint_input(&manifest, &revisions).expect("valid direct Fixed Blueprint");
    let pins = BTreeMap::from([
        (first.question_id.clone(), first),
        (second.question_id.clone(), second),
    ]);
    let mut stored =
        StoredBlueprintCourseContent::from_create(input.clone(), &pins, &BTreeMap::new())
            .expect("stored canonical Blueprint content");
    validate_loaded_content(&stored, &input, &manifest, &revisions)
        .expect("exact canonical replay");
    let mut wrong_type = stored.clone();
    wrong_type.modules[0].assessments[0].content.assessment_type =
        question_model::AssessmentType::RegularAssignment;
    assert!(validate_loaded_content(&wrong_type, &input, &manifest, &revisions).is_err());
    let StoredBlueprintAssessmentEntry::Fixed {
        question_revision, ..
    } = &mut stored.modules[0].assessments[0].content.entries[0]
    else {
        unreachable!();
    };
    question_revision.revision_number =
        QuestionRevisionNumber::new(2).expect("positive Question Revision Number");
    assert!(validate_loaded_content(&stored, &input, &manifest, &revisions).is_err());
}

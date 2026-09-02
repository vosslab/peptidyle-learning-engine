use grading::QuestionGradingOutcome;
use objects::{ObjectAddress, ObjectStore, PutObject, memory::MemoryObjectStore};
use question_model::{
    ObjectId, QuestionRevisionReference, SourceObjectChecksum, SourceObjectReference,
    StudentResponse, Timestamp, response::ResponseItemReference,
};
use uuid::Uuid;

use super::{PleQuestionBackend, ResolvedPleQuestionJsonSource};
use crate::test_support::ple_question_json_single_choice_bytes;

fn question() -> question_model::QuestionRevision {
    let workspace = question_model::WorkspaceId::from_uuid(Uuid::from_u128(1));
    let document = crate::question_json::PleQuestionJsonDocument::parse(
        &ple_question_json_single_choice_bytes(),
    )
    .expect("tracked PLE Question JSON should parse");
    question_model::QuestionRevision::from_draft(
        document
            .compile(workspace)
            .expect("tracked PLE Question JSON should compile")
            .into_parts()
            .0,
        question_model::QuestionId::from_canonical_parts("ABCDEF", 'G').expect("Question ID"),
        question_model::QuestionRevisionNumber::new(1).expect("revision number"),
        None,
    )
    .expect("PLE fixture has valid backend fields")
}

#[tokio::test]
async fn resolved_question_json_issues_and_grades_from_its_immutable_source_bytes() {
    let question = question();
    let store = MemoryObjectStore::default();
    let source_object_reference = SourceObjectReference {
        object: ObjectId::from_uuid(Uuid::from_u128(901)),
    };
    let question_revision = QuestionRevisionReference {
        question_id: question.question_id.clone(),
        revision_number: question.revision_number,
    };
    let record = store
        .put(PutObject {
            address: ObjectAddress::QuestionSource {
                question_revision,
                object: source_object_reference.object,
            },
            bytes: ple_question_json_single_choice_bytes(),
            media_type: crate::question_json::PLE_QUESTION_JSON_MEDIA_TYPE.to_string(),
            created_at: Timestamp::from_unix_millis(1),
        })
        .await
        .expect("source should store");
    let source_object_checksum =
        SourceObjectChecksum::parse(record.sha256.to_string()).expect("canonical checksum");
    let source = ResolvedPleQuestionJsonSource::resolve(
        &store,
        &question,
        source_object_reference.clone(),
        source_object_checksum.clone(),
    )
    .await
    .expect("source should resolve into its exact Question Revision");
    let adapter = PleQuestionBackend::new();
    let issued = adapter
        .issue_question_json(
            &source,
            question_model::generation::QuestionSeed::new(1),
            &[],
        )
        .expect("source should issue");

    assert_eq!(
        issued.reproduction_details.source_object_reference,
        Some(source_object_reference)
    );
    assert_eq!(
        issued.reproduction_details.source_object_checksum,
        Some(source_object_checksum)
    );
    assert_eq!(
        adapter
            .grade_question_json(
                &source,
                question_model::generation::QuestionSeed::new(1),
                &issued.parameter_hash,
                &issued.reproduction_details,
                &[],
                &StudentResponse::MultipleChoice {
                    selected: vec![ResponseItemReference::new("blue")],
                },
            )
            .expect("source-derived key should grade"),
        QuestionGradingOutcome::Graded(question_model::GradingResult {
            points_earned: 1.0,
            points_possible: 1.0,
            correct: true,
        })
    );
}

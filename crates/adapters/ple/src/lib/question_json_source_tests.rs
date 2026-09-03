use objects::{ObjectAddress, ObjectStore, PutObject, memory::MemoryObjectStore};
use question_model::{
    ObjectId, QuestionId, QuestionRevisionNumber, QuestionRevisionReference, SourceObjectChecksum,
    SourceObjectReference, StudentResponse, Timestamp, generation::QuestionSeed,
    response::ResponseItemReference,
};
use uuid::Uuid;

use super::{PleQuestionBackend, ResolvedPleQuestionJsonSource};
use crate::test_support::ple_question_json_single_choice_bytes;

fn question_revision() -> QuestionRevisionReference {
    QuestionRevisionReference {
        question_id: QuestionId::from_canonical_parts("ABCDEF", 'G').expect("Question ID"),
        revision_number: QuestionRevisionNumber::new(1).expect("revision number"),
    }
}

#[tokio::test]
async fn resolved_question_json_issues_and_grades_from_its_exact_immutable_source() {
    let store = MemoryObjectStore::default();
    let question_revision = question_revision();
    let source_object_reference = SourceObjectReference {
        object: ObjectId::from_uuid(Uuid::from_u128(901)),
    };
    let record = store
        .put(PutObject {
            address: ObjectAddress::QuestionSource {
                question_revision: question_revision.clone(),
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
        question_revision.clone(),
        source_object_reference.clone(),
        source_object_checksum.clone(),
    )
    .await
    .expect("source should resolve");
    let seed = QuestionSeed::new(1);
    let issued = PleQuestionBackend::new()
        .issue_question_json(&source, seed)
        .expect("source should issue");

    assert_eq!(source.question_revision(), &question_revision);
    assert_eq!(
        issued.presentation.variation.question_revision,
        question_revision
    );
    assert_eq!(issued.presentation.variation.question_seed, seed);
    assert_eq!(
        issued.reproduction_details.source_object_reference,
        Some(source_object_reference)
    );
    assert_eq!(
        issued.reproduction_details.source_object_checksum,
        Some(source_object_checksum)
    );
    let evaluation = PleQuestionBackend::new()
        .grade_question_json(
            &source,
            &StudentResponse::MultipleChoice {
                selected: vec![ResponseItemReference::new("blue")],
            },
        )
        .expect("source-derived answer should evaluate");
    assert!(evaluation.evaluation.correct());
    assert_eq!(evaluation.evaluation.normalized_credit(), 1.0);
}

use question_model::generation::QuestionSeed;

use crate::{PleQuestionBackend, PleQuestionBackendError};

use super::tests::{ple_question_json_question, source_object_checksum, source_object_reference};

#[test]
fn issue_records_required_source_evidence_and_replay_refuses_its_absence() {
    let adapter = PleQuestionBackend::new();
    let question = ple_question_json_question();
    let issued = adapter
        .issue(
            &question,
            QuestionSeed::new(82),
            &source_object_reference(),
            &source_object_checksum(),
            &[],
        )
        .expect("trusted source evidence should issue");
    assert_eq!(
        issued.reproduction_details.source_object_reference,
        Some(source_object_reference())
    );
    assert_eq!(
        issued.reproduction_details.source_object_checksum,
        Some(source_object_checksum())
    );

    let mut missing_reference = issued.reproduction_details.clone();
    missing_reference.source_object_reference = None;
    assert!(matches!(
        adapter.reproduce(
            &question,
            QuestionSeed::new(82),
            &issued.parameter_hash,
            &missing_reference,
            &[]
        ),
        Err(PleQuestionBackendError::ReproductionMismatch {
            field: "sourceObjectReference"
        })
    ));

    let mut missing_checksum = issued.reproduction_details.clone();
    missing_checksum.source_object_checksum = None;
    assert!(matches!(
        adapter.reproduce(
            &question,
            QuestionSeed::new(82),
            &issued.parameter_hash,
            &missing_checksum,
            &[]
        ),
        Err(PleQuestionBackendError::ReproductionMismatch {
            field: "sourceObjectChecksum"
        })
    ));
}

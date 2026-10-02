use objects::{ObjectAddress, ObjectStore, PutObject, memory::MemoryObjectStore};
use question_model::{
    AuthorContentLibraryId, GradingResult, ObjectId, PublishedQuestionId,
    PublishedQuestionRevisionTuple, QuestionContentBlock, QuestionRevisionNumber,
    SourceObjectChecksum, StudentResponse, Timestamp,
    response::{QuestionResponseControl, ResponseItemId},
};
use uuid::Uuid;

use super::{PleQuestionBackend, ResolvedPleQuestionJsonSource};
use crate::test_support::ple_question_json_single_choice_bytes;

fn published_question_revision_tuple() -> PublishedQuestionRevisionTuple {
    PublishedQuestionRevisionTuple {
        published_question_id: PublishedQuestionId::from_random_identifier("ABCDEFG")
            .expect("Question ID"),
        revision_number: QuestionRevisionNumber::new(1).expect("revision number"),
    }
}

#[tokio::test]
async fn resolved_question_json_issues_and_grades_from_its_exact_immutable_source() {
    let store = MemoryObjectStore::default();
    let published_question_revision_tuple = published_question_revision_tuple();
    let source_object_id = ObjectId::from_uuid(Uuid::from_u128(901));
    let record = store
        .put(PutObject {
            address: ObjectAddress::QuestionSource {
                published_question_revision_tuple: published_question_revision_tuple.clone(),
                object_id: source_object_id,
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
        published_question_revision_tuple.clone(),
        source_object_id,
        source_object_checksum.clone(),
    )
    .await
    .expect("source should resolve");
    let issued = PleQuestionBackend::new()
        .issue_question_json(&source)
        .expect("source should issue");

    assert_eq!(
        source.published_question_revision_tuple(),
        &published_question_revision_tuple
    );
    assert_eq!(
        issued
            .presentation
            .variation
            .published_question_revision_tuple,
        published_question_revision_tuple
    );
    assert_eq!(
        issued.presentation.variation.reproduction,
        question_model::generation::QuestionReproduction::Static
    );
    assert_eq!(
        issued.reproduction_details.source_object_id,
        Some(source_object_id)
    );
    assert_eq!(
        issued.reproduction_details.source_object_checksum,
        Some(source_object_checksum)
    );
    let evaluation = PleQuestionBackend::new()
        .grade_question_json(
            &source,
            &StudentResponse::MultipleChoice {
                selected: vec![ResponseItemId::new("blue")],
            },
        )
        .expect("source-derived answer should evaluate");
    assert!(evaluation.evaluation.correct());
    assert_eq!(evaluation.evaluation.normalized_credit(), 1.0);
}

#[tokio::test]
async fn grading_and_correctness_decisions_remain_server_owned_and_independent_of_author_supplied_javascript()
 {
    // ASVS 1.3.2: author script is recorded source. Grading does not execute it.
    let author_script = "function grade() { RDKit.get_mol(\"CCO\"); return \"red\"; }";
    let mut document: serde_json::Value =
        serde_json::from_slice(&ple_question_json_single_choice_bytes()).expect("fixture");
    document["authorScript"] = serde_json::json!({
        "source": author_script,
        "libraries": ["rdkit"],
    });
    let bytes = serde_json::to_vec(&document).expect("authored source");
    let store = MemoryObjectStore::default();
    let published_question_revision_tuple = published_question_revision_tuple();
    let source_object_id = ObjectId::from_uuid(Uuid::from_u128(902));
    let record = store
        .put(PutObject {
            address: ObjectAddress::QuestionSource {
                published_question_revision_tuple: published_question_revision_tuple.clone(),
                object_id: source_object_id,
            },
            bytes,
            media_type: crate::question_json::PLE_QUESTION_JSON_MEDIA_TYPE.to_string(),
            created_at: Timestamp::from_unix_millis(1),
        })
        .await
        .expect("source should store");
    let source_object_checksum =
        SourceObjectChecksum::parse(record.sha256.to_string()).expect("canonical checksum");
    let source = ResolvedPleQuestionJsonSource::resolve(
        &store,
        published_question_revision_tuple,
        source_object_id,
        source_object_checksum,
    )
    .await
    .expect("source should resolve");
    let issued = PleQuestionBackend::new()
        .issue_question_json(&source)
        .expect("source should issue");
    let author_content = issued
        .presentation
        .author_content
        .as_ref()
        .expect("issued presentation retains the author script");
    assert_eq!(author_content.source(), author_script);
    assert_eq!(
        author_content.library_ids(),
        &[AuthorContentLibraryId::Rdkit]
    );

    let correct = PleQuestionBackend::new()
        .grade_question_json(
            &source,
            &StudentResponse::MultipleChoice {
                selected: vec![ResponseItemId::new("blue")],
            },
        )
        .expect("server answer key should evaluate");
    assert!(correct.evaluation.correct());
    assert_eq!(correct.evaluation.normalized_credit(), 1.0);

    let claimed_by_script = PleQuestionBackend::new()
        .grade_question_json(
            &source,
            &StudentResponse::MultipleChoice {
                selected: vec![ResponseItemId::new("red")],
            },
        )
        .expect("author script must not change server grading");
    assert!(!claimed_by_script.evaluation.correct());
    assert_eq!(claimed_by_script.evaluation.normalized_credit(), 0.0);
}

#[tokio::test]
async fn questions_are_strictly_and_deterministically_automated_grading_does_not_require_an_instructor()
 {
    let store = MemoryObjectStore::default();
    let published_question_revision_tuple = published_question_revision_tuple();
    let source_object_id = ObjectId::from_uuid(Uuid::from_u128(903));
    let record = store
        .put(PutObject {
            address: ObjectAddress::QuestionSource {
                published_question_revision_tuple: published_question_revision_tuple.clone(),
                object_id: source_object_id,
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
        published_question_revision_tuple,
        source_object_id,
        source_object_checksum,
    )
    .await
    .expect("source should resolve");
    let backend = PleQuestionBackend::new();
    let mut credits = Vec::new();
    for selected in ["blue", "blue", "red", "red"] {
        let evaluation = backend
            .grade_question_json(
                &source,
                &StudentResponse::MultipleChoice {
                    selected: vec![ResponseItemId::new(selected)],
                },
            )
            .expect("automated grading");
        credits.push(evaluation.evaluation.normalized_credit());
    }
    assert_eq!(credits, [1.0, 1.0, 0.0, 0.0]);
}

#[tokio::test]
async fn native_question_backend_owns_rendering_interaction_response_grading_feedback_and_state() {
    let store = MemoryObjectStore::default();
    let published_question_revision_tuple = published_question_revision_tuple();
    let source_object_id = ObjectId::from_uuid(Uuid::from_u128(904));
    let record = store
        .put(PutObject {
            address: ObjectAddress::QuestionSource {
                published_question_revision_tuple: published_question_revision_tuple.clone(),
                object_id: source_object_id,
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
        published_question_revision_tuple,
        source_object_id,
        source_object_checksum,
    )
    .await
    .expect("source should resolve");
    let backend = PleQuestionBackend::new();
    let issued = backend
        .issue_question_json(&source)
        .expect("native source should issue");

    assert_eq!(
        issued.presentation.prompt,
        vec![QuestionContentBlock::Text {
            markdown: "What is my favorite color?".to_string(),
        }]
    );
    assert_eq!(
        issued.presentation.response.control(),
        QuestionResponseControl::ChoiceSelection
    );
    assert_eq!(
        issued.presentation.variation.reproduction,
        question_model::generation::QuestionReproduction::Static
    );
    assert_eq!(issued.reproduction_details.backend.name, crate::ADAPTER_ID);
    assert_eq!(
        issued.reproduction_details.backend.version,
        crate::ADAPTER_VERSION
    );
    assert!(issued.reproduction_details.renderer_version.is_none());
    assert_eq!(issued.reproduction_details.grader.name, crate::GRADING_ID);

    let correct = backend
        .grade_question_json(
            &source,
            &StudentResponse::MultipleChoice {
                selected: vec![ResponseItemId::new("blue")],
            },
        )
        .expect("captured correct choice grades");
    assert!(correct.evaluation.correct());
    assert_eq!(correct.evaluation.normalized_credit(), 1.0);
    let incorrect = StudentResponse::MultipleChoice {
        selected: vec![ResponseItemId::new("red")],
    };
    let incorrect_evaluation = backend
        .grade_question_json(&source, &incorrect)
        .expect("captured incorrect choice grades");
    assert!(!incorrect_evaluation.evaluation.correct());
    assert_eq!(incorrect_evaluation.evaluation.normalized_credit(), 0.0);

    let projected = backend
        .project_recorded_question_json_teaching_content(
            &source,
            Some(&incorrect),
            Some(GradingResult {
                correct: false,
                points_earned: 0.0,
                points_possible: 1.0,
            }),
        )
        .expect("native feedback projects from the recorded outcome");
    let feedback = projected.question_feedback.expect("native feedback");
    assert_eq!(
        feedback.incorrect_feedback,
        Some(vec![QuestionContentBlock::Text {
            markdown: "Try thinking of a cool color.".to_string(),
        }])
    );
    assert!(question_model::QuestionBackend::Ple.is_supported_for_production());
    assert!(question_model::QuestionBackend::Webwork.is_supported_for_production());
    assert!(!question_model::QuestionBackend::Imathas.is_supported_for_production());
}

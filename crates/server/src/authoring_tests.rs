use super::*;
use crate::question_publication::QuestionIdIssuer;
use question_model::{QuestionLicense, QuestionMetadata, QuestionRevisionNumber};

const RECORD_METADATA: &str = r#""metadata":{"questionTitle":"Title","questionDescription":"Description","tags":[],"questionLicense":null,"questionCitation":null,"language":null}"#;

#[test]
fn same_lineage_publication_requires_a_server_validated_positive_parent_revision() {
    let issuer = RandomQuestionIdIssuer::new();
    let question_id = issuer.issue_question_id().expect("issued Question ID");

    assert_eq!(
        existing_parent_published_question_revision_tuple(question_id.to_string(), 1),
        Ok(PublishedQuestionRevisionTuple {
            published_question_id: question_id.clone(),
            revision_number: QuestionRevisionNumber::new(1)
                .expect("positive Question Revision Number"),
        })
    );
    assert!(existing_parent_published_question_revision_tuple(question_id.to_string(), 0).is_err());
    assert!(existing_parent_published_question_revision_tuple("0000-X000".to_string(), 1).is_err());
}

fn valid_revision_publication_metadata() -> QuestionMetadata {
    QuestionMetadata {
        question_title: "Valid title".into(),
        question_description: "Valid description".into(),
        tags: Vec::new(),
        question_license: Some(QuestionLicense::CcBy4_0),
        question_citation: None,
        language: None,
    }
}

#[test]
fn revision_publication_accepts_valid_metadata() {
    assert_eq!(
        validate_revision_publication_metadata(&valid_revision_publication_metadata()),
        Ok(())
    );
}

#[test]
fn revision_publication_requires_a_question_license() {
    let mut metadata = valid_revision_publication_metadata();
    metadata.question_license = None;

    assert_eq!(
        validate_revision_publication_metadata(&metadata),
        Err("Draft Question requires a Question License before publication")
    );
}

#[test]
fn revision_publication_requires_a_valid_title() {
    let mut metadata = valid_revision_publication_metadata();
    metadata.question_title = "   ".into();

    assert_eq!(
        validate_revision_publication_metadata(&metadata),
        Err("Draft Question requires a Title and Description before publication")
    );
}

#[test]
fn revision_publication_requires_a_valid_description() {
    let mut metadata = valid_revision_publication_metadata();
    metadata.question_description = "   ".into();

    assert_eq!(
        validate_revision_publication_metadata(&metadata),
        Err("Draft Question requires a Title and Description before publication")
    );
}

#[test]
fn metadata_question_type_preserves_omitted_null_and_manual_values() {
    let omitted: DraftGeneralFeedbackRequest =
        serde_json::from_str(&format!("{{{},\"generalFeedback\":null}}", RECORD_METADATA))
            .expect("omitted Question Type");
    assert_eq!(omitted.question_type, None);
    assert!(allows_question_type_write(
        QuestionBackend::Ple,
        omitted.question_type
    ));

    let cleared: DraftGeneralFeedbackRequest = serde_json::from_str(&format!(
        "{{{},\"questionType\":null,\"generalFeedback\":null}}",
        RECORD_METADATA
    ))
    .expect("explicit Type clear");
    assert_eq!(cleared.question_type, Some(None));
    assert!(!allows_question_type_write(
        QuestionBackend::Ple,
        cleared.question_type
    ));
    assert!(allows_question_type_write(
        QuestionBackend::Webwork,
        cleared.question_type
    ));

    let manual: DraftGeneralFeedbackRequest = serde_json::from_str(&format!(
        "{{{},\"questionType\":\"hotspot\",\"generalFeedback\":null}}",
        RECORD_METADATA
    ))
    .expect("manual WebWork Type");
    assert_eq!(manual.question_type, Some(Some(QuestionType::Hotspot)));
    assert!(!allows_question_type_write(
        QuestionBackend::Ple,
        manual.question_type
    ));
    assert!(allows_question_type_write(
        QuestionBackend::Webwork,
        manual.question_type
    ));
    assert!(
        serde_json::from_str::<DraftGeneralFeedbackRequest>(&format!(
            "{{{},\"questionType\":\"notAType\",\"generalFeedback\":null}}",
            RECORD_METADATA
        ))
        .is_err()
    );
}

#[test]
fn metadata_read_serializes_question_type_as_nullable() {
    let response = DraftGeneralFeedbackResponse {
        metadata: QuestionMetadata {
            question_title: "Title".into(),
            question_description: "Description".into(),
            tags: Vec::new(),
            question_license: None,
            question_citation: None,
            language: None,
        },
        question_type: None,
        general_feedback: None,
        hint: None,
        worked_solution: None,
        authors: Vec::new(),
    };
    let json = serde_json::to_value(response).expect("metadata response serializes");
    assert_eq!(json["questionType"], serde_json::Value::Null);
    assert_eq!(json["generalFeedback"], serde_json::Value::Null);
    assert_eq!(json["hint"], serde_json::Value::Null);
    assert_eq!(json["workedSolution"], serde_json::Value::Null);
}

#[test]
fn published_questions_include_optional_hint_feedback_and_worked_solution() {
    let feedback_only: DraftGeneralFeedbackRequest = serde_json::from_str(&format!(
        "{{{},\"generalFeedback\":\"Keep the units.\"}}",
        RECORD_METADATA
    ))
    .expect("feedback");
    let (hint, worked_solution, replace_support) =
        authored_support_replacement(feedback_only.hint, feedback_only.worked_solution)
            .expect("omitted support leaves the stored texts");
    assert!(hint.is_none());
    assert!(worked_solution.is_none());
    assert!(!replace_support);

    let support: DraftGeneralFeedbackRequest = serde_json::from_str(&format!(
        "{{{},\"generalFeedback\":\"Keep the units.\",\"hint\":\"Count alleles.\",\"workedSolution\":\"Show the cross.\"}}",
        RECORD_METADATA
    )).expect("support");
    let (hint, worked_solution, replace_support) =
        authored_support_replacement(support.hint, support.worked_solution)
            .expect("both support keys replace the stored texts");
    assert_eq!(hint.as_deref(), Some("Count alleles."));
    assert_eq!(worked_solution.as_deref(), Some("Show the cross."));
    assert!(replace_support);

    let cleared: DraftGeneralFeedbackRequest = serde_json::from_str(&format!(
        "{{{},\"generalFeedback\":null,\"hint\":null,\"workedSolution\":null}}",
        RECORD_METADATA
    ))
    .expect("clear");
    let (hint, worked_solution, replace_support) =
        authored_support_replacement(cleared.hint, cleared.worked_solution)
            .expect("present nulls clear both texts");
    assert!(hint.is_none());
    assert!(worked_solution.is_none());
    assert!(replace_support);

    let one_key: DraftGeneralFeedbackRequest = serde_json::from_str(&format!(
        "{{{},\"generalFeedback\":null,\"hint\":\"Count alleles.\"}}",
        RECORD_METADATA
    ))
    .expect("one key");
    assert!(authored_support_replacement(one_key.hint, one_key.worked_solution).is_err());
    assert!(
        serde_json::from_str::<DraftGeneralFeedbackRequest>(
            &format!("{{{},\"generalFeedback\":null,\"hint\":null,\"workedSolution\":null,\"source\":\"backend\"}}", RECORD_METADATA)
        )
        .is_err()
    );
    let metadata_request: DraftGeneralFeedbackRequest =
        serde_json::from_str(&format!("{{{},\"generalFeedback\":null}}", RECORD_METADATA))
            .expect("nullable language and incomplete record metadata are valid for Draft save");
    assert!(metadata_request.metadata.language.is_none());
    assert!(metadata_request.metadata.question_license.is_none());
}

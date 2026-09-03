use domain::draft_preview::{DraftPreviewRequest, DraftPreviewResult, preview_ple_draft};
use grading::GradingError;
use question_model::capability::{Capability, QuestionBackendCapabilities};
use question_model::generation::QuestionSeed;
use question_model::response::{QuestionResponseFormat, ResponseItemReference};
use question_model::{
    DraftQuestionContent, QuestionAssetId, QuestionBackend, QuestionId, QuestionRevision,
    QuestionRevisionNumber, SourceObjectChecksum, SourceObjectReference, StudentResponse,
    WorkspaceId,
};
use question_model::{
    QuestionAssetReference as PresentedQuestionAssetReference, QuestionContentBlock,
};
use uuid::Uuid;

use super::*;
use crate::test_support::ple_question_json_single_choice_bytes;

fn question_id() -> QuestionId {
    QuestionId::from_canonical_parts("ABCDEF", 'G').expect("Question ID")
}

fn revision_number(value: u32) -> QuestionRevisionNumber {
    QuestionRevisionNumber::new(value).expect("positive Question Revision Number")
}
pub(super) fn source_object_reference() -> SourceObjectReference {
    SourceObjectReference {
        object: question_model::ObjectId::from_uuid(Uuid::from_u128(900)),
    }
}
pub(super) fn source_object_checksum() -> SourceObjectChecksum {
    SourceObjectChecksum::parse("a".repeat(64)).expect("canonical source checksum")
}

pub(super) fn ple_question_json_question() -> QuestionRevision {
    let workspace = WorkspaceId::from_uuid(Uuid::from_u128(1));
    let document = crate::question_json::PleQuestionJsonDocument::parse(
        ple_question_json_single_choice_bytes().as_slice(),
    )
    .expect("stored PLE Question JSON fixture should parse");
    let draft = document
        .compile(workspace)
        .expect("PLE Question JSON fixture should compile")
        .into_parts()
        .0;
    QuestionRevision::from_draft(draft, question_id(), revision_number(1), None)
        .expect("PLE fixture has valid backend fields")
}

#[test]
fn an_implementation_without_an_author_presentation_is_honestly_unavailable() {
    let adapter = PleQuestionBackend::new();
    let question = ple_question_json_question();
    let draft = DraftQuestionContent {
        workspace: question.workspace,
        question_backend: QuestionBackend::Ple,
        webwork_pg_path: None,
        qti_package_item_identifier: None,
        workspace_import_id: None,
        draft_imathas_question_backend_binding: None,
        question_format: question.question_format,
        prompt: question.prompt,
        response: question.response,
        question_type: question.question_type,
        question_attempt_limit: question.question_attempt_limit,
        question_attempt_time_limit: question.question_attempt_time_limit,
        grading: question.grading,
        metadata: question.metadata,
    };

    assert!(
        adapter
            .author_presentation(&draft)
            .expect("the default author-presentation implementation is safe")
            .is_none(),
        "Question Implementations opt in explicitly; the engine never serializes a grading key as a fallback"
    );
}

#[test]
fn ple_question_json_capabilities_are_installed_and_reproducible_without_answer_keys() {
    let adapter = PleQuestionBackend::new();
    let question = ple_question_json_question();
    let expected = QuestionBackendCapabilities::from_iter([
        Capability::ClientRendering,
        Capability::ServerGrading,
        Capability::Hints,
        Capability::QuestionAttemptTimeLimit,
    ]);

    assert_eq!(
        adapter
            .capabilities(&question)
            .expect("implementation is installed"),
        expected
    );
    let issue = adapter
        .issue(
            &question,
            QuestionSeed::new(10),
            &source_object_reference(),
            &source_object_checksum(),
            &[],
        )
        .expect("PLE Question JSON Implementation issue should be key free");
    let replay = adapter
        .reproduce(
            &question,
            QuestionSeed::new(10),
            &issue.reproduction_details,
            &[],
        )
        .expect("PLE Question JSON issue should reproduce exactly");

    assert_eq!(issue.presentation, replay);
    let public = serde_json::to_string(&issue.presentation)
        .expect("issued Question Presentation should serialize for student");
    assert!(!public.contains("correctChoice"));
    assert!(!public.contains("publicContentChecksum"));
}

#[test]
fn ple_question_json_grading_refuses_without_server_persisted_answer_key() {
    let adapter = PleQuestionBackend::new();
    let question = ple_question_json_question();
    let issue = adapter
        .issue(
            &question,
            QuestionSeed::new(11),
            &source_object_reference(),
            &source_object_checksum(),
            &[],
        )
        .expect("PLE Question JSON issue should deliver a reproducible Question Presentation");

    assert!(matches!(
        adapter.grade(
            &question,
            QuestionSeed::new(11),
            &issue.reproduction_details,
            &[],
            &StudentResponse::MultipleChoice {
                selected: vec![ResponseItemReference::new("blue")],
            },
        ),
        Err(PleQuestionBackendError::Grading(
            GradingError::MissingAnswerKey
        ))
    ));
}

#[test]
fn ple_draft_preview_matches_the_published_question_presentation() {
    let adapter = PleQuestionBackend::new();
    let question = ple_question_json_question();
    let seed = QuestionSeed::new(37);
    let issued = adapter
        .issue(
            &question,
            seed,
            &source_object_reference(),
            &source_object_checksum(),
            &[],
        )
        .expect("PLE issue");
    let preview = preview_ple_draft(&DraftPreviewRequest {
        workspace: question.workspace,
        question_backend: QuestionBackend::Ple,
        title: question.metadata.title.clone(),
        prompt: question.prompt.clone(),
        response: question.response.clone(),
    });
    let DraftPreviewResult::Ready { preview } = preview else {
        panic!("PLE previews locally")
    };
    assert_eq!(preview.title, issued.presentation.title);
    assert_eq!(preview.prompt, issued.presentation.prompt);
    assert_eq!(preview.response, issued.presentation.response);
}

#[test]
fn altered_question_attempt_reproduction_details_are_refused_before_grading() {
    let adapter = PleQuestionBackend::new();
    let question = ple_question_json_question();
    let issued = adapter
        .issue(
            &question,
            QuestionSeed::new(5),
            &source_object_reference(),
            &source_object_checksum(),
            &[],
        )
        .expect("valid peptide question should issue");
    let mut altered = issued.reproduction_details;
    altered.grader.version = "different".to_string();

    assert!(matches!(
        adapter.grade(
            &question,
            QuestionSeed::new(5),
            &altered,
            &[],
            &StudentResponse::MultipleChoice {
                selected: vec![ResponseItemReference::new("blue")],
            },
        ),
        Err(PleQuestionBackendError::UnknownQuestionGraderVersion { .. })
    ));
}

#[test]
fn uninstalled_execution_versions_are_refused_before_issue_or_grading() {
    let mut adapter = PleQuestionBackend::new();
    assert!(matches!(
        adapter.select_current_versions(
            backend_version(ADAPTER_ID, "2"),
            grader_version(GRADING_ID, GRADING_VERSION),
        ),
        Err(PleQuestionBackendError::UnknownQuestionBackendVersion { .. })
    ));
}

fn asset(id: u128) -> QuestionAssetId {
    QuestionAssetId::from_uuid(Uuid::from_u128(id))
}

fn image(asset: QuestionAssetId) -> QuestionContentBlock {
    QuestionContentBlock::Image {
        asset: PresentedQuestionAssetReference {
            asset,
            checksum: "fixture-checksum".to_string(),
        },
        description: "A trusted fixture image.".to_string(),
    }
}

#[test]
fn question_asset_object_references_require_exact_complete_trusted_records() {
    let adapter = PleQuestionBackend::new();
    let mut question = ple_question_json_question();
    let rendered_asset = asset(81);
    question.prompt.push(image(rendered_asset));
    let question_asset_object_references = [QuestionAssetObjectReference {
        question_asset: rendered_asset,
        object_reference: ObjectId::from_uuid(Uuid::from_u128(82)),
    }];
    let issued = adapter
        .issue(
            &question,
            QuestionSeed::new(82),
            &source_object_reference(),
            &source_object_checksum(),
            &question_asset_object_references,
        )
        .expect("trusted assets should be recorded at issue time");
    assert_eq!(
        issued.reproduction_details.asset_objects,
        vec![question_asset_object_references[0].object_reference]
    );

    assert!(matches!(
        adapter.issue(&question, QuestionSeed::new(82), &source_object_reference(), &source_object_checksum(), &[]),
        Err(PleQuestionBackendError::MissingAssetBinding(found)) if found == rendered_asset
    ));
    assert!(matches!(
        adapter.issue(
            &ple_question_json_question(),
            QuestionSeed::new(82),
            &source_object_reference(),
            &source_object_checksum(),
            &question_asset_object_references,
        ),
        Err(PleQuestionBackendError::UnrelatedAssetBinding(found)) if found == rendered_asset
    ));
    assert!(matches!(
        adapter.issue(
            &question,
            QuestionSeed::new(82),
            &source_object_reference(),
            &source_object_checksum(),
            &[
                question_asset_object_references[0],
                QuestionAssetObjectReference {
                    question_asset: rendered_asset,
                    object_reference: ObjectId::from_uuid(Uuid::from_u128(83)),
                },
            ],
        ),
        Err(PleQuestionBackendError::ConflictingAssetBinding(found)) if found == rendered_asset
    ));

    let mut altered_reproduction_details = issued.reproduction_details.clone();
    altered_reproduction_details.asset_objects = vec![ObjectId::from_uuid(Uuid::from_u128(84))];

    assert!(matches!(
        adapter.reproduce(
            &question,
            QuestionSeed::new(82),
            &altered_reproduction_details,
            &question_asset_object_references,
        ),
        Err(PleQuestionBackendError::ReproductionMismatch {
            field: "assetObjects"
        })
    ));
}

#[test]
fn nested_response_assets_are_bound_in_canonical_logical_asset_order() {
    let adapter = PleQuestionBackend::new();
    let mut question = ple_question_json_question();
    let prompt_asset = asset(91);
    let response_asset = asset(90);
    question.prompt.push(image(prompt_asset));
    let QuestionResponseFormat::MultipleChoice { choices, .. } = &mut question.response else {
        panic!("stored PLE Question JSON fixture has multiple-choice response")
    };
    choices[0].body.push(image(response_asset));
    let question_asset_object_references = [
        QuestionAssetObjectReference {
            question_asset: prompt_asset,
            object_reference: ObjectId::from_uuid(Uuid::from_u128(191)),
        },
        QuestionAssetObjectReference {
            question_asset: response_asset,
            object_reference: ObjectId::from_uuid(Uuid::from_u128(190)),
        },
    ];

    let issued = adapter
        .issue(
            &question,
            QuestionSeed::new(91),
            &source_object_reference(),
            &source_object_checksum(),
            &question_asset_object_references,
        )
        .expect("prompt and nested response assets should resolve");

    assert_eq!(
        issued.reproduction_details.asset_objects,
        vec![
            question_asset_object_references[1].object_reference,
            question_asset_object_references[0].object_reference,
        ],
        "asset IDs, not caller order, canonically order persisted objects"
    );
}

#[test]
fn rendered_question_presentation_hash_has_a_fixed_compatibility_vector() {
    let adapter = PleQuestionBackend::new();
    let issued = adapter
        .issue(
            &ple_question_json_question(),
            QuestionSeed::new(37),
            &source_object_reference(),
            &source_object_checksum(),
            &[],
        )
        .expect("fixed vector should issue");
    assert_eq!(
        issued.reproduction_details.rendered_question_sha256,
        "62c548e1ff767a76ea4e93ba42c09c3c4bf1a8ca9d7d369be6c58e22aa936088"
    );
}

#[test]
fn historical_blank_or_oversized_titles_are_refused_before_issue() {
    let adapter = PleQuestionBackend::new();
    for title in [" \t".to_string(), "\u{1F9EC}".repeat(513)] {
        let mut question = ple_question_json_question();
        question.metadata.title = title;
        assert!(matches!(
            adapter.issue(
                &question,
                QuestionSeed::new(37),
                &source_object_reference(),
                &source_object_checksum(),
                &[]
            ),
            Err(PleQuestionBackendError::InvalidTitle(_))
        ));
    }
}

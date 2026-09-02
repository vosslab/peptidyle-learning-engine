use std::collections::BTreeMap;

use domain::draft_preview::{DraftPreviewRequest, DraftPreviewResult, preview_ple_draft};
use domain::generator::QuestionVariationParameters;
use domain::student_feedback_release::{StudentFeedbackReleaseDecision, project_student_feedback};
use grading::{AnswerKey, GradingError};
use question_model::answer::NumericResponseTolerance;
use question_model::assignment_activity_rules::{QuestionAttemptLimit, QuestionAttemptTimeLimit};
use question_model::capability::{Capability, QuestionBackendCapabilities};
use question_model::classification::QuestionLicense;
use question_model::generation::{QuestionGeneratorReference, QuestionSeed, QuestionVariationRule};
use question_model::response::{QuestionResponseFormat, ResponseItemReference};
use question_model::{
    DraftQuestionBackendLocator, DraftQuestionContent, QuestionAnswer, QuestionAnswerExplanation,
    QuestionAssetId, QuestionBackendLocator, QuestionFeedback, QuestionFormat, QuestionGradingRule,
    QuestionId, QuestionMetadata, QuestionRevision, QuestionRevisionNumber, QuestionType,
    SourceObjectChecksum, SourceObjectReference, StudentResponse, WorkspaceId,
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
    QuestionRevision::from_draft(
        draft,
        question_id(),
        revision_number(1),
        QuestionBackendLocator::Ple,
    )
}

fn metadata(title: &str) -> QuestionMetadata {
    QuestionMetadata {
        title: title.to_string(),
        question_description: format!("Instructor-facing summary for {title}."),
        tags: Vec::new(),
        classifications: Vec::new(),
        question_license: Some(QuestionLicense::CcBySa4_0),
        question_citation: None,
        language: "en-US".to_string(),
    }
}

#[test]
fn an_implementation_without_an_author_presentation_is_honestly_unavailable() {
    let mut adapter = PleQuestionBackend::empty();
    adapter
        .register_implementation(NumericReferenceImplementation)
        .expect("the test implementation is unique");
    let question = ple_question_json_question();
    let mut draft = DraftQuestionContent {
        workspace: question.workspace,
        backend_locator: DraftQuestionBackendLocator::Ple,
        question_format: question.question_format,
        prompt: question.prompt,
        response: question.response,
        question_type: question.question_type,
        question_attempt_limit: question.question_attempt_limit,
        question_attempt_time_limit: question.question_attempt_time_limit,
        question_variation_rule: question.question_variation_rule,
        grading: question.grading,
        metadata: question.metadata,
    };
    draft.question_format = QuestionFormat::PleAlgorithmic;
    draft.question_type = QuestionType::Numeric;
    draft.question_variation_rule = QuestionVariationRule::Static;
    draft.prompt = vec![QuestionContentBlock::Text {
        markdown: "Enter the reference value.".to_string(),
    }];

    assert!(
        adapter
            .author_presentation(&draft, QuestionSeed::new(4))
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
            &issue.parameter_hash,
            &issue.reproduction_details,
            &[],
        )
        .expect("PLE Question JSON issue should reproduce exactly");

    assert_eq!(issue.presentation, replay);
    let public = serde_json::to_string(&issue.presentation)
        .expect("issued Question Presentation should serialize for student");
    assert!(!public.contains("correctChoice"));
    assert!(!public.contains("publicSha256"));
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
            &issue.parameter_hash,
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
    let preview = preview_ple_draft(
        &DraftPreviewRequest {
            workspace: question.workspace,
            backend_locator: DraftQuestionBackendLocator::Ple,
            title: question.metadata.title.clone(),
            prompt: question.prompt.clone(),
            response: question.response.clone(),
            question_variation_rule: question.question_variation_rule.clone(),
        },
        seed,
    )
    .expect("PLE preview");
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
            &issued.parameter_hash,
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
            &issued.parameter_hash,
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
        "c0e7badee61758c5bc1fb2ea0e99244847424c4565df07fefb570f872a577f4f"
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

#[derive(Debug, Clone, Copy)]
struct NumericReferenceImplementation;

impl PleQuestionImplementation for NumericReferenceImplementation {
    fn question_format(&self) -> QuestionFormat {
        QuestionFormat::PleAlgorithmic
    }

    fn question_type(&self) -> QuestionType {
        QuestionType::Numeric
    }

    fn implementation_release(&self) -> crate::generator::PleQuestionImplementationRelease {
        crate::generator::PleQuestionImplementationRelease {
            id: "numeric-reference".to_string(),
            version: "1".to_string(),
        }
    }

    fn generator(&self) -> Option<QuestionGeneratorReference> {
        None
    }

    fn capabilities(&self) -> QuestionBackendCapabilities {
        QuestionBackendCapabilities::from_iter([
            Capability::ClientRendering,
            Capability::ServerGrading,
        ])
    }

    fn derive_answer_key(
        &self,
        question: &QuestionRevision,
        _generated: &QuestionVariationParameters,
    ) -> Result<Option<AnswerKey>, PleQuestionBackendError> {
        if !matches!(question.response, QuestionResponseFormat::Numeric { .. }) {
            return Err(
                PleQuestionBackendError::IncompatibleQuestionImplementation {
                    message: "numeric response required".to_string(),
                },
            );
        }
        Ok(Some(AnswerKey::Numeric { expected: 7.0 }))
    }

    fn derive_question_feedback_answer_and_explanation(
        &self,
        _question: &QuestionRevision,
        _generated: &QuestionVariationParameters,
        _presentation: &question_model::QuestionVariationPresentation,
        _answer_key: Option<&AnswerKey>,
        _result: &question_model::GradingResult,
        _response: &StudentResponse,
    ) -> Result<
        (
            QuestionFeedback,
            Option<QuestionAnswer>,
            Option<QuestionAnswerExplanation>,
        ),
        PleQuestionBackendError,
    > {
        Ok((
            QuestionFeedback {
                choice_feedback: Some(vec![QuestionContentBlock::Text {
                    markdown: "Feedback value.".to_string(),
                }]),
                correct_feedback: None,
                incorrect_feedback: None,
            },
            QuestionAnswer::new(vec![QuestionContentBlock::Text {
                markdown: "Answer value.".to_string(),
            }]),
            QuestionAnswerExplanation::new(vec![QuestionContentBlock::Text {
                markdown: "Explanation value.".to_string(),
            }]),
        ))
    }
}

#[test]
fn verified_grading_keeps_question_feedback_answer_and_explanation_distinct_for_student_release() {
    let mut adapter = PleQuestionBackend::empty();
    adapter
        .register_implementation(NumericReferenceImplementation)
        .expect("test implementation should register");
    let question = QuestionRevision {
        question_id: question_id(),
        revision_number: revision_number(4),
        workspace: WorkspaceId::from_uuid(Uuid::from_u128(4)),
        backend_locator: QuestionBackendLocator::Ple,
        question_format: QuestionFormat::PleAlgorithmic,
        prompt: vec![QuestionContentBlock::Text {
            markdown: "Enter the reference value.".to_string(),
        }],
        response: QuestionResponseFormat::Numeric {
            tolerance: NumericResponseTolerance::Exact,
            unit: None,
        },
        question_type: QuestionType::Numeric,
        question_attempt_limit: QuestionAttemptLimit {
            max_attempts: Some(1),
        },
        question_attempt_time_limit: QuestionAttemptTimeLimit::Unlimited,
        question_variation_rule: QuestionVariationRule::Static,
        grading: QuestionGradingRule::AllOrNothing { points: 1.0 },
        metadata: metadata("Release roles"),
    };
    let issued = adapter
        .issue(
            &question,
            QuestionSeed::new(12),
            &source_object_reference(),
            &source_object_checksum(),
            &[],
        )
        .expect("verified PLE Question should issue");
    let evaluation = adapter
        .grade_with_feedback_answer_and_explanation(
            &question,
            QuestionSeed::new(12),
            &issued.parameter_hash,
            &issued.reproduction_details,
            &[],
            &StudentResponse::Numeric { value: 7.0 },
        )
        .expect("verified PLE Question should grade");
    let grading::QuestionGradingOutcome::Graded(result) = evaluation.outcome else {
        panic!("the numeric response should produce a grading result")
    };
    let disclosed = project_student_feedback(
        StudentFeedbackReleaseDecision {
            score: false,
            per_item_correctness: false,
            feedback_text: true,
            question_answer: true,
            question_answer_explanation: true,
            class_statistics: false,
        },
        Some(result),
        &evaluation.question_feedback,
        evaluation.question_answer.as_ref(),
        evaluation.question_answer_explanation.as_ref(),
    )
    .expect("the release decision exposes each selected teaching role");

    assert_eq!(
        disclosed.choice_feedback,
        Some(vec![QuestionContentBlock::Text {
            markdown: "Feedback value.".to_string(),
        }])
    );
    assert_eq!(
        disclosed.question_answer,
        Some(vec![QuestionContentBlock::Text {
            markdown: "Answer value.".to_string(),
        }])
    );
    assert_eq!(
        disclosed.question_answer_explanation,
        Some(vec![QuestionContentBlock::Text {
            markdown: "Explanation value.".to_string(),
        }])
    );
}

#[test]
fn a_second_implementation_plugs_into_the_registry_without_engine_changes() {
    let mut adapter = PleQuestionBackend::empty();
    adapter
        .register_implementation(NumericReferenceImplementation)
        .expect("new implementation identifier should register");
    let question = QuestionRevision {
        question_id: question_id(),
        revision_number: revision_number(3),
        workspace: WorkspaceId::from_uuid(Uuid::from_u128(4)),
        backend_locator: QuestionBackendLocator::Ple,
        question_format: QuestionFormat::PleAlgorithmic,
        prompt: vec![QuestionContentBlock::Text {
            markdown: "Enter the reference value.".to_string(),
        }],
        response: QuestionResponseFormat::Numeric {
            tolerance: NumericResponseTolerance::Exact,
            unit: None,
        },
        question_type: QuestionType::Numeric,
        question_attempt_limit: QuestionAttemptLimit {
            max_attempts: Some(1),
        },
        question_attempt_time_limit: QuestionAttemptTimeLimit::Unlimited,
        question_variation_rule: QuestionVariationRule::Static,
        grading: QuestionGradingRule::AllOrNothing { points: 1.0 },
        metadata: metadata("Numeric registry extension"),
    };
    let issued = adapter
        .issue(
            &question,
            QuestionSeed::new(123),
            &source_object_reference(),
            &source_object_checksum(),
            &[],
        )
        .expect("registered Question Implementation should issue through the generic adapter");

    assert!(
        adapter
            .capabilities(&question)
            .expect("registered source should expose capabilities")
            .supports(Capability::ServerGrading)
    );
    assert!(matches!(
        adapter.grade(
            &question,
            QuestionSeed::new(123),
            &issued.parameter_hash,
            &issued.reproduction_details,
            &[],
            &StudentResponse::Numeric { value: 7.0 },
        ),
        Ok(QuestionGradingOutcome::Graded(result)) if result.correct
    ));
}

#[derive(Debug, Clone, Copy)]
struct VersionedNumericImplementation {
    version: &'static str,
    implementation_release: &'static str,
    expected: f64,
    supports_client_rendering: bool,
}

impl PleQuestionImplementation for VersionedNumericImplementation {
    fn question_format(&self) -> QuestionFormat {
        QuestionFormat::PleAlgorithmic
    }

    fn question_type(&self) -> QuestionType {
        QuestionType::Numeric
    }

    fn implementation_release(&self) -> crate::generator::PleQuestionImplementationRelease {
        crate::generator::PleQuestionImplementationRelease {
            id: "versioned-numeric".to_string(),
            version: self.implementation_release.to_string(),
        }
    }

    fn generator(&self) -> Option<QuestionGeneratorReference> {
        Some(QuestionGeneratorReference {
            id: "versioned-numeric-generator".to_string(),
            version: self.version.to_string(),
        })
    }

    fn capabilities(&self) -> QuestionBackendCapabilities {
        let mut capabilities = vec![Capability::ServerGrading];
        if self.supports_client_rendering {
            capabilities.push(Capability::ClientRendering);
        }
        QuestionBackendCapabilities::from_iter(capabilities)
    }

    fn derive_answer_key(
        &self,
        question: &QuestionRevision,
        _generated: &QuestionVariationParameters,
    ) -> Result<Option<AnswerKey>, PleQuestionBackendError> {
        let _ = question;
        Ok(Some(AnswerKey::Numeric {
            expected: self.expected,
        }))
    }
}

fn versioned_numeric_question(version: &str) -> QuestionRevision {
    QuestionRevision {
        question_id: question_id(),
        revision_number: revision_number(if version == "1" { 5 } else { 6 }),
        workspace: WorkspaceId::from_uuid(Uuid::from_u128(7)),
        backend_locator: QuestionBackendLocator::Ple,
        question_format: QuestionFormat::PleAlgorithmic,
        prompt: vec![QuestionContentBlock::Text {
            markdown: "Enter the generated reference value.".to_string(),
        }],
        response: QuestionResponseFormat::Numeric {
            tolerance: NumericResponseTolerance::Exact,
            unit: None,
        },
        question_type: QuestionType::Numeric,
        question_attempt_limit: QuestionAttemptLimit {
            max_attempts: Some(1),
        },
        question_attempt_time_limit: QuestionAttemptTimeLimit::Unlimited,
        question_variation_rule: QuestionVariationRule::Seeded {
            generator: QuestionGeneratorReference {
                id: "versioned-numeric-generator".to_string(),
                version: version.to_string(),
            },
            parameters: BTreeMap::new(),
        },
        grading: QuestionGradingRule::AllOrNothing { points: 1.0 },
        metadata: metadata("Versioned numeric implementation"),
    }
}

#[test]
fn additive_generator_versions_coexist_while_question_summary_capabilities_stay_conservative() {
    let mut adapter = PleQuestionBackend::empty();
    adapter
        .register_implementation(VersionedNumericImplementation {
            version: "1",
            implementation_release: "1",
            expected: 1.0,
            supports_client_rendering: true,
        })
        .expect("first generator version should register");
    adapter
        .register_implementation(VersionedNumericImplementation {
            version: "2",
            implementation_release: "2",
            expected: 2.0,
            supports_client_rendering: false,
        })
        .expect("additive generator version should coexist with the first");

    let version_one = versioned_numeric_question("1");
    let version_two = versioned_numeric_question("2");
    let first_issue = adapter
        .issue(
            &version_one,
            QuestionSeed::new(41),
            &source_object_reference(),
            &source_object_checksum(),
            &[],
        )
        .expect("published generator version 1 remains dispatchable");
    let second_issue = adapter
        .issue(
            &version_two,
            QuestionSeed::new(41),
            &source_object_reference(),
            &source_object_checksum(),
            &[],
        )
        .expect("published generator version 2 dispatches independently");

    let question_summary_capabilities = adapter
        .capabilities(&version_one)
        .expect("implementation capabilities should resolve without a generator reference");
    assert!(question_summary_capabilities.supports(Capability::ServerGrading));
    assert!(!question_summary_capabilities.supports(Capability::ClientRendering));
    assert!(matches!(
        adapter.grade(
            &version_one,
            QuestionSeed::new(41),
            &first_issue.parameter_hash,
            &first_issue.reproduction_details,
            &[],
            &StudentResponse::Numeric { value: 1.0 },
        ),
        Ok(QuestionGradingOutcome::Graded(result)) if result.correct
    ));
    assert!(matches!(
        adapter.grade(
            &version_two,
            QuestionSeed::new(41),
            &second_issue.parameter_hash,
            &second_issue.reproduction_details,
            &[],
            &StudentResponse::Numeric { value: 2.0 },
        ),
        Ok(QuestionGradingOutcome::Graded(result)) if result.correct
    ));
}

#[test]
fn ple_question_implementation_registration_is_unique_per_source_contract() {
    let mut adapter = PleQuestionBackend::empty();
    adapter
        .register_implementation(VersionedNumericImplementation {
            version: "1",
            implementation_release: "1",
            expected: 1.0,
            supports_client_rendering: true,
        })
        .expect("first source contract should register");

    assert!(matches!(
        adapter.register_implementation(VersionedNumericImplementation {
            version: "1",
            implementation_release: "replacement",
            expected: 2.0,
            supports_client_rendering: false,
        }),
        Err(PleQuestionBackendError::DuplicateQuestionImplementation {
            question_format: QuestionFormat::PleAlgorithmic,
            question_type: QuestionType::Numeric,
            generator: Some(QuestionGeneratorReference { .. }),
        })
    ));
}

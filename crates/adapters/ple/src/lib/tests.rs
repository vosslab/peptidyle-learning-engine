use std::collections::BTreeMap;

use domain::draft_preview::{DraftPreviewRequest, DraftPreviewResult, preview_ple_draft};
use domain::generator::QuestionVariationParameters;
use grading::{AnswerKey, GradingError};
use question_model::answer::{NumericResponseTolerance, ResponseSelectionRule};
use question_model::assignment_activity_rules::{QuestionAttemptLimit, QuestionAttemptTimeLimit};
use question_model::capability::{Capability, QuestionBackendCapabilities};
use question_model::classification::License;
use question_model::envelope::{
    QuestionAssetReference as PresentedQuestionAssetReference, QuestionContentBlock,
};
use question_model::generation::{
    QuestionGeneratorParameter, QuestionGeneratorReference, QuestionSeed,
    QuestionVariationDefinition,
};
use question_model::response::{QuestionChoice, QuestionResponseFormat, ResponseItemReference};
use question_model::{
    DraftQuestionBackendLocator, DraftQuestionRevision, QuestionAssetId, QuestionBackendLocator,
    QuestionFormat, QuestionGradingRule, QuestionId, QuestionMetadata, QuestionRevision,
    QuestionRevisionNumber, QuestionType, StudentResponse, WorkspaceId,
};
use uuid::Uuid;

use super::*;
use crate::test_support::flat_single_choice_bytes;

fn question_id() -> QuestionId {
    QuestionId::from_canonical_parts("ABCDEF", 'G').expect("Question ID")
}

fn revision_number(value: u32) -> QuestionRevisionNumber {
    QuestionRevisionNumber::new(value).expect("positive Question Revision Number")
}

fn flat_question() -> QuestionRevision {
    let workspace = WorkspaceId::from_uuid(Uuid::from_u128(1));
    let document =
        crate::flat_question::FlatQuestionDocument::parse(flat_single_choice_bytes().as_slice())
            .expect("stored flat fixture should parse");
    let draft = document
        .compile(workspace)
        .expect("flat fixture should compile")
        .into_parts()
        .0;
    QuestionRevision::from_draft(
        draft,
        question_id(),
        revision_number(1),
        QuestionBackendLocator::Ple,
    )
}

fn question_choice(id: &str, label: &str) -> QuestionChoice {
    QuestionChoice {
        id: ResponseItemReference::new(id),
        body: vec![QuestionContentBlock::Text {
            markdown: label.to_string(),
        }],
    }
}

fn metadata(title: &str) -> QuestionMetadata {
    QuestionMetadata {
        title: title.to_string(),
        tags: Vec::new(),
        classifications: Vec::new(),
        license: License::CcBySa,
        language: "en-US".to_string(),
    }
}

fn peptide_question() -> QuestionRevision {
    peptide_question_with_generator_version(peptide_bond_geometry::GENERATOR_VERSION)
}

fn peptide_question_with_generator_version(generator_version: &str) -> QuestionRevision {
    let mut parameters = BTreeMap::new();
    parameters.insert(
        "residue".to_string(),
        QuestionGeneratorParameter::Choice {
            options: vec!["alanine".to_string(), "glycine".to_string()],
        },
    );
    QuestionRevision {
        question_id: question_id(),
        revision_number: revision_number(1),
        workspace: WorkspaceId::from_uuid(Uuid::from_u128(2)),
        backend_locator: QuestionBackendLocator::Ple,
        question_format: QuestionFormat::PleAlgorithmic,
        prompt: vec![QuestionContentBlock::Text {
            markdown: "In a peptide containing {{residue}}, which linkage is planar?".to_string(),
        }],
        response: QuestionResponseFormat::MultipleChoice {
            choices: vec![
                question_choice("ester", "ester"),
                question_choice("amide", "amide"),
                question_choice("ether", "ether"),
            ],
            selection: ResponseSelectionRule::ExactlyOne,
        },
        question_type: QuestionType::MultipleChoice,
        question_attempt_limit: QuestionAttemptLimit { max_attempts: None },
        question_attempt_time_limit: QuestionAttemptTimeLimit::Limited {
            seconds: 90,
            grace_seconds: 5,
        },
        question_variation_definition: QuestionVariationDefinition::Seeded {
            generator: QuestionGeneratorReference {
                id: peptide_bond_geometry::GENERATOR_ID.to_string(),
                version: generator_version.to_string(),
            },
            parameters,
        },
        grading: QuestionGradingRule::AllOrNothing { points: 2.0 },
        metadata: metadata("Peptide-bond geometry"),
    }
}

fn peptide_draft() -> DraftQuestionRevision {
    let question = peptide_question();
    DraftQuestionRevision {
        workspace: question.workspace,
        backend_locator: DraftQuestionBackendLocator::Ple,
        question_format: question.question_format,
        prompt: question.prompt,
        response: question.response,
        question_type: question.question_type,
        question_attempt_limit: question.question_attempt_limit,
        question_attempt_time_limit: question.question_attempt_time_limit,
        question_variation_definition: question.question_variation_definition,
        grading: question.grading,
        metadata: question.metadata,
    }
}

#[test]
fn author_presentation_is_deterministic_varied_and_contains_only_display_material() {
    let adapter = PleQuestionBackend::new();
    let draft = peptide_draft();
    let first = adapter
        .author_presentation(&draft, QuestionSeed::new(1))
        .expect("valid peptide draft should materialize")
        .expect("peptide Question Implementation supplies an author presentation");
    let replay = adapter
        .author_presentation(&draft, QuestionSeed::new(1))
        .expect("valid peptide draft should replay")
        .expect("peptide Question Implementation supplies an author presentation");

    assert!(first == replay, "the same seed must replay exactly");
    assert!(matches!(
        &first.prompt[0],
        QuestionContentBlock::Text { markdown }
            if !markdown.contains("{{residue}}")
                && (markdown.contains("alanine") || markdown.contains("glycine"))
    ));
    assert_eq!(
        first.question_answer,
        vec![QuestionContentBlock::Text {
            markdown: "amide".to_string(),
        }],
        "the presentation copies the public choice body, never its identifier"
    );
    assert!(matches!(
        first.question_answer_explanation.as_deref(),
        Some([QuestionContentBlock::Text { markdown }])
            if markdown.contains("partial double-bond") && markdown.contains("planar")
    ));

    let varies = (2..=256).any(|seed| {
        adapter
            .author_presentation(&draft, QuestionSeed::new(seed))
            .expect("valid seeded draft should materialize")
            .is_some_and(|presentation| presentation.prompt != first.prompt)
    });
    assert!(
        varies,
        "the author preview must reveal an actual generated variant"
    );
}

#[test]
fn an_implementation_without_an_author_presentation_is_honestly_unavailable() {
    let mut adapter = PleQuestionBackend::empty();
    adapter
        .register_implementation(NumericReferenceImplementation)
        .expect("the test implementation is unique");
    let mut draft = peptide_draft();
    draft.backend_locator = DraftQuestionBackendLocator::Ple;
    draft.question_format = QuestionFormat::PleAlgorithmic;
    draft.question_type = QuestionType::Numeric;
    draft.question_variation_definition = QuestionVariationDefinition::Static;
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
fn same_seed_issues_the_same_key_free_question_and_reproduction_record() {
    let adapter = PleQuestionBackend::new();
    let question = peptide_question();

    let first = adapter
        .issue(&question, QuestionSeed::new(37), &[])
        .expect("valid peptide question should issue");
    let replay = adapter
        .issue(&question, QuestionSeed::new(37), &[])
        .expect("same question and seed should issue again");

    assert_eq!(first, replay);
    let delivered = serde_json::to_string(&first.envelope)
        .expect("issued envelope should serialize for the browser");
    assert!(!delivered.contains("correct"));
    assert!(!delivered.contains("expected"));
    assert_eq!(first.reproduction_details.backend.version, ADAPTER_VERSION);
    assert_eq!(first.reproduction_details.grader.version, GRADING_VERSION);
    assert_eq!(
        first.reproduction_details.generator,
        Some(QuestionGeneratorReference {
            id: peptide_bond_geometry::GENERATOR_ID.to_string(),
            version: peptide_bond_geometry::GENERATOR_VERSION.to_string(),
        })
    );
}

#[test]
fn flat_question_capabilities_are_installed_and_reproducible_without_answer_keys() {
    let adapter = PleQuestionBackend::new();
    let question = flat_question();
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
        .issue(&question, QuestionSeed::new(10), &[])
        .expect("flat Question Implementation issue should be key free");
    let replay = adapter
        .reproduce(
            &question,
            QuestionSeed::new(10),
            &issue.parameter_hash,
            &issue.reproduction_details,
            &[],
        )
        .expect("flat issue should reproduce exactly");

    assert_eq!(issue.envelope, replay);
    let public = serde_json::to_string(&issue.envelope)
        .expect("issued envelope should serialize for student");
    assert!(!public.contains("correctChoice"));
    assert!(!public.contains("publicSha256"));
}

#[test]
fn flat_question_grade_refuses_without_server_persisted_material() {
    let adapter = PleQuestionBackend::new();
    let question = flat_question();
    let issue = adapter
        .issue(&question, QuestionSeed::new(11), &[])
        .expect("flat issue should deliver reproducible envelope");

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
fn ple_draft_preview_matches_the_published_envelope_presentation() {
    let adapter = PleQuestionBackend::new();
    let question = peptide_question();
    let seed = QuestionSeed::new(37);
    let issued = adapter.issue(&question, seed, &[]).expect("PLE issue");
    let preview = preview_ple_draft(
        &DraftPreviewRequest {
            workspace: question.workspace,
            backend_locator: DraftQuestionBackendLocator::Ple,
            title: question.metadata.title.clone(),
            prompt: question.prompt.clone(),
            response: question.response.clone(),
            question_variation_definition: question.question_variation_definition.clone(),
        },
        seed,
    )
    .expect("PLE preview");
    let DraftPreviewResult::Ready { preview } = preview else {
        panic!("PLE previews locally")
    };
    assert_eq!(preview.title, issued.envelope.title);
    assert_eq!(preview.prompt, issued.envelope.prompt);
    assert_eq!(preview.response, issued.envelope.response);
}

#[test]
fn correct_and_wrong_responses_are_graded_only_after_regeneration() {
    let adapter = PleQuestionBackend::new();
    let question = peptide_question();
    let issued = adapter
        .issue(&question, QuestionSeed::new(99), &[])
        .expect("valid peptide question should issue");

    let correct = adapter
        .grade(
            &question,
            QuestionSeed::new(99),
            &issued.parameter_hash,
            &issued.reproduction_details,
            &[],
            &StudentResponse::MultipleChoice {
                selected: vec![ResponseItemReference::new("amide")],
            },
        )
        .expect("matching attempt should grade");
    let wrong = adapter
        .grade(
            &question,
            QuestionSeed::new(99),
            &issued.parameter_hash,
            &issued.reproduction_details,
            &[],
            &StudentResponse::MultipleChoice {
                selected: vec![ResponseItemReference::new("ester")],
            },
        )
        .expect("matching attempt should grade");

    assert!(matches!(
        correct,
        QuestionGradingOutcome::Graded(result) if result.correct && result.points_earned == 2.0
    ));
    assert!(matches!(
        wrong,
        QuestionGradingOutcome::Graded(result) if !result.correct && result.points_earned == 0.0
    ));
}

#[test]
fn peptide_feedback_uses_public_choice_blocks_without_exposing_key_material() {
    let adapter = PleQuestionBackend::new();
    let question = peptide_question();
    let issued = adapter
        .issue(&question, QuestionSeed::new(99), &[])
        .expect("PLE issue");
    let (outcome, content) = adapter
        .grade_with_feedback(
            &question,
            QuestionSeed::new(99),
            &issued.parameter_hash,
            &issued.reproduction_details,
            &[],
            &StudentResponse::MultipleChoice {
                selected: vec![ResponseItemReference::new("ester")],
            },
        )
        .expect("verified wrong response receives teaching feedback");
    assert!(matches!(outcome, QuestionGradingOutcome::Graded(result) if !result.correct));
    assert_eq!(
        content
            .question_answer
            .map(|answer| answer.content().to_vec()),
        Some(vec![QuestionContentBlock::Text {
            markdown: "amide".to_string(),
        }])
    );
    let hint = adapter
        .hint_for_issued_question(
            &question,
            QuestionSeed::new(99),
            &issued.parameter_hash,
            &issued.reproduction_details,
            &[],
        )
        .expect("verified issued Question accepts a hint request")
        .expect("implemented Question Implementation advertises a real hint");
    let explanation = content
        .question_answer_explanation
        .expect("implemented Question Implementation provides a Question Answer Explanation");
    assert!(
        matches!(&hint.content()[0], QuestionContentBlock::Text { markdown } if markdown.contains("lone pair"))
    );
    assert!(
        matches!(&explanation.content()[0], QuestionContentBlock::Text { markdown } if markdown.contains("partial double-bond") && markdown.contains("planar"))
    );
    assert!(
        adapter
            .capabilities(&question)
            .expect("registered Question Implementation")
            .supports(Capability::Hints)
    );
}

#[test]
fn altered_question_attempt_reproduction_details_are_refused_before_grading() {
    let adapter = PleQuestionBackend::new();
    let question = peptide_question();
    let issued = adapter
        .issue(&question, QuestionSeed::new(5), &[])
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
                selected: vec![ResponseItemReference::new("amide")],
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

#[test]
fn uninstalled_generator_versions_are_refused_without_fallback() {
    let adapter = PleQuestionBackend::new();
    let mut question = peptide_question();
    let QuestionVariationDefinition::Seeded { generator, .. } =
        &mut question.question_variation_definition
    else {
        panic!("peptide fixture is seeded")
    };
    generator.version = "2".to_string();

    assert!(matches!(
        adapter.issue(&question, QuestionSeed::new(61), &[]),
        Err(PleQuestionBackendError::UnknownQuestionImplementation { generator: Some(found), .. })
            if found.version == "2"
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
    let mut question = peptide_question();
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
            &question_asset_object_references,
        )
        .expect("trusted assets should be recorded at issue time");
    assert_eq!(
        issued.reproduction_details.asset_objects,
        vec![question_asset_object_references[0].object_reference]
    );

    assert!(matches!(
        adapter.issue(&question, QuestionSeed::new(82), &[]),
        Err(PleQuestionBackendError::MissingAssetBinding(found)) if found == rendered_asset
    ));
    assert!(matches!(
        adapter.issue(
            &peptide_question(),
            QuestionSeed::new(82),
            &question_asset_object_references,
        ),
        Err(PleQuestionBackendError::UnrelatedAssetBinding(found)) if found == rendered_asset
    ));
    assert!(matches!(
        adapter.issue(
            &question,
            QuestionSeed::new(82),
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
    let mut question = peptide_question();
    let prompt_asset = asset(91);
    let response_asset = asset(90);
    question.prompt.push(image(prompt_asset));
    let QuestionResponseFormat::MultipleChoice { choices, .. } = &mut question.response else {
        panic!("peptide fixture has multiple-choice response")
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
fn rendered_envelope_hash_has_a_fixed_compatibility_vector() {
    let adapter = PleQuestionBackend::new();
    let issued = adapter
        .issue(&peptide_question(), QuestionSeed::new(37), &[])
        .expect("fixed vector should issue");
    assert_eq!(
        issued.reproduction_details.rendered_question_sha256,
        "96836584526d6fac9585dd32c77c579c681c49fc93b5d4be6085623d571e6e7d"
    );
}

#[test]
fn historical_blank_or_oversized_titles_are_refused_before_issue() {
    let adapter = PleQuestionBackend::new();
    for title in [" \t".to_string(), "\u{1F9EC}".repeat(513)] {
        let mut question = peptide_question();
        question.metadata.title = title;
        assert!(matches!(
            adapter.issue(&question, QuestionSeed::new(37), &[]),
            Err(PleQuestionBackendError::InvalidTitle(_))
        ));
    }
}

#[test]
fn an_implementation_refuses_seeded_content_that_cannot_show_its_variation() {
    let adapter = PleQuestionBackend::new();
    let mut question = peptide_question();
    question.prompt = vec![QuestionContentBlock::Text {
        markdown: "Which linkage is planar?".to_string(),
    }];

    assert!(matches!(
        adapter.issue(&question, QuestionSeed::new(1), &[]),
        Err(PleQuestionBackendError::IncompatibleQuestionImplementation { .. })
    ));
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
        question_variation_definition: QuestionVariationDefinition::Static,
        grading: QuestionGradingRule::AllOrNothing { points: 1.0 },
        metadata: metadata("Numeric registry extension"),
    };
    let issued = adapter
        .issue(&question, QuestionSeed::new(123), &[])
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
        question_variation_definition: QuestionVariationDefinition::Seeded {
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
fn additive_generator_versions_coexist_while_catalog_capabilities_stay_conservative() {
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
        .issue(&version_one, QuestionSeed::new(41), &[])
        .expect("published generator version 1 remains dispatchable");
    let second_issue = adapter
        .issue(&version_two, QuestionSeed::new(41), &[])
        .expect("published generator version 2 dispatches independently");

    let catalog_capabilities = adapter
        .capabilities(&version_one)
        .expect("implementation capabilities should resolve without a generator reference");
    assert!(catalog_capabilities.supports(Capability::ServerGrading));
    assert!(!catalog_capabilities.supports(Capability::ClientRendering));
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

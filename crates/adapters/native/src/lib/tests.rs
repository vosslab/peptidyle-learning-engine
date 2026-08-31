use std::collections::BTreeMap;

use domain::draft_preview::{DraftPreviewRequest, DraftPreviewResult, preview_native_draft};
use domain::generator::GeneratedVariant;
use grading::{AnswerKey, GradingError};
use question_model::answer::{NumericResponseTolerance, ResponseSelectionRule};
use question_model::assignment_activity_rules::{QuestionAttemptLimit, QuestionAttemptTimeLimit};
use question_model::capability::{Capability, QuestionBackendCapabilities};
use question_model::envelope::{AssetRef, ContentBlock};
use question_model::generation::{GeneratorReference, ParameterSpec, RandomizationDefinition};
use question_model::response::{ResponseItemReference, ChoiceOption, QuestionResponseFormat};
use question_model::taxonomy::License;
use question_model::{
    AssetId, DraftQuestionDefinition, DraftQuestionSource, GradingDefinition,
    ImplementationVersion, QuestionFormat, QuestionId, QuestionMetadata, QuestionSource,
    QuestionType, QuestionVersionNumber, StudentResponse, WorkspaceId,
};
use uuid::Uuid;

use super::*;
use crate::test_support::flat_single_choice_bytes;

fn question_id() -> QuestionId {
    QuestionId::from_canonical_parts("ABCDEF", 'G').expect("Question ID")
}

fn version_number(value: u32) -> QuestionVersionNumber {
    QuestionVersionNumber::new(value).expect("positive Question Version Number")
}

fn flat_question() -> QuestionDefinition {
    let workspace = WorkspaceId::from_uuid(Uuid::from_u128(1));
    let document =
        crate::flat_question::FlatQuestionDocument::parse(flat_single_choice_bytes().as_slice())
            .expect("stored flat fixture should parse");
    let draft = document
        .compile(workspace)
        .expect("flat fixture should compile")
        .into_parts()
        .0;
    QuestionDefinition::from_draft(
        draft,
        question_id(),
        version_number(1),
        QuestionSource::Native,
    )
}

fn choice(id: &str, label: &str) -> ChoiceOption {
    ChoiceOption {
        id: ResponseItemReference::new(id),
        body: vec![ContentBlock::Text {
            markdown: label.to_string(),
        }],
    }
}

fn metadata(title: &str) -> QuestionMetadata {
    QuestionMetadata {
        title: title.to_string(),
        tags: Vec::new(),
        taxonomy: Vec::new(),
        license: License::CcBySa,
        language: "en-US".to_string(),
    }
}

fn peptide_question() -> QuestionDefinition {
    peptide_question_with_generator_version(peptide_bond_geometry::GENERATOR_VERSION)
}

fn peptide_question_with_generator_version(generator_version: &str) -> QuestionDefinition {
    let mut parameters = BTreeMap::new();
    parameters.insert(
        "residue".to_string(),
        ParameterSpec::Choice {
            options: vec!["alanine".to_string(), "glycine".to_string()],
        },
    );
    QuestionDefinition {
        question_id: question_id(),
        version_number: version_number(1),
        workspace: WorkspaceId::from_uuid(Uuid::from_u128(2)),
        source: QuestionSource::Native,
        question_format: QuestionFormat::NativeAlgorithmic,
        prompt: vec![ContentBlock::Text {
            markdown: "In a peptide containing {{residue}}, which linkage is planar?".to_string(),
        }],
        response: QuestionResponseFormat::MultipleChoice {
            choices: vec![
                choice("ester", "ester"),
                choice("amide", "amide"),
                choice("ether", "ether"),
            ],
            selection: ResponseSelectionRule::ExactlyOne,
        },
        question_type: QuestionType::MultipleChoice,
        question_attempt_limit: QuestionAttemptLimit { max_attempts: None },
        question_attempt_time_limit: QuestionAttemptTimeLimit::Limited {
            seconds: 90,
            grace_seconds: 5,
        },
        randomization: RandomizationDefinition::Seeded {
            generator: GeneratorReference {
                id: peptide_bond_geometry::GENERATOR_ID.to_string(),
                version: generator_version.to_string(),
            },
            parameters,
        },
        grading: GradingDefinition::AllOrNothing { points: 2.0 },
        metadata: metadata("Peptide-bond geometry"),
    }
}

fn peptide_draft() -> DraftQuestionDefinition {
    let question = peptide_question();
    DraftQuestionDefinition {
        workspace: question.workspace,
        source: DraftQuestionSource::Native,
        question_format: question.question_format,
        prompt: question.prompt,
        response: question.response,
        question_type: question.question_type,
        question_attempt_limit: question.question_attempt_limit,
        question_attempt_time_limit: question.question_attempt_time_limit,
        randomization: question.randomization,
        grading: question.grading,
        metadata: question.metadata,
    }
}

#[test]
fn author_presentation_is_deterministic_varied_and_contains_only_display_material() {
    let adapter = NativeAdapter::new();
    let draft = peptide_draft();
    let first = adapter
        .author_presentation(&draft, Seed::new(1))
        .expect("valid peptide draft should materialize")
        .expect("peptide Question Implementation supplies an author presentation");
    let replay = adapter
        .author_presentation(&draft, Seed::new(1))
        .expect("valid peptide draft should replay")
        .expect("peptide Question Implementation supplies an author presentation");

    assert!(first == replay, "the same seed must replay exactly");
    assert!(matches!(
        &first.prompt[0],
        ContentBlock::Text { markdown }
            if !markdown.contains("{{residue}}")
                && (markdown.contains("alanine") || markdown.contains("glycine"))
    ));
    assert_eq!(
        first.correct_response,
        vec![ContentBlock::Text {
            markdown: "amide".to_string(),
        }],
        "the presentation copies the public choice body, never its identifier"
    );
    assert!(matches!(
        first.rationale.as_deref(),
        Some([ContentBlock::Text { markdown }])
            if markdown.contains("partial double-bond") && markdown.contains("planar")
    ));

    let varies = (2..=256).any(|seed| {
        adapter
            .author_presentation(&draft, Seed::new(seed))
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
    let mut adapter = NativeAdapter::empty();
    adapter
        .register_implementation(NumericReferenceImplementation)
        .expect("the test implementation is unique");
    let mut draft = peptide_draft();
    draft.source = DraftQuestionSource::Native;
    draft.question_format = QuestionFormat::NativeAlgorithmic;
    draft.question_type = QuestionType::Numeric;
    draft.randomization = RandomizationDefinition::Static;
    draft.prompt = vec![ContentBlock::Text {
        markdown: "Enter the reference value.".to_string(),
    }];

    assert!(
        adapter
            .author_presentation(&draft, Seed::new(4))
            .expect("the default author-presentation implementation is safe")
            .is_none(),
        "Question Implementations opt in explicitly; the engine never serializes a grading key as a fallback"
    );
}

#[test]
fn same_seed_issues_the_same_key_free_question_and_reproduction_record() {
    let adapter = NativeAdapter::new();
    let question = peptide_question();

    let first = adapter
        .issue(&question, Seed::new(37), &[])
        .expect("valid peptide question should issue");
    let replay = adapter
        .issue(&question, Seed::new(37), &[])
        .expect("same question and seed should issue again");

    assert_eq!(first, replay);
    let delivered = serde_json::to_string(&first.envelope)
        .expect("issued envelope should serialize for the browser");
    assert!(!delivered.contains("correct"));
    assert!(!delivered.contains("expected"));
    assert_eq!(first.source_record.adapter.version, ADAPTER_VERSION);
    assert_eq!(first.source_record.grading.version, GRADING_VERSION);
    assert_eq!(
        first.source_record.generator,
        Some(GeneratorReference {
            id: peptide_bond_geometry::GENERATOR_ID.to_string(),
            version: peptide_bond_geometry::GENERATOR_VERSION.to_string(),
        })
    );
}

#[test]
fn flat_question_capabilities_are_installed_and_reproducible_without_answer_keys() {
    let adapter = NativeAdapter::new();
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
        .issue(&question, Seed::new(10), &[])
        .expect("flat Question Implementation issue should be key free");
    let replay = adapter
        .reproduce(
            &question,
            Seed::new(10),
            &issue.parameter_hash,
            &issue.source_record,
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
    let adapter = NativeAdapter::new();
    let question = flat_question();
    let issue = adapter
        .issue(&question, Seed::new(11), &[])
        .expect("flat issue should deliver reproducible envelope");

    assert!(matches!(
        adapter.grade(
            &question,
            Seed::new(11),
            &issue.parameter_hash,
            &issue.source_record,
            &[],
            &StudentResponse::MultipleChoice {
                selected: vec![ResponseItemReference::new("blue")],
            },
        ),
        Err(NativeAdapterError::Grading(GradingError::MissingAnswerKey))
    ));
}

#[test]
fn native_draft_preview_matches_the_published_envelope_presentation() {
    let adapter = NativeAdapter::new();
    let question = peptide_question();
    let seed = Seed::new(37);
    let issued = adapter.issue(&question, seed, &[]).expect("native issue");
    let preview = preview_native_draft(
        &DraftPreviewRequest {
            workspace: question.workspace,
            source: DraftQuestionSource::Native,
            title: question.metadata.title.clone(),
            prompt: question.prompt.clone(),
            response: question.response.clone(),
            randomization: question.randomization.clone(),
        },
        seed,
    )
    .expect("native preview");
    let DraftPreviewResult::Ready { preview } = preview else {
        panic!("native previews locally")
    };
    assert_eq!(preview.title, issued.envelope.title);
    assert_eq!(preview.prompt, issued.envelope.prompt);
    assert_eq!(preview.response, issued.envelope.response);
}

#[test]
fn correct_and_wrong_responses_are_graded_only_after_regeneration() {
    let adapter = NativeAdapter::new();
    let question = peptide_question();
    let issued = adapter
        .issue(&question, Seed::new(99), &[])
        .expect("valid peptide question should issue");

    let correct = adapter
        .grade(
            &question,
            Seed::new(99),
            &issued.parameter_hash,
            &issued.source_record,
            &[],
            &StudentResponse::MultipleChoice {
                selected: vec![ResponseItemReference::new("amide")],
            },
        )
        .expect("matching attempt should grade");
    let wrong = adapter
        .grade(
            &question,
            Seed::new(99),
            &issued.parameter_hash,
            &issued.source_record,
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
    let adapter = NativeAdapter::new();
    let question = peptide_question();
    let issued = adapter
        .issue(&question, Seed::new(99), &[])
        .expect("native issue");
    let (outcome, feedback) = adapter
        .grade_with_feedback(
            &question,
            Seed::new(99),
            &issued.parameter_hash,
            &issued.source_record,
            &[],
            &StudentResponse::MultipleChoice {
                selected: vec![ResponseItemReference::new("ester")],
            },
        )
        .expect("verified wrong response receives teaching feedback");
    assert!(matches!(outcome, QuestionGradingOutcome::Graded(result) if !result.correct));
    assert_eq!(
        feedback.correct_response,
        Some(vec![ContentBlock::Text {
            markdown: "amide".to_string(),
        }])
    );
    let hint = feedback
        .hint
        .expect("implemented Question Implementation advertises a real hint");
    let rationale = feedback
        .rationale
        .expect("implemented Question Implementation provides rationale");
    assert!(matches!(&hint[0], ContentBlock::Text { markdown } if markdown.contains("lone pair")));
    assert!(
        matches!(&rationale[0], ContentBlock::Text { markdown } if markdown.contains("partial double-bond") && markdown.contains("planar"))
    );
    assert!(
        adapter
            .capabilities(&question)
            .expect("registered Question Implementation")
            .supports(Capability::Hints)
    );
}

#[test]
fn altered_question_attempt_source_record_is_refused_before_grading() {
    let adapter = NativeAdapter::new();
    let question = peptide_question();
    let issued = adapter
        .issue(&question, Seed::new(5), &[])
        .expect("valid peptide question should issue");
    let mut altered = issued.source_record;
    altered.grading.version = "different".to_string();

    assert!(matches!(
        adapter.grade(
            &question,
            Seed::new(5),
            &issued.parameter_hash,
            &altered,
            &[],
            &StudentResponse::MultipleChoice {
                selected: vec![ResponseItemReference::new("amide")],
            },
        ),
        Err(NativeAdapterError::UnknownImplementation {
            field: "grading",
            ..
        })
    ));
}

#[test]
fn uninstalled_execution_versions_are_refused_before_issue_or_grading() {
    let mut adapter = NativeAdapter::new();
    assert!(matches!(
        adapter.select_current_implementations(
            implementation_version(ADAPTER_ID, "2"),
            implementation_version(GRADING_ID, GRADING_VERSION),
        ),
        Err(NativeAdapterError::UnknownImplementation {
            field: "adapter",
            ..
        })
    ));
}

#[test]
fn uninstalled_generator_versions_are_refused_without_fallback() {
    let adapter = NativeAdapter::new();
    let mut question = peptide_question();
    let RandomizationDefinition::Seeded { generator, .. } = &mut question.randomization else {
        panic!("peptide fixture is seeded")
    };
    generator.version = "2".to_string();

    assert!(matches!(
        adapter.issue(&question, Seed::new(61), &[]),
        Err(NativeAdapterError::UnknownQuestionImplementation { generator: Some(found), .. })
            if found.version == "2"
    ));
}

fn asset(id: u128) -> AssetId {
    AssetId::from_uuid(Uuid::from_u128(id))
}

fn image(asset: AssetId) -> ContentBlock {
    ContentBlock::Image {
        asset: AssetRef {
            asset,
            checksum: "fixture-checksum".to_string(),
        },
        description: "A trusted fixture image.".to_string(),
    }
}

#[test]
fn asset_provenance_requires_exact_complete_trusted_bindings() {
    let adapter = NativeAdapter::new();
    let mut question = peptide_question();
    let rendered_asset = asset(81);
    question.prompt.push(image(rendered_asset));
    let bindings = [AssetObjectBinding {
        asset: rendered_asset,
        object: ObjectId::from_uuid(Uuid::from_u128(82)),
    }];
    let issued = adapter
        .issue(&question, Seed::new(82), &bindings)
        .expect("trusted assets should be recorded at issue time");
    assert_eq!(issued.source_record.asset_objects, vec![bindings[0].object]);

    assert!(matches!(
        adapter.issue(&question, Seed::new(82), &[]),
        Err(NativeAdapterError::MissingAssetBinding(found)) if found == rendered_asset
    ));
    assert!(matches!(
        adapter.issue(
            &peptide_question(),
            Seed::new(82),
            &bindings,
        ),
        Err(NativeAdapterError::UnrelatedAssetBinding(found)) if found == rendered_asset
    ));
    assert!(matches!(
        adapter.issue(
            &question,
            Seed::new(82),
            &[
                bindings[0],
                AssetObjectBinding {
                    asset: rendered_asset,
                    object: ObjectId::from_uuid(Uuid::from_u128(83)),
                },
            ],
        ),
        Err(NativeAdapterError::ConflictingAssetBinding(found)) if found == rendered_asset
    ));

    let mut altered_source_record = issued.source_record.clone();
    altered_source_record.asset_objects = vec![ObjectId::from_uuid(Uuid::from_u128(84))];

    assert!(matches!(
        adapter.reproduce(
            &question,
            Seed::new(82),
            &issued.parameter_hash,
            &altered_source_record,
            &bindings,
        ),
        Err(NativeAdapterError::ReproductionMismatch {
            field: "assetObjects"
        })
    ));
}

#[test]
fn nested_response_assets_are_bound_in_canonical_logical_asset_order() {
    let adapter = NativeAdapter::new();
    let mut question = peptide_question();
    let prompt_asset = asset(91);
    let response_asset = asset(90);
    question.prompt.push(image(prompt_asset));
    let QuestionResponseFormat::MultipleChoice { choices, .. } = &mut question.response else {
        panic!("peptide fixture has multiple-choice response")
    };
    choices[0].body.push(image(response_asset));
    let bindings = [
        AssetObjectBinding {
            asset: prompt_asset,
            object: ObjectId::from_uuid(Uuid::from_u128(191)),
        },
        AssetObjectBinding {
            asset: response_asset,
            object: ObjectId::from_uuid(Uuid::from_u128(190)),
        },
    ];

    let issued = adapter
        .issue(&question, Seed::new(91), &bindings)
        .expect("prompt and nested response assets should resolve");

    assert_eq!(
        issued.source_record.asset_objects,
        vec![bindings[1].object, bindings[0].object],
        "asset IDs, not caller order, canonically order persisted objects"
    );
}

#[test]
fn rendered_envelope_hash_has_a_fixed_compatibility_vector() {
    let adapter = NativeAdapter::new();
    let issued = adapter
        .issue(&peptide_question(), Seed::new(37), &[])
        .expect("fixed vector should issue");
    assert_eq!(
        issued.source_record.rendered_question_sha256,
        "d1174a00295e0bf9ab85f935ef6eadab8548dce254a315281f4a704f20afac22"
    );
}

#[test]
fn historical_blank_or_oversized_titles_are_refused_before_issue() {
    let adapter = NativeAdapter::new();
    for title in [" \t".to_string(), "\u{1F9EC}".repeat(513)] {
        let mut question = peptide_question();
        question.metadata.title = title;
        assert!(matches!(
            adapter.issue(&question, Seed::new(37), &[]),
            Err(NativeAdapterError::InvalidTitle(_))
        ));
    }
}

#[test]
fn an_implementation_refuses_seeded_content_that_cannot_show_its_variation() {
    let adapter = NativeAdapter::new();
    let mut question = peptide_question();
    question.prompt = vec![ContentBlock::Text {
        markdown: "Which linkage is planar?".to_string(),
    }];

    assert!(matches!(
        adapter.issue(&question, Seed::new(1), &[]),
        Err(NativeAdapterError::IncompatibleQuestionImplementation { .. })
    ));
}

#[derive(Debug, Clone, Copy)]
struct NumericReferenceImplementation;

impl NativeQuestionImplementation for NumericReferenceImplementation {
    fn question_format(&self) -> QuestionFormat {
        QuestionFormat::NativeAlgorithmic
    }

    fn question_type(&self) -> QuestionType {
        QuestionType::Numeric
    }

    fn implementation_release(&self) -> ImplementationVersion {
        ImplementationVersion {
            id: "numeric-reference".to_string(),
            version: "1".to_string(),
        }
    }

    fn generator(&self) -> Option<GeneratorReference> {
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
        question: &QuestionDefinition,
        _generated: &GeneratedVariant,
    ) -> Result<Option<AnswerKey>, NativeAdapterError> {
        if !matches!(question.response, QuestionResponseFormat::Numeric { .. }) {
            return Err(NativeAdapterError::IncompatibleQuestionImplementation {
                message: "numeric response required".to_string(),
            });
        }
        Ok(Some(AnswerKey::Numeric { expected: 7.0 }))
    }
}

#[test]
fn a_second_implementation_plugs_into_the_registry_without_engine_changes() {
    let mut adapter = NativeAdapter::empty();
    adapter
        .register_implementation(NumericReferenceImplementation)
        .expect("new implementation identifier should register");
    let question = QuestionDefinition {
        question_id: question_id(),
        version_number: version_number(3),
        workspace: WorkspaceId::from_uuid(Uuid::from_u128(4)),
        source: QuestionSource::Native,
        question_format: QuestionFormat::NativeAlgorithmic,
        prompt: vec![ContentBlock::Text {
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
        randomization: RandomizationDefinition::Static,
        grading: GradingDefinition::AllOrNothing { points: 1.0 },
        metadata: metadata("Numeric registry extension"),
    };
    let issued = adapter
        .issue(&question, Seed::new(123), &[])
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
            Seed::new(123),
            &issued.parameter_hash,
            &issued.source_record,
            &[],
            &StudentResponse::Numeric { value: 7.0 },
        ),
        Ok(QuestionGradingOutcome::Graded(result)) if result.correct
    ));
}

#[derive(Debug, Clone, Copy)]
struct VersionedNumericImplementation {
    version: &'static str,
    expected: f64,
    supports_client_rendering: bool,
}

impl NativeQuestionImplementation for VersionedNumericImplementation {
    fn question_format(&self) -> QuestionFormat {
        QuestionFormat::NativeAlgorithmic
    }

    fn question_type(&self) -> QuestionType {
        QuestionType::Numeric
    }

    fn implementation_release(&self) -> ImplementationVersion {
        ImplementationVersion {
            id: "versioned-numeric".to_string(),
            version: self.version.to_string(),
        }
    }

    fn generator(&self) -> Option<GeneratorReference> {
        Some(GeneratorReference {
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
        question: &QuestionDefinition,
        _generated: &GeneratedVariant,
    ) -> Result<Option<AnswerKey>, NativeAdapterError> {
        let _ = question;
        Ok(Some(AnswerKey::Numeric {
            expected: self.expected,
        }))
    }
}

fn versioned_numeric_question(version: &str) -> QuestionDefinition {
    QuestionDefinition {
        question_id: question_id(),
        version_number: version_number(if version == "1" { 5 } else { 6 }),
        workspace: WorkspaceId::from_uuid(Uuid::from_u128(7)),
        source: QuestionSource::Native,
        question_format: QuestionFormat::NativeAlgorithmic,
        prompt: vec![ContentBlock::Text {
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
        randomization: RandomizationDefinition::Seeded {
            generator: GeneratorReference {
                id: "versioned-numeric-generator".to_string(),
                version: version.to_string(),
            },
            parameters: BTreeMap::new(),
        },
        grading: GradingDefinition::AllOrNothing { points: 1.0 },
        metadata: metadata("Versioned numeric implementation"),
    }
}

#[test]
fn additive_generator_versions_coexist_while_catalog_capabilities_stay_conservative() {
    let mut adapter = NativeAdapter::empty();
    adapter
        .register_implementation(VersionedNumericImplementation {
            version: "1",
            expected: 1.0,
            supports_client_rendering: true,
        })
        .expect("first generator version should register");
    adapter
        .register_implementation(VersionedNumericImplementation {
            version: "2",
            expected: 2.0,
            supports_client_rendering: false,
        })
        .expect("additive generator version should coexist with the first");

    let version_one = versioned_numeric_question("1");
    let version_two = versioned_numeric_question("2");
    let first_issue = adapter
        .issue(&version_one, Seed::new(41), &[])
        .expect("published generator version 1 remains dispatchable");
    let second_issue = adapter
        .issue(&version_two, Seed::new(41), &[])
        .expect("published generator version 2 dispatches independently");

    let catalog_capabilities = adapter
        .capabilities(&version_one)
        .expect("implementation capabilities should resolve without a generator reference");
    assert!(catalog_capabilities.supports(Capability::ServerGrading));
    assert!(!catalog_capabilities.supports(Capability::ClientRendering));
    assert!(matches!(
        adapter.grade(
            &version_one,
            Seed::new(41),
            &first_issue.parameter_hash,
            &first_issue.source_record,
            &[],
            &StudentResponse::Numeric { value: 1.0 },
        ),
        Ok(QuestionGradingOutcome::Graded(result)) if result.correct
    ));
    assert!(matches!(
        adapter.grade(
            &version_two,
            Seed::new(41),
            &second_issue.parameter_hash,
            &second_issue.source_record,
            &[],
            &StudentResponse::Numeric { value: 2.0 },
        ),
        Ok(QuestionGradingOutcome::Graded(result)) if result.correct
    ));
}

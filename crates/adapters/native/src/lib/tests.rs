use std::collections::BTreeMap;

use domain::draft_preview::{DraftPreviewRequest, DraftPreviewResult, preview_native_draft};
use domain::generator::GeneratedVariant;
use grading::{AnswerKey, GradingError};
use question_model::answer::{NumericTolerance, SelectionCardinality};
use question_model::capability::{BackendCapabilities, Capability};
use question_model::envelope::{AssetRef, ContentBlock};
use question_model::generation::{ParameterSpec, RandomizationDefinition};
use question_model::response::{ChoiceId, ChoiceOption, ResponseDefinition};
use question_model::run_policy::{AttemptPolicy, FeedbackDisclosure, TimingPolicy};
use question_model::taxonomy::License;
use question_model::{
    AssetId, DraftQuestionDefinition, DraftQuestionSource, GradingDefinition, ProblemId,
    QuestionMetadata, QuestionSource, StudentResponse, VersionId, WorkspaceId,
};
use uuid::Uuid;

use super::*;

const FLAT_FAVORITE_COLOR: &str = r#"{
  "format": "pleFlatQuestion",
  "version": 1,
  "kind": "singleChoice",
  "title": "Favorite color",
  "prompt": "What is my favorite color?",
  "choices": [
    {"id": "blue", "text": "Blue", "feedback": "Blue is a calm choice."},
    {"id": "red", "text": "Red", "feedback": "Red is not my favorite."},
    {"id": "yellow", "text": "Yellow", "feedback": "Yellow is bright."}
  ],
  "correctChoice": "blue",
  "feedback": {
    "correct": "Exactly right.",
    "incorrect": "Try thinking of a cool color."
  },
  "points": 1.0,
  "attemptPolicy": {"maxAttempts": null, "feedback": "immediateFull"},
  "timingPolicy": {"kind": "untimed"},
  "tags": ["example"],
  "taxonomy": [],
  "license": {"kind": "ccBySa"},
  "language": "en-US"
}"#;

fn flat_question() -> QuestionDefinition {
    let workspace = WorkspaceId::from_uuid(Uuid::from_u128(1));
    let document =
        crate::flat_question::FlatQuestionDocument::parse(FLAT_FAVORITE_COLOR.as_bytes())
            .expect("flat fixture should parse");
    let draft = document
        .compile(workspace)
        .expect("flat fixture should compile")
        .into_parts()
        .0;
    QuestionDefinition::from_draft(
        draft,
        ProblemId::from_uuid(Uuid::from_u128(2)),
        VersionId::from_uuid(Uuid::from_u128(3)),
        QuestionSource::Native {
            family: crate::flat_question::FLAT_SINGLE_CHOICE_FAMILY.to_string(),
        },
    )
}

fn choice(id: &str, label: &str) -> ChoiceOption {
    ChoiceOption {
        id: ChoiceId::new(id),
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
        version: VersionId::from_uuid(Uuid::from_u128(1)),
        problem: ProblemId::from_uuid(Uuid::from_u128(10)),
        workspace: WorkspaceId::from_uuid(Uuid::from_u128(2)),
        source: QuestionSource::Native {
            family: peptide_bond_geometry::FAMILY_ID.to_string(),
        },
        prompt: vec![ContentBlock::Text {
            markdown: "In a peptide containing {{residue}}, which linkage is planar?".to_string(),
        }],
        response: ResponseDefinition::MultipleChoice {
            choices: vec![
                choice("ester", "ester"),
                choice("amide", "amide"),
                choice("ether", "ether"),
            ],
            selection: SelectionCardinality::ExactlyOne,
        },
        attempt_policy: AttemptPolicy {
            max_attempts: None,
            feedback: FeedbackDisclosure::ImmediateCorrectness,
        },
        timing_policy: TimingPolicy::PerQuestion {
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
        source: DraftQuestionSource::Native {
            family: peptide_bond_geometry::FAMILY_ID.to_string(),
        },
        prompt: question.prompt,
        response: question.response,
        attempt_policy: question.attempt_policy,
        timing_policy: question.timing_policy,
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
        .expect("peptide family supplies an author presentation");
    let replay = adapter
        .author_presentation(&draft, Seed::new(1))
        .expect("valid peptide draft should replay")
        .expect("peptide family supplies an author presentation");

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
fn a_family_without_an_author_presentation_is_honestly_unavailable() {
    let mut adapter = NativeAdapter::empty();
    adapter
        .register_family(NumericReferenceFamily)
        .expect("the test family is unique");
    let mut draft = peptide_draft();
    draft.source = DraftQuestionSource::Native {
        family: "numeric-reference".to_string(),
    };
    draft.randomization = RandomizationDefinition::Static;
    draft.prompt = vec![ContentBlock::Text {
        markdown: "Enter the reference value.".to_string(),
    }];

    assert!(
        adapter
            .author_presentation(&draft, Seed::new(4))
            .expect("the default author-presentation implementation is safe")
            .is_none(),
        "families opt in explicitly; the engine never serializes a grading key as a fallback"
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
    assert_eq!(first.provenance.adapter.version, ADAPTER_VERSION);
    assert_eq!(first.provenance.grading.version, GRADING_VERSION);
    assert_eq!(
        first.provenance.generator,
        Some(GeneratorReference {
            id: peptide_bond_geometry::GENERATOR_ID.to_string(),
            version: peptide_bond_geometry::GENERATOR_VERSION.to_string(),
        })
    );
}

#[test]
fn flat_family_capabilities_are_installed_and_reproducible_without_answer_keys() {
    let adapter = NativeAdapter::new();
    let question = flat_question();
    let expected = BackendCapabilities::from_iter([
        Capability::ClientRendering,
        Capability::ServerGrading,
        Capability::Hints,
        Capability::PerQuestionTiming,
    ]);

    assert_eq!(
        adapter
            .capabilities(&question.source)
            .expect("family is installed"),
        expected
    );
    let issue = adapter
        .issue(&question, Seed::new(10), &[])
        .expect("flat family issue should be key free");
    let replay = adapter
        .reproduce(
            &question,
            Seed::new(10),
            &issue.parameter_hash,
            &issue.provenance,
            &[],
        )
        .expect("flat issue should reproduce exactly");

    assert_eq!(issue.envelope, replay);
    let public = serde_json::to_string(&issue.envelope)
        .expect("issued envelope should serialize for learner");
    assert!(!public.contains("correctChoice"));
    assert!(!public.contains("publicSha256"));
}

#[test]
fn flat_family_grade_refuses_without_server_persisted_material() {
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
            &issue.provenance,
            &[],
            &StudentResponse::MultipleChoice {
                selected: vec![ChoiceId::new("blue")],
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
            source: DraftQuestionSource::Native {
                family: peptide_bond_geometry::FAMILY_ID.to_string(),
            },
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
            &issued.provenance,
            &[],
            &StudentResponse::MultipleChoice {
                selected: vec![ChoiceId::new("amide")],
            },
        )
        .expect("matching attempt should grade");
    let wrong = adapter
        .grade(
            &question,
            Seed::new(99),
            &issued.parameter_hash,
            &issued.provenance,
            &[],
            &StudentResponse::MultipleChoice {
                selected: vec![ChoiceId::new("ester")],
            },
        )
        .expect("matching attempt should grade");

    assert!(matches!(
        correct,
        GradeOutcome::Graded(result) if result.correct && result.points_earned == 2.0
    ));
    assert!(matches!(
        wrong,
        GradeOutcome::Graded(result) if !result.correct && result.points_earned == 0.0
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
            &issued.provenance,
            &[],
            &StudentResponse::MultipleChoice {
                selected: vec![ChoiceId::new("ester")],
            },
        )
        .expect("verified wrong response receives teaching feedback");
    assert!(matches!(outcome, GradeOutcome::Graded(result) if !result.correct));
    assert_eq!(
        feedback.correct_response,
        Some(vec![ContentBlock::Text {
            markdown: "amide".to_string(),
        }])
    );
    let hint = feedback
        .hint
        .expect("implemented family advertises a real hint");
    let rationale = feedback
        .rationale
        .expect("implemented family provides rationale");
    assert!(matches!(&hint[0], ContentBlock::Text { markdown } if markdown.contains("lone pair")));
    assert!(
        matches!(&rationale[0], ContentBlock::Text { markdown } if markdown.contains("partial double-bond") && markdown.contains("planar"))
    );
    assert!(
        adapter
            .capabilities(&question.source)
            .expect("registered family")
            .supports(Capability::Hints)
    );
}

#[test]
fn altered_attempt_provenance_is_refused_before_grading() {
    let adapter = NativeAdapter::new();
    let question = peptide_question();
    let issued = adapter
        .issue(&question, Seed::new(5), &[])
        .expect("valid peptide question should issue");
    let mut altered = issued.provenance;
    altered.grading.version = "different".to_string();

    assert!(matches!(
        adapter.grade(
            &question,
            Seed::new(5),
            &issued.parameter_hash,
            &altered,
            &[],
            &StudentResponse::MultipleChoice {
                selected: vec![ChoiceId::new("amide")],
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
        Err(NativeAdapterError::UnknownGenerator { family, generator: Some(found) })
            if family == peptide_bond_geometry::FAMILY_ID && found.version == "2"
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
    assert_eq!(issued.provenance.asset_objects, vec![bindings[0].object]);

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

    let mut altered_provenance = issued.provenance.clone();
    altered_provenance.asset_objects = vec![ObjectId::from_uuid(Uuid::from_u128(84))];

    assert!(matches!(
        adapter.reproduce(
            &question,
            Seed::new(82),
            &issued.parameter_hash,
            &altered_provenance,
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
    let ResponseDefinition::MultipleChoice { choices, .. } = &mut question.response else {
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
        issued.provenance.asset_objects,
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
        issued.provenance.rendered_question_sha256,
        "7300981097ff06e8237a30336738efcba49eb5236219d8002934666c01334a86"
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
fn a_family_refuses_seeded_content_that_cannot_show_its_variation() {
    let adapter = NativeAdapter::new();
    let mut question = peptide_question();
    question.prompt = vec![ContentBlock::Text {
        markdown: "Which linkage is planar?".to_string(),
    }];

    assert!(matches!(
        adapter.issue(&question, Seed::new(1), &[]),
        Err(NativeAdapterError::InvalidFamilyDefinition { .. })
    ));
}

#[derive(Debug, Clone, Copy)]
struct NumericReferenceFamily;

impl NativeQuestionFamily for NumericReferenceFamily {
    fn family(&self) -> &'static str {
        "numeric-reference"
    }

    fn generator(&self) -> Option<GeneratorReference> {
        None
    }

    fn capabilities(&self) -> BackendCapabilities {
        BackendCapabilities::from_iter([Capability::ClientRendering, Capability::ServerGrading])
    }

    fn derive_answer_key(
        &self,
        question: &QuestionDefinition,
        _generated: &GeneratedVariant,
    ) -> Result<Option<AnswerKey>, NativeAdapterError> {
        if !matches!(question.response, ResponseDefinition::Numeric { .. }) {
            return Err(NativeAdapterError::InvalidFamilyDefinition {
                family: self.family().to_string(),
                message: "numeric response required".to_string(),
            });
        }
        Ok(Some(AnswerKey::Numeric { expected: 7.0 }))
    }
}

#[test]
fn a_second_family_plugs_into_the_registry_without_engine_changes() {
    let mut adapter = NativeAdapter::empty();
    adapter
        .register_family(NumericReferenceFamily)
        .expect("new family identifier should register");
    let question = QuestionDefinition {
        version: VersionId::from_uuid(Uuid::from_u128(3)),
        problem: ProblemId::from_uuid(Uuid::from_u128(11)),
        workspace: WorkspaceId::from_uuid(Uuid::from_u128(4)),
        source: QuestionSource::Native {
            family: "numeric-reference".to_string(),
        },
        prompt: vec![ContentBlock::Text {
            markdown: "Enter the reference value.".to_string(),
        }],
        response: ResponseDefinition::Numeric {
            tolerance: NumericTolerance::Exact,
            unit: None,
        },
        attempt_policy: AttemptPolicy {
            max_attempts: Some(1),
            feedback: FeedbackDisclosure::Deferred,
        },
        timing_policy: TimingPolicy::Untimed,
        randomization: RandomizationDefinition::Static,
        grading: GradingDefinition::AllOrNothing { points: 1.0 },
        metadata: metadata("Numeric registry extension"),
    };
    let issued = adapter
        .issue(&question, Seed::new(123), &[])
        .expect("registered family should issue through the generic adapter");

    assert!(
        adapter
            .capabilities(&question.source)
            .expect("registered source should expose capabilities")
            .supports(Capability::ServerGrading)
    );
    assert!(matches!(
        adapter.grade(
            &question,
            Seed::new(123),
            &issued.parameter_hash,
            &issued.provenance,
            &[],
            &StudentResponse::Numeric { value: 7.0 },
        ),
        Ok(GradeOutcome::Graded(result)) if result.correct
    ));
}

#[derive(Debug, Clone, Copy)]
struct VersionedNumericFamily {
    version: &'static str,
    expected: f64,
    supports_client_rendering: bool,
}

impl NativeQuestionFamily for VersionedNumericFamily {
    fn family(&self) -> &'static str {
        "versioned-numeric"
    }

    fn generator(&self) -> Option<GeneratorReference> {
        Some(GeneratorReference {
            id: "versioned-numeric-generator".to_string(),
            version: self.version.to_string(),
        })
    }

    fn capabilities(&self) -> BackendCapabilities {
        let mut capabilities = vec![Capability::ServerGrading];
        if self.supports_client_rendering {
            capabilities.push(Capability::ClientRendering);
        }
        BackendCapabilities::from_iter(capabilities)
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
        version: VersionId::from_uuid(Uuid::from_u128(if version == "1" { 5 } else { 6 })),
        problem: ProblemId::from_uuid(Uuid::from_u128(12)),
        workspace: WorkspaceId::from_uuid(Uuid::from_u128(7)),
        source: QuestionSource::Native {
            family: "versioned-numeric".to_string(),
        },
        prompt: vec![ContentBlock::Text {
            markdown: "Enter the generated reference value.".to_string(),
        }],
        response: ResponseDefinition::Numeric {
            tolerance: NumericTolerance::Exact,
            unit: None,
        },
        attempt_policy: AttemptPolicy {
            max_attempts: Some(1),
            feedback: FeedbackDisclosure::Deferred,
        },
        timing_policy: TimingPolicy::Untimed,
        randomization: RandomizationDefinition::Seeded {
            generator: GeneratorReference {
                id: "versioned-numeric-generator".to_string(),
                version: version.to_string(),
            },
            parameters: BTreeMap::new(),
        },
        grading: GradingDefinition::AllOrNothing { points: 1.0 },
        metadata: metadata("Versioned numeric family"),
    }
}

#[test]
fn additive_generator_versions_coexist_while_catalog_capabilities_stay_conservative() {
    let mut adapter = NativeAdapter::empty();
    adapter
        .register_family(VersionedNumericFamily {
            version: "1",
            expected: 1.0,
            supports_client_rendering: true,
        })
        .expect("first generator version should register");
    adapter
        .register_family(VersionedNumericFamily {
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
        .capabilities(&version_one.source)
        .expect("family capabilities should resolve without a generator reference");
    assert!(catalog_capabilities.supports(Capability::ServerGrading));
    assert!(!catalog_capabilities.supports(Capability::ClientRendering));
    assert!(matches!(
        adapter.grade(
            &version_one,
            Seed::new(41),
            &first_issue.parameter_hash,
            &first_issue.provenance,
            &[],
            &StudentResponse::Numeric { value: 1.0 },
        ),
        Ok(GradeOutcome::Graded(result)) if result.correct
    ));
    assert!(matches!(
        adapter.grade(
            &version_two,
            Seed::new(41),
            &second_issue.parameter_hash,
            &second_issue.provenance,
            &[],
            &StudentResponse::Numeric { value: 2.0 },
        ),
        Ok(GradeOutcome::Graded(result)) if result.correct
    ));
}

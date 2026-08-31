//! Reference native Question Implementation: peptide-bond resonance and planarity.
//!
//! This implementation is deliberately small. Its job is to prove that a native
//! question can generate a visible variant, reproduce the same envelope, and
//! grade through the server-only boundary. The registry contract, rather than
//! this biochemistry example, is the reusable engine design.

use std::collections::BTreeSet;

use domain::generator::{GeneratedValue, GeneratedVariant};
use grading::AnswerKey;
use question_model::answer::ResponseSelectionRule;
use question_model::capability::{Capability, QuestionBackendCapabilities};
use question_model::definition::GradingDefinition;
use question_model::envelope::{ContentBlock, QuestionPresentation};
use question_model::generation::GeneratorReference;
use question_model::response::{ResponseItemReference, QuestionResponseFormat};
use question_model::{
    DraftQuestionDefinition, FeedbackContent, GradingResult, ImplementationVersion,
    QuestionDefinition, QuestionFormat, QuestionType, StudentResponse,
};

use crate::NativeAdapterError;
use crate::generator::{AuthorPresentationContent, NativeQuestionImplementation};

/// Stable native Question Implementation name.
pub const IMPLEMENTATION_ID: &str = "peptide-bond-geometry";
/// First release of [`IMPLEMENTATION_ID`].
pub const IMPLEMENTATION_RELEASE: &str = "1";
/// Stable generator identifier used by published definitions.
pub const GENERATOR_ID: &str = "peptide-bond-choice";
/// Initial generator implementation version for [`GENERATOR_ID`].
///
/// This is not the repository CalVer release or a question version.
pub const GENERATOR_VERSION: &str = "1";

const RESIDUE_PARAMETER: &str = "residue";
const CORRECT_CHOICE_ID: &str = "amide";

/// Initial peptide-bond geometry implementation.
#[derive(Debug, Clone, Copy, Default)]
pub struct PeptideBondGeometryV1;

impl NativeQuestionImplementation for PeptideBondGeometryV1 {
    fn question_format(&self) -> QuestionFormat {
        QuestionFormat::NativeAlgorithmic
    }

    fn question_type(&self) -> QuestionType {
        QuestionType::MultipleChoice
    }

    fn implementation_release(&self) -> ImplementationVersion {
        ImplementationVersion {
            id: IMPLEMENTATION_ID.to_string(),
            version: IMPLEMENTATION_RELEASE.to_string(),
        }
    }

    fn generator(&self) -> Option<GeneratorReference> {
        Some(GeneratorReference {
            id: GENERATOR_ID.to_string(),
            version: GENERATOR_VERSION.to_string(),
        })
    }

    fn capabilities(&self) -> QuestionBackendCapabilities {
        QuestionBackendCapabilities::from_iter([
            Capability::AlgorithmicGeneration,
            Capability::ClientRendering,
            Capability::ServerGrading,
            Capability::Hints,
            Capability::QuestionAttemptTimeLimit,
        ])
    }

    fn derive_answer_key(
        &self,
        question: &QuestionDefinition,
        generated: &GeneratedVariant,
    ) -> Result<Option<AnswerKey>, NativeAdapterError> {
        validate_question_shape(question)?;
        if !question.prompt.iter().any(|block| {
            matches!(block, question_model::envelope::ContentBlock::Text { markdown } if markdown.contains("{{residue}}"))
        }) {
            return Err(invalid_definition(
                "prompt must include the {{residue}} token so seeded variants are visible",
            ));
        }
        let _ = generated_residue(generated)?;
        let answer_key = AnswerKey::MultipleChoice {
            correct: BTreeSet::from([ResponseItemReference::new(CORRECT_CHOICE_ID)]),
        };
        Ok(Some(answer_key))
    }

    fn derive_feedback(
        &self,
        question: &QuestionDefinition,
        generated: &GeneratedVariant,
        envelope: &QuestionPresentation,
        answer_key: Option<&AnswerKey>,
        result: &GradingResult,
        response: &StudentResponse,
    ) -> Result<FeedbackContent, NativeAdapterError> {
        validate_question_shape(question)?;
        let _ = generated_residue(generated)?;
        // Require the same trusted key shape used by grading, but never expose
        // it. Correct-response blocks are copied from the public choice body.
        let Some(AnswerKey::MultipleChoice { correct }) = answer_key else {
            return Err(invalid_definition(
                "peptide-bond feedback requires its multiple-choice key",
            ));
        };
        if !correct.contains(&ResponseItemReference::new(CORRECT_CHOICE_ID)) {
            return Err(invalid_definition(
                "peptide-bond feedback key is inconsistent",
            ));
        }
        if envelope.response != question.response {
            return Err(invalid_definition(
                "materialized response does not match the immutable definition",
            ));
        }
        let selected = match response {
            question_model::StudentResponse::MultipleChoice { selected } => selected,
            _ => {
                return Err(invalid_definition(
                    "peptide-bond feedback requires a multiple-choice response",
                ));
            }
        };
        if selected.len() > 1 {
            return Err(invalid_definition(
                "peptide-bond feedback received an invalid selection",
            ));
        }
        // Binding `result` here prevents feedback materialization from being a
        // detached template: it is computed only for this verified grade.
        let _ = result.correct;
        let QuestionResponseFormat::MultipleChoice { choices, .. } = &question.response else {
            return Err(invalid_definition("response must be multiple choice"));
        };
        let correct_choice = choices
            .iter()
            .find(|choice| choice.id.as_str() == CORRECT_CHOICE_ID)
            .ok_or_else(|| {
                invalid_definition("response choices must include the stable amide identifier")
            })?;
        Ok(FeedbackContent {
            hint: Some(vec![ContentBlock::Text {
                markdown: "Compare the C-N bond to an ordinary single bond: ask whether nitrogen's lone pair can share electron density with the neighboring carbonyl.".to_string(),
            }]),
            correct_response: Some(correct_choice.body.clone()),
            rationale: Some(vec![ContentBlock::Text {
                markdown: "In a peptide bond, resonance delocalizes the nitrogen lone pair into the carbonyl. The C-N bond therefore has partial double-bond character, which restricts rotation and keeps the peptide group approximately planar.".to_string(),
            }]),
        })
    }

    fn derive_author_presentation(
        &self,
        question: &DraftQuestionDefinition,
        generated: &GeneratedVariant,
        _prompt: &[ContentBlock],
    ) -> Result<Option<AuthorPresentationContent>, NativeAdapterError> {
        validate_draft_shape(question)?;
        if !question.prompt.iter().any(|block| {
            matches!(block, ContentBlock::Text { markdown } if markdown.contains("{{residue}}"))
        }) {
            return Err(invalid_definition(
                "prompt must include the {{residue}} token so seeded variants are visible",
            ));
        }
        let _ = generated_residue(generated)?;
        let QuestionResponseFormat::MultipleChoice { choices, .. } = &question.response else {
            return Err(invalid_definition("response must be multiple choice"));
        };
        let correct_choice = choices
            .iter()
            .find(|choice| choice.id.as_str() == CORRECT_CHOICE_ID)
            .ok_or_else(|| {
                invalid_definition("response choices must include the stable amide identifier")
            })?;
        Ok(Some(AuthorPresentationContent {
            correct_response: correct_choice.body.clone(),
            rationale: Some(vec![ContentBlock::Text {
                markdown: "In a peptide bond, resonance delocalizes the nitrogen lone pair into the carbonyl. The C-N bond therefore has partial double-bond character, which restricts rotation and keeps the peptide group approximately planar.".to_string(),
            }]),
        }))
    }
}

fn validate_question_shape(question: &QuestionDefinition) -> Result<(), NativeAdapterError> {
    validate_response_and_grading(&question.response, &question.grading)
}

fn validate_draft_shape(question: &DraftQuestionDefinition) -> Result<(), NativeAdapterError> {
    validate_response_and_grading(&question.response, &question.grading)
}

fn validate_response_and_grading(
    response: &QuestionResponseFormat,
    grading: &GradingDefinition,
) -> Result<(), NativeAdapterError> {
    let QuestionResponseFormat::MultipleChoice { choices, selection } = response else {
        return Err(invalid_definition(
            "response must be a multiple-choice definition",
        ));
    };
    if *selection != ResponseSelectionRule::ExactlyOne {
        return Err(invalid_definition(
            "response must select exactly one peptide-bond choice",
        ));
    }
    if !choices
        .iter()
        .any(|choice| choice.id.as_str() == CORRECT_CHOICE_ID)
    {
        return Err(invalid_definition(
            "response choices must include the stable amide identifier",
        ));
    }
    if !matches!(grading, GradingDefinition::AllOrNothing { .. }) {
        return Err(invalid_definition(
            "peptide-bond geometry uses all-or-nothing grading",
        ));
    }
    Ok(())
}

fn generated_residue(generated: &GeneratedVariant) -> Result<&str, NativeAdapterError> {
    let Some(GeneratedValue::Choice { value }) = generated.parameters.get(RESIDUE_PARAMETER) else {
        return Err(invalid_definition(
            "generator must produce a choice parameter named residue",
        ));
    };
    if value.trim().is_empty() {
        return Err(invalid_definition(
            "generated residue must contain visible text",
        ));
    }
    Ok(value)
}

fn invalid_definition(message: &str) -> NativeAdapterError {
    NativeAdapterError::IncompatibleQuestionImplementation {
        message: message.to_string(),
    }
}

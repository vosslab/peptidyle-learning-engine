//! Reproducible Question Variation recipes and their answer-free presentation.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::generation::{
    QuestionGeneratorParameter, QuestionGeneratorReference, QuestionSeed, QuestionVariationRule,
};
use crate::question_content::QuestionContentBlock;
use crate::{QuestionResponseFormat, QuestionRevisionReference};

/// The reproducible generated state for one exact Question Revision and seed.
///
/// The same pair produces the same Question Variation Presentation on every
/// machine, allowing the render cache to serve a repeat request and grading to
/// be re-derived years later.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QuestionVariation {
    /// Exact immutable Question Revision that produced this presentation.
    pub question_revision: QuestionRevisionReference,
    /// Exact generator used for this variation, when the Question is seeded.
    #[serde(skip)]
    pub generator: Option<QuestionGeneratorReference>,
    /// Declared generator parameters, in deterministic key order.
    #[serde(skip)]
    pub parameters: BTreeMap<String, QuestionGeneratorParameter>,
    /// The seed that produced this variant.
    pub seed: QuestionSeed,
}

impl QuestionVariation {
    /// Records an exact static variation with no generator or parameters.
    pub fn static_variation(
        question_revision: QuestionRevisionReference,
        seed: QuestionSeed,
    ) -> Self {
        Self {
            question_revision,
            generator: None,
            parameters: BTreeMap::new(),
            seed,
        }
    }

    /// Records the exact declared variation recipe for an issued Question.
    pub fn from_question_variation_rule(
        question_revision: QuestionRevisionReference,
        question_variation_rule: &QuestionVariationRule,
        seed: QuestionSeed,
    ) -> Self {
        match question_variation_rule {
            QuestionVariationRule::Static => Self::static_variation(question_revision, seed),
            QuestionVariationRule::Seeded {
                generator,
                parameters,
            } => Self {
                question_revision,
                generator: Some(generator.clone()),
                parameters: parameters.clone(),
                seed,
            },
        }
    }
}

/// One answer-free Question Presentation derived from a Question Variation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QuestionVariationPresentation {
    /// The exact reproducible variation this presentation renders.
    pub variation: QuestionVariation,
    /// A bounded student-facing title from published metadata or a safe imported
    /// Question Backend label. This deliberately excludes Question Source, Answer Key,
    /// and Question Grading Input while letting the student identify the issued Question.
    pub title: String,
    /// The prompt, in render order.
    pub prompt: Vec<QuestionContentBlock>,
    /// The shape of response this variant expects.
    pub response: QuestionResponseFormat,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn reference() -> QuestionRevisionReference {
        QuestionRevisionReference {
            question_id: "123-4567".parse().expect("valid Question ID"),
            revision_number: crate::QuestionRevisionNumber::new(1).expect("positive version"),
        }
    }

    #[test]
    fn variation_retains_the_exact_declared_generation_recipe() {
        let static_variation =
            QuestionVariation::static_variation(reference(), QuestionSeed::new(3));
        assert_eq!(static_variation.generator, None);
        assert!(static_variation.parameters.is_empty());

        let mut parameters = BTreeMap::new();
        parameters.insert(
            "count".to_string(),
            QuestionGeneratorParameter::IntegerRange { low: 2, high: 7 },
        );
        let variation = QuestionVariation::from_question_variation_rule(
            reference(),
            &QuestionVariationRule::Seeded {
                generator: QuestionGeneratorReference {
                    id: "counted".to_string(),
                    version: "2".to_string(),
                },
                parameters,
            },
            QuestionSeed::new(5),
        );
        assert_eq!(variation.seed, QuestionSeed::new(5));
        assert_eq!(variation.generator.expect("seeded generator").id, "counted");
        assert_eq!(variation.parameters.len(), 1);
    }
}

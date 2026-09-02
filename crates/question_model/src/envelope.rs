//! The question envelope: the payload a client receives (WP-C1).
//!
//! The envelope is what MOD-UI-RENDER maps to components, so every block kind
//! it can contain is enumerated here. A closed set means the renderer's match
//! is exhaustive, and adding a block kind makes the compiler point at the
//! renderer.
//!
//! The envelope carries prompt content and response shape. Answer keys and
//! Answer Keys, Question Feedback, Question Answer Explanations, and Question
//! Grading Input stay in `crates/grading`; an M3 gate inspects a browser
//! network trace to confirm it.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::QuestionRevisionReference;
use crate::generation::{
    QuestionGeneratorParameter, QuestionGeneratorReference, QuestionSeed, QuestionVariationRule,
};
use crate::identity::QuestionAssetId;
use crate::response::QuestionResponseFormat;

/// A reference to a stored asset.
///
/// The checksum travels with the reference so a client can verify that the
/// bytes it received are the bytes the question was authored against, which is
/// what makes a cached render trustworthy.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QuestionAssetReference {
    /// Identifier of the stored object.
    pub asset: QuestionAssetId,
    /// Hex-encoded checksum computed when the asset was written.
    pub checksum: String,
}

/// One renderable piece of a question prompt.
///
/// Each variant that carries visual content also carries text describing it.
/// That text is required rather than optional: a question whose figure has no
/// description is unusable with a screen reader, and MOD-UI-RENDER surfaces a
/// missing description as an authoring error rather than rendering a gap.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum QuestionContentBlock {
    /// Prose, in a restricted Markdown subset that the renderer sanitizes.
    Text {
        /// Markdown source.
        markdown: String,
    },
    /// A mathematical expression.
    Math {
        /// LaTeX source.
        latex: String,
        /// Spoken-form description for assistive technology.
        description: String,
    },
    /// An image or figure.
    Image {
        /// The stored asset.
        asset: QuestionAssetReference,
        /// Description of what the image conveys.
        description: String,
    },
    /// A code listing.
    Code {
        /// Language name for highlighting, for example `python`.
        language: String,
        /// The listing itself.
        source: String,
    },
    /// A data table.
    Table {
        /// Column headings, left to right.
        headers: Vec<String>,
        /// Rows, each holding one cell per heading.
        rows: Vec<Vec<String>>,
        /// Description of what the table shows.
        description: String,
    },
}

/// The reproducible generated state for one exact Question Revision and seed.
///
/// The same pair produces the same Question Presentation on every machine,
/// allowing the render cache to serve a repeat request and grading to be
/// re-derived years later.
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
    /// Question Backend label. This deliberately excludes authored source and grading
    /// material while letting the student identify the issued question.
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
    fn visual_blocks_carry_their_description() {
        let block = QuestionContentBlock::Math {
            latex: r"\frac{1}{2}".to_string(),
            description: "one half".to_string(),
        };
        let json = serde_json::to_string(&block).expect("serialization should succeed");
        assert!(json.contains("one half"));
    }

    #[test]
    fn blocks_serialize_with_a_discriminant() {
        let block = QuestionContentBlock::Text {
            markdown: "Balance the equation.".to_string(),
        };
        let json = serde_json::to_string(&block).expect("serialization should succeed");
        assert!(json.starts_with(r#"{"kind":"text""#));
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

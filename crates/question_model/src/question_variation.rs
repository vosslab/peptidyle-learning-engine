//! Reproducible Question Variation recipes and their answer-free presentation.

use serde::{Deserialize, Serialize};

use crate::generation::QuestionSeed;
use crate::question_content::QuestionContentBlock;
use crate::{QuestionResponseFormat, QuestionRevisionReference};

/// The reproducible generated state for one exact Question Revision and Question Seed.
///
/// The same pair produces the same Question Variation Presentation on every
/// machine, allowing the render cache to serve a repeat request and grading to
/// be re-derived years later.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QuestionVariation {
    /// Exact immutable Question Revision that produced this presentation.
    pub question_revision: QuestionRevisionReference,
    /// The Question Seed that produced this variant.
    #[serde(rename = "question_seed")]
    pub question_seed: QuestionSeed,
}

impl QuestionVariation {
    /// Records the two exact facts that reproduce an issued Question Variation.
    pub fn from_question_revision_and_question_seed(
        question_revision: QuestionRevisionReference,
        question_seed: QuestionSeed,
    ) -> Self {
        Self {
            question_revision,
            question_seed,
        }
    }
}

/// One answer-free Question Presentation derived from a Question Variation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QuestionVariationPresentation {
    /// The exact reproducible variation this presentation renders.
    pub variation: QuestionVariation,
    /// A bounded student-facing Question Title from published metadata or a safe imported
    /// Question Backend label. This deliberately excludes Question Source, Answer Key,
    /// and Question Grading Input while letting the student identify the issued Question.
    pub question_title: String,
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
    fn variation_retains_its_exact_question_seed() {
        let variation = QuestionVariation::from_question_revision_and_question_seed(
            reference(),
            QuestionSeed::new(5),
        );
        assert_eq!(variation.question_seed, QuestionSeed::new(5));
    }
}

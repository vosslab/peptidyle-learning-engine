//! Question Variation Question Seed values.
//!
//! A [`QuestionSeed`] is a server-selected input for a seeded Question Backend.

use serde::{Deserialize, Serialize};

/// The value that selects one Question Variation.
///
/// Stored only for a seeded source selection and its completed seeded
/// reproduction evidence. Static native Question JSON has no Question Seed.
#[doc(hidden)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct QuestionSeed(u64);

impl QuestionSeed {
    /// Wraps a raw Question Seed value.
    pub fn new(value: u64) -> Self {
        QuestionSeed(value)
    }

    /// The raw value, for hashing and storage.
    pub fn value(&self) -> u64 {
        self.0
    }
}

/// Pre-render source selection for one issued Question.
///
/// This is intentionally distinct from [`QuestionReproduction`]. A seeded
/// backend needs its input before it renders generated parameters; the full
/// reproduction descriptor does not exist until that rendering succeeds and
/// atomically records its parameter checksum.
#[doc(hidden)]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase", deny_unknown_fields)]
pub enum QuestionSourceSelection {
    /// The exact immutable source requires no generator input.
    Static,
    /// Input supplied to a backend before it renders a generated variation.
    Seeded {
        #[serde(rename = "questionSeed")]
        question_seed: QuestionSeed,
    },
}

/// The exact reproduction shape for an issued Question.
///
/// A native PLE Question JSON source is static: its immutable Question
/// Revision and source evidence are sufficient to reproduce its authored
/// content.  A renderer-backed backend may instead be seeded.  Keeping these
/// alternatives tagged prevents a static Question from acquiring a sentinel
/// seed or a made-up generated-parameter checksum.
#[doc(hidden)]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase", deny_unknown_fields)]
pub enum QuestionReproduction {
    /// No generated variation exists for this exact immutable source.
    Static,
    /// A backend-generated variation and the checksum of its generated
    /// parameters.  The pair is inseparable.
    Seeded {
        #[serde(rename = "questionSeed")]
        question_seed: QuestionSeed,
        generated_parameter_sha256: String,
    },
}

impl QuestionReproduction {
    /// Returns the server-only generator seed when this backend used one.
    pub fn question_seed(&self) -> Option<QuestionSeed> {
        match self {
            Self::Static => None,
            Self::Seeded { question_seed, .. } => Some(*question_seed),
        }
    }

    /// Returns the generated-parameter integrity evidence when applicable.
    pub fn generated_parameter_sha256(&self) -> Option<&str> {
        match self {
            Self::Static => None,
            Self::Seeded {
                generated_parameter_sha256,
                ..
            } => Some(generated_parameter_sha256),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_seed_keeps_its_value() {
        assert_eq!(QuestionSeed::new(42).value(), 42);
    }
}

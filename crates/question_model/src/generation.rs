//! Question Variation Question Seed values.
//!
//! A [`QuestionSeed`] records one server-selected input used alongside an exact
//! Question Revision by Question Backends and reproduction evidence.

use serde::{Deserialize, Serialize};

/// The value that selects one Question Variation.
///
/// Stored with every attempt so the exact Question Variation a student saw can be
/// rebuilt later, which is what makes a grade auditable years after the fact.
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_seed_keeps_its_value() {
        assert_eq!(QuestionSeed::new(42).value(), 42);
    }
}

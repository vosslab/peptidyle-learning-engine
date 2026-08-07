//! Generation specifications: the input side of seeded generation (WP-C1).
//!
//! A generation spec plus a [`Seed`] fully determines a variant. Everything a
//! generator reads arrives through those two values, which is what lets the
//! same spec run on the server and in the browser and agree byte for byte.
//! That agreement is the WP-C5 seed-parity gate, and it underwrites both the
//! render cache and the reproducibility record.
//!
//! Determinism rules for anyone implementing a generator: read parameters from
//! the spec, take randomness from the seed, and use ordered collections
//! wherever iteration order reaches output.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

/// The value that selects one variant of a question.
///
/// Stored with every attempt so the exact variant a student saw can be
/// rebuilt later, which is what makes a grade auditable years after the fact.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct Seed(u64);

impl Seed {
    /// Wraps a raw seed value.
    pub fn new(value: u64) -> Self {
        Seed(value)
    }

    /// The raw value, for hashing and storage.
    pub fn value(&self) -> u64 {
        self.0
    }
}

/// A single authored parameter a generator reads.
///
/// Parameters are declared rather than computed inline so a preview can show
/// an instructor the space a question draws from, and so the seed-vector
/// corpus can cover every branch of it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum ParameterSpec {
    /// An integer drawn from an inclusive range.
    IntegerRange {
        /// Smallest value, inclusive.
        low: i64,
        /// Largest value, inclusive.
        high: i64,
    },
    /// A decimal drawn from an inclusive range, rounded to `decimals`.
    DecimalRange {
        /// Smallest value, inclusive.
        low: f64,
        /// Largest value, inclusive.
        high: f64,
        /// Decimal places the drawn value is rounded to.
        decimals: u8,
    },
    /// One entry drawn from an authored list.
    Choice {
        /// The candidate values, in authoring order.
        options: Vec<String>,
    },
    /// A value fixed by the author, drawn identically every time.
    Fixed {
        /// The value every variant receives.
        value: String,
    },
}

/// How a question varies between students and between runs.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum RandomizationDefinition {
    /// Every student sees identical content.
    ///
    /// The honest declaration for imported questions that carry no generator.
    /// A question declaring this stays readable in a printed exam.
    Static,
    /// Content is generated from the seed.
    Seeded {
        /// Generator name the backend adapter dispatches on.
        generator: String,
        /// Parameters the generator reads, keyed by name.
        ///
        /// A `BTreeMap` because iteration order reaches generated output, and
        /// determinism requires that order to be the same everywhere.
        parameters: BTreeMap<String, ParameterSpec>,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_seed_keeps_its_value() {
        assert_eq!(Seed::new(42).value(), 42);
    }

    #[test]
    fn parameters_keep_a_stable_order_through_serialization() {
        let mut parameters = BTreeMap::new();
        parameters.insert(
            "mass".to_string(),
            ParameterSpec::IntegerRange { low: 1, high: 9 },
        );
        parameters.insert(
            "element".to_string(),
            ParameterSpec::Choice {
                options: vec!["Na".to_string(), "K".to_string()],
            },
        );
        let definition = RandomizationDefinition::Seeded {
            generator: "molar_mass".to_string(),
            parameters,
        };
        let json = serde_json::to_string(&definition).expect("serialization should succeed");
        // BTreeMap orders keys, so "element" precedes "mass" on every machine.
        // The quotes matter: a bare "mass" also matches inside "molar_mass".
        let element_at = json.find(r#""element""#).expect("element key present");
        let mass_at = json.find(r#""mass""#).expect("mass key present");
        assert!(element_at < mass_at);
    }
}

//! Seeded parameter generation (WP-C5, MOD-GEN).
//!
//! [`generate`] is a pure function of a [`QuestionSeed`] and a
//! [`QuestionVariationRule`]. Random draws come directly from
//! `rand_chacha::ChaCha20Rng`; this module does not use `rand` distributions,
//! whose sampling implementations are a separate compatibility surface.
//! Ordered input and output maps make canonical JSON and its SHA-256 stable.

use std::collections::BTreeMap;
use std::fmt::Write as _;

use question_model::generation::{
    QuestionGeneratorParameter, QuestionGeneratorReference, QuestionSeed, QuestionVariationRule,
};
use rand_chacha::rand_core::{Rng, SeedableRng};
use rand_chacha::ChaCha20Rng;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// Domain separator used to expand a 64-bit seed into ChaCha20's 256-bit key.
const SEED_DOMAIN: &[u8] = b"peptidyle-learning-engine/generator/v1\0";

/// One generated parameter value with its authored semantic kind preserved.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum QuestionVariationParameterValue {
    /// Integer sampled from an inclusive authored range.
    Integer {
        /// Sampled integer.
        value: i64,
    },
    /// Decimal represented exactly as a fixed-precision string.
    Decimal {
        /// Sampled decimal, including the authored number of trailing places.
        value: String,
    },
    /// Value selected from an authored option list.
    Choice {
        /// Selected option.
        value: String,
    },
    /// Authored value that consumes no random draw.
    Fixed {
        /// Fixed value.
        value: String,
    },
}

/// Complete deterministic parameter output for one Question Variation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QuestionVariationParameters {
    /// Generator identifier and version pinned by the published Question Revision.
    pub generator: Option<QuestionGeneratorReference>,
    /// Generated values in stable parameter-name order.
    pub parameters: BTreeMap<String, QuestionVariationParameterValue>,
}

impl QuestionVariationParameters {
    /// Serializes the ordered output and returns its lowercase SHA-256.
    ///
    /// # Errors
    ///
    /// Returns [`GenerationError::Serialization`] if the output cannot be
    /// represented by `serde_json`.
    pub fn sha256(&self) -> Result<String, GenerationError> {
        let bytes = serde_json::to_vec(self)
            .map_err(|error| GenerationError::Serialization(error.to_string()))?;
        let digest = Sha256::digest(bytes);
        let mut hex = String::with_capacity(64);
        for byte in digest {
            let _ = write!(hex, "{byte:02x}");
        }
        Ok(hex)
    }
}

/// Rejected Question Variation Rule parameter or output operation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GenerationError {
    /// Integer lower bound exceeded its upper bound.
    InvalidIntegerRange {
        /// Parameter whose range is invalid.
        parameter: String,
        /// Inclusive lower bound.
        low: i64,
        /// Inclusive upper bound.
        high: i64,
    },
    /// Decimal bounds were non-finite, reversed, imprecise, or too large.
    InvalidDecimalRange {
        /// Parameter whose range is invalid.
        parameter: String,
    },
    /// Authored choice list had no choices.
    EmptyChoice {
        /// Parameter whose option list is empty.
        parameter: String,
    },
    /// Canonical output serialization failed.
    Serialization(String),
}

impl std::fmt::Display for GenerationError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidIntegerRange {
                parameter,
                low,
                high,
            } => write!(
                formatter,
                "integer parameter {parameter} has invalid range {low}..={high}"
            ),
            Self::InvalidDecimalRange { parameter } => {
                write!(
                    formatter,
                    "decimal parameter {parameter} has an invalid range"
                )
            }
            Self::EmptyChoice { parameter } => {
                write!(formatter, "choice parameter {parameter} has no options")
            }
            Self::Serialization(message) => {
                write!(
                    formatter,
                    "generated output could not be serialized: {message}"
                )
            }
        }
    }
}

impl std::error::Error for GenerationError {}

/// Generates one exact Question Variation Parameters map from a seed and authored
/// Question Variation Rule.
///
/// Static Question Variation Rules return empty Question Variation Parameters and consume no
/// randomness. Seeded Question Variation Rules iterate their `BTreeMap` in key order. Fixed and
/// single-value parameters consume no draw, so adding one cannot perturb
/// unrelated random parameters.
///
/// # Errors
///
/// Returns [`GenerationError`] for reversed ranges, decimal bounds that cannot
/// be represented at their authored precision, or an empty choice list.
pub fn generate(
    seed: QuestionSeed,
    rule: &QuestionVariationRule,
) -> Result<QuestionVariationParameters, GenerationError> {
    let QuestionVariationRule::Seeded {
        generator,
        parameters,
    } = rule
    else {
        return Ok(QuestionVariationParameters {
            generator: None,
            parameters: BTreeMap::new(),
        });
    };

    let mut rng = ChaCha20Rng::from_seed(expand_seed(seed));
    let mut generated = BTreeMap::new();
    for (name, spec) in parameters {
        let value = generate_value(name, spec, &mut rng)?;
        generated.insert(name.clone(), value);
    }

    Ok(QuestionVariationParameters {
        generator: Some(generator.clone()),
        parameters: generated,
    })
}

/// Returns the canonical output hash without exposing serialization choices to callers.
///
/// # Errors
///
/// Returns the same errors as [`generate`] and [`QuestionVariationParameters::sha256`].
pub fn generate_hash(
    seed: QuestionSeed,
    rule: &QuestionVariationRule,
) -> Result<String, GenerationError> {
    generate(seed, rule)?.sha256()
}

/// Expands the stored 64-bit value into the exact key consumed by ChaCha20.
fn expand_seed(seed: QuestionSeed) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(SEED_DOMAIN);
    hasher.update(seed.value().to_le_bytes());
    hasher.finalize().into()
}

/// Generates one parameter without borrowing randomness from another source.
fn generate_value(
    name: &str,
    spec: &QuestionGeneratorParameter,
    rng: &mut ChaCha20Rng,
) -> Result<QuestionVariationParameterValue, GenerationError> {
    match spec {
        QuestionGeneratorParameter::IntegerRange { low, high } => {
            let value = sample_inclusive(name, *low, *high, rng)?;
            Ok(QuestionVariationParameterValue::Integer { value })
        }
        QuestionGeneratorParameter::DecimalRange {
            low,
            high,
            decimals,
        } => {
            if !low.is_finite() || !high.is_finite() || low > high {
                return Err(GenerationError::InvalidDecimalRange {
                    parameter: name.to_string(),
                });
            }
            let factor = 10_i64.checked_pow(u32::from(*decimals)).ok_or_else(|| {
                GenerationError::InvalidDecimalRange {
                    parameter: name.to_string(),
                }
            })?;
            let low_scaled = scale_decimal(name, *low, factor)?;
            let high_scaled = scale_decimal(name, *high, factor)?;
            let scaled = sample_inclusive(name, low_scaled, high_scaled, rng)?;
            Ok(QuestionVariationParameterValue::Decimal {
                value: format_decimal(scaled, factor, *decimals),
            })
        }
        QuestionGeneratorParameter::Choice { options } => {
            if options.is_empty() {
                return Err(GenerationError::EmptyChoice {
                    parameter: name.to_string(),
                });
            }
            let index = if options.len() == 1 {
                0
            } else {
                usize::try_from(sample_below(
                    rng,
                    u64::try_from(options.len()).map_err(|_| GenerationError::EmptyChoice {
                        parameter: name.to_string(),
                    })?,
                ))
                .map_err(|_| GenerationError::EmptyChoice {
                    parameter: name.to_string(),
                })?
            };
            Ok(QuestionVariationParameterValue::Choice {
                value: options[index].clone(),
            })
        }
        QuestionGeneratorParameter::Fixed { value } => Ok(QuestionVariationParameterValue::Fixed {
            value: value.clone(),
        }),
    }
}

/// Samples an inclusive signed range using only stable `RngCore` bytes.
fn sample_inclusive(
    name: &str,
    low: i64,
    high: i64,
    rng: &mut ChaCha20Rng,
) -> Result<i64, GenerationError> {
    if low > high {
        return Err(GenerationError::InvalidIntegerRange {
            parameter: name.to_string(),
            low,
            high,
        });
    }
    if low == high {
        return Ok(low);
    }

    let span = (i128::from(high) - i128::from(low) + 1) as u128;
    let offset = if span == 1_u128 << 64 {
        u128::from(rng.next_u64())
    } else {
        u128::from(sample_below(
            rng,
            u64::try_from(span).expect("non-full i64 span fits u64"),
        ))
    };
    let value = i128::from(low) + i128::try_from(offset).expect("u64 offset fits i128");
    Ok(i64::try_from(value).expect("sampled value stays inside authored i64 range"))
}

/// Samples `0..upper` without the modulo bias of `next_u64() % upper`.
fn sample_below(rng: &mut ChaCha20Rng, upper: u64) -> u64 {
    debug_assert!(upper > 0);
    let rejection_threshold = upper.wrapping_neg() % upper;
    loop {
        let candidate = rng.next_u64();
        if candidate >= rejection_threshold {
            return candidate % upper;
        }
    }
}

/// Converts an authored decimal bound to its exact scaled integer.
fn scale_decimal(name: &str, value: f64, factor: i64) -> Result<i64, GenerationError> {
    let scaled = value * factor as f64;
    let rounded = scaled.round();
    let tolerance = f64::EPSILON * scaled.abs().max(1.0) * 4.0;
    const I64_EXCLUSIVE_UPPER: f64 = 9_223_372_036_854_775_808.0;
    if !value.is_finite()
        || !scaled.is_finite()
        || (scaled - rounded).abs() > tolerance
        || rounded < i64::MIN as f64
        || rounded >= I64_EXCLUSIVE_UPPER
    {
        return Err(GenerationError::InvalidDecimalRange {
            parameter: name.to_string(),
        });
    }
    Ok(rounded as i64)
}

/// Formats a scaled decimal without target-dependent floating-point output.
fn format_decimal(scaled: i64, factor: i64, decimals: u8) -> String {
    if decimals == 0 {
        return scaled.to_string();
    }

    let signed = i128::from(scaled);
    let magnitude = signed.abs();
    let whole = magnitude / i128::from(factor);
    let fraction = magnitude % i128::from(factor);
    let sign = if signed < 0 { "-" } else { "" };
    format!(
        "{sign}{whole}.{fraction:0>width$}",
        width = usize::from(decimals)
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn static_generation_is_an_explicit_empty_variant() {
        let output = generate(QuestionSeed::new(42), &QuestionVariationRule::Static)
            .expect("static Question Variation Rules should generate");

        assert_eq!(
            output,
            QuestionVariationParameters {
                generator: None,
                parameters: BTreeMap::new(),
            }
        );
    }

    #[test]
    fn fixed_parameters_do_not_shift_random_draws() {
        let random = QuestionGeneratorParameter::IntegerRange {
            low: i64::MIN,
            high: i64::MAX,
        };
        let base = QuestionVariationRule::Seeded {
            generator: test_generator(),
            parameters: BTreeMap::from([("z_random".to_string(), random.clone())]),
        };
        let with_fixed = QuestionVariationRule::Seeded {
            generator: test_generator(),
            parameters: BTreeMap::from([
                (
                    "a_fixed".to_string(),
                    QuestionGeneratorParameter::Fixed {
                        value: "constant".to_string(),
                    },
                ),
                ("z_random".to_string(), random),
            ]),
        };

        let base_output =
            generate(QuestionSeed::new(7), &base).expect("base generation should succeed");
        let fixed_output =
            generate(QuestionSeed::new(7), &with_fixed).expect("fixed generation should succeed");
        assert_eq!(
            base_output.parameters.get("z_random"),
            fixed_output.parameters.get("z_random")
        );
    }

    #[test]
    fn invalid_parameter_shapes_are_refused() {
        let invalid_specs = [
            QuestionGeneratorParameter::IntegerRange { low: 2, high: 1 },
            QuestionGeneratorParameter::DecimalRange {
                low: 0.001,
                high: 0.001,
                decimals: 2,
            },
            QuestionGeneratorParameter::Choice {
                options: Vec::new(),
            },
        ];

        for spec in invalid_specs {
            let rule = QuestionVariationRule::Seeded {
                generator: test_generator(),
                parameters: BTreeMap::from([("invalid".to_string(), spec)]),
            };
            assert!(generate(QuestionSeed::new(1), &rule).is_err());
        }
    }

    fn test_generator() -> QuestionGeneratorReference {
        QuestionGeneratorReference {
            id: "test".to_string(),
            version: "1".to_string(),
        }
    }
}

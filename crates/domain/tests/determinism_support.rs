//! Shared native/WASM assertions for the committed Deterministic Seed Vector Fixture Set.

use std::collections::{BTreeMap, BTreeSet};

use domain::generator::{QuestionVariationParameterValue, generate};
use question_model::generation::{
    QuestionGeneratorParameter, QuestionGeneratorReference, QuestionSeed, QuestionVariationRule,
};
use serde::Deserialize;

/// Minimum vector count required for every registered generator.
const MINIMUM_SEEDS_PER_GENERATOR: usize = 50;

/// Versioned top-level seed-vector fixture.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct DeterministicSeedVectorFixtureSet {
    format_version: u32,
    generators: Vec<QuestionGeneratorSeedVectorSet>,
}

/// One generator rule and its committed expected hashes.
#[derive(Debug, Deserialize)]
struct QuestionGeneratorSeedVectorSet {
    generator: QuestionGeneratorReference,
    rule: QuestionVariationRule,
    vectors: Vec<SeedVector>,
}

/// Expected canonical-output hash for one seed.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SeedVector {
    seed: u64,
    expected_output_sha256: String,
}

/// Parameter-shape branches that the fixture set must exercise.
#[derive(Debug, Default)]
struct ParameterCoverage {
    integer_single: bool,
    integer_range: bool,
    integer_full_i64_range: bool,
    decimal_single: bool,
    decimal_range: bool,
    decimal_zero_places: bool,
    choice_single: bool,
    choice_multiple: bool,
    fixed: bool,
}

impl ParameterCoverage {
    /// Fails when a new or existing generation branch lacks a fixture case.
    fn assert_complete(&self, generator: &str) {
        assert!(
            self.integer_single,
            "{generator}: integer single-value branch missing"
        );
        assert!(
            self.integer_range,
            "{generator}: integer range branch missing"
        );
        assert!(
            self.integer_full_i64_range,
            "{generator}: full i64 range branch missing"
        );
        assert!(
            self.decimal_single,
            "{generator}: decimal single-value branch missing"
        );
        assert!(
            self.decimal_range,
            "{generator}: decimal range branch missing"
        );
        assert!(
            self.decimal_zero_places,
            "{generator}: zero-decimal formatting branch missing"
        );
        assert!(
            self.choice_single,
            "{generator}: choice single-value branch missing"
        );
        assert!(
            self.choice_multiple,
            "{generator}: multi-choice branch missing"
        );
        assert!(self.fixed, "{generator}: fixed-value branch missing");
    }
}

/// Runs the exact fixture set and assertions on the native and browser targets.
pub fn assert_committed_deterministic_seed_vector_fixture_set() {
    let seed_vector_fixture_set: DeterministicSeedVectorFixtureSet =
        serde_json::from_str(include_str!("seed_vectors.json"))
            .expect("seed vector fixture must be valid JSON");
    assert_eq!(
        seed_vector_fixture_set.format_version, 1,
        "unsupported seed-vector format"
    );
    assert!(
        !seed_vector_fixture_set.generators.is_empty(),
        "seed vector fixture set must name a generator"
    );

    let mut generator_ids = BTreeSet::new();
    for generator_seed_vector_set in seed_vector_fixture_set.generators {
        assert!(
            generator_ids.insert(generator_seed_vector_set.generator.clone()),
            "duplicate Question Generator Seed Vector Set: {}@{}",
            generator_seed_vector_set.generator.id,
            generator_seed_vector_set.generator.version
        );
        assert_question_generator_seed_vector_set(&generator_seed_vector_set);
    }
}

/// Verifies coverage and hashes, stopping at the first divergent seed.
fn assert_question_generator_seed_vector_set(
    generator_seed_vector_set: &QuestionGeneratorSeedVectorSet,
) {
    let QuestionVariationRule::Seeded {
        generator,
        parameters,
    } = &generator_seed_vector_set.rule
    else {
        panic!(
            "{}@{}: seed vector fixture rule must be seeded",
            generator_seed_vector_set.generator.id, generator_seed_vector_set.generator.version
        );
    };
    assert_eq!(
        generator, &generator_seed_vector_set.generator,
        "fixture generator must match its rule"
    );
    let generator_label = format!(
        "{}@{}",
        generator_seed_vector_set.generator.id, generator_seed_vector_set.generator.version
    );
    assert!(
        generator_seed_vector_set.vectors.len() >= MINIMUM_SEEDS_PER_GENERATOR,
        "{}: expected at least {MINIMUM_SEEDS_PER_GENERATOR} seeds, got {}",
        generator_label,
        generator_seed_vector_set.vectors.len()
    );
    assert_strict_seed_order(generator_seed_vector_set);
    parameter_coverage(parameters).assert_complete(&generator_label);

    let mut observed: BTreeMap<String, BTreeSet<String>> = parameters
        .keys()
        .map(|name| (name.clone(), BTreeSet::new()))
        .collect();

    for vector in &generator_seed_vector_set.vectors {
        assert_hash_shape(&generator_label, vector);
        let output = generate(
            QuestionSeed::new(vector.seed),
            &generator_seed_vector_set.rule,
        )
        .unwrap_or_else(|error| {
            panic!(
                "generator `{}` first divergent seed {}: generation failed: {error}",
                generator_label, vector.seed
            )
        });
        let actual = output.sha256().unwrap_or_else(|error| {
            panic!(
                "generator `{}` first divergent seed {}: hashing failed: {error}",
                generator_label, vector.seed
            )
        });
        assert_eq!(
            actual, vector.expected_output_sha256,
            "generator `{}` first divergent seed {}",
            generator_label, vector.seed
        );
        for (name, value) in output.parameters {
            observed
                .get_mut(&name)
                .expect("generated parameter must come from the rule")
                .insert(observed_value(value));
        }
    }

    assert_observed_variation(&generator_label, parameters, &observed);
}

/// Confirms vectors have one deterministic first-divergence order.
fn assert_strict_seed_order(generator_seed_vector_set: &QuestionGeneratorSeedVectorSet) {
    for pair in generator_seed_vector_set.vectors.windows(2) {
        assert!(
            pair[0].seed < pair[1].seed,
            "{}@{}: seed vectors must be strictly increasing",
            generator_seed_vector_set.generator.id,
            generator_seed_vector_set.generator.version
        );
    }
}

/// Rejects missing, truncated, or non-hex expected hashes before comparison.
fn assert_hash_shape(generator: &str, vector: &SeedVector) {
    assert!(
        vector.expected_output_sha256.len() == 64
            && vector
                .expected_output_sha256
                .bytes()
                .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase()),
        "{generator}: seed {} has an invalid expected SHA-256",
        vector.seed
    );
}

/// Inspects the authored rule so every implementation branch is required.
fn parameter_coverage(
    parameters: &BTreeMap<String, QuestionGeneratorParameter>,
) -> ParameterCoverage {
    let mut coverage = ParameterCoverage::default();
    for spec in parameters.values() {
        match spec {
            QuestionGeneratorParameter::IntegerRange { low, high } => {
                coverage.integer_single |= low == high;
                coverage.integer_range |= low < high;
                coverage.integer_full_i64_range |= *low == i64::MIN && *high == i64::MAX;
            }
            QuestionGeneratorParameter::DecimalRange {
                low,
                high,
                decimals,
            } => {
                coverage.decimal_single |= low == high;
                coverage.decimal_range |= low < high;
                coverage.decimal_zero_places |= *decimals == 0;
            }
            QuestionGeneratorParameter::Choice { options } => {
                coverage.choice_single |= options.len() == 1;
                coverage.choice_multiple |= options.len() > 1;
            }
            QuestionGeneratorParameter::Fixed { .. } => coverage.fixed = true,
        }
    }
    coverage
}

/// Requires random branches to vary and deterministic branches to remain fixed.
fn assert_observed_variation(
    generator: &str,
    parameters: &BTreeMap<String, QuestionGeneratorParameter>,
    observed: &BTreeMap<String, BTreeSet<String>>,
) {
    for (name, spec) in parameters {
        let count = observed
            .get(name)
            .expect("every authored parameter must be observed")
            .len();
        let should_vary = match spec {
            QuestionGeneratorParameter::IntegerRange { low, high } => low < high,
            QuestionGeneratorParameter::DecimalRange { low, high, .. } => low < high,
            QuestionGeneratorParameter::Choice { options } => options.len() > 1,
            QuestionGeneratorParameter::Fixed { .. } => false,
        };
        if should_vary {
            assert!(count > 1, "{generator}: parameter {name} never varied");
        } else {
            assert_eq!(count, 1, "{generator}: fixed parameter {name} varied");
        }
    }
}

/// Stable display used only to count distinct values observed by the fixture set.
fn observed_value(value: QuestionVariationParameterValue) -> String {
    match value {
        QuestionVariationParameterValue::Integer { value } => format!("integer:{value}"),
        QuestionVariationParameterValue::Decimal { value } => format!("decimal:{value}"),
        QuestionVariationParameterValue::Choice { value } => format!("choice:{value}"),
        QuestionVariationParameterValue::Fixed { value } => format!("fixed:{value}"),
    }
}

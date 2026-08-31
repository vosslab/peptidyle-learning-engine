//! Shared native/WASM assertions for the committed determinism corpus.

use std::collections::{BTreeMap, BTreeSet};

use domain::generator::{QuestionVariationParameterValue, generate};
use question_model::generation::{
    QuestionGeneratorParameter, QuestionGeneratorReference, QuestionSeed,
    QuestionVariationDefinition,
};
use serde::Deserialize;

/// Minimum corpus size required for every registered generator.
const MINIMUM_SEEDS_PER_GENERATOR: usize = 50;

/// Versioned top-level seed-vector fixture.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SeedCorpus {
    format_version: u32,
    generators: Vec<GeneratorCorpus>,
}

/// One generator definition and its committed expected hashes.
#[derive(Debug, Deserialize)]
struct GeneratorCorpus {
    generator: QuestionGeneratorReference,
    definition: QuestionVariationDefinition,
    vectors: Vec<SeedVector>,
}

/// Expected canonical-output hash for one seed.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SeedVector {
    seed: u64,
    expected_output_sha256: String,
}

/// Parameter-shape branches that the corpus must exercise.
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

/// Runs the exact same corpus and assertions on the native and browser targets.
pub fn assert_committed_seed_vectors() {
    let corpus: SeedCorpus = serde_json::from_str(include_str!("seed_vectors.json"))
        .expect("seed vector fixture must be valid JSON");
    assert_eq!(corpus.format_version, 1, "unsupported seed-vector format");
    assert!(
        !corpus.generators.is_empty(),
        "seed corpus must name a generator"
    );

    let mut generator_ids = BTreeSet::new();
    for generator in corpus.generators {
        assert!(
            generator_ids.insert(generator.generator.clone()),
            "duplicate generator corpus: {}@{}",
            generator.generator.id,
            generator.generator.version
        );
        assert_generator_corpus(&generator);
    }
}

/// Verifies coverage and hashes, stopping at the first divergent seed.
fn assert_generator_corpus(corpus: &GeneratorCorpus) {
    let QuestionVariationDefinition::Seeded {
        generator,
        parameters,
    } = &corpus.definition
    else {
        panic!(
            "{}@{}: corpus definition must be seeded",
            corpus.generator.id, corpus.generator.version
        );
    };
    assert_eq!(
        generator, &corpus.generator,
        "fixture generator must match its definition"
    );
    let generator_label = format!("{}@{}", corpus.generator.id, corpus.generator.version);
    assert!(
        corpus.vectors.len() >= MINIMUM_SEEDS_PER_GENERATOR,
        "{}: expected at least {MINIMUM_SEEDS_PER_GENERATOR} seeds, got {}",
        generator_label,
        corpus.vectors.len()
    );
    assert_strict_seed_order(corpus);
    parameter_coverage(parameters).assert_complete(&generator_label);

    let mut observed: BTreeMap<String, BTreeSet<String>> = parameters
        .keys()
        .map(|name| (name.clone(), BTreeSet::new()))
        .collect();

    for vector in &corpus.vectors {
        assert_hash_shape(&generator_label, vector);
        let output =
            generate(QuestionSeed::new(vector.seed), &corpus.definition).unwrap_or_else(|error| {
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
                .expect("generated parameter must come from the definition")
                .insert(observed_value(value));
        }
    }

    assert_observed_variation(&generator_label, parameters, &observed);
}

/// Confirms vectors have one deterministic first-divergence order.
fn assert_strict_seed_order(corpus: &GeneratorCorpus) {
    for pair in corpus.vectors.windows(2) {
        assert!(
            pair[0].seed < pair[1].seed,
            "{}@{}: seed vectors must be strictly increasing",
            corpus.generator.id,
            corpus.generator.version
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

/// Inspects the authored definition so every implementation branch is required.
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

/// Stable display used only to count distinct values observed by the corpus.
fn observed_value(value: QuestionVariationParameterValue) -> String {
    match value {
        QuestionVariationParameterValue::Integer { value } => format!("integer:{value}"),
        QuestionVariationParameterValue::Decimal { value } => format!("decimal:{value}"),
        QuestionVariationParameterValue::Choice { value } => format!("choice:{value}"),
        QuestionVariationParameterValue::Fixed { value } => format!("fixed:{value}"),
    }
}

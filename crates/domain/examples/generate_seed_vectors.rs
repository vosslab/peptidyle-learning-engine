//! Regenerates the reviewed Deterministic Seed Vector Fixture Set.

use std::error::Error;
use std::fs;
use std::path::PathBuf;

use domain::generator::generate_hash;
use question_model::generation::{QuestionGeneratorReference, QuestionSeed, QuestionVariationRule};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct DeterministicSeedVectorFixtureSet {
    format_version: u32,
    generators: Vec<QuestionGeneratorSeedVectorSet>,
}

#[derive(Debug, Serialize, Deserialize)]
struct QuestionGeneratorSeedVectorSet {
    generator: QuestionGeneratorReference,
    rule: QuestionVariationRule,
    vectors: Vec<SeedVector>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SeedVector {
    seed: u64,
    expected_output_sha256: String,
}

fn main() -> Result<(), Box<dyn Error>> {
    let fixture_path = fixture_path();
    let source = fs::read_to_string(&fixture_path)?;
    let mut seed_vector_fixture_set: DeterministicSeedVectorFixtureSet =
        serde_json::from_str(&source)?;

    for generator_seed_vector_set in &mut seed_vector_fixture_set.generators {
        for vector in &mut generator_seed_vector_set.vectors {
            vector.expected_output_sha256 = generate_hash(
                QuestionSeed::new(vector.seed),
                &generator_seed_vector_set.rule,
            )?;
        }
    }

    let output = format!(
        "{}\n",
        serde_json::to_string_pretty(&seed_vector_fixture_set)?
    );
    match std::env::args().nth(1).as_deref() {
        None => print!("{output}"),
        Some("--write") => {
            fs::write(&fixture_path, output)?;
            println!("updated {}", fixture_path.display());
        }
        Some(argument) => return Err(format!("unknown argument: {argument}").into()),
    }
    Ok(())
}

fn fixture_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/seed_vectors.json")
}

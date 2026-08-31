//! Regenerates the reviewed WP-C5 seed-vector baseline.

use std::error::Error;
use std::fs;
use std::path::PathBuf;

use domain::generator::generate_hash;
use question_model::generation::{
    QuestionGeneratorReference, QuestionSeed, QuestionVariationDefinition,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SeedCorpus {
    format_version: u32,
    generators: Vec<GeneratorCorpus>,
}

#[derive(Debug, Serialize, Deserialize)]
struct GeneratorCorpus {
    generator: QuestionGeneratorReference,
    definition: QuestionVariationDefinition,
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
    let mut corpus: SeedCorpus = serde_json::from_str(&source)?;

    for generator in &mut corpus.generators {
        for vector in &mut generator.vectors {
            vector.expected_output_sha256 =
                generate_hash(QuestionSeed::new(vector.seed), &generator.definition)?;
        }
    }

    let output = format!("{}\n", serde_json::to_string_pretty(&corpus)?);
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

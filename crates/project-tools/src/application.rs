//! Project-tools command dispatch and owned CLI implementations.
//!
//! Run through the pipeline scripts rather than directly:
//!
//! ```text
//! ./pipeline/build_wasm.sh
//! ```
//!
//! Usage:
//!
//! ```text
//! cargo tools bindgen <input.wasm> <web|node> <out-dir> <out-name>
//! cargo tools fixtures <--check|--write>
//! cargo tools tsgen [model-dir] [out-dir]
//! cargo tools e2e-seed --database-url <URL> --apply-migrations --tenant <UUID> --instructor <UUID> --student <UUID>
//! # Explicit renderer-acceptance fixture only (not normal local-stack seeding):
//! cargo tools e2e-seed --webwork-pilot --database-url <URL> --apply-migrations
//! --tenant <UUID> --instructor <UUID> --student <UUID> --s3-endpoint <URL>
//! --s3-region <REGION> --private-content-bucket <BUCKET>
//! ```

use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use wasm_bindgen_cli_support::Bindgen;

use crate::{database, e2e_seed, fixtures, pilot_content, tsgen};

/// Where the Rust question model lives, relative to the repo root.
const DEFAULT_MODEL_DIR: &str = "crates/question_model/src";

/// Where the generated TypeScript goes, relative to the repo root.
///
/// Root-level build output, ignored by Git and regenerated before every build
/// and check. It remains in the TypeScript, ESLint, and Prettier validation
/// scopes even though it is not authored source.
const DEFAULT_TS_OUT_DIR: &str = "generated/api";

/// Where intentional, tracked fixture evidence lives.
const DEFAULT_FIXTURE_DIR: &str = "tests/fixtures/published_problem";

/// Where the derivative TypeScript fixture projection is generated.
const DEFAULT_FIXTURE_TS: &str = "generated/fixtures/published_problem.ts";

pub(crate) fn run() -> Result<()> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let Some(command) = args.first() else {
        bail!("usage: cargo tools <bindgen|database|fixtures|pilot-content|tsgen|e2e-seed> ...");
    };

    match command.as_str() {
        "bindgen" => run_bindgen(&args[1..]),
        "database" => database::run(&args[1..]),
        "fixtures" => run_fixtures(&args[1..]),
        "pilot-content" => pilot_content::run(&args[1..]),
        "tsgen" => run_tsgen(&args[1..]),
        "e2e-seed" => e2e_seed::run(&args[1..]),
        other => bail!("unknown command: {other}"),
    }
}

/// Checks or deliberately refreshes the WP-C7 fixture corpus.
fn run_fixtures(args: &[String]) -> Result<()> {
    let mode = match args {
        [flag] if flag == "--check" => fixtures::Mode::Check,
        [flag] if flag == "--write" => fixtures::Mode::Write,
        _ => bail!("usage: cargo tools fixtures <--check|--write>"),
    };

    let report = fixtures::run(
        Path::new(DEFAULT_FIXTURE_DIR),
        Path::new(DEFAULT_FIXTURE_TS),
        mode,
    )?;
    println!(
        "fixtures: {} {} tracked file(s); wrote {}",
        report.action, report.tracked_files, DEFAULT_FIXTURE_TS
    );
    Ok(())
}

/// Regenerates the TypeScript definitions for the question model.
fn run_tsgen(args: &[String]) -> Result<()> {
    let model_dir = args.first().map_or(DEFAULT_MODEL_DIR, String::as_str);
    let out_dir = args.get(1).map_or(DEFAULT_TS_OUT_DIR, String::as_str);

    let count = tsgen::run(Path::new(model_dir), Path::new(out_dir))
        .with_context(|| format!("generating TypeScript from {model_dir}"))?;

    println!("tsgen: wrote {count} type(s) to {out_dir}");
    Ok(())
}

/// Generates the JavaScript glue and the processed `.wasm` for one flavor.
///
/// Two flavors exist because the consumers differ: the browser client loads
/// the `web` output as an ES module, and the WP-F2 Node gate loads the `node`
/// output through CommonJS. Both come from the same compiled module.
fn run_bindgen(args: &[String]) -> Result<()> {
    let [input, flavor, out_dir, out_name] = args else {
        bail!("usage: cargo tools bindgen <input.wasm> <web|node> <out-dir> <out-name>");
    };

    let input_path = PathBuf::from(input);
    if !input_path.is_file() {
        bail!("input module not found: {input}");
    }

    let mut bindgen = Bindgen::new();
    bindgen
        .input_path(&input_path)
        .out_name(out_name)
        // Keep debug info out of the generated glue; it is build output, not
        // something a human reads.
        .debug(false)
        .keep_debug(false);

    match flavor.as_str() {
        "web" => {
            bindgen.web(true).context("selecting the web target")?;
        }
        "node" => {
            bindgen
                .nodejs(true)
                .context("selecting the nodejs target")?;
        }
        other => bail!("unknown flavor: {other} (expected web or node)"),
    }

    bindgen
        .generate(out_dir)
        .with_context(|| format!("generating {flavor} bindings into {out_dir}"))?;

    Ok(())
}

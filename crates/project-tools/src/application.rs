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
//! cargo tools fixtures --check
//! cargo tools tsgen [out-dir]
//! ```

use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail, ensure};
use wasm_bindgen_cli_support::Bindgen;

use crate::{database, fixtures, pilot_content, tsgen};

/// Rust roots that own generated browser contract declarations, relative to the repo root.
const DEFAULT_CONTRACT_ROOTS: [&str; 2] = [
    "crates/question_model/src",
    "crates/browser-api-contract/src",
];

/// Where the generated TypeScript goes, relative to the repo root.
///
/// Root-level build output, ignored by Git and regenerated before every build
/// and check. It remains in the TypeScript, ESLint, and Prettier validation
/// scopes even though it is not authored source.
const DEFAULT_TS_OUT_DIR: &str = "generated/api";

/// Where intentional, tracked fixture evidence lives.
const DEFAULT_FIXTURE_DIR: &str = "tests/fixtures/published_question";

pub(crate) fn run() -> Result<()> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let Some(command) = args.first() else {
        bail!("usage: cargo tools <bindgen|database|fixtures|pilot-content|tsgen> ...");
    };

    match command.as_str() {
        "bindgen" => run_bindgen(&args[1..]),
        "database" => database::run(&args[1..]),
        "fixtures" => run_fixtures(&args[1..]),
        "pilot-content" => pilot_content::run(&args[1..]),
        "tsgen" => run_tsgen(&args[1..]),
        other => bail!("unknown command: {other}"),
    }
}

/// Checks the stored published-Question fixture data.
fn run_fixtures(args: &[String]) -> Result<()> {
    let [flag] = args else {
        bail!("usage: cargo tools fixtures --check");
    };
    ensure!(flag == "--check", "usage: cargo tools fixtures --check");

    let report = fixtures::run(Path::new(DEFAULT_FIXTURE_DIR))?;
    println!(
        "fixtures: {} {} tracked file(s)",
        report.action, report.tracked_files
    );
    Ok(())
}

/// Regenerates the TypeScript definitions for the application-owned contract roots.
fn run_tsgen(args: &[String]) -> Result<()> {
    let out_dir = match args {
        [] => DEFAULT_TS_OUT_DIR,
        [out_dir] => out_dir,
        _ => bail!("usage: cargo tools tsgen [out-dir]"),
    };
    let contract_roots: Vec<&Path> = DEFAULT_CONTRACT_ROOTS.iter().map(Path::new).collect();
    let root_names = DEFAULT_CONTRACT_ROOTS.join(", ");

    let count = tsgen::run(&contract_roots, Path::new(out_dir))
        .with_context(|| format!("generating TypeScript from contract roots {root_names}"))?;

    println!("tsgen: wrote {count} type(s) to {out_dir}");
    Ok(())
}

/// Generates the JavaScript glue and the processed `.wasm` for one flavor.
///
/// Two flavors exist because the consumers differ: the browser client loads
/// the `web` output as an ES module, and the Node binding check loads the `node`
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

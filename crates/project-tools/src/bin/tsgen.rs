//! Source-only TypeScript contract generator.
//!
//! This binary has no runtime-tool feature dependencies.  It parses the two
//! explicit Rust contract roots and writes the ignored browser declaration
//! output, preserving the optional output-directory argument from the older
//! runtime-host subcommand. Use `cargo tsgen [out-dir]` for this lightweight
//! front door.

use std::path::Path;

use anyhow::{Context, Result, bail};

use project_tools::tsgen;

/// Rust roots that own generated browser contract declarations, relative to the repository root.
const DEFAULT_CONTRACT_ROOTS: [&str; 2] = [
    "crates/question_model/src",
    "crates/browser-api-contract/src",
];

/// Root-level generated TypeScript output, relative to the repository root.
const DEFAULT_TS_OUT_DIR: &str = "generated/api";

fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let out_dir = match args.as_slice() {
        [] => DEFAULT_TS_OUT_DIR,
        [out_dir] => out_dir,
        _ => bail!("usage: cargo tsgen [out-dir]"),
    };
    let contract_roots: Vec<&Path> = DEFAULT_CONTRACT_ROOTS.iter().map(Path::new).collect();
    let root_names = DEFAULT_CONTRACT_ROOTS.join(", ");
    let count = tsgen::run(&contract_roots, Path::new(out_dir))
        .with_context(|| format!("generating TypeScript from contract roots {root_names}"))?;

    println!("tsgen: wrote {count} type(s) to {out_dir}");
    Ok(())
}

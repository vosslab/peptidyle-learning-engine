//! Repository automation tasks (WP-F2).
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
//! cargo run -p xtask -- bindgen <input.wasm> <flavor> <out-dir> <out-name>
//! ```
//!
//! `flavor` is `web` or `node`.

use std::path::PathBuf;

use anyhow::{Context, Result, bail};
use wasm_bindgen_cli_support::Bindgen;

fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let Some(command) = args.first() else {
        bail!("usage: xtask bindgen <input.wasm> <web|node> <out-dir> <out-name>");
    };

    match command.as_str() {
        "bindgen" => run_bindgen(&args[1..]),
        other => bail!("unknown command: {other}"),
    }
}

/// Generates the JavaScript glue and the processed `.wasm` for one flavor.
///
/// Two flavors exist because the consumers differ: the browser client loads
/// the `web` output as an ES module, and the WP-F2 Node gate loads the `node`
/// output through CommonJS. Both come from the same compiled module.
fn run_bindgen(args: &[String]) -> Result<()> {
    let [input, flavor, out_dir, out_name] = args else {
        bail!("usage: xtask bindgen <input.wasm> <web|node> <out-dir> <out-name>");
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

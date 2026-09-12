//! Command-line behavior for validating and publishing Pilot content.

use std::path::Path;

use anyhow::{Result, bail};

use super::{DEFAULT_MANIFEST, publish, validate};

pub(crate) fn run(args: &[String]) -> Result<()> {
    if matches!(args, [command] if command == "publish") {
        return publish();
    }
    let manifest = match args {
        [] => Path::new(DEFAULT_MANIFEST),
        _ => bail!("usage: cargo tools pilot-content [publish]"),
    };
    let report = validate(manifest)?;
    println!(
        "pilot content: {} chapters, {} reviewed questions, {} adapted PGML sources",
        report.chapters.len(),
        report.question_count,
        report.adapted_source_count
    );
    for chapter in report.chapters {
        println!(
            "- {chapter}: WeBWorK MC, WeBWorK MATCH, PLE Question JSON MC, PLE Question JSON MATCH"
        );
    }
    Ok(())
}

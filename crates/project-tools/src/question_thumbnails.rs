//! Explicit existing-Revision thumbnail maintenance command.

use std::io::{self, Write};

use anyhow::{Context, Result, bail};
use server_core::question_thumbnail_backfill::QuestionThumbnailBackfillOutcome;

const USAGE: &str = "usage: cargo tools question-thumbnails backfill (no additional arguments; reads existing migration/storage/renderer configuration)";

pub(crate) fn run(args: &[String]) -> Result<()> {
    if matches!(args, [flag] if flag == "--help" || flag == "-h") {
        println!("{USAGE}");
        return Ok(());
    }
    match args {
        [action] if action == "backfill" => {}
        _ => bail!(USAGE),
    }
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .context("creating thumbnail maintenance runtime")?;
    let mut output = io::stdout().lock();
    let mut output_failed = false;
    let totals = runtime.block_on(
        server_core::composition::backfill_question_thumbnails_from_env(|revision, outcome| {
            let identity = format!(
                "{} revision {}",
                revision.published_question_id,
                revision.revision_number.get()
            );
            let line = match outcome {
                QuestionThumbnailBackfillOutcome::Created => format!("{identity}: created"),
                QuestionThumbnailBackfillOutcome::AlreadyComplete => {
                    format!("{identity}: already-complete")
                }
                QuestionThumbnailBackfillOutcome::Failed { stage, reason } => {
                    format!("{identity}: failed stage={stage} reason={reason}")
                }
            };
            // Flush each exact result while a potentially long finite run continues.
            if writeln!(output, "{line}")
                .and_then(|()| output.flush())
                .is_err()
            {
                output_failed = true;
            }
        }),
    )?;
    writeln!(
        output,
        "question-thumbnails backfill: created={} already-complete={} failed={}",
        totals.created, totals.already_complete, totals.failed
    )?;
    output.flush()?;
    if output_failed {
        bail!("thumbnail maintenance progress output failed");
    }
    if totals.failed != 0 {
        bail!(
            "thumbnail maintenance has {} failed Revisions; repair their recorded stages and rerun",
            totals.failed
        );
    }
    Ok(())
}

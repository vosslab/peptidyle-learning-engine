//! Fixed point-only CSV/quoted-TSV representation of authorized Gradebook evidence.

use std::borrow::Cow;

use learning_data_access::{CourseGradebook, CourseGradebookStudentWork};
use question_model::AssessmentAttemptCompletion;
use serde::Deserialize;

#[derive(Clone, Copy, Deserialize)]
#[serde(rename_all = "lowercase")]
pub(super) enum Format {
    Csv,
    Tsv,
}

impl Format {
    pub(super) fn extension(self) -> &'static str {
        match self {
            Self::Csv => "csv",
            Self::Tsv => "tsv",
        }
    }

    pub(super) fn media_type(self) -> &'static str {
        match self {
            Self::Csv => "text/csv; charset=utf-8",
            Self::Tsv => "text/tab-separated-values; charset=utf-8",
        }
    }

    fn delimiter(self) -> u8 {
        match self {
            Self::Csv => b',',
            Self::Tsv => b'\t',
        }
    }
}

const COLUMNS: [&str; 7] = [
    "roster_id",
    "roster_name",
    "assessment_reference",
    "assessment_title",
    "status",
    "points_earned",
    "points_possible",
];

pub(super) fn encode(gradebook: &CourseGradebook, format: Format) -> anyhow::Result<Vec<u8>> {
    let mut rows: Vec<_> = gradebook.student_work.iter().collect();
    rows.sort_by(|left, right| {
        left.roster_id
            .as_bytes()
            .cmp(right.roster_id.as_bytes())
            .then_with(|| {
                left.assessment_reference
                    .to_string()
                    .cmp(&right.assessment_reference.to_string())
            })
    });
    // ASVS 1.1.2/1.2.10: escape at the final output boundary using the CSV writer.
    let mut writer = csv::WriterBuilder::new()
        .delimiter(format.delimiter())
        .quote_style(csv::QuoteStyle::Always)
        .terminator(csv::Terminator::CRLF)
        .from_writer(Vec::new());
    writer.write_record(COLUMNS)?;
    for row in rows {
        let status = status(row)?;
        let (earned, possible) = match &row.score {
            Some(score) => (
                point_text(score.points_earned)?,
                point_text(score.points_possible)?,
            ),
            None => (String::new(), String::new()),
        };
        // ASVS 14.2.6: explicitly select only the seven authorized export fields.
        let cells = [
            spreadsheet_text(&row.roster_id),
            spreadsheet_text(&row.roster_name),
            Cow::Owned(row.assessment_reference.to_string()),
            spreadsheet_text(&row.assessment_title),
            Cow::Borrowed(status),
            Cow::Owned(earned),
            Cow::Owned(possible),
        ];
        writer.write_record(cells.iter().map(|cell| cell.as_bytes()))?;
    }
    Ok(writer.into_inner()?)
}

fn status(row: &CourseGradebookStudentWork) -> anyhow::Result<&'static str> {
    if row.score.is_some()
        && row.assessment_attempt_completion != Some(AssessmentAttemptCompletion::Completed)
    {
        anyhow::bail!("Unsubmitted Gradebook evidence cannot carry a score");
    }
    Ok(match row.assessment_attempt_completion {
        Some(AssessmentAttemptCompletion::Completed) => "submitted",
        Some(AssessmentAttemptCompletion::InProgress) if row.expired_submitting => {
            "expired_submitting"
        }
        Some(AssessmentAttemptCompletion::InProgress) => "in_progress",
        None => "not_started",
    })
}

fn point_text(value: f64) -> anyhow::Result<String> {
    if !value.is_finite() || value < 0.0 {
        anyhow::bail!("Gradebook points must be finite and nonnegative");
    }
    Ok(if value == 0.0 {
        "0".to_owned()
    } else {
        value.to_string()
    })
}

fn spreadsheet_text(value: &str) -> Cow<'_, str> {
    // ASVS 1.2.10: quotes alone do not stop spreadsheet formulas. Preserve the
    // original label after the safety prefix, including whitespace and controls.
    let direct_trigger = value.starts_with(['=', '+', '-', '@', '\t', '\r', '\n', '\0']);
    let trimmed = value.trim_start_matches(|character: char| {
        character.is_whitespace() || character.is_ascii_control()
    });
    if direct_trigger || trimmed.starts_with(['=', '+', '-', '@']) {
        Cow::Owned(format!("'{value}"))
    } else {
        Cow::Borrowed(value)
    }
}

#[cfg(test)]
#[path = "export_tests.rs"]
mod tests;

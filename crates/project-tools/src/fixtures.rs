//! Validation for stored Question fixture evidence.
//!
//! Question content is data. The tracked JSON and asset files are the fixture
//! authority; this module loads them through production domain types and
//! validates their relationships. Executable source never authors or rewrites
//! the Question represented by that stored data.

use std::collections::BTreeSet;
use std::fs;
use std::path::{Component, Path, PathBuf};

use anyhow::{Context, Result, bail, ensure};
use question_model::{
    AssignmentAttempt, AssignmentGrade, AssignmentProgressRecord, AssignmentSummary,
    GradebookSummaryRow, IssuedQuestion, QuestionAttempt, QuestionRevisionReference,
    QuestionSummary, SourceObjectChecksum, SourceObjectReference, StudentRecordId,
};
use serde::Deserialize;
use sha2::{Digest, Sha256};

const FIXTURE_SET_FILENAME: &str = "fixture_set.json";
const ASSET_DIRECTORY: &str = "assets";
const MAX_FIXTURE_SET_BYTES: u64 = 4 * 1024 * 1024;
const MAX_ASSET_BYTES: u64 = 16 * 1024 * 1024;
const FIXTURE_SCHEMA_VERSION: u32 = 4;
const MODEL_SCHEMA_VERSION: u32 = 1;

/// Summary printed by the project-tools command.
pub struct Report {
    /// Human-readable action for the terminal summary.
    pub action: &'static str,
    /// Number of stored fixture files validated.
    pub tracked_files: usize,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct FixtureAsset {
    object: question_model::ObjectId,
    filename: String,
    media_type: String,
    sha256: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct StoredFixtureSet {
    fixture_schema_version: u32,
    model_schema_version: u32,
    source_object_reference: SourceObjectReference,
    source_object_checksum: SourceObjectChecksum,
    question_summary: QuestionSummary,
    assets: Vec<FixtureAsset>,
    course: question_model::CourseSummary,
    assignment: AssignmentSummary,
    student_record: StudentRecordId,
    #[serde(rename = "assignment_attempts")]
    assignment_attempts: Vec<AssignmentAttempt>,
    issued_questions: Vec<IssuedQuestion>,
    attempts: Vec<QuestionAttempt>,
    assignment_grade: AssignmentGrade,
    assignment_progress: AssignmentProgressRecord,
    gradebook: Vec<GradebookSummaryRow>,
}

/// Validates the stored fixture data without generating Question content.
pub fn run(fixture_dir: &Path) -> Result<Report> {
    let fixture_set = load_fixture_set(fixture_dir)?;
    validate_fixture_set(fixture_dir, &fixture_set)?;
    Ok(Report {
        action: "validated",
        tracked_files: 1 + fixture_set.assets.len(),
    })
}

fn load_fixture_set(fixture_dir: &Path) -> Result<StoredFixtureSet> {
    // ASVS 5.3.2: the filename is application-owned and the caller supplies
    // the repository-owned fixture root rather than a user-controlled path.
    let path = fixture_dir.join(FIXTURE_SET_FILENAME);
    let bytes = read_bounded(&path, MAX_FIXTURE_SET_BYTES)?;
    // ASVS 1.5.2 and 2.2.1: deserialize into one closed, typed structure and
    // validate its schema and domain relationships before using the data.
    serde_json::from_slice(&bytes).with_context(|| format!("decoding {}", path.display()))
}

fn validate_fixture_set(fixture_dir: &Path, fixture_set: &StoredFixtureSet) -> Result<()> {
    ensure!(
        fixture_set.fixture_schema_version == FIXTURE_SCHEMA_VERSION,
        "{} uses fixture schema {}; expected {}",
        fixture_dir.display(),
        fixture_set.fixture_schema_version,
        FIXTURE_SCHEMA_VERSION
    );
    ensure!(
        fixture_set.model_schema_version == MODEL_SCHEMA_VERSION,
        "{} uses model schema {}; expected {}",
        fixture_dir.display(),
        fixture_set.model_schema_version,
        MODEL_SCHEMA_VERSION
    );
    ensure!(
        !fixture_set.assets.is_empty(),
        "stored Question fixture has no assets"
    );
    ensure!(
        !fixture_set.assignment_attempts.is_empty(),
        "stored fixture has no Assignment Attempts"
    );
    ensure!(
        fixture_set.issued_questions.len() == fixture_set.attempts.len(),
        "stored fixture must pair every Question Attempt with one Issued Question"
    );

    let asset_root = fixture_dir.join(ASSET_DIRECTORY);
    for asset in &fixture_set.assets {
        validate_asset(&asset_root, asset)?;
    }

    ensure!(
        fixture_set.course.id == fixture_set.assignment.course_id,
        "stored Assignment must belong to the stored Course Instance"
    );
    ensure!(
        fixture_set.student_record == fixture_set.assignment_grade.student_record
            && fixture_set.student_record == fixture_set.assignment_progress.student_record,
        "stored Assignment Grade and Assignment Progress must belong to the stored Student Record"
    );
    ensure!(
        fixture_set.assignment_grade.assignment == fixture_set.assignment.id
            && fixture_set.assignment_progress.assignment == fixture_set.assignment.id,
        "stored Assignment Grade and Assignment Progress must belong to the stored Assignment"
    );
    ensure!(
        !fixture_set.gradebook.is_empty(),
        "stored fixture must include Gradebook evidence"
    );
    for row in &fixture_set.gradebook {
        ensure!(
            row.course_id == fixture_set.course.id
                && row.assignment_id == fixture_set.assignment.id
                && row.student_record_id == fixture_set.student_record,
            "stored Gradebook row must match its Course, Assignment, and Student Record"
        );
    }

    let seeds: BTreeSet<u64> = fixture_set
        .attempts
        .iter()
        .map(|attempt| attempt.question_seed.value())
        .collect();
    ensure!(
        seeds.len() == fixture_set.attempts.len(),
        "stored Question Attempts must use distinct fixture seeds"
    );
    for attempt in &fixture_set.attempts {
        ensure!(
            fixture_set
                .issued_questions
                .iter()
                .any(|issued_question| issued_question.id == attempt.issued_question),
            "Question Attempt references an absent Issued Question"
        );
        ensure!(
            attempt
                .reproduction_details
                .source_object_reference
                .as_ref()
                == Some(&fixture_set.source_object_reference)
                && attempt.reproduction_details.source_object_checksum.as_ref()
                    == Some(&fixture_set.source_object_checksum),
            "Question Attempt reproduction must retain the fixture source identity"
        );
        ensure!(
            attempt.reproduction_details.asset_objects
                == fixture_set
                    .assets
                    .iter()
                    .map(|asset| asset.object)
                    .collect::<Vec<_>>(),
            "Question Attempt reproduction must retain the fixture asset objects"
        );
    }

    ensure!(
        fixture_set.question_summary.latest_question_revision
            == QuestionRevisionReference {
                question_id: fixture_set.question_summary.question_id.clone(),
                revision_number: fixture_set
                    .question_summary
                    .latest_question_revision
                    .revision_number,
            },
        "Question Summary Latest Question Revision must name its Question lineage"
    );
    Ok(())
}

fn validate_asset(asset_root: &Path, asset: &FixtureAsset) -> Result<()> {
    let relative = validate_asset_filename(&asset.filename)?;
    validate_media_type(&asset.media_type, &relative)?;

    // ASVS 5.3.2: resolve only a validated single-component filename beneath
    // the repository-owned asset directory, then confirm the canonical path.
    let canonical_root = asset_root
        .canonicalize()
        .with_context(|| format!("resolving {}", asset_root.display()))?;
    let path = asset_root.join(&relative);
    let canonical_path = path
        .canonicalize()
        .with_context(|| format!("resolving {}", path.display()))?;
    ensure!(
        canonical_path.starts_with(&canonical_root),
        "stored fixture asset escapes {}",
        asset_root.display()
    );
    let bytes = read_bounded(&canonical_path, MAX_ASSET_BYTES)?;
    ensure!(
        sha256(&bytes) == asset.sha256,
        "stored fixture asset checksum mismatch for {}",
        canonical_path.display()
    );
    Ok(())
}

fn validate_asset_filename(filename: &str) -> Result<PathBuf> {
    let path = Path::new(filename);
    let mut components = path.components();
    let Some(Component::Normal(name)) = components.next() else {
        bail!("stored fixture asset filename must be one normal path component");
    };
    ensure!(
        components.next().is_none(),
        "stored fixture asset filename must not contain directories"
    );
    Ok(PathBuf::from(name))
}

fn validate_media_type(media_type: &str, path: &Path) -> Result<()> {
    let extension = path.extension().and_then(|value| value.to_str());
    let accepted = matches!(
        (media_type, extension),
        ("image/svg+xml", Some("svg"))
            | ("image/png", Some("png"))
            | ("image/jpeg", Some("jpg" | "jpeg"))
            | ("image/webp", Some("webp"))
    );
    ensure!(
        accepted,
        "stored fixture asset media type and extension disagree"
    );
    Ok(())
}

fn read_bounded(path: &Path, maximum: u64) -> Result<Vec<u8>> {
    let metadata = fs::metadata(path).with_context(|| format!("reading {}", path.display()))?;
    ensure!(
        metadata.is_file(),
        "{} is not a regular file",
        path.display()
    );
    ensure!(
        metadata.len() <= maximum,
        "{} exceeds the {}-byte fixture limit",
        path.display(),
        maximum
    );
    fs::read(path).with_context(|| format!("reading {}", path.display()))
}

fn sha256(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    let mut encoded = String::with_capacity(64);
    for byte in digest {
        use std::fmt::Write as _;
        let _ = write!(encoded, "{byte:02x}");
    }
    encoded
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture_root() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/published_question")
    }

    #[test]
    fn stored_fixture_set_reproduces_through_the_ple_question_backend() {
        let report = run(&fixture_root()).expect("stored fixture fixture_set should validate");
        assert_eq!(report.action, "validated");
        assert!(report.tracked_files > 1);
    }

    #[test]
    fn asset_filename_accepts_one_stored_name() {
        assert_eq!(
            validate_asset_filename("question-image.svg").expect("stored filename"),
            PathBuf::from("question-image.svg")
        );
    }

    #[test]
    fn asset_filename_rejects_path_components() {
        assert!(validate_asset_filename("../question-image.svg").is_err());
        assert!(validate_asset_filename("nested/question-image.svg").is_err());
        assert!(validate_asset_filename("/question-image.svg").is_err());
    }
}

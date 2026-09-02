//! Validation for the reviewed Chapter 1 Pilot Question Set.

use std::collections::HashSet;
use std::fmt::Write as _;
use std::path::{Component, Path, PathBuf};

use adapter_ple::question_json::PleQuestionJsonDocument;
use anyhow::{Context, Result, bail};
use question_model::response::{
    QuestionType, ResponseItemReference, StudentMatch, StudentResponse,
};
use question_model::{
    QuestionBackendLocator, QuestionFormat, QuestionId, QuestionRevision, QuestionRevisionNumber,
    WorkspaceId, classification::QuestionLicense,
};
use serde::Deserialize;
use serde_json::Value;
use sha2::{Digest, Sha256};
use uuid::Uuid;

const DEFAULT_MANIFEST: &str = "content/pilot/chapter_1_assignments.yaml";
const EXPECTED_QUESTION_SHAPES: [(Backend, PilotQuestionType); 4] = [
    (Backend::Webwork, PilotQuestionType::MultipleChoice),
    (Backend::Webwork, PilotQuestionType::Matching),
    (Backend::PleQuestionJson, PilotQuestionType::MultipleChoice),
    (Backend::PleQuestionJson, PilotQuestionType::Matching),
];

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct PilotManifest {
    version: u32,
    source_project: SourceProject,
    pub(crate) chapters: Vec<Chapter>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct SourceProject {
    repository: String,
    revision: String,
    author: String,
    affiliation: String,
    content_license: String,
    pgml_code_license: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Chapter {
    pub(crate) slug: String,
    course: String,
    pub(crate) course_title: String,
    chapter: u32,
    pub(crate) assignment_title: String,
    pub(crate) questions: Vec<Question>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Question {
    pub(crate) slug: String,
    pub(crate) title: String,
    pub(crate) backend: Backend,
    pub(crate) question_type: PilotQuestionType,
    pub(crate) points: u32,
    pub(crate) source: PathBuf,
    source_sha256: String,
    #[serde(default)]
    upstream_sha256: Option<String>,
    #[serde(default)]
    source_item: Option<String>,
    #[serde(default)]
    pub(crate) payload: Option<PathBuf>,
    #[serde(default)]
    payload_sha256: Option<String>,
    #[serde(default)]
    changes: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) enum Backend {
    Webwork,
    PleQuestionJson,
}

/// The Question Types present in the deliberately small Pilot Question Set.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) enum PilotQuestionType {
    MultipleChoice,
    Matching,
}

struct ValidationReport {
    chapters: Vec<String>,
    question_count: usize,
    adapted_source_count: usize,
}

pub(super) fn run(args: &[String]) -> Result<()> {
    let manifest = match args {
        [] => Path::new(DEFAULT_MANIFEST),
        [path] => Path::new(path),
        _ => bail!("usage: cargo tools pilot-content [manifest.yaml]"),
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

fn validate(manifest_path: &Path) -> Result<ValidationReport> {
    let manifest = read_manifest(manifest_path)?;
    let root = manifest_path
        .parent()
        .ok_or_else(|| anyhow::anyhow!("pilot manifest must have a parent directory"))?;
    validate_loaded_manifest(&manifest, root)
}

fn read_manifest(manifest_path: &Path) -> Result<PilotManifest> {
    let manifest_bytes = std::fs::read(manifest_path)
        .with_context(|| format!("reading pilot manifest {}", manifest_path.display()))?;
    serde_yaml_ng::from_slice(&manifest_bytes)
        .with_context(|| format!("decoding pilot manifest {}", manifest_path.display()))
}

fn validate_loaded_manifest(manifest: &PilotManifest, root: &Path) -> Result<ValidationReport> {
    validate_manifest_contract(manifest)?;

    let mut question_slugs = HashSet::new();
    let mut adapted_source_count = 0;
    for (chapter_index, chapter) in manifest.chapters.iter().enumerate() {
        validate_chapter(chapter)?;
        for (question_index, question) in chapter.questions.iter().enumerate() {
            if !question_slugs.insert(question.slug.as_str()) {
                bail!("pilot question slug is repeated: {}", question.slug);
            }
            validate_question(
                root,
                chapter,
                question,
                u128::try_from(chapter_index * 4 + question_index + 1)
                    .expect("the eight-item pilot index fits u128"),
            )?;
            adapted_source_count += usize::from(!question.changes.is_empty());
        }
    }
    Ok(ValidationReport {
        chapters: manifest
            .chapters
            .iter()
            .map(|chapter| chapter.assignment_title.clone())
            .collect(),
        question_count: manifest
            .chapters
            .iter()
            .map(|chapter| chapter.questions.len())
            .sum(),
        adapted_source_count,
    })
}

fn validate_manifest_contract(manifest: &PilotManifest) -> Result<()> {
    if manifest.version != 1 {
        bail!("pilot manifest version must be 1");
    }
    if manifest.chapters.len() != 2 {
        bail!("pilot manifest must define Genetics and Biochemistry Chapter 1");
    }
    let source = &manifest.source_project;
    for (name, value) in [
        ("repository", source.repository.as_str()),
        ("author", source.author.as_str()),
        ("affiliation", source.affiliation.as_str()),
    ] {
        if value.trim().is_empty() {
            bail!("pilot source-project {name} must not be blank");
        }
    }
    if !is_lower_hex(&source.revision, 40) {
        bail!("pilot source-project revision must be a lowercase Git commit");
    }
    if source.content_license != "CC-BY-4.0" || source.pgml_code_license != "LGPL-3.0-or-later" {
        bail!("pilot source-project licenses differ from the reviewed upstream declaration");
    }
    let courses = manifest
        .chapters
        .iter()
        .map(|chapter| chapter.course.as_str())
        .collect::<HashSet<_>>();
    if courses != HashSet::from(["Genetics", "Biochemistry"]) {
        bail!("pilot manifest must define Genetics and Biochemistry");
    }
    Ok(())
}

fn validate_chapter(chapter: &Chapter) -> Result<()> {
    if chapter.slug.trim().is_empty()
        || chapter.course_title.trim().is_empty()
        || chapter.assignment_title.trim().is_empty()
    {
        bail!("pilot chapter slug, course title, and assignment title must not be blank");
    }
    if chapter.chapter != 1 {
        bail!("pilot assignments must be Chapter 1");
    }
    if chapter.questions.len() != 4 {
        bail!(
            "{} must contain exactly four questions",
            chapter.assignment_title
        );
    }
    let shapes = chapter
        .questions
        .iter()
        .map(|question| (question.backend, question.question_type))
        .collect::<HashSet<_>>();
    if shapes != HashSet::from(EXPECTED_QUESTION_SHAPES) {
        bail!(
            "{} must contain one WeBWorK MC, WeBWorK MATCH, PLE Question JSON MC, and PLE Question JSON MATCH",
            chapter.assignment_title
        );
    }
    Ok(())
}

fn validate_question(
    root: &Path,
    chapter: &Chapter,
    question: &Question,
    identity: u128,
) -> Result<()> {
    if question.title.trim().is_empty() || question.slug.trim().is_empty() {
        bail!("pilot question title and slug must not be blank");
    }
    let expected_points = match question.question_type {
        PilotQuestionType::MultipleChoice => 1,
        PilotQuestionType::Matching => 4,
    };
    if question.points != expected_points {
        bail!("{} must be worth {expected_points} point(s)", question.slug);
    }
    let source = corpus_file(root, &question.source)?;
    validate_digest(&source, &question.source_sha256)?;
    match question.backend {
        Backend::Webwork => validate_webwork(question, &source),
        Backend::PleQuestionJson => validate_flat(root, chapter, question, &source, identity),
    }
}

fn validate_webwork(question: &Question, source: &Path) -> Result<()> {
    if question.source_item.is_some()
        || question.payload.is_some()
        || question.payload_sha256.is_some()
    {
        bail!("WeBWorK pilot entries must use their PGML source directly");
    }
    let upstream = question
        .upstream_sha256
        .as_deref()
        .ok_or_else(|| anyhow::anyhow!("WeBWorK pilot entry lacks its upstream checksum"))?;
    if !is_lower_hex(upstream, 64) {
        bail!("WeBWorK upstream checksum is malformed");
    }
    if upstream != question.source_sha256 && question.changes.is_empty() {
        bail!("adapted WeBWorK source must list its instructor-facing changes");
    }
    let text = std::fs::read_to_string(source)
        .with_context(|| format!("reading WeBWorK source {}", source.display()))?;
    for marker in [
        "## DESCRIPTION",
        "DOCUMENT();",
        "BEGIN_PGML",
        "ENDDOCUMENT();",
    ] {
        if !text.contains(marker) {
            bail!("WeBWorK source {} lacks {marker}", source.display());
        }
    }
    if text.contains("\\'") {
        bail!(
            "WeBWorK source {} contains an apostrophe escape rejected by the shipped PG translator; use a q{{}} literal",
            source.display()
        );
    }
    let question_type_marker = match question.question_type {
        PilotQuestionType::MultipleChoice => "RadioButtons",
        PilotQuestionType::Matching => "make_popup",
    };
    if !text.contains(question_type_marker) {
        bail!(
            "WeBWorK source {} lacks {question_type_marker}",
            source.display()
        );
    }
    Ok(())
}

fn validate_flat(
    root: &Path,
    chapter: &Chapter,
    question: &Question,
    source: &Path,
    identity: u128,
) -> Result<()> {
    if question.upstream_sha256.is_some() || !question.changes.is_empty() {
        bail!(
            "PLE Question JSON entries record adaptations in their reviewed payload, not as PGML changes"
        );
    }
    let source_item = question
        .source_item
        .as_deref()
        .ok_or_else(|| anyhow::anyhow!("PLE Question JSON entry lacks its source item"))?;
    validate_source_item(source, source_item, question.question_type)?;
    let payload_relative = question
        .payload
        .as_deref()
        .ok_or_else(|| anyhow::anyhow!("PLE Question JSON entry lacks its payload"))?;
    let payload_sha256 = question
        .payload_sha256
        .as_deref()
        .ok_or_else(|| anyhow::anyhow!("PLE Question JSON entry lacks its payload checksum"))?;
    let payload = corpus_file(root, payload_relative)?;
    validate_digest(&payload, payload_sha256)?;
    let bytes = std::fs::read(&payload)
        .with_context(|| format!("reading PLE Question JSON payload {}", payload.display()))?;
    let document = PleQuestionJsonDocument::parse(&bytes)
        .with_context(|| format!("validating PLE Question JSON payload {}", payload.display()))?;
    let compiled = document
        .compile(WorkspaceId::from_uuid(Uuid::from_u128(identity)))
        .with_context(|| format!("compiling PLE Question JSON payload {}", payload.display()))?;
    compiled.private().validate_for_draft(compiled.draft())?;
    if compiled.draft().metadata.title != question.title {
        bail!("PLE Question JSON pilot payload title differs from its manifest entry");
    }
    if compiled.draft().metadata.question_license != Some(QuestionLicense::CcBy4_0) {
        bail!("PLE Question JSON pilot payload must retain the CC BY license");
    }
    if !compiled
        .draft()
        .metadata
        .classifications
        .iter()
        .any(|classification| classification.name == format!("{} Chapter 1", chapter.course))
    {
        bail!("PLE Question JSON pilot payload classification does not match its chapter");
    }
    let expected_question_type = match question.question_type {
        PilotQuestionType::MultipleChoice => QuestionType::MultipleChoice,
        PilotQuestionType::Matching => QuestionType::Matching,
    };
    if !matches!(
        compiled.draft().backend_locator,
        question_model::DraftQuestionBackendLocator::Ple
    ) {
        bail!("PLE Question JSON payload compiled to a non-PLE Question Source");
    }
    if compiled.draft().question_format != QuestionFormat::PleQuestionJson
        || compiled.draft().question_type != expected_question_type
    {
        bail!("PLE Question JSON payload question_type differs from its manifest entry");
    }
    validate_answer_separation(compiled.draft())?;
    validate_correct_and_wrong_grading(compiled, &bytes, question.question_type)
}

fn validate_answer_separation(draft: &question_model::DraftQuestionContent) -> Result<()> {
    let public = serde_json::to_string(draft)?;
    for private_key in [
        "\"correctChoice\":",
        "\"correctChoices\":",
        "\"matches\":",
        "\"correctOrder\":",
        "\"correctRegions\":",
        "\"answers\":",
    ] {
        if public.contains(private_key) {
            bail!("compiled PLE Question JSON student definition exposes {private_key}");
        }
    }
    Ok(())
}

fn validate_correct_and_wrong_grading(
    compiled: adapter_ple::question_json::CompiledPleQuestionJson,
    source: &[u8],
    question_type: PilotQuestionType,
) -> Result<()> {
    let value: Value = serde_json::from_slice(source)?;
    let response = value
        .get("response")
        .and_then(Value::as_object)
        .ok_or_else(|| anyhow::anyhow!("PLE Question JSON payload lacks its response object"))?;
    let (correct, wrong) = source_responses(response, question_type)?;
    let (draft, private, _question_hint) = compiled.into_parts();
    if !matches!(
        draft.backend_locator,
        question_model::DraftQuestionBackendLocator::Ple
    ) {
        bail!("PLE Question JSON payload compiled to a non-PLE Question Source");
    }
    let published = QuestionRevision::from_draft(
        draft,
        QuestionId::from_canonical_parts("ABCDEF", 'G').expect("Question ID"),
        QuestionRevisionNumber::new(1).expect("positive version"),
        QuestionBackendLocator::Ple,
    );
    let correct_result = private.evaluate(&published, &correct)?;
    let wrong_result = private.evaluate(&published, &wrong)?;
    let grading::QuestionGradingOutcome::Graded(correct_result) = correct_result.outcome else {
        bail!("PLE Question JSON correct response did not produce a grade");
    };
    let grading::QuestionGradingOutcome::Graded(wrong_result) = wrong_result.outcome else {
        bail!("PLE Question JSON wrong response did not produce a grade");
    };
    if !correct_result.correct || wrong_result.correct {
        bail!("PLE Question JSON pilot payload did not distinguish correct and wrong responses");
    }
    Ok(())
}

fn source_responses(
    response: &serde_json::Map<String, Value>,
    question_type: PilotQuestionType,
) -> Result<(StudentResponse, StudentResponse)> {
    match question_type {
        PilotQuestionType::MultipleChoice => {
            let correct = string_field(response, "correctChoice")?;
            let wrong = response
                .get("choices")
                .and_then(Value::as_array)
                .and_then(|choices| {
                    choices.iter().find_map(|choice| {
                        let id = choice.get("id")?.as_str()?;
                        (id != correct).then_some(id)
                    })
                })
                .ok_or_else(|| {
                    anyhow::anyhow!("PLE Question JSON MC payload lacks a wrong choice")
                })?;
            Ok((
                StudentResponse::MultipleChoice {
                    selected: vec![ResponseItemReference::new(correct)],
                },
                StudentResponse::MultipleChoice {
                    selected: vec![ResponseItemReference::new(wrong)],
                },
            ))
        }
        PilotQuestionType::Matching => {
            let matches = response
                .get("matches")
                .and_then(Value::as_array)
                .ok_or_else(|| anyhow::anyhow!("PLE Question JSON MATCH payload lacks matches"))?;
            let correct = matches
                .iter()
                .map(student_match)
                .collect::<Result<Vec<_>>>()?;
            let mut wrong = correct.clone();
            if wrong.len() < 2 {
                bail!("PLE Question JSON MATCH payload needs at least two matches");
            }
            let first = wrong[0].choice.clone();
            wrong[0].choice = wrong[1].choice.clone();
            wrong[1].choice = first;
            Ok((
                StudentResponse::Matching { matches: correct },
                StudentResponse::Matching { matches: wrong },
            ))
        }
    }
}

fn student_match(value: &Value) -> Result<StudentMatch> {
    let record = value
        .as_object()
        .ok_or_else(|| anyhow::anyhow!("PLE Question JSON match is not an object"))?;
    Ok(StudentMatch {
        prompt: ResponseItemReference::new(string_field(record, "prompt")?),
        choice: ResponseItemReference::new(string_field(record, "choice")?),
    })
}

fn string_field<'a>(record: &'a serde_json::Map<String, Value>, name: &str) -> Result<&'a str> {
    record
        .get(name)
        .and_then(Value::as_str)
        .ok_or_else(|| anyhow::anyhow!("PLE Question JSON response lacks string field {name}"))
}

fn validate_source_item(source: &Path, item: &str, question_type: PilotQuestionType) -> Result<()> {
    let expected_kind = match question_type {
        PilotQuestionType::MultipleChoice => "MC",
        PilotQuestionType::Matching => "MAT",
    };
    let marker = format!("<p>{item}</p>");
    let mut reader = csv::ReaderBuilder::new()
        .delimiter(b'\t')
        .has_headers(false)
        .flexible(true)
        .quoting(false)
        .from_path(source)
        .with_context(|| format!("opening BBQ source {}", source.display()))?;
    let mut selected = None;
    for record in reader.records() {
        let record = record.with_context(|| format!("parsing BBQ source {}", source.display()))?;
        if record.get(1).is_some_and(|stem| stem.contains(&marker))
            && selected.replace(record).is_some()
        {
            bail!("BBQ source item {item} is repeated");
        }
    }
    let selected = selected.ok_or_else(|| anyhow::anyhow!("BBQ source item {item} is missing"))?;
    if selected.get(0) != Some(expected_kind) {
        bail!("BBQ source item {item} has the wrong question_type");
    }
    if selected.len() < 6 || (selected.len() - 2) % 2 != 0 {
        bail!("BBQ source item {item} has malformed paired fields");
    }
    if question_type == PilotQuestionType::MultipleChoice {
        let correct = selected
            .iter()
            .skip(3)
            .step_by(2)
            .filter(|status| status.eq_ignore_ascii_case("correct"))
            .count();
        if correct != 1 {
            bail!("BBQ MC source item {item} must contain exactly one correct choice");
        }
    }
    Ok(())
}

fn corpus_file(root: &Path, relative: &Path) -> Result<PathBuf> {
    if relative.is_absolute()
        || relative
            .components()
            .any(|component| !matches!(component, Component::Normal(_)))
    {
        bail!("Pilot Question Set paths must be simple relative paths");
    }
    let path = root.join(relative);
    let metadata = std::fs::symlink_metadata(&path)
        .with_context(|| format!("reading Pilot Question Set metadata {}", path.display()))?;
    if !metadata.is_file() || metadata.file_type().is_symlink() {
        bail!(
            "Pilot Question Set path is not a regular file: {}",
            path.display()
        );
    }
    Ok(path)
}

fn validate_digest(path: &Path, expected: &str) -> Result<()> {
    if !is_lower_hex(expected, 64) {
        bail!("pilot checksum is not lowercase SHA-256: {expected}");
    }
    let bytes = std::fs::read(path)
        .with_context(|| format!("reading Pilot Question Set file {}", path.display()))?;
    let mut actual = String::with_capacity(64);
    for byte in Sha256::digest(bytes) {
        write!(&mut actual, "{byte:02x}").expect("writing to a String cannot fail");
    }
    if actual != expected {
        bail!("Pilot Question Set checksum changed for {}", path.display());
    }
    Ok(())
}

fn is_lower_hex(value: &str, length: usize) -> bool {
    value.len() == length
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tracked_chapter_one_corpus_has_the_required_human_guidance_shape() {
        let manifest = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../content/pilot/chapter_1_assignments.yaml");
        let report = validate(&manifest).expect("tracked pilot content should validate");
        assert_eq!(report.chapters.len(), 2);
        assert_eq!(report.question_count, 8);
    }

    #[test]
    fn digest_validation_requires_lowercase_sha256() {
        assert!(is_lower_hex(&"a".repeat(64), 64));
        assert!(!is_lower_hex(&"A".repeat(64), 64));
        assert!(!is_lower_hex(&"a".repeat(63), 64));
    }
}

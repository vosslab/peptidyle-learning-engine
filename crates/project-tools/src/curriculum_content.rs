//! Trusted publication of the fresh canonical Genetics catalog.
//!
//! This tool is an import boundary, not a product Question parser. Each listed
//! source is an already-accepted opaque WeBWorK PGML document that publishes
//! as one ordinary Question and one direct Fixed Blueprint entry.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Component, Path, PathBuf};

use anyhow::{Context, Result, bail, ensure};
use question_model::{
    AssessmentType, MAX_ASSESSMENT_ORDERED_ENTRIES, QuestionFormat, QuestionLicense,
};
use serde::Deserialize;
use sha2::{Digest, Sha256};

pub(crate) mod parameterized_publication;
pub(crate) mod publication;

const USAGE: &str = "usage: PLE_CURRICULUM_CONTENT_ROOT=<content-root> cargo tools curriculum-content <validate|publish> <manifest>\n       PLE_CURRICULUM_CONTENT_ROOT=<content-root> cargo tools curriculum-content publish-parameterized <manifest> <source-id>";
const CONTENT_ROOT_ENV: &str = "PLE_CURRICULUM_CONTENT_ROOT";
const MAX_WEBWORK_PG_SOURCE_BYTES: usize = 262_144;

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Manifest {
    version: u32,
    pub(crate) course: Course,
    pub(crate) parameterized_sources: Vec<ParameterizedSource>,
    pub(crate) topics: Vec<Topic>,
}

/// Pinned canonical algorithmic input for one ordinary Question publication.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ParameterizedSource {
    pub(crate) classification: Option<crate::pilot_content::AuthoredClassification>,
    pub(crate) source_id: String,
    pub(crate) topic_slug: String,
    pub(crate) question_title: String,
    pub(crate) question_description: String,
    pub(crate) question_type: CurriculumQuestionType,
    pub(crate) source_format: WebworkSourceFormat,
    pub(crate) pg_source: PathBuf,
    pub(crate) pg_sha256: String,
    pub(crate) webwork_pg_path: String,
    pub(crate) canonical_author_source_url: String,
    pub(crate) canonical_author_source_sha256: String,
    pub(crate) content_license: String,
    pub(crate) source_code_license: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Course {
    pub(crate) assessment_type: AssessmentType,
    pub(crate) short_name: String,
    pub(crate) long_name: String,
    pub(crate) module_label: String,
    pub(crate) source_repository: String,
    pub(crate) source_revision: String,
    pub(crate) author: String,
    pub(crate) content_license: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Topic {
    pub(crate) slug: String,
    pub(crate) title: String,
    pub(crate) instructions: String,
    pub(crate) source_ids: Vec<String>,
}

#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) enum CurriculumQuestionType {
    MultipleChoice,
    MultipleAnswer,
    FillInBlank,
    MultipleFillInBlank,
    Numeric,
    Matching,
}

#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "lowercase")]
pub(crate) enum WebworkSourceFormat {
    Pg,
    Pgml,
}

pub(crate) fn run(args: &[String]) -> Result<()> {
    if matches!(args, [flag] if flag == "--help" || flag == "-h") {
        println!("{USAGE}");
        println!("The manifest and every source path are relative to the explicit content root.");
        return Ok(());
    }
    match args {
        [action, manifest] if action == "validate" => {
            let manifest = load(Path::new(manifest))?;
            println!(
                "curriculum content: {} topic(s), {} canonical source(s)",
                manifest.topics.len(),
                manifest.parameterized_sources.len()
            );
            Ok(())
        }
        [action, manifest] if action == "publish" => {
            publication::publish(load(Path::new(manifest))?)
        }
        [action, manifest, source_id] if action == "publish-parameterized" => {
            let manifest = load_selected_parameterized(Path::new(manifest), source_id)?;
            parameterized_publication::publish_selected(manifest)
        }
        _ => bail!("{USAGE}"),
    }
}

pub(crate) fn load(path: &Path) -> Result<Manifest> {
    let repository = repository_root()?;
    load_from_root(&repository, path)
}

pub(crate) fn load_from_root(root: &Path, path: &Path) -> Result<Manifest> {
    let repository = root
        .canonicalize()
        .with_context(|| format!("canonicalizing curriculum content root {}", root.display()))?;
    let path = contained_file(&repository, path)?;
    let bytes = std::fs::read(&path)
        .with_context(|| format!("reading curriculum manifest {}", path.display()))?;
    let manifest: Manifest =
        serde_yaml_ng::from_slice(&bytes).context("decoding curriculum manifest")?;
    validate(&manifest, &repository)?;
    Ok(manifest)
}

fn load_selected_parameterized(path: &Path, source_id: &str) -> Result<Manifest> {
    let repository = repository_root()?;
    let repository = repository.canonicalize().with_context(|| {
        format!(
            "canonicalizing curriculum content root {}",
            repository.display()
        )
    })?;
    let path = contained_file(&repository, path)?;
    let bytes = std::fs::read(&path)
        .with_context(|| format!("reading curriculum manifest {}", path.display()))?;
    let mut manifest: Manifest =
        serde_yaml_ng::from_slice(&bytes).context("decoding curriculum manifest")?;
    let selected_count = manifest
        .parameterized_sources
        .iter()
        .filter(|source| source.source_id == source_id)
        .count();
    ensure!(
        selected_count == 1,
        "canonical curriculum source ID must identify exactly one source: {source_id}"
    );
    manifest
        .parameterized_sources
        .retain(|source| source.source_id == source_id);
    validate_selected_parameterized_manifest(&manifest, &repository)?;
    Ok(manifest)
}

pub(crate) fn repository_root() -> Result<PathBuf> {
    let configured = std::env::var_os(CONTENT_ROOT_ENV)
        .map(PathBuf::from)
        .with_context(|| format!("{CONTENT_ROOT_ENV} must name the curriculum content root"))?;
    configured
        .canonicalize()
        .with_context(|| format!("canonicalizing {CONTENT_ROOT_ENV} for curriculum content"))
}

fn validate(manifest: &Manifest, root: &Path) -> Result<()> {
    validate_manifest_header(manifest)?;
    ensure!(
        !manifest.parameterized_sources.is_empty(),
        "canonical Genetics manifest must contain accepted sources"
    );
    ensure!(
        !manifest.topics.is_empty(),
        "canonical Genetics manifest must contain topics"
    );
    ensure!(
        manifest.topics.len() <= MAX_ASSESSMENT_ORDERED_ENTRIES,
        "canonical Genetics Blueprint has too many topics"
    );

    let sources = validate_source_set(&manifest.parameterized_sources, root)?;
    ensure!(
        manifest
            .parameterized_sources
            .iter()
            .all(|source| source.content_license == manifest.course.content_license),
        "canonical Genetics sources must use the course content license"
    );
    let mut topic_slugs = BTreeSet::new();
    let mut referenced_sources = BTreeSet::new();
    for topic in &manifest.topics {
        validate_topic_metadata(topic)?;
        ensure!(
            topic_slugs.insert(topic.slug.as_str()),
            "canonical Genetics topic slugs must be unique"
        );
        ensure!(
            !topic.source_ids.is_empty(),
            "canonical Genetics topics must contain direct Question sources"
        );
        ensure!(
            topic.source_ids.len() <= MAX_ASSESSMENT_ORDERED_ENTRIES,
            "canonical Genetics topic has too many direct Question entries"
        );
        for source_id in &topic.source_ids {
            let source = sources.get(source_id.as_str()).with_context(|| {
                format!("canonical Genetics topic references unknown source {source_id}")
            })?;
            ensure!(
                source.topic_slug == topic.slug,
                "canonical Genetics source topic differs for {source_id}"
            );
            ensure!(
                referenced_sources.insert(source_id.as_str()),
                "canonical Genetics source is referenced more than once: {source_id}"
            );
        }
    }
    ensure!(
        referenced_sources.len() == sources.len(),
        "every canonical Genetics source must have exactly one direct topic entry"
    );
    Ok(())
}

fn validate_manifest_header(manifest: &Manifest) -> Result<()> {
    ensure!(
        manifest.version == 1,
        "curriculum manifest version must be 1"
    );
    for value in [
        &manifest.course.short_name,
        &manifest.course.long_name,
        &manifest.course.module_label,
        &manifest.course.source_repository,
        &manifest.course.source_revision,
        &manifest.course.author,
        &manifest.course.content_license,
    ] {
        ensure!(
            !value.trim().is_empty() && value == value.trim(),
            "curriculum course text must be trimmed and nonempty"
        );
    }
    question_model::validate_blueprint_course_title(&manifest.course.short_name)
        .map_err(|_| anyhow::anyhow!("curriculum Blueprint short name is invalid"))?;
    question_model::validate_blueprint_course_title(&manifest.course.long_name)
        .map_err(|_| anyhow::anyhow!("curriculum Blueprint long name is invalid"))?;
    question_model::validate_blueprint_course_title(&manifest.course.module_label)
        .map_err(|_| anyhow::anyhow!("curriculum Blueprint module label is invalid"))?;
    serde_json::from_value::<QuestionLicense>(serde_json::Value::String(
        manifest.course.content_license.clone(),
    ))
    .map_err(|_| anyhow::anyhow!("curriculum content license is not publishable"))?;
    Ok(())
}

fn validate_topic_metadata(topic: &Topic) -> Result<()> {
    for value in [&topic.slug, &topic.title] {
        ensure!(
            !value.trim().is_empty() && value == value.trim(),
            "canonical Genetics topic metadata must be trimmed and nonempty"
        );
    }
    ensure!(
        !topic.instructions.trim().is_empty(),
        "canonical Genetics topic instructions are invalid"
    );
    question_model::AssessmentTitle::try_new(topic.title.clone())
        .map_err(|_| anyhow::anyhow!("curriculum Assessment title is invalid"))?;
    question_model::AssessmentInstructions::try_new(topic.instructions.clone())
        .map_err(|_| anyhow::anyhow!("curriculum Assessment instructions are invalid"))?;
    Ok(())
}

pub(crate) fn validate_selected_parameterized_manifest(
    manifest: &Manifest,
    root: &Path,
) -> Result<()> {
    validate_manifest_header(manifest)?;
    ensure!(
        manifest.parameterized_sources.len() == 1,
        "selected canonical publication requires exactly one source"
    );
    let source = &manifest.parameterized_sources[0];
    ensure!(
        source.content_license == manifest.course.content_license,
        "selected canonical source license differs from its publication license"
    );
    let matching_topics = manifest
        .topics
        .iter()
        .filter(|topic| topic.slug == source.topic_slug)
        .collect::<Vec<_>>();
    ensure!(
        matching_topics.len() == 1,
        "selected canonical source must identify exactly one manifest Topic"
    );
    let topic = matching_topics[0];
    validate_topic_metadata(topic)?;
    ensure!(
        topic
            .source_ids
            .iter()
            .filter(|source_id| **source_id == source.source_id)
            .count()
            == 1,
        "selected canonical source must have exactly one direct topic entry"
    );
    validate_parameterized_source_metadata(source, root)
}

fn validate_source_set<'a>(
    sources: &'a [ParameterizedSource],
    root: &Path,
) -> Result<BTreeMap<&'a str, &'a ParameterizedSource>> {
    let mut by_id = BTreeMap::new();
    let mut local_paths = BTreeSet::new();
    let mut webwork_paths = BTreeSet::new();
    let mut checksums = BTreeSet::new();
    let mut draft_identities = BTreeSet::new();
    for source in sources {
        validate_parameterized_source_metadata(source, root)?;
        ensure!(
            source.content_license == "CC-BY-4.0",
            "canonical Genetics source content license is unsupported"
        );
        ensure!(
            by_id.insert(source.source_id.as_str(), source).is_none(),
            "canonical Genetics source IDs must be unique"
        );
        ensure!(
            local_paths.insert(&source.pg_source),
            "canonical Genetics local source paths must be unique"
        );
        ensure!(
            webwork_paths.insert(&source.webwork_pg_path),
            "canonical Genetics WeBWorK paths must be unique"
        );
        ensure!(
            checksums.insert(&source.pg_sha256),
            "canonical Genetics source checksums must be unique"
        );
        ensure!(
            draft_identities.insert((&source.question_title, &source.question_description)),
            "canonical Genetics Question title and description pairs must be unique"
        );
    }
    Ok(by_id)
}

fn validate_parameterized_source_metadata(source: &ParameterizedSource, root: &Path) -> Result<()> {
    source
        .classification
        .as_ref()
        .with_context(|| {
            format!(
                "{} requires explicit authored classification before publication",
                source.source_id
            )
        })?
        .validate()
        .with_context(|| format!("classification for {}", source.source_id))?;
    for value in [
        &source.source_id,
        &source.topic_slug,
        &source.question_title,
        &source.question_description,
        &source.webwork_pg_path,
        &source.canonical_author_source_url,
        &source.canonical_author_source_sha256,
        &source.content_license,
        &source.source_code_license,
    ] {
        ensure!(
            !value.trim().is_empty() && value == value.trim(),
            "canonical Genetics source metadata must be trimmed and nonempty"
        );
    }
    question_model::validate_question_title(&source.question_title)
        .map_err(|_| anyhow::anyhow!("canonical Genetics Question title is invalid"))?;
    question_model::validate_question_description(&source.question_description)
        .map_err(|_| anyhow::anyhow!("canonical Genetics Question description is invalid"))?;
    ensure!(
        parameterized_source_paths_match_topic(
            &source.topic_slug,
            &source.pg_source,
            &source.webwork_pg_path,
        ),
        "canonical Genetics source must be bundled under its topic path"
    );
    ensure!(
        source_format_paths_match(
            source.source_format,
            &source.pg_source,
            &source.webwork_pg_path
        ),
        "canonical Genetics source format must match both declared paths"
    );
    ensure!(
        source.content_license == "CC-BY-4.0" && source.source_code_license == "LGPL-3.0-or-later",
        "canonical Genetics license pin is unsupported"
    );
    ensure!(
        valid_pinned_vosslab_github_blob_url(&source.canonical_author_source_url),
        "canonical Genetics author-source URL is not an immutable vosslab GitHub blob pin"
    );
    ensure!(
        is_lower_hex(&source.canonical_author_source_sha256),
        "canonical Genetics author-source checksum is invalid"
    );
    let bytes = read_parameterized_pg_source(root, source)?;
    ensure!(
        bytes.len() <= MAX_WEBWORK_PG_SOURCE_BYTES,
        "canonical Genetics source exceeds the supported WeBWorK bound"
    );
    std::str::from_utf8(&bytes).context("canonical Genetics source is not UTF-8")?;
    Ok(())
}

fn parameterized_source_paths_match_topic(
    topic_slug: &str,
    local_path: &Path,
    webwork_path: &str,
) -> bool {
    let local_topic = Path::new("pg").join(topic_slug);
    let webwork_topic = format!("genetics/{topic_slug}/");
    local_path.starts_with(local_topic)
        && local_path
            .components()
            .all(|component| matches!(component, Component::Normal(_)))
        && valid_pg_path(webwork_path)
        && webwork_path.starts_with(&webwork_topic)
}

fn source_format_paths_match(
    source_format: WebworkSourceFormat,
    local_path: &Path,
    webwork_path: &str,
) -> bool {
    let suffix = match source_format {
        WebworkSourceFormat::Pg => ".pg",
        WebworkSourceFormat::Pgml => ".pgml",
    };
    local_path.to_string_lossy().ends_with(suffix) && webwork_path.ends_with(suffix)
}

pub(crate) const fn webwork_question_format(source_format: WebworkSourceFormat) -> QuestionFormat {
    match source_format {
        WebworkSourceFormat::Pg => QuestionFormat::WebworkPg,
        WebworkSourceFormat::Pgml => QuestionFormat::WebworkPgml,
    }
}

fn valid_pinned_vosslab_github_blob_url(value: &str) -> bool {
    let Some(path) = value.strip_prefix("https://github.com/vosslab/") else {
        return false;
    };
    let Some((repository, remainder)) = path.split_once("/blob/") else {
        return false;
    };
    let Some((revision, source_path)) = remainder.split_once('/') else {
        return false;
    };
    !repository.is_empty()
        && !repository.contains('/')
        && is_lower_hex_40(revision)
        && valid_repository_path(source_path)
}

fn is_lower_hex_40(value: &str) -> bool {
    value.len() == 40
        && value.bytes().all(|byte| {
            byte.is_ascii_digit() || (byte.is_ascii_lowercase() && byte.is_ascii_hexdigit())
        })
}

fn valid_repository_path(value: &str) -> bool {
    !value.contains(['?', '#', '\\', '\0'])
        && value
            .split('/')
            .all(|part| !part.is_empty() && !matches!(part, "." | ".."))
}

pub(crate) fn read_parameterized_pg_source(
    root: &Path,
    source: &ParameterizedSource,
) -> Result<Vec<u8>> {
    ensure!(
        is_lower_hex(&source.pg_sha256),
        "canonical source checksum must be lowercase SHA-256"
    );
    let path = contained_file(root, &source.pg_source)?;
    let bytes = std::fs::read(&path)
        .with_context(|| format!("reading canonical PG source {}", path.display()))?;
    ensure!(
        sha256_hex(&bytes) == source.pg_sha256,
        "canonical PG source checksum changed for {}",
        source.source_id
    );
    Ok(bytes)
}

fn contained_file(root: &Path, relative: &Path) -> Result<PathBuf> {
    ensure!(
        !relative.is_absolute() && !relative.as_os_str().is_empty(),
        "curriculum paths must be nonempty repository-relative paths"
    );
    ensure!(
        !relative.components().any(|part| matches!(
            part,
            Component::ParentDir | Component::RootDir | Component::Prefix(_)
        )),
        "curriculum paths must not traverse outside the repository"
    );
    let path = root
        .join(relative)
        .canonicalize()
        .with_context(|| format!("resolving curriculum path {}", relative.display()))?;
    ensure!(
        path.is_file() && path.starts_with(root),
        "curriculum path escapes the repository or is not a file"
    );
    Ok(path)
}

pub(crate) fn sha256_hex(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn is_lower_hex(value: &str) -> bool {
    value.len() == 64
        && value.bytes().all(|byte| {
            byte.is_ascii_digit() || (byte.is_ascii_lowercase() && byte.is_ascii_hexdigit())
        })
}

fn valid_pg_path(value: &str) -> bool {
    !value.starts_with('/')
        && value.len() <= 1024
        && !value.contains(['\\', '\0'])
        && !value
            .split('/')
            .any(|part| part.is_empty() || matches!(part, "." | ".."))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn checksum_shape_rejects_uppercase_and_wrong_lengths() {
        assert!(is_lower_hex(&"a".repeat(64)));
        assert!(!is_lower_hex(&"A".repeat(64)));
        assert!(!is_lower_hex(&"a".repeat(63)));
    }

    #[test]
    fn webwork_path_refuses_traversal_and_absolute_paths() {
        assert!(valid_pg_path("genetics/topic01/question.pgml"));
        assert!(!valid_pg_path("../question.pgml"));
        assert!(!valid_pg_path("/question.pgml"));
    }
}

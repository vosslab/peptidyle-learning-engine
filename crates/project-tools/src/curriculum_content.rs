//! Trusted, reproducible publication of retained curriculum content.
//!
//! This tool is deliberately an import boundary, not a product question parser:
//! every listed source is already a supported opaque WeBWorK PG document.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Component, Path, PathBuf};

use anyhow::{Context, Result, bail, ensure};
use question_model::{
    AssessmentType, MAX_ASSESSMENT_ORDERED_ENTRIES, MAX_ASSESSMENT_QUESTION_POOL_ITEMS,
    MAX_QUESTION_POOL_ITEMS_PER_ASSESSMENT_ENTRY, QuestionFormat, QuestionLicense,
};
use serde::Deserialize;
use sha2::{Digest, Sha256};

pub(crate) mod parameterized_publication;
pub(crate) mod publication;

const USAGE: &str = "usage: PLE_CURRICULUM_CONTENT_ROOT=<content-root> cargo tools curriculum-content <validate|publish|publish-parameterized> <manifest>";
const CONTENT_ROOT_ENV: &str = "PLE_CURRICULUM_CONTENT_ROOT";
const MAX_WEBWORK_PG_SOURCE_BYTES: usize = 262_144;

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Manifest {
    version: u32,
    course: Course,
    #[serde(default)]
    parameterized_sources: Vec<ParameterizedSource>,
    topics: Vec<Topic>,
}

/// Pinned canonical algorithmic input for one ordinary Question publication.
///
/// A source carries all Question metadata itself.  In particular, it does not
/// identify a static bank, Pool, or generated variant: one canonical PG/PGML
/// document creates one ordinary immutable Question lineage.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ParameterizedSource {
    pub(crate) source_id: String,
    pub(crate) topic_slug: String,
    pub(crate) question_title: String,
    pub(crate) question_description: String,
    pub(crate) question_type: CurriculumQuestionType,
    pub(crate) source_format: WebworkSourceFormat,
    /// C840-only catalog replacement target. It is absent for a source that
    /// has not passed per-family acceptance and never supplies source metadata.
    #[serde(default)]
    pub(crate) replaces_static_bank_slug: Option<String>,
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
    pub(crate) banks: Vec<Bank>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Bank {
    pub(crate) slug: String,
    pub(crate) title: String,
    pub(crate) source: PathBuf,
    pub(crate) source_sha256: String,
    #[serde(default = "one")]
    pub(crate) selection_count: u32,
    pub(crate) rows: Vec<Row>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Row {
    pub(crate) row_id: String,
    pub(crate) question_title: String,
    pub(crate) question_description: String,
    pub(crate) question_type: CurriculumQuestionType,
    pub(crate) pg_source: PathBuf,
    pub(crate) pg_sha256: String,
    pub(crate) webwork_pg_path: String,
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

/// Explicit canonical WeBWorK source representation.  It selects durable
/// revision metadata; it never changes the WeBWorK backend or parses source.
#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "lowercase")]
pub(crate) enum WebworkSourceFormat {
    Pg,
    Pgml,
}

fn one() -> u32 {
    1
}

pub(crate) fn run(args: &[String]) -> Result<()> {
    if matches!(args, [flag] if flag == "--help" || flag == "-h") {
        println!("{USAGE}");
        println!("The manifest and every source path are relative to the explicit content root.");
        return Ok(());
    }
    let [action, manifest] = args else {
        bail!("{USAGE}");
    };
    let manifest = load(Path::new(manifest))?;
    match action.as_str() {
        "validate" => {
            println!(
                "curriculum content: {} topic(s), {} bank(s), {} source row(s)",
                manifest.topics.len(),
                manifest
                    .topics
                    .iter()
                    .map(|topic| topic.banks.len())
                    .sum::<usize>(),
                manifest
                    .topics
                    .iter()
                    .flat_map(|topic| &topic.banks)
                    .map(|bank| bank.rows.len())
                    .sum::<usize>()
            );
            Ok(())
        }
        "publish" => publication::publish(manifest),
        "publish-parameterized" => parameterized_publication::publish(manifest),
        _ => bail!("{USAGE}"),
    }
}

/// Read and validate the entire trusted manifest before any publication write.
pub(crate) fn load(path: &Path) -> Result<Manifest> {
    let repository = repository_root()?;
    load_from_root(&repository, path)
}

/// Load trusted curriculum from the fixed content root selected by an installer.
///
/// The public command retains its explicit environment-selected root. Installers
/// use this separate entry point so they never mutate process-global environment
/// state merely to invoke the same validation and publication path.
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

fn repository_root() -> Result<PathBuf> {
    // The image is built elsewhere, so CARGO_MANIFEST_DIR cannot locate runtime
    // curriculum files. The caller must make the immutable content mount explicit.
    let configured = std::env::var_os(CONTENT_ROOT_ENV)
        .map(PathBuf::from)
        .with_context(|| format!("{CONTENT_ROOT_ENV} must name the curriculum content root"))?;
    configured
        .canonicalize()
        .with_context(|| format!("canonicalizing {CONTENT_ROOT_ENV} for curriculum content"))
}

fn validate(manifest: &Manifest, root: &Path) -> Result<()> {
    // ASVS 2.2.1: admit bounded, internally consistent source hierarchy before publication writes.
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
    let mut topic_slugs = BTreeSet::new();
    let mut bank_slugs = BTreeSet::new();
    let mut row_ids = BTreeSet::new();
    let mut pg_checksum_rows = BTreeMap::new();
    let mut pg_path_rows = BTreeMap::new();
    let accepted_replacements = manifest
        .parameterized_sources
        .iter()
        .filter_map(|source| {
            source
                .replaces_static_bank_slug
                .as_ref()
                .map(|bank| (&source.topic_slug, bank))
        })
        .collect::<BTreeSet<_>>();
    ensure!(
        manifest.topics.len() <= MAX_ASSESSMENT_ORDERED_ENTRIES,
        "curriculum Blueprint has too many ordered topic Assessments"
    );
    for topic in &manifest.topics {
        ensure!(
            topic_slugs.insert(&topic.slug),
            "curriculum topic slugs must be unique"
        );
        ensure!(
            !topic.title.trim().is_empty() && topic.title == topic.title.trim(),
            "curriculum topic title is invalid"
        );
        ensure!(
            !topic.instructions.trim().is_empty(),
            "curriculum topic instructions are invalid"
        );
        question_model::AssessmentTitle::try_new(topic.title.clone())
            .map_err(|_| anyhow::anyhow!("curriculum Assessment title is invalid"))?;
        question_model::AssessmentInstructions::try_new(topic.instructions.clone())
            .map_err(|_| anyhow::anyhow!("curriculum Assessment instructions are invalid"))?;
        ensure!(
            !topic.banks.is_empty(),
            "curriculum topics must contain source banks"
        );
        ensure!(
            topic.banks.len() <= MAX_ASSESSMENT_ORDERED_ENTRIES,
            "curriculum Assessment has too many ordered Pool entries"
        );
        let mut total_pool_items = 0_usize;
        for bank in &topic.banks {
            let has_accepted_replacement =
                accepted_replacements.contains(&(&topic.slug, &bank.slug));
            ensure!(
                bank_slugs.insert((&topic.slug, &bank.slug)),
                "curriculum bank slugs must be unique within a topic"
            );
            ensure!(
                !bank.title.trim().is_empty() && bank.title == bank.title.trim(),
                "curriculum bank title is invalid"
            );
            ensure!(
                !bank.rows.is_empty(),
                "curriculum banks must contain source rows"
            );
            ensure!(
                bank.rows.len() <= MAX_QUESTION_POOL_ITEMS_PER_ASSESSMENT_ENTRY,
                "curriculum Pool has too many items"
            );
            total_pool_items = total_pool_items
                .checked_add(bank.rows.len())
                .context("curriculum Assessment Pool item count overflowed")?;
            validate_pool_bounds(topic.banks.len(), bank.rows.len(), total_pool_items)?;
            ensure!(
                bank.selection_count > 0
                    && usize::try_from(bank.selection_count).ok() <= Some(bank.rows.len()),
                "curriculum bank selection count is invalid"
            );
            if !has_accepted_replacement {
                verify_file_checksum(root, &bank.source, &bank.source_sha256, "bank source")?;
            }
            let mut bank_row_ids = BTreeSet::new();
            for row in &bank.rows {
                ensure!(
                    row_ids.insert((&topic.slug, &bank.slug, &row.row_id)),
                    "curriculum row identities must be unique"
                );
                question_model::validate_question_title(&row.question_title)
                    .map_err(|_| anyhow::anyhow!("curriculum Question title is invalid"))?;
                question_model::validate_question_description(&row.question_description)
                    .map_err(|_| anyhow::anyhow!("curriculum Question description is invalid"))?;
                ensure!(
                    bank_row_ids.insert(&row.row_id),
                    "curriculum bank row identities must be unique"
                );
                for value in [
                    &row.row_id,
                    &row.question_title,
                    &row.question_description,
                    &row.webwork_pg_path,
                ] {
                    ensure!(
                        !value.trim().is_empty() && value == value.trim(),
                        "curriculum row metadata must be trimmed and nonempty"
                    );
                }
                ensure!(
                    valid_pg_path(&row.webwork_pg_path),
                    "curriculum WeBWorK PG path is invalid"
                );
                let key = format!("{}/{}/{}", topic.slug, bank.slug, row.row_id);
                if !has_accepted_replacement {
                    ensure!(
                        pg_checksum_rows
                            .insert(row.pg_sha256.as_str(), key.clone())
                            .is_none(),
                        "curriculum PG checksum maps to more than one source row: {}",
                        row.pg_sha256
                    );
                    ensure!(
                        pg_path_rows
                            .insert(row.webwork_pg_path.as_str(), key)
                            .is_none(),
                        "curriculum WeBWorK PG path maps to more than one source row: {}",
                        row.webwork_pg_path
                    );
                    let pg =
                        verify_file_checksum(root, &row.pg_source, &row.pg_sha256, "PG source")?;
                    ensure!(
                        pg <= MAX_WEBWORK_PG_SOURCE_BYTES,
                        "curriculum PG source exceeds the supported WeBWorK bound"
                    );
                }
            }
        }
    }
    ensure!(
        !manifest.topics.is_empty(),
        "curriculum manifest must contain topics"
    );
    validate_parameterized_sources(
        &manifest.parameterized_sources,
        root,
        &topic_slugs,
        &bank_slugs,
        &pg_path_rows,
    )?;
    Ok(())
}

fn validate_parameterized_sources(
    sources: &[ParameterizedSource],
    root: &Path,
    topic_slugs: &BTreeSet<&String>,
    bank_slugs: &BTreeSet<(&String, &String)>,
    static_paths: &BTreeMap<&str, String>,
) -> Result<()> {
    let mut source_ids = BTreeSet::new();
    let mut replacement_banks = BTreeSet::new();
    let mut source_paths = BTreeSet::new();
    let mut parameterized_paths = BTreeSet::new();
    for source in sources {
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
                "parameterized Genetics metadata must be trimmed and nonempty"
            );
        }
        ensure!(
            source_ids.insert(&source.source_id),
            "parameterized Genetics source IDs must be unique"
        );
        ensure!(
            source_paths.insert(&source.pg_source),
            "parameterized Genetics source paths must be unique"
        );
        ensure!(
            topic_slugs.contains(&&source.topic_slug),
            "parameterized Genetics source must identify an existing Topic"
        );
        if let Some(bank_slug) = &source.replaces_static_bank_slug {
            ensure!(
                !bank_slug.trim().is_empty() && bank_slug == bank_slug.trim(),
                "parameterized Genetics replacement bank slug is invalid"
            );
            ensure!(
                bank_slugs.contains(&(&source.topic_slug, bank_slug)),
                "parameterized Genetics replacement bank does not exist in its Topic"
            );
            ensure!(
                replacement_banks.insert((&source.topic_slug, bank_slug)),
                "parameterized Genetics replacement bank has more than one canonical source"
            );
        }
        question_model::validate_question_title(&source.question_title)
            .map_err(|_| anyhow::anyhow!("parameterized Genetics Question title is invalid"))?;
        question_model::validate_question_description(&source.question_description).map_err(
            |_| anyhow::anyhow!("parameterized Genetics Question description is invalid"),
        )?;
        ensure!(
            source.pg_source.starts_with("pg/topic")
                && valid_pg_path(&source.webwork_pg_path)
                && source.webwork_pg_path.starts_with("genetics/topic"),
            "canonical Genetics source must be locally bundled under its topic path"
        );
        ensure!(
            source_format_paths_match(
                source.source_format,
                &source.pg_source,
                &source.webwork_pg_path
            ),
            "parameterized Genetics source format must match both declared source paths"
        );
        ensure!(
            parameterized_paths.insert(&source.webwork_pg_path)
                && !static_paths.contains_key(source.webwork_pg_path.as_str()),
            "parameterized Genetics PG paths must be unique and distinct from static paths"
        );
        ensure!(
            source.content_license == "CC-BY-4.0"
                && source.source_code_license == "LGPL-3.0-or-later",
            "parameterized Genetics license pin is unsupported"
        );
        ensure!(
            valid_pinned_vosslab_github_blob_url(&source.canonical_author_source_url),
            "parameterized Genetics canonical author-source URL is not an immutable vosslab GitHub blob pin"
        );
        ensure!(
            is_lower_hex(&source.canonical_author_source_sha256),
            "parameterized Genetics canonical author-source checksum is invalid"
        );
        let bytes = std::fs::read(contained_file(root, &source.pg_source)?).with_context(|| {
            format!(
                "reading parameterized PG source {}",
                source.pg_source.display()
            )
        })?;
        ensure!(
            bytes.len() <= MAX_WEBWORK_PG_SOURCE_BYTES
                && sha256_hex(&bytes) == source.pg_sha256
                && is_lower_hex(&source.pg_sha256),
            "parameterized Genetics PG source pin is invalid"
        );
        std::str::from_utf8(&bytes).context("parameterized Genetics PG source is not UTF-8")?;
    }
    Ok(())
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

pub(crate) fn read_pg_source(root: &Path, row: &Row) -> Result<Vec<u8>> {
    let path = contained_file(root, &row.pg_source)?;
    let bytes =
        std::fs::read(&path).with_context(|| format!("reading PG source {}", path.display()))?;
    ensure!(
        sha256_hex(&bytes) == row.pg_sha256,
        "PG source checksum changed for {}",
        row.row_id
    );
    Ok(bytes)
}

/// Reads one C824-pinned parameterized source after the complete manifest has
/// already established its provenance, license, path, and SHA-256 pin.
pub(crate) fn read_parameterized_pg_source(
    root: &Path,
    source: &ParameterizedSource,
) -> Result<Vec<u8>> {
    let path = contained_file(root, &source.pg_source)?;
    let bytes = std::fs::read(&path).with_context(|| {
        format!(
            "reading parameterized PG source {}",
            source.pg_source.display()
        )
    })?;
    ensure!(
        sha256_hex(&bytes) == source.pg_sha256,
        "parameterized PG source checksum changed for {}",
        source.source_id
    );
    Ok(bytes)
}

fn verify_file_checksum(
    root: &Path,
    relative: &Path,
    expected: &str,
    label: &str,
) -> Result<usize> {
    ensure!(
        is_lower_hex(expected),
        "{label} checksum must be lowercase SHA-256"
    );
    let path = contained_file(root, relative)?;
    let bytes =
        std::fs::read(&path).with_context(|| format!("reading {label} {}", path.display()))?;
    ensure!(
        sha256_hex(&bytes) == expected,
        "{label} checksum differs for {}",
        relative.display()
    );
    Ok(bytes.len())
}

fn contained_file(root: &Path, relative: &Path) -> Result<PathBuf> {
    // ASVS 5.3.2: accept only validated relative paths contained by the explicit trusted root.
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

fn validate_pool_bounds(
    entry_count: usize,
    pool_items: usize,
    total_pool_items: usize,
) -> Result<()> {
    ensure!(
        entry_count <= MAX_ASSESSMENT_ORDERED_ENTRIES,
        "curriculum Assessment has too many ordered Pool entries"
    );
    ensure!(
        pool_items <= MAX_QUESTION_POOL_ITEMS_PER_ASSESSMENT_ENTRY,
        "curriculum Pool has too many items"
    );
    ensure!(
        total_pool_items <= MAX_ASSESSMENT_QUESTION_POOL_ITEMS,
        "curriculum Assessment has too many Pool items"
    );
    Ok(())
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
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temporary_root() -> PathBuf {
        let path = std::env::temp_dir().join(format!(
            "ple-curriculum-content-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("system clock after Unix epoch")
                .as_nanos()
        ));
        fs::create_dir(&path).expect("temporary root created");
        path
    }

    #[test]
    fn checksum_shape_rejects_uppercase_and_wrong_lengths() {
        assert!(is_lower_hex(&"a".repeat(64)));
        assert!(!is_lower_hex(&"A".repeat(64)));
        assert!(!is_lower_hex(&"a".repeat(63)));
    }

    #[test]
    fn webwork_path_refuses_traversal_and_absolute_paths() {
        assert!(valid_pg_path("genetics/topic01/question.pg"));
        assert!(!valid_pg_path("../question.pg"));
        assert!(!valid_pg_path("/question.pg"));
    }

    #[test]
    fn contained_files_stay_inside_a_temporary_root_and_match_checksums() {
        let root = temporary_root()
            .canonicalize()
            .expect("temporary root canonicalized");
        let source = root.join("source.pg");
        fs::write(&source, b"DOCUMENT();").expect("source written");
        let expected = sha256_hex(b"DOCUMENT();");

        assert_eq!(
            verify_file_checksum(&root, Path::new("source.pg"), &expected, "PG source")
                .expect("contained source with matching checksum"),
            b"DOCUMENT();".len()
        );
        assert!(contained_file(&root, Path::new("../outside.pg")).is_err());
        assert!(
            verify_file_checksum(&root, Path::new("source.pg"), &"0".repeat(64), "PG source")
                .is_err()
        );

        fs::remove_dir_all(root).expect("temporary root removed");
    }

    #[test]
    fn pool_bounds_are_rejected_before_publication() {
        assert!(validate_pool_bounds(1, 1, 1).is_ok());
        assert!(validate_pool_bounds(MAX_ASSESSMENT_ORDERED_ENTRIES + 1, 1, 1).is_err());
        assert!(
            validate_pool_bounds(
                1,
                MAX_QUESTION_POOL_ITEMS_PER_ASSESSMENT_ENTRY + 1,
                MAX_QUESTION_POOL_ITEMS_PER_ASSESSMENT_ENTRY + 1
            )
            .is_err()
        );
        assert!(validate_pool_bounds(2, 1, MAX_ASSESSMENT_QUESTION_POOL_ITEMS + 1).is_err());
    }
}

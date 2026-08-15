//! Protected replay manifest for the reviewed Chapter 1 teaching corpus.

use super::chapter_one::{PilotChapterSpec, PilotQuestionSpec};
use super::chapter_one_identity::pilot_uuid;
use super::*;
use learning_data_access::PublishedProblemRecord;
use std::os::unix::fs::MetadataExt;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[serde(deny_unknown_fields)]
pub(super) struct ChapterOnePilotManifest {
    pub(super) chapters: Vec<ChapterManifest>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[serde(deny_unknown_fields)]
pub(super) struct ChapterManifest {
    pub(super) slug: String,
    pub(super) course_id: CourseId,
    pub(super) assignment_id: AssignmentId,
    pub(super) enrollment_id: EnrollmentId,
    pub(super) questions: Vec<QuestionManifest>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[serde(deny_unknown_fields)]
pub(super) struct QuestionManifest {
    pub(super) slug: String,
    pub(super) display_id: String,
    pub(super) problem_id: ProblemId,
    pub(super) version_id: VersionId,
}

/// The durable course records are the Chapter 1 corpus state. Published
/// questions deliberately have generated identities, so they cannot safely
/// decide whether a host-side replay manifest belongs to this database.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum ChapterOneCorpusState {
    Fresh,
    Published,
}

/// Selects fresh publication or protected replay from the durable Chapter 1
/// course markers. A host manifest is consulted only after the database
/// proves that this corpus was already published.
pub(super) async fn select_chapter_one_resume_manifest(
    store: &learning_data_access::postgres::PostgresStore,
    context: TenantContext,
    tenant: TenantId,
    chapters: &[PilotChapterSpec],
    manifest_path: Option<&str>,
) -> Result<Option<ChapterOnePilotManifest>> {
    let mut markers = Vec::with_capacity(chapters.len());
    for chapter in chapters {
        let course = store
            .get_course(
                context,
                CourseId::from_uuid(pilot_uuid(tenant, &chapter.slug, "course")),
            )
            .await?;
        markers.push(course.is_some());
    }
    let state = chapter_one_corpus_state(markers)?;
    let Some(path) = chapter_one_resume_manifest_path(state, manifest_path)? else {
        return Ok(None);
    };
    let manifest = read_resume_manifest(path)?;
    validate_resume_manifest(&manifest, tenant, chapters)?;
    Ok(Some(manifest))
}

pub(super) fn chapter_one_resume_manifest_path(
    state: ChapterOneCorpusState,
    candidate: Option<&str>,
) -> Result<Option<&str>> {
    match state {
        // A stale host file must not turn an empty disposable database into a
        // replay. Fresh publication owns new generated Question IDs.
        ChapterOneCorpusState::Fresh => Ok(None),
        ChapterOneCorpusState::Published => candidate.map(Some).context(
            "Chapter 1 corpus is already published; supply --chapter-one-existing-manifest to resume safely",
        ),
    }
}

/// Interprets the two deterministic, non-question course markers as one
/// corpus state. A partial state cannot prove either a fresh or replay-safe
/// publication and therefore stops before publication.
pub(super) fn chapter_one_corpus_state(
    markers: impl IntoIterator<Item = bool>,
) -> Result<ChapterOneCorpusState> {
    let mut any_present = false;
    let mut any_absent = false;
    for present in markers {
        any_present |= present;
        any_absent |= !present;
    }
    match (any_present, any_absent) {
        (false, true) => Ok(ChapterOneCorpusState::Fresh),
        (true, false) => Ok(ChapterOneCorpusState::Published),
        (true, true) => bail!(
            "Chapter 1 corpus has a partial course-marker state; repair the disposable database before seeding"
        ),
        (false, false) => bail!("Chapter 1 corpus has no expected course markers"),
    }
}

pub(super) fn read_resume_manifest(path: &str) -> Result<ChapterOnePilotManifest> {
    let metadata = std::fs::symlink_metadata(path)
        .context("reading --chapter-one-existing-manifest metadata")?;
    if !metadata.file_type().is_file() || metadata.file_type().is_symlink() {
        bail!("--chapter-one-existing-manifest must be a regular file, not a symlink");
    }
    if metadata.mode() & 0o7777 != 0o600 {
        bail!("--chapter-one-existing-manifest must have mode 0600 on Unix hosts");
    }
    if metadata.len() > 64 * 1024 {
        bail!("--chapter-one-existing-manifest exceeds the 64 KiB limit");
    }
    let manifest: ChapterOnePilotManifest = serde_json::from_slice(
        &std::fs::read(path).context("reading --chapter-one-existing-manifest")?,
    )
    .context("parsing --chapter-one-existing-manifest")?;
    Ok(manifest)
}

pub(super) fn validate_resume_manifest(
    manifest: &ChapterOnePilotManifest,
    tenant: TenantId,
    tracked: &[PilotChapterSpec],
) -> Result<()> {
    if manifest.chapters.len() != tracked.len() {
        bail!("existing Chapter 1 manifest chapter count differs from tracked corpus");
    }
    let mut enrollments = std::collections::BTreeSet::new();
    let mut question_ids = std::collections::BTreeSet::new();
    let mut references = std::collections::BTreeSet::new();
    for (actual, expected) in manifest.chapters.iter().zip(tracked) {
        if actual.slug != expected.slug
            || actual.course_id != CourseId::from_uuid(pilot_uuid(tenant, &expected.slug, "course"))
            || actual.assignment_id
                != AssignmentId::from_uuid(pilot_uuid(tenant, &expected.slug, "assignment"))
            || actual.questions.len() != expected.questions.len()
        {
            bail!("existing Chapter 1 manifest does not match tracked chapter identity");
        }
        if !enrollments.insert(actual.enrollment_id) {
            bail!("existing Chapter 1 manifest reuses an enrollment ID");
        }
        for (question, tracked_question) in actual.questions.iter().zip(&expected.questions) {
            if question.slug != tracked_question.slug {
                bail!("existing Chapter 1 manifest question order differs from tracked corpus");
            }
            let parsed: question_model::ProblemDisplayRef =
                question.display_id.parse().map_err(|_| {
                    anyhow::anyhow!("existing Chapter 1 manifest contains an invalid Question ID")
                })?;
            if parsed.to_string() != question.display_id
                || !question_ids.insert(question.display_id.clone())
                || !references.insert((question.problem_id, question.version_id))
            {
                bail!(
                    "existing Chapter 1 manifest repeats or non-canonically encodes a question identity"
                );
            }
        }
    }
    Ok(())
}

pub(super) async fn resumed_question(
    store: &learning_data_access::postgres::PostgresStore,
    context: TenantContext,
    manifest: Option<&ChapterOnePilotManifest>,
    chapter_slug: &str,
    spec: &PilotQuestionSpec,
) -> Result<Option<PublishedProblemRecord>> {
    let Some(manifest) = manifest else {
        return Ok(None);
    };
    let manifest_question = manifest
        .chapters
        .iter()
        .find(|chapter| chapter.slug == chapter_slug)
        .and_then(|chapter| {
            chapter
                .questions
                .iter()
                .find(|question| question.slug == spec.slug)
        })
        .context("existing Chapter 1 manifest does not contain the tracked question")?;
    let display = manifest_question.display_id.parse().map_err(|error| {
        anyhow::anyhow!("existing Chapter 1 manifest contains an invalid Question ID: {error}")
    })?;
    let record = store
        .resolve_catalog_problem(context, display)
        .await?
        .context("existing Chapter 1 manifest Question ID no longer resolves")?;
    if record.problem != manifest_question.problem_id
        || record.version != manifest_question.version_id
    {
        bail!("existing Chapter 1 manifest Question ID resolved to a different immutable version");
    }
    Ok(Some(record))
}

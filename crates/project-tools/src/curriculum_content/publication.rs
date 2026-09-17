//! Fresh canonical Genetics Blueprint publication through ordinary lifecycles.

use std::collections::BTreeMap;
use std::path::Path;

use anyhow::{Context, Result, bail, ensure};
use learning_data_access::{
    BlueprintCourseStore, PublishedQuestionLibraryEntry, QuestionLibraryStore, SessionTokenHash,
    StoredBlueprintAssessmentEntry, StoredBlueprintCourse, StoredBlueprintCourseContent,
    postgres::{PostgresBlueprintCourseStore, PostgresQuestionLibraryStore, lazy_pool},
};
use question_model::{
    AssessmentEntryScoringRule, BlueprintAssessmentEntryInput, BlueprintAvailability,
    BlueprintRevision, CreateBlueprintCourseInput, QuestionAttemptLimit, QuestionAttemptTimeLimit,
    QuestionAuthor, QuestionAuthorDisplayName, QuestionAuthorship, QuestionBackend,
    QuestionLicense, QuestionRevisionReference, RequestChecksum, WorkspaceId,
};
use sha2::{Digest, Sha256};
use uuid::Uuid;

use super::{Course, Manifest, ParameterizedSource, repository_root};

mod blueprint_input;
mod receipt;

use blueprint_input::blueprint_input;
pub(crate) use receipt::Receipt;

const SESSION_HASH_ENV: &str = "PLE_CURRICULUM_PUBLICATION_SESSION_TOKEN_HASH";
const WORKSPACE_ENV: &str = "PLE_CURRICULUM_PUBLICATION_WORKSPACE_ID";

type SourceRevisions = BTreeMap<String, QuestionRevisionReference>;
type PublishedIndex = BTreeMap<String, Vec<PublishedQuestionLibraryEntry>>;

pub(crate) fn publish(manifest: Manifest) -> Result<()> {
    let session = required_session_hash()?;
    let workspace = required_workspace()?;
    let root = repository_root()?;
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .context("creating the curriculum publication runtime")?;
    let receipt = runtime
        .block_on(async move { publish_with_context(session, workspace, manifest, &root).await })?;
    println!("{receipt}");
    Ok(())
}

pub(crate) async fn publish_with_context(
    session: SessionTokenHash,
    workspace: WorkspaceId,
    manifest: Manifest,
    root: &Path,
) -> Result<Receipt> {
    // All source bytes and metadata are admitted before database configuration
    // or writes. The same-name Blueprint check below is also read-only and
    // precedes the ordinary Question publisher.
    super::validate(&manifest, root)?;
    let database_url = required_environment("DATABASE_URL")?;
    let pool =
        lazy_pool(&database_url).context("curriculum publication database URL is invalid")?;
    let library = PostgresQuestionLibraryStore::new(pool.clone());
    let classification_store =
        learning_data_access::postgres::PostgresContentClassificationStore::new(pool.clone());
    let resolved = manifest
        .course
        .classification
        .resolve(&classification_store, session)
        .await?;
    let classification = question_model::CourseClassification {
        discipline_uuid: resolved.discipline_uuid,
        subject_uuid: Some(resolved.subject_uuid),
        topic_uuid: resolved.topic_uuid,
        subtopic_uuid: resolved.subtopic_uuid,
        tags: Vec::new(),
    };
    let blueprints = PostgresBlueprintCourseStore::new(pool);
    let authorship = curriculum_authorship(&manifest)?;
    let license = curriculum_license(&manifest)?;

    let existing = published_index(
        library
            .list_published_question_library_entries(session)
            .await
            .context("reading ordinary published Question provenance before publication")?,
    );
    let existing_revisions = existing_source_revisions(
        &manifest.parameterized_sources,
        &existing,
        &authorship,
        &license,
    )?;
    if let Some(retained) = matching_blueprint(session, &manifest.course, &blueprints).await? {
        ensure!(
            existing_revisions.len() == manifest.parameterized_sources.len(),
            "same-name Genetics Blueprint does not resolve every canonical source before publication"
        );
        let input = blueprint_input(&manifest, &existing_revisions, classification.clone())?;
        ensure!(
            retained.classification == classification,
            "same-name Genetics Blueprint classification conflicts with the authored Course"
        );
        validate_loaded_content(&retained.content, &input, &manifest, &existing_revisions)
            .context("same-name Genetics Blueprint conflicts with the canonical fresh catalog")?;
        return Receipt::new(
            retained.reference.to_string(),
            retained.current_revision.value(),
            &manifest,
            &existing_revisions,
        );
    }

    let publication = super::parameterized_publication::publish_with_context(
        session,
        workspace,
        manifest.clone(),
        root,
    )
    .await
    .context("publishing canonical Genetics Questions")?;
    let revisions = publication.question_revisions();
    let input = blueprint_input(&manifest, &revisions, classification)?;
    let (reference, revision) =
        create_blueprint(session, &input, &manifest, &revisions, &blueprints).await?;
    Receipt::new(
        reference.to_string(),
        revision.value(),
        &manifest,
        &revisions,
    )
}

async fn matching_blueprint(
    session: SessionTokenHash,
    course: &Course,
    store: &PostgresBlueprintCourseStore,
) -> Result<Option<StoredBlueprintCourse>> {
    // Include retained history so archive cannot bypass the name collision/reuse guard.
    let mut matching = Vec::new();
    let mut page = learning_data_access::PageRequest::first(
        learning_data_access::PageSize::new(100).expect("bounded publication inventory"),
    );
    loop {
        let result = store
            .list_blueprint_courses(
                session,
                learning_data_access::BlueprintCourseListRequest {
                    page: page.clone(),
                    query: String::new(),
                    include_archived: true,
                    public_only: false,
                    promoted_only: false,
                    discipline_uuid: None,
                    subject_uuid: None,
                    topic_uuid: None,
                    subtopic_uuid: None,
                    cross_discipline: false,
                },
            )
            .await
            .context(
                "listing Blueprint Courses including Archived before curriculum publication",
            )?;
        matching.extend(result.items.into_iter().filter(|summary| {
            summary.short_name == course.short_name || summary.long_name == course.long_name
        }));
        match result.next_cursor {
            Some(cursor) => page.after = Some(cursor),
            None => break,
        }
    }
    ensure!(
        matching.len() <= 1,
        "more than one ordinary Blueprint Course collides with a Genetics catalog name"
    );
    let Some(summary) = matching.first() else {
        return Ok(None);
    };
    ensure!(
        summary.short_name == course.short_name && summary.long_name == course.long_name,
        "ordinary Blueprint Course has a partial Genetics catalog name collision"
    );
    store
        .load_blueprint_course(session, summary.reference.clone())
        .await
        .context("loading same-name Genetics Blueprint before publication")
        .map(Some)
}

fn published_index(entries: Vec<PublishedQuestionLibraryEntry>) -> PublishedIndex {
    let mut index = PublishedIndex::new();
    for entry in entries {
        index
            .entry(entry.source_object_checksum.as_str().to_owned())
            .or_default()
            .push(entry);
    }
    index
}

fn existing_source_revisions(
    sources: &[ParameterizedSource],
    entries: &PublishedIndex,
    authorship: &QuestionAuthorship,
    license: &QuestionLicense,
) -> Result<SourceRevisions> {
    let mut revisions = SourceRevisions::new();
    for source in sources {
        let matches = entries
            .get(&source.pg_sha256)
            .map(Vec::as_slice)
            .unwrap_or_default();
        ensure!(
            matches.len() <= 1,
            "ordinary Question provenance has more than one lineage for canonical source {}",
            source.source_id
        );
        let Some(entry) = matches.first() else {
            continue;
        };
        ensure_source_entry_compatible(entry, source, authorship, license)?;
        ensure!(
            revisions
                .insert(source.source_id.clone(), entry.question_revision.clone())
                .is_none(),
            "canonical source identity is duplicated: {}",
            source.source_id
        );
    }
    Ok(revisions)
}

fn ensure_source_entry_compatible(
    entry: &PublishedQuestionLibraryEntry,
    source: &ParameterizedSource,
    authorship: &QuestionAuthorship,
    license: &QuestionLicense,
) -> Result<()> {
    ensure!(
        entry.question_title == source.question_title
            && entry.question_description == source.question_description
            && entry.backend == QuestionBackend::Webwork
            && entry.question_format == super::webwork_question_format(source.source_format)
            && entry.question_type
                == super::parameterized_publication::question_type(source.question_type)
            && entry.source_media_type == "text/x-wework-pg"
            && entry.source_object_checksum.as_str() == source.pg_sha256
            && entry.webwork_pg_path.as_deref() == Some(source.webwork_pg_path.as_str())
            && entry.authorship == *authorship
            && entry.question_license == *license,
        "ordinary Question provenance conflicts with canonical source {}",
        source.source_id
    );
    Ok(())
}

async fn create_blueprint(
    session: SessionTokenHash,
    input: &CreateBlueprintCourseInput,
    manifest: &Manifest,
    revisions: &SourceRevisions,
    store: &PostgresBlueprintCourseStore,
) -> Result<(question_model::BlueprintCourseReference, BlueprintRevision)> {
    let checksum = request_checksum(input)?;
    let receipt = store
        .create_blueprint_course(session, checksum, input.clone(), Default::default())
        .await
        .context("creating canonical Genetics Blueprint Course")?;
    ensure!(
        receipt.blueprint_revision.revision == BlueprintRevision::INITIAL,
        "canonical Genetics Blueprint creation did not return Revision 1"
    );
    let loaded = store
        .load_blueprint_course(session, receipt.blueprint_revision.reference.clone())
        .await
        .context("reloading canonical Genetics Blueprint Course")?;
    ensure!(
        loaded.availability == BlueprintAvailability::Private,
        "new canonical Genetics Blueprint Course is not Private"
    );
    validate_loaded_content(&loaded.content, input, manifest, revisions)?;
    ensure!(
        loaded.classification == input.classification,
        "created Genetics Blueprint classification differs from authored metadata"
    );
    Ok((loaded.reference, loaded.current_revision))
}

fn validate_loaded_content(
    actual: &StoredBlueprintCourseContent,
    expected: &CreateBlueprintCourseInput,
    manifest: &Manifest,
    revisions: &SourceRevisions,
) -> Result<()> {
    ensure!(
        actual.modules.len() == 1 && expected.modules.len() == 1,
        "canonical Genetics Blueprint must contain one module"
    );
    let actual_module = &actual.modules[0];
    let expected_module = &expected.modules[0];
    ensure!(
        actual_module.label == expected_module.label
            && actual_module.assessments.len() == expected_module.assessments.len()
            && actual_module.assessments.len() == manifest.topics.len(),
        "canonical Genetics Blueprint topic structure differs"
    );
    for ((actual, expected), topic) in actual_module
        .assessments
        .iter()
        .zip(&expected_module.assessments)
        .zip(&manifest.topics)
    {
        ensure!(
            actual.content.assessment_type == expected.assessment_type
                && actual.content.title == expected.title
                && actual.content.instructions == expected.instructions
                && actual.content.defaults == expected.defaults
                && actual.content.entries.len() == expected.entries.len()
                && actual.content.entries.len() == topic.source_ids.len(),
            "canonical Genetics Blueprint Assessment content differs"
        );
        for ((actual_entry, expected_entry), source_id) in actual
            .content
            .entries
            .iter()
            .zip(&expected.entries)
            .zip(&topic.source_ids)
        {
            let (
                StoredBlueprintAssessmentEntry::Fixed {
                    question_revision,
                    points_possible,
                    scoring_rule,
                    question_attempt_limit,
                    question_attempt_time_limit,
                },
                BlueprintAssessmentEntryInput::Fixed(expected_fixed),
            ) = (actual_entry, expected_entry)
            else {
                bail!("canonical Genetics Blueprint entries must all be direct Fixed Questions");
            };
            let expected_revision = revisions.get(source_id).with_context(|| {
                format!("canonical Genetics revision is missing for source {source_id}")
            })?;
            ensure!(
                question_revision == expected_revision
                    && question_revision == &expected_fixed.published_question
                    && points_possible == &expected_fixed.points_possible
                    && scoring_rule == &AssessmentEntryScoringRule::Normal
                    && scoring_rule == &expected_fixed.scoring_rule
                    && question_attempt_limit == &QuestionAttemptLimit { max_attempts: None }
                    && question_attempt_limit == &expected_fixed.question_attempt_limit
                    && question_attempt_time_limit == &QuestionAttemptTimeLimit::Unlimited
                    && question_attempt_time_limit == &expected_fixed.question_attempt_time_limit,
                "canonical Genetics fixed Question pin or policy differs for {source_id}"
            );
        }
    }
    Ok(())
}

fn curriculum_authorship(manifest: &Manifest) -> Result<QuestionAuthorship> {
    QuestionAuthorship::new(vec![QuestionAuthor {
        display_name: QuestionAuthorDisplayName::new(manifest.course.author.clone())
            .map_err(|_| anyhow::anyhow!("curriculum source author is invalid"))?,
    }])
    .map_err(|_| anyhow::anyhow!("curriculum source authorship is invalid"))
}

fn curriculum_license(manifest: &Manifest) -> Result<QuestionLicense> {
    serde_json::from_value(serde_json::Value::String(
        manifest.course.content_license.clone(),
    ))
    .map_err(|_| anyhow::anyhow!("curriculum content license is not publishable"))
}

fn request_checksum(input: &impl serde::Serialize) -> Result<RequestChecksum> {
    let bytes =
        serde_json::to_vec(input).context("encoding normalized curriculum Blueprint input")?;
    Ok(RequestChecksum::from_bytes(Sha256::digest(bytes).into()))
}

fn required_session_hash() -> Result<SessionTokenHash> {
    let value = required_environment(SESSION_HASH_ENV)?;
    SessionTokenHash::from_hex(&value)
        .map_err(|_| anyhow::anyhow!("{SESSION_HASH_ENV} must be a session-token hash"))
}

fn required_workspace() -> Result<WorkspaceId> {
    let value = required_environment(WORKSPACE_ENV)?;
    Ok(WorkspaceId::from_uuid(Uuid::parse_str(&value).map_err(
        |_| anyhow::anyhow!("{WORKSPACE_ENV} must be a UUID"),
    )?))
}

fn required_environment(name: &str) -> Result<String> {
    let value = std::env::var(name).with_context(|| format!("{name} must be set"))?;
    ensure!(
        !value.is_empty() && !value.contains(['\0', '\n', '\r']),
        "{name} must not be empty"
    );
    Ok(value)
}

#[cfg(test)]
mod tests;

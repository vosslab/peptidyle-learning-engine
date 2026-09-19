//! Canonical Genetics source publication through the ordinary Question lifecycle.
//!
//! The explicit selected-source command and the fresh catalog batch share this
//! ordinary Draft-to-Published-Question path. It never creates a Pool or mutates
//! a Blueprint, archive, or historical Revision.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{Context, Result, bail, ensure};
use learning_data_access::{
    AuthoringDraft, AuthoringDraftStore, CreateAuthoringDraftInput,
    DraftQuestionSourceBindingInput, DraftQuestionSourceBindingStore, DraftQuestionUuid,
    PublishedQuestionLibraryEntry, QuestionLibraryStore, SessionTokenHash,
    postgres::{
        PostgresAuthoringDraftStore, PostgresDraftQuestionSourceBindingStore,
        PostgresQuestionLibraryStore, lazy_pool,
    },
};
use objects::{ObjectAddress, ObjectStore, PutObject};
use question_model::{
    ObjectId, QuestionAuthor, QuestionAuthorDisplayName, QuestionAuthorship, QuestionBackend,
    QuestionFormat, QuestionLicense, QuestionRevisionReason, QuestionRevisionTuple, QuestionType,
    SourceObjectChecksum, Timestamp, WorkspaceId,
};
use server_core::question_publication::{
    NewQuestionLineagePublicationCommand, NewQuestionLineagePublisher, RandomQuestionIdIssuer,
};
use uuid::Uuid;

use super::{
    Manifest, ParameterizedSource, read_parameterized_pg_source, repository_root,
    validate_selected_parameterized_manifest, webwork_question_format,
};

const SESSION_HASH_ENV: &str = "PLE_CURRICULUM_PUBLICATION_SESSION_TOKEN_HASH";
const WORKSPACE_ENV: &str = "PLE_CURRICULUM_PUBLICATION_WORKSPACE_ID";
const INITIAL_PUBLICATION_REASON: &str =
    "Initial publication from trusted parameterized curriculum import";

/// Runs one explicitly selected parameterized-only ordinary publication.
pub(crate) fn publish_selected(manifest: Manifest) -> Result<()> {
    let session = required_session_hash()?;
    let workspace = required_workspace()?;
    let root = repository_root()?;
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .context("creating the parameterized curriculum publication runtime")?;
    let receipt = runtime.block_on(async move {
        publish_selected_with_context(session, workspace, manifest, &root).await
    })?;
    println!("{receipt}");
    Ok(())
}

/// Publishes exactly the manifest's approved parameterized sources.
///
/// The caller must have loaded the complete manifest with `load` or
/// `load_from_root`; those validation functions verify every local source pin,
/// canonical author-source provenance, license pair, and self-contained
/// Question metadata before this function starts a write. Every actual write
/// still goes through the regular Draft -> source binding -> immutable
/// new-lineage publisher.
pub(crate) async fn publish_with_context(
    session: SessionTokenHash,
    workspace: WorkspaceId,
    manifest: Manifest,
    root: &Path,
) -> Result<Receipt> {
    // ASVS 2.2.1-2.2.3: even trusted callers of this internal helper must
    // validate the exact bounded manifest, source pins, provenance, licenses,
    // and self-contained Question metadata before the first object-store write.
    super::validate(&manifest, root)?;
    publish_validated_with_context(session, workspace, manifest, root).await
}

/// Publishes the one source retained by the selected-source loader.
///
/// This admission path intentionally does not validate unrelated static rows.
/// It revalidates the exact selected metadata and source bytes before reading
/// publication configuration, opening database connections, or writing.
pub(crate) async fn publish_selected_with_context(
    session: SessionTokenHash,
    workspace: WorkspaceId,
    manifest: Manifest,
    root: &Path,
) -> Result<Receipt> {
    validate_selected_parameterized_manifest(&manifest, root)?;
    publish_validated_with_context(session, workspace, manifest, root).await
}

async fn publish_validated_with_context(
    session: SessionTokenHash,
    workspace: WorkspaceId,
    manifest: Manifest,
    root: &Path,
) -> Result<Receipt> {
    let database_url = required_environment("DATABASE_URL")?;
    let pool = lazy_pool(&database_url)
        .context("parameterized curriculum publication database URL is invalid")?;
    let drafts = PostgresAuthoringDraftStore::new(pool.clone());
    let bindings = PostgresDraftQuestionSourceBindingStore::new(pool.clone());
    let classification_store =
        learning_data_access::postgres::PostgresContentClassificationStore::new(pool.clone());
    let library = PostgresQuestionLibraryStore::new(pool.clone());
    let mut classifications = BTreeMap::new();
    let objects = server_core::composition::question_library_object_store_from_env()
        .await
        .context("configuring ordinary Question source object store")?;
    let issuer = server_core::composition::question_id_issuer();
    let authorship = QuestionAuthorship::new(vec![QuestionAuthor {
        display_name: QuestionAuthorDisplayName::new(manifest.course.author.clone())
            .map_err(|_| anyhow::anyhow!("curriculum source author is invalid"))?,
    }])
    .map_err(|_| anyhow::anyhow!("curriculum source authorship is invalid"))?;
    let license = serde_json::from_value::<QuestionLicense>(serde_json::Value::String(
        manifest.course.content_license.clone(),
    ))
    .map_err(|_| anyhow::anyhow!("curriculum content license is not publishable"))?;

    // Build the admission plan before the first Object Store or authoring
    // write. ASVS 2.3.1 and 2.3.3: validation and recovery preflight are
    // fail-closed; every actual write uses the ordinary atomic lifecycle.
    let prepared_sources = manifest
        .parameterized_sources
        .iter()
        .map(|source| {
            Ok(PreparedSource {
                context: source_context(&manifest, source)?,
                bytes: read_parameterized_pg_source(root, source)?,
                source: source.clone(),
            })
        })
        .collect::<Result<Vec<_>>>()?;
    let existing = published_index(
        library
            .list_published_question_library_entries(session)
            .await
            .context("reading ordinary published Question provenance")?,
    );
    let draft_index = preflight_drafts(session, workspace, &prepared_sources, &drafts).await?;
    let admitted = prepared_sources
        .into_iter()
        .map(|prepared| {
            let existing = existing_publication(
                &existing,
                &prepared.source,
                &prepared.context,
                &authorship,
                &license,
            )?;
            let resumable_draft = draft_index
                .get(&(
                    prepared.context.title.clone(),
                    prepared.context.description.clone(),
                ))
                .cloned()
                .unwrap_or(None);
            Ok(AdmittedSource {
                prepared,
                existing,
                resumable_draft,
            })
        })
        .collect::<Result<Vec<_>>>()?;

    // Existing immutable provenance is reusable despite later metadata edits.
    // Resolve only new lineages, all before the first write in this batch.
    for source in admitted
        .iter()
        .filter(|entry| entry.existing.is_none())
        .map(|entry| &entry.prepared.source)
    {
        classifications.insert(
            source.source_id.clone(),
            source
                .classification
                .as_ref()
                .with_context(|| {
                    format!(
                        "{} requires explicit authored classification before publication",
                        source.source_id
                    )
                })?
                .resolve(&classification_store, session)
                .await
                .with_context(|| {
                    format!(
                        "resolving curriculum classification for {}",
                        source.source_id
                    )
                })?,
        );
    }
    let mut published = BTreeMap::new();
    for admitted_source in admitted {
        let PreparedSource {
            source,
            context,
            bytes,
        } = admitted_source.prepared;
        let revision = match admitted_source.existing {
            Some(revision) => revision,
            None => {
                publish_source(
                    session,
                    workspace,
                    &source,
                    classifications
                        .get(&source.source_id)
                        .context("curriculum classification admission is incomplete")?,
                    &context,
                    bytes,
                    admitted_source.resumable_draft,
                    &authorship,
                    &license,
                    &drafts,
                    &bindings,
                    &objects,
                    &issuer,
                )
                .await?
            }
        };
        let source_id = source.source_id;
        ensure!(
            published.insert(source_id.clone(), revision).is_none(),
            "parameterized curriculum source identity duplicated: {}",
            source_id
        );
    }
    Receipt::new(&manifest, published)
}

#[derive(Debug, Clone)]
struct SourceContext {
    title: String,
    description: String,
    question_type: QuestionType,
}

struct PreparedSource {
    source: ParameterizedSource,
    context: SourceContext,
    bytes: Vec<u8>,
}

struct AdmittedSource {
    prepared: PreparedSource,
    existing: Option<QuestionRevisionTuple>,
    resumable_draft: Option<AuthoringDraft>,
}

type DraftIndex = BTreeMap<(String, String), Option<AuthoringDraft>>;

/// Preloads the only resumable Draft candidates before the first write. A
/// same-metadata Draft with another source pin is an ambiguous recovery state,
/// so admission stops instead of silently creating a second Draft.
async fn preflight_drafts(
    session: SessionTokenHash,
    workspace: WorkspaceId,
    sources: &[PreparedSource],
    drafts: &PostgresAuthoringDraftStore,
) -> Result<DraftIndex> {
    let expected = sources
        .iter()
        .map(|source| {
            (
                (
                    source.context.title.clone(),
                    source.context.description.clone(),
                ),
                &source.source,
            )
        })
        .collect::<BTreeMap<_, _>>();
    let keys = expected.keys().cloned().collect::<BTreeSet<_>>();
    let mut candidates = BTreeMap::<(String, String), Vec<AuthoringDraft>>::new();
    for summary in drafts
        .list_authoring_drafts(session)
        .await
        .context("listing ordinary Authoring Drafts for batch admission")?
    {
        let key = (summary.title, summary.description);
        if keys.contains(&key) {
            let draft = drafts
                .load_authoring_draft(session, summary.draft_question_uuid)
                .await
                .context("loading ordinary Authoring Draft for batch admission")?;
            candidates.entry(key).or_default().push(draft);
        }
    }
    let mut index = DraftIndex::new();
    for (key, source) in expected {
        let resumable = candidates.remove(&key).unwrap_or_default();
        if resumable.iter().any(|draft| {
            draft.workspace != workspace
                || draft.source_record.sha256.to_string() != source.pg_sha256
        }) {
            bail!(
                "ordinary Authoring Draft provenance conflicts with parameterized source {}",
                source.source_id
            );
        }
        ensure!(
            resumable.len() <= 1,
            "ordinary Authoring Draft provenance has more than one resumable Draft for parameterized source {}",
            source.source_id
        );
        index.insert(key, resumable.into_iter().next());
    }
    Ok(index)
}

fn source_context(manifest: &Manifest, source: &ParameterizedSource) -> Result<SourceContext> {
    ensure!(
        manifest
            .topics
            .iter()
            .any(|topic| topic.slug == source.topic_slug),
        "parameterized source {} has no manifest Topic",
        source.source_id
    );
    Ok(SourceContext {
        title: source.question_title.clone(),
        description: source.question_description.clone(),
        question_type: question_type(source.question_type),
    })
}

type PublishedIndex = BTreeMap<String, Vec<PublishedQuestionLibraryEntry>>;

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

fn existing_publication(
    entries: &PublishedIndex,
    source: &ParameterizedSource,
    context: &SourceContext,
    authorship: &QuestionAuthorship,
    license: &QuestionLicense,
) -> Result<Option<QuestionRevisionTuple>> {
    let matches = entries
        .get(&source.pg_sha256)
        .map(Vec::as_slice)
        .unwrap_or_default();
    if matches.len() > 1 {
        bail!(
            "ordinary Question provenance has more than one lineage for parameterized source {}",
            source.source_id
        );
    }
    let Some(entry) = matches.first() else {
        return Ok(None);
    };
    ensure!(
        entry.question_title == context.title
            && entry.question_description == context.description
            && entry.backend == QuestionBackend::Webwork
            && entry.question_format == webwork_question_format(source.source_format)
            && entry.question_type == context.question_type
            && entry.source_media_type == "text/x-wework-pg"
            && entry.source_object_checksum.as_str() == source.pg_sha256
            && entry.webwork_pg_path.as_deref() == Some(source.webwork_pg_path.as_str())
            && entry.authorship == *authorship
            && entry.question_license == *license,
        "ordinary Question provenance conflicts with parameterized source {}",
        source.source_id
    );
    Ok(Some(entry.question_revision_tuple.clone()))
}

#[allow(clippy::too_many_arguments)]
async fn publish_source(
    session: SessionTokenHash,
    workspace: WorkspaceId,
    source: &ParameterizedSource,
    classification: &crate::pilot_content::ResolvedClassification,
    context: &SourceContext,
    bytes: Vec<u8>,
    resumable_draft: Option<AuthoringDraft>,
    authorship: &QuestionAuthorship,
    license: &QuestionLicense,
    drafts: &PostgresAuthoringDraftStore,
    bindings: &PostgresDraftQuestionSourceBindingStore,
    objects: &objects::s3::S3ObjectStore,
    issuer: &RandomQuestionIdIssuer,
) -> Result<QuestionRevisionTuple> {
    let draft = matching_or_new_draft(
        session,
        workspace,
        source,
        context,
        bytes,
        resumable_draft,
        drafts,
        objects,
    )
    .await?;
    let checksum = SourceObjectChecksum::parse(source.pg_sha256.clone()).map_err(|_| {
        anyhow::anyhow!(
            "parameterized PG checksum is invalid for {}",
            source.source_id
        )
    })?;
    let bound_edit_number = bindings
        .bind_draft_question_source(
            session,
            DraftQuestionSourceBindingInput {
                draft_question_uuid: draft.draft_question_uuid,
                expected_draft_question_edit_number: draft.edit_number,
                workspace,
                question_backend: QuestionBackend::Webwork,
                question_format: webwork_question_format(source.source_format),
                question_type: context.question_type,
                webwork_pg_path: Some(source.webwork_pg_path.clone()),
                draft_imathas_question_backend_binding: None,
                source_object_id: draft.source_record.id,
                source_object_checksum: checksum,
            },
        )
        .await
        .context("binding ordinary parameterized curriculum Draft source evidence")?;
    NewQuestionLineagePublisher::new(objects.clone(), bindings.clone(), *issuer, None)
        .publish(
            session,
            NewQuestionLineagePublicationCommand {
                draft_question_uuid: draft.draft_question_uuid,
                expected_draft_question_edit_number: bound_edit_number,
                workspace,
                question_authorship: authorship.clone(),
                discipline_uuid: classification.discipline_uuid,
                subject_uuid: classification.subject_uuid,
                topic_uuid: classification.topic_uuid,
                subtopic_uuid: classification.subtopic_uuid,
                initial_shared_tags: Vec::new(),
                question_license: license.clone(),
                question_revision_reason: QuestionRevisionReason::new(
                    INITIAL_PUBLICATION_REASON.to_owned(),
                )
                .expect("fixed parameterized publication reason is valid"),
            },
            now(),
        )
        .await
        .context("publishing ordinary parameterized curriculum Question lineage")
}

#[allow(clippy::too_many_arguments)]
async fn matching_or_new_draft(
    session: SessionTokenHash,
    workspace: WorkspaceId,
    source: &ParameterizedSource,
    context: &SourceContext,
    bytes: Vec<u8>,
    resumable_draft: Option<AuthoringDraft>,
    drafts: &PostgresAuthoringDraftStore,
    objects: &objects::s3::S3ObjectStore,
) -> Result<AuthoringDraft> {
    if let Some(draft) = resumable_draft {
        return Ok(draft);
    }
    let source_record = objects
        .put(PutObject {
            address: ObjectAddress::WorkspaceQuestionSource {
                workspace,
                object: ObjectId::generate(),
            },
            bytes,
            media_type: "text/x-wework-pg".to_owned(),
            created_at: now(),
        })
        .await
        .context("writing ordinary parameterized curriculum Draft source bytes")?;
    drafts
        .create_authoring_draft(
            session,
            workspace,
            CreateAuthoringDraftInput {
                draft_question_uuid: DraftQuestionUuid::from_uuid(Uuid::now_v7()),
                source_record,
                question_format: webwork_question_format(source.source_format),
                webwork_pg_path: Some(source.webwork_pg_path.clone()),
                question_type: context.question_type,
                title: context.title.clone(),
                description: context.description.clone(),
                language: "en".to_owned(),
            },
        )
        .await
        .context("creating ordinary parameterized curriculum Authoring Draft")
}

pub(crate) fn question_type(value: super::CurriculumQuestionType) -> QuestionType {
    match value {
        super::CurriculumQuestionType::MultipleChoice => QuestionType::MultipleChoice,
        super::CurriculumQuestionType::MultipleAnswer => QuestionType::MultipleAnswer,
        super::CurriculumQuestionType::FillInBlank => QuestionType::FillInBlank,
        super::CurriculumQuestionType::MultipleFillInBlank => QuestionType::MultipleFillInBlank,
        super::CurriculumQuestionType::Numeric => QuestionType::Numeric,
        super::CurriculumQuestionType::Matching => QuestionType::Matching,
    }
}

fn now() -> Timestamp {
    let milliseconds = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .unwrap_or_default();
    Timestamp::from_unix_millis(i64::try_from(milliseconds).unwrap_or(i64::MAX))
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

/// Receipt deliberately contains source pins and ordinary immutable Question
/// Revision Tuples only.  It does not imply a Pool, Blueprint, or archive change.
#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Receipt {
    source_repository: String,
    source_revision: String,
    sources: Vec<ReceiptSource>,
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct ReceiptSource {
    source_id: String,
    topic_slug: String,
    question_title: String,
    question_description: String,
    question_type: QuestionType,
    source_format: QuestionFormat,
    source_path: String,
    source_checksum: String,
    webwork_pg_path: String,
    canonical_author_source_url: String,
    canonical_author_source_checksum: String,
    question_revision_tuple: QuestionRevisionTuple,
}

impl Receipt {
    /// Returns the immutable Question Revision produced for each admitted
    /// source so the fresh publisher can construct direct Fixed entries.
    pub(crate) fn question_revisions(&self) -> BTreeMap<String, QuestionRevisionTuple> {
        self.sources
            .iter()
            .map(|source| {
                (
                    source.source_id.clone(),
                    source.question_revision_tuple.clone(),
                )
            })
            .collect()
    }

    fn new(
        manifest: &Manifest,
        published: BTreeMap<String, QuestionRevisionTuple>,
    ) -> Result<Self> {
        let sources = manifest
            .parameterized_sources
            .iter()
            .map(|source| {
                let question_revision_tuple =
                    published.get(&source.source_id).cloned().with_context(|| {
                        format!(
                            "parameterized receipt is missing source {}",
                            source.source_id
                        )
                    })?;
                Ok(ReceiptSource {
                    source_id: source.source_id.clone(),
                    topic_slug: source.topic_slug.clone(),
                    question_title: source.question_title.clone(),
                    question_description: source.question_description.clone(),
                    question_type: question_type(source.question_type),
                    source_format: webwork_question_format(source.source_format),
                    source_path: source.pg_source.display().to_string(),
                    source_checksum: source.pg_sha256.clone(),
                    webwork_pg_path: source.webwork_pg_path.clone(),
                    canonical_author_source_url: source.canonical_author_source_url.clone(),
                    canonical_author_source_checksum: source.canonical_author_source_sha256.clone(),
                    question_revision_tuple,
                })
            })
            .collect::<Result<Vec<_>>>()?;
        Ok(Self {
            source_repository: manifest.course.source_repository.clone(),
            source_revision: manifest.course.source_revision.clone(),
            sources,
        })
    }
}

impl std::fmt::Display for Receipt {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        serde_json::to_string(self)
            .map_err(|_| std::fmt::Error)?
            .fmt(formatter)
    }
}

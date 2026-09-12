//! Ordinary Authoring Workspace publication for the fixed Pilot sources.

use std::collections::BTreeMap;
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{Context, Result, bail, ensure};
use learning_data_access::{
    AuthoringDraft, AuthoringDraftStore, CreateAuthoringDraftInput,
    DraftQuestionSourceBindingInput, DraftQuestionSourceBindingStore, DraftQuestionUuid,
    QuestionLibraryStore, SessionTokenHash,
    postgres::{
        PostgresAuthoringDraftStore, PostgresDraftQuestionSourceBindingStore,
        PostgresQuestionLibraryStore,
    },
};
use objects::{ObjectAddress, ObjectStore, PutObject};
use question_model::{
    ObjectId, QuestionAuthor, QuestionAuthorDisplayName, QuestionAuthorship, QuestionBackend,
    QuestionFormat, QuestionLicense, QuestionRevisionReason, QuestionRevisionReference,
    SourceObjectChecksum, SourceObjectReference, Timestamp, WorkspaceId,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use server_core::question_publication::{
    NewQuestionLineagePublicationCommand, NewQuestionLineagePublisher,
};

use super::{Backend, PublicationPlan, PublicationSource, publication_plan};

const PILOT_PUBLICATION_SESSION_HASH_ENV: &str = "PLE_PILOT_PUBLICATION_SESSION_TOKEN_HASH";
const PILOT_PUBLICATION_WORKSPACE_ENV: &str = "PLE_PILOT_PUBLICATION_WORKSPACE_ID";
const INITIAL_PUBLICATION_REASON: &str = "Initial publication from Authoring Workspace";

/// One non-secret exact reference handed to the next installation stage.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct PublishedPilotQuestion {
    source_sha256: String,
    question_revision: QuestionRevisionReference,
}

/// Publishes the fixed reviewed Pilot inventory through the ordinary Authoring
/// Workspace and Question Publication stores.
pub(crate) fn publish() -> Result<()> {
    let session = required_session_hash()?;
    let workspace = required_workspace()?;
    println!("{}", publish_with_context(session, workspace)?);
    Ok(())
}

/// Publishes fixed Pilot content using the installation-owned ordinary session
/// and workspace, returning only immutable publication provenance.
pub(crate) fn publish_with_context(
    session: SessionTokenHash,
    workspace: WorkspaceId,
) -> Result<String> {
    let plan = publication_plan()?;
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .context("creating the Pilot publication runtime")?;
    let published = runtime.block_on(async move {
        let database_url = required_environment("DATABASE_URL")?;
        let pool = learning_data_access::postgres::lazy_pool(&database_url)
            .context("Pilot publication database URL is invalid")?;
        let drafts = PostgresAuthoringDraftStore::new(pool.clone());
        let publication = PostgresDraftQuestionSourceBindingStore::new(pool.clone());
        let library = PostgresQuestionLibraryStore::new(pool);
        let objects = server_core::composition::question_library_object_store_from_env()
            .await
            .context("configuring the ordinary Question source object store")?;
        let issuer = server_core::composition::question_id_issuer_from_env()
            .context("loading the Question ID publication capability")?;
        publish_plan(
            session,
            workspace,
            plan,
            PilotPublicationServices {
                drafts: &drafts,
                publication: &publication,
                library: &library,
                objects: &objects,
                issuer: &issuer,
            },
        )
        .await
    })?;
    serde_json::to_string(&published).context("encoding the Pilot publication mapping")
}

/// Confirms that a handoff names exactly the fixed reviewed sources.
pub(crate) fn validate_publication_mapping_json(value: &str) -> Result<()> {
    let mut mapping: BTreeMap<String, PublishedPilotQuestion> =
        serde_json::from_str(value).context("decoding the Pilot publication mapping")?;
    let plan = publication_plan()?;
    ensure!(
        mapping.len() == plan.questions.len(),
        "Pilot publication mapping has the wrong number of questions"
    );
    for source in plan.questions {
        let published = mapping
            .remove(&source.slug)
            .with_context(|| format!("Pilot publication mapping lacks {}", source.slug))?;
        ensure!(
            published.source_sha256 == source.source_sha256,
            "Pilot publication mapping checksum differs for {}",
            source.slug
        );
        ensure!(
            published.question_revision.revision_number.get() > 0,
            "Pilot publication mapping revision is invalid for {}",
            source.slug
        );
    }
    ensure!(
        mapping.is_empty(),
        "Pilot publication mapping has an unknown source"
    );
    Ok(())
}

/// The ordinary stores and capabilities that make one Pilot publication run.
///
/// They share a lifetime because the operation borrows all of them until every
/// source has either been confirmed or published.
struct PilotPublicationServices<'a> {
    drafts: &'a PostgresAuthoringDraftStore,
    publication: &'a PostgresDraftQuestionSourceBindingStore,
    library: &'a PostgresQuestionLibraryStore,
    objects: &'a objects::s3::S3ObjectStore,
    issuer: &'a server_core::question_publication::HmacQuestionIdIssuer,
}

async fn publish_plan(
    session: SessionTokenHash,
    workspace: WorkspaceId,
    plan: PublicationPlan,
    services: PilotPublicationServices<'_>,
) -> Result<BTreeMap<String, PublishedPilotQuestion>> {
    let authorship = QuestionAuthorship::new(vec![QuestionAuthor {
        display_name: QuestionAuthorDisplayName::new(plan.source_project_author)
            .map_err(|_| anyhow::anyhow!("Pilot source author is invalid"))?,
    }])
    .map_err(|_| anyhow::anyhow!("Pilot source authorship is invalid"))?;
    let license = pilot_license(&plan.source_project_content_license)?;
    let existing = services
        .library
        .list_published_question_library_entries(session)
        .await
        .context("reading ordinary published Question provenance")?;
    let mut published = BTreeMap::new();
    for question in plan.questions {
        if let Some(revision) = existing_publication(&existing, &question, &authorship, &license)? {
            published.insert(
                question.slug,
                PublishedPilotQuestion {
                    source_sha256: question.source_sha256,
                    question_revision: revision,
                },
            );
            continue;
        }
        let draft = matching_or_new_draft(
            session,
            workspace,
            &question,
            services.drafts,
            services.objects,
        )
        .await?;
        let checksum = SourceObjectChecksum::parse(question.source_sha256.clone())
            .map_err(|_| anyhow::anyhow!("Pilot source checksum is invalid"))?;
        services
            .publication
            .bind_draft_question_source(
                session,
                DraftQuestionSourceBindingInput {
                    draft_question_uuid: draft.draft_question_uuid,
                    expected_draft_question_edit_number: draft.edit_number,
                    workspace,
                    question_backend: question_backend(question.backend),
                    question_format: question_format(question.backend),
                    webwork_pg_path: question.webwork_pg_path.clone(),
                    draft_imathas_question_backend_binding: None,
                    source_object_reference: SourceObjectReference {
                        object: draft.source_record.id,
                    },
                    source_object_checksum: checksum,
                },
            )
            .await
            .context("binding ordinary Pilot Draft source evidence")?;
        let draft = services
            .drafts
            .load_authoring_draft(session, draft.reference)
            .await
            .context("reloading the bound ordinary Pilot Draft")?;
        let publisher = NewQuestionLineagePublisher::new(
            services.objects.clone(),
            services.publication.clone(),
            services.issuer.clone(),
        );
        let revision = publisher
            .publish(
                session,
                NewQuestionLineagePublicationCommand {
                    draft_question_uuid: draft.draft_question_uuid,
                    expected_draft_question_edit_number: draft.edit_number,
                    workspace,
                    question_authorship: authorship.clone(),
                    question_license: license.clone(),
                    question_revision_reason: QuestionRevisionReason::new(
                        INITIAL_PUBLICATION_REASON.to_string(),
                    )
                    .expect("the fixed initial-publication reason is valid"),
                },
                now(),
            )
            .await
            .context("publishing the ordinary Pilot Question lineage")?;
        published.insert(
            question.slug,
            PublishedPilotQuestion {
                source_sha256: question.source_sha256,
                question_revision: revision,
            },
        );
    }
    Ok(published)
}

fn pilot_license(value: &str) -> Result<QuestionLicense> {
    serde_json::from_value(serde_json::Value::String(value.to_owned()))
        .map_err(|_| anyhow::anyhow!("Pilot source content license is not publishable"))
}

fn existing_publication(
    entries: &[learning_data_access::PublishedQuestionLibraryEntry],
    question: &PublicationSource,
    authorship: &QuestionAuthorship,
    license: &QuestionLicense,
) -> Result<Option<QuestionRevisionReference>> {
    let matching = entries
        .iter()
        .filter(|entry| entry.source_object_checksum.as_str() == question.source_sha256)
        .collect::<Vec<_>>();
    if matching.len() > 1 {
        bail!(
            "ordinary Question provenance has more than one lineage for Pilot source {}",
            question.slug
        );
    }
    let Some(entry) = matching.into_iter().next() else {
        return Ok(None);
    };
    if entry.question_title != question.question_title
        || entry.question_description != question.question_description
        || entry.backend != question_backend(question.backend)
        || entry.source_media_type != question.source_media_type
        || entry.authorship != *authorship
        || entry.question_license != *license
    {
        bail!(
            "ordinary Question provenance conflicts with reviewed Pilot source {}",
            question.slug
        );
    }
    Ok(Some(entry.question_revision.clone()))
}

async fn matching_or_new_draft(
    session: SessionTokenHash,
    workspace: WorkspaceId,
    question: &PublicationSource,
    drafts: &PostgresAuthoringDraftStore,
    objects: &objects::s3::S3ObjectStore,
) -> Result<AuthoringDraft> {
    for summary in drafts
        .list_authoring_drafts(session)
        .await
        .context("listing ordinary Authoring Drafts")?
    {
        if summary.title != question.question_title
            || summary.description != question.question_description
        {
            continue;
        }
        let draft = drafts
            .load_authoring_draft(session, summary.reference)
            .await
            .context("loading matching ordinary Authoring Draft")?;
        if draft.workspace == workspace
            && draft.source_record.sha256.to_string() == question.source_sha256
        {
            return Ok(draft);
        }
    }
    let source_record = objects
        .put(PutObject {
            address: ObjectAddress::WorkspaceQuestionSource {
                workspace,
                object: ObjectId::generate(),
            },
            bytes: question.source_bytes.clone(),
            media_type: question.source_media_type.to_string(),
            created_at: now(),
        })
        .await
        .context("writing ordinary Authoring Draft source bytes")?;
    drafts
        .create_authoring_draft(
            session,
            workspace,
            CreateAuthoringDraftInput {
                draft_question_uuid: DraftQuestionUuid::from_uuid(Uuid::now_v7()),
                source_record,
                webwork_pg_path: question.webwork_pg_path.clone(),
                title: question.question_title.clone(),
                description: question.question_description.clone(),
                language: question.language.clone(),
            },
        )
        .await
        .context("creating ordinary Pilot Authoring Draft")
}

fn question_backend(backend: Backend) -> QuestionBackend {
    match backend {
        Backend::Webwork => QuestionBackend::Webwork,
        Backend::PleQuestionJson => QuestionBackend::Ple,
    }
}

fn question_format(backend: Backend) -> QuestionFormat {
    match backend {
        Backend::Webwork => QuestionFormat::WebworkPg,
        Backend::PleQuestionJson => QuestionFormat::PleQuestionJson,
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
    let value = required_environment(PILOT_PUBLICATION_SESSION_HASH_ENV)?;
    SessionTokenHash::from_hex(&value).map_err(|_| {
        anyhow::anyhow!("{PILOT_PUBLICATION_SESSION_HASH_ENV} must be a session-token hash")
    })
}

fn required_workspace() -> Result<WorkspaceId> {
    let value = required_environment(PILOT_PUBLICATION_WORKSPACE_ENV)?;
    let uuid = Uuid::parse_str(&value)
        .map_err(|_| anyhow::anyhow!("{PILOT_PUBLICATION_WORKSPACE_ENV} must be a UUID"))?;
    Ok(WorkspaceId::from_uuid(uuid))
}

fn required_environment(name: &str) -> Result<String> {
    let value = std::env::var(name).with_context(|| format!("{name} must be set"))?;
    if value.is_empty() || value.contains(['\0', '\n', '\r']) {
        bail!("{name} must not be empty");
    }
    Ok(value)
}

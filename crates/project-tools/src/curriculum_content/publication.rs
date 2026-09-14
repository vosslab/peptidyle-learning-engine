//! Ordinary Authoring Workspace and Blueprint publication for trusted curriculum.

use std::collections::BTreeMap;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{Context, Result, bail, ensure};
use learning_data_access::{
    AuthoringDraft, AuthoringDraftStore, BlueprintCourseStore, CreateAuthoringDraftInput,
    DraftQuestionSourceBindingInput, DraftQuestionSourceBindingStore, DraftQuestionUuid,
    PublishedQuestionLibraryEntry, QuestionLibraryStore, SessionTokenHash,
    StoredBlueprintAssignmentEntry, StoredBlueprintCourse, StoredBlueprintCourseContent,
    postgres::{
        PostgresAuthoringDraftStore, PostgresBlueprintCourseStore,
        PostgresDraftQuestionSourceBindingStore, PostgresQuestionLibraryStore, lazy_pool,
    },
};
use objects::{ObjectAddress, ObjectStore, PutObject};
use question_model::{
    AssignmentActivityRules, AssignmentEntryScoringRule, AssignmentInstructions,
    AssignmentPointValue, BlueprintAssignmentContentInput, BlueprintAssignmentDefaults,
    BlueprintAssignmentEntryInput, BlueprintAvailability, BlueprintRevision,
    CreateBlueprintCourseInput, CreateBlueprintModuleInput, LateWorkRule, ObjectId,
    QuestionAttemptLimit, QuestionAttemptTimeLimit, QuestionAuthor, QuestionAuthorDisplayName,
    QuestionAuthorship, QuestionBackend, QuestionFormat, QuestionLicense,
    QuestionPoolSelectedQuestionOrder, QuestionPoolSelectionRule, QuestionRevisionReason,
    QuestionRevisionReference, QuestionType, RequestChecksum, ReusablePoolInput,
    SourceObjectChecksum, SourceObjectReference, Timestamp, WorkspaceId,
};
use server_core::question_publication::{
    HmacQuestionIdIssuer, NewQuestionLineagePublicationCommand, NewQuestionLineagePublisher,
};
use sha2::{Digest, Sha256};
use uuid::Uuid;

use super::{
    Course, CurriculumQuestionType, Manifest, Row, Topic, read_pg_source, repository_root,
};

const SESSION_HASH_ENV: &str = "PLE_CURRICULUM_PUBLICATION_SESSION_TOKEN_HASH";
const WORKSPACE_ENV: &str = "PLE_CURRICULUM_PUBLICATION_WORKSPACE_ID";
const INITIAL_PUBLICATION_REASON: &str = "Initial publication from trusted curriculum import";

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
    let database_url = required_environment("DATABASE_URL")?;
    let pool =
        lazy_pool(&database_url).context("curriculum publication database URL is invalid")?;
    let drafts = PostgresAuthoringDraftStore::new(pool.clone());
    let bindings = PostgresDraftQuestionSourceBindingStore::new(pool.clone());
    let library = PostgresQuestionLibraryStore::new(pool.clone());
    let blueprints = PostgresBlueprintCourseStore::new(pool);
    let objects = server_core::composition::question_library_object_store_from_env()
        .await
        .context("configuring ordinary Question source object store")?;
    let issuer = server_core::composition::question_id_issuer_from_env()
        .context("loading Question ID publication capability")?;
    let authorship = QuestionAuthorship::new(vec![QuestionAuthor {
        display_name: QuestionAuthorDisplayName::new(manifest.course.author.clone())
            .map_err(|_| anyhow::anyhow!("curriculum source author is invalid"))?,
    }])
    .map_err(|_| anyhow::anyhow!("curriculum source authorship is invalid"))?;
    let license = serde_json::from_value::<QuestionLicense>(serde_json::Value::String(
        manifest.course.content_license.clone(),
    ))
    .map_err(|_| anyhow::anyhow!("curriculum content license is not publishable"))?;
    if let Some(retained) = matching_blueprint(session, &manifest.course, &blueprints).await? {
        let published = retained_published_references(
            session,
            &manifest,
            &retained,
            &library,
            &authorship,
            &license,
        )
        .await?;
        let input = blueprint_input(&manifest, &published)?;
        validate_loaded_content(&retained.content, &input, Some(&manifest), &published)?;
        return Receipt::new(
            retained.reference.to_string(),
            retained.current_revision.value(),
            &manifest,
            &published,
        );
    }
    let existing = library
        .list_published_question_library_entries(session)
        .await
        .context("reading ordinary published Question provenance")?;
    let existing = published_index(existing);
    let draft_index = draft_index(session, &drafts).await?;
    let mut published = BTreeMap::new();
    for topic in &manifest.topics {
        for bank in &topic.banks {
            for row in &bank.rows {
                let key = source_key(topic, bank, row);
                let revision = if let Some(revision) =
                    existing_publication(&existing, row, &authorship, &license)?
                {
                    revision
                } else {
                    publish_row(
                        session,
                        workspace,
                        row,
                        &authorship,
                        &license,
                        &draft_index,
                        &drafts,
                        &bindings,
                        &objects,
                        &issuer,
                        root,
                    )
                    .await?
                };
                ensure!(
                    published.insert(key.clone(), revision).is_none(),
                    "curriculum source identity duplicated: {key}"
                );
            }
        }
    }
    let input = blueprint_input(&manifest, &published)?;
    let (reference, revision) =
        create_blueprint(session, &input, &manifest, &published, &blueprints).await?;
    Receipt::new(
        reference.to_string(),
        revision.value(),
        &manifest,
        &published,
    )
}

async fn matching_blueprint(
    session: SessionTokenHash,
    course: &Course,
    store: &PostgresBlueprintCourseStore,
) -> Result<Option<StoredBlueprintCourse>> {
    let matching = store
        .list_blueprint_courses(session)
        .await
        .context("listing ordinary Blueprint Courses before curriculum publication")?
        .into_iter()
        .filter(|summary| {
            summary.short_name == course.short_name && summary.long_name == course.long_name
        })
        .collect::<Vec<_>>();
    if matching.len() > 1 {
        bail!("more than one ordinary Blueprint Course has the curriculum lineage names");
    }
    let Some(summary) = matching.first() else {
        return Ok(None);
    };
    let loaded = store
        .load_blueprint_course(session, summary.reference)
        .await
        .context("reloading retained curriculum Blueprint Course before publication")?;
    ensure!(
        loaded.availability == BlueprintAvailability::Available,
        "retained curriculum Blueprint Course is not Available"
    );
    Ok(Some(loaded))
}

async fn retained_published_references(
    session: SessionTokenHash,
    manifest: &Manifest,
    retained: &StoredBlueprintCourse,
    library: &PostgresQuestionLibraryStore,
    authorship: &QuestionAuthorship,
    license: &QuestionLicense,
) -> Result<BTreeMap<String, QuestionRevisionReference>> {
    let Some(module) = retained.content.modules.first() else {
        bail!("retained curriculum Blueprint has no module");
    };
    ensure!(
        retained.content.modules.len() == 1 && module.assignments.len() == manifest.topics.len(),
        "retained curriculum Blueprint topic mapping conflicts with the manifest"
    );
    let mut published = BTreeMap::new();
    for (assignment, topic) in module.assignments.iter().zip(&manifest.topics) {
        ensure!(
            assignment.content.entries.len() == topic.banks.len(),
            "retained curriculum Blueprint Pool mapping conflicts with topic {}",
            topic.slug
        );
        for (entry, bank) in assignment.content.entries.iter().zip(&topic.banks) {
            let StoredBlueprintAssignmentEntry::Pool {
                question_revisions, ..
            } = entry
            else {
                bail!("retained curriculum Blueprint entries must remain Question Pools");
            };
            ensure!(
                question_revisions.len() == bank.rows.len(),
                "retained curriculum Pool membership conflicts with bank {}",
                bank.slug
            );
            for (reference, row) in question_revisions.iter().zip(&bank.rows) {
                let entry = library
                    .load_published_question_revision_library_entry(session, reference)
                    .await
                    .context("resolving retained curriculum Question Revision")?;
                ensure_entry_compatible(&entry, row, authorship, license)?;
                let key = source_key(topic, bank, row);
                ensure!(
                    published.insert(key.clone(), reference.clone()).is_none(),
                    "retained curriculum source identity duplicated: {key}"
                );
            }
        }
    }
    Ok(published)
}

type DraftIndex = BTreeMap<(String, String), Vec<question_model::DraftQuestionReference>>;
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

async fn draft_index(
    session: SessionTokenHash,
    drafts: &PostgresAuthoringDraftStore,
) -> Result<DraftIndex> {
    let mut index = DraftIndex::new();
    for summary in drafts
        .list_authoring_drafts(session)
        .await
        .context("listing ordinary Authoring Drafts")?
    {
        index
            .entry((summary.title, summary.description))
            .or_default()
            .push(summary.reference);
    }
    Ok(index)
}

#[allow(clippy::too_many_arguments)]
async fn publish_row(
    session: SessionTokenHash,
    workspace: WorkspaceId,
    row: &Row,
    authorship: &QuestionAuthorship,
    license: &QuestionLicense,
    draft_index: &DraftIndex,
    drafts: &PostgresAuthoringDraftStore,
    bindings: &PostgresDraftQuestionSourceBindingStore,
    objects: &objects::s3::S3ObjectStore,
    issuer: &HmacQuestionIdIssuer,
    root: &Path,
) -> Result<QuestionRevisionReference> {
    let draft =
        matching_or_new_draft(session, workspace, row, draft_index, drafts, objects, root).await?;
    let checksum = SourceObjectChecksum::parse(row.pg_sha256.clone())
        .map_err(|_| anyhow::anyhow!("curriculum PG checksum is invalid for {}", row.row_id))?;
    bindings
        .bind_draft_question_source(
            session,
            DraftQuestionSourceBindingInput {
                draft_question_uuid: draft.draft_question_uuid,
                expected_draft_question_edit_number: draft.edit_number,
                workspace,
                question_backend: QuestionBackend::Webwork,
                question_format: QuestionFormat::WebworkPg,
                question_type: question_type(row.question_type),
                webwork_pg_path: Some(row.webwork_pg_path.clone()),
                draft_imathas_question_backend_binding: None,
                source_object_reference: SourceObjectReference {
                    object: draft.source_record.id,
                },
                source_object_checksum: checksum,
            },
        )
        .await
        .context("binding ordinary curriculum Draft source evidence")?;
    let bound = drafts
        .load_authoring_draft(session, draft.reference)
        .await
        .context("reloading bound ordinary curriculum Draft")?;
    NewQuestionLineagePublisher::new(objects.clone(), bindings.clone(), issuer.clone())
        .publish(
            session,
            NewQuestionLineagePublicationCommand {
                draft_question_uuid: bound.draft_question_uuid,
                expected_draft_question_edit_number: bound.edit_number,
                workspace,
                question_authorship: authorship.clone(),
                question_license: license.clone(),
                question_revision_reason: QuestionRevisionReason::new(
                    INITIAL_PUBLICATION_REASON.to_owned(),
                )
                .expect("fixed publication reason is valid"),
            },
            now(),
        )
        .await
        .context("publishing ordinary curriculum Question lineage")
}

async fn matching_or_new_draft(
    session: SessionTokenHash,
    workspace: WorkspaceId,
    row: &Row,
    index: &DraftIndex,
    drafts: &PostgresAuthoringDraftStore,
    objects: &objects::s3::S3ObjectStore,
    root: &Path,
) -> Result<AuthoringDraft> {
    let key = (row.question_title.clone(), row.question_description.clone());
    if let Some(references) = index.get(&key) {
        for reference in references {
            let draft = drafts
                .load_authoring_draft(session, *reference)
                .await
                .context("loading matching ordinary curriculum Draft")?;
            if draft.workspace == workspace
                && draft.source_record.sha256.to_string() == row.pg_sha256
            {
                return Ok(draft);
            }
        }
    }
    let source_record = objects
        .put(PutObject {
            address: ObjectAddress::WorkspaceQuestionSource {
                workspace,
                object: ObjectId::generate(),
            },
            bytes: read_pg_source(root, row)?,
            media_type: "text/x-wework-pg".to_owned(),
            created_at: now(),
        })
        .await
        .context("writing ordinary curriculum Draft source bytes")?;
    drafts
        .create_authoring_draft(
            session,
            workspace,
            CreateAuthoringDraftInput {
                draft_question_uuid: DraftQuestionUuid::from_uuid(Uuid::now_v7()),
                source_record,
                webwork_pg_path: Some(row.webwork_pg_path.clone()),
                question_type: question_type(row.question_type),
                title: row.question_title.clone(),
                description: row.question_description.clone(),
                language: "en".to_owned(),
            },
        )
        .await
        .context("creating ordinary curriculum Authoring Draft")
}

fn existing_publication(
    entries: &PublishedIndex,
    row: &Row,
    authorship: &QuestionAuthorship,
    license: &QuestionLicense,
) -> Result<Option<QuestionRevisionReference>> {
    let matches = entries
        .get(&row.pg_sha256)
        .map(Vec::as_slice)
        .unwrap_or_default();
    if matches.len() > 1 {
        bail!(
            "ordinary Question provenance has more than one lineage for curriculum row {}",
            row.row_id
        );
    }
    let Some(entry) = matches.first() else {
        return Ok(None);
    };
    ensure_entry_compatible(entry, row, authorship, license)?;
    Ok(Some(entry.question_revision.clone()))
}

fn ensure_entry_compatible(
    entry: &PublishedQuestionLibraryEntry,
    row: &Row,
    authorship: &QuestionAuthorship,
    license: &QuestionLicense,
) -> Result<()> {
    if entry.question_title != row.question_title
        || entry.question_description != row.question_description
        || entry.backend != QuestionBackend::Webwork
        || entry.question_type != question_type(row.question_type)
        || entry.source_media_type != "text/x-wework-pg"
        || entry.source_object_checksum.as_str() != row.pg_sha256
        || entry.authorship != *authorship
        || entry.question_license != *license
    {
        bail!(
            "ordinary Question provenance conflicts with curriculum row {}",
            row.row_id
        );
    }
    // The Question Library entry does not expose WeBWorK's immutable source
    // path. The retained Blueprint path is verified through the exact source
    // revision above; new publication binds this manifest path before publish.
    Ok(())
}

fn blueprint_input(
    manifest: &Manifest,
    published: &BTreeMap<String, QuestionRevisionReference>,
) -> Result<CreateBlueprintCourseInput> {
    let mut assignments = Vec::with_capacity(manifest.topics.len());
    for topic in &manifest.topics {
        let mut entries = Vec::with_capacity(topic.banks.len());
        for bank in &topic.banks {
            let items = bank
                .rows
                .iter()
                .map(|row| {
                    published
                        .get(&source_key(topic, bank, row))
                        .map(|reference| reference.question_id.clone())
                        .with_context(|| {
                            format!("published curriculum source is missing for {}", row.row_id)
                        })
                })
                .collect::<Result<Vec<_>>>()?;
            entries.push(BlueprintAssignmentEntryInput::Pool(ReusablePoolInput {
                items,
                selection_count: bank.selection_count,
                points_per_item: AssignmentPointValue::from_whole(1),
                scoring_rule: AssignmentEntryScoringRule::Normal,
                selection_rule: QuestionPoolSelectionRule {
                    selected_question_order: QuestionPoolSelectedQuestionOrder::QuestionPoolOrder,
                },
                question_attempt_limit: QuestionAttemptLimit { max_attempts: None },
                question_attempt_time_limit: QuestionAttemptTimeLimit::Unlimited,
            }));
        }
        assignments.push(BlueprintAssignmentContentInput {
            title: topic.title.clone(),
            instructions: AssignmentInstructions::try_new(topic.instructions.clone())
                .map_err(|_| anyhow::anyhow!("curriculum topic instructions are invalid"))?,
            entries,
            defaults: BlueprintAssignmentDefaults {
                assignment_attempt_time_limit_seconds: None,
                attempt_limit: None,
                late_work_rule: LateWorkRule::Reject,
                activity_rules: AssignmentActivityRules::default(),
                student_feedback_release_rule: Default::default(),
            },
        });
    }
    let input = CreateBlueprintCourseInput {
        short_name: manifest.course.short_name.clone(),
        long_name: manifest.course.long_name.clone(),
        modules: vec![CreateBlueprintModuleInput {
            label: manifest.course.module_label.clone(),
            assignments,
        }],
    };
    input
        .validate()
        .map_err(|error| anyhow::anyhow!("curriculum Blueprint input is invalid: {error}"))?;
    Ok(input)
}

async fn create_blueprint(
    session: SessionTokenHash,
    input: &CreateBlueprintCourseInput,
    manifest: &Manifest,
    published: &BTreeMap<String, QuestionRevisionReference>,
    store: &PostgresBlueprintCourseStore,
) -> Result<(question_model::BlueprintCourseReference, BlueprintRevision)> {
    let checksum = request_checksum(input)?;
    let receipt = store
        .create_blueprint_course(session, checksum, input.clone())
        .await
        .context("creating ordinary curriculum Blueprint Course")?;
    ensure!(
        receipt.blueprint_revision.revision == BlueprintRevision::INITIAL,
        "curriculum Blueprint creation did not return Revision 1"
    );
    let loaded = store
        .load_blueprint_course(session, receipt.blueprint_revision.reference)
        .await
        .context("reloading ordinary curriculum Blueprint Course")?;
    ensure!(
        loaded.availability == BlueprintAvailability::Available,
        "new curriculum Blueprint Course is not Available"
    );
    validate_loaded_content(&loaded.content, input, Some(manifest), published)?;
    Ok((loaded.reference, loaded.current_revision))
}

fn validate_loaded_content(
    actual: &StoredBlueprintCourseContent,
    expected: &CreateBlueprintCourseInput,
    manifest: Option<&Manifest>,
    expected_revisions: &BTreeMap<String, QuestionRevisionReference>,
) -> Result<()> {
    ensure!(
        actual.modules.len() == 1 && expected.modules.len() == 1,
        "curriculum Blueprint must retain one module"
    );
    let actual = &actual.modules[0];
    let expected = &expected.modules[0];
    ensure!(
        actual.label == expected.label && actual.assignments.len() == expected.assignments.len(),
        "curriculum Blueprint topic ordering differs"
    );
    for (assignment_index, (actual, expected)) in actual
        .assignments
        .iter()
        .zip(&expected.assignments)
        .enumerate()
    {
        ensure!(
            actual.content.title == expected.title
                && actual.content.instructions == expected.instructions
                && actual.content.defaults == expected.defaults
                && actual.content.entries.len() == expected.entries.len(),
            "curriculum Blueprint Assignment content differs"
        );
        for (entry_index, (actual, expected)) in actual
            .content
            .entries
            .iter()
            .zip(&expected.entries)
            .enumerate()
        {
            let (
                StoredBlueprintAssignmentEntry::Pool {
                    question_revisions,
                    selection_count,
                    points_per_item,
                    scoring_rule,
                    selection_rule,
                    question_attempt_limit,
                    question_attempt_time_limit,
                },
                BlueprintAssignmentEntryInput::Pool(expected),
            ) = (actual, expected)
            else {
                bail!("curriculum Blueprint entries must remain Question Pools");
            };
            ensure!(
                question_revisions
                    .iter()
                    .map(|reference| &reference.question_id)
                    .eq(expected.items.iter())
                    && selection_count == &expected.selection_count
                    && points_per_item == &expected.points_per_item
                    && scoring_rule == &expected.scoring_rule
                    && selection_rule == &expected.selection_rule
                    && question_attempt_limit == &expected.question_attempt_limit
                    && question_attempt_time_limit == &expected.question_attempt_time_limit,
                "curriculum Blueprint Pool pins or policy differs"
            );
            if !expected_revisions.is_empty() {
                let manifest =
                    manifest.context("curriculum manifest is required for exact pin checks")?;
                let topic = manifest.topics.get(assignment_index).with_context(
                    || "curriculum Blueprint Assignment has no manifest topic for exact pin check",
                )?;
                let bank = topic.banks.get(entry_index).with_context(
                    || "curriculum Blueprint Pool has no manifest bank for exact pin check",
                )?;
                let expected_pins = bank
                    .rows
                    .iter()
                    .map(|row| {
                        expected_revisions
                            .get(&source_key(topic, bank, row))
                            .cloned()
                            .with_context(|| {
                                format!(
                                    "curriculum source is missing an exact Question Revision for {}",
                                    row.row_id
                                )
                            })
                    })
                    .collect::<Result<Vec<_>>>()?;
                ensure_exact_pool_pins(question_revisions, &expected_pins)?;
            }
        }
    }
    Ok(())
}

fn ensure_exact_pool_pins(
    actual: &[QuestionRevisionReference],
    expected: &[QuestionRevisionReference],
) -> Result<()> {
    ensure!(
        actual == expected,
        "curriculum Blueprint Pool exact Question Revision pins differ"
    );
    Ok(())
}

fn source_key(topic: &Topic, bank: &super::Bank, row: &Row) -> String {
    format!("{}/{}/{}", topic.slug, bank.slug, row.row_id)
}
fn question_type(value: CurriculumQuestionType) -> QuestionType {
    match value {
        CurriculumQuestionType::MultipleChoice => QuestionType::MultipleChoice,
        CurriculumQuestionType::MultipleAnswer => QuestionType::MultipleAnswer,
        CurriculumQuestionType::FillInBlank => QuestionType::FillInBlank,
        CurriculumQuestionType::MultipleFillInBlank => QuestionType::MultipleFillInBlank,
        CurriculumQuestionType::Numeric => QuestionType::Numeric,
        CurriculumQuestionType::Matching => QuestionType::Matching,
    }
}
fn now() -> Timestamp {
    let milliseconds = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .unwrap_or_default();
    Timestamp::from_unix_millis(i64::try_from(milliseconds).unwrap_or(i64::MAX))
}
fn request_checksum(input: &CreateBlueprintCourseInput) -> Result<RequestChecksum> {
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

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Receipt {
    blueprint_reference: String,
    blueprint_revision: u64,
    source_repository: String,
    source_revision: String,
    topics: Vec<ReceiptTopic>,
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct ReceiptTopic {
    source_key: String,
    title: String,
    banks: Vec<ReceiptBank>,
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct ReceiptBank {
    source_key: String,
    title: String,
    source_path: String,
    source_checksum: String,
    selection_count: u32,
    backend: QuestionBackend,
    rows: Vec<ReceiptRow>,
    pool_question_revisions: Vec<QuestionRevisionReference>,
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct ReceiptRow {
    source_key: String,
    row_id: String,
    title: String,
    source_path: String,
    source_checksum: String,
    backend: QuestionBackend,
    question_type: QuestionType,
    webwork_pg_path: String,
    pool_position: usize,
    question_revision: QuestionRevisionReference,
}
impl Receipt {
    pub(crate) fn installation_summary(&self) -> String {
        let bank_count = self
            .topics
            .iter()
            .map(|topic| topic.banks.len())
            .sum::<usize>();
        let row_count = self
            .topics
            .iter()
            .flat_map(|topic| &topic.banks)
            .map(|bank| bank.rows.len())
            .sum::<usize>();
        format!(
            "Genetics Blueprint {} revision {}: {} topics, {} banks, {} Questions",
            self.blueprint_reference,
            self.blueprint_revision,
            self.topics.len(),
            bank_count,
            row_count
        )
    }

    fn new(
        reference: String,
        revision: u64,
        manifest: &Manifest,
        published: &BTreeMap<String, QuestionRevisionReference>,
    ) -> Result<Self> {
        let topics = manifest
            .topics
            .iter()
            .map(|topic| -> Result<ReceiptTopic> {
                Ok(ReceiptTopic {
                source_key: topic.slug.clone(),
                title: topic.title.clone(),
                banks: topic
                    .banks
                    .iter()
                    .map(|bank| -> Result<ReceiptBank> {
                        let rows = bank
                            .rows
                            .iter()
                            .enumerate()
                            .map(|(pool_position, row)| -> Result<ReceiptRow> {
                                let source_key = source_key(topic, bank, row);
                                let question_revision = published
                                    .get(&source_key)
                                    .cloned()
                                    .with_context(|| {
                                        format!(
                                            "curriculum receipt is missing a published revision for {}",
                                            row.row_id
                                        )
                                    })?;
                                Ok(ReceiptRow {
                                    source_key,
                                    row_id: row.row_id.clone(),
                                    title: row.question_title.clone(),
                                    source_path: row.pg_source.display().to_string(),
                                    source_checksum: row.pg_sha256.clone(),
                                    backend: QuestionBackend::Webwork,
                                    question_type: question_type(row.question_type),
                                    webwork_pg_path: row.webwork_pg_path.clone(),
                                    pool_position,
                                    question_revision,
                                })
                            })
                            .collect::<Result<Vec<_>>>()?;
                        Ok(ReceiptBank {
                            source_key: format!("{}/{}", topic.slug, bank.slug),
                            title: bank.title.clone(),
                            source_path: bank.source.display().to_string(),
                            source_checksum: bank.source_sha256.clone(),
                            selection_count: bank.selection_count,
                            backend: QuestionBackend::Webwork,
                            pool_question_revisions: rows
                                .iter()
                                .map(|row| row.question_revision.clone())
                                .collect(),
                            rows,
                        })
                    })
                    .collect::<Result<Vec<_>>>()?,
                })
            })
            .collect::<Result<Vec<_>>>()?;
        Ok(Self {
            blueprint_reference: reference,
            blueprint_revision: revision,
            source_repository: manifest.course.source_repository.clone(),
            source_revision: manifest.course.source_revision.clone(),
            topics,
        })
    }
}
impl std::fmt::Display for Receipt {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        serde_json::to_string(self)
            .map_err(|_| std::fmt::Error)?
            .fmt(f)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use question_model::{QuestionId, QuestionRevisionNumber};

    fn reference(question_id: &str, revision_number: u32) -> QuestionRevisionReference {
        QuestionRevisionReference {
            question_id: question_id
                .parse::<QuestionId>()
                .expect("valid Question ID"),
            revision_number: QuestionRevisionNumber::new(revision_number)
                .expect("positive Question Revision Number"),
        }
    }

    fn receipt_manifest() -> Manifest {
        Manifest {
            version: 1,
            course: Course {
                short_name: "Genetics".to_owned(),
                long_name: "Genetics Blueprint".to_owned(),
                module_label: "Genetics".to_owned(),
                source_repository: "owner/repository".to_owned(),
                source_revision: "abc123".to_owned(),
                author: "Author".to_owned(),
                content_license: "CC-BY-4.0".to_owned(),
            },
            topics: vec![Topic {
                slug: "topic".to_owned(),
                title: "Topic title".to_owned(),
                instructions: "Instructions".to_owned(),
                banks: vec![super::super::Bank {
                    slug: "bank".to_owned(),
                    title: "Bank title".to_owned(),
                    source: "bank.txt".into(),
                    source_sha256: "b".repeat(64),
                    selection_count: 1,
                    rows: vec![
                        Row {
                            row_id: "first".to_owned(),
                            question_title: "First".to_owned(),
                            question_description: "First description".to_owned(),
                            question_type: CurriculumQuestionType::MultipleChoice,
                            pg_source: "first.pg".into(),
                            pg_sha256: "1".repeat(64),
                            webwork_pg_path: "topic/first.pg".to_owned(),
                        },
                        Row {
                            row_id: "second".to_owned(),
                            question_title: "Second".to_owned(),
                            question_description: "Second description".to_owned(),
                            question_type: CurriculumQuestionType::MultipleChoice,
                            pg_source: "second.pg".into(),
                            pg_sha256: "2".repeat(64),
                            webwork_pg_path: "topic/second.pg".to_owned(),
                        },
                    ],
                }],
            }],
        }
    }

    #[test]
    fn receipt_retains_source_provenance_and_exact_pool_pin_order() {
        let manifest = receipt_manifest();
        let first = reference("7K3-M9QX", 1);
        let second = reference("8K3-M9QX", 2);
        let published = BTreeMap::from([
            ("topic/bank/first".to_owned(), first.clone()),
            ("topic/bank/second".to_owned(), second.clone()),
        ]);

        let receipt = Receipt::new("BP-TEST".to_owned(), 1, &manifest, &published)
            .expect("receipt from complete source map");
        let bank = &receipt.topics[0].banks[0];
        assert_eq!(bank.pool_question_revisions, vec![first, second]);
        assert_eq!(bank.source_path, "bank.txt");
        assert_eq!(bank.selection_count, 1);
        assert_eq!(bank.rows[0].row_id, "first");
        assert_eq!(bank.rows[0].source_path, "first.pg");
        assert_eq!(bank.rows[0].webwork_pg_path, "topic/first.pg");
        assert_eq!(bank.rows[1].pool_position, 1);
        assert_eq!(receipt.source_repository, "owner/repository");
        assert_eq!(receipt.source_revision, "abc123");
    }

    #[test]
    fn exact_pool_pins_reject_a_newer_revision_of_the_same_question() {
        let revision_one = reference("7K3-M9QX", 1);
        let revision_two = reference("7K3-M9QX", 2);

        assert!(
            ensure_exact_pool_pins(
                std::slice::from_ref(&revision_one),
                std::slice::from_ref(&revision_one)
            )
            .is_ok()
        );
        assert!(ensure_exact_pool_pins(&[revision_two], &[revision_one]).is_err());
    }

    #[test]
    fn retained_blueprint_rejects_a_newer_revision_of_the_same_question() {
        let manifest = receipt_manifest();
        let first = reference("7K3-M9QX", 1);
        let second = reference("8K3-M9QX", 2);
        let published = BTreeMap::from([
            ("topic/bank/first".to_owned(), first.clone()),
            ("topic/bank/second".to_owned(), second.clone()),
        ]);
        let input = blueprint_input(&manifest, &published).expect("valid Blueprint input");
        let pins = BTreeMap::from([
            (first.question_id.clone(), first.clone()),
            (second.question_id.clone(), second.clone()),
        ]);
        let mut actual = StoredBlueprintCourseContent::from_create(input.clone(), &pins)
            .expect("stored Blueprint content");
        let StoredBlueprintAssignmentEntry::Pool {
            question_revisions, ..
        } = &mut actual.modules[0].assignments[0].content.entries[0]
        else {
            panic!("fixture retains one Question Pool");
        };
        question_revisions[0] = reference("7K3-M9QX", 2);

        assert!(validate_loaded_content(&actual, &input, Some(&manifest), &published).is_err());
    }
}

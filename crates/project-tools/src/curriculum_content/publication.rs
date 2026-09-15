//! Ordinary Authoring Workspace and Blueprint publication for trusted curriculum.

use std::collections::BTreeMap;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{Context, Result, bail, ensure};
use learning_data_access::{
    AuthoringDraft, AuthoringDraftStore, BlueprintCourseStore, CreateAuthoringDraftInput,
    DraftQuestionSourceBindingInput, DraftQuestionSourceBindingStore, DraftQuestionUuid,
    PublishedQuestionLibraryEntry, QuestionLibraryStore, SessionTokenHash,
    StoredBlueprintAssessmentEntry, StoredBlueprintCourse, StoredBlueprintCourseContent,
    postgres::{
        PostgresAuthoringDraftStore, PostgresBlueprintCourseStore,
        PostgresDraftQuestionSourceBindingStore, PostgresQuestionLibraryStore, lazy_pool,
    },
};
use objects::{ObjectAddress, ObjectStore, PutObject};
use question_model::{
    AssessmentActivityRules, AssessmentEntryScoringRule, AssessmentInstructions,
    AssessmentPointValue, BlueprintAssessmentContentInput, BlueprintAssessmentDefaults,
    BlueprintAssessmentEditChoice, BlueprintAssessmentEntryInput,
    BlueprintAssessmentReplacementInput, BlueprintAvailability, BlueprintModuleEditChoice,
    BlueprintModuleReplacementInput, BlueprintRevision, CreateBlueprintCourseInput,
    CreateBlueprintModuleInput, LateWorkRule, ObjectId, QuestionAttemptLimit,
    QuestionAttemptTimeLimit, QuestionAuthor, QuestionAuthorDisplayName, QuestionAuthorship,
    QuestionBackend, QuestionFormat, QuestionLicense, QuestionPoolSelectedQuestionOrder,
    QuestionPoolSelectionRule, QuestionRevisionReason, QuestionRevisionReference, QuestionType,
    ReplaceBlueprintCourseContentInput, RequestChecksum, ReusableFixedQuestionInput,
    ReusablePoolInput, SourceObjectChecksum, SourceObjectReference, Timestamp, WorkspaceId,
};
use server_core::question_publication::{
    HmacQuestionIdIssuer, NewQuestionLineagePublicationCommand, NewQuestionLineagePublisher,
};
use sha2::{Digest, Sha256};
use uuid::Uuid;

use super::{
    Course, CurriculumQuestionType, Manifest, ParameterizedSource, Row, Topic, read_pg_source,
    repository_root,
};

mod receipt;

pub(crate) use receipt::Receipt;

const SESSION_HASH_ENV: &str = "PLE_CURRICULUM_PUBLICATION_SESSION_TOKEN_HASH";
const WORKSPACE_ENV: &str = "PLE_CURRICULUM_PUBLICATION_WORKSPACE_ID";
const INITIAL_PUBLICATION_REASON: &str = "Initial publication from trusted curriculum import";

type ReplacementRevisions = BTreeMap<(String, String), QuestionRevisionReference>;

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
    super::validate(&manifest, root)?;
    let replacement_revisions =
        publish_accepted_replacements(session, workspace, &manifest, root).await?;
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
        let input = blueprint_input(&manifest, &published, &replacement_revisions)?;
        let has_pending_replacement = validate_loaded_content(
            &retained.content,
            &input,
            Some(&manifest),
            &published,
            &replacement_revisions,
            true,
        )?;
        if has_pending_replacement {
            let replacement = replacement_input(&retained.content, &input)?;
            let checksum = request_checksum(&replacement)?;
            blueprints
                .save_blueprint_course(
                    session,
                    retained.reference,
                    retained.current_revision,
                    checksum,
                    replacement,
                )
                .await
                .context("CAS replacing accepted static curriculum expansion")?;
            let updated = blueprints
                .load_blueprint_course(session, retained.reference)
                .await
                .context("reloading C840-replaced curriculum Blueprint Course")?;
            validate_loaded_content(
                &updated.content,
                &input,
                Some(&manifest),
                &published,
                &replacement_revisions,
                false,
            )?;
            return Receipt::new(
                updated.reference.to_string(),
                updated.current_revision.value(),
                &manifest,
                &published,
                &replacement_revisions,
            );
        }
        return Receipt::new(
            retained.reference.to_string(),
            retained.current_revision.value(),
            &manifest,
            &published,
            &replacement_revisions,
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
            if replacement_revisions.contains_key(&(topic.slug.clone(), bank.slug.clone())) {
                continue;
            }
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
    let input = blueprint_input(&manifest, &published, &replacement_revisions)?;
    let (reference, revision) = create_blueprint(
        session,
        &input,
        &manifest,
        &published,
        &replacement_revisions,
        &blueprints,
    )
    .await?;
    Receipt::new(
        reference.to_string(),
        revision.value(),
        &manifest,
        &published,
        &replacement_revisions,
    )
}

/// Publishes only C839-accepted sources that have an explicit catalog target.
/// Other canonical sources remain independently publishable C838 candidates;
/// they cannot silently alter the ordinary Genetics Blueprint.
async fn publish_accepted_replacements(
    session: SessionTokenHash,
    workspace: WorkspaceId,
    manifest: &Manifest,
    root: &Path,
) -> Result<ReplacementRevisions> {
    let mut replacement_manifest = manifest.clone();
    replacement_manifest
        .parameterized_sources
        .retain(|source| source.replaces_static_bank_slug.is_some());
    if replacement_manifest.parameterized_sources.is_empty() {
        return Ok(ReplacementRevisions::new());
    }
    let receipt = super::parameterized_publication::publish_with_context(
        session,
        workspace,
        replacement_manifest.clone(),
        root,
    )
    .await
    .context("publishing accepted canonical algorithmic replacements")?;
    let revisions = receipt.question_revisions();
    replacement_manifest
        .parameterized_sources
        .iter()
        .map(|source| {
            let bank = source
                .replaces_static_bank_slug
                .as_ref()
                .expect("replacement manifest contains explicit targets");
            let revision = revisions.get(&source.source_id).cloned().with_context(|| {
                format!(
                    "accepted canonical publication is missing source {}",
                    source.source_id
                )
            })?;
            Ok(((source.topic_slug.clone(), bank.clone()), revision))
        })
        .collect()
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
        loaded.availability == BlueprintAvailability::Private,
        "retained curriculum Blueprint Course is not Private"
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
        retained.content.modules.len() == 1 && module.assessments.len() == manifest.topics.len(),
        "retained curriculum Blueprint topic mapping conflicts with the manifest"
    );
    let mut published = BTreeMap::new();
    for (assessment, topic) in module.assessments.iter().zip(&manifest.topics) {
        ensure!(
            assessment.content.entries.len() == topic.banks.len(),
            "retained curriculum Blueprint Pool mapping conflicts with topic {}",
            topic.slug
        );
        for (entry, bank) in assessment.content.entries.iter().zip(&topic.banks) {
            if replacement_source(manifest, topic, bank).is_some()
                && matches!(entry, StoredBlueprintAssessmentEntry::Fixed { .. })
            {
                // The later exact Fixed-entry validation proves this is the
                // already-converted canonical pin. Static provenance is only
                // needed while a reviewed Pool is still eligible for CAS.
                continue;
            }
            let StoredBlueprintAssessmentEntry::Pool {
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
                question_format: QuestionFormat::WebworkPg,
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
    replacements: &ReplacementRevisions,
) -> Result<CreateBlueprintCourseInput> {
    let mut assessments = Vec::with_capacity(manifest.topics.len());
    for topic in &manifest.topics {
        let mut entries = Vec::with_capacity(topic.banks.len());
        for bank in &topic.banks {
            if replacement_source(manifest, topic, bank).is_some() {
                let revision = replacements
                    .get(&(topic.slug.clone(), bank.slug.clone()))
                    .with_context(|| {
                        format!(
                            "accepted canonical replacement is missing for {}/{}",
                            topic.slug, bank.slug
                        )
                    })?;
                entries.push(BlueprintAssessmentEntryInput::Fixed(
                    ReusableFixedQuestionInput {
                        question_id: revision.question_id.clone(),
                        points_possible: AssessmentPointValue::from_whole(1),
                        scoring_rule: AssessmentEntryScoringRule::Normal,
                        question_attempt_limit: QuestionAttemptLimit { max_attempts: None },
                        question_attempt_time_limit: QuestionAttemptTimeLimit::Unlimited,
                    },
                ));
                continue;
            }
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
            entries.push(BlueprintAssessmentEntryInput::Pool(ReusablePoolInput {
                items,
                selection_count: bank.selection_count,
                points_per_item: AssessmentPointValue::from_whole(1),
                scoring_rule: AssessmentEntryScoringRule::Normal,
                selection_rule: QuestionPoolSelectionRule {
                    selected_question_order: QuestionPoolSelectedQuestionOrder::QuestionPoolOrder,
                },
                question_attempt_limit: QuestionAttemptLimit { max_attempts: None },
                question_attempt_time_limit: QuestionAttemptTimeLimit::Unlimited,
            }));
        }
        assessments.push(BlueprintAssessmentContentInput {
            title: topic.title.clone(),
            instructions: AssessmentInstructions::try_new(topic.instructions.clone())
                .map_err(|_| anyhow::anyhow!("curriculum topic instructions are invalid"))?,
            entries,
            defaults: BlueprintAssessmentDefaults {
                assessment_attempt_time_limit_seconds: None,
                attempt_limit: None,
                late_work_rule: LateWorkRule::Reject,
                activity_rules: AssessmentActivityRules::default(),
                student_feedback_release_rule: Default::default(),
            },
        });
    }
    let input = CreateBlueprintCourseInput {
        short_name: manifest.course.short_name.clone(),
        long_name: manifest.course.long_name.clone(),
        modules: vec![CreateBlueprintModuleInput {
            label: manifest.course.module_label.clone(),
            assessments,
        }],
    };
    input
        .validate()
        .map_err(|error| anyhow::anyhow!("curriculum Blueprint input is invalid: {error}"))?;
    Ok(input)
}

fn replacement_source<'a>(
    manifest: &'a Manifest,
    topic: &Topic,
    bank: &super::Bank,
) -> Option<&'a ParameterizedSource> {
    manifest.parameterized_sources.iter().find(|source| {
        source.topic_slug == topic.slug
            && source.replaces_static_bank_slug.as_deref() == Some(bank.slug.as_str())
    })
}

async fn create_blueprint(
    session: SessionTokenHash,
    input: &CreateBlueprintCourseInput,
    manifest: &Manifest,
    published: &BTreeMap<String, QuestionRevisionReference>,
    replacements: &ReplacementRevisions,
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
        loaded.availability == BlueprintAvailability::Private,
        "new curriculum Blueprint Course is not Private"
    );
    validate_loaded_content(
        &loaded.content,
        input,
        Some(manifest),
        published,
        replacements,
        false,
    )?;
    Ok((loaded.reference, loaded.current_revision))
}

fn validate_loaded_content(
    actual: &StoredBlueprintCourseContent,
    expected: &CreateBlueprintCourseInput,
    manifest: Option<&Manifest>,
    expected_revisions: &BTreeMap<String, QuestionRevisionReference>,
    replacements: &ReplacementRevisions,
    allow_pending_replacement: bool,
) -> Result<bool> {
    ensure!(
        actual.modules.len() == 1 && expected.modules.len() == 1,
        "curriculum Blueprint must retain one module"
    );
    let actual = &actual.modules[0];
    let expected = &expected.modules[0];
    ensure!(
        actual.label == expected.label && actual.assessments.len() == expected.assessments.len(),
        "curriculum Blueprint topic ordering differs"
    );
    let mut has_pending_replacement = false;
    for (assessment_index, (actual, expected)) in actual
        .assessments
        .iter()
        .zip(&expected.assessments)
        .enumerate()
    {
        ensure!(
            actual.content.title == expected.title
                && actual.content.instructions == expected.instructions
                && actual.content.defaults == expected.defaults
                && actual.content.entries.len() == expected.entries.len(),
            "curriculum Blueprint Assessment content differs"
        );
        for (entry_index, (actual, expected)) in actual
            .content
            .entries
            .iter()
            .zip(&expected.entries)
            .enumerate()
        {
            match (actual, expected) {
                (
                    StoredBlueprintAssessmentEntry::Pool {
                        question_revisions,
                        selection_count,
                        points_per_item,
                        scoring_rule,
                        selection_rule,
                        question_attempt_limit,
                        question_attempt_time_limit,
                    },
                    BlueprintAssessmentEntryInput::Pool(expected),
                ) => {
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
                        let manifest = manifest
                            .context("curriculum manifest is required for exact pin checks")?;
                        let topic = manifest.topics.get(assessment_index).with_context(|| {
                            "curriculum Blueprint Assessment has no manifest topic for exact pin check"
                        })?;
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
                (
                    StoredBlueprintAssessmentEntry::Fixed {
                        question_revision,
                        points_possible,
                        scoring_rule,
                        question_attempt_limit,
                        question_attempt_time_limit,
                    },
                    BlueprintAssessmentEntryInput::Fixed(expected),
                ) => {
                    ensure!(
                        question_revision.question_id == expected.question_id
                            && points_possible == &expected.points_possible
                            && scoring_rule == &expected.scoring_rule
                            && question_attempt_limit == &expected.question_attempt_limit
                            && question_attempt_time_limit == &expected.question_attempt_time_limit,
                        "curriculum Blueprint fixed Question pins or policy differs"
                    );
                    if !replacements.is_empty() {
                        let manifest = manifest.context(
                            "curriculum manifest is required for exact replacement pin checks",
                        )?;
                        let topic = manifest.topics.get(assessment_index).with_context(|| {
                            "curriculum Blueprint Assessment has no manifest topic for exact pin check"
                        })?;
                        let bank = topic.banks.get(entry_index).with_context(|| {
                            "curriculum Blueprint fixed entry has no manifest bank for exact pin check"
                        })?;
                        if replacement_source(manifest, topic, bank).is_some() {
                            let expected_revision = replacements
                                .get(&(topic.slug.clone(), bank.slug.clone()))
                                .with_context(|| {
                                    format!(
                                        "accepted canonical replacement is missing for {}/{}",
                                        topic.slug, bank.slug
                                    )
                                })?;
                            ensure!(
                                question_revision == expected_revision,
                                "curriculum Blueprint fixed Question exact Revision pin differs"
                            );
                        }
                    }
                }
                (
                    StoredBlueprintAssessmentEntry::Pool {
                        question_revisions,
                        selection_count,
                        points_per_item,
                        scoring_rule,
                        selection_rule,
                        question_attempt_limit,
                        question_attempt_time_limit,
                    },
                    BlueprintAssessmentEntryInput::Fixed(_),
                ) if allow_pending_replacement => {
                    let manifest = manifest.context(
                        "curriculum manifest is required for accepted replacement checks",
                    )?;
                    let topic = manifest.topics.get(assessment_index).with_context(|| {
                        "curriculum Blueprint Assessment has no manifest topic for accepted replacement check"
                    })?;
                    let bank = topic.banks.get(entry_index).with_context(|| {
                        "curriculum Blueprint Pool has no manifest bank for accepted replacement check"
                    })?;
                    ensure!(
                        replacement_source(manifest, topic, bank).is_some()
                            && replacements.contains_key(&(topic.slug.clone(), bank.slug.clone())),
                        "curriculum Blueprint has an unaccepted static Pool replacement"
                    );
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
                    ensure!(
                        selection_count == &bank.selection_count
                            && *points_per_item == AssessmentPointValue::from_whole(1)
                            && *scoring_rule == AssessmentEntryScoringRule::Normal
                            && *selection_rule
                                == QuestionPoolSelectionRule {
                                    selected_question_order:
                                        QuestionPoolSelectedQuestionOrder::QuestionPoolOrder,
                                }
                            && *question_attempt_limit
                                == QuestionAttemptLimit { max_attempts: None }
                            && *question_attempt_time_limit == QuestionAttemptTimeLimit::Unlimited,
                        "accepted static Pool policy differs before canonical replacement"
                    );
                    ensure_exact_pool_pins(question_revisions, &expected_pins)?;
                    has_pending_replacement = true;
                }
                _ => bail!("curriculum Blueprint entry kind differs"),
            }
        }
    }
    Ok(has_pending_replacement)
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

/// Converts complete reviewed content into a compare-and-swap replacement
/// while retaining every existing module and Assessment identity. Historical
/// Blueprint Revisions remain untouched in the Store; only the head receives
/// the accepted direct canonical Question entry.
fn replacement_input(
    retained: &StoredBlueprintCourseContent,
    input: &CreateBlueprintCourseInput,
) -> Result<ReplaceBlueprintCourseContentInput> {
    ensure!(
        retained.modules.len() == input.modules.len(),
        "retained curriculum Blueprint module mapping conflicts with replacement input"
    );
    let modules = retained
        .modules
        .iter()
        .zip(&input.modules)
        .map(|(retained_module, expected_module)| -> Result<_> {
            ensure!(
                retained_module.assessments.len() == expected_module.assessments.len(),
                "retained curriculum Blueprint Assessment mapping conflicts with replacement input"
            );
            Ok(BlueprintModuleReplacementInput {
                choice: BlueprintModuleEditChoice::Retained {
                    blueprint_module_reference: retained_module.blueprint_module_reference,
                },
                label: expected_module.label.clone(),
                assessments: retained_module
                    .assessments
                    .iter()
                    .zip(&expected_module.assessments)
                    .map(|(retained_assessment, expected_assessment)| {
                        BlueprintAssessmentReplacementInput {
                            choice: BlueprintAssessmentEditChoice::Retained {
                                blueprint_assessment_reference: retained_assessment
                                    .blueprint_assessment_reference,
                            },
                            content: expected_assessment.clone(),
                        }
                    })
                    .collect(),
            })
        })
        .collect::<Result<Vec<_>>>()?;
    let replacement = ReplaceBlueprintCourseContentInput { modules };
    replacement
        .validate()
        .map_err(|error| anyhow::anyhow!("curriculum Blueprint replacement is invalid: {error}"))?;
    Ok(replacement)
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

//! Host-only publication of the reviewed Chapter 1 teaching corpus.

use super::*;
use learning_data_access::{
    FlatQuestionGradingPayload, FlatQuestionPublicationPromotion, FlatQuestionStore,
    PublishedProblemRecord, PublishedSourceArtifact, UpsertFlatQuestionCommand,
};
use question_model::definition::QuestionDefinition;
use question_model::response::ChoiceOption;

const PILOT_PROVENANCE: &str = "Reviewed Chapter 1 pilot corpus from biology-problems-website revision 11f9ff635bd20d8fa334c360a8cba86bb0ab6527";
const PILOT_CONVERGENCE_ATTEMPTS: u8 = 3;
pub(super) const CHAPTER_ONE_FAKE_STUDENT_DISPLAY_NAME: &str = "Mary Fake Student";

const GENETICS_WEBWORK_MC: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../content/pilot/sources/genetics/genetic_disorders-which_one.pgml"
));
const GENETICS_WEBWORK_MATCHING: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../content/pilot/sources/genetics/genetic_disorders-matching.pgml"
));
const GENETICS_FLAT_MC: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../content/pilot/flat/genetics-disorders-mc.json"
));
const GENETICS_FLAT_MATCHING: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../content/pilot/flat/genetics-disorders-matching.json"
));
const BIOCHEMISTRY_WEBWORK_MC: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../content/pilot/sources/biochemistry/biochemical_functional_groups-which_one.pgml"
));
const BIOCHEMISTRY_WEBWORK_MATCHING: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../content/pilot/sources/biochemistry/biochemical_functional_groups-matching.pgml"
));
const BIOCHEMISTRY_FLAT_MC: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../content/pilot/flat/biochemistry-functional-groups-mc.json"
));
const BIOCHEMISTRY_FLAT_MATCHING: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../content/pilot/flat/biochemistry-functional-groups-matching.json"
));

#[derive(Clone, Copy)]
pub(super) enum PilotQuestionKind {
    WebworkMultipleChoice,
    WebworkMatching,
    FlatMultipleChoice,
    FlatMatching,
}

pub(super) struct PilotQuestionSpec {
    pub(super) slug: String,
    pub(super) title: String,
    pub(super) source_path: String,
    pub(super) source: &'static [u8],
    pub(super) kind: PilotQuestionKind,
    pub(super) points: u32,
}

pub(super) struct PilotChapterSpec {
    pub(super) slug: String,
    pub(super) course_title: String,
    pub(super) assignment_title: String,
    pub(super) questions: Vec<PilotQuestionSpec>,
}

pub(super) fn pilot_chapters() -> Result<Vec<PilotChapterSpec>> {
    let manifest = crate::pilot_content::validated_tracked_manifest()?;
    manifest
        .chapters
        .into_iter()
        .map(|chapter| {
            let questions = chapter
                .questions
                .into_iter()
                .map(pilot_question)
                .collect::<Result<Vec<_>>>()?;
            Ok(PilotChapterSpec {
                slug: chapter.slug,
                course_title: chapter.course_title,
                assignment_title: chapter.assignment_title,
                questions,
            })
        })
        .collect()
}

fn pilot_question(question: crate::pilot_content::Question) -> Result<PilotQuestionSpec> {
    use crate::pilot_content::{Backend, Family};

    let kind = match (question.backend, question.family) {
        (Backend::Webwork, Family::MultipleChoice) => PilotQuestionKind::WebworkMultipleChoice,
        (Backend::Webwork, Family::Matching) => PilotQuestionKind::WebworkMatching,
        (Backend::PleFlat, Family::MultipleChoice) => PilotQuestionKind::FlatMultipleChoice,
        (Backend::PleFlat, Family::Matching) => PilotQuestionKind::FlatMatching,
    };
    let relative_path = match question.backend {
        Backend::Webwork => question.source,
        Backend::PleFlat => question
            .payload
            .context("validated flat pilot question lacks its payload")?,
    };
    let source_path = format!("content/pilot/{}", relative_path.display());
    let source = match source_path.as_str() {
        "content/pilot/sources/genetics/genetic_disorders-which_one.pgml" => GENETICS_WEBWORK_MC,
        "content/pilot/sources/genetics/genetic_disorders-matching.pgml" => {
            GENETICS_WEBWORK_MATCHING
        }
        "content/pilot/flat/genetics-disorders-mc.json" => GENETICS_FLAT_MC,
        "content/pilot/flat/genetics-disorders-matching.json" => GENETICS_FLAT_MATCHING,
        "content/pilot/sources/biochemistry/biochemical_functional_groups-which_one.pgml" => {
            BIOCHEMISTRY_WEBWORK_MC
        }
        "content/pilot/sources/biochemistry/biochemical_functional_groups-matching.pgml" => {
            BIOCHEMISTRY_WEBWORK_MATCHING
        }
        "content/pilot/flat/biochemistry-functional-groups-mc.json" => BIOCHEMISTRY_FLAT_MC,
        "content/pilot/flat/biochemistry-functional-groups-matching.json" => {
            BIOCHEMISTRY_FLAT_MATCHING
        }
        _ => bail!(
            "validated pilot inventory contains an unembedded publication source: {source_path}"
        ),
    };
    Ok(PilotQuestionSpec {
        slug: question.slug,
        title: question.title,
        source_path,
        source,
        kind,
        points: question.points,
    })
}

pub(super) async fn seed_chapter_one_pilot(
    arguments: &SeedArguments,
) -> Result<ChapterOnePilotManifest> {
    let chapter_specs = pilot_chapters()?;
    let storage = arguments
        .chapter_one_pilot
        .as_ref()
        .expect("Chapter 1 storage exists after explicit flag dispatch");
    let pool = learning_data_access::postgres::lazy_pool(&arguments.database_url)
        .context("invalid --database-url for Chapter 1 pilot seed")?;
    learning_data_access::postgres::apply_migrations(&pool)
        .await
        .context("applying embedded migrations for Chapter 1 pilot seed")?;
    let store = question_id_store(pool)?;
    let context = TenantContext::from_authenticated_session(arguments.tenant);
    let resumed = select_chapter_one_resume_manifest(
        &store,
        context,
        arguments.tenant,
        &chapter_specs,
        arguments.chapter_one_existing_manifest.as_deref(),
    )
    .await?;
    let objects = pilot_object_store(storage)?;
    let mut chapters = Vec::with_capacity(chapter_specs.len());
    let mut statistics_fixture = None;

    for chapter in chapter_specs {
        let course_id = CourseId::from_uuid(pilot_uuid(arguments.tenant, &chapter.slug, "course"));
        let assignment_id =
            AssignmentId::from_uuid(pilot_uuid(arguments.tenant, &chapter.slug, "assignment"));
        // The deterministic course is the durable outer marker. It exists
        // before any question/object publication, so an interrupted Chapter
        // One run stops at the protected manifest boundary instead of
        // classifying retained publication as a fresh corpus.
        ensure_webwork_pilot_course(
            &store,
            context,
            arguments.instructor,
            CourseRecord {
                id: course_id,
                tenant: arguments.tenant,
                title: chapter.course_title.clone(),
                term: question_model::CourseTerm::from_parts(
                    "2026-08-24",
                    "2026-12-18",
                    "America/Chicago",
                )
                .expect("explicit fixture course term"),
            },
        )
        .await?;
        let mut published = Vec::with_capacity(chapter.questions.len());
        let mut items = Vec::with_capacity(chapter.questions.len());
        for (position, question) in chapter.questions.iter().enumerate() {
            let existing =
                resumed_question(&store, context, resumed.as_ref(), &chapter.slug, question)
                    .await?;
            let ids = existing
                .as_ref()
                .map_or_else(QuestionIds::generate, QuestionIds::from_published);
            let record = match question.kind {
                PilotQuestionKind::WebworkMultipleChoice | PilotQuestionKind::WebworkMatching => {
                    publish_webwork_question(
                        &store,
                        &objects,
                        context,
                        arguments.instructor,
                        question,
                        &ids,
                        existing.as_ref(),
                    )
                    .await?
                }
                PilotQuestionKind::FlatMultipleChoice | PilotQuestionKind::FlatMatching => {
                    publish_flat_question(
                        &store,
                        &objects,
                        context,
                        arguments.instructor,
                        question,
                        &ids,
                        existing.as_ref(),
                    )
                    .await?
                }
            };
            let reference = ProblemVersionRef {
                problem: record.problem,
                version: record.version,
            };
            if is_catalog_statistics_fixture(question) {
                if !matches!(question.kind, PilotQuestionKind::FlatMultipleChoice) {
                    bail!(
                        "Chapter 1 catalog-statistics fixture must remain a flat multiple-choice question"
                    );
                }
                if statistics_fixture
                    .replace(ChapterOneStatisticsFixture {
                        reference,
                        source: question.source,
                    })
                    .is_some()
                {
                    bail!("Chapter 1 catalog-statistics fixture is published more than once");
                }
            }
            items.push(AssignmentItem {
                id: AssignmentItemId::from_uuid(pilot_uuid(
                    arguments.tenant,
                    &question.slug,
                    "assignment-item",
                )),
                reference,
                position: u32::try_from(position).expect("four questions fit u32"),
                points_possible: PointValue::from_whole(question.points),
                delivery_state: AssignmentDeliveryState::Active,
                scoring_mode: AssignmentScoringMode::Normal,
            });
            published.push(QuestionManifest {
                slug: question.slug.clone(),
                display_id: record.question_id.to_string(),
                problem_id: record.problem,
                version_id: record.version,
            });
        }

        ensure_webwork_pilot_assignment(
            &store,
            context,
            AssignmentRecord {
                id: assignment_id,
                tenant: arguments.tenant,
                course_id,
                title: chapter.assignment_title.clone(),
                audience: question_model::AssignmentAudience::CourseWide,
                disclosure_policy: question_model::LearnerDisclosurePolicy::default(),
                items,
                selection_groups: Vec::new(),
                policies: RunPolicies {
                    completion: CompletionRequirement::AnswerAll,
                    grade: GradePolicy::Highest,
                    continued_practice: ContinuedPractice::Unlimited,
                    variation: VariationPolicy::NewSeeds,
                },
            },
        )
        .await?;
        let enrollment = upsert_chapter_one_student(
            &store,
            context,
            arguments.instructor,
            arguments.student,
            course_id,
            assignment_id,
        )
        .await?;
        chapters.push(ChapterManifest {
            slug: chapter.slug,
            course_id,
            assignment_id,
            enrollment_id: enrollment.id,
            questions: published,
        });
    }
    let statistics_fixture = statistics_fixture
        .ok_or_else(|| anyhow::anyhow!("Chapter 1 catalog-statistics fixture is missing"))?;
    seed_chapter_one_statistics(&store, context, arguments, statistics_fixture).await?;
    let output = ChapterOnePilotManifest { chapters };
    if resumed.as_ref().is_some_and(|input| input != &output) {
        bail!(
            "existing Chapter 1 manifest does not exactly match regenerated publication manifest"
        );
    }
    Ok(output)
}

/// Creates the disposable Chapter 1 learner through the canonical membership
/// owner, then materializes its one assignment receipt through the sole
/// entitlement seam.
pub(super) async fn upsert_chapter_one_student<S>(
    store: &S,
    context: TenantContext,
    instructor: UserId,
    student: UserId,
    course: CourseId,
    assignment: AssignmentId,
) -> Result<AssignmentEnrollment>
where
    S: Store + CourseRosterStore,
{
    let accepted = store
        .upsert_course_member(
            context,
            UpsertCourseMember {
                course,
                user: student,
                display_name: CHAPTER_ONE_FAKE_STUDENT_DISPLAY_NAME.to_string(),
                roster_contact: None,
            },
        )
        .await
        .context("creating the disposable Chapter 1 learner through the canonical roster")?;
    if accepted.member.status != learning_data_access::CourseMemberStatus::Active
        || accepted.member.roster_email.is_some()
        || accepted.member.roster_id.is_some()
    {
        bail!("Chapter 1 learner creation did not produce an active no-contact roster member");
    }
    ensure_webwork_pilot_enrollment(store, context, instructor, student, course, assignment)
        .await
        .context("materializing Chapter 1 assignment enrollment")
}

fn pilot_object_store(storage: &WebworkPilotStorage) -> Result<objects::s3::S3ObjectStore> {
    let client = objects::minio::client(&objects::minio::EndpointConfig {
        endpoint_url: storage.endpoint_url.clone(),
        region: storage.region.clone(),
        access_key_id: required_secret_environment("AWS_ACCESS_KEY_ID")?,
        secret_access_key: required_secret_environment("AWS_SECRET_ACCESS_KEY")?,
    });
    Ok(objects::s3::S3ObjectStore::new(
        client,
        objects::s3::BucketNames {
            private_content: storage.private_content_bucket.clone(),
            ..objects::s3::BucketNames::default()
        },
    ))
}

pub(super) async fn publish_webwork_question<S, O>(
    store: &S,
    objects: &O,
    context: TenantContext,
    publisher: UserId,
    spec: &PilotQuestionSpec,
    ids: &QuestionIds,
    existing: Option<&PublishedProblemRecord>,
) -> Result<PublishedProblemRecord>
where
    S: Store + CatalogStore + CatalogSourceStore + AuthoritativeTimeStore,
    O: ObjectStore,
{
    let reference = ProblemVersionRef {
        problem: ids.problem,
        version: ids.version,
    };
    let source = QuestionSource::Webwork {
        pg_path: spec.source_path.to_string(),
    };
    let draft = DraftRecord {
        tenant: context.tenant_id(),
        question: webwork_draft(ids.workspace, spec),
        derived_from: None,
    };
    let source_sha256 = objects::Sha256Digest::compute(spec.source).to_string();
    let capabilities =
        adapter_webwork::reviewed_webwork_source_profile_capabilities(&source, &source_sha256)
            .context("resolving reviewed WeBWorK pilot capabilities")?;
    let source_key = ObjectKey::ProblemSource {
        problem: ids.problem,
        version: ids.version,
        object: ids.published_source,
    };
    if let Some(existing) = existing {
        return verify_resumed_question(
            store,
            objects,
            context,
            publisher,
            existing,
            &draft.question,
            &source,
            &capabilities,
            question_model::QuestionBackend::Webwork,
            spec.source,
            "text/x-wework-pg",
        )
        .await
        .map(|()| existing.clone());
    }
    if let Some(existing) = store.get_catalog_problem(context, reference).await? {
        verify_existing_question(
            store,
            objects,
            context,
            publisher,
            &existing,
            &draft.question,
            &source,
            &capabilities,
            question_model::QuestionBackend::Webwork,
            &source_key,
            spec.source,
            "text/x-wework-pg",
        )
        .await?;
        return Ok(existing);
    }
    let artifact = put_pilot_object(
        store,
        objects,
        context,
        source_key.clone(),
        spec.source,
        "text/x-wework-pg",
        Some(ids.version),
    )
    .await?;
    let violations = domain::policy::validate_draft_for_publication(&draft.question, &capabilities);
    if !violations.is_empty() {
        bail!("reviewed WeBWorK pilot question failed capability admission: {violations:?}");
    }
    let source_artifact = PublishedSourceArtifact {
        reference,
        backend: question_model::QuestionBackend::Webwork,
        object: artifact,
    };
    for _ in 0..PILOT_CONVERGENCE_ATTEMPTS {
        if let Some(existing) = store.get_catalog_problem(context, reference).await? {
            verify_existing_question(
                store,
                objects,
                context,
                publisher,
                &existing,
                &draft.question,
                &source,
                &capabilities,
                question_model::QuestionBackend::Webwork,
                &source_key,
                spec.source,
                "text/x-wework-pg",
            )
            .await?;
            return Ok(existing);
        }
        let Some(saved) = ensure_pilot_draft(store, context, publisher, draft.clone()).await?
        else {
            // A concurrent publisher may consume this exact draft before this
            // caller rereads it. The next iteration rechecks the immutable
            // version before creating anything else.
            continue;
        };
        match store
            .publish_draft(
                context,
                publisher,
                PublishDraftCommand {
                    expected_draft: draft.clone(),
                    expected_revision: saved.revision,
                    publication: reference,
                    published_source: source.clone(),
                    source_artifact: Some(source_artifact.clone()),
                    qti_promotion: None,
                    flat_question_promotion: None,
                    publisher,
                    scope: PublicationScope::Institution,
                    byline: question_model::PublicByline::new(vec![
                        question_model::PublicAuthorName::new(
                            "Chapter One Instructor".to_string(),
                        )?,
                    ])?,
                    capabilities: capabilities.clone(),
                },
            )
            .await
        {
            Ok(record) => return Ok(record),
            Err(StoreError::AlreadyExists | StoreError::Conflict) => continue,
            Err(error) => return Err(error).context("publishing reviewed WeBWorK pilot question"),
        }
    }
    bail!("reviewed WeBWorK pilot publication did not converge")
}

async fn publish_flat_question(
    store: &learning_data_access::postgres::PostgresStore,
    objects: &objects::s3::S3ObjectStore,
    context: TenantContext,
    publisher: UserId,
    spec: &PilotQuestionSpec,
    ids: &QuestionIds,
    existing: Option<&PublishedProblemRecord>,
) -> Result<PublishedProblemRecord> {
    let reference = ProblemVersionRef {
        problem: ids.problem,
        version: ids.version,
    };
    let document = adapter_native::flat_question::FlatQuestionDocument::parse(spec.source)
        .with_context(|| format!("parsing reviewed flat pilot source {}", spec.slug))?;
    let canonical = document
        .canonical_bytes()
        .with_context(|| format!("canonicalizing reviewed flat pilot source {}", spec.slug))?;
    let (question, private) = document
        .compile(ids.workspace)
        .with_context(|| format!("compiling reviewed flat pilot source {}", spec.slug))?
        .into_parts();
    let draft = DraftRecord {
        tenant: context.tenant_id(),
        question,
        derived_from: None,
    };
    let family = match &draft.question.source {
        DraftQuestionSource::Native { family } => family.clone(),
        _ => unreachable!("the flat compiler always emits a native draft"),
    };
    let published_source = QuestionSource::Native {
        family: family.clone(),
    };
    let capabilities = NativeAdapter::new()
        .capabilities(&published_source)
        .context("resolving reviewed flat pilot capabilities")?;
    if let Some(existing) = existing {
        return verify_resumed_question(
            store,
            objects,
            context,
            publisher,
            existing,
            &draft.question,
            &published_source,
            &capabilities,
            question_model::QuestionBackend::Native,
            &canonical,
            adapter_native::flat_question::FLAT_QUESTION_MEDIA_TYPE,
        )
        .await
        .map(|()| existing.clone());
    }
    let published_key = ObjectKey::ProblemSource {
        problem: ids.problem,
        version: ids.version,
        object: ids.published_source,
    };
    if let Some(existing) = store.get_catalog_problem(context, reference).await? {
        verify_existing_question(
            store,
            objects,
            context,
            publisher,
            &existing,
            &draft.question,
            &published_source,
            &capabilities,
            question_model::QuestionBackend::Native,
            &published_key,
            &canonical,
            adapter_native::flat_question::FLAT_QUESTION_MEDIA_TYPE,
        )
        .await?;
        return Ok(existing);
    }
    let source_record = put_pilot_object(
        store,
        objects,
        context,
        ObjectKey::WorkspaceQuestionSource {
            tenant: context.tenant_id(),
            workspace: ids.workspace,
            object: ids.workspace_source,
        },
        &canonical,
        adapter_native::flat_question::FLAT_QUESTION_MEDIA_TYPE,
        None,
    )
    .await?;
    let grading = FlatQuestionGradingPayload::from_private(&private)?;
    let canonical_source_sha256 = objects::Sha256Digest::compute(&canonical).to_string();
    let public_binding_sha256 = grading.public_binding_sha256().to_string();
    let staged = if let Some(existing) = store
        .flat_question_source(context, publisher, ids.workspace)
        .await?
    {
        let saved = store
            .get_draft(context, publisher, ids.workspace)
            .await?
            .context("staged flat pilot source has no matching draft")?;
        if saved.record != draft
            || saved.revision != existing.workspace_revision
            || existing.source_family != family
            || existing.source_record != source_record
            || existing.canonical_source_sha256 != canonical_source_sha256
            || existing.public_binding_sha256 != public_binding_sha256
        {
            bail!("existing staged flat pilot question differs from reviewed content");
        }
        existing
    } else {
        if store
            .get_draft(context, publisher, ids.workspace)
            .await?
            .is_some()
        {
            bail!("existing pilot draft is not a reviewed flat-question staging record");
        }
        store
            .upsert_flat_question(
                context,
                publisher,
                UpsertFlatQuestionCommand {
                    expected_revision: None,
                    draft: draft.clone(),
                    source: source_record,
                    canonical_source_sha256,
                    public_binding_sha256,
                    grading,
                },
            )
            .await
            .context("staging reviewed flat pilot question")?
    };
    let published_object = put_pilot_object(
        store,
        objects,
        context,
        published_key.clone(),
        &canonical,
        adapter_native::flat_question::FLAT_QUESTION_MEDIA_TYPE,
        Some(ids.version),
    )
    .await?;
    for _ in 0..PILOT_CONVERGENCE_ATTEMPTS {
        if let Some(existing) = store.get_catalog_problem(context, reference).await? {
            verify_existing_question(
                store,
                objects,
                context,
                publisher,
                &existing,
                &draft.question,
                &published_source,
                &capabilities,
                question_model::QuestionBackend::Native,
                &published_key,
                &canonical,
                adapter_native::flat_question::FLAT_QUESTION_MEDIA_TYPE,
            )
            .await?;
            return Ok(existing);
        }
        match store
            .publish_draft(
                context,
                publisher,
                PublishDraftCommand {
                    expected_draft: draft.clone(),
                    expected_revision: staged.workspace_revision,
                    publication: reference,
                    published_source: published_source.clone(),
                    source_artifact: Some(PublishedSourceArtifact {
                        reference,
                        backend: question_model::QuestionBackend::Native,
                        object: published_object.clone(),
                    }),
                    qti_promotion: None,
                    flat_question_promotion: Some(FlatQuestionPublicationPromotion {
                        source: staged.clone(),
                        import_origin: None,
                        published_question: draft.question.clone(),
                        assets: Vec::new(),
                    }),
                    publisher,
                    scope: PublicationScope::Institution,
                    byline: question_model::PublicByline::new(vec![
                        question_model::PublicAuthorName::new(
                            "Chapter One Instructor".to_string(),
                        )?,
                    ])?,
                    capabilities: capabilities.clone(),
                },
            )
            .await
        {
            Ok(record) => return Ok(record),
            Err(StoreError::AlreadyExists | StoreError::Conflict) => continue,
            Err(error) => return Err(error).context("publishing reviewed flat pilot question"),
        }
    }
    bail!("reviewed flat pilot publication did not converge")
}

async fn ensure_pilot_draft<S>(
    store: &S,
    context: TenantContext,
    publisher: UserId,
    expected: DraftRecord,
) -> Result<Option<learning_data_access::WorkspaceDraft>>
where
    S: Store,
{
    match store
        .get_draft(context, publisher, expected.question.workspace)
        .await?
    {
        Some(actual) if actual.record == expected => Ok(Some(actual)),
        Some(_) => bail!("existing pilot draft differs from reviewed content"),
        None => match store
            .upsert_draft(context, publisher, None, expected.clone())
            .await
        {
            Ok(saved) => Ok(Some(saved)),
            Err(StoreError::AlreadyExists | StoreError::Conflict) => store
                .get_draft(context, publisher, expected.question.workspace)
                .await?
                .map_or_else(
                    || Ok(None),
                    |actual| {
                        (actual.record == expected)
                            .then_some(actual)
                            .context("concurrent pilot draft differs from reviewed content")
                            .map(Some)
                    },
                ),
            Err(error) => Err(error).context("staging reviewed pilot draft"),
        },
    }
}

async fn put_pilot_object<S, O>(
    store: &S,
    objects: &O,
    context: TenantContext,
    key: ObjectKey,
    bytes: &[u8],
    media_type: &str,
    version: Option<VersionId>,
) -> Result<objects::ObjectRecord>
where
    S: Store + AuthoritativeTimeStore,
    O: ObjectStore,
{
    let expected_sha256 = objects::Sha256Digest::compute(bytes);
    let expected_id = key.object_id();
    let record = match objects
        .put(PutObject {
            key: key.clone(),
            bytes: bytes.to_vec(),
            media_type: media_type.to_string(),
            license: "CC-BY-4.0".to_string(),
            provenance: PILOT_PROVENANCE.to_string(),
            created_at: store.authoritative_time(context).await?,
        })
        .await
    {
        Ok(record) => record,
        Err(objects::ObjectStoreError::AlreadyExists) => objects.get(&key).await?.record,
        Err(error) => return Err(error).context("writing reviewed pilot source object"),
    };
    if record.id != expected_id
        || record.key != key
        || record.sha256 != expected_sha256
        || record.size_bytes != u64::try_from(bytes.len()).expect("pilot source fits u64")
        || record.media_type != media_type
        || record.category != ObjectCategory::Source
        || record.version != version
        || record.license != "CC-BY-4.0"
        || record.provenance != PILOT_PROVENANCE
    {
        bail!("existing pilot source object differs from reviewed content");
    }
    Ok(record)
}

pub(super) fn webwork_draft(
    workspace: WorkspaceId,
    spec: &PilotQuestionSpec,
) -> DraftQuestionDefinition {
    let response = match spec.kind {
        PilotQuestionKind::WebworkMultipleChoice => ResponseDefinition::MultipleChoice {
            choices: vec![placeholder_choice("renderer-choice", "Rendered by WeBWorK")],
            selection: SelectionCardinality::ExactlyOne,
        },
        PilotQuestionKind::WebworkMatching => ResponseDefinition::Matching {
            prompts: vec![placeholder_choice("renderer-prompt", "Rendered by WeBWorK")],
            choices: vec![placeholder_choice("renderer-choice", "Rendered by WeBWorK")],
        },
        PilotQuestionKind::FlatMultipleChoice | PilotQuestionKind::FlatMatching => {
            unreachable!("flat questions compile their own draft")
        }
    };
    DraftQuestionDefinition {
        workspace,
        source: DraftQuestionSource::Webwork {
            pg_path: spec.source_path.to_string(),
        },
        prompt: vec![ContentBlock::Text {
            markdown: "This question is rendered by the private WeBWorK service.".to_string(),
        }],
        response,
        attempt_policy: AttemptPolicy { max_attempts: None },
        timing_policy: TimingPolicy::Untimed,
        randomization: RandomizationDefinition::Seeded {
            generator: GeneratorReference {
                id: "webwork-problem-seed".to_string(),
                version: "1".to_string(),
            },
            parameters: BTreeMap::new(),
        },
        grading: match spec.kind {
            PilotQuestionKind::WebworkMatching => GradingDefinition::PartialCredit {
                points: f64::from(spec.points),
            },
            _ => GradingDefinition::AllOrNothing {
                points: f64::from(spec.points),
            },
        },
        metadata: QuestionMetadata {
            title: spec.title.to_string(),
            tags: vec![Tag::new("chapter-1"), Tag::new("webwork-pilot")],
            taxonomy: Vec::new(),
            license: License::CcBy,
            language: "en-US".to_string(),
        },
    }
}

fn placeholder_choice(id: &str, text: &str) -> ChoiceOption {
    ChoiceOption {
        id: ChoiceId::new(id),
        body: vec![ContentBlock::Text {
            markdown: text.to_string(),
        }],
    }
}

#[allow(clippy::too_many_arguments)]
async fn verify_existing_question<S, O>(
    store: &S,
    objects: &O,
    context: TenantContext,
    publisher: UserId,
    record: &PublishedProblemRecord,
    expected_draft: &DraftQuestionDefinition,
    expected_source: &QuestionSource,
    expected_capabilities: &BackendCapabilities,
    expected_backend: question_model::QuestionBackend,
    expected_key: &ObjectKey,
    expected_bytes: &[u8],
    expected_media_type: &str,
) -> Result<()>
where
    S: CatalogSourceStore,
    O: ObjectStore,
{
    let expected_question = QuestionDefinition::from_draft(
        expected_draft.clone(),
        record.problem,
        record.version,
        expected_source.clone(),
    );
    if record.question != expected_question
        || record.capabilities != *expected_capabilities
        || record.scope != PublicationScope::Institution
        || record.lifecycle != CatalogLifecycle::Published
        || record.author_ids.as_slice() != [publisher]
        || record.derived_from.is_some()
    {
        bail!("existing Chapter 1 pilot publication differs from reviewed content");
    }
    let reference = ProblemVersionRef {
        problem: record.problem,
        version: record.version,
    };
    let artifact = store
        .catalog_source_artifact(context, reference)
        .await?
        .context("existing Chapter 1 pilot publication has no source artifact")?;
    let stored = objects
        .get(expected_key)
        .await
        .context("reading existing Chapter 1 pilot source object")?;
    if artifact.reference != reference
        || artifact.backend != expected_backend
        || artifact.object != stored.record
        || stored.bytes != expected_bytes
        || stored.record.key != *expected_key
        || stored.record.id != expected_key.object_id()
        || stored.record.sha256 != objects::Sha256Digest::compute(expected_bytes)
        || stored.record.size_bytes
            != u64::try_from(expected_bytes.len()).expect("pilot source fits u64")
        || stored.record.media_type != expected_media_type
        || stored.record.category != ObjectCategory::Source
        || stored.record.version != Some(record.version)
        || stored.record.license != "CC-BY-4.0"
        || stored.record.provenance != PILOT_PROVENANCE
    {
        bail!("existing Chapter 1 pilot source differs from reviewed content");
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
async fn verify_resumed_question<S, O>(
    store: &S,
    objects: &O,
    context: TenantContext,
    publisher: UserId,
    record: &PublishedProblemRecord,
    expected_draft: &DraftQuestionDefinition,
    expected_source: &QuestionSource,
    expected_capabilities: &BackendCapabilities,
    expected_backend: question_model::QuestionBackend,
    expected_bytes: &[u8],
    expected_media_type: &str,
) -> Result<()>
where
    S: CatalogSourceStore,
    O: ObjectStore,
{
    let reference = ProblemVersionRef {
        problem: record.problem,
        version: record.version,
    };
    let artifact = store
        .catalog_source_artifact(context, reference)
        .await?
        .context("resumed Chapter 1 publication has no source artifact")?;
    verify_existing_question(
        store,
        objects,
        context,
        publisher,
        record,
        expected_draft,
        expected_source,
        expected_capabilities,
        expected_backend,
        &artifact.object.key,
        expected_bytes,
        expected_media_type,
    )
    .await
}

//! Host-only E2E seed webwork capability.

use super::*;

/// Seeds one licensed, immutable PGML source through the same PostgreSQL
/// catalog binding that the production WebWork backend later resolves. This is
/// an opt-in host tool: no HTTP route or browser-supplied storage value exists.
pub(super) async fn seed_webwork_pilot(arguments: &SeedArguments) -> Result<Manifest> {
    let student = arguments.course_student()?;
    let storage = arguments
        .webwork_pilot
        .as_ref()
        .expect("WebWork pilot storage exists after explicit flag dispatch");
    let pool = learning_data_access::postgres::lazy_pool(&arguments.database_url)
        .context("invalid --database-url for WebWork E2E seed")?;
    learning_data_access::postgres::apply_migrations(&pool)
        .await
        .context("applying embedded migrations for WebWork E2E seed")?;
    let store = crate::postgres_store::configured_postgres_store(pool)?;
    let context = TenantContext::from_authenticated_session(arguments.tenant);
    let marker = WebworkPilotSeedIds::fresh_for_installation();
    let existing_course = store
        .get_course(context, marker.course)
        .await
        .context("reading WebWork seed course marker")?;
    let existing_assignment = store
        .get_assignment_for_edit(context, marker.assignment)
        .await
        .context("reading WebWork seed assignment marker")?;
    let (ids, published) = match seed_replay_state(
        existing_course.is_some(),
        existing_assignment.is_some(),
        "WebWork seed",
    )? {
        SeedReplayState::Fresh => {
            // Persist this marker before object and catalog publication so a
            // later retry cannot mint a second publication after a crash.
            ensure_webwork_pilot_course(
                &store,
                context,
                arguments.instructor,
                webwork_pilot_course(arguments, marker.course),
            )
            .await?;
            publish_fresh_webwork(&store, context, arguments, storage, marker).await?
        }
        SeedReplayState::Replay => {
            let course = existing_course.expect("replay state has a course marker");
            let assignment = existing_assignment.expect("replay state has an assignment marker");
            let reference = webwork_assignment_reference(&assignment.record, marker)?;
            let published = store
                .get_catalog_problem(context, reference)
                .await
                .context("reading retained WebWork publication")?
                .context("WebWork seed assignment refers to a missing publication")?;
            let artifact = store
                .catalog_source_artifact(context, reference)
                .await
                .context("reading retained WebWork source binding")?
                .context("retained WebWork publication has no source binding")?;
            let ids = WebworkPilotSeedIds::from_published(&published, artifact.object.id);
            let expected_course = webwork_pilot_course(arguments, ids.course);
            if !webwork_pilot_course_seed_matches(&course, &expected_course) {
                bail!("WebWork seed course marker differs from the reviewed host seed");
            }
            let source_record =
                put_webwork_pilot_source(&store, context, storage, reference, artifact.object.id)
                    .await?;
            ensure_webwork_pilot_publication(
                &store,
                context,
                arguments.instructor,
                DraftRecord {
                    question: webwork_pilot_draft(ids.workspace),
                    derived_from: None,
                },
                reference,
                source_record,
                webwork_capabilities(),
            )
            .await?;
            let expected_assignment = webwork_pilot_assignment(arguments, ids, reference);
            if assignment.record != expected_assignment {
                bail!("WebWork seed assignment differs from the retained immutable publication");
            }
            (ids, published)
        }
    };
    let reference = ProblemVersionRef {
        problem: published.problem,
        version: published.version,
    };
    let course = webwork_pilot_course(arguments, ids.course);
    ensure_webwork_pilot_course(&store, context, arguments.instructor, course).await?;
    let assignment = webwork_pilot_assignment(arguments, ids, reference);
    ensure_webwork_pilot_assignment(&store, context, arguments.instructor, assignment).await?;
    let enrollment = ensure_webwork_pilot_enrollment(
        &store,
        context,
        arguments.instructor,
        student,
        ids.course,
        ids.assignment,
    )
    .await?;
    Ok(Manifest {
        course_id: ids.course,
        assignment_id: ids.assignment,
        enrollment_id: enrollment.id,
        question_id: published.question_id.clone(),
        problem_id: published.problem,
        version_id: published.version,
    })
}

async fn publish_fresh_webwork(
    store: &learning_data_access::postgres::PostgresStore,
    context: TenantContext,
    arguments: &SeedArguments,
    storage: &WebworkPilotStorage,
    ids: WebworkPilotSeedIds,
) -> Result<(
    WebworkPilotSeedIds,
    learning_data_access::PublishedProblemRecord,
)> {
    let reference = ProblemVersionRef {
        problem: ids.problem,
        version: ids.version,
    };
    let source_record =
        put_webwork_pilot_source(store, context, storage, reference, ids.source_object).await?;
    let draft = DraftRecord {
        question: webwork_pilot_draft(ids.workspace),
        derived_from: None,
    };
    let capabilities = webwork_capabilities();
    let violations = domain::policy::validate_draft_for_publication(&draft.question, &capabilities);
    if !violations.is_empty() {
        bail!("WebWork pilot seed draft failed publication capability admission: {violations:?}");
    }
    let published = ensure_webwork_pilot_publication(
        store,
        context,
        arguments.instructor,
        draft,
        reference,
        source_record,
        capabilities,
    )
    .await?;
    Ok((ids, published))
}

fn webwork_pilot_course(arguments: &SeedArguments, course: CourseId) -> CourseRecord {
    CourseRecord {
        id: course,
        title: "PLE WebWork pilot E2E course".to_string(),
        term: question_model::CourseTerm::from_parts("2026-08-24", "2026-12-18", "America/Chicago")
            .expect("explicit fixture course term"),
    }
}

fn webwork_pilot_assignment(
    arguments: &SeedArguments,
    ids: WebworkPilotSeedIds,
    reference: ProblemVersionRef,
) -> AssignmentRecord {
    AssignmentRecord {
        id: ids.assignment,
        course_id: ids.course,
        title: "PLE WebWork pilot E2E assignment".to_string(),
        lifecycle: question_model::AssignmentLifecycle::Published,
        instructions: question_model::AssignmentInstructions::try_new(
            "Solve the guided WeBWorK pilot problem, then explain your reasoning.".to_string(),
        )
        .expect("WebWork pilot instructions are valid"),
        audience: question_model::AssignmentAudience::CourseWide,
        disclosure_policy: question_model::StudentDisclosurePolicy::default(),
        items: vec![AssignmentItem {
            id: ids.assignment_item,
            reference,
            position: 0,
            points_possible: PointValue::from_whole(1),
            delivery_state: AssignmentDeliveryState::Active,
            scoring_mode: AssignmentScoringMode::Normal,
        }],
        selection_groups: Vec::new(),
        policies: RunPolicies {
            completion: CompletionRequirement::AnswerAll,
            grade: GradePolicy::Highest,
            continued_practice: ContinuedPractice::Unlimited,
            variation: VariationPolicy::NewSeeds,
        },
    }
}

fn webwork_assignment_reference(
    assignment: &AssignmentRecord,
    ids: WebworkPilotSeedIds,
) -> Result<ProblemVersionRef> {
    let Some(item) = assignment.items.first() else {
        bail!("WebWork seed assignment has no fixed publication item");
    };
    if assignment.items.len() != 1 || item.id != ids.assignment_item || item.position != 0 {
        bail!("WebWork seed assignment does not retain one reviewed fixed item");
    }
    Ok(item.reference)
}

pub(super) async fn put_webwork_pilot_source(
    store: &learning_data_access::postgres::PostgresStore,
    context: TenantContext,
    storage: &WebworkPilotStorage,
    reference: ProblemVersionRef,
    object: ObjectId,
) -> Result<objects::ObjectRecord> {
    validate_webwork_pilot_source_provenance(WEBWORK_PILOT_SOURCE)?;
    let access_key_id = required_secret_environment("AWS_ACCESS_KEY_ID")?;
    let secret_access_key = required_secret_environment("AWS_SECRET_ACCESS_KEY")?;
    let client = objects::minio::client(&objects::minio::EndpointConfig {
        endpoint_url: storage.endpoint_url.clone(),
        region: storage.region.clone(),
        access_key_id,
        secret_access_key,
    });
    let objects = objects::s3::S3ObjectStore::new(
        client,
        objects::s3::BucketNames {
            private_content: storage.private_content_bucket.clone(),
            ..objects::s3::BucketNames::default()
        },
    );
    let key = webwork_pilot_source_key(reference, object);
    let request = PutObject {
        key: key.clone(),
        bytes: WEBWORK_PILOT_SOURCE.to_vec(),
        media_type: "text/x-wework-pg".to_string(),
        license: "CC-BY-4.0".to_string(),
        provenance: WEBWORK_PILOT_SOURCE_PROVENANCE.to_string(),
        created_at: store
            .authoritative_time()
            .await
            .context("reading database time for WebWork source provenance")?,
    };
    let record = match objects.put(request).await {
        Ok(record) => record,
        Err(objects::ObjectStoreError::AlreadyExists) => {
            objects
                .get(&key)
                .await
                .context("reading existing immutable WebWork pilot source")?
                .record
        }
        Err(error) => return Err(error).context("writing immutable WebWork pilot source"),
    };
    if record.id != object
        || record.key != key
        || record.category != ObjectCategory::Source
        || record.version != Some(reference.version)
        || record.sha256.to_string() != WEBWORK_PILOT_SOURCE_SHA256
        || record.size_bytes != u64::try_from(WEBWORK_PILOT_SOURCE.len()).expect("source fits u64")
        || record.media_type != "text/x-wework-pg"
        || record.license != "CC-BY-4.0"
        || record.provenance != WEBWORK_PILOT_SOURCE_PROVENANCE
    {
        bail!("existing WebWork pilot source does not match its immutable provenance record");
    }
    Ok(record)
}

/// Checks the tracked source against the reviewed digest before any private
/// object write. This keeps provenance admission explicit and fail-closed
/// (ASVS 2.2.1).
pub(super) fn validate_webwork_pilot_source_provenance(source: &[u8]) -> Result<()> {
    if objects::Sha256Digest::compute(source).to_string() != WEBWORK_PILOT_SOURCE_SHA256 {
        bail!("tracked WebWork pilot source digest differs from its recorded provenance");
    }
    Ok(())
}

pub(super) fn required_secret_environment(name: &str) -> Result<String> {
    let value = std::env::var(name)
        .with_context(|| format!("{name} is required for WebWork pilot object storage"))?;
    if value.is_empty() {
        bail!("{name} must not be empty for WebWork pilot object storage");
    }
    Ok(value)
}

pub(super) async fn ensure_webwork_pilot_publication<S>(
    store: &S,
    context: TenantContext,
    publisher: UserId,
    draft: DraftRecord,
    reference: ProblemVersionRef,
    source_record: objects::ObjectRecord,
    capabilities: BackendCapabilities,
) -> Result<learning_data_access::PublishedProblemRecord>
where
    S: Store + CatalogStore + CatalogSourceStore,
{
    let expected_question = question_model::QuestionDefinition::from_draft(
        draft.question.clone(),
        reference.problem,
        reference.version,
        webwork_pilot_published_source(),
    );
    let expected_artifact = learning_data_access::PublishedSourceArtifact {
        reference,
        backend: question_model::QuestionBackend::Webwork,
        object: source_record,
    };
    for _ in 0..WEBWORK_PILOT_CONVERGENCE_ATTEMPTS {
        if let Some(existing) = store
            .get_catalog_problem(context, reference)
            .await
            .context("reading deterministic WebWork pilot publication")?
        {
            verify_webwork_pilot_publication(
                store,
                context,
                publisher,
                &existing,
                &expected_question,
                &expected_artifact,
                &capabilities,
            )
            .await?;
            return Ok(existing);
        }
        let Some(saved_draft) =
            ensure_webwork_pilot_draft(store, context, publisher, draft.clone()).await?
        else {
            // A colliding seeder may have consumed the exact draft while this
            // caller reread it. Recheck the immutable publication first.
            continue;
        };
        let command = PublishDraftCommand {
            expected_draft: draft.clone(),
            expected_revision: saved_draft.revision,
            publication: reference,
            published_source: webwork_pilot_published_source(),
            source_artifact: Some(expected_artifact.clone()),
            qti_promotion: None,
            flat_question_promotion: None,
            publisher,
            scope: PublicationScope::Institution,
            byline: question_model::PublicByline::new(vec![
                question_model::PublicAuthorName::new("E2E Instructor".to_string())?,
            ])?,
            capabilities: capabilities.clone(),
        };
        match store.publish_draft(context, publisher, command).await {
            Ok(published) => {
                verify_webwork_pilot_publication(
                    store,
                    context,
                    publisher,
                    &published,
                    &expected_question,
                    &expected_artifact,
                    &capabilities,
                )
                .await?;
                return Ok(published);
            }
            Err(StoreError::AlreadyExists) => continue,
            Err(error) => {
                return Err(error).context("publishing deterministic WebWork pilot E2E question");
            }
        }
    }
    bail!("WebWork pilot publication did not converge after concurrent seed retries")
}

/// Returns a matching draft to publish, or `None` when another seeder consumed
/// it between a conflict and reread so the caller must recheck publication.
pub(super) async fn ensure_webwork_pilot_draft<S>(
    store: &S,
    context: TenantContext,
    publisher: UserId,
    expected: DraftRecord,
) -> Result<Option<learning_data_access::WorkspaceDraft>>
where
    S: Store,
{
    let existing = store
        .get_draft(context, publisher, expected.question.workspace)
        .await
        .context("reading resumable WebWork pilot draft")?;
    match existing {
        Some(stored) => reconcile_webwork_pilot_draft(Some(stored), &expected),
        None => match store
            .upsert_draft(context, publisher, None, expected.clone())
            .await
        {
            Ok(stored) => reconcile_webwork_pilot_draft(Some(stored), &expected),
            Err(StoreError::Conflict | StoreError::AlreadyExists) => {
                let raced = store
                    .get_draft(context, publisher, expected.question.workspace)
                    .await
                    .context("rereading WebWork pilot draft after seed conflict")?;
                reconcile_webwork_pilot_draft(raced, &expected)
            }
            Err(error) => Err(error).context("writing deterministic WebWork pilot E2E draft"),
        },
    }
}

/// The pure post-conflict decision used by the injected draft-create race test.
pub(super) fn reconcile_webwork_pilot_draft(
    stored: Option<learning_data_access::WorkspaceDraft>,
    expected: &DraftRecord,
) -> Result<Option<learning_data_access::WorkspaceDraft>> {
    match stored {
        Some(stored) if stored.record == *expected => Ok(Some(stored)),
        Some(_) => bail!("existing WebWork pilot draft differs from the deterministic seed"),
        None => Ok(None),
    }
}

pub(super) async fn verify_webwork_pilot_publication<S>(
    store: &S,
    context: TenantContext,
    publisher: UserId,
    actual: &learning_data_access::PublishedProblemRecord,
    expected_question: &question_model::QuestionDefinition,
    expected_artifact: &learning_data_access::PublishedSourceArtifact,
    expected_capabilities: &BackendCapabilities,
) -> Result<()>
where
    S: CatalogSourceStore,
{
    let reference = expected_artifact.reference;
    if actual.problem != reference.problem
        || actual.version != reference.version
        || actual.question != *expected_question
        || actual.capabilities != *expected_capabilities
        || actual.scope != PublicationScope::Institution
        || actual.lifecycle != CatalogLifecycle::Published
        || actual.author_ids != vec![publisher]
        || actual.derived_from.is_some()
    {
        bail!("existing WebWork pilot publication differs from the deterministic seed");
    }
    let artifact = store
        .catalog_source_artifact(context, reference)
        .await
        .context("reading immutable WebWork pilot source binding")?
        .ok_or_else(|| {
            anyhow::anyhow!("existing WebWork pilot publication has no source binding")
        })?;
    if artifact != *expected_artifact {
        bail!("existing WebWork pilot source binding differs from the deterministic seed");
    }
    Ok(())
}

pub(super) async fn ensure_webwork_pilot_course<S>(
    store: &S,
    context: TenantContext,
    host_seed_instructor: UserId,
    expected: CourseRecord,
) -> Result<()>
where
    S: Store + SessionStore,
{
    match store
        .get_course(context, expected.id)
        .await
        .context("reading deterministic WebWork pilot course")?
    {
        Some(actual) if webwork_pilot_course_seed_matches(&actual, &expected) => Ok(()),
        Some(_) => bail!("existing WebWork pilot course differs from the deterministic seed"),
        None => {
            let session = ensure_webwork_pilot_sysadmin_session(
                store,
                context.tenant_id(),
                host_seed_instructor,
            )
            .await?;
            store
                .create_course(
                    context,
                    learning_data_access::CreateCourseCommand {
                        course: expected.clone(),
                        authority: CourseCreationAuthority::Sysadmin {
                            actor: host_seed_instructor,
                            session: session.token_hash,
                        },
                    },
                )
                .await
                .context("creating WebWork pilot E2E course")?;
            let actual = store
                .get_course(context, expected.id)
                .await
                .context("reloading created WebWork pilot course")?
                .ok_or_else(|| anyhow::anyhow!("created WebWork pilot course disappeared"))?;
            if !webwork_pilot_course_seed_matches(&actual, &expected) {
                bail!("created WebWork pilot course differs from the deterministic seed");
            }
            Ok(())
        }
    }
}

const WEBWORK_PILOT_SESSION_DISPLAY_NAME: &str = "E2E host seed Sysadmin";
const WEBWORK_PILOT_SESSION_LIFETIME_SECONDS: u32 = 7 * 24 * 60 * 60;

/// Establishes the ordinary, tenant-bound authority used by the host seed.
///
/// The credential is synthetic E2E setup data, never a browser credential. A
/// deterministic hash lets concurrent or repeated seed invocations reuse the
/// same active session; every reused record is checked before its authority is
/// passed to the course-creation broker.
async fn ensure_webwork_pilot_sysadmin_session<S>(
    store: &S,
    tenant: TenantId,
    instructor: UserId,
) -> Result<learning_data_access::SessionRecord>
where
    S: SessionStore,
{
    let token_hash = SessionTokenHash::compute(
        format!(
            "ple-e2e-webwork-course-sysadmin:{}:{}",
            tenant.as_uuid(),
            instructor.as_uuid()
        )
        .as_bytes(),
    );
    let expected_subject = SessionSubject::new(
        tenant,
        instructor,
        WEBWORK_PILOT_SESSION_DISPLAY_NAME,
        vec![UserRole::Sysadmin],
    )
    .expect("fixed WebWork E2E Sysadmin identity is valid");
    let lifetime = SessionLifetime::from_seconds(WEBWORK_PILOT_SESSION_LIFETIME_SECONDS)
        .expect("fixed WebWork E2E session lifetime is positive");

    let record = match store.resolve_session(token_hash).await {
        Ok(Some(record)) => record,
        Ok(None) => match store
            .create_session(token_hash, expected_subject.clone(), lifetime)
            .await
        {
            Ok(record) => record,
            Err(StoreError::AlreadyExists) => store
                .resolve_session(token_hash)
                .await
                .context("rereading deterministic WebWork E2E Sysadmin session after race")?
                .ok_or_else(|| anyhow::anyhow!("WebWork E2E Sysadmin session disappeared"))?,
            Err(error) => return Err(error).context("creating WebWork E2E Sysadmin session"),
        },
        Err(error) => return Err(error).context("resolving WebWork E2E Sysadmin session"),
    };
    if record.token_hash != token_hash
        || record.subject != expected_subject
        || record.expires_at <= record.created_at
    {
        bail!("existing WebWork E2E Sysadmin session differs from the deterministic seed");
    }
    Ok(record)
}

/// Verifies the authored course marker. Canonical membership belongs to its
/// dedicated owner and is never reconstructed from this aggregate.
pub(super) fn webwork_pilot_course_seed_matches(
    actual: &CourseRecord,
    expected: &CourseRecord,
) -> bool {
    actual.id == expected.id
        && actual.tenant == expected.tenant
        && actual.title == expected.title
        && actual.term == expected.term
}

pub(super) async fn ensure_webwork_pilot_assignment<S>(
    store: &S,
    context: TenantContext,
    instructor: UserId,
    expected: AssignmentRecord,
) -> Result<()>
where
    S: Store,
{
    if expected.lifecycle != question_model::AssignmentLifecycle::Published {
        bail!("deterministic demo assignment must converge to Published");
    }
    let mut draft = expected.clone();
    draft.lifecycle = question_model::AssignmentLifecycle::Draft;

    match store
        .get_assignment_for_edit(context, expected.id)
        .await
        .context("reading deterministic WebWork pilot assignment")?
    {
        Some(actual) if actual.record == expected => Ok(()),
        Some(actual) if actual.record == draft => {
            store
                .put_assignment_teaching_settings(
                    context,
                    PutAssignmentTeachingSettingsCommand {
                        actor: instructor,
                        course: expected.course_id,
                        assignment: expected.id,
                        expected_revision: actual.revision,
                        settings: question_model::AssignmentTeachingSettings {
                            lifecycle: question_model::AssignmentLifecycle::Published,
                            instructions: expected.instructions.clone(),
                            base_policy: question_model::BaseAssignmentPolicy::default(),
                        },
                    },
                )
                .await
                .context("publishing deterministic demo assignment")?;
            let published = store
                .get_assignment_for_edit(context, expected.id)
                .await
                .context("reloading published deterministic demo assignment")?
                .context("published deterministic demo assignment disappeared")?;
            if published.record != expected {
                bail!("published demo assignment differs from the deterministic seed");
            }
            Ok(())
        }
        Some(_) => bail!("existing WebWork pilot assignment differs from the deterministic seed"),
        None => {
            let created = match store
                .create_assignment(
                    context,
                    CreateAssignmentCommand {
                        actor: instructor,
                        assignment: draft.clone(),
                        base_policy: question_model::BaseAssignmentPolicy::default(),
                    },
                )
                .await
            {
                Ok(record) => record,
                Err(StoreError::AlreadyExists) => store
                    .get_assignment_for_edit(context, expected.id)
                    .await
                    .context("reading concurrently created WebWork pilot assignment")?
                    .ok_or_else(|| {
                        anyhow::anyhow!(
                            "WebWork pilot assignment creation conflicted without creating its assignment ID; a nested deterministic identity is already in use"
                        )
                    })?,
                Err(error) => return Err(error).context("creating WebWork pilot E2E assignment"),
            };
            if created.record == expected {
                return Ok(());
            }
            if created.record != draft {
                bail!("created demo assignment differs from the deterministic draft seed");
            }
            store
                .put_assignment_teaching_settings(
                    context,
                    PutAssignmentTeachingSettingsCommand {
                        actor: instructor,
                        course: expected.course_id,
                        assignment: expected.id,
                        expected_revision: created.revision,
                        settings: question_model::AssignmentTeachingSettings {
                            lifecycle: question_model::AssignmentLifecycle::Published,
                            instructions: expected.instructions.clone(),
                            base_policy: question_model::BaseAssignmentPolicy::default(),
                        },
                    },
                )
                .await
                .context("publishing newly created deterministic demo assignment")?;
            let published = store
                .get_assignment_for_edit(context, expected.id)
                .await
                .context("reloading newly published deterministic demo assignment")?
                .context("newly published deterministic demo assignment disappeared")?;
            if published.record != expected {
                bail!("published demo assignment differs from the deterministic seed");
            }
            Ok(())
        }
    }
}

pub(super) async fn ensure_webwork_pilot_enrollment<S>(
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
    ensure_named_course_enrollment(
        store,
        context,
        instructor,
        student,
        course,
        assignment,
        "Replica E2E learner",
    )
    .await
}

pub(super) async fn ensure_named_course_enrollment<S>(
    store: &S,
    context: TenantContext,
    instructor: UserId,
    student: UserId,
    course: CourseId,
    assignment: AssignmentId,
    display_name: &str,
) -> Result<AssignmentEnrollment>
where
    S: Store + CourseRosterStore,
{
    // Seed learners are established through the sole roster owner before an
    // instructor explicitly issues their historical entitlement receipt.
    // Neither a course aggregate nor an enrollment supplies current access.
    store
        .upsert_course_member(
            context,
            instructor,
            UpsertCourseMember {
                course,
                user: student,
                display_name: display_name.to_string(),
                roster_contact: None,
            },
        )
        .await
        .context("establishing the WebWork pilot learner through the canonical roster")?;
    match store
        .issue_assignment_entitlement(
            context,
            learning_data_access::MaterializeAssignmentEntitlementCommand::for_instructor_action(
                student,
                course,
                assignment,
                instructor,
                question_model::EntitlementPurpose::InstructorIssue,
            )
            .map_err(anyhow::Error::from)?,
        )
        .await
        .context("materializing WebWork pilot enrollment through current entitlement")?
    {
        learning_data_access::AssignmentEntitlementMaterialization::Granted(materialized) => {
            Ok(materialized.enrollment)
        }
        learning_data_access::AssignmentEntitlementMaterialization::Denied(_) => {
            bail!("WebWork pilot learner is not currently entitled to its assignment")
        }
    }
}

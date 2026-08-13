//! Host-only E2E seed webwork capability.

use super::*;

/// Seeds one licensed, immutable PGML source through the same PostgreSQL
/// catalog binding that the production WebWork backend later resolves. This is
/// an opt-in host tool: no HTTP route or browser-supplied storage value exists.
pub(super) async fn seed_webwork_pilot(arguments: &SeedArguments) -> Result<Manifest> {
    let storage = arguments
        .webwork_pilot
        .as_ref()
        .expect("WebWork pilot storage exists after explicit flag dispatch");
    let pool = learning_data_access::postgres::lazy_pool(&arguments.database_url)
        .context("invalid --database-url for WebWork E2E seed")?;
    learning_data_access::postgres::apply_migrations(&pool)
        .await
        .context("applying embedded migrations for WebWork E2E seed")?;
    let store = learning_data_access::postgres::PostgresStore::new(pool);
    let context = TenantContext::from_authenticated_session(arguments.tenant);
    let ids = WebworkPilotSeedIds::for_tenant(arguments.tenant);
    let reference = ProblemVersionRef {
        problem: ids.problem,
        version: ids.version,
    };
    let source_record =
        put_webwork_pilot_source(&store, context, storage, reference, ids.source_object).await?;
    let draft = DraftRecord {
        tenant: arguments.tenant,
        question: webwork_pilot_draft(ids.workspace),
        revises: None,
        derived_from: None,
    };
    let capabilities = webwork_capabilities();
    let violations = domain::policy::validate_draft_for_publication(&draft.question, &capabilities);
    if !violations.is_empty() {
        bail!("WebWork pilot seed draft failed publication capability admission: {violations:?}");
    }
    ensure_webwork_pilot_publication(
        &store,
        context,
        arguments.instructor,
        draft,
        reference,
        source_record,
        capabilities,
    )
    .await?;
    let course = CourseRecord {
        id: ids.course,
        tenant: arguments.tenant,
        title: "PLE WebWork pilot E2E course".to_string(),
        members: vec![
            CourseMembership {
                user: arguments.instructor,
                role: CourseMembershipRole::Instructor,
            },
            CourseMembership {
                user: arguments.student,
                role: CourseMembershipRole::Student,
            },
        ],
    };
    ensure_webwork_pilot_course(&store, context, course).await?;
    let assignment = AssignmentRecord {
        id: ids.assignment,
        tenant: arguments.tenant,
        course_id: ids.course,
        title: "PLE WebWork pilot E2E assignment".to_string(),
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
    };
    ensure_webwork_pilot_assignment(&store, context, assignment).await?;
    let enrollment = ensure_webwork_pilot_enrollment(
        &store,
        context,
        AssignmentEnrollment {
            id: ids.enrollment,
            tenant: arguments.tenant,
            assignment: ids.assignment,
            user: arguments.student,
            student: StudentId::from_uuid(arguments.student.as_uuid()),
            first_completed_at: None,
            current_grade_run: None,
            best_grade_run: None,
        },
    )
    .await?;
    Ok(Manifest {
        assignment_id: ids.assignment,
        enrollment_id: enrollment.id,
        problem_id: ids.problem,
        version_id: ids.version,
    })
}

pub(super) async fn put_webwork_pilot_source(
    store: &learning_data_access::postgres::PostgresStore,
    context: TenantContext,
    storage: &WebworkPilotStorage,
    reference: ProblemVersionRef,
    object: ObjectId,
) -> Result<objects::ObjectRecord> {
    if objects::Sha256Digest::compute(WEBWORK_PILOT_SOURCE).to_string()
        != WEBWORK_PILOT_SOURCE_SHA256
    {
        bail!("tracked WebWork pilot source digest differs from its recorded provenance");
    }
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
            .authoritative_time(context)
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
) -> Result<()>
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
            return verify_webwork_pilot_publication(
                store,
                context,
                publisher,
                &existing,
                &expected_question,
                &expected_artifact,
                &capabilities,
            )
            .await;
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
            capabilities: capabilities.clone(),
        };
        match store.publish_draft(context, publisher, command).await {
            Ok(published) => {
                return verify_webwork_pilot_publication(
                    store,
                    context,
                    publisher,
                    &published,
                    &expected_question,
                    &expected_artifact,
                    &capabilities,
                )
                .await;
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
        || actual.version_number.value() != 1
        || actual.question != *expected_question
        || actual.capabilities != *expected_capabilities
        || actual.scope != PublicationScope::Institution
        || actual.lifecycle != CatalogLifecycle::Published
        || actual.authors != vec![publisher]
        || actual.previous_version.is_some()
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
    expected: CourseRecord,
) -> Result<()>
where
    S: Store,
{
    match store
        .get_course(context, expected.id)
        .await
        .context("reading deterministic WebWork pilot course")?
    {
        Some(actual) if webwork_pilot_course_seed_matches(&actual, &expected) => Ok(()),
        Some(_) => bail!("existing WebWork pilot course differs from the deterministic seed"),
        None => {
            store
                .upsert_course(context, expected.clone())
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

/// Verifies the authored course seed without claiming ownership of roster
/// membership. The canonical roster transaction may add students after the
/// course is created; seeded identity, title, and required roles remain exact,
/// and an unexpected instructor still fails closed.
pub(super) fn webwork_pilot_course_seed_matches(
    actual: &CourseRecord,
    expected: &CourseRecord,
) -> bool {
    actual.id == expected.id
        && actual.tenant == expected.tenant
        && actual.title == expected.title
        && expected
            .members
            .iter()
            .all(|membership| actual.members.contains(membership))
        && actual.members.iter().all(|membership| {
            expected.members.contains(membership)
                || membership.role == CourseMembershipRole::Student
        })
}

pub(super) async fn ensure_webwork_pilot_assignment<S>(
    store: &S,
    context: TenantContext,
    expected: AssignmentRecord,
) -> Result<()>
where
    S: Store,
{
    match store
        .get_assignment_for_edit(context, expected.id)
        .await
        .context("reading deterministic WebWork pilot assignment")?
    {
        Some(actual) if actual.record == expected => Ok(()),
        Some(_) => bail!("existing WebWork pilot assignment differs from the deterministic seed"),
        None => {
            let created = match store.create_untimed_assignment(context, expected.clone()).await {
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
            if created.record != expected {
                bail!("created WebWork pilot assignment differs from the deterministic seed");
            }
            Ok(())
        }
    }
}

pub(super) async fn ensure_webwork_pilot_enrollment<S>(
    store: &S,
    context: TenantContext,
    expected: AssignmentEnrollment,
) -> Result<AssignmentEnrollment>
where
    S: Store,
{
    match store
        .get_enrollment(context, expected.id)
        .await
        .context("reading deterministic WebWork pilot enrollment")?
    {
        Some(actual) if webwork_pilot_enrollment_identity_matches(&actual, &expected) => Ok(actual),
        Some(_) => bail!("existing WebWork pilot enrollment differs from the deterministic seed"),
        None => match store.create_enrollment(context, expected.clone()).await {
            Ok(()) => store
                .get_enrollment(context, expected.id)
                .await
                .context("reloading created WebWork pilot enrollment")?
                .ok_or_else(|| anyhow::anyhow!("created WebWork pilot enrollment disappeared")),
            Err(StoreError::AlreadyExists) => {
                find_assignment_enrollment(store, context, expected.assignment, expected.user).await
            }
            Err(error) => Err(error).context("creating WebWork pilot E2E enrollment"),
        },
    }
}

pub(super) fn webwork_pilot_enrollment_identity_matches(
    actual: &AssignmentEnrollment,
    expected: &AssignmentEnrollment,
) -> bool {
    actual.id == expected.id
        && actual.tenant == expected.tenant
        && actual.assignment == expected.assignment
        && actual.user == expected.user
}

pub(super) async fn find_assignment_enrollment<S>(
    store: &S,
    context: TenantContext,
    assignment: AssignmentId,
    user: UserId,
) -> Result<AssignmentEnrollment>
where
    S: Store,
{
    let course = store
        .get_assignment(context, assignment)
        .await
        .context("reading pilot assignment after enrollment conflict")?
        .context("pilot assignment disappeared after enrollment conflict")?
        .course_id;
    let page_size = PageSize::new(PageSize::MAX).expect("maximum page size is valid");
    let mut request = PageRequest::first(page_size);
    loop {
        let page = store
            .list_gradebook_rows(context, course, request)
            .await
            .context("reading pilot gradebook after enrollment conflict")?;
        for row in page.items {
            if row.assignment_id != assignment {
                continue;
            }
            let actual = store
                .get_enrollment(context, row.enrollment_id)
                .await
                .context("reading roster-created pilot enrollment")?
                .context("roster-created pilot enrollment disappeared")?;
            if actual.tenant == context.tenant_id()
                && actual.assignment == assignment
                && actual.user == user
            {
                return Ok(actual);
            }
        }
        let Some(cursor) = page.next_cursor else {
            break;
        };
        request = PageRequest::after(cursor, page_size);
    }
    bail!("an existing assignment enrollment could not be resolved for the pilot student")
}

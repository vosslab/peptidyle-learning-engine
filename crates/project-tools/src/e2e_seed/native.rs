//! Host-only E2E seed native publication and verified replay.

use super::*;

/// Seeds the native replica course. The deterministic course and assignment
/// records are a protected replay marker; a fresh run mints its question
/// workspace/problem/version values and a replay reads the exact retained
/// assignment reference back from PostgreSQL.
pub(super) async fn seed_native(arguments: SeedArguments) -> Result<Manifest> {
    let pool = learning_data_access::postgres::lazy_pool(&arguments.database_url)
        .context("invalid --database-url for e2e seed")?;
    if arguments.apply_migrations {
        learning_data_access::postgres::apply_migrations(&pool)
            .await
            .context("applying embedded migrations for e2e seed")?;
    }
    let store = crate::postgres_store::configured_postgres_store(pool)?;
    let context = TenantContext::from_authenticated_session(arguments.tenant);
    let marker = SeedIds::fresh_for_tenant(arguments.tenant);
    seed_native_records(store, context, &arguments, marker).await
}

async fn seed_native_records(
    store: learning_data_access::postgres::PostgresStore,
    context: TenantContext,
    arguments: &SeedArguments,
    marker: SeedIds,
) -> Result<Manifest> {
    let student = arguments.course_student()?;
    let existing_course = store
        .get_course(context, marker.course)
        .await
        .context("reading native seed course marker")?;
    let existing_assignment = store
        .get_assignment_for_edit(context, marker.assignment)
        .await
        .context("reading native seed assignment marker")?;

    let (ids, published) = match seed_replay_state(
        existing_course.is_some(),
        existing_assignment.is_some(),
        "native seed",
    )? {
        SeedReplayState::Fresh => {
            // Persist this marker before immutable publication. Acceptance seed
            // intentionally retains its independent reset-only contract.
            ensure_webwork_pilot_course(
                &store,
                context,
                arguments.instructor,
                native_course(arguments, marker.course),
            )
            .await?;
            publish_fresh_native(&store, context, arguments, marker).await?
        }
        SeedReplayState::Replay => {
            let course = existing_course.expect("replay state has a course marker");
            let assignment = existing_assignment.expect("replay state has an assignment marker");
            let reference = native_assignment_reference(&assignment.record, marker)?;
            let published = store
                .get_catalog_problem(context, reference)
                .await
                .context("reading retained native publication")?
                .context("native seed assignment refers to a missing publication")?;
            let ids = SeedIds::from_published(arguments.tenant, &published);
            let expected_course = native_course(arguments, ids.course);
            if !webwork_pilot_course_seed_matches(&course, &expected_course) {
                bail!("native seed course marker differs from the reviewed host seed");
            }
            verify_native_publication(
                &store,
                context,
                &published,
                arguments.instructor,
                replica_native_draft(ids.workspace),
            )
            .await?;
            let expected_assignment = native_assignment(arguments, ids, reference);
            if assignment.record != expected_assignment {
                bail!("native seed assignment differs from the retained immutable publication");
            }
            (ids, published)
        }
    };
    let reference = ProblemVersionRef {
        problem: published.problem,
        version: published.version,
    };
    let assignment = native_assignment(arguments, ids, reference);
    ensure_webwork_pilot_course(
        &store,
        context,
        arguments.instructor,
        native_course(arguments, ids.course),
    )
    .await?;
    ensure_webwork_pilot_assignment(&store, context, arguments.instructor, assignment.clone())
        .await?;
    let enrollment = ensure_named_course_enrollment(
        &store,
        context,
        arguments.instructor,
        student,
        ids.course,
        ids.assignment,
        "Replica E2E learner",
    )
    .await
    .context("creating native seed enrollment")?;

    if arguments.exercise_scoring {
        exercise_scoring_generation(
            &store,
            context,
            arguments.instructor,
            student,
            ids,
            assignment,
        )
        .await?;
    }
    Ok(Manifest {
        course_id: ids.course,
        assignment_id: ids.assignment,
        enrollment_id: enrollment.id,
        question_id: published.question_id,
        problem_id: published.problem,
        version_id: published.version,
    })
}

async fn publish_fresh_native(
    store: &learning_data_access::postgres::PostgresStore,
    context: TenantContext,
    arguments: &SeedArguments,
    ids: SeedIds,
) -> Result<(SeedIds, learning_data_access::PublishedProblemRecord)> {
    let draft = DraftRecord {
        tenant: arguments.tenant,
        question: replica_native_draft(ids.workspace),
        derived_from: None,
    };
    let capabilities = native_capabilities()?;
    let violations = domain::policy::validate_draft_for_publication(&draft.question, &capabilities);
    if !violations.is_empty() {
        bail!("native E2E seed draft failed publication capability admission: {violations:?}");
    }
    let saved = store
        .upsert_draft(context, arguments.instructor, None, draft.clone())
        .await
        .context("writing fresh native E2E draft")?;
    let published = store
        .publish_draft(
            context,
            arguments.instructor,
            PublishDraftCommand {
                expected_draft: draft.clone(),
                expected_revision: saved.revision,
                publication: ProblemVersionRef {
                    problem: ids.problem,
                    version: ids.version,
                },
                published_source: QuestionSource::Native {
                    family: "peptide_bond_geometry".to_string(),
                },
                source_artifact: None,
                qti_promotion: None,
                flat_question_promotion: None,
                publisher: arguments.instructor,
                scope: PublicationScope::Institution,
                byline: question_model::PublicByline::new(vec![
                    question_model::PublicAuthorName::new("E2E Instructor".to_string())?,
                ])?,
                capabilities,
            },
        )
        .await
        .context("publishing fresh native E2E question")?;
    verify_native_publication(
        store,
        context,
        &published,
        arguments.instructor,
        draft.question,
    )
    .await?;
    Ok((ids, published))
}

pub(super) fn native_course(arguments: &SeedArguments, course: CourseId) -> CourseRecord {
    CourseRecord {
        id: course,
        tenant: arguments.tenant,
        title: "PLE replica E2E course".to_string(),
        term: question_model::CourseTerm::from_parts("2026-08-24", "2026-12-18", "America/Chicago")
            .expect("explicit fixture course term"),
    }
}

pub(super) fn native_assignment(
    arguments: &SeedArguments,
    ids: SeedIds,
    reference: ProblemVersionRef,
) -> AssignmentRecord {
    AssignmentRecord {
        id: ids.assignment,
        tenant: arguments.tenant,
        course_id: ids.course,
        title: "PLE replica E2E assignment".to_string(),
        lifecycle: question_model::AssignmentLifecycle::Published,
        instructions: question_model::AssignmentInstructions::try_new(
            "Work through the peptide-bond geometry evidence before submitting.".to_string(),
        )
        .expect("native seed instructions are valid"),
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

fn native_assignment_reference(
    assignment: &AssignmentRecord,
    ids: SeedIds,
) -> Result<ProblemVersionRef> {
    let Some(item) = assignment.items.first() else {
        bail!("native seed assignment has no fixed publication item");
    };
    if assignment.items.len() != 1 || item.id != ids.assignment_item || item.position != 0 {
        bail!("native seed assignment does not retain one reviewed fixed item");
    }
    Ok(item.reference)
}

async fn verify_native_publication<S>(
    store: &S,
    context: TenantContext,
    record: &learning_data_access::PublishedProblemRecord,
    publisher: UserId,
    draft: DraftQuestionDefinition,
) -> Result<()>
where
    S: CatalogSourceStore,
{
    let source = QuestionSource::Native {
        family: "peptide_bond_geometry".to_string(),
    };
    let expected = question_model::QuestionDefinition::from_draft(
        draft,
        record.problem,
        record.version,
        source,
    );
    let canonical_question_id: question_model::QuestionId =
        record.question_id.to_string().parse().map_err(|error| {
            anyhow::anyhow!("native retained publication has an invalid Question ID: {error}")
        })?;
    if canonical_question_id != record.question_id
        || record.question != expected
        || record.capabilities != native_capabilities()?
        || record.scope != PublicationScope::Institution
        || record.lifecycle != CatalogLifecycle::Published
        || record.author_ids.as_slice() != [publisher]
        || record.derived_from.is_some()
    {
        bail!("native retained publication differs from the reviewed immutable source");
    }
    let reference = ProblemVersionRef {
        problem: record.problem,
        version: record.version,
    };
    if store
        .catalog_source_artifact(context, reference)
        .await
        .context("reading native retained publication source binding")?
        .is_some()
    {
        bail!("native retained publication unexpectedly binds a private source artifact");
    }
    Ok(())
}

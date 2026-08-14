//! Host-only E2E seed native capability.

use super::*;

pub(super) async fn seed_native(arguments: SeedArguments) -> Result<Manifest> {
    let pool = learning_data_access::postgres::lazy_pool(&arguments.database_url)
        .context("invalid --database-url for e2e seed")?;
    if arguments.apply_migrations {
        learning_data_access::postgres::apply_migrations(&pool)
            .await
            .context("applying embedded migrations for e2e seed")?;
    }
    let store = question_id_store(pool)?;
    let context = TenantContext::from_authenticated_session(arguments.tenant);
    let ids = SeedIds::for_tenant(arguments.tenant);
    let draft = DraftRecord {
        tenant: arguments.tenant,
        question: native_draft(ids.workspace),
        revises: None,
        derived_from: None,
    };
    let capabilities = native_capabilities()?;
    let violations = domain::policy::validate_draft_for_publication(&draft.question, &capabilities);
    if !violations.is_empty() {
        bail!("native E2E seed draft failed publication capability admission: {violations:?}");
    }

    let saved_draft = store
        .upsert_draft(context, arguments.instructor, None, draft.clone())
        .await
        .context("writing deterministic native E2E draft")?;
    store
        .publish_draft(
            context,
            arguments.instructor,
            PublishDraftCommand {
                expected_draft: draft,
                expected_revision: saved_draft.revision,
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
                capabilities,
            },
        )
        .await
        .context("publishing deterministic native E2E question")?;
    store
        .upsert_course(
            context,
            CourseRecord {
                id: ids.course,
                tenant: arguments.tenant,
                title: "PLE replica E2E course".to_string(),
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
            },
        )
        .await
        .context("creating E2E course")?;
    let assignment = AssignmentRecord {
        id: ids.assignment,
        tenant: arguments.tenant,
        course_id: ids.course,
        title: "PLE replica E2E assignment".to_string(),
        items: vec![AssignmentItem {
            id: ids.assignment_item,
            reference: ProblemVersionRef {
                problem: ids.problem,
                version: ids.version,
            },
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
    store
        .create_untimed_assignment(context, assignment.clone())
        .await
        .context("creating E2E assignment")?;
    let reloaded = store
        .get_assignment_for_edit(context, ids.assignment)
        .await
        .context("reloading normalized E2E assignment")?
        .ok_or_else(|| anyhow::anyhow!("normalized E2E assignment was not readable"))?;
    if reloaded.record != assignment {
        bail!("normalized E2E assignment did not round-trip exactly");
    }
    let replaced = store
        .replace_assignment_preserving_timing(
            context,
            ids.course,
            ids.assignment,
            reloaded.revision,
            AssignmentUpdate {
                title: assignment.title.clone(),
                items: assignment.items.clone(),
                selection_groups: assignment.selection_groups.clone(),
                policies: assignment.policies,
            },
        )
        .await
        .context("replacing normalized E2E assignment")?;
    if replaced.record != assignment {
        bail!("normalized E2E assignment replacement did not round-trip exactly");
    }
    store
        .create_enrollment(
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
        .await
        .context("creating E2E enrollment")?;

    if arguments.exercise_scoring {
        exercise_scoring_generation(
            &store,
            context,
            arguments.instructor,
            arguments.student,
            ids,
            assignment,
        )
        .await?;
    }
    if arguments.exercise_timing {
        exercise_assignment_timing(
            &store,
            context,
            arguments.instructor,
            arguments.student,
            ids,
        )
        .await?;
    }

    Ok(Manifest {
        assignment_id: ids.assignment,
        enrollment_id: ids.enrollment,
        problem_id: ids.problem,
        version_id: ids.version,
    })
}

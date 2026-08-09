//! Host-only durable seed data for the replica restart E2E gate.
//!
//! This command is deliberately not an API feature. It applies the embedded
//! migrations and uses the production PostgreSQL store contract, then writes
//! only non-secret identifiers to stdout for the browser E2E runner.

use std::{collections::BTreeMap, time::Duration};

use adapter_native::NativeAdapter;
use anyhow::{Context, Result, bail};
use learning_data_access::{
    AssignmentExceptionLimit, AssignmentExceptionTimestamp, AssignmentPolicyException,
    AssignmentPolicyExceptionTarget, AssignmentRecord, AssignmentScoringCommitOutcome,
    AssignmentScoringWorkerCommand, AssignmentScoringWorkerStore, AssignmentUpdate,
    AttemptAutoSubmitCommitOutcome, AttemptAutoSubmitWorkerCommand, AttemptAutoSubmitWorkerStore,
    AttemptSupportActionId, AuthoritativeTimeStore, CatalogStore, ClearAttemptCommand,
    CourseGroupRecord, CourseRecord, DeleteAndRegradeAssignmentItemCommand,
    DeleteAssignmentPolicyExceptionCommand, DraftRecord, ForceSubmitAttemptCommand,
    IssueQuestionAttemptCommand, JobClaimFilter, JobLeaseDuration, JobPayload, JobStore,
    PageRequest, PageSize, PublishDraftCommand, PutCourseGroupCommand,
    SetAssignmentPolicyExceptionCommand, Store, StoreError, SubmissionIdempotencyKey,
    SubmitQuestionAttemptCommand, TenantContext, UpdateAssignmentTimingCommand,
};
use question_model::answer::SelectionCardinality;
use question_model::capability::BackendCapabilities;
use question_model::definition::{
    DraftQuestionDefinition, DraftQuestionSource, GradingDefinition, QuestionMetadata,
    QuestionSource,
};
use question_model::envelope::ContentBlock;
use question_model::generation::{GeneratorReference, ParameterSpec, RandomizationDefinition};
use question_model::response::{ChoiceId, ChoiceOption, ResponseDefinition};
use question_model::run_policy::{
    AttemptPolicy, CompletionRequirement, ContinuedPractice, FeedbackDisclosure, GradePolicy,
    RunPolicies, TimingPolicy, VariationPolicy,
};
use question_model::taxonomy::{License, Tag};
use question_model::{
    ActivityTimestamp, AssignmentDeliveryState, AssignmentEnrollment, AssignmentId, AssignmentItem,
    AssignmentItemId, AssignmentPolicyExceptionId, AssignmentScoringMode, AssignmentTimingPolicy,
    AttemptProvenance, AttemptResult, AttemptStatus, CourseGroupId, CourseId, CourseMembership,
    CourseMembershipRole, EnrollmentId, FeedbackContent, ImplementationVersion, PointValue,
    ProblemId, ProblemVersionRef, PublicationScope, QuestionAttemptId, RunId, StudentId, TenantId,
    UserId, VersionId, WorkspaceId,
};
use serde::Serialize;
use sha2::{Digest, Sha256};
use uuid::Uuid;

const USAGE: &str = "usage: cargo tools e2e-seed --database-url <URL> --tenant <UUID> (--instructor <UUID>|--user <UUID>) --student <UUID> --apply-migrations [--exercise-scoring] [--exercise-timing]";

/// Non-secret identifiers the replica E2E runner needs to start an assignment.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct Manifest {
    assignment_id: AssignmentId,
    enrollment_id: EnrollmentId,
    problem_id: ProblemId,
    version_id: VersionId,
}

struct SeedArguments {
    database_url: String,
    tenant: TenantId,
    instructor: UserId,
    student: UserId,
    apply_migrations: bool,
    exercise_scoring: bool,
    exercise_timing: bool,
}

/// Dispatches the host-only command without adding an API route or a service.
pub fn run(args: &[String]) -> Result<()> {
    if matches!(args, [flag] if flag == "--help" || flag == "-h") {
        println!("{USAGE}");
        return Ok(());
    }
    let arguments = parse_arguments(args)?;
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .context("creating the e2e seed runtime")?;
    let manifest = runtime.block_on(seed(arguments))?;
    println!("{}", serde_json::to_string(&manifest)?);
    Ok(())
}

fn parse_arguments(args: &[String]) -> Result<SeedArguments> {
    let mut database_url = None;
    let mut tenant = None;
    let mut instructor = None;
    let mut student = None;
    let mut apply_migrations = false;
    let mut exercise_scoring = false;
    let mut exercise_timing = false;
    let mut index = 0;
    while index < args.len() {
        let flag = &args[index];
        index += 1;
        if flag == "--exercise-scoring" && !exercise_scoring {
            exercise_scoring = true;
            continue;
        }
        if flag == "--exercise-timing" && !exercise_timing {
            exercise_timing = true;
            continue;
        }
        if flag == "--apply-migrations" && !apply_migrations {
            apply_migrations = true;
            continue;
        }
        let Some(value) = args.get(index) else {
            bail!("{flag} requires a value; {USAGE}");
        };
        index += 1;
        match flag.as_str() {
            "--database-url" if database_url.is_none() => database_url = Some(value.clone()),
            "--tenant" if tenant.is_none() => tenant = Some(parse_tenant(value, "tenant")?),
            "--instructor" | "--user" if instructor.is_none() => {
                instructor = Some(parse_user(value, "instructor")?);
            }
            "--student" if student.is_none() => student = Some(parse_user(value, "student")?),
            _ => bail!("unknown, duplicate, or misplaced argument {flag}; {USAGE}"),
        }
    }
    let arguments = SeedArguments {
        database_url: database_url
            .ok_or_else(|| anyhow::anyhow!("--database-url is required; {USAGE}"))?,
        tenant: tenant.ok_or_else(|| anyhow::anyhow!("--tenant is required; {USAGE}"))?,
        instructor: instructor
            .ok_or_else(|| anyhow::anyhow!("--instructor is required; {USAGE}"))?,
        student: student.ok_or_else(|| anyhow::anyhow!("--student is required; {USAGE}"))?,
        apply_migrations,
        exercise_scoring,
        exercise_timing,
    };
    if arguments.instructor == arguments.student {
        bail!("--instructor and --student must identify different users for the E2E course");
    }
    if !arguments.apply_migrations {
        bail!("--apply-migrations is required because e2e-seed changes database schema; {USAGE}");
    }
    Ok(arguments)
}

fn parse_tenant(value: &str, name: &str) -> Result<TenantId> {
    Ok(TenantId::from_uuid(
        Uuid::parse_str(value).with_context(|| format!("{name} must be a UUID"))?,
    ))
}

fn parse_user(value: &str, name: &str) -> Result<UserId> {
    Ok(UserId::from_uuid(
        Uuid::parse_str(value).with_context(|| format!("{name} must be a UUID"))?,
    ))
}

async fn seed(arguments: SeedArguments) -> Result<Manifest> {
    let pool = learning_data_access::postgres::lazy_pool(&arguments.database_url)
        .context("invalid --database-url for e2e seed")?;
    if arguments.apply_migrations {
        learning_data_access::postgres::apply_migrations(&pool)
            .await
            .context("applying embedded migrations for e2e seed")?;
    }
    let store = learning_data_access::postgres::PostgresStore::new(pool);
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
        .create_assignment(context, assignment.clone())
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
        .replace_assignment(
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

async fn exercise_assignment_timing(
    store: &learning_data_access::postgres::PostgresStore,
    context: TenantContext,
    instructor: UserId,
    student: UserId,
    ids: SeedIds,
) -> Result<()> {
    let reference = ProblemVersionRef {
        problem: ids.problem,
        version: ids.version,
    };
    let timing_assignment = AssignmentRecord {
        id: ids.timing_assignment,
        tenant: context.tenant_id(),
        course_id: ids.course,
        title: "PLE mutable timing acceptance".to_string(),
        items: [
            ids.timing_assignment_item_one,
            ids.timing_assignment_item_two,
            ids.timing_assignment_item_three,
        ]
        .into_iter()
        .enumerate()
        .map(|(position, id)| AssignmentItem {
            id,
            reference,
            position: u32::try_from(position).expect("three positions fit"),
            points_possible: PointValue::from_whole(1),
            delivery_state: AssignmentDeliveryState::Active,
            scoring_mode: AssignmentScoringMode::Normal,
        })
        .collect(),
        selection_groups: Vec::new(),
        policies: RunPolicies {
            completion: CompletionRequirement::AnswerAll,
            grade: GradePolicy::Highest,
            continued_practice: ContinuedPractice::Unlimited,
            variation: VariationPolicy::NewSeeds,
        },
    };
    let created = store
        .create_assignment(context, timing_assignment)
        .await
        .context("creating mutable timing acceptance assignment")?;
    store
        .create_enrollment(
            context,
            AssignmentEnrollment {
                id: ids.timing_enrollment,
                tenant: context.tenant_id(),
                assignment: ids.timing_assignment,
                user: student,
                student: StudentId::from_uuid(student.as_uuid()),
                first_completed_at: None,
                current_grade_run: None,
                best_grade_run: None,
            },
        )
        .await
        .context("creating mutable timing acceptance enrollment")?;
    let hidden = AssignmentTimingPolicy {
        visible: false,
        ..AssignmentTimingPolicy::default()
    };
    let initial_command = UpdateAssignmentTimingCommand {
        actor: instructor,
        course: ids.course,
        assignment: ids.timing_assignment,
        expected_revision: created.revision,
        policy: hidden,
    };
    if store
        .update_assignment_timing(
            context,
            UpdateAssignmentTimingCommand {
                actor: student,
                ..initial_command
            },
        )
        .await
        != Err(StoreError::NotFound)
    {
        bail!("student changed mutable assignment timing");
    }
    let hidden = store
        .update_assignment_timing(context, initial_command)
        .await
        .context("hiding timing acceptance assignment")?;
    if store
        .start_or_resume_run(context, student, ids.timing_assignment, ids.timing_run)
        .await
        != Err(StoreError::NotFound)
    {
        bail!("hidden assignment allowed a new student run");
    }
    let now = store
        .authoritative_time(context)
        .await
        .context("reading database clock for availability acceptance")?;
    let future = AssignmentTimingPolicy {
        available_at: Some(add_millis(now, 60_000)?),
        ..AssignmentTimingPolicy::default()
    };
    let future = store
        .update_assignment_timing(
            context,
            UpdateAssignmentTimingCommand {
                expected_revision: hidden.revision,
                policy: future,
                ..initial_command
            },
        )
        .await
        .context("setting future assignment availability")?;
    if !matches!(
        store
            .start_or_resume_run(context, student, ids.timing_assignment, ids.timing_run)
            .await,
        Err(StoreError::InvalidRecord(_))
    ) {
        bail!("future assignment availability allowed an early run");
    }
    let open_now = store
        .authoritative_time(context)
        .await
        .context("reading database clock for timing acceptance")?;
    let open_policy = AssignmentTimingPolicy {
        closes_at: Some(add_millis(open_now, 60_000)?),
        ..AssignmentTimingPolicy::default()
    };
    let open = store
        .update_assignment_timing(
            context,
            UpdateAssignmentTimingCommand {
                expected_revision: future.revision,
                policy: open_policy,
                ..initial_command
            },
        )
        .await
        .context("opening bounded timing acceptance assignment")?;
    if store
        .update_assignment_timing(
            context,
            UpdateAssignmentTimingCommand {
                expected_revision: future.revision,
                policy: open_policy,
                ..initial_command
            },
        )
        .await
        .context("replaying exact timing update")?
        != open
    {
        bail!("exact timing replay changed the revision or queue");
    }
    let run = store
        .start_or_resume_run(context, student, ids.timing_assignment, ids.timing_run)
        .await
        .context("starting mutable timing acceptance run")?;
    let implementation = |name: &str| ImplementationVersion {
        id: name.to_string(),
        version: "timing-acceptance-1".to_string(),
    };
    let issue = |attempt, position, seed| IssueQuestionAttemptCommand {
        actor: student,
        attempt,
        run: run.id,
        assignment_position: position,
        problem: ids.problem,
        question_version: ids.version,
        seed,
        parameter_hash: format!("database-timing-parameters-{position}"),
        provenance: AttemptProvenance {
            adapter: implementation("native"),
            renderer: None,
            generator: None,
            source_artifact: None,
            asset_objects: Vec::new(),
            grading: implementation("native"),
            rendered_question_sha256: format!("database-timing-render-{position}"),
        },
        prefetched: None,
        predecessor_submission: None,
    };
    let first = store
        .issue_or_resume_question_attempt(context, issue(ids.timing_attempt_one, 0, 31))
        .await
        .context("issuing shortened timing attempt")?;
    let shorten_at = store
        .authoritative_time(context)
        .await
        .context("reading shortening clock")?;
    let shortened = store
        .update_assignment_timing(
            context,
            UpdateAssignmentTimingCommand {
                expected_revision: open.revision,
                policy: AssignmentTimingPolicy {
                    closes_at: Some(shorten_at),
                    ..AssignmentTimingPolicy::default()
                },
                ..initial_command
            },
        )
        .await
        .context("shortening active timing below elapsed time")?;
    assert_auto_submitted_without_work(store, context, first.id).await?;

    let second_now = store.authoritative_time(context).await?;
    let second_due = add_millis(second_now, 200)?;
    let second_policy = AssignmentTimingPolicy {
        closes_at: Some(second_due),
        ..AssignmentTimingPolicy::default()
    };
    let second_policy = store
        .update_assignment_timing(
            context,
            UpdateAssignmentTimingCommand {
                expected_revision: shortened.revision,
                policy: second_policy,
                ..initial_command
            },
        )
        .await
        .context("scheduling natural auto-submit")?;
    let second = store
        .issue_or_resume_question_attempt(context, issue(ids.timing_attempt_two, 1, 32))
        .await
        .context("issuing natural auto-submit attempt")?;
    tokio::time::sleep(Duration::from_millis(250)).await;
    let due = store
        .claim_next_job(&JobClaimFilter::all(), JobLeaseDuration::from_seconds(30)?)
        .await
        .context("claiming natural auto-submit")?
        .ok_or_else(|| anyhow::anyhow!("natural auto-submit was not claimable"))?;
    let due_generation = auto_submit_generation(&due.payload, second.id)?;
    if store
        .commit_attempt_auto_submit(
            context,
            AttemptAutoSubmitWorkerCommand {
                job: due.id,
                lease: due.lease_token,
                attempt: second.id,
                timing_generation: due_generation,
            },
        )
        .await
        .context("committing natural auto-submit")?
        != AttemptAutoSubmitCommitOutcome::AutoSubmitted
    {
        bail!("natural deadline did not auto-submit the attempt");
    }
    assert_auto_submitted_without_work(store, context, second.id).await?;

    let third_now = store.authoritative_time(context).await?;
    let third_due = add_millis(third_now, 200)?;
    let third_policy = store
        .update_assignment_timing(
            context,
            UpdateAssignmentTimingCommand {
                expected_revision: second_policy.revision,
                policy: AssignmentTimingPolicy {
                    closes_at: Some(third_due),
                    ..AssignmentTimingPolicy::default()
                },
                ..initial_command
            },
        )
        .await
        .context("scheduling leased extension acceptance")?;
    let third = store
        .issue_or_resume_question_attempt(context, issue(ids.timing_attempt_three, 2, 33))
        .await
        .context("issuing leased extension attempt")?;
    tokio::time::sleep(Duration::from_millis(250)).await;
    let stale = store
        .claim_next_job(&JobClaimFilter::all(), JobLeaseDuration::from_seconds(30)?)
        .await
        .context("claiming old timing generation")?
        .ok_or_else(|| anyhow::anyhow!("old timing generation was not claimable"))?;
    let stale_generation = auto_submit_generation(&stale.payload, third.id)?;
    let extension_now = store.authoritative_time(context).await?;
    let extended = store
        .update_assignment_timing(
            context,
            UpdateAssignmentTimingCommand {
                expected_revision: third_policy.revision,
                policy: AssignmentTimingPolicy {
                    closes_at: Some(add_millis(extension_now, 60_000)?),
                    ..AssignmentTimingPolicy::default()
                },
                ..initial_command
            },
        )
        .await
        .context("extending a leased timing generation")?;
    if store
        .commit_attempt_auto_submit(
            context,
            AttemptAutoSubmitWorkerCommand {
                job: stale.id,
                lease: stale.lease_token,
                attempt: third.id,
                timing_generation: stale_generation,
            },
        )
        .await
        .context("resolving leased old timing generation")?
        != AttemptAutoSubmitCommitOutcome::Rescheduled
    {
        bail!("leased old timing generation was not rescheduled");
    }
    if store
        .claim_next_job(&JobClaimFilter::all(), JobLeaseDuration::from_seconds(30)?)
        .await
        .context("checking extended deadline queue")?
        .is_some()
    {
        bail!("extended deadline remained immediately claimable");
    }
    let final_now = store.authoritative_time(context).await?;
    let closed = store
        .update_assignment_timing(
            context,
            UpdateAssignmentTimingCommand {
                expected_revision: extended.revision,
                policy: AssignmentTimingPolicy {
                    closes_at: Some(final_now),
                    ..AssignmentTimingPolicy::default()
                },
                ..initial_command
            },
        )
        .await
        .context("shortening the rescheduled deadline")?;
    assert_auto_submitted_without_work(store, context, third.id).await?;

    let exception_now = store
        .authoritative_time(context)
        .await
        .context("reading exception acceptance clock")?;
    let base = store
        .update_assignment_timing(
            context,
            UpdateAssignmentTimingCommand {
                expected_revision: closed.revision,
                policy: AssignmentTimingPolicy {
                    available_at: Some(add_millis(exception_now, 60_000)?),
                    closes_at: Some(add_millis(exception_now, 60_000)?),
                    time_limit_seconds: Some(1),
                    attempt_limit: Some(1),
                    ..AssignmentTimingPolicy::default()
                },
                ..initial_command
            },
        )
        .await
        .context("setting restrictive base policy for exception acceptance")?;
    let group = store
        .put_course_group(
            context,
            PutCourseGroupCommand {
                actor: instructor,
                expected_revision: None,
                record: CourseGroupRecord {
                    id: ids.timing_group,
                    tenant: context.tenant_id(),
                    course: ids.course,
                    title: "Database extended testing".to_string(),
                    members: vec![student],
                },
            },
        )
        .await
        .context("creating timing acceptance group")?;
    let group_exception = store
        .set_assignment_policy_exception(
            context,
            SetAssignmentPolicyExceptionCommand {
                actor: instructor,
                course: ids.course,
                assignment: ids.timing_assignment,
                expected_revision: base.revision,
                exception: AssignmentPolicyException {
                    id: ids.timing_group_exception,
                    target: AssignmentPolicyExceptionTarget::CourseGroup(ids.timing_group),
                    available_at: Some(AssignmentExceptionTimestamp::Unrestricted),
                    closes_at: Some(AssignmentExceptionTimestamp::At(add_millis(
                        exception_now,
                        120_000,
                    )?)),
                    time_limit_seconds: Some(AssignmentExceptionLimit::Value(60)),
                    attempt_limit: Some(AssignmentExceptionLimit::Value(2)),
                },
            },
        )
        .await
        .context("creating group timing exception")?;
    let student_record = StudentId::from_uuid(student.as_uuid());
    let student_exception = store
        .set_assignment_policy_exception(
            context,
            SetAssignmentPolicyExceptionCommand {
                actor: instructor,
                course: ids.course,
                assignment: ids.timing_assignment,
                expected_revision: group_exception.assignment_revision,
                exception: AssignmentPolicyException {
                    id: ids.timing_student_exception,
                    target: AssignmentPolicyExceptionTarget::Student(student_record),
                    available_at: Some(AssignmentExceptionTimestamp::At(exception_now)),
                    closes_at: Some(AssignmentExceptionTimestamp::At(add_millis(
                        exception_now,
                        90_000,
                    )?)),
                    time_limit_seconds: Some(AssignmentExceptionLimit::Value(2)),
                    attempt_limit: Some(AssignmentExceptionLimit::Value(3)),
                },
            },
        )
        .await
        .context("creating direct student timing exception")?;
    let resolved = store
        .resolve_assignment_timing(context, ids.timing_assignment, student_record)
        .await
        .context("resolving combined timing exceptions")?
        .ok_or_else(|| anyhow::anyhow!("timing exception enrollment disappeared"))?;
    if resolved.policy.available_at.is_some()
        || resolved.policy.closes_at != Some(add_millis(exception_now, 120_000)?)
        || resolved.policy.time_limit_seconds != Some(60)
        || resolved.policy.attempt_limit != Some(3)
        || resolved.contributors
            != vec![
                AssignmentPolicyExceptionTarget::Student(student_record),
                AssignmentPolicyExceptionTarget::CourseGroup(ids.timing_group),
            ]
    {
        bail!("student and group timing exceptions did not resolve dimension-wise");
    }
    let exception_run = store
        .start_or_resume_run(
            context,
            student,
            ids.timing_assignment,
            ids.timing_exception_run,
        )
        .await
        .context("starting run opened by timing exception")?;
    let exception_attempt = store
        .issue_or_resume_question_attempt(
            context,
            IssueQuestionAttemptCommand {
                actor: student,
                attempt: ids.timing_exception_attempt,
                run: exception_run.id,
                assignment_position: 0,
                problem: ids.problem,
                question_version: ids.version,
                seed: 34,
                parameter_hash: "database-timing-exception-parameters".to_string(),
                provenance: AttemptProvenance {
                    adapter: implementation("native"),
                    renderer: None,
                    generator: None,
                    source_artifact: None,
                    asset_objects: Vec::new(),
                    grading: implementation("native"),
                    rendered_question_sha256: "database-timing-exception-render".to_string(),
                },
                prefetched: None,
                predecessor_submission: None,
            },
        )
        .await
        .context("issuing timing exception attempt")?;
    let recorded = store
        .get_attempt_resolved_timing(context, exception_attempt.id)
        .await
        .context("reading recorded timing exception resolution")?
        .ok_or_else(|| anyhow::anyhow!("attempt timing resolution was not recorded"))?;
    if recorded.policy != resolved.policy || recorded.contributors != resolved.contributors {
        bail!("issued attempt did not record its effective timing exceptions");
    }
    tokio::time::sleep(Duration::from_millis(2_100)).await;
    store
        .upsert_course(
            context,
            CourseRecord {
                id: ids.course,
                tenant: context.tenant_id(),
                title: "PLE replica E2E course".to_string(),
                members: vec![CourseMembership {
                    user: instructor,
                    role: CourseMembershipRole::Instructor,
                }],
            },
        )
        .await
        .context("removing accommodated student from course group membership")?;
    assert_auto_submitted_without_work(store, context, exception_attempt.id).await?;
    let terminal = store
        .get_attempt_resolved_timing(context, exception_attempt.id)
        .await
        .context("reading terminal timing exception resolution")?
        .ok_or_else(|| anyhow::anyhow!("terminal attempt lost its timing resolution"))?;
    if terminal.policy.time_limit_seconds != Some(2)
        || terminal.contributors != vec![AssignmentPolicyExceptionTarget::Student(student_record)]
    {
        bail!("course membership removal did not immediately re-resolve active timing");
    }
    let after_student = store
        .delete_assignment_policy_exception(
            context,
            DeleteAssignmentPolicyExceptionCommand {
                actor: instructor,
                course: ids.course,
                assignment: ids.timing_assignment,
                expected_revision: student_exception.assignment_revision,
                exception: ids.timing_student_exception,
            },
        )
        .await
        .context("deleting direct student timing exception")?;
    store
        .delete_assignment_policy_exception(
            context,
            DeleteAssignmentPolicyExceptionCommand {
                actor: instructor,
                course: ids.course,
                assignment: ids.timing_assignment,
                expected_revision: after_student,
                exception: ids.timing_group_exception,
            },
        )
        .await
        .context("deleting group timing exception")?;
    if store
        .get_course_group(context, ids.timing_group)
        .await
        .context("reading timing group after course membership removal")?
        .is_none_or(|stored| stored.revision != group.revision || !stored.record.members.is_empty())
    {
        bail!("course membership removal did not preserve an empty revisioned group");
    }
    Ok(())
}

fn add_millis(value: ActivityTimestamp, millis: i64) -> Result<ActivityTimestamp> {
    Ok(ActivityTimestamp::from_unix_millis(
        value
            .as_unix_millis()
            .checked_add(millis)
            .ok_or_else(|| anyhow::anyhow!("timing acceptance timestamp overflow"))?,
    ))
}

fn auto_submit_generation(payload: &JobPayload, attempt: QuestionAttemptId) -> Result<u64> {
    match payload {
        JobPayload::AutoSubmitAttempt {
            attempt: queued,
            timing_generation,
        } if *queued == attempt => Ok(*timing_generation),
        _ => bail!("timing acceptance claimed another job family or attempt"),
    }
}

async fn assert_auto_submitted_without_work(
    store: &learning_data_access::postgres::PostgresStore,
    context: TenantContext,
    attempt: QuestionAttemptId,
) -> Result<()> {
    let current = store
        .get_question_attempt(context, attempt)
        .await
        .context("reading auto-submitted attempt")?
        .ok_or_else(|| anyhow::anyhow!("auto-submitted attempt disappeared"))?;
    if current.status != AttemptStatus::AutoSubmitted
        || current.response.is_some()
        || current.result.is_some()
        || current.timer.submitted_at.is_none()
    {
        bail!("auto-submit fabricated a response or grade, or did not close the attempt");
    }
    Ok(())
}

async fn exercise_scoring_generation(
    store: &learning_data_access::postgres::PostgresStore,
    context: TenantContext,
    instructor: UserId,
    student: UserId,
    ids: SeedIds,
    assignment: AssignmentRecord,
) -> Result<()> {
    let run = store
        .start_or_resume_run(context, student, ids.assignment, ids.run)
        .await
        .context("starting database scoring acceptance run")?;
    let implementation = |name: &str| ImplementationVersion {
        id: name.to_string(),
        version: "acceptance-1".to_string(),
    };
    let attempt = store
        .issue_or_resume_question_attempt(
            context,
            IssueQuestionAttemptCommand {
                actor: student,
                attempt: ids.attempt,
                run: run.id,
                assignment_position: 0,
                problem: ids.problem,
                question_version: ids.version,
                seed: 17,
                parameter_hash: "database-scoring-parameters".to_string(),
                provenance: AttemptProvenance {
                    adapter: implementation("native"),
                    renderer: None,
                    generator: None,
                    source_artifact: None,
                    asset_objects: Vec::new(),
                    grading: implementation("native"),
                    rendered_question_sha256: "database-scoring-render".to_string(),
                },
                prefetched: None,
                predecessor_submission: None,
            },
        )
        .await
        .context("issuing database scoring acceptance attempt")?;
    store
        .submit_question_attempt(
            context,
            SubmitQuestionAttemptCommand {
                actor: student,
                attempt: attempt.id,
                response: question_model::StudentResponse::MultipleChoice {
                    selected: vec![ChoiceId::new("amide")],
                },
                result: AttemptResult {
                    correct: false,
                    points_earned: 0.5,
                    points_possible: 1.0,
                },
                feedback: FeedbackContent::default(),
                idempotency_key: SubmissionIdempotencyKey::parse("database-scoring-acceptance")?,
            },
        )
        .await
        .context("submitting database scoring acceptance attempt")?;
    let current = store
        .get_assignment_for_edit(context, ids.assignment)
        .await
        .context("reading assignment before database scoring acceptance edit")?
        .ok_or_else(|| anyhow::anyhow!("acceptance assignment disappeared"))?;
    let mut items = assignment.items;
    items[0].points_possible = PointValue::from_whole(2);
    let changed = store
        .replace_assignment(
            context,
            ids.course,
            ids.assignment,
            current.revision,
            AssignmentUpdate {
                title: current.record.title,
                items,
                selection_groups: current.record.selection_groups,
                policies: current.record.policies,
            },
        )
        .await
        .context("changing points for database scoring acceptance")?;
    if changed.scoring_status != question_model::ScoringStatus::Recalculating {
        bail!("point change did not hide stale scores as recalculating");
    }
    let claimed = store
        .claim_next_job(&JobClaimFilter::all(), JobLeaseDuration::from_seconds(60)?)
        .await
        .context("claiming database scoring acceptance job")?
        .ok_or_else(|| anyhow::anyhow!("point change did not queue scoring work"))?;
    let JobPayload::RecalculateAssignment {
        assignment,
        generation,
    } = claimed.payload
    else {
        bail!("database scoring acceptance claimed another job family");
    };
    let command = AssignmentScoringWorkerCommand {
        job: claimed.id,
        lease: claimed.lease_token,
        assignment,
        generation,
    };
    store
        .prepare_assignment_scoring(context, command)
        .await
        .context("staging database scoring generation")?;
    let staged = store
        .get_assignment_for_edit(context, ids.assignment)
        .await
        .context("reading privately staged database scoring assignment")?
        .ok_or_else(|| anyhow::anyhow!("staged acceptance assignment disappeared"))?;
    if staged.scoring_status != question_model::ScoringStatus::Recalculating {
        bail!("private staging exposed partial current scores");
    }
    let mut superseding_items = changed.record.items.clone();
    superseding_items[0].points_possible = PointValue::from_whole(3);
    let superseding = store
        .replace_assignment(
            context,
            ids.course,
            ids.assignment,
            changed.revision,
            AssignmentUpdate {
                title: changed.record.title,
                items: superseding_items,
                selection_groups: changed.record.selection_groups,
                policies: changed.record.policies,
            },
        )
        .await
        .context("superseding an in-flight database scoring generation")?;
    if superseding.scoring_generation <= generation
        || superseding.scoring_status != question_model::ScoringStatus::Recalculating
    {
        bail!("new scoring generation did not supersede staged work");
    }
    if store
        .commit_assignment_scoring(context, command)
        .await
        .context("discarding superseded database scoring staging")?
        != AssignmentScoringCommitOutcome::Superseded
    {
        bail!("old scoring generation was not discarded");
    }
    let pending = store
        .get_assignment_for_edit(context, ids.assignment)
        .await
        .context("reading assignment after superseded database scoring work")?
        .ok_or_else(|| anyhow::anyhow!("superseded acceptance assignment disappeared"))?;
    if pending.scoring_generation != superseding.scoring_generation
        || pending.scoring_status != question_model::ScoringStatus::Recalculating
    {
        bail!("discarding old scoring work changed the current generation");
    }
    let current_job = store
        .claim_next_job(&JobClaimFilter::all(), JobLeaseDuration::from_seconds(60)?)
        .await
        .context("claiming superseding database scoring acceptance job")?
        .ok_or_else(|| anyhow::anyhow!("superseding point change did not queue scoring work"))?;
    let JobPayload::RecalculateAssignment {
        assignment: current_assignment,
        generation: current_generation,
    } = current_job.payload
    else {
        bail!("database scoring acceptance claimed another job family");
    };
    if current_assignment != ids.assignment || current_generation != superseding.scoring_generation
    {
        bail!("database scoring acceptance claimed the wrong generation");
    }
    let current_command = AssignmentScoringWorkerCommand {
        job: current_job.id,
        lease: current_job.lease_token,
        assignment: current_assignment,
        generation: current_generation,
    };
    store
        .prepare_assignment_scoring(context, current_command)
        .await
        .context("staging current database scoring generation")?;
    let concurrent_run = store
        .start_or_resume_run(context, student, ids.assignment, ids.concurrent_run)
        .await
        .context("starting a run during database scoring acceptance")?;
    let concurrent_attempt = store
        .issue_or_resume_question_attempt(
            context,
            IssueQuestionAttemptCommand {
                actor: student,
                attempt: ids.concurrent_attempt,
                run: concurrent_run.id,
                assignment_position: 0,
                problem: ids.problem,
                question_version: ids.version,
                seed: 18,
                parameter_hash: "database-scoring-concurrent-parameters".to_string(),
                provenance: AttemptProvenance {
                    adapter: implementation("native"),
                    renderer: None,
                    generator: None,
                    source_artifact: None,
                    asset_objects: Vec::new(),
                    grading: implementation("native"),
                    rendered_question_sha256: "database-scoring-concurrent-render".to_string(),
                },
                prefetched: None,
                predecessor_submission: None,
            },
        )
        .await
        .context("issuing an attempt during database scoring acceptance")?;
    store
        .submit_question_attempt(
            context,
            SubmitQuestionAttemptCommand {
                actor: student,
                attempt: concurrent_attempt.id,
                response: question_model::StudentResponse::MultipleChoice {
                    selected: vec![ChoiceId::new("amide")],
                },
                result: AttemptResult {
                    correct: true,
                    points_earned: 1.0,
                    points_possible: 1.0,
                },
                feedback: FeedbackContent::default(),
                idempotency_key: SubmissionIdempotencyKey::parse("database-scoring-concurrent")?,
            },
        )
        .await
        .context("submitting an attempt during database scoring acceptance")?;
    if store
        .commit_assignment_scoring(context, current_command)
        .await
        != Err(StoreError::Conflict)
    {
        bail!("stale scoring staging ignored a concurrent submission");
    }
    store
        .prepare_assignment_scoring(context, current_command)
        .await
        .context("restaging after concurrent database scoring activity")?;
    if store
        .commit_assignment_scoring(context, current_command)
        .await
        .context("committing current database scoring generation")?
        != AssignmentScoringCommitOutcome::Committed
    {
        bail!("current scoring generation did not commit");
    }
    let committed = store
        .get_assignment_for_edit(context, ids.assignment)
        .await
        .context("reading committed database scoring assignment")?
        .ok_or_else(|| anyhow::anyhow!("rescored acceptance assignment disappeared"))?;
    if committed.scoring_status != question_model::ScoringStatus::Current
        || committed.scoring_generation != current_generation
    {
        bail!("rescored assignment did not become current");
    }
    assert_seed_summary_score(store, context, student, ids.run, 1.0).await?;
    exercise_attempt_support(store, context, instructor, student, ids).await?;
    recalculate_seed_item(
        store,
        context,
        ids,
        PointValue::ZERO,
        AssignmentScoringMode::Normal,
    )
    .await?;
    assert_seed_summary_score(store, context, student, ids.run, 0.0).await?;
    recalculate_seed_item(
        store,
        context,
        ids,
        PointValue::from_whole(4),
        AssignmentScoringMode::FullCredit,
    )
    .await?;
    assert_seed_summary_score(store, context, student, ids.run, 1.0).await?;
    recalculate_seed_item(
        store,
        context,
        ids,
        PointValue::from_whole(2),
        AssignmentScoringMode::ExtraCredit,
    )
    .await?;
    assert_seed_summary_score(store, context, student, ids.run, 2.0).await?;
    recalculate_seed_item(
        store,
        context,
        ids,
        PointValue::from_whole(2),
        AssignmentScoringMode::Excluded,
    )
    .await?;
    assert_seed_summary_score(store, context, student, ids.run, 0.0).await?;
    exercise_delete_and_regrade(store, context, student, ids).await?;
    Ok(())
}

async fn exercise_attempt_support(
    store: &learning_data_access::postgres::PostgresStore,
    context: TenantContext,
    instructor: UserId,
    student: UserId,
    ids: SeedIds,
) -> Result<()> {
    let run = store
        .start_or_resume_run(context, student, ids.assignment, ids.support_run)
        .await
        .context("starting attempt-support acceptance run")?;
    let provenance = AttemptProvenance {
        adapter: ImplementationVersion {
            id: "native".to_string(),
            version: "acceptance-1".to_string(),
        },
        renderer: None,
        generator: None,
        source_artifact: None,
        asset_objects: Vec::new(),
        grading: ImplementationVersion {
            id: "native".to_string(),
            version: "acceptance-1".to_string(),
        },
        rendered_question_sha256: "database-attempt-support-render".to_string(),
    };
    let attempt = store
        .issue_or_resume_question_attempt(
            context,
            IssueQuestionAttemptCommand {
                actor: student,
                attempt: ids.support_attempt,
                run: run.id,
                assignment_position: 0,
                problem: ids.problem,
                question_version: ids.version,
                seed: 20,
                parameter_hash: "database-attempt-support-parameters".to_string(),
                provenance: provenance.clone(),
                prefetched: None,
                predecessor_submission: None,
            },
        )
        .await
        .context("issuing force-submit acceptance attempt")?;
    let force_action = AttemptSupportActionId::from_uuid(derived_uuid(
        context.tenant_id(),
        "support-force-action",
    ));
    let forced = store
        .force_submit_attempt(
            context,
            ForceSubmitAttemptCommand {
                action: force_action,
                actor: instructor,
                attempt: attempt.id,
            },
        )
        .await
        .context("force-submitting active database attempt")?;
    if forced.previous_status != AttemptStatus::InProgress
        || forced.resulting_status != AttemptStatus::NeedsManualGrading
    {
        bail!("force-submit stored an invalid status transition");
    }
    if store
        .force_submit_attempt(
            context,
            ForceSubmitAttemptCommand {
                action: force_action,
                actor: instructor,
                attempt: attempt.id,
            },
        )
        .await
        .context("replaying database force-submit")?
        != forced
    {
        bail!("force-submit retry did not return the original audit record");
    }
    let current = store
        .get_question_attempt(context, attempt.id)
        .await
        .context("reading force-submitted database attempt")?
        .ok_or_else(|| anyhow::anyhow!("force-submitted database attempt disappeared"))?;
    if current.status != AttemptStatus::NeedsManualGrading
        || current.response.is_some()
        || current.result.is_some()
        || current.timer.submitted_at != Some(forced.occurred_at)
    {
        bail!("force-submit fabricated work or failed to expose current status");
    }
    if store
        .submit_question_attempt(
            context,
            SubmitQuestionAttemptCommand {
                actor: student,
                attempt: attempt.id,
                response: question_model::StudentResponse::MultipleChoice {
                    selected: vec![ChoiceId::new("amide")],
                },
                result: AttemptResult {
                    correct: true,
                    points_earned: 1.0,
                    points_possible: 1.0,
                },
                feedback: FeedbackContent::default(),
                idempotency_key: SubmissionIdempotencyKey::parse("database-submit-after-force")?,
            },
        )
        .await
        != Err(StoreError::Conflict)
    {
        bail!("ordinary submission remained open after force-submit");
    }

    let clear_forced_action = AttemptSupportActionId::from_uuid(derived_uuid(
        context.tenant_id(),
        "support-clear-forced-action",
    ));
    store
        .clear_attempt(
            context,
            ClearAttemptCommand {
                action: clear_forced_action,
                actor: instructor,
                attempt: attempt.id,
            },
        )
        .await
        .context("clearing force-submitted database attempt")?;
    let replacement = store
        .issue_or_resume_question_attempt(
            context,
            IssueQuestionAttemptCommand {
                actor: student,
                attempt: ids.support_replacement,
                run: run.id,
                assignment_position: 0,
                problem: ids.problem,
                question_version: ids.version,
                seed: 21,
                parameter_hash: "database-attempt-support-replacement".to_string(),
                provenance,
                prefetched: None,
                predecessor_submission: None,
            },
        )
        .await
        .context("issuing replacement after database clear")?;
    store
        .submit_question_attempt(
            context,
            SubmitQuestionAttemptCommand {
                actor: student,
                attempt: replacement.id,
                response: question_model::StudentResponse::MultipleChoice {
                    selected: vec![ChoiceId::new("amide")],
                },
                result: AttemptResult {
                    correct: true,
                    points_earned: 1.0,
                    points_possible: 1.0,
                },
                feedback: FeedbackContent::default(),
                idempotency_key: SubmissionIdempotencyKey::parse(
                    "database-attempt-support-replacement",
                )?,
            },
        )
        .await
        .context("submitting database support replacement")?;
    let clear_scored_action = AttemptSupportActionId::from_uuid(derived_uuid(
        context.tenant_id(),
        "support-clear-scored-action",
    ));
    let cleared = store
        .clear_attempt(
            context,
            ClearAttemptCommand {
                action: clear_scored_action,
                actor: instructor,
                attempt: replacement.id,
            },
        )
        .await
        .context("clearing scored database attempt")?;
    if cleared.previous_status != AttemptStatus::Submitted
        || cleared.resulting_status != AttemptStatus::Cleared
    {
        bail!("clear stored an invalid submitted-attempt transition");
    }
    if store
        .clear_attempt(
            context,
            ClearAttemptCommand {
                action: clear_scored_action,
                actor: instructor,
                attempt: replacement.id,
            },
        )
        .await
        .context("replaying scored database clear")?
        != cleared
    {
        bail!("clear retry did not return the original audit record");
    }
    let pending = store
        .get_assignment_for_edit(context, ids.assignment)
        .await
        .context("reading assignment after database clear")?
        .ok_or_else(|| anyhow::anyhow!("assignment disappeared after database clear"))?;
    if pending.scoring_status != question_model::ScoringStatus::Recalculating {
        bail!("clearing a scored attempt did not fence the current grade");
    }
    commit_next_seed_scoring_job(store, context, ids.assignment, pending.scoring_generation)
        .await?;
    if store
        .claim_next_job(&JobClaimFilter::all(), JobLeaseDuration::from_seconds(30)?)
        .await
        .context("checking exact clear retry queue effects")?
        .is_some()
    {
        bail!("an exact clear retry queued duplicate scoring work");
    }
    if !store
        .get_run_summary_page(
            context,
            student,
            run.id,
            PageRequest::first(PageSize::new(10)?),
        )
        .await
        .context("reading student summary after database clear")?
        .outcomes
        .items
        .is_empty()
    {
        bail!("student summary exposed cleared database evidence");
    }
    if store
        .get_run_summary_page(
            context,
            instructor,
            run.id,
            PageRequest::first(PageSize::new(10)?),
        )
        .await
        .context("reading instructor evidence after database clear")?
        .outcomes
        .items
        .len()
        != 2
    {
        bail!("instructor database summary lost cleared evidence");
    }
    Ok(())
}

async fn exercise_delete_and_regrade(
    store: &learning_data_access::postgres::PostgresStore,
    context: TenantContext,
    student: UserId,
    ids: SeedIds,
) -> Result<()> {
    let run = store
        .start_or_resume_run(context, student, ids.assignment, ids.retirement_run)
        .await
        .context("starting Delete and Regrade acceptance run")?;
    let attempt = store
        .issue_or_resume_question_attempt(
            context,
            IssueQuestionAttemptCommand {
                actor: student,
                attempt: ids.retirement_attempt,
                run: run.id,
                assignment_position: 0,
                problem: ids.problem,
                question_version: ids.version,
                seed: 19,
                parameter_hash: "database-delete-and-regrade-parameters".to_string(),
                provenance: AttemptProvenance {
                    adapter: ImplementationVersion {
                        id: "native".to_string(),
                        version: "acceptance-1".to_string(),
                    },
                    renderer: None,
                    generator: None,
                    source_artifact: None,
                    asset_objects: Vec::new(),
                    grading: ImplementationVersion {
                        id: "native".to_string(),
                        version: "acceptance-1".to_string(),
                    },
                    rendered_question_sha256: "database-delete-and-regrade-render".to_string(),
                },
                prefetched: None,
                predecessor_submission: None,
            },
        )
        .await
        .context("issuing Delete and Regrade acceptance attempt")?;
    let current = store
        .get_assignment_for_edit(context, ids.assignment)
        .await
        .context("reading Delete and Regrade acceptance assignment")?
        .ok_or_else(|| anyhow::anyhow!("Delete and Regrade assignment disappeared"))?;
    let command = DeleteAndRegradeAssignmentItemCommand {
        course: ids.course,
        assignment: ids.assignment,
        item: ids.assignment_item,
        expected_revision: current.revision,
    };
    if store
        .delete_and_regrade_assignment_item(context, command)
        .await
        != Err(StoreError::Conflict)
    {
        bail!("Delete and Regrade did not block an affected active attempt");
    }
    store
        .submit_question_attempt(
            context,
            SubmitQuestionAttemptCommand {
                actor: student,
                attempt: attempt.id,
                response: question_model::StudentResponse::MultipleChoice {
                    selected: vec![ChoiceId::new("amide")],
                },
                result: AttemptResult {
                    correct: true,
                    points_earned: 1.0,
                    points_possible: 1.0,
                },
                feedback: FeedbackContent::default(),
                idempotency_key: SubmissionIdempotencyKey::parse("database-delete-and-regrade")?,
            },
        )
        .await
        .context("submitting Delete and Regrade acceptance attempt")?;
    let retired = store
        .delete_and_regrade_assignment_item(context, command)
        .await
        .context("retiring submitted Delete and Regrade item")?;
    let item = retired
        .record
        .items
        .iter()
        .find(|item| item.id == ids.assignment_item)
        .ok_or_else(|| anyhow::anyhow!("retired item tombstone disappeared"))?;
    if item.delivery_state != AssignmentDeliveryState::Retired
        || item.scoring_mode != AssignmentScoringMode::Excluded
        || retired.scoring_status != question_model::ScoringStatus::Recalculating
    {
        bail!("Delete and Regrade did not persist an excluded tombstone");
    }
    let replay = store
        .delete_and_regrade_assignment_item(
            context,
            DeleteAndRegradeAssignmentItemCommand {
                expected_revision: retired.revision,
                ..command
            },
        )
        .await
        .context("replaying Delete and Regrade acceptance command")?;
    if replay != retired {
        bail!("Delete and Regrade replay created another revision");
    }
    commit_next_seed_scoring_job(store, context, ids.assignment, retired.scoring_generation)
        .await?;
    if !store
        .get_run_summary_page(
            context,
            student,
            run.id,
            PageRequest::first(PageSize::new(10)?),
        )
        .await
        .context("reading student summary after Delete and Regrade")?
        .outcomes
        .items
        .is_empty()
    {
        bail!("student summary exposed retired response or feedback");
    }
    let future = store
        .start_or_resume_run(context, student, ids.assignment, ids.post_retirement_run)
        .await
        .context("starting post-retirement acceptance run")?;
    if !store
        .assignment_run_items(context, future.id)
        .await
        .context("reading post-retirement acceptance run items")?
        .is_empty()
    {
        bail!("future run still delivered a retired assignment item");
    }
    Ok(())
}

async fn recalculate_seed_item(
    store: &learning_data_access::postgres::PostgresStore,
    context: TenantContext,
    ids: SeedIds,
    points: PointValue,
    mode: AssignmentScoringMode,
) -> Result<()> {
    let current = store
        .get_assignment_for_edit(context, ids.assignment)
        .await
        .context("reading assignment before scoring-mode acceptance edit")?
        .ok_or_else(|| anyhow::anyhow!("scoring-mode acceptance assignment disappeared"))?;
    let mut items = current.record.items.clone();
    items[0].points_possible = points;
    items[0].scoring_mode = mode;
    let changed = store
        .replace_assignment(
            context,
            ids.course,
            ids.assignment,
            current.revision,
            AssignmentUpdate {
                title: current.record.title,
                items,
                selection_groups: current.record.selection_groups,
                policies: current.record.policies,
            },
        )
        .await
        .context("changing scoring mode for database acceptance")?;
    if changed.scoring_status != question_model::ScoringStatus::Recalculating {
        bail!("scoring-mode edit did not enter recalculation");
    }
    commit_next_seed_scoring_job(store, context, ids.assignment, changed.scoring_generation).await
}

async fn commit_next_seed_scoring_job(
    store: &learning_data_access::postgres::PostgresStore,
    context: TenantContext,
    expected_assignment: AssignmentId,
    expected_generation: question_model::ScoringGeneration,
) -> Result<()> {
    let claimed = store
        .claim_next_job(&JobClaimFilter::all(), JobLeaseDuration::from_seconds(60)?)
        .await
        .context("claiming database scoring acceptance job")?
        .ok_or_else(|| anyhow::anyhow!("database scoring edit did not queue work"))?;
    let JobPayload::RecalculateAssignment {
        assignment,
        generation,
    } = claimed.payload
    else {
        bail!("database scoring acceptance claimed another job family");
    };
    if assignment != expected_assignment || generation != expected_generation {
        bail!("database scoring acceptance claimed the wrong generation");
    }
    let command = AssignmentScoringWorkerCommand {
        job: claimed.id,
        lease: claimed.lease_token,
        assignment,
        generation,
    };
    store
        .prepare_assignment_scoring(context, command)
        .await
        .context("staging database scoring acceptance generation")?;
    if store
        .commit_assignment_scoring(context, command)
        .await
        .context("committing database scoring acceptance generation")?
        != AssignmentScoringCommitOutcome::Committed
    {
        bail!("database scoring acceptance generation did not commit");
    }
    Ok(())
}

async fn assert_seed_summary_score(
    store: &learning_data_access::postgres::PostgresStore,
    context: TenantContext,
    student: UserId,
    run: RunId,
    expected: f64,
) -> Result<()> {
    let page = store
        .get_run_summary_page(context, student, run, PageRequest::first(PageSize::new(1)?))
        .await
        .context("reading database scoring acceptance summary")?;
    let actual = page
        .summary
        .current_score
        .ok_or_else(|| anyhow::anyhow!("database scoring acceptance summary has no score"))?;
    if (actual - expected).abs() > f64::EPSILON {
        bail!("database scoring acceptance expected summary {expected}, got {actual}");
    }
    Ok(())
}

/// Reads the production native registry instead of maintaining a second
/// capability declaration in the E2E bootstrap.
fn native_capabilities() -> Result<BackendCapabilities> {
    NativeAdapter::new()
        .capabilities(&QuestionSource::Native {
            family: "peptide_bond_geometry".to_string(),
        })
        .context("resolving capabilities for the native E2E question family")
}

#[derive(Clone, Copy)]
struct SeedIds {
    workspace: WorkspaceId,
    problem: ProblemId,
    version: VersionId,
    course: CourseId,
    assignment: AssignmentId,
    assignment_item: AssignmentItemId,
    enrollment: EnrollmentId,
    run: RunId,
    attempt: QuestionAttemptId,
    concurrent_run: RunId,
    concurrent_attempt: QuestionAttemptId,
    support_run: RunId,
    support_attempt: QuestionAttemptId,
    support_replacement: QuestionAttemptId,
    retirement_run: RunId,
    retirement_attempt: QuestionAttemptId,
    post_retirement_run: RunId,
    timing_assignment: AssignmentId,
    timing_assignment_item_one: AssignmentItemId,
    timing_assignment_item_two: AssignmentItemId,
    timing_assignment_item_three: AssignmentItemId,
    timing_enrollment: EnrollmentId,
    timing_run: RunId,
    timing_attempt_one: QuestionAttemptId,
    timing_attempt_two: QuestionAttemptId,
    timing_attempt_three: QuestionAttemptId,
    timing_group: CourseGroupId,
    timing_group_exception: AssignmentPolicyExceptionId,
    timing_student_exception: AssignmentPolicyExceptionId,
    timing_exception_run: RunId,
    timing_exception_attempt: QuestionAttemptId,
}

impl SeedIds {
    fn for_tenant(tenant: TenantId) -> Self {
        Self {
            workspace: WorkspaceId::from_uuid(derived_uuid(tenant, "workspace")),
            problem: ProblemId::from_uuid(derived_uuid(tenant, "problem")),
            version: VersionId::from_uuid(derived_uuid(tenant, "version")),
            course: CourseId::from_uuid(derived_uuid(tenant, "course")),
            assignment: AssignmentId::from_uuid(derived_uuid(tenant, "assignment")),
            assignment_item: AssignmentItemId::from_uuid(derived_uuid(tenant, "assignment-item")),
            enrollment: EnrollmentId::from_uuid(derived_uuid(tenant, "enrollment")),
            run: RunId::from_uuid(derived_uuid(tenant, "run")),
            attempt: QuestionAttemptId::from_uuid(derived_uuid(tenant, "attempt")),
            concurrent_run: RunId::from_uuid(derived_uuid(tenant, "concurrent-run")),
            concurrent_attempt: QuestionAttemptId::from_uuid(derived_uuid(
                tenant,
                "concurrent-attempt",
            )),
            support_run: RunId::from_uuid(derived_uuid(tenant, "support-run")),
            support_attempt: QuestionAttemptId::from_uuid(derived_uuid(tenant, "support-attempt")),
            support_replacement: QuestionAttemptId::from_uuid(derived_uuid(
                tenant,
                "support-replacement",
            )),
            retirement_run: RunId::from_uuid(derived_uuid(tenant, "retirement-run")),
            retirement_attempt: QuestionAttemptId::from_uuid(derived_uuid(
                tenant,
                "retirement-attempt",
            )),
            post_retirement_run: RunId::from_uuid(derived_uuid(tenant, "post-retirement-run")),
            timing_assignment: AssignmentId::from_uuid(derived_uuid(tenant, "timing-assignment")),
            timing_assignment_item_one: AssignmentItemId::from_uuid(derived_uuid(
                tenant,
                "timing-assignment-item-one",
            )),
            timing_assignment_item_two: AssignmentItemId::from_uuid(derived_uuid(
                tenant,
                "timing-assignment-item-two",
            )),
            timing_assignment_item_three: AssignmentItemId::from_uuid(derived_uuid(
                tenant,
                "timing-assignment-item-three",
            )),
            timing_enrollment: EnrollmentId::from_uuid(derived_uuid(tenant, "timing-enrollment")),
            timing_run: RunId::from_uuid(derived_uuid(tenant, "timing-run")),
            timing_attempt_one: QuestionAttemptId::from_uuid(derived_uuid(
                tenant,
                "timing-attempt-one",
            )),
            timing_attempt_two: QuestionAttemptId::from_uuid(derived_uuid(
                tenant,
                "timing-attempt-two",
            )),
            timing_attempt_three: QuestionAttemptId::from_uuid(derived_uuid(
                tenant,
                "timing-attempt-three",
            )),
            timing_group: CourseGroupId::from_uuid(derived_uuid(tenant, "timing-group")),
            timing_group_exception: AssignmentPolicyExceptionId::from_uuid(derived_uuid(
                tenant,
                "timing-group-exception",
            )),
            timing_student_exception: AssignmentPolicyExceptionId::from_uuid(derived_uuid(
                tenant,
                "timing-student-exception",
            )),
            timing_exception_run: RunId::from_uuid(derived_uuid(tenant, "timing-exception-run")),
            timing_exception_attempt: QuestionAttemptId::from_uuid(derived_uuid(
                tenant,
                "timing-exception-attempt",
            )),
        }
    }
}

/// Stable IDs make the manifest repeatable for an isolated disposable E2E DB.
fn derived_uuid(tenant: TenantId, label: &str) -> Uuid {
    let mut hasher = Sha256::new();
    hasher.update(b"ple-replica-e2e-seed-v1:");
    hasher.update(tenant.as_uuid().as_bytes());
    hasher.update(label.as_bytes());
    let digest = hasher.finalize();
    let mut bytes = [0_u8; 16];
    bytes.copy_from_slice(&digest[..16]);
    // Mark as RFC 4122 variant / deterministic version 5-shaped UUID without
    // claiming a UUIDv7 was minted by a browser-facing boundary.
    bytes[6] = (bytes[6] & 0x0f) | 0x50;
    bytes[8] = (bytes[8] & 0x3f) | 0x80;
    Uuid::from_bytes(bytes)
}

fn native_draft(workspace: WorkspaceId) -> DraftQuestionDefinition {
    let mut parameters = BTreeMap::new();
    parameters.insert(
        "residue".to_string(),
        ParameterSpec::Choice {
            options: vec!["glycine".to_string(), "alanine".to_string()],
        },
    );
    DraftQuestionDefinition {
        workspace,
        source: DraftQuestionSource::Native {
            family: "peptide_bond_geometry".to_string(),
        },
        prompt: vec![ContentBlock::Text {
            markdown: "In the {{residue}} peptide example, which bond has restricted rotation because resonance gives it partial double-bond character?".to_string(),
        }],
        response: ResponseDefinition::MultipleChoice {
            choices: vec![
                choice("amide", "The carbonyl carbon-to-nitrogen bond"),
                choice("carbonyl", "The carbonyl carbon-to-oxygen bond"),
                choice("alpha-carbon", "The nitrogen-to-alpha-carbon bond"),
            ],
            selection: SelectionCardinality::ExactlyOne,
        },
        attempt_policy: AttemptPolicy {
            max_attempts: Some(1),
            feedback: FeedbackDisclosure::Deferred,
        },
        timing_policy: TimingPolicy::Untimed,
        randomization: RandomizationDefinition::Seeded {
            generator: GeneratorReference {
                id: "peptide-bond-choice".to_string(),
                version: "1".to_string(),
            },
            parameters,
        },
        grading: GradingDefinition::AllOrNothing { points: 1.0 },
        metadata: QuestionMetadata {
            title: "Peptide bond resonance and planarity".to_string(),
            tags: vec![Tag::new("replica-e2e")],
            taxonomy: Vec::new(),
            license: License::CcBy,
            language: "en-US".to_string(),
        },
    }
}

fn choice(id: &str, text: &str) -> ChoiceOption {
    ChoiceOption {
        id: ChoiceId::new(id),
        body: vec![ContentBlock::Text {
            markdown: text.to_string(),
        }],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn help_has_no_secret_or_browser_seed_endpoint() {
        assert!(USAGE.contains("e2e-seed"));
        assert!(!USAGE.contains("token"));
        assert!(!USAGE.contains("answer"));
    }

    #[test]
    fn parsing_requires_distinct_course_members() {
        let id = "00000000-0000-0000-0000-000000000001".to_string();
        let result = parse_arguments(&[
            "--database-url".to_string(),
            "postgres://example".to_string(),
            "--tenant".to_string(),
            id.clone(),
            "--instructor".to_string(),
            id.clone(),
            "--student".to_string(),
            id,
        ]);
        assert!(result.is_err());
    }

    #[test]
    fn ids_are_stable_and_separated_by_purpose() {
        let tenant = TenantId::from_uuid(Uuid::from_u128(9));
        let first = SeedIds::for_tenant(tenant);
        let second = SeedIds::for_tenant(tenant);
        assert_eq!(first.assignment, second.assignment);
        assert_ne!(first.assignment.as_uuid(), first.enrollment.as_uuid());
        assert_ne!(first.problem.as_uuid(), first.version.as_uuid());
    }

    #[test]
    fn native_seed_matches_catalog_publication_capability_admission() {
        let draft = native_draft(WorkspaceId::from_uuid(Uuid::from_u128(12)));
        let violations = domain::policy::validate_draft_for_publication(
            &draft,
            &native_capabilities().expect("registered native family has capabilities"),
        );

        assert!(
            violations.is_empty(),
            "the host seed must pass the same capability check as catalog publication: {violations:?}"
        );
    }
}

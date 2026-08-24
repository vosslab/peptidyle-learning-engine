#![cfg(feature = "postgres")]

//! Disposable PostgreSQL 17 oracle for the WP-PROF-T1 teaching projection.
//!
//! The Store constructs lifecycle and policy state. The small SQL probes below
//! prove only physical persistence, forced RLS, and sealed receipt history.

#[path = "postgres_course_creation_support.rs"]
mod course_creation_support;
use course_creation_support::sysadmin_course_creation_authority;

use domain::effective_assignment_policy::BaseAssignmentPolicy;
use learning_data_access::postgres::{PostgresStore, lazy_pool, verify_application_schema};
use learning_data_access::{
    AssignmentRecord, CatalogStore, CourseRecord, CourseRosterStore, CreateAssignmentCommand,
    CreateCourseCommand, DraftRecord, FlatGradingCapability, IssueQuestionAttemptCommand,
    LearnerWorkRoutingBinding, PresentationCapability, PutAssignmentTeachingSettingsCommand, Store,
    StoreError, TenantContext, UpsertCourseMember, WebworkGradingCapability,
};
use question_model::answer::NumericTolerance;
use question_model::envelope::ContentBlock;
use question_model::generation::RandomizationDefinition;
use question_model::run_policy::{
    AttemptPolicy, CompletionRequirement, ContinuedPractice, GradePolicy, RunPolicies,
    TimingPolicy, VariationPolicy,
};
use question_model::taxonomy::License;
use question_model::{
    ActivityTimestamp, AssignmentAudience, AssignmentDeliveryState, AssignmentId,
    AssignmentInstructions, AssignmentItem, AssignmentItemId, AssignmentLifecycle,
    AssignmentScoringMode, AssignmentTeachingSettings, BackendCapabilities, Capability, CourseId,
    DraftQuestionDefinition, DraftQuestionSource, GradingDefinition, ImplementationVersion,
    LateSubmissionPolicy, PointValue, ProblemId, ProblemVersionRef, PublicationScope,
    QuestionAttemptId, QuestionMetadata, QuestionSource, ResponseDefinition, RunId, TenantId,
    UserId, VersionId, WorkspaceId,
};
use std::num::NonZeroU32;
use uuid::Uuid;

fn id() -> Uuid {
    let mut bytes = [0_u8; 16];
    getrandom::fill(&mut bytes).expect("live fixture UUID randomness");
    Uuid::from_bytes(bytes)
}

fn policies() -> RunPolicies {
    RunPolicies {
        completion: CompletionRequirement::AnswerAll,
        grade: GradePolicy::Highest,
        continued_practice: ContinuedPractice::Unlimited,
        variation: VariationPolicy::NewSeeds,
    }
}

async fn publish_question(
    store: &PostgresStore,
    context: TenantContext,
    tenant: TenantId,
    instructor: UserId,
) -> ProblemVersionRef {
    let reference = ProblemVersionRef {
        problem: ProblemId::from_uuid(id()),
        version: VersionId::from_uuid(id()),
    };
    let draft = DraftRecord {
        tenant,
        question: DraftQuestionDefinition {
            workspace: WorkspaceId::from_uuid(id()),
            source: DraftQuestionSource::Native {
                family: "t1_live".to_string(),
            },
            prompt: vec![ContentBlock::Text {
                markdown: "T1 live fixture".to_string(),
            }],
            response: ResponseDefinition::Numeric {
                tolerance: NumericTolerance::Absolute { epsilon: 0.01 },
                unit: None,
            },
            attempt_policy: AttemptPolicy { max_attempts: None },
            timing_policy: TimingPolicy::Untimed,
            randomization: RandomizationDefinition::Static,
            grading: GradingDefinition::AllOrNothing { points: 1.0 },
            metadata: QuestionMetadata {
                title: "T1 live fixture".to_string(),
                tags: Vec::new(),
                taxonomy: Vec::new(),
                license: License::CcBy,
                language: "en-US".to_string(),
            },
        },
        derived_from: None,
    };
    let saved = store
        .upsert_draft(context, instructor, None, draft.clone())
        .await
        .expect("draft");
    store
        .publish_draft(
            context,
            instructor,
            learning_data_access::PublishDraftCommand {
                expected_draft: draft,
                expected_revision: saved.revision,
                publication: reference,
                published_source: QuestionSource::Native {
                    family: "t1_live".to_string(),
                },
                publisher: instructor,
                scope: PublicationScope::Public,
                byline: question_model::PublicByline::new(vec![
                    question_model::PublicAuthorName::new("PLE fixture".to_string())
                        .expect("byline"),
                ])
                .expect("byline"),
                source_artifact: None,
                qti_promotion: None,
                flat_question_promotion: None,
                capabilities: BackendCapabilities::from_iter([Capability::ServerGrading]),
            },
        )
        .await
        .expect("publish");
    reference
}

fn settings(
    lifecycle: AssignmentLifecycle,
    instructions: &str,
    policy: BaseAssignmentPolicy,
) -> AssignmentTeachingSettings {
    AssignmentTeachingSettings {
        lifecycle,
        instructions: AssignmentInstructions::try_new(instructions.to_string())
            .expect("fixture instructions"),
        base_policy: policy,
    }
}

fn issue(
    learner: UserId,
    run: RunId,
    attempt: QuestionAttemptId,
    reference: ProblemVersionRef,
    course: CourseId,
    assignment: AssignmentId,
) -> IssueQuestionAttemptCommand {
    IssueQuestionAttemptCommand {
        actor: learner,
        binding: LearnerWorkRoutingBinding::new(course, assignment),
        attempt,
        run,
        assignment_position: 0,
        problem: reference.problem,
        question_version: reference.version,
        seed: 1,
        presentation_capability: PresentationCapability::NotApplicable,
        presentation: None,
        presentation_snapshot: None,
        grading_envelope: None,
        flat_grading: None,
        flat_grading_capability: FlatGradingCapability::NotApplicable,
        webwork_grading: None,
        webwork_grading_capability: WebworkGradingCapability::NotApplicable,
        parameter_hash: "t1-live".to_string(),
        provenance: question_model::AttemptProvenance {
            adapter: ImplementationVersion {
                id: "t1-live".to_string(),
                version: "1".to_string(),
            },
            renderer: None,
            generator: None,
            source_artifact: None,
            asset_objects: Vec::new(),
            grading: ImplementationVersion {
                id: "t1-live-grade".to_string(),
                version: "1".to_string(),
            },
            rendered_question_sha256: "t1-live-render".to_string(),
        },
        webwork_replay: None,
        prefetched: None,
        predecessor_submission: None,
    }
}

#[tokio::test]
#[ignore = "requires the disposable PostgreSQL 17 database baseline"]
async fn postgres_assignment_teaching_projection_is_atomic_current_and_rls_bound() {
    let url = std::env::var("PLE_TEST_DATABASE_URL").expect("disposable database URL");
    let pool = lazy_pool(&url).expect("PostgreSQL URL");
    verify_application_schema(&pool)
        .await
        .expect("migrated schema");
    let store = PostgresStore::with_question_id_secret(pool.clone(), [0x71; 32]);
    let tenant = TenantId::from_uuid(id());
    let foreign_tenant = TenantId::from_uuid(id());
    let context = TenantContext::from_authenticated_session(tenant);
    let foreign_context = TenantContext::from_authenticated_session(foreign_tenant);
    let instructor = UserId::from_uuid(id());
    let learner = UserId::from_uuid(id());
    let outsider = UserId::from_uuid(id());
    let course = CourseId::from_uuid(id());
    store
        .create_course(
            context,
            CreateCourseCommand {
                course: CourseRecord {
                    id: course,
                    tenant,
                    title: "T1 live course".to_string(),
                    term: question_model::CourseTerm::from_parts(
                        "2000-01-01",
                        "2099-12-31",
                        "America/Chicago",
                    )
                    .expect("term"),
                },
                authority: sysadmin_course_creation_authority(&store, tenant, course, instructor)
                    .await,
            },
        )
        .await
        .expect("course");
    store
        .upsert_course_member(
            context,
            instructor,
            UpsertCourseMember {
                course,
                user: learner,
                display_name: "T1 learner".to_string(),
                roster_contact: None,
            },
        )
        .await
        .expect("learner");
    let reference = publish_question(&store, context, tenant, instructor).await;
    let assignment = AssignmentId::from_uuid(id());
    let now_millis: i64 =
        sqlx::query_scalar("SELECT (extract(epoch FROM clock_timestamp()) * 1000)::bigint")
            .fetch_one(&pool)
            .await
            .expect("authoritative fixture time");
    let policy = BaseAssignmentPolicy {
        available_at: Some(ActivityTimestamp::from_unix_millis(now_millis - 60_000)),
        due_at: Some(ActivityTimestamp::from_unix_millis(now_millis + 600_000)),
        closes_at: Some(ActivityTimestamp::from_unix_millis(now_millis + 1_200_000)),
        time_limit_seconds: Some(NonZeroU32::new(120).expect("positive")),
        attempt_limit: Some(NonZeroU32::new(2).expect("positive")),
        late_submission: LateSubmissionPolicy::Accept,
        deadline_behavior: question_model::AssignmentDeadlineBehavior::AutoSubmit,
    };
    let created = store
        .create_assignment(
            context,
            CreateAssignmentCommand {
                actor: instructor,
                assignment: AssignmentRecord {
                    id: assignment,
                    tenant,
                    course_id: course,
                    title: "T1 lifecycle".to_string(),
                    lifecycle: AssignmentLifecycle::Draft,
                    instructions: AssignmentInstructions::try_new("read first".to_string())
                        .expect("instructions"),
                    audience: AssignmentAudience::CourseWide,
                    items: vec![AssignmentItem {
                        id: AssignmentItemId::from_uuid(id()),
                        reference,
                        position: 0,
                        points_possible: PointValue::from_whole(1),
                        delivery_state: AssignmentDeliveryState::Active,
                        scoring_mode: AssignmentScoringMode::Normal,
                    }],
                    selection_groups: Vec::new(),
                    disclosure_policy: question_model::LearnerDisclosurePolicy::default(),
                    policies: policies(),
                },
                base_policy: BaseAssignmentPolicy::default(),
            },
        )
        .await
        .expect("draft assignment");
    assert!(
        store
            .learner_get_enrollment_for_assignment(context, learner, assignment)
            .await
            .expect("draft read")
            .is_none()
    );
    let published = store
        .put_assignment_teaching_settings(
            context,
            PutAssignmentTeachingSettingsCommand {
                actor: instructor,
                course,
                assignment,
                expected_revision: created.revision,
                settings: settings(
                    AssignmentLifecycle::Published,
                    "published instructions",
                    policy,
                ),
            },
        )
        .await
        .expect("publish atomically");
    assert_eq!(published.policy, policy);
    let run = store
        .start_or_resume_run(
            context,
            learner,
            LearnerWorkRoutingBinding::new(course, assignment),
            RunId::from_uuid(id()),
        )
        .await
        .expect("published G1 permits run");
    let issued = store
        .issue_or_resume_question_attempt(
            context,
            issue(
                learner,
                run.id,
                QuestionAttemptId::from_uuid(id()),
                reference,
                course,
                assignment,
            ),
        )
        .await
        .expect("issue receipt");
    let old_receipt = store
        .get_issued_effective_policy_receipt(context, issued.id)
        .await
        .expect("receipt")
        .expect("sealed");
    let scoring_before: i64 = sqlx::query_scalar(
        "SELECT scoring_generation FROM assignment WHERE tenant_id=$1 AND assignment_id=$2",
    )
    .bind(tenant.as_uuid())
    .bind(assignment.as_uuid())
    .fetch_one(&pool)
    .await
    .expect("generation");
    let revised_policy = BaseAssignmentPolicy {
        time_limit_seconds: Some(NonZeroU32::new(180).expect("positive")),
        ..policy
    };
    let changed = store
        .put_assignment_teaching_settings(
            context,
            PutAssignmentTeachingSettingsCommand {
                actor: instructor,
                course,
                assignment,
                expected_revision: published.revision,
                settings: settings(
                    AssignmentLifecycle::Published,
                    "revised instructions",
                    revised_policy,
                ),
            },
        )
        .await
        .expect("settings CAS");
    assert!(changed.revision > published.revision);
    assert_eq!(
        store
            .put_assignment_teaching_settings(
                context,
                PutAssignmentTeachingSettingsCommand {
                    actor: instructor,
                    course,
                    assignment,
                    expected_revision: published.revision,
                    settings: settings(AssignmentLifecycle::Published, "stale", policy)
                }
            )
            .await,
        Err(StoreError::Conflict)
    );
    let current = store
        .get_issued_effective_policy_receipt(context, issued.id)
        .await
        .expect("current receipt")
        .expect("receipt");
    assert!(current.generation > old_receipt.generation);
    let old_limit: Option<i32> = sqlx::query_scalar("SELECT resolved_time_limit_seconds FROM attempt_effective_policy_receipt WHERE tenant_id=$1 AND attempt_id=$2 AND receipt_generation=$3").bind(tenant.as_uuid()).bind(issued.id.as_uuid()).bind(i64::try_from(old_receipt.generation).expect("generation")).fetch_one(&pool).await.expect("historical receipt");
    assert_eq!(old_limit, Some(120));
    let scoring_after: i64 = sqlx::query_scalar(
        "SELECT scoring_generation FROM assignment WHERE tenant_id=$1 AND assignment_id=$2",
    )
    .bind(tenant.as_uuid())
    .bind(assignment.as_uuid())
    .fetch_one(&pool)
    .await
    .expect("generation");
    assert_eq!(scoring_after, scoring_before);
    let closed = store
        .put_assignment_teaching_settings(
            context,
            PutAssignmentTeachingSettingsCommand {
                actor: instructor,
                course,
                assignment,
                expected_revision: changed.revision,
                settings: settings(AssignmentLifecycle::Closed, "closed", revised_policy),
            },
        )
        .await
        .expect("close");
    assert!(
        store
            .start_or_resume_run(
                context,
                learner,
                LearnerWorkRoutingBinding::new(course, assignment),
                RunId::from_uuid(id()),
            )
            .await
            .is_err(),
        "closed G1 denies new run"
    );
    assert!(
        store
            .get_assignment_for_edit(foreign_context, assignment)
            .await
            .expect("foreign RLS")
            .is_none()
    );
    assert_eq!(
        store
            .put_assignment_teaching_settings(
                context,
                PutAssignmentTeachingSettingsCommand {
                    actor: outsider,
                    course,
                    assignment,
                    expected_revision: closed.revision,
                    settings: settings(AssignmentLifecycle::Archived, "no", revised_policy)
                }
            )
            .await,
        Err(StoreError::NotFound),
        "a nonmember cannot enumerate the assignment through a settings write"
    );
    assert_eq!(
        store
            .put_assignment_teaching_settings(
                context,
                PutAssignmentTeachingSettingsCommand {
                    actor: instructor,
                    course,
                    assignment,
                    expected_revision: closed.revision,
                    settings: settings(AssignmentLifecycle::Archived, "archived", revised_policy)
                }
            )
            .await
            .expect("archive")
            .policy,
        revised_policy
    );
}

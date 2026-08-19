#![cfg(feature = "postgres")]

//! Disposable PostgreSQL 17 oracle for the normalized S3 policy boundary.
//!
//! Store commands construct the teaching state.  SQL below is deliberately
//! limited to PostgreSQL-only facts: RLS, grants, and the sealed receipt
//! relations that no in-memory Store can prove.

use domain::effective_assignment_policy::{
    BaseAssignmentPolicy, GroupAccommodation, GroupScheduleOffset, IndividualPolicyException,
    PolicyModificationMode, PolicyPatch, PolicyPatchSet, PolicySource, ScheduleOffsetSeconds,
};
use learning_data_access::postgres::{PostgresStore, lazy_pool, verify_application_schema};
use learning_data_access::{
    AssignmentRecord, CatalogStore, CourseGroupRecord, CourseRecord, CourseRosterStore,
    CreateCourseCommand, DraftRecord, FlatGradingCapability, IssueQuestionAttemptCommand,
    PresentationCapability, PutBaseAssignmentPolicyCommand, PutCourseGroupCommand,
    PutGroupAccommodationCommand, PutGroupScheduleOffsetCommand,
    PutIndividualPolicyExceptionCommand, ResolveEffectivePolicyCommand, Store,
    StoredIndividualPolicyException, TenantContext, UpsertCourseMember, WebworkGradingCapability,
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
    ActivityTimestamp, AssignmentDeliveryState, AssignmentId, AssignmentItem, AssignmentItemId,
    AssignmentPolicyExceptionId, AssignmentScoringMode, BackendCapabilities, Capability,
    CourseGroupId, CourseGroupPurpose, CourseId, DraftQuestionDefinition, DraftQuestionSource,
    GradingDefinition, ImplementationVersion, LateSubmissionPolicy, PointValue, ProblemId,
    ProblemVersionRef, PublicationScope, QuestionAttemptId, QuestionMetadata, QuestionSource,
    ResponseDefinition, RunId, TenantId, UserId, VersionId, WorkspaceId,
};
use sqlx::PgPool;
use std::num::NonZeroU32;
use uuid::Uuid;

#[path = "conformance/effective_policy_parity.rs"]
mod effective_policy_parity;

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
                family: "molar_mass".to_string(),
            },
            prompt: vec![ContentBlock::Text {
                markdown: "S3 effective policy fixture".to_string(),
            }],
            response: ResponseDefinition::Numeric {
                tolerance: NumericTolerance::Relative { fraction: 0.01 },
                unit: Some("g/mol".to_string()),
            },
            attempt_policy: AttemptPolicy { max_attempts: None },
            timing_policy: TimingPolicy::Untimed,
            randomization: RandomizationDefinition::Static,
            grading: GradingDefinition::AllOrNothing { points: 1.0 },
            metadata: QuestionMetadata {
                title: "S3 effective policy fixture".to_string(),
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
        .expect("save live policy fixture draft");
    store
        .publish_draft(
            context,
            instructor,
            learning_data_access::PublishDraftCommand {
                expected_draft: draft,
                expected_revision: saved.revision,
                publication: reference,
                published_source: QuestionSource::Native {
                    family: "molar_mass".to_string(),
                },
                publisher: instructor,
                scope: PublicationScope::Public,
                byline: question_model::PublicByline::new(vec![
                    question_model::PublicAuthorName::new("PLE fixture".to_string())
                        .expect("valid test byline"),
                ])
                .expect("valid test byline"),
                source_artifact: None,
                qti_promotion: None,
                flat_question_promotion: None,
                capabilities: BackendCapabilities::from_iter([Capability::ServerGrading]),
            },
        )
        .await
        .expect("publish live policy fixture question");
    reference
}

fn issue_command(
    learner: UserId,
    run: RunId,
    attempt: QuestionAttemptId,
    reference: ProblemVersionRef,
) -> IssueQuestionAttemptCommand {
    IssueQuestionAttemptCommand {
        actor: learner,
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
        parameter_hash: "postgres-effective-policy".to_string(),
        provenance: question_model::AttemptProvenance {
            adapter: ImplementationVersion {
                id: "postgres-effective-policy".to_string(),
                version: "1".to_string(),
            },
            renderer: None,
            generator: None,
            source_artifact: None,
            asset_objects: Vec::new(),
            grading: ImplementationVersion {
                id: "postgres-effective-policy-grading".to_string(),
                version: "1".to_string(),
            },
            rendered_question_sha256: "postgres-effective-policy-render".to_string(),
        },
        webwork_replay: None,
        prefetched: None,
        predecessor_submission: None,
    }
}

async fn student_cannot_write_policy_relations(pool: &PgPool, tenant: TenantId) {
    let mut transaction = pool.begin().await.expect("begin student privilege probe");
    sqlx::query("SET LOCAL ROLE ple_student")
        .execute(&mut *transaction)
        .await
        .expect("assume student role");
    sqlx::query("SELECT set_config('ple.tenant_id', $1, true)")
        .bind(tenant.to_string())
        .execute(&mut *transaction)
        .await
        .expect("scope student role");
    for relation in [
        "assignment_effective_policy_base",
        "assignment_group_schedule_offset",
        "assignment_group_accommodation",
        "assignment_individual_policy_exception",
        "attempt_effective_policy_receipt",
        "attempt_effective_policy_receipt_field_source",
        "attempt_effective_policy_current",
    ] {
        let allowed: bool =
            sqlx::query_scalar("SELECT has_table_privilege(current_user, $1, 'INSERT')")
                .bind(relation)
                .fetch_one(&mut *transaction)
                .await
                .expect("read student privilege");
        assert!(!allowed, "ple_student must not write {relation}");
    }
    transaction
        .rollback()
        .await
        .expect("rollback privilege probe");
}

#[tokio::test]
#[ignore = "requires the disposable PostgreSQL 17 database baseline"]
async fn postgres_effective_policy_is_normalized_precedence_bound_and_rls_enforced() {
    let database_url = std::env::var("PLE_TEST_DATABASE_URL")
        .expect("PLE_TEST_DATABASE_URL must name the disposable acceptance database");
    let pool = lazy_pool(&database_url).expect("valid live PostgreSQL URL");
    verify_application_schema(&pool)
        .await
        .expect("full migrated application schema is compatible");
    let store = PostgresStore::with_question_id_secret(pool.clone(), [0x34; 32]);
    effective_policy_parity::exercise_effective_policy_resolution_parity(&store).await;
    let tenant = TenantId::from_uuid(id());
    let other_tenant = TenantId::from_uuid(id());
    let context = TenantContext::from_authenticated_session(tenant);
    let other_context = TenantContext::from_authenticated_session(other_tenant);
    let instructor = UserId::from_uuid(id());
    let learner = UserId::from_uuid(id());
    let course = CourseId::from_uuid(id());
    store
        .create_course(
            context,
            CreateCourseCommand {
                course: CourseRecord {
                    id: course,
                    tenant,
                    title: "S3 live policy course".to_string(),
                    term: question_model::CourseTerm::from_parts(
                        "2026-08-24",
                        "2026-12-18",
                        "America/Chicago",
                    )
                    .expect("valid fixture term"),
                },
                initial_instructor: instructor,
            },
        )
        .await
        .expect("create course");
    store
        .upsert_course_member(
            context,
            UpsertCourseMember {
                course,
                user: learner,
                display_name: "S3 learner".to_string(),
                roster_contact: None,
            },
        )
        .await
        .expect("create learner membership");
    let student = store
        .get_current_course_membership(context, course, learner)
        .await
        .expect("read learner membership")
        .expect("learner membership exists")
        .student
        .expect("learner has stable student identity");
    let reference = publish_question(&store, context, tenant, instructor).await;
    let assignment = AssignmentId::from_uuid(id());
    let created = store
        .create_untimed_assignment(
            context,
            AssignmentRecord {
                id: assignment,
                tenant,
                course_id: course,
                title: "S3 normalized policy assignment".to_string(),
                audience: question_model::AssignmentAudience::CourseWide,
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
        )
        .await
        .expect("create assignment");
    let membership = store
        .get_current_course_membership(context, course, learner)
        .await
        .expect("read learner membership")
        .expect("learner membership exists");
    let schedule_group = CourseGroupId::from_uuid(id());
    let accommodation_group = CourseGroupId::from_uuid(id());
    for (id, purpose, title) in [
        (
            schedule_group,
            CourseGroupPurpose::Section,
            "S3 schedule group",
        ),
        (
            accommodation_group,
            CourseGroupPurpose::Accommodation,
            "S3 accommodation group",
        ),
    ] {
        store
            .put_course_group(
                context,
                PutCourseGroupCommand {
                    actor: instructor,
                    expected_revision: None,
                    record: CourseGroupRecord {
                        id,
                        tenant,
                        course,
                        purpose,
                        title: title.to_string(),
                        members: vec![membership.id],
                    },
                },
            )
            .await
            .expect("create applicable policy scope");
    }
    let revised = store
        .put_group_schedule_offset(
            context,
            PutGroupScheduleOffsetCommand {
                actor: instructor,
                course,
                assignment,
                expected_revision: created.revision,
                offset: GroupScheduleOffset {
                    group: schedule_group,
                    offset_seconds: ScheduleOffsetSeconds::try_new(60)
                        .expect("bounded nonzero offset"),
                },
            },
        )
        .await
        .expect("store normalized M2 offset");
    let revised = store
        .put_group_accommodation(
            context,
            PutGroupAccommodationCommand {
                actor: instructor,
                course,
                assignment,
                expected_revision: revised,
                accommodation: GroupAccommodation {
                    group: accommodation_group,
                    mode: PolicyModificationMode::ExtendOnly,
                    patch: PolicyPatchSet {
                        time_limit_seconds: PolicyPatch::Set(
                            NonZeroU32::new(240).expect("positive limit"),
                        ),
                        ..PolicyPatchSet::INHERIT
                    },
                },
            },
        )
        .await
        .expect("store normalized M3 accommodation");
    let revised = store
        .put_individual_policy_exception(
            context,
            PutIndividualPolicyExceptionCommand {
                actor: instructor,
                course,
                assignment,
                expected_revision: revised,
                exception: StoredIndividualPolicyException {
                    id: AssignmentPolicyExceptionId::from_uuid(id()),
                    exception: IndividualPolicyException {
                        student,
                        mode: PolicyModificationMode::Override,
                        patch: PolicyPatchSet {
                            time_limit_seconds: PolicyPatch::Set(
                                NonZeroU32::new(300).expect("positive limit"),
                            ),
                            ..PolicyPatchSet::INHERIT
                        },
                    },
                },
            },
        )
        .await
        .expect("M4 exists before any learner receipt");
    assert!(
        store
            .learner_get_enrollment_for_assignment(context, learner, assignment)
            .await
            .expect("read pre-materialization enrollment")
            .is_none()
    );
    let configured = store
        .put_base_assignment_policy(
            context,
            PutBaseAssignmentPolicyCommand {
                actor: instructor,
                course,
                assignment,
                expected_revision: revised,
                policy: BaseAssignmentPolicy {
                    available_at: Some(ActivityTimestamp::from_unix_millis(0)),
                    due_at: None,
                    closes_at: None,
                    time_limit_seconds: Some(NonZeroU32::new(120).expect("positive limit")),
                    attempt_limit: None,
                    late_submission: LateSubmissionPolicy::Accept,
                    deadline_behavior: question_model::AssignmentDeadlineBehavior::AutoSubmit,
                },
            },
        )
        .await
        .expect("store M1 policy");
    let grant = store
        .evaluate_assignment_entitlement(context, learner, course, assignment)
        .await
        .expect("evaluate S5 grant");
    let resolution = store
        .resolve_effective_policy(
            context,
            ResolveEffectivePolicyCommand {
                assignment,
                lifecycle: domain::effective_assignment_policy::AssignmentLifecycleGate::Open,
                entitlement: grant,
                authorization: domain::effective_assignment_policy::AuthorizationGate::Authorized,
                now: ActivityTimestamp::from_unix_millis(1_000),
                prior_run_count: 0,
            },
        )
        .await
        .expect("resolve policy")
        .expect("assignment exists");
    let domain::effective_assignment_policy::EffectivePolicyDecision::Allowed { policy, .. } =
        resolution.decision
    else {
        panic!("S5-granted learner must receive the composed S3 policy");
    };
    assert_eq!(
        policy.time_limit_seconds.value,
        Some(NonZeroU32::new(300).expect("positive limit"))
    );
    assert_eq!(
        policy.time_limit_seconds.source,
        PolicySource::IndividualException(student)
    );
    assert_eq!(resolution.revision, configured.revision);
    let denied = store
        .resolve_effective_policy(
            context,
            ResolveEffectivePolicyCommand {
                assignment,
                lifecycle: domain::effective_assignment_policy::AssignmentLifecycleGate::Open,
                entitlement: domain::entitlement::EntitlementDecision::Denied(
                    domain::entitlement::EntitlementDenial::LearnerNotActiveCourseStudent,
                ),
                authorization: domain::effective_assignment_policy::AuthorizationGate::Authorized,
                now: ActivityTimestamp::from_unix_millis(1_000),
                prior_run_count: 0,
            },
        )
        .await
        .expect("a denied S5 decision resolves closed")
        .expect("assignment exists");
    assert!(matches!(
        denied.decision,
        domain::effective_assignment_policy::EffectivePolicyDecision::Denied {
            gate: domain::effective_assignment_policy::PolicyGate::Entitlement,
            ..
        }
    ));
    let run = store
        .start_or_resume_run(context, learner, assignment, RunId::from_uuid(id()))
        .await
        .expect("granted learner starts atomically");
    let issued = store
        .issue_or_resume_question_attempt(
            context,
            issue_command(
                learner,
                run.id,
                QuestionAttemptId::from_uuid(id()),
                reference,
            ),
        )
        .await
        .expect("issue attempt with sealed policy receipt");
    let receipt = store
        .get_issued_effective_policy_receipt(context, issued.id)
        .await
        .expect("read sealed receipt")
        .expect("issued attempt has receipt");
    assert_eq!(receipt.generation, 1);

    let changed = store
        .put_base_assignment_policy(
            context,
            PutBaseAssignmentPolicyCommand {
                actor: instructor,
                course,
                assignment,
                expected_revision: configured.revision,
                policy: BaseAssignmentPolicy {
                    time_limit_seconds: Some(NonZeroU32::new(180).expect("positive limit")),
                    ..configured.policy
                },
            },
        )
        .await
        .expect("append next authored policy generation");
    assert!(changed.revision > configured.revision);
    let current_receipt = store
        .get_issued_effective_policy_receipt(context, issued.id)
        .await
        .expect("read active receipt after policy edit")
        .expect("active attempt retains a policy receipt");
    assert!(current_receipt.generation > receipt.generation);
    let historical_limit: Option<i32> = sqlx::query_scalar(
        "SELECT resolved_time_limit_seconds FROM attempt_effective_policy_receipt \
         WHERE tenant_id=$1 AND attempt_id=$2 AND receipt_generation=$3",
    )
    .bind(tenant.as_uuid())
    .bind(issued.id.as_uuid())
    .bind(i64::try_from(receipt.generation).expect("receipt generation fits"))
    .fetch_one(&pool)
    .await
    .expect("read sealed historical receipt");
    assert_eq!(historical_limit, Some(300));
    assert!(
        store
            .get_base_assignment_policy(other_context, assignment)
            .await
            .expect("foreign RLS read")
            .is_none()
    );
    student_cannot_write_policy_relations(&pool, tenant).await;
}

#![cfg(feature = "postgres")]

//! Disposable PostgreSQL 17 oracle for the normalized S3 policy boundary.
//!
//! Store commands construct the teaching state.  SQL below is deliberately
//! limited to PostgreSQL-only facts: RLS, grants, and the sealed receipt
//! relations that no in-memory Store can prove.

#[path = "postgres_course_creation_support.rs"]
mod course_creation_support;
use course_creation_support::sysadmin_course_creation_authority;

#[path = "fixtures/published_assignment.rs"]
mod published_assignment;
use published_assignment::create_published_assignment;

use domain::effective_assignment_policy::{
    BaseAssignmentPolicy, GroupAccommodation, GroupScheduleOffset, IndividualPolicyException,
    PolicyModificationMode, PolicyPatch, PolicyPatchSet, PolicySource, ScheduleOffsetSeconds,
};
use learning_data_access::postgres::{PostgresStore, lazy_pool, verify_application_schema};
use learning_data_access::{
    AssignmentRecord, CatalogStore, CourseGroupRecord, CourseRecord, CourseRosterStore,
    CreateCourseCommand, DraftRecord, FlatGradingCapability, IssueQuestionAttemptCommand,
    IssuedQuestionFamilyWitnessV1, IssuedQuestionSnapshotV1, LearnerWorkRoutingBinding,
    NativeExecutionEnvelopeCapability, PresentationCapability,
    PutAssignmentTeachingSettingsCommand, PutCourseGroupCommand, PutGroupAccommodationCommand,
    PutGroupScheduleOffsetCommand, PutIndividualPolicyExceptionCommand, QtiGradingCapability,
    ResolveEffectivePolicyCommand, Store, StoredIndividualPolicyException, TenantContext,
    UpsertCourseMember, WebworkGradingCapability,
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

const TERM_BASE_MILLIS: i64 = 1_787_590_800_000;

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

struct IssueCommandFixture<'a> {
    store: &'a PostgresStore,
    context: TenantContext,
    learner: UserId,
    run: RunId,
    attempt: QuestionAttemptId,
    reference: ProblemVersionRef,
    course: CourseId,
    assignment: AssignmentId,
}

impl IssueCommandFixture<'_> {
    async fn build(self) -> IssueQuestionAttemptCommand {
        let question = self
            .store
            .get_catalog_problem(self.context, self.reference)
            .await
            .expect("read the published effective-policy question")
            .expect("published effective-policy question exists")
            .question;
        let issued_question_snapshot = IssuedQuestionSnapshotV1::new(
            question,
            IssuedQuestionFamilyWitnessV1::Native {
                physical_asset_bindings: Vec::new(),
            },
        )
        .expect("construct exact effective-policy native question snapshot");
        IssueQuestionAttemptCommand {
            actor: self.learner,
            binding: LearnerWorkRoutingBinding::new(self.course, self.assignment),
            attempt: self.attempt,
            run: self.run,
            assignment_position: 0,
            problem: self.reference.problem,
            question_version: self.reference.version,
            issued_question_snapshot,
            seed: 1,
            presentation_capability: PresentationCapability::NotApplicable,
            presentation: None,
            presentation_snapshot: None,
            grading_envelope: None,
            native_execution_envelope_capability: NativeExecutionEnvelopeCapability::Required,
            flat_grading: None,
            flat_grading_capability: FlatGradingCapability::NotApplicable,
            webwork_grading: None,
            webwork_grading_capability: WebworkGradingCapability::NotApplicable,
            qti_grading: None,
            qti_grading_capability: QtiGradingCapability::NotApplicable,
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

async fn application_cannot_update_assignments(pool: &PgPool) {
    let allowed: bool =
        sqlx::query_scalar("SELECT has_table_privilege('ple_app', 'public.assignment', 'UPDATE')")
            .fetch_one(pool)
            .await
            .expect("read assignment update privilege");
    assert!(
        !allowed,
        "ple_app must use the assignment broker capability"
    );
}

async fn active_attempt_witness(
    pool: &PgPool,
    tenant: TenantId,
    actor: UserId,
    course: CourseId,
    assignment: AssignmentId,
    revision: learning_data_access::AssignmentRevision,
) -> (String, i64, i64, Vec<Uuid>) {
    let mut transaction = pool.begin().await.expect("begin witness transaction");
    sqlx::query("SET LOCAL ROLE ple_app")
        .execute(&mut *transaction)
        .await
        .expect("assume application role for witness");
    sqlx::query("SELECT set_config('ple.tenant_id', $1, true)")
        .bind(tenant.to_string())
        .execute(&mut *transaction)
        .await
        .expect("scope witness transaction");
    let witness = sqlx::query_as::<_, (String, i64, i64, Vec<Uuid>)>(
        "SELECT * FROM public.ple_prepare_assignment_active_attempt_reresolution($1,$2,$3,$4,$5)",
    )
    .bind(tenant.as_uuid())
    .bind(actor.as_uuid())
    .bind(course.as_uuid())
    .bind(assignment.as_uuid())
    .bind(i64::try_from(revision.value()).expect("revision fits PostgreSQL bigint"))
    .fetch_one(&mut *transaction)
    .await
    .expect("prepare opaque active-attempt witness");
    transaction
        .rollback()
        .await
        .expect("rollback witness transaction");
    witness
}

#[tokio::test]
#[ignore = "requires the disposable PostgreSQL 17 database baseline"]
async fn postgres_effective_policy_is_normalized_precedence_bound_and_rls_enforced() {
    let runtime = load_acceptance_runtime();
    let database_url = runtime.admin_url().expose();
    let pool = lazy_pool(database_url).expect("valid live PostgreSQL URL");
    verify_application_schema(&pool)
        .await
        .expect("full migrated application schema is compatible");
    application_cannot_update_assignments(&pool).await;
    let store = PostgresStore::with_question_id_secret(pool.clone(), [0x34; 32]);
    effective_policy_parity::exercise_effective_policy_resolution_parity(&store).await;
    let tenant = TenantId::from_uuid(id());
    let other_tenant = TenantId::from_uuid(id());
    let context = TenantContext::from_authenticated_session(tenant);
    let other_context = TenantContext::from_authenticated_session(other_tenant);
    let instructor = UserId::from_uuid(id());
    let learner = UserId::from_uuid(id());
    let second_learner = UserId::from_uuid(id());
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
                authority: sysadmin_course_creation_authority(&store, tenant, course, instructor)
                    .await,
            },
        )
        .await
        .expect("create course");
    store
        .upsert_course_member(
            context,
            instructor,
            UpsertCourseMember {
                course,
                user: learner,
                display_name: "S3 learner".to_string(),
                roster_contact: None,
            },
        )
        .await
        .expect("create learner membership");
    store
        .upsert_course_member(
            context,
            instructor,
            UpsertCourseMember {
                course,
                user: second_learner,
                display_name: "S3 second learner".to_string(),
                roster_contact: None,
            },
        )
        .await
        .expect("create second learner membership");
    let student = store
        .get_current_course_membership(context, course, learner)
        .await
        .expect("read learner membership")
        .expect("learner membership exists")
        .student
        .expect("learner has stable student identity");
    let reference = publish_question(&store, context, tenant, instructor).await;
    let assignment = AssignmentId::from_uuid(id());
    let created = create_published_assignment(
        &store,
        context,
        instructor,
        AssignmentRecord {
            id: assignment,
            tenant,
            course_id: course,
            title: "S3 normalized policy assignment".to_string(),
            lifecycle: question_model::AssignmentLifecycle::Published,
            instructions: question_model::AssignmentInstructions::default(),
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
        BaseAssignmentPolicy::default(),
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
        .put_assignment_teaching_settings(
            context,
            PutAssignmentTeachingSettingsCommand {
                actor: instructor,
                course,
                assignment,
                expected_revision: revised,
                settings: question_model::AssignmentTeachingSettings {
                    lifecycle: question_model::AssignmentLifecycle::Published,
                    instructions: question_model::AssignmentInstructions::default(),
                    base_policy: BaseAssignmentPolicy {
                        available_at: None,
                        due_at: None,
                        closes_at: None,
                        time_limit_seconds: Some(NonZeroU32::new(120).expect("positive limit")),
                        attempt_limit: None,
                        late_submission: LateSubmissionPolicy::Accept,
                        deadline_behavior: question_model::AssignmentDeadlineBehavior::AutoSubmit,
                    },
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
                entitlement: grant,
                authorization: domain::effective_assignment_policy::AuthorizationGate::Authorized,
                now: ActivityTimestamp::from_unix_millis(TERM_BASE_MILLIS + 1_000),
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
                entitlement: domain::entitlement::EntitlementDecision::Denied(
                    domain::entitlement::EntitlementDenial::LearnerNotActiveCourseStudent,
                ),
                authorization: domain::effective_assignment_policy::AuthorizationGate::Authorized,
                now: ActivityTimestamp::from_unix_millis(TERM_BASE_MILLIS + 1_000),
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
        .start_or_resume_run(
            context,
            learner,
            LearnerWorkRoutingBinding::new(course, assignment),
            RunId::from_uuid(id()),
        )
        .await
        .expect("granted learner starts atomically");
    let issued = store
        .issue_or_resume_question_attempt(
            context,
            IssueCommandFixture {
                store: &store,
                context,
                learner,
                run: run.id,
                attempt: QuestionAttemptId::from_uuid(id()),
                reference,
                course,
                assignment,
            }
            .build()
            .await,
        )
        .await
        .expect("issue attempt with sealed policy receipt");
    let second_run = store
        .start_or_resume_run(
            context,
            second_learner,
            LearnerWorkRoutingBinding::new(course, assignment),
            RunId::from_uuid(id()),
        )
        .await
        .expect("start a second active run for the broker witness");
    let second_issued = store
        .issue_or_resume_question_attempt(
            context,
            IssueCommandFixture {
                store: &store,
                context,
                learner: second_learner,
                run: second_run.id,
                attempt: QuestionAttemptId::from_uuid(id()),
                reference,
                course,
                assignment,
            }
            .build()
            .await,
        )
        .await
        .expect("issue the second active attempt");
    let receipt = store
        .get_issued_effective_policy_receipt(context, issued.id)
        .await
        .expect("read sealed receipt")
        .expect("issued attempt has receipt");
    assert_eq!(receipt.generation, 1);
    let second_receipt = store
        .get_issued_effective_policy_receipt(context, second_issued.id)
        .await
        .expect("read second sealed receipt")
        .expect("second active attempt has receipt");
    assert_eq!(second_receipt.generation, 1);
    let revision_before_change = store
        .get_assignment_for_edit(context, assignment)
        .await
        .expect("read assignment before broker witness")
        .expect("assignment exists")
        .revision;
    let witness = active_attempt_witness(
        &pool,
        tenant,
        instructor,
        course,
        assignment,
        revision_before_change,
    )
    .await;
    let mut expected_attempts = vec![issued.id.as_uuid(), second_issued.id.as_uuid()];
    expected_attempts.sort_unstable();
    assert_eq!(witness.0, "published");
    assert_eq!(
        witness.1,
        i64::try_from(revision_before_change.value()).expect("revision fits")
    );
    assert_eq!(witness.2, 2, "both active attempts are witnessed");
    assert_eq!(witness.3, expected_attempts, "witness is exact and sorted");
    let witness_result: String = sqlx::query_scalar(
        "SELECT pg_get_function_result(\
         'public.ple_prepare_assignment_active_attempt_reresolution(uuid,uuid,uuid,uuid,bigint)'::regprocedure)",
    )
    .fetch_one(&pool)
    .await
    .expect("inspect active-attempt witness result contract");
    for private_name in ["payload", "student", "response", "evidence"] {
        assert!(
            !witness_result.contains(private_name),
            "opaque witness must not expose private learner field {private_name}"
        );
    }

    let changed = store
        .put_assignment_teaching_settings(
            context,
            PutAssignmentTeachingSettingsCommand {
                actor: instructor,
                course,
                assignment,
                expected_revision: configured.revision,
                settings: question_model::AssignmentTeachingSettings {
                    lifecycle: question_model::AssignmentLifecycle::Published,
                    instructions: question_model::AssignmentInstructions::default(),
                    base_policy: BaseAssignmentPolicy {
                        time_limit_seconds: Some(NonZeroU32::new(180).expect("positive limit")),
                        ..configured.policy
                    },
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
    let second_current_receipt = store
        .get_issued_effective_policy_receipt(context, second_issued.id)
        .await
        .expect("read second active receipt after policy edit")
        .expect("second active attempt retains a policy receipt");
    assert!(second_current_receipt.generation > second_receipt.generation);
    let current_effects: Vec<(Uuid, i64, i64, Option<Uuid>)> = sqlx::query_as(
        "SELECT attempt_id, receipt_generation, timing_generation, job_id \
         FROM attempt_effective_policy_current \
         WHERE tenant_id=$1 AND attempt_id = ANY($2) ORDER BY attempt_id",
    )
    .bind(tenant.as_uuid())
    .bind(expected_attempts.clone())
    .fetch_all(&pool)
    .await
    .expect("read active effect pointers");
    assert_eq!(current_effects.len(), 2);
    assert!(
        current_effects.iter().all(|(_, generation, timing, job)| {
            *generation > 1 && *timing > 1 && job.is_some()
        })
    );
    let jobs: Vec<(Uuid, String)> = sqlx::query_as(
        "SELECT job_id, state FROM worker_job WHERE tenant_id=$1 AND job_id = ANY($2) ORDER BY job_id",
    )
    .bind(tenant.as_uuid())
    .bind(
        current_effects
            .iter()
            .filter_map(|(_, _, _, job)| *job)
            .collect::<Vec<_>>(),
    )
    .fetch_all(&pool)
    .await
    .expect("read rescheduled timing jobs");
    assert_eq!(
        jobs.len(),
        2,
        "each active attempt receives one current job"
    );
    assert!(jobs.iter().all(|(_, state)| state == "ready"));
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
    let stale = store
        .put_assignment_teaching_settings(
            context,
            PutAssignmentTeachingSettingsCommand {
                actor: instructor,
                course,
                assignment,
                expected_revision: configured.revision,
                settings: question_model::AssignmentTeachingSettings {
                    lifecycle: question_model::AssignmentLifecycle::Published,
                    instructions: question_model::AssignmentInstructions::default(),
                    base_policy: BaseAssignmentPolicy {
                        time_limit_seconds: Some(NonZeroU32::new(240).expect("positive limit")),
                        ..changed.policy
                    },
                },
            },
        )
        .await;
    assert!(
        matches!(stale, Err(learning_data_access::StoreError::Conflict)),
        "stale broker preparation refuses the mutation before changing effects"
    );
    assert_eq!(
        store
            .get_assignment_for_edit(context, assignment)
            .await
            .expect("read assignment after stale mutation")
            .expect("assignment remains")
            .revision,
        changed.revision
    );
    assert_eq!(
        store
            .get_issued_effective_policy_receipt(context, issued.id)
            .await
            .expect("read first receipt after stale mutation")
            .expect("first receipt remains")
            .generation,
        current_receipt.generation
    );
    assert_eq!(
        store
            .get_issued_effective_policy_receipt(context, second_issued.id)
            .await
            .expect("read second receipt after stale mutation")
            .expect("second receipt remains")
            .generation,
        second_current_receipt.generation
    );
    let original_payload: (serde_json::Value, String) = sqlx::query_as(
        "SELECT payload, payload_sha256::text FROM question_attempt \
         WHERE tenant_id=$1 AND attempt_id=$2",
    )
    .bind(tenant.as_uuid())
    .bind(issued.id.as_uuid())
    .fetch_one(&pool)
    .await
    .expect("capture immutable attempt payload before failure injection");
    let corruption_error = sqlx::query(
        "UPDATE question_attempt SET payload='null'::jsonb, payload_sha256=repeat('0',64) \
         WHERE tenant_id=$1 AND attempt_id=$2",
    )
    .bind(tenant.as_uuid())
    .bind(issued.id.as_uuid())
    .execute(&pool)
    .await
    .expect_err("immutable attempt guard rejects malformed payload corruption");
    assert_eq!(
        corruption_error
            .as_database_error()
            .and_then(|error| error.code())
            .as_deref(),
        Some("22023")
    );
    assert_eq!(
        store
            .get_issued_effective_policy_receipt(context, issued.id)
            .await
            .expect("read first receipt after rejected corruption")
            .expect("first receipt remains")
            .generation,
        current_receipt.generation
    );
    assert_eq!(
        store
            .get_assignment_for_edit(context, assignment)
            .await
            .expect("read assignment after rejected corruption")
            .expect("assignment remains")
            .revision,
        changed.revision
    );
    assert_eq!(
        store
            .get_issued_effective_policy_receipt(context, second_issued.id)
            .await
            .expect("read second receipt after rejected corruption")
            .expect("second receipt remains")
            .generation,
        second_current_receipt.generation
    );
    let effects_after_failure: Vec<(Uuid, i64, i64, Option<Uuid>)> = sqlx::query_as(
        "SELECT attempt_id, receipt_generation, timing_generation, job_id \
         FROM attempt_effective_policy_current \
         WHERE tenant_id=$1 AND attempt_id = ANY($2) ORDER BY attempt_id",
    )
    .bind(tenant.as_uuid())
    .bind(expected_attempts.clone())
    .fetch_all(&pool)
    .await
    .expect("read effect pointers after rejected corruption");
    assert_eq!(effects_after_failure, current_effects);
    let job_ids = jobs.iter().map(|(job, _)| *job).collect::<Vec<_>>();
    let jobs_after_failure: Vec<(Uuid, String)> = sqlx::query_as(
        "SELECT job_id, state FROM worker_job WHERE tenant_id=$1 AND job_id = ANY($2) ORDER BY job_id",
    )
    .bind(tenant.as_uuid())
    .bind(job_ids)
    .fetch_all(&pool)
    .await
    .expect("read timing jobs after rejected corruption");
    assert_eq!(jobs_after_failure, jobs);
    let retained_payload: (serde_json::Value, String) = sqlx::query_as(
        "SELECT payload, payload_sha256::text FROM question_attempt \
         WHERE tenant_id=$1 AND attempt_id=$2",
    )
    .bind(tenant.as_uuid())
    .bind(issued.id.as_uuid())
    .fetch_one(&pool)
    .await
    .expect("read immutable attempt payload after rejected corruption");
    assert_eq!(retained_payload, original_payload);
    assert!(
        store
            .get_base_assignment_policy(other_context, assignment)
            .await
            .expect("foreign RLS read")
            .is_none()
    );
    student_cannot_write_policy_relations(&pool, tenant).await;
}
#[path = "support/acceptance_runtime.rs"]
mod acceptance_runtime;
use acceptance_runtime::load as load_acceptance_runtime;

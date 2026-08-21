#![cfg(feature = "postgres")]

//! Disposable PostgreSQL 17 oracle entry point for the T2 group boundary.
//!
//! It intentionally starts by proving that a disposable endpoint presents the
//! fully migrated application schema.  The adjacent S3/S5 ignored live tests
//! remain the source-owned fixtures for publication, issue, receipt-history,
//! and RLS mechanics; this file reserves the behavior-named T2 entry point
//! without creating a divergent private fixture.

#[path = "fixtures/published_assignment.rs"]
mod published_assignment;

use domain::effective_assignment_policy::{
    BaseAssignmentPolicy, GroupAccommodation, GroupScheduleOffset, IndividualPolicyException,
    PolicyModificationMode, PolicyPatch, PolicyPatchSet, PolicySource, ScheduleOffsetSeconds,
};
use learning_data_access::postgres::{PostgresStore, lazy_pool, verify_application_schema};
use learning_data_access::{
    AssignmentRecord, AssignmentUpdate, CatalogStore, CourseGroupManagementStore,
    CourseGroupMembershipWarning, CourseGroupRecord, CourseRecord, CourseRosterStore,
    CreateCourseCommand, DeleteGroupAccommodationCommand, DeleteGroupScheduleOffsetCommand,
    DeleteIndividualPolicyExceptionCommand, DraftRecord, FlatGradingCapability,
    IssueQuestionAttemptCommand, PageRequest, PageSize, PresentationCapability,
    PutCourseGroupCommand, PutGroupAccommodationCommand, PutGroupScheduleOffsetCommand,
    PutIndividualPolicyExceptionCommand, Store, StoreError, StoredIndividualPolicyException,
    TenantContext, UpsertCourseMember, WebworkGradingCapability,
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
    AssignmentPolicyExceptionId, AssignmentScoringMode, AttemptStatus, BackendCapabilities,
    Capability, CourseGroupId, CourseGroupPurpose, CourseId, DraftQuestionDefinition,
    DraftQuestionSource, GradingDefinition, ImplementationVersion, PointValue, ProblemId,
    ProblemVersionRef, PublicationScope, QuestionAttemptId, QuestionMetadata, QuestionSource,
    ResponseDefinition, RunId, TenantId, UserId, VersionId, WorkspaceId,
};
use std::num::NonZeroU32;
use uuid::Uuid;

fn id() -> Uuid {
    let mut bytes = [0; 16];
    getrandom::fill(&mut bytes).expect("uuid");
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

async fn publish(
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
                family: "molar_mass".into(),
            },
            prompt: vec![ContentBlock::Text {
                markdown: "T2 group fixture".into(),
            }],
            response: ResponseDefinition::Numeric {
                tolerance: NumericTolerance::Relative { fraction: 0.01 },
                unit: None,
            },
            attempt_policy: AttemptPolicy { max_attempts: None },
            timing_policy: TimingPolicy::Untimed,
            randomization: RandomizationDefinition::Static,
            grading: GradingDefinition::AllOrNothing { points: 1.0 },
            metadata: QuestionMetadata {
                title: "T2 fixture".into(),
                tags: vec![],
                taxonomy: vec![],
                license: License::CcBy,
                language: "en-US".into(),
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
                    family: "molar_mass".into(),
                },
                publisher: instructor,
                scope: PublicationScope::Public,
                byline: question_model::PublicByline::new(vec![
                    question_model::PublicAuthorName::new("PLE fixture".into()).expect("byline"),
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

fn issue(
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
        parameter_hash: "t2".into(),
        provenance: question_model::AttemptProvenance {
            adapter: ImplementationVersion {
                id: "t2".into(),
                version: "1".into(),
            },
            renderer: None,
            generator: None,
            source_artifact: None,
            asset_objects: vec![],
            grading: ImplementationVersion {
                id: "t2-grade".into(),
                version: "1".into(),
            },
            rendered_question_sha256: "t2".into(),
        },
        webwork_replay: None,
        prefetched: None,
        predecessor_submission: None,
    }
}
async fn current(
    store: &PostgresStore,
    context: TenantContext,
    attempt: QuestionAttemptId,
) -> learning_data_access::IssuedEffectivePolicyReceipt {
    store
        .get_issued_effective_policy_receipt(context, attempt)
        .await
        .expect("receipt")
        .expect("current receipt")
}
async fn revision(
    store: &PostgresStore,
    context: TenantContext,
    assignment: AssignmentId,
) -> learning_data_access::AssignmentRevision {
    store
        .get_assignment_for_edit(context, assignment)
        .await
        .expect("assignment")
        .expect("assignment")
        .revision
}

fn assignment_update(
    record: &AssignmentRecord,
    audience: question_model::AssignmentAudience,
) -> AssignmentUpdate {
    AssignmentUpdate {
        title: record.title.clone(),
        audience,
        items: record.items.clone(),
        selection_groups: record.selection_groups.clone(),
        disclosure_policy: record.disclosure_policy,
        policies: record.policies,
    }
}

async fn receipt_generations(
    pool: &sqlx::PgPool,
    tenant: TenantId,
    attempt: QuestionAttemptId,
) -> Vec<i64> {
    sqlx::query_scalar(
        "SELECT receipt_generation FROM attempt_effective_policy_receipt \
         WHERE tenant_id=$1 AND attempt_id=$2 ORDER BY receipt_generation",
    )
    .bind(tenant.as_uuid())
    .bind(attempt.as_uuid())
    .fetch_all(pool)
    .await
    .expect("physical receipt history")
}

async fn group_rows_for_app(pool: &sqlx::PgPool, tenant: Option<TenantId>) -> (i64, i64) {
    let mut tx = pool.begin().await.expect("RLS probe transaction");
    sqlx::query("SET LOCAL ROLE ple_app")
        .execute(&mut *tx)
        .await
        .expect("application role");
    if let Some(tenant) = tenant {
        sqlx::query("SELECT set_config('ple.tenant_id', $1, true)")
            .bind(tenant.to_string())
            .execute(&mut *tx)
            .await
            .expect("tenant context");
    }
    let groups = sqlx::query_scalar("SELECT count(*) FROM course_group")
        .fetch_one(&mut *tx)
        .await
        .expect("group RLS count");
    let policies = sqlx::query_scalar("SELECT count(*) FROM course_group_membership_policy")
        .fetch_one(&mut *tx)
        .await
        .expect("policy RLS count");
    tx.rollback().await.expect("RLS probe rollback");
    (groups, policies)
}

#[tokio::test]
#[ignore = "requires the disposable PostgreSQL 17 database baseline"]
async fn postgres_course_group_changes_keep_sealed_effective_policy_history() {
    let database_url = std::env::var("PLE_TEST_DATABASE_URL")
        .expect("PLE_TEST_DATABASE_URL must name the disposable acceptance database");
    let pool = lazy_pool(&database_url).expect("valid disposable PostgreSQL URL");
    verify_application_schema(&pool)
        .await
        .expect("full migrated application schema is compatible");
    let store = PostgresStore::with_question_id_secret(pool.clone(), [0x72; 32]);
    let tenant = TenantId::from_uuid(id());
    let context = TenantContext::from_authenticated_session(tenant);
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
                    title: "T2 group oracle".into(),
                    term: question_model::CourseTerm::from_parts(
                        "2026-08-24",
                        "2026-12-18",
                        "America/Chicago",
                    )
                    .expect("term"),
                },
                initial_instructor: instructor,
            },
        )
        .await
        .expect("course");
    store
        .upsert_course_member(
            context,
            UpsertCourseMember {
                course,
                user: learner,
                display_name: "T2 learner".into(),
                roster_contact: None,
            },
        )
        .await
        .expect("member");
    let membership = store
        .get_current_course_membership(context, course, learner)
        .await
        .expect("member read")
        .expect("member");
    let student = membership.student.expect("student");
    for purpose in [
        CourseGroupPurpose::Section,
        CourseGroupPurpose::Lab,
        CourseGroupPurpose::Cohort,
        CourseGroupPurpose::Accommodation,
        CourseGroupPurpose::Work,
    ] {
        assert_eq!(
            store
                .get_course_group_purpose_policy(context, instructor, course, purpose)
                .await
                .expect("policy")
                .expect("policy")
                .policy,
            question_model::CourseGroupPurposePolicy::default_for_purpose(purpose)
        );
    }
    let section = CourseGroupId::from_uuid(id());
    let section_view = store
        .put_course_group(
            context,
            PutCourseGroupCommand {
                actor: instructor,
                expected_revision: None,
                record: CourseGroupRecord {
                    id: section,
                    tenant,
                    course,
                    purpose: CourseGroupPurpose::Section,
                    title: "Section".into(),
                    members: vec![membership.id],
                },
            },
        )
        .await
        .expect("section");
    let lab = CourseGroupId::from_uuid(id());
    store
        .put_course_group(
            context,
            PutCourseGroupCommand {
                actor: instructor,
                expected_revision: None,
                record: CourseGroupRecord {
                    id: lab,
                    tenant,
                    course,
                    purpose: CourseGroupPurpose::Lab,
                    title: "Lab".into(),
                    members: vec![membership.id],
                },
            },
        )
        .await
        .expect("lab");
    let accommodation = CourseGroupId::from_uuid(id());
    let accommodation_view = store
        .put_course_group(
            context,
            PutCourseGroupCommand {
                actor: instructor,
                expected_revision: None,
                record: CourseGroupRecord {
                    id: accommodation,
                    tenant,
                    course,
                    purpose: CourseGroupPurpose::Accommodation,
                    title: "Accommodation".into(),
                    members: vec![membership.id],
                },
            },
        )
        .await
        .expect("accommodation");
    let audience = CourseGroupId::from_uuid(id());
    let audience_view = store
        .put_course_group(
            context,
            PutCourseGroupCommand {
                actor: instructor,
                expected_revision: None,
                record: CourseGroupRecord {
                    id: audience,
                    tenant,
                    course,
                    purpose: CourseGroupPurpose::Section,
                    title: "Audience".into(),
                    members: vec![membership.id],
                },
            },
        )
        .await
        .expect("audience");
    let unused = store
        .put_course_group(
            context,
            PutCourseGroupCommand {
                actor: instructor,
                expected_revision: None,
                record: CourseGroupRecord {
                    id: CourseGroupId::from_uuid(id()),
                    tenant,
                    course,
                    purpose: CourseGroupPurpose::Work,
                    title: "Unused".into(),
                    members: vec![],
                },
            },
        )
        .await
        .expect("unreferenced group");
    let warnings = store
        .course_group_membership_warnings(context, instructor, course)
        .await
        .expect("warnings");
    assert_eq!(
        warnings,
        vec![CourseGroupMembershipWarning {
            membership: membership.id,
            purpose: CourseGroupPurpose::Section,
            membership_count: 2,
            disposition: question_model::MultipleMembershipDisposition::AllowedWithWarning,
        }],
        "Section warns, while the single Lab membership remains allowed without a warning"
    );
    let page_size = PageSize::new(1).expect("size");
    let mut next_cursor = None;
    let mut references = Vec::new();
    for page_number in 0..5 {
        let request = match next_cursor.take() {
            Some(cursor) => PageRequest::after(cursor, page_size),
            None => PageRequest::first(page_size),
        };
        let page = store
            .list_course_groups(context, instructor, course, request)
            .await
            .expect("page");
        assert_eq!(page.items.len(), 1, "page {page_number} has one group");
        references.push(page.items[0].reference.number());
        next_cursor = page.next_cursor;
        assert_eq!(
            next_cursor.is_some(),
            page_number < 4,
            "continuation cursor reaches each expected group exactly once"
        );
    }
    assert_eq!(
        references.len(),
        5,
        "pagination returns every created group"
    );
    assert!(
        references.windows(2).all(|pair| pair[0] < pair[1]),
        "pagination follows strictly increasing numeric group references"
    );
    let mut unique_references = references.clone();
    unique_references.sort_unstable();
    unique_references.dedup();
    assert_eq!(
        unique_references.len(),
        references.len(),
        "pagination has no duplicate group"
    );
    assert_eq!(group_rows_for_app(&pool, None).await, (0, 0));
    assert_eq!(
        group_rows_for_app(&pool, Some(TenantId::from_uuid(id()))).await,
        (0, 0)
    );
    let reference = publish(&store, context, tenant, instructor).await;
    let assignment = AssignmentId::from_uuid(id());
    published_assignment::create_published_assignment(
        &store,
        context,
        instructor,
        AssignmentRecord {
            id: assignment,
            tenant,
            course_id: course,
            title: "T2 assignment".into(),
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
            selection_groups: vec![],
            disclosure_policy: question_model::LearnerDisclosurePolicy::default(),
            policies: policies(),
        },
        BaseAssignmentPolicy {
            due_at: Some(ActivityTimestamp::from_unix_millis(1_797_465_600_000)),
            time_limit_seconds: Some(NonZeroU32::new(60).expect("limit")),
            ..BaseAssignmentPolicy::default()
        },
    )
    .await
    .expect("assignment");
    let run = store
        .start_or_resume_run(context, learner, assignment, RunId::from_uuid(id()))
        .await
        .expect("run");
    let attempt = store
        .issue_or_resume_question_attempt(
            context,
            issue(
                learner,
                run.id,
                QuestionAttemptId::from_uuid(id()),
                reference,
            ),
        )
        .await
        .expect("attempt");
    let first = current(&store, context, attempt.id).await;
    let after_m2 = store
        .put_group_schedule_offset(
            context,
            PutGroupScheduleOffsetCommand {
                actor: instructor,
                course,
                assignment,
                expected_revision: revision(&store, context, assignment).await,
                offset: GroupScheduleOffset {
                    group: section,
                    offset_seconds: ScheduleOffsetSeconds::try_new(60).expect("offset"),
                },
            },
        )
        .await
        .expect("M2");
    let m2 = current(&store, context, attempt.id).await;
    assert!(m2.generation > first.generation);
    assert_eq!(
        m2.policy.due_at.source,
        PolicySource::GroupScheduleOffsets(vec![section])
    );
    assert_eq!(
        m2.policy.due_at.value,
        Some(ActivityTimestamp::from_unix_millis(1_797_465_660_000))
    );
    assert!(matches!(
        store
            .delete_course_group(context, instructor, course, section, section_view.revision)
            .await,
        Err(StoreError::Conflict)
    ));
    let mut wrong_section = section_view.record.clone();
    wrong_section.purpose = CourseGroupPurpose::Work;
    assert!(
        store
            .put_course_group(
                context,
                PutCourseGroupCommand {
                    actor: instructor,
                    expected_revision: Some(section_view.revision),
                    record: wrong_section,
                },
            )
            .await
            .is_err(),
        "referenced M2 group refuses a purpose transition"
    );
    assert_eq!(
        store
            .get_course_group(context, section)
            .await
            .expect("section"),
        Some(section_view.clone()),
        "failed referenced purpose transition preserves group"
    );
    let after_m2_delete = store
        .delete_group_schedule_offset(
            context,
            DeleteGroupScheduleOffsetCommand {
                actor: instructor,
                course,
                assignment,
                expected_revision: after_m2,
                group: section,
            },
        )
        .await
        .expect("M2 delete");
    let m2_deleted = current(&store, context, attempt.id).await;
    assert!(m2_deleted.generation > m2.generation);
    assert_eq!(m2_deleted.policy.due_at.source, PolicySource::Base);
    assert_eq!(
        m2_deleted.policy.due_at.value,
        Some(ActivityTimestamp::from_unix_millis(1_797_465_600_000))
    );
    let revision_before_invalid = revision(&store, context, assignment).await;
    let after_m3 = store
        .put_group_accommodation(
            context,
            PutGroupAccommodationCommand {
                actor: instructor,
                course,
                assignment,
                expected_revision: after_m2_delete,
                accommodation: GroupAccommodation {
                    group: lab,
                    mode: PolicyModificationMode::ExtendOnly,
                    patch: PolicyPatchSet {
                        time_limit_seconds: PolicyPatch::Set(NonZeroU32::new(300).expect("limit")),
                        ..PolicyPatchSet::INHERIT
                    },
                },
            },
        )
        .await;
    assert!(
        matches!(after_m3, Err(StoreError::InvalidRecord(_))),
        "Lab cannot be an M3 accommodation"
    );
    assert_eq!(
        current(&store, context, attempt.id).await,
        m2_deleted,
        "invalid command rolls back"
    );
    assert_eq!(
        revision(&store, context, assignment).await,
        revision_before_invalid,
        "invalid M3 preserves assignment revision"
    );
    let group_before_invalid = store
        .get_course_group(context, lab)
        .await
        .expect("lab read");
    assert_eq!(
        group_before_invalid.as_ref().expect("lab").revision.value(),
        1
    );
    let after_m3 = store
        .put_group_accommodation(
            context,
            PutGroupAccommodationCommand {
                actor: instructor,
                course,
                assignment,
                expected_revision: revision(&store, context, assignment).await,
                accommodation: GroupAccommodation {
                    group: accommodation,
                    mode: PolicyModificationMode::ExtendOnly,
                    patch: PolicyPatchSet {
                        time_limit_seconds: PolicyPatch::Set(NonZeroU32::new(300).expect("limit")),
                        ..PolicyPatchSet::INHERIT
                    },
                },
            },
        )
        .await
        .expect("M3");
    let m3 = current(&store, context, attempt.id).await;
    assert!(m3.generation > m2_deleted.generation);
    assert_eq!(
        m3.policy.time_limit_seconds.source,
        PolicySource::GroupAccommodations(vec![accommodation])
    );
    assert_eq!(
        m3.policy.time_limit_seconds.value,
        Some(NonZeroU32::new(300).expect("limit"))
    );
    assert!(matches!(
        store
            .delete_course_group(
                context,
                instructor,
                course,
                accommodation,
                accommodation_view.revision,
            )
            .await,
        Err(StoreError::Conflict)
    ));
    let mut wrong_accommodation = accommodation_view.record.clone();
    wrong_accommodation.purpose = CourseGroupPurpose::Work;
    assert!(
        store
            .put_course_group(
                context,
                PutCourseGroupCommand {
                    actor: instructor,
                    expected_revision: Some(accommodation_view.revision),
                    record: wrong_accommodation,
                },
            )
            .await
            .is_err(),
        "referenced M3 group refuses a purpose transition"
    );
    assert_eq!(current(&store, context, attempt.id).await, m3);
    let _after_m3_delete = store
        .delete_group_accommodation(
            context,
            DeleteGroupAccommodationCommand {
                actor: instructor,
                course,
                assignment,
                expected_revision: after_m3,
                group: accommodation,
            },
        )
        .await
        .expect("M3 delete");
    let m3_deleted = current(&store, context, attempt.id).await;
    assert!(m3_deleted.generation > m3.generation);
    assert_eq!(
        m3_deleted.policy.time_limit_seconds.source,
        PolicySource::Base
    );
    assert_eq!(
        m3_deleted.policy.time_limit_seconds.value,
        Some(NonZeroU32::new(60).expect("limit"))
    );
    let after_m4 = store
        .put_individual_policy_exception(
            context,
            PutIndividualPolicyExceptionCommand {
                actor: instructor,
                course,
                assignment,
                expected_revision: revision(&store, context, assignment).await,
                exception: StoredIndividualPolicyException {
                    id: AssignmentPolicyExceptionId::from_uuid(id()),
                    exception: IndividualPolicyException {
                        student,
                        mode: PolicyModificationMode::Override,
                        patch: PolicyPatchSet {
                            time_limit_seconds: PolicyPatch::Set(
                                NonZeroU32::new(600).expect("limit"),
                            ),
                            ..PolicyPatchSet::INHERIT
                        },
                    },
                },
            },
        )
        .await
        .expect("M4");
    let m4 = current(&store, context, attempt.id).await;
    assert!(m4.generation > m3_deleted.generation);
    assert_eq!(
        m4.policy.time_limit_seconds.source,
        PolicySource::IndividualException(student)
    );
    assert_eq!(
        m4.policy.time_limit_seconds.value,
        Some(NonZeroU32::new(600).expect("limit"))
    );
    store
        .delete_individual_policy_exception(
            context,
            DeleteIndividualPolicyExceptionCommand {
                actor: instructor,
                course,
                assignment,
                expected_revision: after_m4,
                student,
            },
        )
        .await
        .expect("M4 delete");
    let m4_deleted = current(&store, context, attempt.id).await;
    assert!(m4_deleted.generation > m4.generation);
    assert_eq!(
        m4_deleted.policy.time_limit_seconds.source,
        PolicySource::Base
    );
    assert_eq!(
        m4_deleted.policy.time_limit_seconds.value,
        Some(NonZeroU32::new(60).expect("limit"))
    );

    let assigned = store
        .get_assignment_for_edit(context, assignment)
        .await
        .expect("assignment read")
        .expect("assignment");
    store
        .replace_assignment(
            context,
            course,
            assignment,
            assigned.revision,
            assignment_update(
                &assigned.record,
                question_model::AssignmentAudience::any_of_groups(vec![audience])
                    .expect("audience"),
            ),
        )
        .await
        .expect("audience narrowing");
    let audience_current = current(&store, context, attempt.id).await;
    assert!(audience_current.generation > m4_deleted.generation);
    let mut wrong_audience = audience_view.record.clone();
    wrong_audience.purpose = CourseGroupPurpose::Work;
    assert!(
        store
            .put_course_group(
                context,
                PutCourseGroupCommand {
                    actor: instructor,
                    expected_revision: Some(audience_view.revision),
                    record: wrong_audience,
                },
            )
            .await
            .is_err(),
        "referenced audience group refuses a purpose transition"
    );
    assert!(matches!(
        store
            .delete_course_group(
                context,
                instructor,
                course,
                audience,
                audience_view.revision
            )
            .await,
        Err(StoreError::Conflict)
    ));
    let mut revoked_audience = audience_view.record.clone();
    revoked_audience.members.clear();
    store
        .put_course_group(
            context,
            PutCourseGroupCommand {
                actor: instructor,
                expected_revision: Some(audience_view.revision),
                record: revoked_audience,
            },
        )
        .await
        .expect("audience removal");
    assert_eq!(
        store
            .get_issued_effective_policy_receipt(context, attempt.id)
            .await
            .expect("receipt read"),
        None,
        "current entitlement denial removes only the current pointer"
    );
    assert_eq!(
        store
            .get_question_attempt(context, attempt.id)
            .await
            .expect("attempt read")
            .expect("attempt")
            .status,
        AttemptStatus::AutoSubmitted
    );
    let generations = receipt_generations(&pool, tenant, attempt.id).await;
    assert!(
        generations.len() >= 8,
        "each retained transition has physical history"
    );
    assert!(
        generations.windows(2).all(|pair| pair[0] < pair[1]),
        "physical history retains distinct old receipt generations"
    );
    assert!(
        store
            .delete_course_group(
                context,
                instructor,
                course,
                unused.record.id,
                unused.revision
            )
            .await
            .expect("unreferenced group delete")
    );
}

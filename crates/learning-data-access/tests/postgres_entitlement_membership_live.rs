#![cfg(feature = "postgres")]

//! Disposable PostgreSQL 17 oracle for S5 current entitlement and receipts.
//!
//! This deliberately uses Store commands to construct teaching state.  The
//! narrowly scoped SQL below observes PostgreSQL-only facts: forced RLS,
//! receipt immutability, and least-privilege grants.

use learning_data_access::postgres::{PostgresStore, lazy_pool, verify_application_schema};
use learning_data_access::{
    AssignmentEntitlementMaterialization, AssignmentRecord, CatalogStore, CourseGroupRecord,
    CourseRecord, CourseRosterStore, CreateCourseCommand, DraftRecord,
    MaterializeAssignmentEntitlementCommand, PageRequest, PageSize, PutCourseGroupCommand,
    RevokeCourseMember, SessionLifetime, SessionStore, SessionSubject, SessionTokenHash, Store,
    StoreError, TenantContext, UpsertCourseMember,
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
    AssignmentAudience, AssignmentDeliveryState, AssignmentId, AssignmentItem, AssignmentItemId,
    AssignmentRunTiming, AssignmentScoringMode, BackendCapabilities, Capability, CourseGroupId,
    CourseGroupPurpose, CourseId, DraftQuestionDefinition, DraftQuestionSource, EntitlementPurpose,
    MaterializationBasis, PointValue, ProblemId, ProblemVersionRef, PublicationScope,
    QuestionMetadata, QuestionSource, ResponseDefinition, TenantId, UserId, UserRole, VersionId,
    WorkspaceId,
};
use sqlx::Row;
use uuid::Uuid;

#[path = "postgres_entitlement_membership_live/security_probes.rs"]
mod security_probes;
use security_probes::{denied_scope_append, denied_write};

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
                markdown: "S5 PostgreSQL entitlement fixture".to_string(),
            }],
            response: ResponseDefinition::Numeric {
                tolerance: NumericTolerance::Relative { fraction: 0.01 },
                unit: Some("g/mol".to_string()),
            },
            attempt_policy: AttemptPolicy { max_attempts: None },
            timing_policy: TimingPolicy::Untimed,
            randomization: RandomizationDefinition::Static,
            grading: question_model::GradingDefinition::AllOrNothing { points: 1.0 },
            metadata: QuestionMetadata {
                title: "S5 PostgreSQL entitlement fixture".to_string(),
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
        .expect("save entitlement fixture draft");
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
        .expect("publish entitlement fixture question");
    reference
}

fn assignment(
    tenant: TenantId,
    course: CourseId,
    assignment: AssignmentId,
    reference: ProblemVersionRef,
    audience: AssignmentAudience,
) -> AssignmentRecord {
    AssignmentRecord {
        id: assignment,
        tenant,
        course_id: course,
        title: "S5 entitlement assignment".to_string(),
        audience,
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
    }
}

#[tokio::test]
#[ignore = "requires the disposable PostgreSQL 17 database baseline"]
async fn postgres_entitlement_membership_is_derived_materialized_and_rls_enforced() {
    let database_url = std::env::var("PLE_TEST_DATABASE_URL")
        .expect("PLE_TEST_DATABASE_URL must name the disposable acceptance database");
    let pool = lazy_pool(&database_url).expect("valid disposable PostgreSQL URL");
    verify_application_schema(&pool)
        .await
        .expect("full migrated application schema is compatible");
    let store = PostgresStore::with_question_id_secret(pool.clone(), [0x53; 32]);
    let tenant = TenantId::from_uuid(id());
    let foreign_tenant = TenantId::from_uuid(id());
    let context = TenantContext::from_authenticated_session(tenant);
    let course = CourseId::from_uuid(id());
    let instructor = UserId::from_uuid(id());
    let student = UserId::from_uuid(id());
    let instructor_session = SessionTokenHash::compute(id().as_bytes());
    store
        .create_session(
            instructor_session,
            SessionSubject::new(
                tenant,
                instructor,
                "S5 entitlement live instructor",
                vec![UserRole::Instructor],
            )
            .expect("valid instructor session"),
            SessionLifetime::from_seconds(3_600).expect("bounded fixture session"),
        )
        .await
        .expect("persist instructor session for revocation");
    store
        .create_course(
            context,
            CreateCourseCommand {
                course: CourseRecord {
                    id: course,
                    tenant,
                    title: "S5 entitlement live course".to_string(),
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
        .expect("course and initial instructor are atomic");
    let initial = store
        .get_current_course_membership(context, course, instructor)
        .await
        .expect("read initial instructor membership")
        .expect("course creation persists instructor membership");
    assert_eq!(initial.user, instructor);
    let learner = store
        .upsert_course_member(
            context,
            UpsertCourseMember {
                course,
                user: student,
                display_name: "S5 learner".to_string(),
                roster_contact: None,
            },
        )
        .await
        .expect("activate canonical student membership");
    let membership = store
        .get_current_course_membership(context, course, student)
        .await
        .expect("read current student membership")
        .expect("student membership exists");
    assert_eq!(membership.id.as_uuid(), learner.member.id.as_uuid());

    let reference = publish_question(&store, context, tenant, instructor).await;
    let assignment_id = AssignmentId::from_uuid(id());
    store
        .create_assignment_with_timing(
            context,
            assignment(
                tenant,
                course,
                assignment_id,
                reference,
                AssignmentAudience::CourseWide,
            ),
            AssignmentRunTiming::default(),
        )
        .await
        .expect("create explicitly course-wide assignment");

    let outsider = UserId::from_uuid(id());
    assert_eq!(
        store
            .issue_assignment_entitlement(
                context,
                MaterializeAssignmentEntitlementCommand::for_instructor_action(
                    student,
                    course,
                    assignment_id,
                    outsider,
                    EntitlementPurpose::InstructorIssue,
                )
                .expect("typed outsider issue command"),
            )
            .await,
        Err(StoreError::Forbidden),
        "an actor without current exact-course instructor authority cannot mint provenance"
    );

    let command = MaterializeAssignmentEntitlementCommand::for_instructor_action(
        student,
        course,
        assignment_id,
        instructor,
        EntitlementPurpose::InstructorIssue,
    )
    .expect("typed instructor issue");
    let first = store
        .issue_assignment_entitlement(context, command)
        .await
        .expect("current course-wide learner is entitled");
    let AssignmentEntitlementMaterialization::Granted(first) = first else {
        panic!("current course-wide learner must receive a receipt")
    };
    assert_eq!(first.provenance.membership, membership.id);
    assert_eq!(first.provenance.basis, MaterializationBasis::CourseWide);
    let replay = store
        .issue_assignment_entitlement(context, command)
        .await
        .expect("idempotent explicit issue");
    let AssignmentEntitlementMaterialization::Granted(replay) = replay else {
        panic!("same current entitlement remains granted")
    };
    assert_eq!(replay.enrollment.id, first.enrollment.id);
    assert_eq!(
        replay.disposition,
        question_model::MaterializationDisposition::Existing,
        "a replay never creates another historical receipt"
    );

    let enrollment_count: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM enrollment WHERE tenant_id = $1 AND assignment_id = $2 AND student_id = $3",
    )
    .bind(tenant.as_uuid())
    .bind(assignment_id.as_uuid())
    .bind(first.enrollment.student.as_uuid())
    .fetch_one(&pool)
    .await
    .expect("inspect materialized enrollment count");
    assert_eq!(enrollment_count, 1);
    let receipt_counts = sqlx::query(
        "SELECT (SELECT count(*) FROM student_assignment_summary WHERE tenant_id = $1 AND enrollment_id = $2) AS summaries, \
                (SELECT count(*) FROM enrollment_entitlement_basis_receipt WHERE tenant_id = $1 AND enrollment_id = $2) AS bases",
    )
    .bind(tenant.as_uuid())
    .bind(first.enrollment.id.as_uuid())
    .fetch_one(&pool)
    .await
    .expect("inspect receipt companions");
    assert_eq!(receipt_counts.get::<i64, _>("summaries"), 1);
    assert_eq!(receipt_counts.get::<i64, _>("bases"), 1);

    let concurrent_assignment = AssignmentId::from_uuid(id());
    store
        .create_assignment_with_timing(
            context,
            assignment(
                tenant,
                course,
                concurrent_assignment,
                reference,
                AssignmentAudience::CourseWide,
            ),
            AssignmentRunTiming::default(),
        )
        .await
        .expect("create concurrent materialization assignment");
    let concurrent_command = MaterializeAssignmentEntitlementCommand::for_instructor_action(
        student,
        course,
        concurrent_assignment,
        instructor,
        EntitlementPurpose::InstructorIssue,
    )
    .expect("typed concurrent instructor issue");
    let (left, right) = tokio::join!(
        store.issue_assignment_entitlement(context, concurrent_command),
        store.issue_assignment_entitlement(context, concurrent_command),
    );
    let concurrent_enrollment = [left, right]
        .into_iter()
        .map(|result| match result.expect("concurrent issue completes") {
            AssignmentEntitlementMaterialization::Granted(value) => value.enrollment.id,
            AssignmentEntitlementMaterialization::Denied(reason) => {
                panic!("current learner unexpectedly denied during concurrent issue: {reason:?}")
            }
        })
        .collect::<Vec<_>>();
    assert_eq!(concurrent_enrollment[0], concurrent_enrollment[1]);
    let concurrent_count: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM enrollment WHERE tenant_id = $1 AND assignment_id = $2 AND student_id = $3",
    )
    .bind(tenant.as_uuid())
    .bind(concurrent_assignment.as_uuid())
    .bind(first.enrollment.student.as_uuid())
    .fetch_one(&pool)
    .await
    .expect("inspect concurrent receipt count");
    assert_eq!(concurrent_count, 1, "concurrent materialization serializes");

    denied_write(
        &pool,
        tenant,
        "UPDATE enrollment SET materialized_at = materialized_at WHERE false",
    )
    .await;
    denied_write(
        &pool,
        tenant,
        "UPDATE enrollment_entitlement_basis_receipt SET scope_kind = scope_kind WHERE false",
    )
    .await;
    denied_write(
        &pool,
        tenant,
        "DELETE FROM enrollment_applicable_policy_scope_receipt WHERE false",
    )
    .await;
    let materializer: Option<String> = sqlx::query_scalar(
        "SELECT to_regprocedure(\
         'public.ple_materialize_assignment_entitlement(uuid,uuid,uuid,uuid,uuid,uuid,uuid,text,uuid,text,integer,text,uuid,text,uuid[],text[])'\
         )::text",
    )
    .fetch_one(&pool)
    .await
    .expect("inspect removed materializer capability");
    assert!(
        materializer.is_none(),
        "application role must have no separately callable materialization authority"
    );

    let forced: bool = sqlx::query_scalar(
        "SELECT relforcerowsecurity FROM pg_class WHERE oid = 'public.enrollment'::regclass",
    )
    .fetch_one(&pool)
    .await
    .expect("inspect enrollment forced RLS");
    assert!(forced);
    let mut foreign = pool.begin().await.expect("begin cross-tenant RLS probe");
    sqlx::query("SET LOCAL ROLE ple_app")
        .execute(&mut *foreign)
        .await
        .expect("assume application role for RLS probe");
    sqlx::query("SELECT set_config('ple.tenant_id', $1, true)")
        .bind(foreign_tenant.to_string())
        .execute(&mut *foreign)
        .await
        .expect("scope foreign tenant");
    let visible: i64 =
        sqlx::query_scalar("SELECT count(*) FROM enrollment WHERE enrollment_id = $1")
            .bind(first.enrollment.id.as_uuid())
            .fetch_one(&mut *foreign)
            .await
            .expect("RLS query executes but filters foreign receipt");
    assert_eq!(visible, 0, "forced RLS hides foreign educational evidence");
    foreign.rollback().await.expect("rollback RLS probe");

    // Group audience is an OR over audience-capable group purposes.  The
    // pure model owns capability mapping; this Store setup proves the live
    // database accepts the canonical membership key rather than a user alias.
    let section = CourseGroupId::from_uuid(id());
    let section_group = store
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
                    title: "Thursday section".to_string(),
                    members: vec![membership.id],
                },
            },
        )
        .await
        .expect("section admits canonical student membership");
    let group_assignment = AssignmentId::from_uuid(id());
    store
        .create_assignment_with_timing(
            context,
            assignment(
                tenant,
                course,
                group_assignment,
                reference,
                AssignmentAudience::any_of_groups(vec![section]).expect("nonempty group audience"),
            ),
            AssignmentRunTiming::default(),
        )
        .await
        .expect("create section audience assignment");
    assert!(matches!(
        store
            .evaluate_assignment_entitlement(context, student, course, group_assignment)
            .await,
        Ok(domain::entitlement::EntitlementDecision::Granted(_))
    ));
    let outside_student = UserId::from_uuid(id());
    let outside = store
        .upsert_course_member(
            context,
            UpsertCourseMember {
                course,
                user: outside_student,
                display_name: "Outside section learner".to_string(),
                roster_contact: None,
            },
        )
        .await
        .expect("activate a learner outside the section");
    assert!(matches!(
        store
            .evaluate_assignment_entitlement(context, outside_student, course, group_assignment)
            .await,
        Ok(domain::entitlement::EntitlementDecision::Denied(_))
    ));

    // Cursor progression belongs to the visible sequence. Inaccessible rows
    // before and between granted assignments must not create empty pages or a
    // continuation that reveals their existence.
    let pagination_course = CourseId::from_uuid(id());
    store
        .create_course(
            context,
            CreateCourseCommand {
                course: CourseRecord {
                    id: pagination_course,
                    tenant,
                    title: "Entitlement pagination".to_string(),
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
        .expect("create isolated pagination course");
    store
        .upsert_course_member(
            context,
            UpsertCourseMember {
                course: pagination_course,
                user: student,
                display_name: "Visible pagination learner".to_string(),
                roster_contact: None,
            },
        )
        .await
        .expect("activate pagination learner");
    store
        .upsert_course_member(
            context,
            UpsertCourseMember {
                course: pagination_course,
                user: outside_student,
                display_name: "Hidden pagination learner".to_string(),
                roster_contact: None,
            },
        )
        .await
        .expect("activate hidden pagination learner");
    let pagination_outside_membership = store
        .get_current_course_membership(context, pagination_course, outside_student)
        .await
        .expect("read hidden pagination membership")
        .expect("hidden pagination membership exists");
    let hidden_group = CourseGroupId::from_uuid(id());
    store
        .put_course_group(
            context,
            PutCourseGroupCommand {
                actor: instructor,
                expected_revision: None,
                record: CourseGroupRecord {
                    id: hidden_group,
                    tenant,
                    course: pagination_course,
                    purpose: CourseGroupPurpose::Section,
                    title: "Other section".to_string(),
                    members: vec![pagination_outside_membership.id],
                },
            },
        )
        .await
        .expect("create hidden audience group");
    let first_visible = AssignmentId::from_uuid(Uuid::from_u128(98_101));
    let second_visible = AssignmentId::from_uuid(Uuid::from_u128(98_103));
    for (assignment_id, audience) in [
        (
            AssignmentId::from_uuid(Uuid::from_u128(98_100)),
            AssignmentAudience::any_of_groups(vec![hidden_group])
                .expect("nonempty hidden audience"),
        ),
        (first_visible, AssignmentAudience::CourseWide),
        (
            AssignmentId::from_uuid(Uuid::from_u128(98_102)),
            AssignmentAudience::any_of_groups(vec![hidden_group])
                .expect("nonempty hidden audience"),
        ),
        (second_visible, AssignmentAudience::CourseWide),
    ] {
        store
            .create_assignment_with_timing(
                context,
                assignment(
                    tenant,
                    pagination_course,
                    assignment_id,
                    reference,
                    audience,
                ),
                AssignmentRunTiming::default(),
            )
            .await
            .expect("create entitlement pagination assignment");
    }
    let first_page = store
        .list_learner_entitled_assignments(
            context,
            student,
            pagination_course,
            PageRequest::first(PageSize::new(1).expect("valid page size")),
        )
        .await
        .expect("first visible entitlement page");
    assert_eq!(
        first_page
            .items
            .iter()
            .map(|assignment| assignment.id)
            .collect::<Vec<_>>(),
        vec![first_visible]
    );
    let second_page = store
        .list_learner_entitled_assignments(
            context,
            student,
            pagination_course,
            PageRequest::after(
                first_page
                    .next_cursor
                    .expect("another visible assignment exists"),
                PageSize::new(1).expect("valid page size"),
            ),
        )
        .await
        .expect("second visible entitlement page");
    assert_eq!(
        second_page
            .items
            .iter()
            .map(|assignment| assignment.id)
            .collect::<Vec<_>>(),
        vec![second_visible]
    );
    assert!(second_page.next_cursor.is_none());

    store
        .put_course_group(
            context,
            PutCourseGroupCommand {
                actor: instructor,
                expected_revision: Some(section_group.revision),
                record: CourseGroupRecord {
                    id: section,
                    tenant,
                    course,
                    purpose: CourseGroupPurpose::Section,
                    title: "Thursday section".to_string(),
                    members: Vec::new(),
                },
            },
        )
        .await
        .expect("remove learner from the audience group");
    assert!(matches!(
        store
            .evaluate_assignment_entitlement(context, student, course, group_assignment)
            .await,
        Ok(domain::entitlement::EntitlementDecision::Denied(_))
    ));

    // This race is the PostgreSQL transaction boundary: either materialization
    // observes the still-current group and leaves one historical receipt, or
    // the audience edit wins and the issue is denied.  In both serial orders,
    // the final current decision is denied and no duplicate receipt can exist.
    let race_group = CourseGroupId::from_uuid(id());
    let race_group = store
        .put_course_group(
            context,
            PutCourseGroupCommand {
                actor: instructor,
                expected_revision: None,
                record: CourseGroupRecord {
                    id: race_group,
                    tenant,
                    course,
                    purpose: CourseGroupPurpose::Lab,
                    title: "Concurrent audience lab".to_string(),
                    members: vec![membership.id],
                },
            },
        )
        .await
        .expect("create race audience group");
    let race_assignment = AssignmentId::from_uuid(id());
    store
        .create_assignment_with_timing(
            context,
            assignment(
                tenant,
                course,
                race_assignment,
                reference,
                AssignmentAudience::any_of_groups(vec![race_group.record.id])
                    .expect("nonempty race audience"),
            ),
            AssignmentRunTiming::default(),
        )
        .await
        .expect("create race audience assignment");
    let race_issue = MaterializeAssignmentEntitlementCommand::for_instructor_action(
        student,
        course,
        race_assignment,
        instructor,
        EntitlementPurpose::InstructorIssue,
    )
    .expect("typed race issue");
    let race_remove = PutCourseGroupCommand {
        actor: instructor,
        expected_revision: Some(race_group.revision),
        record: CourseGroupRecord {
            id: race_group.record.id,
            tenant,
            course,
            purpose: CourseGroupPurpose::Lab,
            title: "Concurrent audience lab".to_string(),
            members: Vec::new(),
        },
    };
    let (race_issue, race_remove) = tokio::join!(
        store.issue_assignment_entitlement(context, race_issue),
        store.put_course_group(context, race_remove),
    );
    race_remove.expect("audience edit serializes with entitlement issue");
    assert!(matches!(
        race_issue.expect("issue serializes with audience edit"),
        AssignmentEntitlementMaterialization::Granted(_)
            | AssignmentEntitlementMaterialization::Denied(_)
    ));
    assert!(matches!(
        store
            .evaluate_assignment_entitlement(context, student, course, race_assignment)
            .await,
        Ok(domain::entitlement::EntitlementDecision::Denied(_))
    ));
    let raced_receipts: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM enrollment WHERE tenant_id = $1 AND assignment_id = $2 AND student_id = $3",
    )
    .bind(tenant.as_uuid())
    .bind(race_assignment.as_uuid())
    .bind(
        membership
            .student
            .expect("student membership carries student identity")
            .as_uuid(),
    )
    .fetch_one(&pool)
    .await
    .expect("inspect race receipt cardinality");
    assert!(
        raced_receipts <= 1,
        "serialized race cannot duplicate receipts"
    );

    let accommodation = CourseGroupId::from_uuid(id());
    let work = CourseGroupId::from_uuid(id());
    for (id, purpose, title) in [
        (
            accommodation,
            CourseGroupPurpose::Accommodation,
            "Extra time",
        ),
        (work, CourseGroupPurpose::Work, "Study team"),
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
            .expect("typed non-audience group persists with canonical member");
    }
    let scoped_assignment = AssignmentId::from_uuid(id());
    store
        .create_assignment_with_timing(
            context,
            assignment(
                tenant,
                course,
                scoped_assignment,
                reference,
                AssignmentAudience::CourseWide,
            ),
            AssignmentRunTiming::default(),
        )
        .await
        .expect("create course-wide policy-scope fixture");
    let scoped = store
        .issue_assignment_entitlement(
            context,
            MaterializeAssignmentEntitlementCommand::for_instructor_action(
                student,
                course,
                scoped_assignment,
                instructor,
                EntitlementPurpose::InstructorIssue,
            )
            .expect("typed scoped instructor issue"),
        )
        .await
        .expect("materialize applicable policy scopes");
    let AssignmentEntitlementMaterialization::Granted(scoped) = scoped else {
        panic!("course-wide learner remains entitled")
    };
    let scope_purposes: Vec<String> = sqlx::query_scalar(
        "SELECT course_group_purpose FROM enrollment_applicable_policy_scope_receipt \
         WHERE tenant_id = $1 AND enrollment_id = $2 ORDER BY course_group_purpose",
    )
    .bind(tenant.as_uuid())
    .bind(scoped.enrollment.id.as_uuid())
    .fetch_all(&pool)
    .await
    .expect("inspect persisted applicable policy scopes");
    assert!(
        scope_purposes
            .iter()
            .any(|purpose| purpose == "accommodation")
    );
    assert!(
        !scope_purposes.iter().any(|purpose| purpose == "work"),
        "Work has no audience or policy capability"
    );
    let late_accommodation = CourseGroupId::from_uuid(id());
    store
        .put_course_group(
            context,
            PutCourseGroupCommand {
                actor: instructor,
                expected_revision: None,
                record: CourseGroupRecord {
                    id: late_accommodation,
                    tenant,
                    course,
                    purpose: CourseGroupPurpose::Accommodation,
                    title: "Added after materialization".to_string(),
                    members: vec![membership.id],
                },
            },
        )
        .await
        .expect("create a valid current scope after materialization");
    denied_scope_append(
        &pool,
        tenant,
        scoped.enrollment.id,
        course,
        late_accommodation,
    )
    .await;

    let revoked_revision = store
        .revoke_course_member(
            context,
            instructor_session,
            RevokeCourseMember {
                course,
                member: learner.member.id,
                expected_revision: outside.roster_revision,
            },
        )
        .await
        .expect("revoke current learner membership");
    assert!(revoked_revision.value() > outside.roster_revision.value());
    assert_eq!(
        store
            .get_current_course_membership(context, course, student)
            .await
            .expect("read revoked membership"),
        None,
        "revocation removes current authority, not historical evidence"
    );
    assert!(matches!(
        store
            .issue_assignment_entitlement(
                context,
                MaterializeAssignmentEntitlementCommand::for_instructor_action(
                    student,
                    course,
                    assignment_id,
                    instructor,
                    EntitlementPurpose::InstructorIssue,
                )
                .expect("typed post-revocation issue"),
            )
            .await,
        Ok(AssignmentEntitlementMaterialization::Denied(_))
    ));
    assert_eq!(
        sqlx::query_scalar::<_, i64>(
            "SELECT count(*) FROM enrollment WHERE tenant_id = $1 AND enrollment_id = $2",
        )
        .bind(tenant.as_uuid())
        .bind(first.enrollment.id.as_uuid())
        .fetch_one(&pool)
        .await
        .expect("historical receipt remains after revocation"),
        1
    );

    let reinvited = store
        .upsert_course_member(
            context,
            UpsertCourseMember {
                course,
                user: student,
                display_name: "S5 learner reinvited".to_string(),
                roster_contact: None,
            },
        )
        .await
        .expect("reinvite creates a fresh membership episode");
    let fresh_membership = store
        .get_current_course_membership(context, course, student)
        .await
        .expect("read reinvited membership")
        .expect("reinvite restores current authority");
    assert_ne!(fresh_membership.id, membership.id);
    assert_eq!(fresh_membership.student, membership.student);
    assert_ne!(reinvited.member.id.as_uuid(), learner.member.id.as_uuid());
    let reissued = store
        .issue_assignment_entitlement(
            context,
            MaterializeAssignmentEntitlementCommand::for_instructor_action(
                student,
                course,
                assignment_id,
                instructor,
                EntitlementPurpose::InstructorIssue,
            )
            .expect("typed reinvite issue"),
        )
        .await
        .expect("reinvited learner regains derived authority");
    let AssignmentEntitlementMaterialization::Granted(reissued) = reissued else {
        panic!("reinvited learner is current and entitled")
    };
    assert_eq!(reissued.enrollment.id, first.enrollment.id);
    assert_eq!(reissued.provenance.membership, membership.id);
}

//! Reusable Store conformance suite, first run against memory in WP-C4.

use objects::{ObjectKey, ObjectRecord, Sha256Digest};
use question_model::answer::NumericTolerance;
use question_model::envelope::ContentBlock;
use question_model::generation::RandomizationDefinition;
use question_model::response::StudentResponse;
use question_model::run_policy::{AttemptPolicy, FeedbackDisclosure, TimingPolicy};
use question_model::taxonomy::{License, Tag, TaxonomyTerm};
use question_model::{
    ActivityTimestamp, AssetId, AssignmentEnrollment, AssignmentId, AssignmentRun,
    AttemptProvenance, AttemptResult, AttemptTimerRecord, BackendCapabilities, Capability,
    CatalogLifecycle, CompletionRequirement, ContinuedPractice, CourseId, CourseMembership,
    CourseMembershipRole, CourseRole, EnrollmentId, GeneratorReference, GradePolicy,
    GradingDefinition, ImplementationVersion, ObjectId, ProblemId, ProblemVersionRef,
    PublicationScope, QuestionAttempt, QuestionAttemptId, QuestionDefinition, QuestionMetadata,
    QuestionSource, ResponseDefinition, RunId, RunMode, RunPolicies, StudentId, TenantId, UserId,
    UserRole, VariationPolicy, VersionId, WorkspaceId,
};
use store::memory::MemoryStore;
#[cfg(feature = "postgres")]
use store::postgres::{PostgresStore, apply_migrations, lazy_pool};
use store::{
    ActivityTransition, AssetDeliveryId, AssetDeliveryRecord, AssetDeliveryScope, AssetStore,
    AssignmentRecord, CatalogStore, CatalogTransition, CourseListScope, CourseRecord, DraftRecord,
    IssueQuestionAttemptCommand, PageRequest, PageSize, PublishDraftCommand, PublishedVersionRef,
    SessionLifetime, SessionStore, SessionSubject, SessionTokenHash, Store, StoreError,
    SubmissionIdempotencyKey, SubmitQuestionAttemptCommand, TenantContext,
};
use uuid::Uuid;

fn uuid(value: u128) -> Uuid {
    Uuid::from_u128(value)
}

fn question(
    problem: Option<ProblemId>,
    version: VersionId,
    workspace: WorkspaceId,
) -> QuestionDefinition {
    QuestionDefinition {
        version,
        problem,
        workspace,
        source: QuestionSource::Native {
            family: "molar_mass".to_string(),
        },
        prompt: vec![ContentBlock::Text {
            markdown: "What is the molar mass?".to_string(),
        }],
        response: ResponseDefinition::Numeric {
            tolerance: NumericTolerance::Relative { fraction: 0.01 },
            unit: Some("g/mol".to_string()),
        },
        attempt_policy: AttemptPolicy {
            max_attempts: None,
            feedback: FeedbackDisclosure::ImmediateFull,
        },
        timing_policy: TimingPolicy::Untimed,
        randomization: RandomizationDefinition::Static,
        grading: GradingDefinition::AllOrNothing { points: 1.0 },
        metadata: QuestionMetadata {
            title: "Molar mass".to_string(),
            tags: vec![Tag::new("biochemistry")],
            taxonomy: Vec::new(),
            license: License::CcBySa,
            language: "en-US".to_string(),
        },
    }
}

fn policies() -> RunPolicies {
    RunPolicies {
        completion: CompletionRequirement::AllCorrect,
        grade: GradePolicy::Highest,
        continued_practice: ContinuedPractice::Unlimited,
        variation: VariationPolicy::NewSeeds,
    }
}

fn implementation(id: &str) -> ImplementationVersion {
    ImplementationVersion {
        id: id.to_string(),
        version: "1".to_string(),
    }
}

fn generator(id: &str) -> GeneratorReference {
    GeneratorReference {
        id: id.to_string(),
        version: "1".to_string(),
    }
}

fn object_record(key: ObjectKey, bytes: &[u8], at: i64) -> ObjectRecord {
    ObjectRecord {
        id: key.object_id(),
        bucket: key.bucket(),
        sha256: Sha256Digest::compute(bytes),
        size_bytes: u64::try_from(bytes.len()).expect("fixture size should fit"),
        media_type: "image/svg+xml".to_string(),
        category: key.category(),
        version: key.version_id(),
        license: "CC BY-SA 4.0".to_string(),
        provenance: "asset delivery conformance fixture".to_string(),
        created_at: ActivityTimestamp::from_unix_millis(at),
        key,
    }
}

async fn exercise_asset_store<S>(store: &S)
where
    S: Store + CatalogStore + AssetStore,
{
    let tenant = TenantId::from_uuid(uuid(401));
    let foreign_tenant = TenantId::from_uuid(uuid(402));
    let context = TenantContext::from_authenticated_session(tenant);
    let foreign_context = TenantContext::from_authenticated_session(foreign_tenant);
    let publisher = UserId::from_uuid(uuid(403));
    let student = UserId::from_uuid(uuid(404));
    let stranger = UserId::from_uuid(uuid(405));
    let public_problem = ProblemId::from_uuid(uuid(406));
    let public_version = VersionId::from_uuid(uuid(407));
    let institution_problem = ProblemId::from_uuid(uuid(408));
    let institution_version = VersionId::from_uuid(uuid(409));

    for (problem, version, workspace, scope) in [
        (
            public_problem,
            public_version,
            WorkspaceId::from_uuid(uuid(410)),
            PublicationScope::Public,
        ),
        (
            institution_problem,
            institution_version,
            WorkspaceId::from_uuid(uuid(411)),
            PublicationScope::Institution,
        ),
    ] {
        let draft = DraftRecord {
            tenant,
            question: question(None, version, workspace),
            revises: None,
            derived_from: None,
        };
        store
            .upsert_draft(context, draft.clone())
            .await
            .expect("asset fixture draft should save");
        store
            .publish_draft(
                context,
                PublishDraftCommand {
                    expected_draft: draft,
                    problem,
                    publisher,
                    scope,
                    capabilities: BackendCapabilities::from_iter([Capability::ServerGrading]),
                },
            )
            .await
            .expect("asset fixture should publish");
    }

    let public_asset = AssetId::from_uuid(uuid(412));
    let public_object = ObjectId::from_uuid(uuid(413));
    let public_delivery = AssetDeliveryRecord {
        id: AssetDeliveryId::from_asset(public_asset),
        object: object_record(
            ObjectKey::ProblemAsset {
                problem: public_problem,
                version: public_version,
                asset: public_asset,
                object: public_object,
            },
            b"public",
            1_000,
        ),
        scope: AssetDeliveryScope::Catalog {
            asset: public_asset,
            reference: ProblemVersionRef {
                problem: public_problem,
                version: public_version,
            },
        },
    };
    let institution_asset = AssetId::from_uuid(uuid(414));
    let institution_object = ObjectId::from_uuid(uuid(415));
    let institution_delivery = AssetDeliveryRecord {
        id: AssetDeliveryId::from_asset(institution_asset),
        object: object_record(
            ObjectKey::ProblemAsset {
                problem: institution_problem,
                version: institution_version,
                asset: institution_asset,
                object: institution_object,
            },
            b"institution",
            1_000,
        ),
        scope: AssetDeliveryScope::Catalog {
            asset: institution_asset,
            reference: ProblemVersionRef {
                problem: institution_problem,
                version: institution_version,
            },
        },
    };
    let student_object = ObjectId::from_uuid(uuid(416));
    let student_delivery = AssetDeliveryRecord {
        id: AssetDeliveryId::from_object(student_object),
        object: object_record(
            ObjectKey::StudentRecord {
                tenant,
                object: student_object,
            },
            b"student export",
            1_000,
        ),
        scope: AssetDeliveryScope::StudentRecord {
            tenant,
            authorized_users: vec![student],
        },
    };

    for record in [
        public_delivery.clone(),
        institution_delivery.clone(),
        student_delivery.clone(),
    ] {
        store
            .register_asset_delivery(context, record)
            .await
            .expect("valid asset delivery should register");
    }
    assert_eq!(
        store
            .register_asset_delivery(context, public_delivery.clone())
            .await,
        Err(StoreError::AlreadyExists),
        "delivery records are immutable"
    );

    assert_eq!(
        store
            .get_public_asset_delivery(public_delivery.id)
            .await
            .expect("public lookup should run"),
        Some(public_delivery.clone())
    );
    assert_eq!(
        store
            .get_public_asset_delivery(institution_delivery.id)
            .await
            .expect("institution lookup should run"),
        None
    );
    assert_eq!(
        store
            .get_public_asset_delivery(student_delivery.id)
            .await
            .expect("student-record lookup should run"),
        None
    );

    let institution_authorized = store
        .authorize_asset_delivery(context, student, institution_delivery.id)
        .await
        .expect("institution asset should be visible in its tenant");
    assert_eq!(institution_authorized.record, institution_delivery);
    assert_eq!(
        store
            .authorize_asset_delivery(foreign_context, student, institution_delivery.id)
            .await,
        Err(StoreError::NotFound),
        "institution assets must not cross tenant grants"
    );
    let student_authorized = store
        .authorize_asset_delivery(context, student, student_delivery.id)
        .await
        .expect("named student should receive their record");
    assert_eq!(student_authorized.record, student_delivery);
    assert_eq!(
        store
            .authorize_asset_delivery(context, stranger, student_authorized.record.id)
            .await,
        Err(StoreError::NotFound),
        "unauthorized identities must not learn that a student record exists"
    );
    assert_eq!(
        store
            .authorize_asset_delivery(foreign_context, student, student_authorized.record.id,)
            .await,
        Err(StoreError::NotFound),
        "RLS tenant context must protect student records"
    );

    let temporary = ObjectId::from_uuid(uuid(417));
    let invalid = AssetDeliveryRecord {
        id: AssetDeliveryId::from_object(temporary),
        object: object_record(
            ObjectKey::Temporary { object: temporary },
            b"temporary",
            1_000,
        ),
        scope: AssetDeliveryScope::StudentRecord {
            tenant,
            authorized_users: vec![student],
        },
    };
    assert!(matches!(
        store.register_asset_delivery(context, invalid).await,
        Err(StoreError::InvalidRecord(_))
    ));
}

async fn exercise_store<S>(store: &S)
where
    S: Store + CatalogStore,
{
    let tenant = TenantId::from_uuid(uuid(1));
    let foreign_tenant = TenantId::from_uuid(uuid(2));
    let context = TenantContext::from_authenticated_session(tenant);
    let foreign_context = TenantContext::from_authenticated_session(foreign_tenant);
    let workspace = WorkspaceId::from_uuid(uuid(3));
    let problem = ProblemId::from_uuid(uuid(4));
    let version = VersionId::from_uuid(uuid(5));
    let second_problem = ProblemId::from_uuid(uuid(6));
    let second_version = VersionId::from_uuid(uuid(7));
    let assignment_id = AssignmentId::from_uuid(uuid(8));
    let course_id = CourseId::from_uuid(uuid(17));
    let course_user = UserId::from_uuid(uuid(18));
    let enrollment_id = EnrollmentId::from_uuid(uuid(9));
    let run_id = RunId::from_uuid(uuid(10));
    let practice_run_id = RunId::from_uuid(uuid(14));
    let draft = DraftRecord {
        tenant,
        question: question(None, version, workspace),
        revises: None,
        derived_from: None,
    };
    let publisher = UserId::from_uuid(uuid(16));
    let assignment = AssignmentRecord {
        id: assignment_id,
        tenant,
        course_id,
        title: "Molar mass mastery".to_string(),
        problems: vec![PublishedVersionRef { problem, version }],
        policies: policies(),
    };

    let mut invalid_draft = draft.clone();
    invalid_draft.question.attempt_policy.max_attempts = Some(0);
    assert_eq!(
        store.upsert_draft(context, invalid_draft).await,
        Err(StoreError::InvalidRecord(
            "question max attempts must be greater than zero".to_string()
        ))
    );

    store
        .upsert_draft(context, draft.clone())
        .await
        .expect("conforming draft write should succeed");
    let published = store
        .publish_draft(
            context,
            PublishDraftCommand {
                expected_draft: draft.clone(),
                problem,
                publisher,
                scope: PublicationScope::Public,
                capabilities: BackendCapabilities::from_iter([Capability::ServerGrading]),
            },
        )
        .await
        .expect("conforming publish should succeed");
    let second_draft = DraftRecord {
        tenant,
        question: question(None, second_version, workspace),
        revises: None,
        derived_from: None,
    };
    store
        .upsert_draft(context, second_draft.clone())
        .await
        .expect("second draft write should succeed");
    store
        .publish_draft(
            context,
            PublishDraftCommand {
                expected_draft: second_draft,
                problem: second_problem,
                publisher,
                scope: PublicationScope::Public,
                capabilities: BackendCapabilities::from_iter([Capability::ServerGrading]),
            },
        )
        .await
        .expect("second publish should succeed");

    let first_page = store
        .list_published_problems(PageRequest::first(
            PageSize::new(1).expect("one is a valid page size"),
        ))
        .await
        .expect("first catalog page should load");
    let second_page = store
        .list_published_problems(PageRequest::after(
            first_page
                .next_cursor
                .clone()
                .expect("first page should carry a cursor"),
            PageSize::new(1).expect("one is a valid page size"),
        ))
        .await
        .expect("second catalog page should load");

    store
        .upsert_course(
            context,
            CourseRecord {
                id: course_id,
                tenant,
                title: "Biochemistry".to_string(),
                members: vec![
                    CourseMembership {
                        user: course_user,
                        role: CourseMembershipRole::Instructor,
                    },
                    CourseMembership {
                        user: UserId::from_uuid(uuid(14)),
                        role: CourseMembershipRole::Student,
                    },
                ],
            },
        )
        .await
        .expect("conforming course write should succeed");
    store
        .upsert_assignment(context, assignment.clone())
        .await
        .expect("conforming assignment write should succeed");
    store
        .create_enrollment(
            context,
            AssignmentEnrollment {
                id: enrollment_id,
                tenant,
                assignment: assignment_id,
                user: UserId::from_uuid(uuid(14)),
                student: StudentId::from_uuid(uuid(11)),
                first_completed_at: None,
                current_grade_run: None,
                best_grade_run: None,
            },
        )
        .await
        .expect("conforming enrollment creation should succeed");
    store
        .apply_activity_transition(
            context,
            ActivityTransition::StartRun {
                run: AssignmentRun {
                    id: run_id,
                    tenant,
                    enrollment: enrollment_id,
                    run_number: 1,
                    started_at: ActivityTimestamp::from_unix_millis(100),
                    completed_at: None,
                    score: None,
                    mode: RunMode::Assigned,
                    variation: VariationPolicy::NewSeeds,
                },
            },
        )
        .await
        .expect("conforming run start should succeed");
    store
        .apply_activity_transition(
            context,
            ActivityTransition::RecordQuestionAttempt {
                attempt: Box::new(QuestionAttempt {
                    id: QuestionAttemptId::from_uuid(uuid(12)),
                    tenant,
                    run: run_id,
                    problem,
                    question_version: version,
                    assignment_position: 0,
                    seed: 42,
                    parameter_hash: "parameters-sha256".to_string(),
                    response: Some(StudentResponse::Numeric { value: 18.0 }),
                    result: Some(AttemptResult {
                        correct: true,
                        points_earned: 1.0,
                        points_possible: 1.0,
                    }),
                    timer: AttemptTimerRecord {
                        issued_at: ActivityTimestamp::from_unix_millis(110),
                        deadline: None,
                        submitted_at: Some(ActivityTimestamp::from_unix_millis(120)),
                    },
                    provenance: AttemptProvenance {
                        adapter: implementation("native"),
                        renderer: None,
                        generator: Some(generator("molar-mass")),
                        source_artifact: None,
                        asset_objects: vec![ObjectId::from_uuid(uuid(13))],
                        grading: implementation("numeric"),
                        rendered_question_sha256: "render-sha256".to_string(),
                    },
                }),
            },
        )
        .await
        .expect("conforming attempt write should succeed");
    let summary = store
        .apply_activity_transition(
            context,
            ActivityTransition::CompleteRun {
                run: run_id,
                score: 1.0,
                at: ActivityTimestamp::from_unix_millis(130),
            },
        )
        .await
        .expect("conforming completion should succeed");
    let completed_run = store
        .get_run(context, run_id)
        .await
        .expect("run read should succeed")
        .expect("completed run should exist");
    let attempt = store
        .get_question_attempt(context, QuestionAttemptId::from_uuid(uuid(12)))
        .await
        .expect("attempt read should succeed")
        .expect("question attempt should exist");

    store
        .apply_activity_transition(
            context,
            ActivityTransition::StartRun {
                run: AssignmentRun {
                    id: practice_run_id,
                    tenant,
                    enrollment: enrollment_id,
                    run_number: 2,
                    started_at: ActivityTimestamp::from_unix_millis(140),
                    completed_at: None,
                    score: None,
                    mode: RunMode::Practice,
                    variation: VariationPolicy::NewSeeds,
                },
            },
        )
        .await
        .expect("continued practice should remain available after completion");
    let practice_summary = store
        .apply_activity_transition(
            context,
            ActivityTransition::CompleteRun {
                run: practice_run_id,
                score: 0.8,
                at: ActivityTimestamp::from_unix_millis(150),
            },
        )
        .await
        .expect("continued-practice completion should succeed");
    let enrollment = store
        .get_enrollment(context, enrollment_id)
        .await
        .expect("enrollment read should succeed")
        .expect("enrollment should exist");
    let persisted_summary = store
        .get_summary(context, enrollment_id)
        .await
        .expect("summary read should succeed")
        .expect("summary should exist");

    let tenant_mismatch = store
        .upsert_draft(
            foreign_context,
            DraftRecord {
                tenant,
                question: question(None, VersionId::from_uuid(uuid(15)), workspace),
                revises: None,
                derived_from: None,
            },
        )
        .await;
    let tenant_assignments = store
        .list_assignments(
            context,
            course_id,
            PageRequest::first(PageSize::new(10).expect("ten is a valid page size")),
        )
        .await
        .expect("assignment list should load");
    let member_courses = store
        .list_courses(
            context,
            CourseListScope::Member(course_user),
            PageRequest::first(PageSize::new(10).expect("ten is a valid page size")),
        )
        .await
        .expect("member course list should load");
    let nonmember_courses = store
        .list_courses(
            context,
            CourseListScope::Member(UserId::from_uuid(uuid(19))),
            PageRequest::first(PageSize::new(10).expect("ten is a valid page size")),
        )
        .await
        .expect("nonmember course list should load");
    let administrator_courses = store
        .list_courses(
            context,
            CourseListScope::TenantAdministrator,
            PageRequest::first(PageSize::new(10).expect("ten is a valid page size")),
        )
        .await
        .expect("administrator course list should load");
    let run_page = store
        .list_runs(
            context,
            enrollment_id,
            PageRequest::first(PageSize::new(10).expect("ten is a valid page size")),
        )
        .await
        .expect("run list should load");

    assert_eq!((first_page.items.len(), second_page.items.len()), (1, 1));
    assert_eq!(store.get_draft(context, workspace).await, Ok(None));
    assert_eq!(
        store.get_published_problem(problem, version).await,
        Ok(Some(published))
    );
    assert_eq!(
        store.get_assignment(context, assignment_id).await,
        Ok(Some(assignment))
    );
    assert_eq!(tenant_assignments.items.len(), 1);
    assert_eq!(member_courses.items.len(), 1);
    assert_eq!(member_courses.items[0].role, CourseRole::Instructor);
    assert!(nonmember_courses.items.is_empty());
    assert_eq!(
        administrator_courses.items[0].role,
        CourseRole::Administrator
    );
    assert_eq!(store.get_course(foreign_context, course_id).await, Ok(None));
    assert_eq!(
        (
            summary.current_score,
            summary.completed_run_count,
            summary.total_question_attempts,
        ),
        (Some(1.0), 1, 1)
    );
    assert_eq!(practice_summary, persisted_summary);
    assert_eq!(
        (
            persisted_summary.current_score,
            persisted_summary.best_score,
            persisted_summary.latest_score,
            persisted_summary.completed_run_count,
        ),
        (Some(1.0), Some(1.0), Some(0.8), 2)
    );
    assert_eq!(
        (
            enrollment.first_completed_at,
            enrollment.current_grade_run,
            enrollment.best_grade_run,
        ),
        (
            Some(ActivityTimestamp::from_unix_millis(130)),
            Some(run_id),
            Some(run_id),
        )
    );
    assert_eq!(
        (
            completed_run.completed_at,
            attempt.problem,
            run_page.items.len()
        ),
        (Some(ActivityTimestamp::from_unix_millis(130)), problem, 2,)
    );
    assert_eq!(tenant_mismatch, Err(StoreError::TenantMismatch));
    assert_eq!(store.get_draft(foreign_context, workspace).await, Ok(None));
    assert_eq!(
        store.get_assignment(foreign_context, assignment_id).await,
        Ok(None)
    );
    assert_eq!(
        store.get_enrollment(foreign_context, enrollment_id).await,
        Ok(None)
    );
    assert_eq!(store.get_run(foreign_context, run_id).await, Ok(None));
    assert_eq!(
        store
            .get_question_attempt(foreign_context, QuestionAttemptId::from_uuid(uuid(12)))
            .await,
        Ok(None)
    );
    assert_eq!(
        store.get_summary(foreign_context, enrollment_id).await,
        Ok(None)
    );
}

async fn exercise_session_replicas(issuer: &dyn SessionStore, next_replica: &dyn SessionStore) {
    let token_hash = SessionTokenHash::compute(b"opaque replica test credential");
    let wrong_token_hash = SessionTokenHash::compute(b"different credential");
    let subject = SessionSubject::new(
        TenantId::from_uuid(uuid(101)),
        UserId::from_uuid(uuid(102)),
        "Replica Student",
        vec![UserRole::Student],
    )
    .expect("fixture identity should be valid");
    let lifetime = SessionLifetime::from_seconds(60).expect("positive lifetime");

    let issued = issuer
        .create_session(token_hash, subject.clone(), lifetime)
        .await
        .expect("first replica should issue a session");
    let resumed = next_replica
        .resolve_session(token_hash)
        .await
        .expect("second replica should resolve a session");

    assert_eq!(resumed, Some(issued));
    assert_eq!(
        next_replica.resolve_session(wrong_token_hash).await,
        Ok(None),
        "a different cookie must not reveal any session"
    );

    next_replica
        .revoke_session(token_hash)
        .await
        .expect("second replica should revoke the session");
    assert_eq!(issuer.resolve_session(token_hash).await, Ok(None));
    next_replica
        .revoke_session(token_hash)
        .await
        .expect("repeat revocation should be idempotent");
}

async fn exercise_run_api_store<S>(store: &S)
where
    S: Store + CatalogStore,
{
    let tenant = TenantId::from_uuid(uuid(401));
    let context = TenantContext::from_authenticated_session(tenant);
    let publisher = UserId::from_uuid(uuid(402));
    let student_user = UserId::from_uuid(uuid(403));
    let workspace = WorkspaceId::from_uuid(uuid(404));
    let problem = ProblemId::from_uuid(uuid(405));
    let version = VersionId::from_uuid(uuid(406));
    let course = CourseId::from_uuid(uuid(407));
    let assignment = AssignmentId::from_uuid(uuid(408));
    let enrollment = EnrollmentId::from_uuid(uuid(409));
    let first_run = RunId::from_uuid(uuid(410));
    let ignored_resume_id = RunId::from_uuid(uuid(411));
    let attempt_id = QuestionAttemptId::from_uuid(uuid(412));

    let draft = DraftRecord {
        tenant,
        question: question(None, version, workspace),
        revises: None,
        derived_from: None,
    };
    store
        .upsert_draft(context, draft.clone())
        .await
        .expect("run fixture draft");
    store
        .publish_draft(
            context,
            PublishDraftCommand {
                expected_draft: draft,
                problem,
                publisher,
                scope: PublicationScope::Public,
                capabilities: BackendCapabilities::from_iter([Capability::ServerGrading]),
            },
        )
        .await
        .expect("run fixture publication");
    store
        .upsert_course(
            context,
            CourseRecord {
                id: course,
                tenant,
                title: "Run API biochemistry".to_string(),
                members: vec![
                    CourseMembership {
                        user: publisher,
                        role: CourseMembershipRole::Instructor,
                    },
                    CourseMembership {
                        user: student_user,
                        role: CourseMembershipRole::Student,
                    },
                ],
            },
        )
        .await
        .expect("run fixture course");
    store
        .upsert_assignment(
            context,
            AssignmentRecord {
                id: assignment,
                tenant,
                course_id: course,
                title: "Run API assignment".to_string(),
                problems: vec![
                    ProblemVersionRef { problem, version },
                    ProblemVersionRef { problem, version },
                ],
                policies: policies(),
            },
        )
        .await
        .expect("run fixture assignment");
    store
        .create_enrollment(
            context,
            AssignmentEnrollment {
                id: enrollment,
                tenant,
                assignment,
                user: student_user,
                student: StudentId::from_uuid(uuid(413)),
                first_completed_at: None,
                current_grade_run: None,
                best_grade_run: None,
            },
        )
        .await
        .expect("run fixture enrollment");

    let run = store
        .start_or_resume_run(context, student_user, assignment, first_run)
        .await
        .expect("first run should start");
    let resumed = store
        .start_or_resume_run(context, student_user, assignment, ignored_resume_id)
        .await
        .expect("active run should resume");
    assert_eq!(resumed, run);

    let issue = IssueQuestionAttemptCommand {
        actor: student_user,
        attempt: attempt_id,
        run: run.id,
        assignment_position: 0,
        problem,
        question_version: version,
        seed: 991,
        parameter_hash: "parameter-hash".to_string(),
        provenance: AttemptProvenance {
            adapter: implementation("native"),
            renderer: None,
            generator: None,
            source_artifact: None,
            asset_objects: Vec::new(),
            grading: implementation("numeric"),
            rendered_question_sha256: "rendered-hash".to_string(),
        },
    };
    let attempt = store
        .issue_or_resume_question_attempt(context, issue.clone())
        .await
        .expect("question should issue");
    let resumed_attempt = store
        .issue_or_resume_question_attempt(
            context,
            IssueQuestionAttemptCommand {
                attempt: QuestionAttemptId::from_uuid(uuid(414)),
                seed: 992,
                ..issue
            },
        )
        .await
        .expect("unanswered question should resume");
    assert_eq!(resumed_attempt, attempt);

    let blocked_second_position = store
        .issue_or_resume_question_attempt(
            context,
            IssueQuestionAttemptCommand {
                actor: student_user,
                attempt: QuestionAttemptId::from_uuid(uuid(415)),
                run: run.id,
                assignment_position: 1,
                problem,
                question_version: version,
                seed: 993,
                parameter_hash: "second-parameter-hash".to_string(),
                provenance: AttemptProvenance {
                    adapter: implementation("native"),
                    renderer: None,
                    generator: None,
                    source_artifact: None,
                    asset_objects: Vec::new(),
                    grading: implementation("numeric"),
                    rendered_question_sha256: "second-rendered-hash".to_string(),
                },
            },
        )
        .await;
    assert!(matches!(
        blocked_second_position,
        Err(StoreError::InvalidRecord(message))
            if message == "another question attempt is already active in this run"
    ));

    let response = StudentResponse::Numeric { value: 18.0 };
    let key = SubmissionIdempotencyKey::parse("submission-401").expect("valid key");
    assert_eq!(
        store
            .replay_submission(context, student_user, attempt.id, &response, &key)
            .await,
        Ok(None)
    );
    let invalid_result = store
        .submit_question_attempt(
            context,
            SubmitQuestionAttemptCommand {
                actor: student_user,
                attempt: attempt.id,
                response: response.clone(),
                result: AttemptResult {
                    correct: false,
                    points_earned: 2.0,
                    points_possible: 1.0,
                },
                idempotency_key: key.clone(),
            },
        )
        .await;
    assert!(matches!(invalid_result, Err(StoreError::InvalidRecord(_))));
    assert_eq!(
        store
            .replay_submission(context, student_user, attempt.id, &response, &key)
            .await,
        Ok(None),
        "a rejected backend result must leave the attempt unsubmitted"
    );
    let submitted = store
        .submit_question_attempt(
            context,
            SubmitQuestionAttemptCommand {
                actor: student_user,
                attempt: attempt.id,
                response: response.clone(),
                result: AttemptResult {
                    correct: true,
                    points_earned: 1.0,
                    points_possible: 1.0,
                },
                idempotency_key: key.clone(),
            },
        )
        .await
        .expect("first response should commit");
    let replay = store
        .replay_submission(context, student_user, attempt.id, &response, &key)
        .await
        .expect("replay lookup")
        .expect("first receipt should replay");
    assert_eq!(replay.attempt, submitted.attempt);
    assert_eq!(
        store
            .submit_question_attempt(
                context,
                SubmitQuestionAttemptCommand {
                    actor: student_user,
                    attempt: attempt.id,
                    response: response.clone(),
                    result: AttemptResult {
                        correct: false,
                        points_earned: 0.0,
                        points_possible: 1.0,
                    },
                    idempotency_key: key.clone(),
                },
            )
            .await
            .expect("exact replay should ignore the changed proposed grade")
            .attempt,
        submitted.attempt
    );
    assert_eq!(
        store
            .replay_submission(
                context,
                student_user,
                attempt.id,
                &StudentResponse::Numeric { value: 19.0 },
                &key,
            )
            .await,
        Err(StoreError::Conflict)
    );
    let changed_key =
        SubmissionIdempotencyKey::parse("submission-401-new").expect("valid changed key");
    assert_eq!(
        store
            .replay_submission(context, student_user, attempt.id, &response, &changed_key)
            .await,
        Err(StoreError::Conflict)
    );
    assert_eq!(submitted.run.completed_at, None);

    let second_attempt = store
        .issue_or_resume_question_attempt(
            context,
            IssueQuestionAttemptCommand {
                actor: student_user,
                attempt: QuestionAttemptId::from_uuid(uuid(415)),
                run: run.id,
                assignment_position: 1,
                problem,
                question_version: version,
                seed: 993,
                parameter_hash: "second-parameter-hash".to_string(),
                provenance: AttemptProvenance {
                    adapter: implementation("native"),
                    renderer: None,
                    generator: None,
                    source_artifact: None,
                    asset_objects: Vec::new(),
                    grading: implementation("numeric"),
                    rendered_question_sha256: "second-rendered-hash".to_string(),
                },
            },
        )
        .await
        .expect("the next position should issue after the active response commits");
    let completed = store
        .submit_question_attempt(
            context,
            SubmitQuestionAttemptCommand {
                actor: student_user,
                attempt: second_attempt.id,
                response: response.clone(),
                result: AttemptResult {
                    correct: true,
                    points_earned: 1.0,
                    points_possible: 1.0,
                },
                idempotency_key: SubmissionIdempotencyKey::parse("submission-402")
                    .expect("valid second key"),
            },
        )
        .await
        .expect("second response should complete the run");
    assert_eq!(
        completed.run.completed_at,
        completed.attempt.timer.submitted_at
    );
    assert_eq!(
        (
            completed.summary.completed_run_count,
            completed.summary.total_question_attempts,
            completed.summary.current_score,
        ),
        (1, 2, Some(1.0))
    );
    let attempt_page = store
        .list_question_attempts(
            context,
            run.id,
            PageRequest::first(PageSize::new(10).expect("valid page size")),
        )
        .await
        .expect("attempt page");
    assert_eq!(
        attempt_page.items,
        vec![submitted.attempt, completed.attempt]
    );
}

async fn exercise_catalog_store<S>(store: &S)
where
    S: Store + CatalogStore,
{
    let tenant = TenantId::from_uuid(uuid(301));
    let foreign_tenant = TenantId::from_uuid(uuid(302));
    let context = TenantContext::from_authenticated_session(tenant);
    let foreign_context = TenantContext::from_authenticated_session(foreign_tenant);
    let publisher = UserId::from_uuid(uuid(303));
    let other_user = UserId::from_uuid(uuid(304));
    let tenant_course = CourseId::from_uuid(uuid(317));
    let foreign_course = CourseId::from_uuid(uuid(318));
    store
        .upsert_course(
            context,
            CourseRecord {
                id: tenant_course,
                tenant,
                title: "Tenant biochemistry".to_string(),
                members: vec![CourseMembership {
                    user: publisher,
                    role: CourseMembershipRole::Instructor,
                }],
            },
        )
        .await
        .expect("tenant course should save");
    store
        .upsert_course(
            foreign_context,
            CourseRecord {
                id: foreign_course,
                tenant: foreign_tenant,
                title: "Foreign biochemistry".to_string(),
                members: vec![CourseMembership {
                    user: other_user,
                    role: CourseMembershipRole::Instructor,
                }],
            },
        )
        .await
        .expect("foreign course should save");
    let institution_workspace = WorkspaceId::from_uuid(uuid(305));
    let institution_problem = ProblemId::from_uuid(uuid(306));
    let institution_version = VersionId::from_uuid(uuid(307));
    let mut institution_question = question(None, institution_version, institution_workspace);
    institution_question.metadata.taxonomy = vec![
        TaxonomyTerm {
            scheme: "discipline/core".to_string(),
            code: "BIOC".to_string(),
            label: "Biochemistry".to_string(),
        },
        TaxonomyTerm {
            scheme: "discipline".to_string(),
            code: "core/BIOC".to_string(),
            label: "Biochemistry integration".to_string(),
        },
    ];
    let institution_draft = DraftRecord {
        tenant,
        question: institution_question,
        revises: None,
        derived_from: None,
    };
    store
        .upsert_draft(context, institution_draft.clone())
        .await
        .expect("institution draft should save");
    let institution_record = store
        .publish_draft(
            context,
            PublishDraftCommand {
                expected_draft: institution_draft.clone(),
                problem: institution_problem,
                publisher,
                scope: PublicationScope::Institution,
                capabilities: BackendCapabilities::from_iter([Capability::ServerGrading]),
            },
        )
        .await
        .expect("institution publication should succeed");

    assert_eq!(
        institution_record.question.problem,
        Some(institution_problem)
    );
    assert_eq!(
        store
            .get_draft(context, institution_workspace)
            .await
            .expect("published draft lookup"),
        None
    );
    assert_eq!(
        store
            .get_catalog_problem(
                foreign_context,
                ProblemVersionRef {
                    problem: institution_problem,
                    version: institution_version,
                },
            )
            .await,
        Ok(None),
        "institution publication must not cross its visibility grant"
    );
    assert_eq!(
        store
            .get_published_problem(institution_problem, institution_version)
            .await,
        Ok(None),
        "the context-free public-content contract must not expose institution content"
    );
    let tenant_taxonomy = store
        .list_catalog_taxonomy(
            context,
            PageRequest::first(PageSize::new(10).expect("valid page size")),
        )
        .await
        .expect("tenant taxonomy should list");
    let foreign_taxonomy = store
        .list_catalog_taxonomy(
            foreign_context,
            PageRequest::first(PageSize::new(10).expect("valid page size")),
        )
        .await
        .expect("foreign taxonomy should list");
    assert_eq!(
        tenant_taxonomy
            .items
            .iter()
            .map(|term| (term.scheme.as_str(), term.code.as_str()))
            .collect::<Vec<_>>(),
        vec![("discipline", "core/BIOC"), ("discipline/core", "BIOC"),],
        "taxonomy identity is the scheme/code pair, even when either contains a slash"
    );
    assert!(foreign_taxonomy.items.is_empty());
    store
        .upsert_assignment(
            context,
            AssignmentRecord {
                id: AssignmentId::from_uuid(uuid(313)),
                tenant,
                course_id: tenant_course,
                title: "Institution content".to_string(),
                problems: vec![ProblemVersionRef {
                    problem: institution_problem,
                    version: institution_version,
                }],
                policies: policies(),
            },
        )
        .await
        .expect("publishing tenant should assign institution content");
    assert!(matches!(
        store
            .upsert_assignment(
                foreign_context,
                AssignmentRecord {
                    id: AssignmentId::from_uuid(uuid(314)),
                    tenant: foreign_tenant,
                    course_id: foreign_course,
                    title: "Hidden institution content".to_string(),
                    problems: vec![ProblemVersionRef {
                        problem: institution_problem,
                        version: institution_version,
                    }],
                    policies: policies(),
                },
            )
            .await,
        Err(StoreError::InvalidRecord(_))
    ));

    let public_workspace = WorkspaceId::from_uuid(uuid(308));
    let public_problem = ProblemId::from_uuid(uuid(309));
    let public_version = VersionId::from_uuid(uuid(310));
    let public_draft = DraftRecord {
        tenant,
        question: question(None, public_version, public_workspace),
        revises: None,
        derived_from: None,
    };
    store
        .upsert_draft(context, public_draft.clone())
        .await
        .expect("public draft should save");
    store
        .publish_draft(
            context,
            PublishDraftCommand {
                expected_draft: public_draft,
                problem: public_problem,
                publisher,
                scope: PublicationScope::Public,
                capabilities: BackendCapabilities::from_iter([Capability::ServerGrading]),
            },
        )
        .await
        .expect("public publication should succeed");
    let foreign_catalog = store
        .list_catalog(
            foreign_context,
            PageRequest::first(PageSize::new(10).expect("valid page size")),
        )
        .await
        .expect("foreign public catalog should list");
    assert_eq!(foreign_catalog.items.len(), 1);
    assert_eq!(foreign_catalog.items[0].problem, public_problem);

    let revision_version = VersionId::from_uuid(uuid(311));
    let revision_workspace = WorkspaceId::from_uuid(uuid(312));
    let revision_draft = DraftRecord {
        tenant,
        question: question(None, revision_version, revision_workspace),
        revises: Some(ProblemVersionRef {
            problem: public_problem,
            version: public_version,
        }),
        derived_from: None,
    };
    store
        .upsert_draft(context, revision_draft.clone())
        .await
        .expect("revision draft should save");
    let revision = store
        .publish_draft(
            context,
            PublishDraftCommand {
                expected_draft: revision_draft,
                problem: public_problem,
                publisher,
                scope: PublicationScope::Public,
                capabilities: BackendCapabilities::from_iter([Capability::ServerGrading]),
            },
        )
        .await
        .expect("owned linear revision should publish");
    assert_eq!(revision.previous_version, Some(public_version));
    assert_eq!(revision.authors, vec![publisher]);

    assert_eq!(
        store
            .transition_catalog_problem(
                context,
                other_user,
                ProblemVersionRef {
                    problem: public_problem,
                    version: public_version,
                },
                CatalogTransition::Deprecate {
                    reason: "Correction available".to_string(),
                },
            )
            .await,
        Err(StoreError::Forbidden)
    );
    let deprecated = store
        .transition_catalog_problem(
            context,
            publisher,
            ProblemVersionRef {
                problem: public_problem,
                version: public_version,
            },
            CatalogTransition::Deprecate {
                reason: " Correction available ".to_string(),
            },
        )
        .await
        .expect("author should deprecate");
    assert!(matches!(
        deprecated.lifecycle,
        CatalogLifecycle::Deprecated { ref reason } if reason == "Correction available"
    ));
    let exact_deprecated = store
        .get_catalog_problem(
            foreign_context,
            ProblemVersionRef {
                problem: public_problem,
                version: public_version,
            },
        )
        .await
        .expect("exact deprecated lookup should run");
    assert!(
        exact_deprecated.is_some(),
        "existing references remain resolvable"
    );
    store
        .upsert_assignment(
            context,
            AssignmentRecord {
                id: AssignmentId::from_uuid(uuid(315)),
                tenant,
                course_id: tenant_course,
                title: "Deprecated exact reference".to_string(),
                problems: vec![ProblemVersionRef {
                    problem: public_problem,
                    version: public_version,
                }],
                policies: policies(),
            },
        )
        .await
        .expect("a deprecated version remains assignable by exact reference");
    let browse_after_deprecation = store
        .list_catalog(
            foreign_context,
            PageRequest::first(PageSize::new(10).expect("valid page size")),
        )
        .await
        .expect("catalog should list");
    assert_eq!(browse_after_deprecation.items.len(), 1);
    assert_eq!(browse_after_deprecation.items[0].version, revision_version);

    let archived = store
        .transition_catalog_problem(
            context,
            publisher,
            ProblemVersionRef {
                problem: public_problem,
                version: public_version,
            },
            CatalogTransition::Archive,
        )
        .await
        .expect("deprecated version should archive");
    assert!(matches!(
        archived.lifecycle,
        CatalogLifecycle::Archived { .. }
    ));
    assert!(matches!(
        store
            .upsert_assignment(
                context,
                AssignmentRecord {
                    id: AssignmentId::from_uuid(uuid(316)),
                    tenant,
                    course_id: tenant_course,
                    title: "Archived exact reference".to_string(),
                    problems: vec![ProblemVersionRef {
                        problem: public_problem,
                        version: public_version,
                    }],
                    policies: policies(),
                },
            )
            .await,
        Err(StoreError::InvalidRecord(_))
    ));
}

#[tokio::test]
async fn memory_store_conforms() {
    let store = MemoryStore::default();
    exercise_store(&store).await;
    exercise_session_replicas(&store, &store.clone()).await;
}

#[tokio::test]
async fn memory_run_api_store_conforms() {
    let store = MemoryStore::default();
    store
        .set_authoritative_time(ActivityTimestamp::from_unix_millis(500))
        .expect("memory clock");
    exercise_run_api_store(&store).await;
}

#[tokio::test]
async fn memory_catalog_store_conforms() {
    exercise_catalog_store(&MemoryStore::default()).await;
}

#[tokio::test]
async fn memory_asset_store_conforms_and_records_protected_access() {
    let store = MemoryStore::default();
    store
        .set_authoritative_time(ActivityTimestamp::from_unix_millis(7_000))
        .expect("memory clock should be writable");
    exercise_asset_store(&store).await;
    let events = store
        .asset_access_events()
        .expect("memory audit events should be readable");
    assert_eq!(events.len(), 2, "only authorized protected requests log");
    assert!(
        events
            .iter()
            .all(|event| event.occurred_at == ActivityTimestamp::from_unix_millis(7_000))
    );
}

#[tokio::test]
async fn memory_sessions_use_the_backend_clock_for_expiry() {
    let store = MemoryStore::default();
    store
        .set_authoritative_time(ActivityTimestamp::from_unix_millis(1_000))
        .expect("memory clock should be writable");
    let token_hash = SessionTokenHash::compute(b"expiring credential");
    let subject = SessionSubject::new(
        TenantId::from_uuid(uuid(201)),
        UserId::from_uuid(uuid(202)),
        "Expiring Student",
        vec![UserRole::Student],
    )
    .expect("fixture identity should be valid");
    store
        .create_session(
            token_hash,
            subject,
            SessionLifetime::from_seconds(1).expect("positive lifetime"),
        )
        .await
        .expect("session should be created");
    store
        .set_authoritative_time(ActivityTimestamp::from_unix_millis(2_000))
        .expect("memory clock should advance");

    assert_eq!(store.resolve_session(token_hash).await, Ok(None));
}

#[cfg(feature = "postgres")]
#[tokio::test]
#[ignore = "requires a dedicated empty database in PLE_POSTGRES_TEST_URL"]
async fn postgres_store_conforms_and_enforces_database_boundaries() {
    let database_url = std::env::var("PLE_POSTGRES_TEST_URL")
        .expect("set PLE_POSTGRES_TEST_URL to a dedicated empty PostgreSQL database");
    let pool = lazy_pool(&database_url).expect("PostgreSQL test URL should be valid");

    apply_migrations(&pool)
        .await
        .expect("fresh migration application should succeed");
    apply_migrations(&pool)
        .await
        .expect("checksummed migrations should be idempotent");
    let issuer = PostgresStore::new(pool.clone());
    let next_replica = PostgresStore::new(pool.clone());
    exercise_store(&issuer).await;
    exercise_catalog_store(&issuer).await;
    exercise_run_api_store(&issuer).await;
    exercise_asset_store(&issuer).await;
    exercise_session_replicas(&issuer, &next_replica).await;

    let mut foreign_transaction = pool
        .begin()
        .await
        .expect("foreign-tenant transaction should start");
    sqlx::query("SET LOCAL ROLE ple_app")
        .execute(&mut *foreign_transaction)
        .await
        .expect("migration login should be able to assume ple_app");
    sqlx::query("SELECT set_config('ple.tenant_id', $1, true)")
        .bind(TenantId::from_uuid(uuid(2)).to_string())
        .execute(&mut *foreign_transaction)
        .await
        .expect("foreign tenant context should be set locally");
    let visible_rows: i64 = sqlx::query_scalar(
        "SELECT \
         (SELECT count(*) FROM workspace_draft) + \
         (SELECT count(*) FROM course) + \
         (SELECT count(*) FROM course_member) + \
         (SELECT count(*) FROM assignment) + \
         (SELECT count(*) FROM assignment_problem) + \
         (SELECT count(*) FROM enrollment) + \
         (SELECT count(*) FROM student_assignment_summary) + \
         (SELECT count(*) FROM assignment_run) + \
         (SELECT count(*) FROM question_attempt) + \
         (SELECT count(*) FROM submission_idempotency) + \
         (SELECT count(*) FROM submission) + \
         (SELECT count(*) FROM grade_event) + \
         (SELECT count(*) FROM audit_event) + \
         (SELECT count(*) FROM asset_delivery)",
    )
    .fetch_one(&mut *foreign_transaction)
    .await
    .expect("foreign-tenant visibility query should run");
    assert_eq!(visible_rows, 0, "forced RLS must hide every tenant row");
    foreign_transaction
        .commit()
        .await
        .expect("foreign-tenant transaction should commit");

    let mut student_transaction = pool
        .begin()
        .await
        .expect("student-role transaction should start");
    sqlx::query("SET LOCAL ROLE ple_student")
        .execute(&mut *student_transaction)
        .await
        .expect("migration login should be able to assume ple_student");
    let answer_key_read = sqlx::query("SELECT key_payload FROM answer_key LIMIT 1")
        .execute(&mut *student_transaction)
        .await;
    let error = answer_key_read.expect_err("student role must not read answer-bearing tables");
    let sqlx::Error::Database(database_error) = error else {
        panic!("expected PostgreSQL permission error, received {error}");
    };
    assert_eq!(database_error.code().as_deref(), Some("42501"));
    student_transaction
        .rollback()
        .await
        .expect("aborted student transaction should roll back");

    pool.close().await;
}

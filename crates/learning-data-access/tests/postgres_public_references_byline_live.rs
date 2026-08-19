#![cfg(feature = "postgres")]

//! Disposable PostgreSQL 17 oracle for public route references and bylines.

use learning_data_access::postgres::{PostgresStore, lazy_pool, verify_application_schema};
use learning_data_access::{
    AssignmentRecord, CatalogStore, CourseRecord, CourseRosterStore, CreateCourseCommand,
    DraftRecord, NavigationReferenceStore, PublishDraftCommand, Store, TenantContext,
    UpsertCourseMember,
};
use question_model::answer::NumericTolerance;
use question_model::envelope::ContentBlock;
use question_model::generation::RandomizationDefinition;
use question_model::run_policy::{
    AttemptPolicy, CompletionRequirement, ContinuedPractice, FeedbackDisclosure, GradePolicy,
    RunPolicies, TimingPolicy, VariationPolicy,
};
use question_model::taxonomy::License;
use question_model::{
    AssignmentDeliveryState, AssignmentId, AssignmentItem, AssignmentItemId, AssignmentRunTiming,
    AssignmentScoringMode, BackendCapabilities, Capability, CourseGroupReference, CourseId,
    DraftQuestionDefinition, DraftQuestionSource, GradingDefinition, PointValue, ProblemId,
    ProblemVersionRef, PublicAuthorName, PublicByline, PublicationScope, QuestionMetadata,
    QuestionSource, ResponseDefinition, RunId, TenantId, UserId, VersionId, WorkspaceId,
};
use sqlx::{PgPool, Row};
use uuid::Uuid;

fn id() -> Uuid {
    let mut bytes = [0_u8; 16];
    getrandom::fill(&mut bytes).expect("live fixture UUID randomness");
    Uuid::from_bytes(bytes)
}

fn byline() -> PublicByline {
    PublicByline::new(vec![
        PublicAuthorName::new("Ada Lovelace".to_string()).expect("valid byline name"),
        PublicAuthorName::new("Grace Hopper".to_string()).expect("valid byline name"),
    ])
    .expect("valid reviewed byline")
}

fn policies() -> RunPolicies {
    RunPolicies {
        completion: CompletionRequirement::AnswerAll,
        grade: GradePolicy::Highest,
        continued_practice: ContinuedPractice::Unlimited,
        variation: VariationPolicy::NewSeeds,
    }
}

fn draft(tenant: TenantId, workspace: WorkspaceId) -> DraftRecord {
    DraftRecord {
        tenant,
        question: DraftQuestionDefinition {
            workspace,
            source: DraftQuestionSource::Native {
                family: "molar_mass".to_string(),
            },
            prompt: vec![ContentBlock::Text {
                markdown: "Which molar mass is correct?".to_string(),
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
                title: "Byline reference fixture".to_string(),
                tags: Vec::new(),
                taxonomy: Vec::new(),
                license: License::CcBy,
                language: "en-US".to_string(),
            },
        },
        derived_from: None,
    }
}

fn assignment_item(reference: ProblemVersionRef) -> AssignmentItem {
    AssignmentItem {
        id: AssignmentItemId::from_uuid(id()),
        reference,
        position: 0,
        points_possible: PointValue::from_whole(1),
        delivery_state: AssignmentDeliveryState::Active,
        scoring_mode: AssignmentScoringMode::Normal,
    }
}

async fn publish(
    store: &PostgresStore,
    context: TenantContext,
    tenant: TenantId,
    publisher: UserId,
) -> learning_data_access::PublishedProblemRecord {
    let draft = draft(tenant, WorkspaceId::from_uuid(id()));
    let saved = store
        .upsert_draft(context, publisher, None, draft.clone())
        .await
        .expect("save explicit-byline draft");
    store
        .publish_draft(
            context,
            publisher,
            PublishDraftCommand {
                expected_draft: draft,
                expected_revision: saved.revision,
                publication: ProblemVersionRef {
                    problem: ProblemId::from_uuid(id()),
                    version: VersionId::from_uuid(id()),
                },
                published_source: QuestionSource::Native {
                    family: "molar_mass".to_string(),
                },
                publisher,
                scope: PublicationScope::Public,
                byline: byline(),
                source_artifact: None,
                qti_promotion: None,
                flat_question_promotion: None,
                capabilities: BackendCapabilities::from_iter([Capability::ServerGrading]),
            },
        )
        .await
        .expect("publish explicit-byline fixture")
}

fn constraint(error: &sqlx::Error) -> Option<&str> {
    error
        .as_database_error()
        .and_then(|error| error.constraint())
}

fn database_code(error: &sqlx::Error) -> Option<String> {
    error
        .as_database_error()
        .and_then(|error| error.code())
        .map(|code| code.into_owned())
}

async fn assert_invalid_byline_insert_rolls_back(pool: &PgPool) {
    let problem = id();
    let version = id();
    let mut transaction = pool
        .begin()
        .await
        .expect("begin invalid publication transaction");
    sqlx::query(
        "INSERT INTO problem (problem_id, question_id, owner_tenant_id, owner_user_id, visibility, license) \
         VALUES ($1, 'M1N2P3Q', $2, $3, 'public', 'CC-BY-4.0')",
    )
    .bind(problem)
    .bind(id())
    .bind(id())
    .execute(&mut *transaction)
    .await
    .expect("prepare invalid public-byline parent problem atomically");
    let error = sqlx::query(
        "INSERT INTO problem_version \
         (problem_id, version_id, content_sha256, workspace_id, title, author_ids, public_byline) \
         VALUES ($1, $2, repeat('a', 64), $3, 'Rejected byline publication', \
                 jsonb_build_array($4::text), $5)",
    )
    .bind(problem)
    .bind(version)
    .bind(id())
    .bind(id().to_string())
    .bind(Vec::<Option<String>>::new())
    .execute(&mut *transaction)
    .await
    .expect_err("invalid publication byline must fail before publication side effects");
    assert_eq!(
        constraint(&error),
        Some("problem_version_public_byline_check")
    );
    transaction
        .rollback()
        .await
        .expect("discard rejected publication transaction");
    let problem_count: i64 =
        sqlx::query_scalar("SELECT count(*) FROM problem WHERE problem_id = $1")
            .bind(problem)
            .fetch_one(pool)
            .await
            .expect("rejected publication leaves no public identity row");
    let version_count: i64 =
        sqlx::query_scalar("SELECT count(*) FROM problem_version WHERE problem_id = $1")
            .bind(problem)
            .fetch_one(pool)
            .await
            .expect("rejected publication leaves no version row");
    let payload_count: i64 =
        sqlx::query_scalar("SELECT count(*) FROM problem_version_payload WHERE problem_id = $1")
            .bind(problem)
            .fetch_one(pool)
            .await
            .expect("rejected publication leaves no payload row");
    let projection_count: i64 =
        sqlx::query_scalar("SELECT count(*) FROM catalog_search_document WHERE problem_id = $1")
            .bind(problem)
            .fetch_one(pool)
            .await
            .expect("rejected publication leaves no catalog projection");
    assert_eq!(
        problem_count, 0,
        "rejected byline leaves no public identity row"
    );
    assert_eq!(version_count, 0, "rejected byline leaves no version row");
    assert_eq!(payload_count, 0, "rejected byline leaves no payload row");
    assert_eq!(
        projection_count, 0,
        "rejected byline leaves no catalog projection"
    );
}

async fn insert_group_and_read_public_id(pool: &PgPool, tenant: TenantId, course: CourseId) -> i32 {
    let mut transaction = pool.begin().await.expect("begin group allocation probe");
    sqlx::query("SET LOCAL ROLE ple_app")
        .execute(&mut *transaction)
        .await
        .expect("use application RLS role");
    sqlx::query("SELECT set_config('ple.tenant_id', $1, true)")
        .bind(tenant.to_string())
        .execute(&mut *transaction)
        .await
        .expect("set group tenant context");
    let public_id: i32 = sqlx::query(
        "INSERT INTO course_group \
         (tenant_id, course_id, course_group_id, purpose, title) \
         VALUES ($1, $2, $3, 'section', 'Reference group') RETURNING public_id",
    )
    .bind(tenant.as_uuid())
    .bind(course.as_uuid())
    .bind(id())
    .fetch_one(&mut *transaction)
    .await
    .expect("tenant-local group insert allocates global identity")
    .try_get("public_id")
    .expect("group identity is an integer");
    transaction.commit().await.expect("commit group allocation");
    public_id
}

#[tokio::test]
#[ignore = "requires a fresh disposable PostgreSQL 17 database with the full migration chain"]
async fn postgres_public_references_and_bylines_are_normalized_authorized_and_immutable() {
    let database_url = std::env::var("PLE_TEST_DATABASE_URL")
        .expect("PLE_TEST_DATABASE_URL must name the disposable acceptance database");
    let pool = lazy_pool(&database_url).expect("valid live PostgreSQL URL");
    verify_application_schema(&pool)
        .await
        .expect("full migrated application schema is compatible");
    let store = PostgresStore::with_question_id_secret(pool.clone(), [0x7a; 32]);
    let tenant = TenantId::from_uuid(id());
    let foreign_tenant = TenantId::from_uuid(id());
    let context = TenantContext::from_authenticated_session(tenant);
    let foreign_context = TenantContext::from_authenticated_session(foreign_tenant);
    let course = CourseId::from_uuid(id());
    let foreign_course = CourseId::from_uuid(id());
    let instructor = UserId::from_uuid(id());
    let student = UserId::from_uuid(id());
    let outsider = UserId::from_uuid(id());

    store
        .create_course(
            context,
            CreateCourseCommand {
                course: CourseRecord {
                    id: course,
                    tenant,
                    title: "Public reference course".to_string(),
                    term: question_model::CourseTerm::from_parts(
                        "2026-08-24",
                        "2026-12-18",
                        "America/Chicago",
                    )
                    .expect("valid fixture course term"),
                },
                initial_instructor: instructor,
            },
        )
        .await
        .expect("create owning course");
    store
        .create_course(
            foreign_context,
            CreateCourseCommand {
                course: CourseRecord {
                    id: foreign_course,
                    tenant: foreign_tenant,
                    title: "Foreign reference course".to_string(),
                    term: question_model::CourseTerm::from_parts(
                        "2026-08-24",
                        "2026-12-18",
                        "America/Chicago",
                    )
                    .expect("valid foreign fixture course term"),
                },
                initial_instructor: outsider,
            },
        )
        .await
        .expect("create foreign course");
    let published = publish(&store, context, tenant, instructor).await;
    assert_eq!(published.byline, byline());
    let workspace = WorkspaceId::from_uuid(id());
    store
        .upsert_draft(context, instructor, None, draft(tenant, workspace))
        .await
        .expect("create separately addressable workspace fixture");
    let assignment = AssignmentId::from_uuid(id());
    store
        .create_assignment_with_timing(
            context,
            AssignmentRecord {
                id: assignment,
                tenant,
                course_id: course,
                title: "Public-reference practice".to_string(),
                audience: question_model::AssignmentAudience::CourseWide,
                items: vec![assignment_item(ProblemVersionRef {
                    problem: published.problem,
                    version: published.version,
                })],
                selection_groups: Vec::new(),
                policies: policies(),
            },
            AssignmentRunTiming {
                time_limit_seconds: None,
            },
        )
        .await
        .expect("create assignment");
    store
        .upsert_course_member(
            context,
            UpsertCourseMember {
                course,
                user: student,
                display_name: "Reference student".to_string(),
                roster_contact: None,
            },
        )
        .await
        .expect("create student membership and enrollment");
    let run = store
        .start_or_resume_run(context, student, assignment, RunId::from_uuid(id()))
        .await
        .expect("start owned learner run");

    let course_reference = store
        .course_reference(context, instructor, course)
        .await
        .expect("course reference lookup")
        .expect("instructor can see course reference");
    let assignment_reference = store
        .assignment_reference(context, instructor, assignment)
        .await
        .expect("assignment reference lookup")
        .expect("instructor can see assignment reference");
    let run_reference = store
        .run_reference(context, student, run.id)
        .await
        .expect("run reference lookup")
        .expect("owning student can see run reference");
    let workspace_reference = store
        .workspace_reference(context, instructor, workspace)
        .await
        .expect("workspace reference lookup")
        .expect("publisher can see workspace reference");
    assert!(course_reference.number() > 0);
    assert!(assignment_reference.number() > 0);
    assert!(run_reference.number() > 0);
    assert!(workspace_reference.number() > 0);
    assert_eq!(
        course_reference.to_string().parse(),
        Ok(course_reference),
        "course reference persists as its complete public string"
    );
    assert_eq!(
        assignment_reference.to_string().parse(),
        Ok(assignment_reference),
        "assignment reference persists as its complete public string"
    );
    assert_eq!(
        run_reference.to_string().parse(),
        Ok(run_reference),
        "run reference persists as its complete public string"
    );
    assert_eq!(
        workspace_reference.to_string().parse(),
        Ok(workspace_reference),
        "workspace reference persists as its complete public string"
    );
    assert_eq!(
        store
            .resolve_course_reference(context, instructor, course_reference)
            .await,
        Ok(Some(course))
    );
    assert_eq!(
        store
            .resolve_course_reference(context, outsider, course_reference)
            .await,
        Ok(None)
    );
    assert_eq!(
        store
            .resolve_course_reference(foreign_context, outsider, course_reference)
            .await,
        Ok(None)
    );
    assert_eq!(
        store
            .resolve_assignment_reference(context, instructor, assignment_reference)
            .await
            .expect("instructor resolves exact assignment")
            .expect("assignment remains visible")
            .assignment,
        assignment
    );
    assert_eq!(
        store
            .resolve_assignment_reference(context, outsider, assignment_reference)
            .await,
        Ok(None)
    );
    assert_eq!(
        store
            .resolve_assignment_reference(foreign_context, outsider, assignment_reference)
            .await,
        Ok(None)
    );
    let run_identity = store
        .resolve_run_reference(context, student, run_reference)
        .await
        .expect("student resolves own current run")
        .expect("owned run remains visible");
    assert_eq!(run_identity.course, course);
    assert_eq!(run_identity.assignment, assignment);
    assert_eq!(run_identity.enrollment, run.enrollment);
    assert_eq!(run_identity.run, run.id);
    assert_eq!(
        store
            .resolve_run_reference(context, instructor, run_reference)
            .await,
        Ok(Some(run_identity))
    );
    assert_eq!(
        store
            .resolve_run_reference(context, outsider, run_reference)
            .await,
        Ok(None)
    );
    assert_eq!(
        store
            .resolve_run_reference(foreign_context, outsider, run_reference)
            .await,
        Ok(None)
    );
    assert_eq!(
        store
            .resolve_workspace_reference(context, instructor, workspace_reference)
            .await,
        Ok(Some(workspace))
    );
    assert_eq!(
        store
            .resolve_workspace_reference(context, outsider, workspace_reference)
            .await,
        Ok(None)
    );
    assert_eq!(
        store
            .resolve_workspace_reference(foreign_context, outsider, workspace_reference)
            .await,
        Ok(None)
    );

    let own_group = insert_group_and_read_public_id(&pool, tenant, course).await;
    let foreign_group =
        insert_group_and_read_public_id(&pool, foreign_tenant, foreign_course).await;
    assert!(own_group > 0 && foreign_group > 0);
    assert_ne!(own_group, foreign_group, "group route scalar is global");
    let group_reference = CourseGroupReference::new(own_group as u64)
        .expect("positive PostgreSQL group identity is a route reference");
    assert_eq!(
        group_reference.to_string().parse(),
        Ok(group_reference),
        "group reference persists as its complete public string"
    );
    let mut foreign_group_read = pool.begin().await.expect("begin foreign group RLS probe");
    sqlx::query("SET LOCAL ROLE ple_app")
        .execute(&mut *foreign_group_read)
        .await
        .expect("use application RLS role");
    sqlx::query("SELECT set_config('ple.tenant_id', $1, true)")
        .bind(foreign_tenant.to_string())
        .execute(&mut *foreign_group_read)
        .await
        .expect("set foreign tenant context");
    let visible: i64 = sqlx::query_scalar("SELECT count(*) FROM course_group WHERE public_id = $1")
        .bind(own_group)
        .fetch_one(&mut *foreign_group_read)
        .await
        .expect("foreign group query is filtered");
    assert_eq!(visible, 0);
    foreign_group_read
        .rollback()
        .await
        .expect("end foreign RLS probe");

    for invalid in [
        Vec::new(),
        vec![Some("Ada".to_string()), Some("Ada".to_string())],
        vec![Some(" Ada".to_string())],
        vec![Some(String::new())],
        vec![Some("Ada\n".to_string())],
        vec![Some("a".repeat(121))],
        vec![Some("Ada".to_string()), None],
        vec![Some("Ada".to_string()); 17],
    ] {
        let valid: bool = sqlx::query_scalar("SELECT public.ple_valid_public_byline($1)")
            .bind(invalid)
            .fetch_one(&pool)
            .await
            .expect("native byline predicate evaluates every malformed shape");
        assert!(!valid, "native byline predicate rejects malformed input");
    }
    assert_invalid_byline_insert_rolls_back(&pool).await;
    let (stored_byline, search_text): (Vec<String>, String) = sqlx::query_as(
        "SELECT public_byline, byline_text FROM catalog_search_document WHERE problem_id = $1 AND version_id = $2",
    )
    .bind(published.problem.as_uuid())
    .bind(published.version.as_uuid())
    .fetch_one(&pool)
    .await
    .expect("catalog projection persists immutable reviewed byline");
    assert_eq!(stored_byline, vec!["Ada Lovelace", "Grace Hopper"]);
    assert_eq!(search_text, "Ada Lovelace, Grace Hopper");
    assert!(!search_text.contains(&instructor.to_string()));
    let document: serde_json::Value = sqlx::query_scalar(
        "SELECT to_jsonb(document) FROM catalog_search_document AS document \
         WHERE problem_id = $1 AND version_id = $2",
    )
    .bind(published.problem.as_uuid())
    .bind(published.version.as_uuid())
    .fetch_one(&pool)
    .await
    .expect("catalog projection remains browser-safe");
    assert!(
        !document.to_string().contains(&instructor.to_string()),
        "catalog projection contains the reviewed byline, never private author UUID authority"
    );
    let immutable_error = sqlx::query(
        "UPDATE problem_version SET public_byline = ARRAY['Different name'] \
         WHERE problem_id = $1 AND version_id = $2",
    )
    .bind(published.problem.as_uuid())
    .bind(published.version.as_uuid())
    .execute(&pool)
    .await
    .expect_err("published byline is an immutable publication snapshot");
    assert_eq!(database_code(&immutable_error).as_deref(), Some("55000"));
    let unchanged: (Vec<String>, String) = sqlx::query_as(
        "SELECT public_byline, byline_text FROM catalog_search_document WHERE problem_id = $1 AND version_id = $2",
    )
    .bind(published.problem.as_uuid())
    .bind(published.version.as_uuid())
    .fetch_one(&pool)
    .await
    .expect("immutable publication keeps catalog projection unchanged");
    assert_eq!(unchanged, (stored_byline.clone(), search_text.clone()));
    let byline_search = store
        .search_catalog(
            context,
            question_model::CatalogSearchQuery {
                text: Some("Grace Hopper".to_string()),
                ..question_model::CatalogSearchQuery::default()
            },
        )
        .await
        .expect("byline is a catalog search value");
    assert!(
        byline_search
            .items
            .iter()
            .any(|item| item.byline == byline())
    );
}

#![cfg(feature = "postgres")]

//! Disposable PostgreSQL oracle for atomic assignment-editor timing saves.

use learning_data_access::postgres::{PostgresStore, lazy_pool, verify_application_schema};
use learning_data_access::{
    AssignmentEditorUpdate, AssignmentRecord, AuthoritativeTimeStore, CatalogStore, CourseRecord,
    CourseRosterStore, DraftRecord, FlatGradingCapability, IssueQuestionAttemptCommand,
    PresentationCapability, PublishDraftCommand, Store, StoreError, TenantContext,
    UpdateAssignmentTimingCommand, UpsertCourseMember,
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
    ActivityTimestamp, AssignmentDeadlineBehavior, AssignmentDeliveryState, AssignmentId,
    AssignmentItem, AssignmentItemId, AssignmentRunTiming, AssignmentScoringMode,
    AssignmentTimingPolicy, AttemptProvenance, BackendCapabilities, Capability, CourseId,
    CourseMembership, CourseMembershipRole, DraftQuestionDefinition, DraftQuestionSource,
    GradingDefinition, ImplementationVersion, LateSubmissionPolicy, PointValue, ProblemId,
    ProblemVersionRef, PublicationScope, QuestionAttemptId, QuestionMetadata, QuestionSource,
    ResponseDefinition, RunId, TenantId, UserId, VersionId, WorkspaceId,
};
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

fn editor_update(title: &str, items: Vec<AssignmentItem>, seconds: u32) -> AssignmentEditorUpdate {
    AssignmentEditorUpdate {
        assignment: learning_data_access::AssignmentUpdate {
            title: title.to_string(),
            items,
            selection_groups: Vec::new(),
            policies: policies(),
        },
        assignment_timing: AssignmentRunTiming {
            time_limit_seconds: Some(seconds),
        },
    }
}

fn provenance() -> AttemptProvenance {
    AttemptProvenance {
        adapter: ImplementationVersion {
            id: "postgres-timing-live".to_string(),
            version: "1".to_string(),
        },
        renderer: None,
        generator: None,
        source_artifact: None,
        asset_objects: Vec::new(),
        grading: ImplementationVersion {
            id: "postgres-timing-live-grading".to_string(),
            version: "1".to_string(),
        },
        rendered_question_sha256: "postgres-timing-live-render".to_string(),
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
                markdown: "Live assignment timing question".to_string(),
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
                title: "Live assignment timing question".to_string(),
                tags: Vec::new(),
                taxonomy: Vec::new(),
                license: License::CcBy,
                language: "en-US".to_string(),
            },
        },
        revises: None,
        derived_from: None,
    };
    let saved = store
        .upsert_draft(context, instructor, None, draft.clone())
        .await
        .expect("save live timing draft");
    store
        .publish_draft(
            context,
            instructor,
            PublishDraftCommand {
                expected_draft: draft,
                expected_revision: saved.revision,
                publication: reference,
                published_source: QuestionSource::Native {
                    family: "molar_mass".to_string(),
                },
                publisher: instructor,
                scope: PublicationScope::Public,
                source_artifact: None,
                qti_promotion: None,
                flat_question_promotion: None,
                capabilities: BackendCapabilities::from_iter([Capability::ServerGrading]),
            },
        )
        .await
        .expect("publish live timing question");
    reference
}

#[tokio::test]
#[ignore = "requires the disposable PostgreSQL acceptance database"]
async fn postgres_assignment_editor_timing_is_atomic_and_reschedules_active_work() {
    let database_url = std::env::var("PLE_TEST_DATABASE_URL")
        .expect("PLE_TEST_DATABASE_URL must name the disposable acceptance database");
    let pool = lazy_pool(&database_url).expect("valid live PostgreSQL URL");
    verify_application_schema(&pool)
        .await
        .expect("live PostgreSQL schema compatibility");
    let store = PostgresStore::new(pool.clone());
    let tenant = TenantId::from_uuid(id());
    let context = TenantContext::from_authenticated_session(tenant);
    let course = CourseId::from_uuid(id());
    let instructor = UserId::from_uuid(id());
    let student = UserId::from_uuid(id());
    store
        .upsert_course(
            context,
            CourseRecord {
                id: course,
                tenant,
                title: "Live assignment timing course".to_string(),
                members: vec![CourseMembership {
                    user: instructor,
                    role: CourseMembershipRole::Instructor,
                }],
            },
        )
        .await
        .expect("create timing course");
    let reference = publish_question(&store, context, tenant, instructor).await;
    let assignment = AssignmentId::from_uuid(id());
    let created = store
        .create_assignment_with_timing(
            context,
            AssignmentRecord {
                id: assignment,
                tenant,
                course_id: course,
                title: "Timed practice".to_string(),
                items: vec![assignment_item(reference)],
                selection_groups: Vec::new(),
                policies: policies(),
            },
            AssignmentRunTiming {
                time_limit_seconds: Some(900),
            },
        )
        .await
        .expect("create definition and timer together");
    assert_eq!(created.revision.value(), 1);
    assert_eq!(created.assignment_timing.time_limit_seconds, Some(900));
    store
        .upsert_course_member(
            context,
            UpsertCourseMember {
                course,
                user: student,
                display_name: "Live timing student".to_string(),
                roster_contact: None,
            },
        )
        .await
        .expect("canonical roster upsert derives the timed assignment enrollment");

    let now = store
        .authoritative_time(context)
        .await
        .expect("database authoritative time");
    let available_at = ActivityTimestamp::from_unix_millis(now.as_unix_millis() - 60_000);
    let due_at = ActivityTimestamp::from_unix_millis(now.as_unix_millis() + 3_600_000);
    let closes_at = ActivityTimestamp::from_unix_millis(now.as_unix_millis() + 7_200_000);
    let configured = store
        .update_assignment_timing(
            context,
            UpdateAssignmentTimingCommand {
                actor: instructor,
                course,
                assignment,
                expected_revision: created.revision,
                policy: AssignmentTimingPolicy {
                    visible: true,
                    available_at: Some(available_at),
                    due_at: Some(due_at),
                    closes_at: Some(closes_at),
                    late_submission: LateSubmissionPolicy::MarkLate,
                    time_limit_seconds: Some(900),
                    attempt_limit: Some(3),
                    deadline_behavior: AssignmentDeadlineBehavior::AutoSubmit,
                },
            },
        )
        .await
        .expect("configure separate access policy");
    let run = store
        .start_or_resume_run(context, student, assignment, RunId::from_uuid(id()))
        .await
        .expect("start timed run");
    let issued = store
        .issue_or_resume_question_attempt(
            context,
            IssueQuestionAttemptCommand {
                actor: student,
                attempt: QuestionAttemptId::from_uuid(id()),
                run: run.id,
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
                webwork_grading_capability:
                    learning_data_access::WebworkGradingCapability::NotApplicable,
                parameter_hash: "postgres-timing-live-parameters".to_string(),
                provenance: provenance(),
                webwork_replay: None,
                prefetched: None,
                predecessor_submission: None,
            },
        )
        .await
        .expect("issue timed question");
    let initial_deadline = issued.timer.deadline.expect("created timer has a deadline");

    let retimed = store
        .replace_assignment_with_timing(
            context,
            course,
            assignment,
            configured.revision,
            editor_update("Retimed practice", created.record.items.clone(), 1_200),
        )
        .await
        .expect("one editor save changes definition and timer");
    assert_eq!(retimed.revision.value(), configured.revision.value() + 1);
    let retimed_attempt = store
        .get_question_attempt(context, issued.id)
        .await
        .expect("read retimed attempt")
        .expect("issued attempt remains current");
    assert!(
        retimed_attempt.timer.deadline.expect("retimed deadline") > initial_deadline,
        "the active attempt receives the longer whole-run deadline"
    );

    let policy_after_retime = store
        .get_assignment_timing(context, assignment)
        .await
        .expect("read retimed policy")
        .expect("retimed policy exists");
    assert_eq!(policy_after_retime.revision, retimed.revision);
    assert_eq!(policy_after_retime.policy.available_at, Some(available_at));
    assert_eq!(policy_after_retime.policy.due_at, Some(due_at));
    assert_eq!(policy_after_retime.policy.closes_at, Some(closes_at));
    assert_eq!(
        policy_after_retime.policy.late_submission,
        LateSubmissionPolicy::MarkLate
    );
    assert_eq!(policy_after_retime.policy.attempt_limit, Some(3));

    assert_eq!(
        store
            .replace_assignment_with_timing(
                context,
                course,
                assignment,
                configured.revision,
                editor_update("Stale editor", retimed.record.items.clone(), 600),
            )
            .await,
        Err(StoreError::Conflict),
        "a stale editor request changes neither title nor timer"
    );
    assert_eq!(
        store
            .get_assignment_for_edit(context, assignment)
            .await
            .expect("read after refused stale editor")
            .expect("assignment remains present"),
        retimed
    );

    let content_only = store
        .replace_assignment_with_timing(
            context,
            course,
            assignment,
            retimed.revision,
            editor_update(
                "Retimed practice title",
                retimed.record.items.clone(),
                1_200,
            ),
        )
        .await
        .expect("content-only editor save");
    let after_content_only = store
        .get_question_attempt(context, issued.id)
        .await
        .expect("read after content-only save")
        .expect("attempt remains current");
    assert_eq!(after_content_only.timer, retimed_attempt.timer);
    assert_eq!(
        store
            .get_assignment_timing(context, assignment)
            .await
            .expect("read policy after content-only save")
            .expect("policy remains present")
            .policy,
        policy_after_retime.policy
    );

    // Hold the same `FOR SHARE` header lock used by the editor reader while a
    // replacement starts. The relation read under that lock is an all-old
    // editor snapshot; releasing it allows the waiting replacement to commit.
    let mut reader = pool.begin().await.expect("begin reader transaction");
    sqlx::query("SET LOCAL ROLE ple_app")
        .execute(&mut *reader)
        .await
        .expect("reader assumes application role");
    sqlx::query("SELECT set_config('ple.tenant_id', $1, true)")
        .bind(tenant.to_string())
        .execute(&mut *reader)
        .await
        .expect("reader sets tenant context");
    let old_header: (String, i64, Option<i32>) = sqlx::query_as(
        "SELECT title, revision, time_limit_seconds FROM assignment \
         WHERE tenant_id = $1 AND assignment_id = $2 FOR SHARE",
    )
    .bind(tenant.as_uuid())
    .bind(assignment.as_uuid())
    .fetch_one(&mut *reader)
    .await
    .expect("lock editor header");
    let expected_old_item_id = content_only.record.items[0].id.as_uuid();
    let replacement_store = store.clone();
    let replacement = tokio::spawn(async move {
        replacement_store
            .replace_assignment_with_timing(
                context,
                course,
                assignment,
                content_only.revision,
                editor_update(
                    "Coherent replacement",
                    content_only.record.items.clone(),
                    1_500,
                ),
            )
            .await
    });
    let old_item_id: Uuid = sqlx::query_scalar(
        "SELECT assignment_item_id FROM assignment_item \
         WHERE tenant_id = $1 AND assignment_id = $2",
    )
    .bind(tenant.as_uuid())
    .bind(assignment.as_uuid())
    .fetch_one(&mut *reader)
    .await
    .expect("read old editor relations while header lock is held");
    reader
        .commit()
        .await
        .expect("release coherent editor snapshot");
    let replaced = replacement
        .await
        .expect("replacement task completes")
        .expect("replacement commits after reader releases header lock");
    assert_eq!(old_header.0, "Retimed practice title");
    assert_eq!(
        old_header.1,
        i64::try_from(content_only.revision.value()).expect("revision fits")
    );
    assert_eq!(old_header.2, Some(1_200));
    assert_eq!(old_item_id, expected_old_item_id);
    assert_eq!(replaced.record.title, "Coherent replacement");
    assert_eq!(replaced.assignment_timing.time_limit_seconds, Some(1_500));
}

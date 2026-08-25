#![cfg(feature = "postgres")]

//! Disposable PostgreSQL oracle for the T3 preview plane.
//!
//! Run from the private acceptance runtime workspace. This target remains ignored because its fixtures create normal
//! accounts, course members, assignments, and one intentionally auditable
//! derived preview subject.

#[path = "postgres_course_creation_support.rs"]
mod course_creation_support;
use course_creation_support::sysadmin_course_creation_authority;

#[path = "fixtures/published_assignment.rs"]
mod published_assignment;

use domain::effective_assignment_policy::BaseAssignmentPolicy;
use learning_data_access::postgres::{PostgresStore, lazy_pool, verify_application_schema};
use learning_data_access::{
    AssignmentRecord, CatalogStore, CourseRecord, CourseRosterStore, CreateAssignmentCommand,
    CreateCourseCommand, DraftRecord, NavigationReferenceStore, PreviewPlaneStore, Store,
    StoreError, TenantContext, UpsertCourseMember,
};
use published_assignment::create_published_assignment;
use question_model::answer::NumericTolerance;
use question_model::envelope::ContentBlock;
use question_model::generation::RandomizationDefinition;
use question_model::run_policy::{
    AttemptPolicy, CompletionRequirement, ContinuedPractice, GradePolicy, RunPolicies,
    TimingPolicy, VariationPolicy,
};
use question_model::{
    ActivityTimestamp, AssignmentAudience, AssignmentDeliveryState, AssignmentId,
    AssignmentInstructions, AssignmentItem, AssignmentItemId, AssignmentLifecycle,
    AssignmentScoringMode, BackendCapabilities, Capability, CourseId, CourseLocalDateTime,
    CourseTerm, DraftQuestionDefinition, DraftQuestionSource, GradingDefinition, IanaTimeZone,
    LateSubmissionPolicy, PointValue, PreviewSelectedMoment, PreviewSyntheticGroupReferences,
    ProblemId, ProblemVersionRef, PublicationScope, QuestionMetadata, QuestionSource,
    ResponseDefinition, SyntheticPreviewModifiers, SyntheticPreviewSubjectRequest,
    TeachingOperationRevision, TenantId, UserId, VersionId, WorkspaceId,
};
use sqlx::Row;
use std::num::NonZeroU32;
use uuid::Uuid;

fn id() -> Uuid {
    let mut bytes = [0; 16];
    getrandom::fill(&mut bytes).expect("fixture randomness");
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
                family: "t3_preview_live".into(),
            },
            prompt: vec![ContentBlock::Text {
                markdown: "T3 fixture".into(),
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
                title: "T3 fixture".into(),
                tags: vec![],
                taxonomy: vec![],
                license: question_model::taxonomy::License::CcBy,
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
                    family: "t3_preview_live".into(),
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
        .expect("published fixture");
    reference
}

async fn count(pool: &sqlx::PgPool, tenant: TenantId, table: &str) -> i64 {
    let sql = match table {
        "audit_event" => "SELECT count(*) FROM audit_event WHERE tenant_id=$1",
        "course_grade_export_audit" => {
            "SELECT count(*) FROM course_grade_export_audit WHERE tenant_id=$1"
        }
        "enrollment" => "SELECT count(*) FROM enrollment WHERE tenant_id=$1",
        "assignment_run" => "SELECT count(*) FROM assignment_run WHERE tenant_id=$1",
        "question_attempt" => "SELECT count(*) FROM question_attempt WHERE tenant_id=$1",
        "worker_job" => "SELECT count(*) FROM worker_job WHERE tenant_id=$1",
        _ => panic!("audited table"),
    };
    sqlx::query_scalar(sql)
        .bind(tenant.as_uuid())
        .fetch_one(pool)
        .await
        .expect("count")
}

/// The mutable relations a preview must not touch, including the independent
/// grade-export audit retained for instructor exports.
async fn preview_effect_counts(pool: &sqlx::PgPool, tenant: TenantId) -> [i64; 6] {
    [
        count(pool, tenant, "audit_event").await,
        count(pool, tenant, "course_grade_export_audit").await,
        count(pool, tenant, "enrollment").await,
        count(pool, tenant, "assignment_run").await,
        count(pool, tenant, "question_attempt").await,
        count(pool, tenant, "worker_job").await,
    ]
}

async fn membership_reference(
    pool: &sqlx::PgPool,
    tenant: TenantId,
    course: CourseId,
    user: UserId,
) -> question_model::CourseMembershipReference {
    let number: i32 = sqlx::query_scalar(
        "SELECT public_id FROM course_member \
         WHERE tenant_id=$1 AND course_id=$2 AND user_id=$3 AND role='student'",
    )
    .bind(tenant.as_uuid())
    .bind(course.as_uuid())
    .bind(user.as_uuid())
    .fetch_one(pool)
    .await
    .expect("persisted student membership reference");
    question_model::CourseMembershipReference::new(
        u64::try_from(number).expect("positive public membership reference"),
    )
    .expect("well-formed public membership reference")
}

#[tokio::test]
#[ignore = "requires the private acceptance runtime workspace"]
async fn postgres_preview_plane_live_oracle_is_authorized_atomic_and_identity_free() {
    let runtime = load_acceptance_runtime();
    let url = runtime.admin_url().expose();
    let pool = lazy_pool(url).expect("PostgreSQL URL");
    verify_application_schema(&pool)
        .await
        .expect("migrated schema");
    let store = PostgresStore::with_question_id_secret(pool.clone(), [0x73; 32]);
    let tenant = TenantId::from_uuid(id());
    let foreign = TenantId::from_uuid(id());
    let context = TenantContext::from_authenticated_session(tenant);
    let instructor = UserId::from_uuid(id());
    let outsider = UserId::from_uuid(id());
    let learner_a = UserId::from_uuid(id());
    let learner_b = UserId::from_uuid(id());
    let course = CourseId::from_uuid(id());
    store
        .create_course(
            context,
            CreateCourseCommand {
                course: CourseRecord {
                    id: course,
                    tenant,
                    title: "T3 preview oracle".into(),
                    term: CourseTerm::from_parts("2026-01-01", "2026-12-31", "America/Chicago")
                        .expect("term"),
                },
                authority: sysadmin_course_creation_authority(&store, tenant, course, instructor)
                    .await,
            },
        )
        .await
        .expect("course");
    for (user, display) in [
        (learner_a, "Preview learner A"),
        (learner_b, "Preview learner B"),
    ] {
        store
            .upsert_course_member(
                context,
                instructor,
                UpsertCourseMember {
                    course,
                    user,
                    display_name: display.into(),
                    roster_contact: None,
                },
            )
            .await
            .expect("learner");
    }
    let publication = publish(&store, context, tenant, instructor).await;
    let assignment = AssignmentId::from_uuid(id());
    let policy = BaseAssignmentPolicy {
        available_at: Some(ActivityTimestamp::from_unix_millis(1_787_580_000_000)),
        due_at: None,
        closes_at: None,
        time_limit_seconds: Some(NonZeroU32::new(300).expect("positive")),
        attempt_limit: Some(NonZeroU32::new(2).expect("positive")),
        late_submission: LateSubmissionPolicy::Accept,
        deadline_behavior: question_model::AssignmentDeadlineBehavior::AutoSubmit,
    };
    let assignment_record = AssignmentRecord {
        id: assignment,
        tenant,
        course_id: course,
        title: "T3 preview assignment".into(),
        lifecycle: AssignmentLifecycle::Published,
        instructions: AssignmentInstructions::try_new("Preview only".into()).expect("instructions"),
        audience: AssignmentAudience::CourseWide,
        items: vec![AssignmentItem {
            id: AssignmentItemId::from_uuid(id()),
            reference: publication,
            position: 0,
            points_possible: PointValue::from_whole(1),
            delivery_state: AssignmentDeliveryState::Active,
            scoring_mode: AssignmentScoringMode::Normal,
        }],
        selection_groups: vec![],
        disclosure_policy: question_model::LearnerDisclosurePolicy::default(),
        policies: policies(),
    };
    let created = create_published_assignment(
        &store,
        context,
        instructor,
        assignment_record.clone(),
        policy,
    )
    .await
    .expect("published assignment");
    let reference = store
        .assignment_reference(context, instructor, assignment)
        .await
        .expect("reference")
        .expect("reference");
    let revision = TeachingOperationRevision::new(created.revision.value()).expect("revision");
    let moment = PreviewSelectedMoment {
        value: CourseLocalDateTime::parse("2026-08-25T09:00:00.000").expect("time"),
        time_zone: IanaTimeZone::parse("America/Chicago").expect("zone"),
    };
    let before_effects = preview_effect_counts(&pool, tenant).await;
    let first = store
        .list_instructor_preview_schedule(
            context,
            instructor,
            course,
            reference,
            revision,
            learning_data_access::PageRequest::first(
                learning_data_access::PageSize::new(1).expect("size"),
            ),
        )
        .await
        .expect("page one");
    let next = first.next_cursor.clone().expect("second page");
    let second = store
        .list_instructor_preview_schedule(
            context,
            instructor,
            course,
            reference,
            revision,
            learning_data_access::PageRequest::after(
                learning_data_access::Cursor::parse(next).expect("cursor"),
                learning_data_access::PageSize::new(1).expect("size"),
            ),
        )
        .await
        .expect("page two");
    assert_eq!(
        first.rows.len() + second.rows.len(),
        2,
        "stable schedule pagination"
    );
    assert_eq!(
        first,
        store
            .list_instructor_preview_schedule(
                context,
                instructor,
                course,
                reference,
                revision,
                learning_data_access::PageRequest::first(
                    learning_data_access::PageSize::new(1).expect("size"),
                ),
            )
            .await
            .expect("same snapshot-shaped schedule read"),
        "idempotent schedule reads retain the stable projection"
    );
    assert!(matches!(
        store
            .construct_synthetic_preview(
                context,
                instructor,
                course,
                SyntheticPreviewSubjectRequest {
                    assignment: reference,
                    revision,
                    selected_moment: moment.clone(),
                    groups: PreviewSyntheticGroupReferences::try_from(vec![]).expect("groups"),
                    modifiers: SyntheticPreviewModifiers {
                        mode: question_model::PolicyModificationModeView::ExtendOnly,
                        patch: question_model::PolicyPatchView {
                            available_at: question_model::TeachingTimeFieldPatch::Inherit,
                            due_at: question_model::TeachingTimeFieldPatch::Inherit,
                            closes_at: question_model::TeachingTimeFieldPatch::Inherit,
                            time_limit_seconds: question_model::TeachingLimitFieldPatch::Inherit,
                            attempt_limit: question_model::TeachingAttemptLimitFieldPatch::Inherit
                        }
                    }
                }
            )
            .await
            .expect("synthetic")
            .evaluation,
        question_model::PreviewEvaluation::Allowed { .. }
    ));
    assert_eq!(
        preview_effect_counts(&pool, tenant).await,
        before_effects,
        "synthetic is read-only"
    );
    let membership = match &first.rows[0] {
        question_model::InstructorPreviewScheduleRow::Granted { membership, .. }
        | question_model::InstructorPreviewScheduleRow::Denied { membership, .. } => *membership,
    };
    assert_eq!(
        store
            .construct_derived_preview(
                context,
                outsider,
                course,
                question_model::DerivedPreviewSubjectRequest {
                    assignment: reference,
                    revision,
                    selected_moment: moment.clone(),
                    membership
                }
            )
            .await,
        Err(StoreError::NotFound),
        "outsider cannot enumerate"
    );
    assert_eq!(
        preview_effect_counts(&pool, tenant).await,
        before_effects,
        "refusal has no audit"
    );
    assert_eq!(
        store
            .construct_derived_preview(
                context,
                instructor,
                course,
                question_model::DerivedPreviewSubjectRequest {
                    assignment: reference,
                    revision: TeachingOperationRevision::new(revision.value() + 1)
                        .expect("stale revision"),
                    selected_moment: moment.clone(),
                    membership,
                },
            )
            .await,
        Err(StoreError::Conflict),
        "stale revision is refused before subject construction"
    );
    assert_eq!(
        preview_effect_counts(&pool, tenant).await,
        before_effects,
        "stale refusal has no audit"
    );
    let revoked_membership = membership_reference(&pool, tenant, course, learner_b).await;
    let mut revocation = pool.begin().await.expect("begin membership revocation");
    sqlx::query("SELECT set_config('ple.tenant_id', $1, true)")
        .bind(tenant.to_string())
        .execute(&mut *revocation)
        .await
        .expect("bind membership revocation tenant");
    let revoked: String = sqlx::query_scalar(
        "UPDATE course_member SET status='revoked', revoked_at=transaction_timestamp() \
         WHERE tenant_id=$1 AND course_id=$2 AND user_id=$3 RETURNING status",
    )
    .bind(tenant.as_uuid())
    .bind(course.as_uuid())
    .bind(learner_b.as_uuid())
    .fetch_one(&mut *revocation)
    .await
    .expect("persist revoked Student membership");
    revocation
        .commit()
        .await
        .expect("commit membership revocation");
    assert_eq!(revoked, "revoked", "fixture persists revoked membership");
    let before_revoked = preview_effect_counts(&pool, tenant).await;
    assert_eq!(
        store
            .construct_derived_preview(
                context,
                instructor,
                course,
                question_model::DerivedPreviewSubjectRequest {
                    assignment: reference,
                    revision,
                    selected_moment: moment.clone(),
                    membership: revoked_membership,
                },
            )
            .await,
        Err(StoreError::NotFound),
        "a persisted revoked M-reference is concealed"
    );
    assert_eq!(
        preview_effect_counts(&pool, tenant).await,
        before_revoked,
        "revoked M-reference refusal has zero preview audit or state mutation"
    );
    let mut draft_record = assignment_record;
    draft_record.id = AssignmentId::from_uuid(id());
    draft_record.items[0].id = AssignmentItemId::from_uuid(id());
    draft_record.title = "T3 Draft preview assignment".into();
    draft_record.lifecycle = AssignmentLifecycle::Draft;
    let draft = store
        .create_assignment(
            context,
            CreateAssignmentCommand {
                actor: instructor,
                assignment: draft_record,
                base_policy: policy,
            },
        )
        .await
        .expect("Draft assignment");
    let draft_reference = store
        .assignment_reference(context, instructor, draft.record.id)
        .await
        .expect("Draft assignment reference")
        .expect("Draft assignment reference");
    let draft_revision = TeachingOperationRevision::new(draft.revision.value()).expect("revision");
    let before_draft = preview_effect_counts(&pool, tenant).await;
    assert!(matches!(
        store
            .construct_derived_preview(
                context,
                instructor,
                course,
                question_model::DerivedPreviewSubjectRequest {
                    assignment: draft_reference,
                    revision: draft_revision,
                    selected_moment: moment.clone(),
                    membership,
                },
            )
            .await,
        Ok(learning_data_access::PreviewPlaneResult {
            evaluation: question_model::PreviewEvaluation::Denied {
                reason: question_model::PreviewDenialReason::NotEntitled,
            },
            accommodation: None,
        })
    ));
    assert_eq!(
        preview_effect_counts(&pool, tenant).await,
        before_draft,
        "Draft derived request has zero preview audit or state mutation"
    );
    for invalid_moment in [
        PreviewSelectedMoment {
            value: CourseLocalDateTime::parse("2026-08-25T09:00:00.000").expect("time"),
            time_zone: IanaTimeZone::parse("America/New_York").expect("wrong zone"),
        },
        PreviewSelectedMoment {
            value: CourseLocalDateTime::parse("2026-03-08T02:30:00.000").expect("DST gap"),
            time_zone: IanaTimeZone::parse("America/Chicago").expect("Chicago zone"),
        },
    ] {
        let before_invalid_moment = preview_effect_counts(&pool, tenant).await;
        assert!(matches!(
            store
                .construct_derived_preview(
                    context,
                    instructor,
                    course,
                    question_model::DerivedPreviewSubjectRequest {
                        assignment: reference,
                        revision,
                        selected_moment: invalid_moment,
                        membership,
                    },
                )
                .await,
            Err(StoreError::InvalidRecord(_))
        ));
        assert_eq!(
            preview_effect_counts(&pool, tenant).await,
            before_invalid_moment,
            "wrong-zone and Chicago DST-gap refusals have zero preview audit or state mutation"
        );
    }
    let derived = store
        .construct_derived_preview(
            context,
            instructor,
            course,
            question_model::DerivedPreviewSubjectRequest {
                assignment: reference,
                revision,
                selected_moment: moment,
                membership,
            },
        )
        .await
        .expect("derived");
    assert!(matches!(
        derived.evaluation,
        question_model::PreviewEvaluation::Allowed { .. }
    ));
    assert_eq!(
        count(&pool, tenant, "audit_event").await,
        before_effects[0] + 1,
        "one derived audit"
    );
    let row = sqlx::query(
        "SELECT payload, payload_sha256 FROM audit_event WHERE tenant_id=$1 \
         AND action='preview.subject.derived' ORDER BY occurred_at DESC LIMIT 1",
    )
    .bind(tenant.as_uuid())
    .fetch_one(&pool)
    .await
    .expect("audit");
    let payload: serde_json::Value = row.try_get("payload").expect("payload");
    let checksum: String = row.try_get("payload_sha256").expect("checksum");
    assert_eq!(
        checksum,
        objects::Sha256Digest::compute(&serde_json::to_vec(&payload).expect("json")).to_string()
    );
    let text = payload.to_string();
    for forbidden in ["Preview learner", "M-", "U-", "@"] {
        assert!(
            !text.contains(forbidden),
            "audit payload protects {forbidden}"
        );
    }
    assert!(
        store
            .list_instructor_preview_schedule(
                TenantContext::from_authenticated_session(foreign),
                instructor,
                course,
                reference,
                revision,
                learning_data_access::PageRequest::first(
                    learning_data_access::PageSize::new(1).expect("size")
                )
            )
            .await
            .is_err(),
        "foreign RLS conceals course"
    );
    let after_derived = preview_effect_counts(&pool, tenant).await;
    assert_eq!(
        after_derived[1..],
        before_effects[1..],
        "only derived audit changes state"
    );
}
#[path = "support/acceptance_runtime.rs"]
mod acceptance_runtime;
use acceptance_runtime::load as load_acceptance_runtime;

use super::*;

pub(super) fn id() -> Uuid {
    let mut bytes = [0; 16];
    getrandom::fill(&mut bytes).expect("uuid");
    Uuid::from_bytes(bytes)
}
pub(super) fn policies() -> RunPolicies {
    RunPolicies {
        completion: CompletionRequirement::AnswerAll,
        grade: GradePolicy::Highest,
        continued_practice: ContinuedPractice::Unlimited,
        variation: VariationPolicy::NewSeeds,
    }
}

pub(super) async fn publish(
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

pub(super) fn issue(
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
pub(super) async fn current(
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
pub(super) async fn revision(
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

pub(super) fn assignment_update(
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

pub(super) async fn receipt_generations(
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

pub(super) async fn group_rows_for_app(
    pool: &sqlx::PgPool,
    tenant: Option<TenantId>,
) -> (i64, i64) {
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

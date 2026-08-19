use super::super::assets::source_artifact;
use super::super::*;

pub(crate) struct ExternalToolFixture {
    pub(crate) context: TenantContext,
    pub(crate) foreign_context: TenantContext,
    pub(crate) actor: UserId,
    pub(crate) stranger: UserId,
    pub(crate) attempt: QuestionAttemptId,
    pub(crate) binding: learning_data_access::ExternalToolBinding,
}

pub(crate) async fn external_tool_fixture<S>(store: &S) -> ExternalToolFixture
where
    S: Store + CatalogStore + CourseRosterStore,
{
    let tenant = TenantId::from_uuid(uuid(10_001));
    let foreign_tenant = TenantId::from_uuid(uuid(10_002));
    let context = TenantContext::from_authenticated_session(tenant);
    let foreign_context = TenantContext::from_authenticated_session(foreign_tenant);
    let actor = UserId::from_uuid(uuid(10_003));
    let stranger = UserId::from_uuid(uuid(10_004));
    let instructor = UserId::from_uuid(uuid(10_015));
    let workspace = WorkspaceId::from_uuid(uuid(10_005));
    let problem = ProblemId::from_uuid(uuid(10_006));
    let version = VersionId::from_uuid(uuid(10_007));
    let course = CourseId::from_uuid(uuid(10_008));
    let assignment = AssignmentId::from_uuid(uuid(10_009));
    let run_id = RunId::from_uuid(uuid(10_011));
    let attempt = QuestionAttemptId::from_uuid(uuid(10_012));
    let source_object = ObjectId::from_uuid(uuid(10_014));
    let reference = ProblemVersionRef { problem, version };
    let prepared_artifact = source_artifact(reference, QuestionBackend::Imathas, source_object);
    let source_sha256 = prepared_artifact.object.sha256.to_string();
    let mut question = draft_question(workspace);
    question.response = ResponseDefinition::ExternalTool {};
    question.source = DraftQuestionSource::Imathas {
        provider: "institution-imathas".to_string(),
        item_ref: "external-tool-item".to_string(),
    };
    let draft = DraftRecord {
        tenant,
        question,
        derived_from: None,
    };
    let saved_draft = store
        .upsert_draft(context, instructor, None, draft.clone())
        .await
        .expect("external-tool draft");
    store
        .publish_draft(
            context,
            instructor,
            PublishDraftCommand {
                expected_draft: draft,
                expected_revision: saved_draft.revision,
                publication: reference,
                published_source: QuestionSource::Imathas {
                    provider: "institution-imathas".to_string(),
                    item_ref: "external-tool-item".to_string(),
                    snapshot: source_object,
                    snapshot_sha256: source_sha256.clone(),
                    integration_profile: "institution-default".to_string(),
                },
                source_artifact: Some(prepared_artifact),
                qti_promotion: None,
                flat_question_promotion: None,
                publisher: instructor,
                scope: PublicationScope::Public,
                byline: reviewed_byline(),
                capabilities: BackendCapabilities::from_iter([Capability::ServerGrading]),
            },
        )
        .await
        .expect("external-tool publication");
    store
        .create_course(
            context,
            learning_data_access::CreateCourseCommand {
                course: CourseRecord {
                    id: course,
                    tenant,
                    title: "External tool course".to_string(),
                    term: question_model::CourseTerm::from_parts(
                        "2026-08-24",
                        "2026-12-18",
                        "America/Chicago",
                    )
                    .expect("explicit fixture course term"),
                },
                initial_instructor: instructor,
            },
        )
        .await
        .expect("external-tool course");
    for (user, display_name) in [(actor, "External actor"), (stranger, "External stranger")] {
        store
            .upsert_course_member(
                context,
                learning_data_access::UpsertCourseMember {
                    course,
                    user,
                    display_name: display_name.to_string(),
                    roster_contact: None,
                },
            )
            .await
            .expect("external-tool learner membership");
    }
    let mut external_policies = policies();
    external_policies.completion = CompletionRequirement::AnswerAll;
    store
        .create_untimed_assignment(
            context,
            AssignmentRecord {
                id: assignment,
                tenant,
                course_id: course,
                title: "External tool assignment".to_string(),
                audience: question_model::AssignmentAudience::CourseWide,
                items: fixed_items(vec![ProblemVersionRef { problem, version }]),
                selection_groups: Vec::new(),
                policies: external_policies,
            },
        )
        .await
        .expect("external-tool assignment");
    let run = store
        .start_or_resume_run(context, actor, assignment, run_id)
        .await
        .expect("external-tool run");
    let binding = learning_data_access::ExternalToolBinding {
        provider: "institution-imathas".to_string(),
        problem,
        version,
        seed: 761,
        source_object,
        source_sha256: source_sha256.clone(),
        integration_profile: "institution-default".to_string(),
        response_sha256: Sha256Digest::compute(
            &serde_json::to_vec(&StudentResponse::ExternalTool {}).expect("marker encoding"),
        ),
    };
    store
        .issue_or_resume_question_attempt(
            context,
            IssueQuestionAttemptCommand {
                actor,
                attempt,
                run: run.id,
                assignment_position: 0,
                problem,
                question_version: version,
                seed: binding.seed,
                presentation_capability: PresentationCapability::NotApplicable,
                presentation: None,
                presentation_snapshot: None,
                grading_envelope: None,
                flat_grading: None,
                flat_grading_capability: FlatGradingCapability::NotApplicable,
                webwork_grading: None,
                webwork_grading_capability: WebworkGradingCapability::NotApplicable,
                parameter_hash: "external-tool-parameters".to_string(),
                provenance: AttemptProvenance {
                    adapter: implementation("imathas"),
                    renderer: None,
                    generator: None,
                    source_artifact: Some(SourceArtifact {
                        object: source_object,
                        sha256: source_sha256,
                    }),
                    asset_objects: Vec::new(),
                    grading: implementation("imathas"),
                    rendered_question_sha256: "external-tool-rendered".to_string(),
                },
                webwork_replay: None,
                prefetched: None,
                predecessor_submission: None,
            },
        )
        .await
        .expect("external-tool attempt");
    ExternalToolFixture {
        context,
        foreign_context,
        actor,
        stranger,
        attempt,
        binding,
    }
}

pub(super) fn external_begin(
    fixture: &ExternalToolFixture,
    key: &str,
) -> BeginExternalToolGradeCommand {
    BeginExternalToolGradeCommand {
        actor: fixture.actor,
        attempt: fixture.attempt,
        response: StudentResponse::ExternalTool {},
        idempotency_key: SubmissionIdempotencyKey::parse(key).expect("valid external key"),
        binding: fixture.binding.clone(),
        proposed_correlation: PersistedCorrelation::new(b"opaque-broker-correlation".to_vec())
            .expect("correlation"),
        lease_millis: 30_000,
    }
}

pub(super) fn assert_external_debug_is_redacted(
    value: impl std::fmt::Debug,
    fixture: &ExternalToolFixture,
) {
    let rendered = format!("{value:?}");
    let source_object = fixture.binding.source_object.to_string();
    let response_digest = fixture.binding.response_sha256.to_string();
    for secret_or_provenance in [
        fixture.binding.provider.as_str(),
        fixture.binding.integration_profile.as_str(),
        fixture.binding.source_sha256.as_str(),
        source_object.as_str(),
        response_digest.as_str(),
        "opaque-broker-correlation",
        "points_earned",
        "points_possible",
    ] {
        assert!(
            !rendered.contains(secret_or_provenance),
            "external broker debug output must redact `{secret_or_provenance}`: {rendered}"
        );
    }
}

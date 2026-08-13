use super::assets::source_artifact;
use super::*;

pub(super) struct ExternalToolFixture {
    pub(super) context: TenantContext,
    pub(super) foreign_context: TenantContext,
    pub(super) actor: UserId,
    pub(super) stranger: UserId,
    pub(super) attempt: QuestionAttemptId,
    pub(super) binding: learning_data_access::ExternalToolBinding,
}

pub(super) async fn external_tool_fixture<S>(store: &S) -> ExternalToolFixture
where
    S: Store + CatalogStore,
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
    let enrollment = EnrollmentId::from_uuid(uuid(10_010));
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
        revises: None,
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
                capabilities: BackendCapabilities::from_iter([Capability::ServerGrading]),
            },
        )
        .await
        .expect("external-tool publication");
    store
        .upsert_course(
            context,
            CourseRecord {
                id: course,
                tenant,
                title: "External tool course".to_string(),
                members: vec![
                    CourseMembership {
                        user: instructor,
                        role: CourseMembershipRole::Instructor,
                    },
                    CourseMembership {
                        user: actor,
                        role: CourseMembershipRole::Student,
                    },
                    CourseMembership {
                        user: stranger,
                        role: CourseMembershipRole::Student,
                    },
                ],
            },
        )
        .await
        .expect("external-tool course");
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
                items: fixed_items(vec![ProblemVersionRef { problem, version }]),
                selection_groups: Vec::new(),
                policies: external_policies,
            },
        )
        .await
        .expect("external-tool assignment");
    store
        .create_enrollment(
            context,
            AssignmentEnrollment {
                id: enrollment,
                tenant,
                assignment,
                user: actor,
                student: StudentId::from_uuid(uuid(10_013)),
                first_completed_at: None,
                current_grade_run: None,
                best_grade_run: None,
            },
        )
        .await
        .expect("external-tool enrollment");
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

fn external_begin(fixture: &ExternalToolFixture, key: &str) -> BeginExternalToolGradeCommand {
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

fn assert_external_debug_is_redacted(value: impl std::fmt::Debug, fixture: &ExternalToolFixture) {
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

async fn exercise_external_tool_broker<S>(store: &S)
where
    S: Store + CatalogStore + ExternalToolBrokerStore + ExternalToolLaunchSessionStore,
{
    let fixture = external_tool_fixture(store).await;
    let mut provider_url = fixture.binding.clone();
    provider_url.provider = "https://provider.invalid/grade?token=secret".to_string();
    assert!(
        matches!(provider_url.validate(), Err(StoreError::InvalidRecord(_))),
        "provider configuration is an opaque identifier, never a URL or credential container"
    );
    let begin = external_begin(&fixture, "external-tool-submission");
    assert!(
        matches!(
            store
                .begin_or_resume_external_grade(fixture.foreign_context, begin.clone())
                .await,
            Err(StoreError::NotFound)
        ),
        "a foreign tenant cannot discover an exchange or its attempt"
    );
    let mut foreign_actor = begin.clone();
    foreign_actor.actor = fixture.stranger;
    assert!(
        matches!(
            store
                .begin_or_resume_external_grade(fixture.context, foreign_actor)
                .await,
            Err(StoreError::NotFound)
        ),
        "a different tenant member cannot claim another learner's exchange"
    );

    let first = store
        .begin_or_resume_external_grade(fixture.context, begin.clone())
        .await
        .expect("first claim");
    let ExternalToolBegin::Lease(lease) = first else {
        panic!("first broker claim must lease");
    };
    let grade_launch = store
        .create_external_tool_launch_session(
            fixture.context,
            CreateExternalToolLaunchSessionCommand {
                actor: fixture.actor,
                attempt: fixture.attempt,
                binding: fixture.binding.clone(),
                encrypted_provider_state: Some(vec![7; 64]),
                lifetime_millis: 60_000,
            },
        )
        .await
        .expect("grade launch session");
    let grade_launch_proof = ExternalToolLaunchProof {
        session_id: grade_launch.id,
        token: grade_launch.token.clone(),
    };
    let copied_launch = store
        .create_external_tool_launch_session(
            fixture.context,
            CreateExternalToolLaunchSessionCommand {
                actor: fixture.actor,
                attempt: fixture.attempt,
                binding: fixture.binding.clone(),
                encrypted_provider_state: None,
                lifetime_millis: 60_000,
            },
        )
        .await
        .expect("independent launch session for hostile proof checks");
    assert_external_debug_is_redacted(fixture.binding.clone(), &fixture);
    assert_external_debug_is_redacted(lease.clone(), &fixture);
    assert_external_debug_is_redacted(ExternalToolBegin::Lease(lease.clone()), &fixture);
    for mutated in [
        {
            let mut command = begin.clone();
            command.binding.provider = "other-provider".to_string();
            command
        },
        {
            let mut command = begin.clone();
            command.binding.problem = ProblemId::from_uuid(uuid(10_101));
            command
        },
        {
            let mut command = begin.clone();
            command.binding.version = VersionId::from_uuid(uuid(10_102));
            command
        },
        {
            let mut command = begin.clone();
            command.binding.seed += 1;
            command
        },
        {
            let mut command = begin.clone();
            command.binding.source_object = ObjectId::from_uuid(uuid(10_103));
            command
        },
        {
            let mut command = begin.clone();
            command.binding.source_sha256 = "0".repeat(64);
            command
        },
        {
            let mut command = begin.clone();
            command.binding.integration_profile = "other-profile".to_string();
            command
        },
        {
            let mut command = begin.clone();
            command.binding.response_sha256 = Sha256Digest::compute(b"mutated");
            command
        },
        {
            let mut command = begin.clone();
            command.idempotency_key =
                SubmissionIdempotencyKey::parse("other-external-key").unwrap();
            command
        },
    ] {
        assert!(matches!(
            store
                .begin_or_resume_external_grade(fixture.context, mutated)
                .await,
            Err(StoreError::Conflict)
        ));
    }
    let in_progress = store
        .begin_or_resume_external_grade(fixture.context, begin.clone())
        .await
        .expect("active external claim must be observable as in progress");
    assert_external_debug_is_redacted(&in_progress, &fixture);
    assert!(
        matches!(in_progress, ExternalToolBegin::InProgress),
        "failed claims must not alter the active exchange"
    );
    let result = AttemptResult {
        correct: true,
        points_earned: 1.0,
        points_possible: 1.0,
    };
    let recovery_before_stage = CommitVerifiedExternalToolSubmissionCommand {
        actor: fixture.actor,
        attempt: fixture.attempt,
        response: StudentResponse::ExternalTool {},
        idempotency_key: begin.idempotency_key.clone(),
        binding: fixture.binding.clone(),
        correlation: lease.correlation.clone(),
        launch_proof: grade_launch_proof.clone(),
    };
    assert!(matches!(
        store
            .commit_verified_external_tool_submission(
                fixture.context,
                recovery_before_stage.clone(),
            )
            .await,
        Err(StoreError::Conflict)
    ));
    assert!(matches!(
        store
            .stage_external_tool_verification(
                fixture.context,
                StageExternalToolVerificationCommand {
                    actor: fixture.actor,
                    attempt: fixture.attempt,
                    response: StudentResponse::Numeric { value: 1.0 },
                    idempotency_key: begin.idempotency_key.clone(),
                    binding: fixture.binding.clone(),
                    correlation: lease.correlation.clone(),
                    lease_token: lease.token.clone(),
                    result,
                },
            )
            .await,
        Err(StoreError::InvalidRecord(_))
    ));
    store
        .stage_external_tool_verification(
            fixture.context,
            StageExternalToolVerificationCommand {
                actor: fixture.actor,
                attempt: fixture.attempt,
                response: StudentResponse::ExternalTool {},
                idempotency_key: begin.idempotency_key.clone(),
                binding: fixture.binding.clone(),
                correlation: lease.correlation.clone(),
                lease_token: lease.token.clone(),
                result,
            },
        )
        .await
        .expect("current lease stages exactly one verified result");
    let verified = store
        .begin_or_resume_external_grade(fixture.context, begin.clone())
        .await
        .expect("crash recovery reads the staged result without regrading");
    let ExternalToolBegin::VerifiedPending(verified) = verified else {
        panic!("verified work must resume without another provider verification");
    };
    assert_external_debug_is_redacted(&verified, &fixture);
    assert_eq!(verified.binding, fixture.binding);
    assert_eq!(verified.correlation, lease.correlation);
    let recovery = CommitVerifiedExternalToolSubmissionCommand {
        actor: fixture.actor,
        attempt: fixture.attempt,
        response: StudentResponse::ExternalTool {},
        idempotency_key: begin.idempotency_key.clone(),
        binding: verified.binding,
        correlation: verified.correlation,
        launch_proof: grade_launch_proof.clone(),
    };
    let mut wrong_token_proof = recovery.clone();
    wrong_token_proof.launch_proof.token = copied_launch.token.clone();
    assert!(matches!(
        store
            .commit_verified_external_tool_submission(fixture.context, wrong_token_proof)
            .await,
        Err(StoreError::Conflict)
    ));
    store
        .revoke_external_tool_launch_session(
            fixture.context,
            fixture.actor,
            fixture.attempt,
            copied_launch.id,
        )
        .await
        .expect("revoke hostile copied launch");
    let mut revoked_proof = recovery.clone();
    revoked_proof.launch_proof = ExternalToolLaunchProof {
        session_id: copied_launch.id,
        token: copied_launch.token.clone(),
    };
    assert!(matches!(
        store
            .commit_verified_external_tool_submission(fixture.context, revoked_proof)
            .await,
        Err(StoreError::Conflict)
    ));
    for invalid in [
        {
            let mut command = recovery.clone();
            command.actor = fixture.stranger;
            command
        },
        {
            let mut command = recovery.clone();
            command.response = StudentResponse::Numeric { value: 1.0 };
            command
        },
        {
            let mut command = recovery.clone();
            command.idempotency_key =
                SubmissionIdempotencyKey::parse("other-recovery-key").unwrap();
            command
        },
        {
            let mut command = recovery.clone();
            command.binding.integration_profile = "other-profile".to_string();
            command
        },
        {
            let mut command = recovery.clone();
            command.correlation = PersistedCorrelation::new(b"other-correlation".to_vec()).unwrap();
            command
        },
    ] {
        assert!(matches!(
            store
                .commit_verified_external_tool_submission(fixture.context, invalid)
                .await,
            Err(StoreError::NotFound)
                | Err(StoreError::Conflict)
                | Err(StoreError::InvalidRecord(_))
        ));
    }
    assert!(matches!(
        store
            .commit_verified_external_tool_submission(fixture.foreign_context, recovery.clone())
            .await,
        Err(StoreError::NotFound)
    ));
    let (first_commit, replay_commit) = tokio::join!(
        store.commit_verified_external_tool_submission(fixture.context, recovery.clone()),
        store.commit_verified_external_tool_submission(fixture.context, recovery.clone()),
    );
    let committed = first_commit.expect("one recovery committer persists the staged result");
    assert_eq!(
        replay_commit.expect("concurrent exact recovery replays the first receipt"),
        committed
    );
    assert!(
        store
            .resolve_external_tool_launch_session(
                fixture.context,
                fixture.actor,
                fixture.attempt,
                grade_launch.id,
                &grade_launch.token,
            )
            .await
            .expect("consumed launch lookup")
            .is_none(),
        "the launch capability is consumed in the same commit as the receipt"
    );
    assert!(committed.attempt.timer.submitted_at.is_some());
    let committed_begin = store
        .begin_or_resume_external_grade(fixture.context, begin.clone())
        .await
        .expect("same key and marker replay the immutable first receipt");
    assert_external_debug_is_redacted(&committed_begin, &fixture);
    assert!(
        matches!(committed_begin, ExternalToolBegin::Committed(_)),
        "same key and marker replay the immutable first receipt"
    );
    let mut changed_key = begin.clone();
    changed_key.idempotency_key =
        SubmissionIdempotencyKey::parse("external-tool-changed-key").unwrap();
    assert!(matches!(
        store
            .begin_or_resume_external_grade(fixture.context, changed_key)
            .await,
        Err(StoreError::Conflict)
    ));

    let launch = store
        .create_external_tool_launch_session(
            fixture.context,
            CreateExternalToolLaunchSessionCommand {
                actor: fixture.actor,
                attempt: fixture.attempt,
                binding: fixture.binding.clone(),
                encrypted_provider_state: Some(vec![7; 64]),
                lifetime_millis: 60_000,
            },
        )
        .await
        .expect("launch session");
    let second_launch = store
        .create_external_tool_launch_session(
            fixture.context,
            CreateExternalToolLaunchSessionCommand {
                actor: fixture.actor,
                attempt: fixture.attempt,
                binding: fixture.binding.clone(),
                encrypted_provider_state: None,
                lifetime_millis: 60_000,
            },
        )
        .await
        .expect("independent launch session");
    for lifetime_millis in [300_001, 900_000] {
        store
            .create_external_tool_launch_session(
                fixture.context,
                CreateExternalToolLaunchSessionCommand {
                    actor: fixture.actor,
                    attempt: fixture.attempt,
                    binding: fixture.binding.clone(),
                    encrypted_provider_state: None,
                    lifetime_millis,
                },
            )
            .await
            .expect("documented launch-session lifetime must work in every Store");
    }
    assert!(matches!(
        store
            .create_external_tool_launch_session(
                fixture.context,
                CreateExternalToolLaunchSessionCommand {
                    actor: fixture.actor,
                    attempt: fixture.attempt,
                    binding: fixture.binding.clone(),
                    encrypted_provider_state: None,
                    lifetime_millis: 900_001,
                },
            )
            .await,
        Err(StoreError::InvalidRecord(_))
    ));
    assert_ne!(
        launch.id, second_launch.id,
        "launch IDs must be operating-system-random and unique"
    );
    assert!(
        !format!("{launch:?}").contains("ExternalToolLaunchToken"),
        "launch diagnostics must never print cookie material"
    );
    assert!(
        store
            .resolve_external_tool_launch_session(
                fixture.context,
                fixture.actor,
                fixture.attempt,
                launch.id,
                &launch.token
            )
            .await
            .expect("owner resolve")
            .is_some()
    );
    assert!(matches!(
        store
            .resolve_external_tool_launch_session(
                fixture.context,
                fixture.stranger,
                fixture.attempt,
                launch.id,
                &launch.token
            )
            .await,
        Err(StoreError::NotFound)
    ));
    assert!(
        matches!(
            store
                .resolve_external_tool_launch_session(
                    fixture.foreign_context,
                    fixture.actor,
                    fixture.attempt,
                    launch.id,
                    &launch.token
                )
                .await,
            Err(StoreError::NotFound),
        ),
        "a foreign tenant must not discover a launch session tied to another tenant"
    );
    store
        .revoke_external_tool_launch_session(
            fixture.context,
            fixture.actor,
            fixture.attempt,
            launch.id,
        )
        .await
        .expect("owner revoke");
    assert!(
        store
            .resolve_external_tool_launch_session(
                fixture.context,
                fixture.actor,
                fixture.attempt,
                launch.id,
                &launch.token
            )
            .await
            .expect("revoked lookup")
            .is_none()
    );
}

#[tokio::test]
async fn memory_external_tool_broker_conforms() {
    let store = MemoryStore::default();
    store
        .set_authoritative_time(ActivityTimestamp::from_unix_millis(500))
        .expect("memory clock");
    exercise_external_tool_broker(&store).await;
}

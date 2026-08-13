use super::super::*;
use super::fixtures::{assert_external_debug_is_redacted, external_begin, external_tool_fixture};

pub(super) async fn exercise_external_tool_broker<S>(store: &S)
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
    assert!(matches!(
        store
            .begin_or_resume_external_grade(fixture.context, external_begin(&fixture, "fenced"))
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
        matches!(
            store
                .claim_external_tool_activity(
                    fixture.context,
                    fixture.actor,
                    fixture.attempt,
                    grade_launch.id,
                    &grade_launch.token,
                    30_000,
                )
                .await,
            Ok(learning_data_access::ExternalToolActivityClaim::Unavailable)
        ),
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
    let activity = store
        .claim_external_tool_activity(
            fixture.context,
            fixture.actor,
            fixture.attempt,
            launch.id,
            &launch.token,
            30_000,
        )
        .await
        .expect("owner claims provider activity");
    let learning_data_access::ExternalToolActivityClaim::Lease(activity) = activity else {
        panic!("owner must acquire a fresh activity lease");
    };
    assert!(matches!(
        store
            .claim_external_tool_activity(
                fixture.context,
                fixture.actor,
                fixture.attempt,
                launch.id,
                &launch.token,
                30_000,
            )
            .await,
        Ok(learning_data_access::ExternalToolActivityClaim::InProgress)
    ));
    assert!(matches!(
        store
            .revoke_external_tool_launch_session(
                fixture.context,
                fixture.actor,
                fixture.attempt,
                launch.id,
            )
            .await,
        Err(StoreError::Conflict)
    ));
    assert!(matches!(
        store
            .claim_external_tool_activity(
                fixture.context,
                fixture.stranger,
                fixture.attempt,
                launch.id,
                &launch.token,
                30_000,
            )
            .await,
        Err(StoreError::NotFound)
    ));
    assert!(
        matches!(
            store
                .claim_external_tool_activity(
                    fixture.foreign_context,
                    fixture.actor,
                    fixture.attempt,
                    launch.id,
                    &launch.token,
                    30_000,
                )
                .await,
            Err(StoreError::NotFound),
        ),
        "a foreign tenant must not discover a launch session tied to another tenant"
    );
    store
        .release_external_tool_activity(
            fixture.context,
            fixture.actor,
            fixture.attempt,
            launch.id,
            &activity.token,
        )
        .await
        .expect("exact activity holder releases after remote I/O");
    store
        .revoke_external_tool_launch_session(
            fixture.context,
            fixture.actor,
            fixture.attempt,
            launch.id,
        )
        .await
        .expect("owner revoke");
    assert!(matches!(
        store
            .claim_external_tool_activity(
                fixture.context,
                fixture.actor,
                fixture.attempt,
                launch.id,
                &launch.token,
                30_000,
            )
            .await,
        Ok(learning_data_access::ExternalToolActivityClaim::Unavailable)
    ));
}

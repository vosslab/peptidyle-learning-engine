use super::super::*;
use super::broker::exercise_external_tool_broker;
use super::fixtures::{external_begin, external_tool_fixture};

#[tokio::test]
async fn memory_external_tool_broker_conforms() {
    let store = MemoryStore::default();
    store
        .set_authoritative_time(ActivityTimestamp::from_unix_millis(500))
        .expect("memory clock");
    exercise_external_tool_broker(&store).await;
}

#[tokio::test]
async fn finalization_fence_blocks_new_activity_but_allows_its_exact_verifier() {
    let store = MemoryStore::default();
    store
        .set_authoritative_time(ActivityTimestamp::from_unix_millis(500))
        .expect("memory clock");
    let fixture = external_tool_fixture(&store).await;
    let launch = store
        .create_external_tool_launch_session(
            fixture.context,
            CreateExternalToolLaunchSessionCommand {
                actor: fixture.actor,
                student_work_binding: fixture.student_work_binding(),
                attempt: fixture.attempt,
                binding: fixture.binding.clone(),
                encrypted_provider_state: Some(vec![7; 64]),
                lifetime_millis: 60_000,
            },
        )
        .await
        .expect("launch session");
    let begin = external_begin(&fixture, "finalization-fence");
    let ordinary_activity = store
        .claim_external_tool_activity(
            fixture.context,
            fixture.actor,
            fixture.student_work_binding(),
            fixture.attempt,
            launch.id,
            &launch.token,
            30_000,
        )
        .await
        .expect("ordinary provider activity starts before submission");
    let learning_data_access::ExternalToolActivityClaim::Lease(ordinary_activity) =
        ordinary_activity
    else {
        panic!("ordinary provider activity must hold its lease");
    };
    assert!(matches!(
        store
            .begin_or_resume_external_grade(fixture.context, begin.clone())
            .await,
        Ok(ExternalToolBegin::InProgress)
    ));
    store
        .release_external_tool_activity(
            fixture.context,
            fixture.actor,
            fixture.student_work_binding(),
            fixture.attempt,
            launch.id,
            &ordinary_activity.token,
        )
        .await
        .expect("submission can retry after the earlier activity releases");
    let ExternalToolBegin::Lease(verification_lease) = store
        .begin_or_resume_external_grade(fixture.context, begin.clone())
        .await
        .expect("submission atomically establishes finalization fence")
    else {
        panic!("fresh submission must acquire its verification lease");
    };
    assert!(matches!(
        store
            .claim_external_tool_activity(
                fixture.context,
                fixture.actor,
                fixture.student_work_binding(),
                fixture.attempt,
                launch.id,
                &launch.token,
                30_000,
            )
            .await,
        Ok(learning_data_access::ExternalToolActivityClaim::InProgress)
    ));
    let claim = store
        .claim_external_tool_finalization_activity(
            fixture.context,
            learning_data_access::ClaimExternalToolFinalizationActivityCommand {
                actor: fixture.actor,
                student_work_binding: fixture.student_work_binding(),
                attempt: fixture.attempt,
                id: launch.id,
                token: launch.token.clone(),
                verification_lease: verification_lease.token.clone(),
                lease_millis: 30_000,
            },
        )
        .await
        .expect("only the current finalization lease may perform verification I/O");
    let learning_data_access::ExternalToolActivityClaim::Lease(activity) = claim else {
        panic!("the current finalization lease must claim its provider activity");
    };
    store
        .release_external_tool_activity(
            fixture.context,
            fixture.actor,
            fixture.student_work_binding(),
            fixture.attempt,
            launch.id,
            &activity.token,
        )
        .await
        .expect("verifier releases its short provider lease");
    store
        .stage_external_tool_verification(
            fixture.context,
            StageExternalToolVerificationCommand {
                actor: fixture.actor,
                student_work_binding: fixture.student_work_binding(),
                attempt: fixture.attempt,
                response: StudentResponse::ExternalTool {},
                idempotency_key: begin.idempotency_key,
                binding: fixture.binding.clone(),
                correlation: verification_lease.correlation,
                lease_token: verification_lease.token,
                result: AttemptResult {
                    correct: true,
                    points_earned: 1.0,
                    points_possible: 1.0,
                },
            },
        )
        .await
        .expect("verified result is durable before final commit");
    assert!(matches!(
        store
            .claim_external_tool_activity(
                fixture.context,
                fixture.actor,
                fixture.student_work_binding(),
                fixture.attempt,
                launch.id,
                &launch.token,
                30_000,
            )
            .await,
        Ok(learning_data_access::ExternalToolActivityClaim::InProgress)
    ));
}

#[tokio::test]
async fn indeterminate_activity_fence_blocks_reclaim_relaunch_and_submission_retry() {
    let store = MemoryStore::default();
    let fixture = external_tool_fixture(&store).await;
    let launch = store
        .create_external_tool_launch_session(
            fixture.context,
            CreateExternalToolLaunchSessionCommand {
                actor: fixture.actor,
                student_work_binding: fixture.student_work_binding(),
                attempt: fixture.attempt,
                binding: fixture.binding.clone(),
                encrypted_provider_state: Some(vec![7; 64]),
                lifetime_millis: 60_000,
            },
        )
        .await
        .expect("launch");
    let claim = store
        .claim_external_tool_activity(
            fixture.context,
            fixture.actor,
            fixture.student_work_binding(),
            fixture.attempt,
            launch.id,
            &launch.token,
            30_000,
        )
        .await
        .expect("claim");
    let learning_data_access::ExternalToolActivityClaim::Lease(claim) = claim else {
        panic!("fresh activity must claim");
    };
    // A paused replica cannot mark an operation after its lease expired and a
    // replacement holder reclaimed it.
    store
        .set_authoritative_time(ActivityTimestamp::from_unix_millis(30_001))
        .expect("expire first lease");
    assert!(matches!(
        store
            .claim_external_tool_activity(
                fixture.context,
                fixture.actor,
                fixture.student_work_binding(),
                fixture.attempt,
                launch.id,
                &launch.token,
                30_000,
            )
            .await,
        Ok(learning_data_access::ExternalToolActivityClaim::Lease(_))
    ));
    assert!(matches!(
        store
            .begin_external_tool_activity_dispatch(
                fixture.context,
                fixture.actor,
                fixture.student_work_binding(),
                fixture.attempt,
                launch.id,
                &claim.token,
            )
            .await,
        Err(StoreError::Conflict)
    ));
    // Reset to a fresh fixture operation for the pre-dispatch fence case.
    let launch = store
        .create_external_tool_launch_session(
            fixture.context,
            CreateExternalToolLaunchSessionCommand {
                actor: fixture.actor,
                student_work_binding: fixture.student_work_binding(),
                attempt: fixture.attempt,
                binding: fixture.binding.clone(),
                encrypted_provider_state: Some(vec![9; 64]),
                lifetime_millis: 60_000,
            },
        )
        .await
        .expect("replacement launch");
    let claim = store
        .claim_and_begin_external_tool_activity_dispatch(
            fixture.context,
            fixture.actor,
            fixture.student_work_binding(),
            fixture.attempt,
            launch.id,
            &launch.token,
            30_000,
        )
        .await
        .expect("provider POST is atomically fenced before dispatch");
    let learning_data_access::ExternalToolActivityClaim::Lease(claim) = claim else {
        panic!("fresh replacement activity must claim");
    };
    assert!(matches!(
        store
            .claim_external_tool_activity(
                fixture.context,
                fixture.actor,
                fixture.student_work_binding(),
                fixture.attempt,
                launch.id,
                &launch.token,
                30_000,
            )
            .await,
        Ok(learning_data_access::ExternalToolActivityClaim::Unavailable)
    ));
    assert!(matches!(
        store
            .create_external_tool_launch_session(
                fixture.context,
                CreateExternalToolLaunchSessionCommand {
                    actor: fixture.actor,
                    student_work_binding: fixture.student_work_binding(),
                    attempt: fixture.attempt,
                    binding: fixture.binding.clone(),
                    encrypted_provider_state: Some(vec![8; 64]),
                    lifetime_millis: 60_000,
                },
            )
            .await,
        Err(StoreError::Conflict)
    ));
    store
        .complete_external_tool_activity_dispatch(
            fixture.context,
            fixture.actor,
            fixture.student_work_binding(),
            fixture.attempt,
            &claim.token,
        )
        .await
        .expect("only a received response clears its exact operation fence");
    assert!(matches!(
        store
            .claim_external_tool_activity(
                fixture.context,
                fixture.actor,
                fixture.student_work_binding(),
                fixture.attempt,
                launch.id,
                &launch.token,
                30_000,
            )
            .await,
        Ok(learning_data_access::ExternalToolActivityClaim::InProgress)
    ));
}

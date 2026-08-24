//! Connected four-item progression acceptance over a normal Store-created assignment.

use learning_data_access::postgres::PostgresGraderStore;
use learning_data_access::{
    ClaimRehearsalSubmissionRouteCommand, CompleteRehearsalDeliveryRouteCommand,
    RehearsalDeliveryClaimResult, RehearsalDeliveryDispatchResult, RehearsalIdempotencyKey,
    RehearsalIssuedExecutionArtifactV1, RehearsalRouteIdentity, RehearsalRouteMutationStore,
    RehearsalSubmissionClaimResult, SealedRehearsalDeliveryExecution,
    SealedRehearsalDeliveryExecutionStore, SealedRehearsalDeliveryIssuePreparation,
    SealedRehearsalDeliveryIssueWork, SealedRehearsalSubmissionExecutionPreparation,
    SealedRehearsalSubmissionExecutionStore, StoreError,
};
use objects::Sha256Digest;
use question_model::presentation::build_presentation_v1;
use question_model::{
    AttemptProvenance, AttemptResult, DisclosedFeedback, ImplementationVersion,
    PresentationBindingV1, PresentationDigestTokenV1, QuestionEnvelope,
    RehearsalBackendReceiptReference, RehearsalPrivateGradingResult, StudentResponse,
    generation::Seed,
};

use super::canonical_store::{StartedFixture, started_fixture};
use super::post_start::{delivery, route};

fn grade() -> RehearsalPrivateGradingResult {
    RehearsalPrivateGradingResult::Graded {
        result: AttemptResult {
            correct: true,
            points_earned: 1.0,
            points_possible: 1.0,
        },
        feedback: DisclosedFeedback::empty(),
        backend_receipt_reference: RehearsalBackendReceiptReference::new(
            "native:postgres-progression".into(),
        )
        .expect("receipt"),
    }
}

pub(super) async fn grader() -> PostgresGraderStore {
    let url = std::env::var("PLE_TEST_GRADER_DATABASE_URL")
        .expect("PLE_TEST_GRADER_DATABASE_URL names the disposable grader connection");
    PostgresGraderStore::connect_local_development(&url)
        .await
        .expect("dedicated grader connection")
}

pub(super) fn issued_artifact(
    work: &SealedRehearsalDeliveryIssueWork,
    provenance_marker: &str,
) -> RehearsalIssuedExecutionArtifactV1 {
    let question = work.issued_snapshot().question();
    let envelope = QuestionEnvelope {
        version: question.version,
        seed: Seed::new(work.descriptor().deterministic_seed()),
        title: question.metadata.title.clone(),
        prompt: question.prompt.clone(),
        response: question.response.clone(),
    };
    let rendered_question_sha256 =
        Sha256Digest::compute(&serde_json::to_vec(&envelope).expect("envelope bytes")).to_string();
    let presentation = build_presentation_v1(&envelope, &[]).expect("native presentation");
    RehearsalIssuedExecutionArtifactV1::from_issue_work(
        work,
        envelope,
        work.descriptor().frozen_content_digest().to_hex(),
        AttemptProvenance {
            adapter: ImplementationVersion {
                id: "native-adapter".into(),
                version: provenance_marker.into(),
            },
            renderer: None,
            generator: None,
            source_artifact: None,
            asset_objects: Vec::new(),
            grading: ImplementationVersion {
                id: "generic-grader".into(),
                version: "1".into(),
            },
            rendered_question_sha256,
        },
        PresentationBindingV1::new(
            presentation.envelope.presentation_nonce,
            presentation.digest,
        ),
        learning_data_access::ReceiptPresentationSnapshot {
            envelope: presentation.envelope,
            asset_bindings: presentation.asset_bindings,
        },
    )
    .expect("valid sealed native issued artifact")
}

fn active_screen_for_work(
    work: &SealedRehearsalDeliveryIssueWork,
) -> question_model::RehearsalActiveScreenV1 {
    let question = work.issued_snapshot().question();
    let envelope = QuestionEnvelope {
        version: question.version,
        seed: Seed::new(work.descriptor().deterministic_seed()),
        title: question.metadata.title.clone(),
        prompt: question.prompt.clone(),
        response: question.response.clone(),
    };
    let presentation = build_presentation_v1(&envelope, &[]).expect("native presentation");
    question_model::rehearsal_active_screen_from_issued_presentation_v1(&presentation.envelope)
        .expect("answer-free native screen")
}

pub(super) async fn commit_or_resume_issued_execution(
    fixture: &StartedFixture,
    grader: &PostgresGraderStore,
    dispatched: &learning_data_access::DispatchedRehearsalDelivery,
) -> SealedRehearsalDeliveryExecution {
    match grader
        .prepare_or_resume_issued_execution(fixture.context, dispatched)
        .await
        .expect("sealed issue preparation")
    {
        SealedRehearsalDeliveryIssuePreparation::IssueWork(work) => {
            let artifact = issued_artifact(&work, "1");
            grader
                .commit_issued_execution(fixture.context, *work, artifact)
                .await
                .expect("commit exact native issued artifact")
        }
        SealedRehearsalDeliveryIssuePreparation::ExistingArtifact(execution) => *execution,
    }
}

pub(super) fn submission(
    route: RehearsalRouteIdentity,
    key: &str,
    response: StudentResponse,
    presentation_digest: &PresentationDigestTokenV1,
) -> ClaimRehearsalSubmissionRouteCommand {
    ClaimRehearsalSubmissionRouteCommand {
        route,
        response,
        presentation_digest: presentation_digest.clone(),
        idempotency_key: RehearsalIdempotencyKey::new(key.to_string()).expect("submission key"),
    }
}

pub(super) async fn accepted_count(fixture: &StartedFixture) -> i64 {
    sqlx::query_scalar(
        "SELECT count(*) FROM rehearsal_accepted_attempt_integrity WHERE tenant_id=$1 AND rehearsal_run_id=$2",
    )
    .bind(fixture.tenant.as_uuid())
    .bind(fixture.run_id)
    .fetch_one(&fixture.pool)
    .await
    .expect("accepted integrity count")
}

async fn journal_counts(fixture: &StartedFixture) -> (i64, i64, i64, i64, i64) {
    sqlx::query_as(
        "SELECT
            (SELECT count(*) FROM rehearsal_delivery_operation_event
             WHERE tenant_id=$1 AND rehearsal_run_id=$2),
            (SELECT count(*) FROM rehearsal_submission_claim_event
             WHERE tenant_id=$1 AND rehearsal_run_id=$2),
            (SELECT count(*) FROM rehearsal_evidence
             WHERE tenant_id=$1 AND rehearsal_run_id=$2),
            (SELECT count(*) FROM rehearsal_accepted_attempt_integrity
             WHERE tenant_id=$1 AND rehearsal_run_id=$2),
            (SELECT count(*) FROM rehearsal_submission_receipt
             WHERE tenant_id=$1 AND rehearsal_run_id=$2)",
    )
    .bind(fixture.tenant.as_uuid())
    .bind(fixture.run_id)
    .fetch_one(&fixture.pool)
    .await
    .expect("rehearsal journal counts")
}

#[tokio::test]
#[ignore = "requires disposable PostgreSQL 17 application and grader connections"]
async fn issued_artifact_commit_recovers_byte_identically_before_screen_completion() {
    let fixture = started_fixture().await;
    let grader = grader().await;
    let identity = route(&fixture);
    let RehearsalDeliveryClaimResult::Prepared { prepared } = fixture
        .store
        .claim_rehearsal_delivery_from_route(
            fixture.context,
            delivery(identity, "artifact-crash", 0xD1),
        )
        .await
        .expect("delivery preparation")
    else {
        panic!("fresh delivery is prepared");
    };
    let RehearsalDeliveryDispatchResult::Dispatched { dispatched } = fixture
        .store
        .mark_rehearsal_delivery_dispatched_from_route(fixture.context, identity, prepared)
        .await
        .expect("delivery dispatch")
    else {
        panic!("delivery is dispatchable");
    };
    let SealedRehearsalDeliveryIssuePreparation::IssueWork(work) = grader
        .prepare_or_resume_issued_execution(fixture.context, &dispatched)
        .await
        .expect("first sealed issue work")
    else {
        panic!("first sealed preparation is work");
    };
    let artifact = issued_artifact(&work, "1");
    let committed = grader
        .commit_issued_execution(fixture.context, *work, artifact)
        .await
        .expect("artifact commit before screen receipt");
    let SealedRehearsalDeliveryIssuePreparation::ExistingArtifact(recovered) = grader
        .prepare_or_resume_issued_execution(fixture.context, &dispatched)
        .await
        .expect("crash recovery sealed read")
    else {
        panic!("committed artifact resumes without reissuing");
    };
    assert_eq!(
        committed.active_screen().expect("committed active screen"),
        recovered.active_screen().expect("recovered active screen"),
        "crash recovery returns the exact canonical issued presentation"
    );
    fixture
        .store
        .complete_rehearsal_delivery_from_route(
            fixture.context,
            CompleteRehearsalDeliveryRouteCommand {
                route: identity,
                dispatched,
                screen: recovered.active_screen().expect("canonical screen receipt"),
            },
        )
        .await
        .expect("only the committed artifact can complete the screen receipt");
}

#[tokio::test]
#[ignore = "requires disposable PostgreSQL 17 application and grader connections"]
async fn screen_completion_without_issued_artifact_is_atomic_and_leaves_no_receipt() {
    let fixture = started_fixture().await;
    let grader = grader().await;
    let identity = route(&fixture);
    let RehearsalDeliveryClaimResult::Prepared { prepared } = fixture
        .store
        .claim_rehearsal_delivery_from_route(
            fixture.context,
            delivery(identity, "artifact-required-screen", 0xD2),
        )
        .await
        .expect("delivery preparation")
    else {
        panic!("fresh delivery is prepared");
    };
    let RehearsalDeliveryDispatchResult::Dispatched { dispatched } = fixture
        .store
        .mark_rehearsal_delivery_dispatched_from_route(fixture.context, identity, prepared)
        .await
        .expect("delivery dispatch")
    else {
        panic!("delivery is dispatchable");
    };
    let SealedRehearsalDeliveryIssuePreparation::IssueWork(work) = grader
        .prepare_or_resume_issued_execution(fixture.context, &dispatched)
        .await
        .expect("sealed issue preparation")
    else {
        panic!("fresh delivery must have uncommitted issue work");
    };
    let result = fixture
        .store
        .complete_rehearsal_delivery_from_route(
            fixture.context,
            CompleteRehearsalDeliveryRouteCommand {
                route: identity,
                dispatched,
                screen: active_screen_for_work(&work),
            },
        )
        .await;
    assert!(
        result.is_err(),
        "the artifact-required trigger rejects screen completion"
    );
    let (receipts, phases): (i64, i64) = sqlx::query_as(
        "SELECT
            (SELECT count(*) FROM rehearsal_delivery_receipt
              WHERE tenant_id=$1 AND rehearsal_run_id=$2),
            (SELECT count(*) FROM rehearsal_delivery_operation_event
              WHERE tenant_id=$1 AND rehearsal_run_id=$2 AND phase='completed')",
    )
    .bind(fixture.tenant.as_uuid())
    .bind(fixture.run_id)
    .fetch_one(&fixture.pool)
    .await
    .expect("artifact rejection leaves a queryable journal");
    assert_eq!(
        receipts, 0,
        "failed screen completion leaves no receipt row"
    );
    assert_eq!(
        phases, 0,
        "failed screen completion leaves no completed event"
    );
}

#[tokio::test]
#[ignore = "requires disposable PostgreSQL 17 application and grader connections"]
async fn route_submission_before_screen_or_artifact_is_rejected_without_claim_residue() {
    let fixture = started_fixture().await;
    let identity = route(&fixture);
    let RehearsalDeliveryClaimResult::Prepared { prepared } = fixture
        .store
        .claim_rehearsal_delivery_from_route(
            fixture.context,
            delivery(identity, "artifact-required-submission", 0xD3),
        )
        .await
        .expect("delivery preparation")
    else {
        panic!("fresh delivery is prepared");
    };
    let RehearsalDeliveryDispatchResult::Dispatched { dispatched: _ } = fixture
        .store
        .mark_rehearsal_delivery_dispatched_from_route(fixture.context, identity, prepared)
        .await
        .expect("delivery dispatch")
    else {
        panic!("delivery is dispatchable");
    };
    let before: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM rehearsal_submission_claim_root
          WHERE tenant_id=$1 AND rehearsal_run_id=$2",
    )
    .bind(fixture.tenant.as_uuid())
    .bind(fixture.run_id)
    .fetch_one(&fixture.pool)
    .await
    .expect("claim count before route admission");
    let digest =
        PresentationDigestTokenV1::parse("pd1_AAAAAAAAAAAAAAAAAAAAAA").expect("digest token");
    let result = fixture
        .store
        .claim_rehearsal_submission_from_route(
            fixture.context,
            submission(
                identity,
                "artifact-required-submission",
                StudentResponse::Numeric { value: 3.0 },
                &digest,
            ),
        )
        .await;
    assert!(
        result.is_err(),
        "submission cannot be claimed before an issued screen"
    );
    let after: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM rehearsal_submission_claim_root
          WHERE tenant_id=$1 AND rehearsal_run_id=$2",
    )
    .bind(fixture.tenant.as_uuid())
    .bind(fixture.run_id)
    .fetch_one(&fixture.pool)
    .await
    .expect("claim count after route admission");
    assert_eq!(
        after, before,
        "failed pre-screen submission leaves no claim residue"
    );
}

#[tokio::test]
#[ignore = "requires disposable PostgreSQL 17 application and grader connections"]
async fn sealed_artifact_commit_replays_exactly_and_rejects_divergent_bytes() {
    let fixture = started_fixture().await;
    let grader = grader().await;
    let identity = route(&fixture);
    let RehearsalDeliveryClaimResult::Prepared { prepared } = fixture
        .store
        .claim_rehearsal_delivery_from_route(
            fixture.context,
            delivery(identity, "artifact-replay", 0xD4),
        )
        .await
        .expect("delivery preparation")
    else {
        panic!("fresh delivery is prepared");
    };
    let RehearsalDeliveryDispatchResult::Dispatched { dispatched } = fixture
        .store
        .mark_rehearsal_delivery_dispatched_from_route(fixture.context, identity, prepared)
        .await
        .expect("delivery dispatch")
    else {
        panic!("delivery is dispatchable");
    };
    let SealedRehearsalDeliveryIssuePreparation::IssueWork(work) = grader
        .prepare_or_resume_issued_execution(fixture.context, &dispatched)
        .await
        .expect("sealed issue preparation")
    else {
        panic!("fresh delivery must have issue work");
    };
    let artifact = issued_artifact(&work, "replay");
    grader
        .commit_issued_execution(fixture.context, *work, artifact)
        .await
        .expect("first artifact commit");

    let grader_url = std::env::var("PLE_TEST_GRADER_DATABASE_URL").expect("grader database URL");
    let pool = learning_data_access::postgres::lazy_pool(&grader_url).expect("grader pool");
    let operation: uuid::Uuid = sqlx::query_scalar(
        "SELECT operation_id FROM rehearsal_delivery_operation_generation
          WHERE tenant_id=$1 AND rehearsal_run_id=$2
          LIMIT 1",
    )
    .bind(fixture.tenant.as_uuid())
    .bind(fixture.run_id)
    .fetch_one(&fixture.pool)
    .await
    .expect("operation id");
    let mut grader_transaction = pool.begin().await.expect("grader transaction");
    sqlx::query("SELECT set_config('ple.tenant_id', $1, true)")
        .bind(fixture.tenant.as_uuid().to_string())
        .execute(&mut *grader_transaction)
        .await
        .expect("grader tenant context");
    let (bytes, sha): (Vec<u8>, Vec<u8>) = sqlx::query_as(
        "SELECT artifact_bytes, artifact_sha256
           FROM public.ple_prepare_or_resume_rehearsal_issued_execution($1,$2)",
    )
    .bind(fixture.tenant.as_uuid())
    .bind(operation)
    .fetch_one(&mut *grader_transaction)
    .await
    .expect("sealed artifact bytes");
    let replay: String = sqlx::query_scalar(
        "SELECT public.ple_commit_sealed_rehearsal_issued_execution($1,$2,$3,$4)",
    )
    .bind(fixture.tenant.as_uuid())
    .bind(operation)
    .bind(&bytes)
    .bind(&sha)
    .fetch_one(&mut *grader_transaction)
    .await
    .expect("exact artifact replay");
    assert_eq!(replay, "replay");
    let mut divergent = bytes.clone();
    *divergent.last_mut().expect("artifact bytes") ^= 1;
    let divergent_sha = Sha256Digest::compute(&divergent);
    let conflict: String = sqlx::query_scalar(
        "SELECT public.ple_commit_sealed_rehearsal_issued_execution($1,$2,$3,$4)",
    )
    .bind(fixture.tenant.as_uuid())
    .bind(operation)
    .bind(&divergent)
    .bind(divergent_sha.as_bytes())
    .fetch_one(&mut *grader_transaction)
    .await
    .expect("divergent artifact conflict");
    assert_eq!(conflict, "conflict");
    let persisted: (Vec<u8>, Vec<u8>) = sqlx::query_as(
        "SELECT artifact_bytes, artifact_sha256
           FROM public.ple_prepare_or_resume_rehearsal_issued_execution($1,$2)",
    )
    .bind(fixture.tenant.as_uuid())
    .bind(operation)
    .fetch_one(&mut *grader_transaction)
    .await
    .expect("persisted artifact remains unchanged");
    assert_eq!(persisted, (bytes, sha));
    grader_transaction
        .commit()
        .await
        .expect("grader transaction commit");
}

async fn corrupt_accepted_snapshot(fixture: &StartedFixture) {
    let mut transaction = fixture
        .pool
        .begin()
        .await
        .expect("snapshot fault transaction");
    sqlx::query(
        "ALTER TABLE public.rehearsal_frozen_source_snapshot
         DISABLE TRIGGER rehearsal_source_snapshot_append_only",
    )
    .execute(&mut *transaction)
    .await
    .expect("disable disposable snapshot append-only trigger");
    let updated = sqlx::query(
        "UPDATE public.rehearsal_frozen_source_snapshot
            SET issued_snapshot_bytes = issued_snapshot_bytes || decode('00', 'hex'),
                issued_snapshot_sha256 = digest(
                    issued_snapshot_bytes || decode('00', 'hex'), 'sha256')
          WHERE tenant_id=$1 AND rehearsal_run_id=$2 AND ordinal=3",
    )
    .bind(fixture.tenant.as_uuid())
    .bind(fixture.run_id)
    .execute(&mut *transaction)
    .await
    .expect("corrupt disposable accepted snapshot");
    assert_eq!(
        updated.rows_affected(),
        1,
        "one final snapshot is corrupted"
    );
    sqlx::query(
        "ALTER TABLE public.rehearsal_frozen_source_snapshot
         ENABLE TRIGGER rehearsal_source_snapshot_append_only",
    )
    .execute(&mut *transaction)
    .await
    .expect("restore snapshot append-only trigger");
    transaction.commit().await.expect("commit snapshot fault");
}

async fn corrupt_submission_receipt(fixture: &StartedFixture, key: &str) {
    let mut transaction = fixture
        .pool
        .begin()
        .await
        .expect("receipt fault transaction");
    sqlx::query(
        "ALTER TABLE public.rehearsal_submission_receipt
         DISABLE TRIGGER rehearsal_receipt_append_only",
    )
    .execute(&mut *transaction)
    .await
    .expect("disable disposable receipt append-only trigger");
    let updated = sqlx::query(
        "UPDATE public.rehearsal_submission_receipt
            SET receipt_digest = decode(repeat('00', 32), 'hex')
          WHERE tenant_id=$1 AND rehearsal_run_id=$2
            AND claim_id=(SELECT claim_id
                            FROM rehearsal_submission_claim_root
                           WHERE tenant_id=$1 AND rehearsal_run_id=$2
                             AND idempotency_key=$3)",
    )
    .bind(fixture.tenant.as_uuid())
    .bind(fixture.run_id)
    .bind(key)
    .execute(&mut *transaction)
    .await
    .expect("corrupt disposable submission receipt");
    assert_eq!(
        updated.rows_affected(),
        1,
        "one persisted receipt is corrupted"
    );
    sqlx::query(
        "ALTER TABLE public.rehearsal_submission_receipt
         ENABLE TRIGGER rehearsal_receipt_append_only",
    )
    .execute(&mut *transaction)
    .await
    .expect("restore receipt append-only trigger");
    transaction.commit().await.expect("commit receipt fault");
}

async fn assert_append_only_triggers_enabled(fixture: &StartedFixture) {
    for (table, trigger) in [
        (
            "public.rehearsal_frozen_source_snapshot",
            "rehearsal_source_snapshot_append_only",
        ),
        (
            "public.rehearsal_submission_receipt",
            "rehearsal_receipt_append_only",
        ),
    ] {
        let enabled: bool = sqlx::query_scalar(
            "SELECT tgenabled = 'O'
               FROM pg_trigger
              WHERE tgrelid=$1::regclass AND tgname=$2 AND NOT tgisinternal",
        )
        .bind(table)
        .bind(trigger)
        .fetch_one(&fixture.pool)
        .await
        .expect("append-only trigger status");
        assert!(enabled, "{trigger} remains enabled on {table}");
    }
}

#[tokio::test]
#[ignore = "requires disposable PostgreSQL 17 application and grader connections"]
async fn four_frozen_items_advance_only_after_route_bound_accepted_submissions() {
    let fixture = started_fixture().await;
    let grader = grader().await;
    let frozen: Vec<i32> = sqlx::query_scalar(
        "SELECT ordinal FROM rehearsal_frozen_source_snapshot WHERE tenant_id=$1 AND rehearsal_run_id=$2 ORDER BY ordinal",
    )
    .bind(fixture.tenant.as_uuid())
    .bind(fixture.run_id)
    .fetch_all(&fixture.pool)
    .await
    .expect("frozen ordinals");
    assert_eq!(frozen, vec![0, 1, 2, 3]);

    let mut final_receipt = None;
    for ordinal in 0..4_i32 {
        let identity = route(&fixture);
        let claim = fixture
            .store
            .claim_rehearsal_delivery_from_route(
                fixture.context,
                delivery(identity, &format!("continue-{ordinal}"), ordinal as u8 + 1),
            )
            .await
            .expect("route delivery claim");
        let RehearsalDeliveryClaimResult::Prepared { prepared } = claim else {
            panic!("each accepted ordinal opens exactly one prepared delivery");
        };
        let expected_problem = prepared.descriptor().problem();
        let dispatched = fixture
            .store
            .mark_rehearsal_delivery_dispatched_from_route(fixture.context, identity, prepared)
            .await
            .expect("route dispatch");
        let RehearsalDeliveryDispatchResult::Dispatched { dispatched } = dispatched else {
            panic!("ordinary progression dispatches before its run limit");
        };
        let sealed = commit_or_resume_issued_execution(&fixture, &grader, &dispatched).await;
        assert_eq!(
            sealed.issued_snapshot().question().problem,
            expected_problem.problem
        );
        assert_eq!(
            sealed.issued_snapshot().question().version,
            expected_problem.version
        );
        let active = sealed.active_screen().expect("issued artifact screen");
        fixture
            .store
            .complete_rehearsal_delivery_from_route(
                fixture.context,
                CompleteRehearsalDeliveryRouteCommand {
                    route: identity,
                    dispatched,
                    screen: active.clone(),
                },
            )
            .await
            .expect("answer-free screen completion");
        let key = format!("submit-{ordinal}");
        let presentation_digest = active.presentation_digest;
        let response = StudentResponse::Numeric { value: 3.0 };
        let submission = fixture
            .store
            .claim_rehearsal_submission_from_route(
                fixture.context,
                submission(identity, &key, response.clone(), &presentation_digest),
            )
            .await
            .expect("route derives the issued ordinal for submission");
        let RehearsalSubmissionClaimResult::Claimed(claimed) = submission else {
            panic!("one route-bound submission claim is created");
        };
        fixture
            .store
            .mark_rehearsal_submission_dispatched_from_route(
                fixture.context,
                identity,
                claimed.handle,
            )
            .await
            .expect("grading dispatch");
        let SealedRehearsalSubmissionExecutionPreparation::Work(work) = grader
            .prepare_or_resume_sealed_rehearsal_submission_execution(
                fixture.context,
                identity,
                RehearsalIdempotencyKey::new(key.clone()).expect("submission key"),
            )
            .await
            .expect("sealed grading preparation")
        else {
            panic!("dispatched submission must provide sealed grading work");
        };
        let (_grading, completion) = work.into_grading_and_completion();
        let receipt = grader
            .complete_sealed_rehearsal_submission_execution(fixture.context, completion, grade())
            .await
            .expect("deterministic private grading completion");
        if ordinal == 3 {
            final_receipt = Some((key, response, presentation_digest, receipt));
        }
        assert_eq!(accepted_count(&fixture).await, i64::from(ordinal + 1));
    }
    let lifecycle: String = sqlx::query_scalar(
        "SELECT lifecycle FROM rehearsal_run WHERE tenant_id=$1 AND rehearsal_run_id=$2",
    )
    .bind(fixture.tenant.as_uuid())
    .bind(fixture.run_id)
    .fetch_one(&fixture.pool)
    .await
    .expect("final lifecycle");
    assert_eq!(lifecycle, "completed");

    let persisted_reference: i64 = sqlx::query_scalar(
        "SELECT rehearsal_reference FROM rehearsal_run
          WHERE tenant_id=$1 AND rehearsal_run_id=$2",
    )
    .bind(fixture.tenant.as_uuid())
    .bind(fixture.run_id)
    .fetch_one(&fixture.pool)
    .await
    .expect("persisted rehearsal reference");
    assert_eq!(
        persisted_reference,
        i64::from(fixture.rehearsal.number()),
        "fixture reference identifies the completed run without a global sequence assumption"
    );

    let (key, response, presentation_digest, accepted_receipt) =
        final_receipt.expect("final accepted receipt");
    let before_replay = journal_counts(&fixture).await;
    let replay = fixture
        .store
        .claim_rehearsal_submission_from_route(
            fixture.context,
            submission(
                route(&fixture),
                &key,
                response.clone(),
                &presentation_digest,
            ),
        )
        .await
        .expect("exact terminal submission replay");
    let RehearsalSubmissionClaimResult::Replay(replayed_receipt) = replay else {
        panic!("exact terminal submission replays the accepted receipt");
    };
    assert!(replayed_receipt.replayed);
    assert_eq!(replayed_receipt.outcome, accepted_receipt.outcome);
    assert_eq!(
        journal_counts(&fixture).await,
        before_replay,
        "terminal replay appends no delivery, material, evidence, or grading state"
    );

    assert!(matches!(
        fixture
            .store
            .claim_rehearsal_submission_from_route(
                fixture.context,
                submission(
                    route(&fixture),
                    &key,
                    StudentResponse::Numeric { value: 4.0 },
                    &presentation_digest,
                ),
            )
            .await,
        Err(StoreError::Conflict)
    ));
    assert!(matches!(
        fixture
            .store
            .claim_rehearsal_submission_from_route(
                fixture.context,
                submission(
                    route(&fixture),
                    &key,
                    response.clone(),
                    &presentation_digest,
                ),
            )
            .await,
        Ok(RehearsalSubmissionClaimResult::Replay(_))
    ));

    // This disposable fault changes the snapshot bytes and matching checksum,
    // leaving them valid internally but inconsistent with the accepted frozen
    // material binding. Exact replay must not hydrate that material.
    corrupt_accepted_snapshot(&fixture).await;
    let replay_after_snapshot_corruption = fixture
        .store
        .claim_rehearsal_submission_from_route(
            fixture.context,
            submission(
                route(&fixture),
                &key,
                response.clone(),
                &presentation_digest,
            ),
        )
        .await
        .expect("replay bypasses corrupted frozen material");
    assert!(matches!(
        replay_after_snapshot_corruption,
        RehearsalSubmissionClaimResult::Replay(_)
    ));
    corrupt_submission_receipt(&fixture, &key).await;
    let failed_replay = fixture
        .store
        .claim_rehearsal_submission_from_route(
            fixture.context,
            submission(route(&fixture), &key, response, &presentation_digest),
        )
        .await;
    assert!(matches!(failed_replay, Err(StoreError::InvalidRecord(_))));
    assert_append_only_triggers_enabled(&fixture).await;
}

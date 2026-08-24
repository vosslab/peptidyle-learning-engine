//! Connected submission recovery and integrity evidence over issued screens.

use domain::RehearsalPreDispatchAbandonReason;
use learning_data_access::{
    CompleteRehearsalDeliveryRouteCommand, RehearsalDeliveryClaimResult,
    RehearsalDeliveryDispatchResult, RehearsalIdempotencyKey, RehearsalRouteMutationStore,
    RehearsalSubmissionClaimResult, StoreError,
};
use question_model::StudentResponse;

use super::canonical_store::started_fixture;
use super::post_start::{delivery, route};
use super::progression::{accepted_count, commit_or_resume_issued_execution, grader, submission};

#[tokio::test]
#[ignore = "requires disposable PostgreSQL 17 application and grader connections"]
async fn route_submission_pending_conflict_and_pre_dispatch_reclaim_are_closed() {
    let fixture = started_fixture().await;
    let grader = grader().await;
    let identity = route(&fixture);
    let RehearsalDeliveryClaimResult::Prepared { prepared } = fixture
        .store
        .claim_rehearsal_delivery_from_route(
            fixture.context,
            delivery(identity, "pending-delivery", 0xa1),
        )
        .await
        .expect("first delivery claim")
    else {
        panic!("first ordinal has one prepared delivery");
    };
    let dispatched = fixture
        .store
        .mark_rehearsal_delivery_dispatched_from_route(fixture.context, identity, prepared)
        .await
        .expect("route delivery dispatch");
    let RehearsalDeliveryDispatchResult::Dispatched { dispatched } = dispatched else {
        panic!("first delivery dispatches before a run limit");
    };
    let sealed = commit_or_resume_issued_execution(&fixture, &grader, &dispatched).await;
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
        .expect("issued answer-free screen");

    let response = StudentResponse::Numeric { value: 3.0 };
    let digest = active.presentation_digest;
    let RehearsalSubmissionClaimResult::Claimed(claimed) = fixture
        .store
        .claim_rehearsal_submission_from_route(
            fixture.context,
            submission(identity, "pending-submission", response.clone(), &digest),
        )
        .await
        .expect("first submission claim")
    else {
        panic!("first exact submission creates one claim");
    };
    assert!(matches!(
        fixture
            .store
            .claim_rehearsal_submission_from_route(
                fixture.context,
                submission(identity, "pending-submission", response.clone(), &digest,),
            )
            .await,
        Ok(RehearsalSubmissionClaimResult::Pending)
    ));
    assert!(matches!(
        fixture
            .store
            .claim_rehearsal_submission_from_route(
                fixture.context,
                submission(
                    identity,
                    "pending-submission",
                    StudentResponse::Numeric { value: 4.0 },
                    &digest,
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
                submission(identity, "pending-submission", response.clone(), &digest,),
            )
            .await,
        Ok(RehearsalSubmissionClaimResult::Pending)
    ));

    fixture
        .store
        .abandon_rehearsal_submission_before_dispatch_from_route(
            fixture.context,
            identity,
            claimed.handle,
            RehearsalPreDispatchAbandonReason::LocalPreparationFailed,
        )
        .await
        .expect("definite local failure is durably abandoned before grader dispatch");
    assert!(matches!(
        fixture
            .store
            .claim_rehearsal_submission_from_route(
                fixture.context,
                submission(identity, "pending-submission", response, &digest),
            )
            .await,
        Ok(RehearsalSubmissionClaimResult::Claimed(_))
    ));
    assert_eq!(
        accepted_count(&fixture).await,
        0,
        "reclaim creates neither accepted evidence nor learner work"
    );
}

#[tokio::test]
#[ignore = "requires disposable PostgreSQL 17 application and grader connections"]
async fn route_keyed_submission_dispatch_is_idempotent_and_serialized() {
    let fixture = started_fixture().await;
    let grader = grader().await;
    let identity = route(&fixture);
    let RehearsalDeliveryClaimResult::Prepared { prepared } = fixture
        .store
        .claim_rehearsal_delivery_from_route(
            fixture.context,
            delivery(identity, "dispatch-delivery", 0xE1),
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
    let sealed = commit_or_resume_issued_execution(&fixture, &grader, &dispatched).await;
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
        .expect("issued screen completion");
    let key = "dispatch-submission";
    let response = StudentResponse::Numeric { value: 3.0 };
    let RehearsalSubmissionClaimResult::Claimed(_) = fixture
        .store
        .claim_rehearsal_submission_from_route(
            fixture.context,
            submission(identity, key, response, &active.presentation_digest),
        )
        .await
        .expect("prepared submission claim")
    else {
        panic!("new submission is claimed");
    };

    let first_dispatch = fixture.store.dispatch_rehearsal_submission_from_route(
        fixture.context,
        identity,
        RehearsalIdempotencyKey::new(key.into()).expect("dispatch key"),
    );
    let second_dispatch = fixture.store.dispatch_rehearsal_submission_from_route(
        fixture.context,
        identity,
        RehearsalIdempotencyKey::new(key.into()).expect("concurrent dispatch key"),
    );
    let (first_dispatch, second_dispatch) = tokio::join!(first_dispatch, second_dispatch);
    let dispatched = first_dispatch.expect("route-keyed grading dispatch");
    let concurrently_replayed = second_dispatch.expect("concurrent dispatch replay");
    assert_eq!(
        dispatched, concurrently_replayed,
        "concurrent dispatch returns one opaque handle"
    );
    let replayed = fixture
        .store
        .dispatch_rehearsal_submission_from_route(
            fixture.context,
            identity,
            RehearsalIdempotencyKey::new(key.into()).expect("dispatch replay key"),
        )
        .await
        .expect("exact dispatch replay");
    assert_eq!(
        dispatched, replayed,
        "repeated dispatch returns one opaque handle"
    );

    let (event_count, phase): (i64, String) = sqlx::query_as(
        "SELECT count(*), max(phase) FROM rehearsal_submission_claim_event
          WHERE tenant_id=$1 AND rehearsal_run_id=$2
            AND claim_id=(SELECT claim_id FROM rehearsal_submission_claim_root
                           WHERE tenant_id=$1 AND rehearsal_run_id=$2
                             AND idempotency_key=$3)",
    )
    .bind(fixture.tenant.as_uuid())
    .bind(fixture.run_id)
    .bind(key)
    .fetch_one(&fixture.pool)
    .await
    .expect("dispatch event count");
    assert_eq!(event_count, 2, "prepared and one gradingDispatched event");
    assert_eq!(phase, "gradingDispatched");
}

#[tokio::test]
#[ignore = "requires disposable PostgreSQL 17 application and grader connections"]
async fn recomputed_binding_substitution_refuses_dispatch_without_new_effects() {
    let fixture = started_fixture().await;
    let grader = grader().await;
    let identity = route(&fixture);
    let RehearsalDeliveryClaimResult::Prepared { prepared } = fixture
        .store
        .claim_rehearsal_delivery_from_route(
            fixture.context,
            delivery(identity, "binding-delivery", 0xE2),
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
    let sealed = commit_or_resume_issued_execution(&fixture, &grader, &dispatched).await;
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
        .expect("issued screen completion");
    let key = "binding-substitution";
    let response = StudentResponse::Numeric { value: 3.0 };
    let RehearsalSubmissionClaimResult::Claimed(_) = fixture
        .store
        .claim_rehearsal_submission_from_route(
            fixture.context,
            submission(identity, key, response, &active.presentation_digest),
        )
        .await
        .expect("prepared submission claim")
    else {
        panic!("new submission is claimed");
    };

    let before: (i64, i64, i64) = sqlx::query_as(
        "SELECT
            (SELECT count(*) FROM rehearsal_submission_claim_event
              WHERE tenant_id=$1 AND rehearsal_run_id=$2),
            (SELECT count(*) FROM rehearsal_evidence
              WHERE tenant_id=$1 AND rehearsal_run_id=$2),
            (SELECT count(*) FROM rehearsal_submission_receipt
              WHERE tenant_id=$1 AND rehearsal_run_id=$2)",
    )
    .bind(fixture.tenant.as_uuid())
    .bind(fixture.run_id)
    .fetch_one(&fixture.pool)
    .await
    .expect("pre-substitution counts");
    sqlx::query(
        "ALTER TABLE public.rehearsal_submission_claim_delivery_binding
         DISABLE TRIGGER rehearsal_claim_delivery_binding_append_only",
    )
    .execute(&fixture.pool)
    .await
    .expect("disable disposable binding trigger");
    let changed = sqlx::query(
        "UPDATE public.rehearsal_submission_claim_delivery_binding binding
            SET issued_screen_digest=decode(repeat('11',32),'hex'),
                binding_digest=public.ple_rehearsal_claim_delivery_binding_digest(
                    binding.tenant_id, binding.rehearsal_run_id, binding.claim_id,
                    claim.request_fingerprint, claim.attempt_id, binding.delivery_root_id,
                    binding.delivery_generation, binding.delivery_operation_id,
                    decode(repeat('11',32),'hex'))
           FROM public.rehearsal_submission_claim_root claim
          WHERE claim.tenant_id=binding.tenant_id
            AND claim.rehearsal_run_id=binding.rehearsal_run_id
            AND claim.claim_id=binding.claim_id
            AND binding.tenant_id=$1 AND binding.rehearsal_run_id=$2
            AND claim.idempotency_key=$3",
    )
    .bind(fixture.tenant.as_uuid())
    .bind(fixture.run_id)
    .bind(key)
    .execute(&fixture.pool)
    .await
    .expect("recompute substituted binding digest");
    assert_eq!(changed.rows_affected(), 1);
    sqlx::query(
        "ALTER TABLE public.rehearsal_submission_claim_delivery_binding
         ENABLE TRIGGER rehearsal_claim_delivery_binding_append_only",
    )
    .execute(&fixture.pool)
    .await
    .expect("restore binding trigger");

    let dispatch = fixture
        .store
        .dispatch_rehearsal_submission_from_route(
            fixture.context,
            identity,
            RehearsalIdempotencyKey::new(key.into()).expect("dispatch key"),
        )
        .await;
    assert!(dispatch.is_err(), "substituted binding refuses dispatch");
    let grader_url = std::env::var("PLE_TEST_GRADER_DATABASE_URL").expect("grader database URL");
    let grader_pool = learning_data_access::postgres::lazy_pool(&grader_url).expect("grader pool");
    let mut grader_transaction = grader_pool.begin().await.expect("grader transaction");
    sqlx::query("SELECT set_config('ple.tenant_id', $1, true)")
        .bind(fixture.tenant.as_uuid().to_string())
        .execute(&mut *grader_transaction)
        .await
        .expect("grader tenant context");
    let sealed = sqlx::query_scalar::<_, String>(
        "SELECT result_kind FROM public.ple_prepare_or_resume_sealed_rehearsal_submission(
            $1,$2,$3,$4,$5,$6,$7)",
    )
    .bind(fixture.tenant.as_uuid())
    .bind(identity.actor.as_uuid())
    .bind(identity.course.as_uuid())
    .bind(i32::try_from(identity.assignment.number()).expect("assignment reference"))
    .bind(i64::try_from(identity.expected_revision.value()).expect("revision"))
    .bind(i64::from(identity.rehearsal.number()))
    .bind(key)
    .fetch_optional(&mut *grader_transaction)
    .await
    .expect("sealed preparation refusal");
    assert!(
        sealed.is_none(),
        "substituted binding refuses sealed preparation"
    );
    grader_transaction
        .commit()
        .await
        .expect("grader transaction commit");
    let after: (i64, i64, i64) = sqlx::query_as(
        "SELECT
            (SELECT count(*) FROM rehearsal_submission_claim_event
              WHERE tenant_id=$1 AND rehearsal_run_id=$2),
            (SELECT count(*) FROM rehearsal_evidence
              WHERE tenant_id=$1 AND rehearsal_run_id=$2),
            (SELECT count(*) FROM rehearsal_submission_receipt
              WHERE tenant_id=$1 AND rehearsal_run_id=$2)",
    )
    .bind(fixture.tenant.as_uuid())
    .bind(fixture.run_id)
    .fetch_one(&fixture.pool)
    .await
    .expect("post-substitution counts");
    assert_eq!(after, before, "refused dispatch adds no event or evidence");
}

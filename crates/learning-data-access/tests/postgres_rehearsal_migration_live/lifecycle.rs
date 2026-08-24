use super::fixture::{app, bytes, id, millis, pool, source, start, start_with_intent};
use sqlx::Row;

#[tokio::test]
#[ignore = "requires PLE_TEST_DATABASE_URL and disposable PostgreSQL 17"]
async fn rehearsal_start_resumes_or_replaces_atomically_and_revokes_the_claim() {
    let pool = pool().await;
    let source = source(&pool).await;
    let old_run = id();
    let mut transaction = app(&pool, source.tenant).await;
    let first = start(&mut transaction, source, old_run, 1).await;
    let resumed = start_with_intent(&mut transaction, source, id(), 1, false, Some(old_run))
        .await
        .expect("known active rehearsal resumes");
    assert_eq!(
        resumed, first,
        "identical subject resumes the active rehearsal"
    );
    let claim = id();
    let operation = id();
    let created: bool = sqlx::query_scalar(
        "SELECT public.ple_rehearsal_create_claim($1,$2,$3,$4,1,$5,$6,$7,'replace-claim',$8,$9,'{}'::jsonb)",
    )
    .bind(source.tenant).bind(source.actor).bind(source.course).bind(source.assignment)
    .bind(old_run).bind(claim).bind(operation).bind(id()).bind(bytes(3))
    .fetch_one(&mut *transaction).await.expect("create prepared claim");
    assert!(created, "active rehearsal accepts its first claim");
    let replaced = start_with_intent(&mut transaction, source, id(), 2, false, Some(old_run))
        .await
        .expect("known active rehearsal replaces");
    assert_ne!(
        replaced, first,
        "different subject atomically replaces the active rehearsal"
    );
    let state = sqlx::query(
        "SELECT lifecycle, phase FROM rehearsal_run run JOIN rehearsal_submission_claim_event event \
         ON event.tenant_id=run.tenant_id AND event.rehearsal_run_id=run.rehearsal_run_id \
         WHERE run.tenant_id=$1 AND run.rehearsal_run_id=$2 ORDER BY event.sequence DESC LIMIT 1",
    )
    .bind(source.tenant).bind(old_run).fetch_one(&mut *transaction).await.expect("replaced run state");
    assert_eq!(
        state.try_get::<String, _>("lifecycle").expect("lifecycle"),
        "discardedByNewSubject"
    );
    assert_eq!(
        state.try_get::<String, _>("phase").expect("phase"),
        "revokedTerminalLifecycle"
    );
    transaction.commit().await.expect("commit replacement");
}

#[tokio::test]
#[ignore = "requires PLE_TEST_DATABASE_URL and disposable PostgreSQL 17"]
async fn frozen_and_completed_claim_append_one_evidence_chain_and_refuse_stale_completion() {
    let pool = pool().await;
    let source = source(&pool).await;
    let run = id();
    let mut transaction = app(&pool, source.tenant).await;
    start(&mut transaction, source, run, 1).await;
    let db_millis = millis(&mut transaction).await;
    let frozen: bool = sqlx::query_scalar(
        "SELECT public.ple_rehearsal_append_frozen_item($1,$2,$3,$4,1,$5,$6,0,$7,'{}'::jsonb,$8,$9,$10,$11,$12,'{}'::jsonb,$13,$14,$9)",
    )
    .bind(source.tenant).bind(source.actor).bind(source.course).bind(source.assignment).bind(run)
    .bind(bytes(0)).bind(bytes(1)).bind(bytes(2)).bind(db_millis).bind(id()).bind(id()).bind(id())
    .bind(bytes(4)).bind(bytes(5))
    .fetch_one(&mut *transaction).await.expect("freeze item");
    assert!(frozen, "frozen item advances evidence head");
    let persisted_millis: i64 = sqlx::query_scalar(
        "SELECT (extract(epoch FROM recorded_at) * 1000)::bigint FROM rehearsal_evidence \
         WHERE tenant_id=$1 AND rehearsal_run_id=$2 AND sequence=1",
    )
    .bind(source.tenant)
    .bind(run)
    .fetch_one(&mut *transaction)
    .await
    .expect("stored evidence timestamp");
    assert_eq!(
        persisted_millis, db_millis,
        "evidence round-trips the database-owned millisecond timestamp exactly"
    );
    let claim = id();
    let operation = id();
    let claim_created: bool = sqlx::query_scalar(
        "SELECT public.ple_rehearsal_create_claim($1,$2,$3,$4,1,$5,$6,$7,'completion-claim',$8,$9,'{}'::jsonb)",
    ).bind(source.tenant).bind(source.actor).bind(source.course).bind(source.assignment).bind(run)
     .bind(claim).bind(operation).bind(id()).bind(bytes(6)).fetch_one(&mut *transaction).await.expect("claim");
    assert!(claim_created, "claim is prepared");
    let dispatched: bool = sqlx::query_scalar(
        "SELECT public.ple_rehearsal_append_claim_event($1,$2,$3,$4,1,$5,$6,$7,'gradingDispatched',NULL)",
    ).bind(source.tenant).bind(source.actor).bind(source.course).bind(source.assignment).bind(run).bind(claim).bind(operation)
     .fetch_one(&mut *transaction).await.expect("dispatch");
    assert!(dispatched, "prepared claim dispatches");
    let completed: bool = sqlx::query_scalar(
        "SELECT public.ple_rehearsal_complete_claim($1,$2,$3,$4,1,$5,$6,$7,$8,1,$9,'{}'::jsonb,$10,$11,'{}'::jsonb,$12)",
    ).bind(source.tenant).bind(source.actor).bind(source.course).bind(source.assignment).bind(run).bind(claim).bind(operation)
     .bind(bytes(1)).bind(bytes(7)).bind(bytes(8)).bind(db_millis).bind(bytes(9))
     .fetch_one(&mut *transaction).await.expect("complete claim");
    assert!(
        completed,
        "completion atomically appends accepted evidence and receipt"
    );
    let stale: bool = sqlx::query_scalar(
        "SELECT public.ple_rehearsal_complete_claim($1,$2,$3,$4,1,$5,$6,$7,$8,2,$9,'{}'::jsonb,$10,$11,'{}'::jsonb,$12)",
    ).bind(source.tenant).bind(source.actor).bind(source.course).bind(source.assignment).bind(run).bind(claim).bind(operation)
     .bind(bytes(7)).bind(bytes(10)).bind(bytes(11)).bind(db_millis).bind(bytes(12))
     .fetch_one(&mut *transaction).await.expect("stale completion returns false");
    assert!(!stale, "completed operation cannot append evidence twice");
    let evidence: i64 = sqlx::query_scalar(
        "SELECT evidence_length FROM rehearsal_run WHERE tenant_id=$1 AND rehearsal_run_id=$2",
    )
    .bind(source.tenant)
    .bind(run)
    .fetch_one(&mut *transaction)
    .await
    .expect("head length");
    assert_eq!(
        evidence, 2,
        "freeze and completion are the only evidence appends"
    );
    transaction
        .commit()
        .await
        .expect("commit evidence lifecycle");
}

#[tokio::test]
#[ignore = "requires PLE_TEST_DATABASE_URL and disposable PostgreSQL 17"]
async fn completed_rehearsal_requires_live_restart_intent_and_an_exact_latest_witness() {
    let pool = pool().await;
    let source = source(&pool).await;
    let completed_run = id();
    let mut transaction = app(&pool, source.tenant).await;
    start(&mut transaction, source, completed_run, 1).await;
    let completed: bool =
        sqlx::query_scalar("SELECT public.ple_rehearsal_terminalize($1,$2,$3,$4,1,$5,'completed')")
            .bind(source.tenant)
            .bind(source.actor)
            .bind(source.course)
            .bind(source.assignment)
            .bind(completed_run)
            .fetch_one(&mut *transaction)
            .await
            .expect("complete live rehearsal");
    assert!(
        completed,
        "completion persists the ordinary terminal rehearsal"
    );
    assert!(
        start_with_intent(
            &mut transaction,
            source,
            id(),
            2,
            false,
            Some(completed_run)
        )
        .await
        .is_none(),
        "completed rehearsal does not restart without explicit live intent"
    );
    assert!(
        start_with_intent(&mut transaction, source, id(), 2, true, Some(id()))
            .await
            .is_none(),
        "stale latest-run witness cannot replace a completed rehearsal"
    );
    assert!(
        start_with_intent(&mut transaction, source, id(), 2, true, Some(completed_run))
            .await
            .is_some(),
        "explicit confirmed restart creates the next durable rehearsal"
    );
    transaction
        .commit()
        .await
        .expect("commit confirmed live restart");
}

#[tokio::test]
#[ignore = "requires PLE_TEST_DATABASE_URL and disposable PostgreSQL 17"]
async fn abandoned_claim_requires_a_fresh_operation_for_the_next_generation() {
    let pool = pool().await;
    let source = source(&pool).await;
    let mut transaction = app(&pool, source.tenant).await;
    let run = id();
    start(&mut transaction, source, run, 1).await;
    let claim = id();
    let old_operation = id();
    let created: bool = sqlx::query_scalar("SELECT public.ple_rehearsal_create_claim($1,$2,$3,$4,1,$5,$6,$7,'reclaim-claim',$8,$9,'{}'::jsonb)")
        .bind(source.tenant).bind(source.actor).bind(source.course).bind(source.assignment).bind(run).bind(claim).bind(old_operation).bind(id()).bind(bytes(1))
        .fetch_one(&mut *transaction).await.expect("claim");
    assert!(created, "claim starts prepared");
    let abandoned: bool = sqlx::query_scalar("SELECT public.ple_rehearsal_append_claim_event($1,$2,$3,$4,1,$5,$6,$7,'abandonedBeforeDispatch','nativeBackendAdmissionRejected')")
        .bind(source.tenant).bind(source.actor).bind(source.course).bind(source.assignment).bind(run).bind(claim).bind(old_operation)
        .fetch_one(&mut *transaction).await.expect("abandon");
    assert!(abandoned, "prepared claim may abandon before dispatch");
    let reused: bool = sqlx::query_scalar(
        "SELECT public.ple_rehearsal_append_claim_event($1,$2,$3,$4,1,$5,$6,$7,'prepared',NULL)",
    )
    .bind(source.tenant)
    .bind(source.actor)
    .bind(source.course)
    .bind(source.assignment)
    .bind(run)
    .bind(claim)
    .bind(old_operation)
    .fetch_one(&mut *transaction)
    .await
    .expect("reused operation result");
    assert!(!reused, "abandoned operation cannot be reclaimed");
    let fresh: bool = sqlx::query_scalar(
        "SELECT public.ple_rehearsal_append_claim_event($1,$2,$3,$4,1,$5,$6,$7,'prepared',NULL)",
    )
    .bind(source.tenant)
    .bind(source.actor)
    .bind(source.course)
    .bind(source.assignment)
    .bind(run)
    .bind(claim)
    .bind(id())
    .fetch_one(&mut *transaction)
    .await
    .expect("fresh operation result");
    assert!(fresh, "fresh operation begins the next claim generation");
    transaction.commit().await.expect("commit reclaim");
}

#[tokio::test]
#[ignore = "requires PLE_TEST_DATABASE_URL and disposable PostgreSQL 17"]
async fn direct_instructor_removal_fence_requires_the_current_actor_and_terminalizes_exactly_its_run()
 {
    let pool = pool().await;
    let source = source(&pool).await;
    let run = id();
    let mut transaction = app(&pool, source.tenant).await;
    start(&mut transaction, source, run, 1).await;
    let roster_revision: i64 = sqlx::query_scalar(
        "SELECT revision FROM course_roster_state WHERE tenant_id=$1 AND course_id=$2",
    )
    .bind(source.tenant)
    .bind(source.course)
    .fetch_one(&mut *transaction)
    .await
    .expect("current roster revision");
    let direct_witness: (i64, i64, Vec<uuid::Uuid>) = sqlx::query_as(
        "SELECT roster_revision, locked_rehearsal_count, locked_rehearsal_run_ids FROM public.ple_prepare_direct_instructor_rehearsal_fence($1,$2,$3,$4,$5)",
    )
    .bind(source.tenant)
    .bind(source.actor)
    .bind(source.course)
    .bind(source.membership)
    .bind(roster_revision)
    .fetch_one(&mut *transaction)
    .await
    .expect("direct-Instructor fence returns its opaque locked-run witness");
    assert_eq!(
        direct_witness.0, roster_revision,
        "prepare returns the locked roster revision"
    );
    assert_eq!(
        direct_witness.1, 1,
        "prepare counts the one active rehearsal"
    );
    assert_eq!(
        direct_witness.2,
        vec![run],
        "prepare returns sorted opaque run identifiers"
    );
    let fenced: i64 = sqlx::query_scalar(
        "SELECT public.ple_fence_rehearsals_for_direct_instructor_removal($1,$2,$3,$4,$5,$6)",
    )
    .bind(source.tenant)
    .bind(source.actor)
    .bind(source.course)
    .bind(source.membership)
    .bind(roster_revision)
    .bind(1_i64)
    .fetch_one(&mut *transaction)
    .await
    .expect("current Instructor source-removal capability");
    assert_eq!(
        fenced, 1,
        "the exact active direct-Instructor rehearsal is fenced"
    );
    let membership_status: String = sqlx::query_scalar(
        "SELECT status FROM course_member WHERE tenant_id=$1 AND course_membership_id=$2",
    )
    .bind(source.tenant)
    .bind(source.membership)
    .fetch_one(&mut *transaction)
    .await
    .expect("revoked target membership");
    assert_eq!(
        membership_status, "revoked",
        "fence atomically revokes the target Instructor"
    );
    let lifecycle: String = sqlx::query_scalar(
        "SELECT lifecycle FROM rehearsal_run WHERE tenant_id=$1 AND rehearsal_run_id=$2",
    )
    .bind(source.tenant)
    .bind(run)
    .fetch_one(&mut *transaction)
    .await
    .expect("fenced lifecycle");
    assert_eq!(
        lifecycle, "discardedSourceContextRemoved",
        "source removal retains the archive while hiding the active projection"
    );
    transaction
        .commit()
        .await
        .expect("commit direct-Instructor fence");
}

#[tokio::test]
#[ignore = "requires PLE_TEST_DATABASE_URL and disposable PostgreSQL 17"]
async fn revision_invalidation_requires_an_exact_advance_and_fences_the_old_revision() {
    let pool = pool().await;
    let source = source(&pool).await;
    let old_run = id();
    let mut old = app(&pool, source.tenant).await;
    start(&mut old, source, old_run, 1).await;
    old.commit().await.expect("commit old rehearsal");
    let mut legacy = app(&pool, source.tenant).await;
    let direct = sqlx::query_scalar::<_, i64>(
        "SELECT public.ple_invalidate_rehearsals_for_assignment($1,$2,$3,$4,1,2)",
    )
    .bind(source.tenant)
    .bind(source.actor)
    .bind(source.course)
    .bind(source.assignment)
    .fetch_one(&mut *legacy)
    .await;
    assert!(
        direct.is_err(),
        "application role cannot invoke the internal invalidation capability"
    );
    legacy
        .rollback()
        .await
        .expect("close denied application invalidation");
    let mut mismatched = app(&pool, source.tenant).await;
    let assignment_witness: (i64, i64, Vec<uuid::Uuid>) = sqlx::query_as(
        "SELECT assignment_revision, locked_rehearsal_count, locked_rehearsal_run_ids FROM public.ple_prepare_assignment_rehearsal_verification($1,$2,$3,$4,1)",
    )
    .bind(source.tenant)
    .bind(source.actor)
    .bind(source.course)
    .bind(source.assignment)
    .fetch_one(&mut *mismatched)
    .await
    .expect("prepare locks the current assignment source and returns opaque witnesses");
    assert_eq!(
        assignment_witness.0, 1,
        "prepare returns the exact locked revision"
    );
    assert_eq!(
        assignment_witness.1, 1,
        "prepare counts the active rehearsal"
    );
    assert_eq!(
        assignment_witness.2,
        vec![old_run],
        "prepare returns sorted opaque run identifiers"
    );
    let wrong_count = sqlx::query_scalar::<_, i64>(
        "SELECT public.ple_put_assignment_teaching_settings($1,$2,$3,$4,1,'{\"lifecycle\":\"published\",\"instructions\":\"wrong count\",\"basePolicy\":{\"availableAt\":null,\"dueAt\":null,\"closesAt\":null,\"lateSubmission\":\"accept\",\"deadlineBehavior\":\"autoSubmit\",\"timeLimitSeconds\":null,\"attemptLimit\":null}}'::jsonb,0)",
    )
    .bind(source.tenant)
    .bind(source.actor)
    .bind(source.course)
    .bind(source.assignment)
    .fetch_one(&mut *mismatched)
    .await;
    assert!(
        wrong_count.is_err(),
        "a verified rehearsal count is mandatory at commit"
    );
    mismatched
        .rollback()
        .await
        .expect("rollback count-mismatch mutation");
    let unchanged_revision: i64 = sqlx::query_scalar(
        "SELECT revision FROM assignment WHERE tenant_id=$1 AND assignment_id=$2",
    )
    .bind(source.tenant)
    .bind(source.assignment)
    .fetch_one(&pool)
    .await
    .expect("count mismatch leaves assignment unchanged");
    assert_eq!(
        unchanged_revision, 1,
        "mismatched verification rolls back the whole mutation"
    );
    let mut mutation = app(&pool, source.tenant).await;
    let changed: i64 = sqlx::query_scalar(
        "SELECT public.ple_put_assignment_teaching_settings($1,$2,$3,$4,1,'{\"lifecycle\":\"published\",\"instructions\":\"revised\",\"basePolicy\":{\"availableAt\":null,\"dueAt\":null,\"closesAt\":null,\"lateSubmission\":\"accept\",\"deadlineBehavior\":\"autoSubmit\",\"timeLimitSeconds\":null,\"attemptLimit\":null}}'::jsonb,$5)",
    )
    .bind(source.tenant)
    .bind(source.actor)
    .bind(source.course)
    .bind(source.assignment)
    .bind(1_i64)
    .fetch_one(&mut *mutation)
    .await
    .expect("exact internal revision invalidation");
    assert_eq!(
        changed, 2,
        "one authorized mutation advances revision exactly once"
    );
    mutation
        .commit()
        .await
        .expect("commit exact revision invalidation");
    let mut stale = app(&pool, source.tenant).await;
    let unchanged = sqlx::query_scalar::<_, i64>(
        "SELECT public.ple_put_assignment_teaching_settings($1,$2,$3,$4,1,'{\"lifecycle\":\"published\",\"instructions\":\"stale\",\"basePolicy\":{\"availableAt\":null,\"dueAt\":null,\"closesAt\":null,\"lateSubmission\":\"accept\",\"deadlineBehavior\":\"autoSubmit\",\"timeLimitSeconds\":null,\"attemptLimit\":null}}'::jsonb,$5)",
    )
    .bind(source.tenant)
    .bind(source.actor)
    .bind(source.course)
    .bind(source.assignment)
    .bind(0_i64)
    .fetch_one(&mut *stale)
    .await;
    assert!(
        unchanged.is_err(),
        "unchanged revision invalidation is refused"
    );
    stale
        .rollback()
        .await
        .expect("close invalidation transaction");
    let lifecycle: String = sqlx::query_scalar(
        "SELECT lifecycle FROM rehearsal_run WHERE tenant_id=$1 AND rehearsal_run_id=$2",
    )
    .bind(source.tenant)
    .bind(old_run)
    .fetch_one(&pool)
    .await
    .expect("old rehearsal lifecycle");
    assert_eq!(
        lifecycle, "discardedStaleRevision",
        "exact old revision invalidation leaves a terminal archive"
    );
}

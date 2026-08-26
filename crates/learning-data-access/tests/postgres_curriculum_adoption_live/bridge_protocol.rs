//! Connected authority checks for the three-call B2 adoption bridge.

use learning_data_access::{
    CurriculumAdoptionStore, SessionLifetime, SessionStore, SessionSubject, SessionTokenHash,
    StoreError, TenantContext,
};
use question_model::{CurriculumPinReplacements, ForkAlphaPreviewRequest, UserId, UserRole};
use serde_json::{Value, json};
use sqlx::types::Json;
use sqlx::{Postgres, Row, Transaction};
use uuid::Uuid;

use super::fixture::AdoptionFixture;

/// The read-only preview transaction stays usable while writes bind the actor
/// through the separately session-locked facade.  These observations protect
/// the protocol boundary, rather than mirroring implementation details.
pub(super) async fn assert_public_bridge_protocol(fixture: &AdoptionFixture) {
    fixture
        .store
        .preview_fork_alpha(
            fixture.context,
            fixture.instructor_session,
            ForkAlphaPreviewRequest {
                source: fixture.alpha,
                replacements: CurriculumPinReplacements::default(),
            },
        )
        .await
        .expect("the public snapshot bridge supports an authorized read-only preview");

    assert_eq!(
        bridge_actor(fixture, fixture.context, fixture.instructor_session)
            .await
            .expect("approved Instructor actor facade"),
        fixture.instructor,
        "the write-binding actor is the active session subject"
    );
    assert_eq!(
        bridge_actor(fixture, fixture.context, fixture.learner_session).await,
        Err(StoreError::Forbidden),
        "a learner cannot acquire a curriculum materialization actor"
    );

    let expired = invalidated_instructor_session(fixture, 0xE1, "expired").await;
    let baseline = mutation_counts(fixture, "b2-bridge-expired").await;
    assert_eq!(
        bridge_actor(fixture, fixture.context, expired).await,
        Err(StoreError::Forbidden),
        "an expired session cannot acquire a curriculum materialization actor"
    );
    assert_eq!(
        mutation_counts(fixture, "b2-bridge-expired").await,
        baseline
    );

    let revoked = invalidated_instructor_session(fixture, 0xE2, "revoked").await;
    let baseline = mutation_counts(fixture, "b2-bridge-revoked").await;
    assert_eq!(
        bridge_actor(fixture, fixture.context, revoked).await,
        Err(StoreError::Forbidden),
        "a revoked session cannot acquire a curriculum materialization actor"
    );
    assert_eq!(
        mutation_counts(fixture, "b2-bridge-revoked").await,
        baseline
    );

    let baseline = mutation_counts(fixture, "b2-bridge-wrong-tenant").await;
    assert_eq!(
        bridge_actor(fixture, fixture.context, fixture.foreign_instructor_session).await,
        Err(StoreError::Forbidden),
        "a session from another tenant cannot acquire this tenant's materialization actor"
    );
    assert_eq!(
        mutation_counts(fixture, "b2-bridge-wrong-tenant").await,
        baseline
    );

    let bridge_only: bool = sqlx::query_scalar(
        "SELECT has_function_privilege('ple_app', \
            'public.ple_curriculum_adoption_materialization_actor_v1(uuid,character)', 'EXECUTE') \
            AND has_function_privilege('ple_app', \
            'public.ple_snapshot_curriculum_adoption_v1(uuid,character,jsonb)', 'EXECUTE') \
            AND has_function_privilege('ple_app', \
            'public.ple_materialize_curriculum_adoption_v1(uuid,character,uuid,jsonb)', 'EXECUTE') \
            AND NOT has_function_privilege('ple_app', \
            'public.ple_compile_curriculum_adoption_facts_v1(uuid,character,uuid,text,jsonb)', 'EXECUTE') \
            AND NOT has_function_privilege('ple_app', \
            'public.ple_cam_consume_materialization_preparation_v1(uuid,character,uuid,jsonb)', 'EXECUTE') \
            AND NOT has_function_privilege('ple_app', \
            'public.ple_apply_fork_alpha_v1(uuid,character,jsonb)', 'EXECUTE')",
    )
    .fetch_one(&fixture.pool)
    .await
    .expect("B2 public bridge privilege query");
    assert!(bridge_only, "ple_app reaches only the B2 public bridge");

    let baseline = mutation_counts(fixture, "b2-bridge-digest").await;
    let preparation = prepare(fixture, "b2-bridge-digest", [17; 32]).await;
    assert_eq!(preparation.snapshot["kind"], "prepare");
    assert_eq!(
        preparation.snapshot["actor"],
        json!(fixture.instructor.as_uuid())
    );
    assert_eq!(
        preparation.snapshot["requestSha256"],
        json!(vec![17_u8; 32])
    );
    refuse_in_current_transaction(
        fixture,
        preparation.transaction,
        preparation.id,
        materialization_envelope(preparation.id, fixture.instructor, [18; 32]),
    )
    .await;
    assert_eq!(mutation_counts(fixture, "b2-bridge-digest").await, baseline);

    let baseline = mutation_counts(fixture, "b2-bridge-actor").await;
    let preparation = prepare(fixture, "b2-bridge-actor", [19; 32]).await;
    refuse_in_current_transaction(
        fixture,
        preparation.transaction,
        preparation.id,
        materialization_envelope(preparation.id, fixture.foreign_instructor, [19; 32]),
    )
    .await;
    assert_eq!(mutation_counts(fixture, "b2-bridge-actor").await, baseline);

    let baseline = mutation_counts(fixture, "b2-bridge-preparation").await;
    let preparation = prepare(fixture, "b2-bridge-preparation", [20; 32]).await;
    let substituted = Uuid::from_bytes([0xB2; 16]);
    refuse_in_current_transaction(
        fixture,
        preparation.transaction,
        substituted,
        materialization_envelope(substituted, fixture.instructor, [20; 32]),
    )
    .await;
    assert_eq!(
        mutation_counts(fixture, "b2-bridge-preparation").await,
        baseline
    );

    let baseline = mutation_counts(fixture, "b2-bridge-cross-tx").await;
    let preparation = prepare(fixture, "b2-bridge-cross-tx", [21; 32]).await;
    preparation
        .transaction
        .commit()
        .await
        .expect("snapshot transaction commits without a materialization");
    let transaction = app_transaction(fixture, fixture.instructor_session).await;
    refuse_in_current_transaction(
        fixture,
        transaction,
        preparation.id,
        materialization_envelope(preparation.id, fixture.instructor, [21; 32]),
    )
    .await;
    assert_eq!(
        mutation_counts(fixture, "b2-bridge-cross-tx").await,
        baseline
    );
}

async fn bridge_actor(
    fixture: &AdoptionFixture,
    context: TenantContext,
    session: SessionTokenHash,
) -> Result<UserId, StoreError> {
    let mut transaction = fixture
        .pool
        .begin()
        .await
        .map_err(|error| StoreError::Unavailable(error.to_string()))?;
    sqlx::query(
        "SELECT set_config('ple.tenant_id', $1, true), set_config('ple.session_hash', $2, true)",
    )
    .bind(context.tenant_id().as_uuid().to_string())
    .bind(session.to_string())
    .execute(&mut *transaction)
    .await
    .map_err(|error| StoreError::Unavailable(error.to_string()))?;
    sqlx::query("SET LOCAL ROLE ple_app")
        .execute(&mut *transaction)
        .await
        .map_err(|error| StoreError::Unavailable(error.to_string()))?;
    let actor: Result<uuid::Uuid, sqlx::Error> = sqlx::query_scalar(
        "SELECT public.ple_curriculum_adoption_materialization_actor_v1($1, $2)",
    )
    .bind(context.tenant_id().as_uuid())
    .bind(session.to_string())
    .fetch_one(&mut *transaction)
    .await;
    match actor {
        Ok(actor) => {
            transaction
                .commit()
                .await
                .map_err(|error| StoreError::Unavailable(error.to_string()))?;
            Ok(UserId::from_uuid(actor))
        }
        Err(error) => {
            transaction.rollback().await.ok();
            let forbidden = error
                .as_database_error()
                .and_then(|database| database.code())
                .is_some_and(|code| code == "42501");
            if forbidden {
                Err(StoreError::Forbidden)
            } else {
                Err(StoreError::Unavailable(error.to_string()))
            }
        }
    }
}

async fn invalidated_instructor_session(
    fixture: &AdoptionFixture,
    marker: u8,
    state: &str,
) -> SessionTokenHash {
    let token = SessionTokenHash::compute(Uuid::from_bytes([marker; 16]).as_bytes());
    fixture
        .store
        .create_session(
            token,
            SessionSubject::new(
                fixture.tenant,
                fixture.instructor,
                "B2 invalid session",
                vec![UserRole::Instructor],
            )
            .expect("invalid-session subject"),
            SessionLifetime::from_seconds(3_600).expect("invalid-session lifetime"),
        )
        .await
        .expect("fixture session capability");
    let update = match state {
        "expired" => {
            "UPDATE auth_session SET created_at = transaction_timestamp() - interval '2 hours', \
             expires_at = transaction_timestamp() - interval '1 hour' WHERE session_hash = $1"
        }
        "revoked" => {
            "UPDATE auth_session SET revoked_at = transaction_timestamp() WHERE session_hash = $1"
        }
        _ => unreachable!("fixed invalid-session state"),
    };
    sqlx::query(update)
        .bind(token.to_string())
        .execute(&fixture.pool)
        .await
        .expect("fixture session invalidation");
    token
}

struct PreparedSnapshot {
    transaction: Transaction<'static, Postgres>,
    id: Uuid,
    snapshot: Value,
}

async fn prepare(fixture: &AdoptionFixture, key: &str, digest: [u8; 32]) -> PreparedSnapshot {
    let mut transaction = app_transaction(fixture, fixture.instructor_session).await;
    let actor = fixture.instructor.as_uuid();
    let operation = json!({
        "version": 1,
        "operation": {"kind": "applyForkAlpha"},
        "request": {
            "source": fixture.alpha,
            "replacements": [],
            "idempotencyKey": key,
        },
        "materializationBinding": {
            "version": 1,
            "actor": actor,
            "requestSha256": digest,
        },
    });
    let snapshot: Json<Value> =
        sqlx::query_scalar("SELECT public.ple_snapshot_curriculum_adoption_v1($1, $2, $3)")
            .bind(fixture.tenant.as_uuid())
            .bind(fixture.instructor_session.to_string())
            .bind(Json(operation))
            .fetch_one(&mut *transaction)
            .await
            .expect("valid write snapshot creates a bound preparation");
    let snapshot = snapshot.0;
    let id = snapshot["preparationId"]
        .as_str()
        .and_then(|value| value.parse().ok())
        .expect("snapshot preparation identifier");
    PreparedSnapshot {
        transaction,
        id,
        snapshot,
    }
}

async fn app_transaction(
    fixture: &AdoptionFixture,
    session: SessionTokenHash,
) -> Transaction<'static, Postgres> {
    let mut transaction = fixture.pool.begin().await.expect("application transaction");
    sqlx::query(
        "SELECT set_config('ple.tenant_id', $1, true), set_config('ple.session_hash', $2, true)",
    )
    .bind(fixture.tenant.as_uuid().to_string())
    .bind(session.to_string())
    .execute(&mut *transaction)
    .await
    .expect("application tenant context");
    sqlx::query("SET LOCAL ROLE ple_app")
        .execute(&mut *transaction)
        .await
        .expect("application bridge role");
    transaction
}

fn materialization_envelope(id: Uuid, actor: UserId, digest: [u8; 32]) -> Value {
    // This plan is intentionally schema-valid but never materialized: the
    // altered actor/digest/preparation must be refused by the binding fence
    // before semantic content or destination state can be consumed.
    json!({
        "version": 1,
        "operation": {"kind": "applyForkAlpha"},
        "preparationId": id,
        "actor": actor.as_uuid(),
        "requestSha256": digest,
        "plan": {
            "kind": "forkAlpha",
            "plan": {
                "semantic": {
                    "semanticInput": {},
                    "canonicalVersion": 1,
                    "canonicalBytes": [0],
                    "semanticDigest": [
                        110, 52, 11, 156, 255, 179, 122, 152, 156, 165, 68, 230, 187, 120,
                        10, 44, 120, 144, 29, 63, 179, 55, 56, 118, 133, 17, 163, 6, 23, 175,
                        160, 29
                    ]
                },
                "source": {"reference": "AC-1", "revision": "1"}
            }
        }
    })
}

async fn refuse_in_current_transaction(
    fixture: &AdoptionFixture,
    mut transaction: Transaction<'static, Postgres>,
    preparation: Uuid,
    envelope: Value,
) {
    let result: Result<Json<Value>, sqlx::Error> =
        sqlx::query_scalar("SELECT public.ple_materialize_curriculum_adoption_v1($1, $2, $3, $4)")
            .bind(fixture.tenant.as_uuid())
            .bind(fixture.instructor_session.to_string())
            .bind(preparation)
            .bind(Json(envelope))
            .fetch_one(&mut *transaction)
            .await;
    assert!(result.is_err(), "a substituted bridge binding is refused");
    transaction.rollback().await.expect("refusal rollback");
}

async fn mutation_counts(fixture: &AdoptionFixture, key: &str) -> (i64, i64, i64) {
    let row = sqlx::query(
        "SELECT \
            (SELECT count(*) FROM curriculum_adoption_receipt WHERE idempotency_key = $1) AS receipts, \
            (SELECT count(*) FROM curriculum_assignment_adoption_evidence) AS evidence, \
            (SELECT count(*) FROM alpha_course) AS alpha_courses",
    )
    .bind(key)
    .fetch_one(&fixture.pool)
    .await
    .expect("bridge mutation baseline");
    (
        row.get("receipts"),
        row.get("evidence"),
        row.get("alpha_courses"),
    )
}

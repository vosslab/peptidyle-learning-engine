//! Production-shaped learner-support invalidation evidence.

use learning_data_access::{AttemptSupportActionId, ClearAttemptCommand, Store, TenantContext};
use question_model::{QuestionAttemptId, TenantId, UserId};
use sqlx::{PgPool, Row};

use super::fresh_uuid;

pub(super) struct AttemptSupportScenario<'a> {
    pub(super) store: &'a learning_data_access::postgres::PostgresStore,
    pub(super) pool: &'a PgPool,
    pub(super) context: TenantContext,
    pub(super) tenant: TenantId,
    pub(super) instructor: UserId,
    pub(super) attempt: QuestionAttemptId,
}

pub(super) async fn prove_learner_support_origin(scenario: AttemptSupportScenario<'_>) {
    let AttemptSupportScenario {
        store,
        pool,
        context,
        tenant,
        instructor,
        attempt,
    } = scenario;
    let action = AttemptSupportActionId::from_uuid(fresh_uuid());
    let support = store
        .clear_attempt(
            context,
            ClearAttemptCommand {
                action,
                actor: instructor,
                attempt,
            },
        )
        .await
        .expect("clear an automated-evaluated attempt through the production adapter");
    assert_eq!(support.action, action, "support receipt retains its action");

    let row = sqlx::query(
        "SELECT actor_id, scoring_generation, recalculation_job_id, grading_operation_id \
         FROM public.scoring_invalidation_origin \
         WHERE tenant_id=$1 AND origin_kind='student_support' AND origin_id=$2",
    )
    .bind(tenant.as_uuid())
    .bind(action.as_uuid())
    .fetch_one(pool)
    .await
    .expect("read learner-support source origin");
    assert_eq!(
        row.try_get::<uuid::Uuid, _>("actor_id").unwrap(),
        instructor.as_uuid()
    );
    assert!(row.try_get::<i64, _>("scoring_generation").unwrap() > 0);
    assert_eq!(
        row.try_get::<uuid::Uuid, _>("recalculation_job_id")
            .unwrap(),
        action.as_uuid(),
        "source action deterministically owns its recalculation job"
    );
    assert!(
        row.try_get::<i64, _>("grading_operation_id").unwrap() > 0,
        "every scoring origin creates an Instructor-visible operation"
    );
}

//! Immutable grading-operation receipt subtype integrity proof.

use question_model::{CourseId, TenantId, UserId};
use sqlx::PgPool;
use uuid::Uuid;

use super::broker::admin_tenant_transaction;

pub(super) async fn prove_subtype_shape_rejection(
    pool: &PgPool,
    tenant: TenantId,
    operation: i32,
    course: CourseId,
    actor: UserId,
) {
    // Each candidate owns a transaction because PostgreSQL marks a transaction
    // failed after a CHECK violation. The missing-field cases cover SQL NULL
    // propagation; the cross-subtype cases cover unrelated-field fences.
    let candidates = [
        (
            "retry requires its resulting operation revision",
            "retry",
            Some(2_i64),
            None,
            Some(2_i64),
            None,
            Some("ready"),
            None,
            None,
        ),
        (
            "retry rejects recalculation-only fields",
            "retry",
            Some(1_i64),
            Some(2_i64),
            Some(2_i64),
            None,
            Some("ready"),
            Some(1_i64),
            None,
        ),
        (
            "recalculation requires its expected assignment revision",
            "recalculate",
            None,
            Some(1_i64),
            None,
            Some(2_i64),
            Some("recalculating"),
            None,
            Some(1_i64),
        ),
        (
            "recalculation rejects retry-only fields",
            "recalculate",
            Some(1_i64),
            Some(2_i64),
            None,
            Some(2_i64),
            Some("recalculating"),
            Some(1_i64),
            Some(1_i64),
        ),
    ];
    for (
        index,
        (
            label,
            action_kind,
            retry_expected,
            retry_resulting,
            execution_generation,
            scoring_generation,
            resulting_state,
            recalculate_expected,
            recalculate_created,
        ),
    ) in candidates.into_iter().enumerate()
    {
        let index = u128::try_from(index).expect("receipt candidate index fits u128");
        let action = Uuid::from_u128(0x7a11_0000_0000_4001_8000_0000_0000_0000 | index);
        let mut transaction = admin_tenant_transaction(pool, tenant).await;
        let result = sqlx::query(
            "INSERT INTO public.grading_operation_receipt (\
             tenant_id, action_id, grading_operation_id, course_id, actor_id, action_kind,\
             request_sha256, resulting_execution_generation, resulting_scoring_generation,\
             resulting_state, retry_expected_operation_revision, retry_resulting_operation_revision,\
             recalculate_expected_assignment_revision, recalculate_created_operation_revision)\
             VALUES ($1,$2,$3,$4,$5,$6,repeat('a',64),$7,$8,$9,$10,$11,$12,$13)",
        )
        .bind(tenant.as_uuid())
        .bind(action)
        .bind(i64::from(operation))
        .bind(course.as_uuid())
        .bind(actor.as_uuid())
        .bind(action_kind)
        .bind(execution_generation)
        .bind(scoring_generation)
        .bind(resulting_state)
        .bind(retry_expected)
        .bind(retry_resulting)
        .bind(recalculate_expected)
        .bind(recalculate_created)
        .execute(&mut *transaction)
        .await;
        assert!(result.is_err(), "malformed receipt was accepted: {label}");
        transaction
            .rollback()
            .await
            .expect("rollback malformed receipt probe");
    }
}

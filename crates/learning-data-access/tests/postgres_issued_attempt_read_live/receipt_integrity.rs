//! Adversarial immutable-evidence checks for the route-bound issued read.

use learning_data_access::postgres::PostgresStore;
use learning_data_access::{
    AttemptSupportActionId, ClearAttemptCommand, IssuedAttemptRead, Store, StoreError,
    StudentWorkRoutingBinding, TenantContext,
};
use question_model::{QuestionAttemptId, TenantId, UserId};
use sqlx::PgPool;

pub(super) struct ReceiptIntegrityOracle<'a> {
    pool: &'a PgPool,
    store: &'a PostgresStore,
    context: TenantContext,
    tenant: TenantId,
    student: UserId,
}

impl<'a> ReceiptIntegrityOracle<'a> {
    pub(super) fn new(
        pool: &'a PgPool,
        store: &'a PostgresStore,
        context: TenantContext,
        tenant: TenantId,
        student: UserId,
    ) -> Self {
        Self {
            pool,
            store,
            context,
            tenant,
            student,
        }
    }

    pub(super) async fn assert_active_issuance_fails_closed(
        &self,
        binding: StudentWorkRoutingBinding,
        attempt: QuestionAttemptId,
    ) {
        let checksum: String = sqlx::query_scalar(
            "SELECT presentation_payload_sha256 FROM public.question_attempt \
             WHERE tenant_id=$1 AND attempt_id=$2",
        )
        .bind(self.tenant.as_uuid())
        .bind(attempt.as_uuid())
        .fetch_one(self.pool)
        .await
        .expect("read disposable active presentation checksum");
        self.set_active_checksum_to_opposite_nibble(attempt).await;
        let corrupt_read = self
            .store
            .read_issued_attempt_evidence(self.context, self.student, binding, attempt)
            .await;
        assert!(
            matches!(
                corrupt_read,
                Err(StoreError::Unavailable(_) | StoreError::InvalidRecord(_))
            ),
            "corrupt active issuance fails closed: {corrupt_read:?}"
        );
        self.restore_active_checksum(attempt, &checksum).await;
    }

    pub(super) async fn assert_clear_preserves_terminal_receipt_read(
        &self,
        binding: StudentWorkRoutingBinding,
        instructor: UserId,
        attempt: QuestionAttemptId,
        action: AttemptSupportActionId,
    ) {
        self.store
            .clear_attempt(
                self.context,
                ClearAttemptCommand {
                    action,
                    actor: instructor,
                    attempt,
                },
            )
            .await
            .expect("clear submitted attempt through the ordinary instructor workflow");
        assert!(
            matches!(
                self.store
                    .read_issued_attempt_evidence(self.context, self.student, binding, attempt)
                    .await,
                Ok(IssuedAttemptRead::TerminalWithoutReceipt(ref read))
                    if read.status() == question_model::AttemptStatus::Cleared
            ),
            "cleared terminal is exposed without a learner receipt"
        );
    }

    async fn set_active_checksum_to_opposite_nibble(&self, attempt: QuestionAttemptId) {
        self.set_attempt_triggers(false).await;
        sqlx::query(
            "UPDATE public.question_attempt \
             SET presentation_payload_sha256=CASE WHEN left(presentation_payload_sha256,1)='0' \
                 THEN '1' ELSE '0' END || substr(presentation_payload_sha256,2) \
             WHERE tenant_id=$1 AND attempt_id=$2",
        )
        .bind(self.tenant.as_uuid())
        .bind(attempt.as_uuid())
        .execute(self.pool)
        .await
        .expect("corrupt disposable active issuance checksum");
        self.set_attempt_triggers(true).await;
    }

    async fn restore_active_checksum(&self, attempt: QuestionAttemptId, checksum: &str) {
        self.set_attempt_triggers(false).await;
        sqlx::query(
            "UPDATE public.question_attempt SET presentation_payload_sha256=$3 \
             WHERE tenant_id=$1 AND attempt_id=$2",
        )
        .bind(self.tenant.as_uuid())
        .bind(attempt.as_uuid())
        .bind(checksum)
        .execute(self.pool)
        .await
        .expect("restore disposable active issuance checksum");
        self.set_attempt_triggers(true).await;
    }

    async fn set_attempt_triggers(&self, enabled: bool) {
        let query = if enabled {
            "ALTER TABLE public.question_attempt ENABLE TRIGGER ALL"
        } else {
            "ALTER TABLE public.question_attempt DISABLE TRIGGER ALL"
        };
        sqlx::query(query)
            .execute(self.pool)
            .await
            .expect("change isolated corruption-probe triggers");
    }
}

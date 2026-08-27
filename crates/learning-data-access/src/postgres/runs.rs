use async_trait::async_trait;
use sqlx::types::Uuid;
use sqlx::{Postgres, Row, Transaction};

use super::*;
use crate::{
    LearnerWorkRoutingBinding, PrefetchedQuestionDescriptorV1, ReceiptNextAttempt,
    validate_issued_flat_grading, validate_issued_webwork_grading, validate_issued_webwork_replay,
};

pub(super) mod attempt_issuance;
pub(super) mod authored_timing;
pub(super) mod learner_projections;
pub(super) mod learner_transition;
pub(super) mod private_execution;
pub(super) use authored_timing::add_seconds;

/// Hydrates the authorized current delivery lifecycle and immutable issuance
/// evidence named by an already validated 1817 broker witness. The read uses
/// only bound parameters and retained broker locks (ASVS 1.2.4, 2.2.1-2.2.3,
/// 2.3.3, 8.2.2/8.3.1, 11.4.3, 14.2.6, 15.4.2, and 16.5.3). It deliberately
/// excludes assignment, run-item, current catalog, renderer, and summary
/// sources, so delivery evidence survives mutable-source evolution.
async fn hydrate_issued_attempt_evidence(
    transaction: &mut Transaction<'_, Postgres>,
    witness: &super::learner_work_preparation::StudentAttemptPreparationWitness,
) -> Result<crate::IssuedAttemptRead, StoreError> {
    let row = sqlx::query(
        "SELECT payload, payload_sha256, presentation_descriptor_version, presentation_nonce, \
                presentation_digest, presentation_capability, presentation_payload, \
                presentation_payload_sha256, grading_envelope_payload, \
                grading_envelope_payload_sha256, \
                attempt_status AS current_attempt_status, \
                floor(extract(epoch FROM submitted_at) * 1000)::bigint AS current_submitted_at \
           FROM question_attempt \
          WHERE tenant_id = $1 AND attempt_id = $2",
    )
    .bind(witness.source.tenant.as_uuid())
    .bind(witness.attempt.as_uuid())
    .fetch_optional(&mut **transaction)
    .await
    .map_err(map_sqlx_error)?
    .ok_or_else(|| {
        StoreError::Unavailable(
            "issued attempt disappeared after learner-work preparation".to_string(),
        )
    })?;
    let current_attempt = decode_issued_attempt_with_current_lifecycle_row(&row)?;
    if current_attempt.tenant != witness.source.tenant
        || current_attempt.id != witness.attempt
        || current_attempt.run != witness.run
        || current_attempt.status != witness.attempt_status
    {
        return Err(StoreError::Unavailable(
            "issued attempt disagrees with learner-work preparation witness".to_string(),
        ));
    }
    let capability = attempt_issuance::presentation_capability_from_row(&row)?;
    let presentation_binding = decode_presentation_binding_row(&row)?;
    let presentation_snapshot =
        attempt_issuance::decode_attempt_presentation_snapshot(&row, capability)?;
    let grading_envelope = attempt_issuance::decode_attempt_grading_envelope(&row, capability)?;
    let presentation_snapshot = crate::validate_issued_presentation(
        capability,
        &current_attempt,
        presentation_binding,
        presentation_snapshot.as_ref(),
        grading_envelope.as_ref(),
    )?;
    let receipt = crate::IssuedAttemptReceiptEvidence::new(
        presentation_binding,
        presentation_snapshot,
        grading_envelope,
    );
    // A submission receipt is immutable evidence rather than a projection
    // cache.  Read it for every lifecycle state so a clear or other terminal
    // transition cannot turn a corrupted receipt into invisible history.
    // The decoder verifies every retained receipt checksum before this state
    // machine chooses a browser-safe projection (ASVS 2.3.1, 2.3.3, 11.4.3).
    let submission_receipt = super::submission_receipts::load_submission_receipt_snapshot(
        transaction,
        witness.source.tenant,
        witness.attempt,
    )
    .await?;
    let receipt_requirement = SubmissionReceiptRequirement::for_status(current_attempt.status);
    let receipt_presentation = validate_submission_receipt_for_issued_attempt(
        submission_receipt,
        receipt_requirement,
        witness,
        &receipt,
    )?;
    match current_attempt.status {
        AttemptStatus::InProgress => Ok(crate::IssuedAttemptRead::Active(Box::new(
            crate::ActiveIssuedAttemptEvidence::new(receipt),
        ))),
        AttemptStatus::Submitted => Ok(crate::IssuedAttemptRead::Submitted(Box::new(
            crate::SubmittedIssuedAttemptRead::new(
                receipt,
                crate::SubmittedQuestionReceipt::new(receipt_presentation),
            ),
        ))),
        status => Ok(crate::IssuedAttemptRead::TerminalWithoutReceipt(Box::new(
            crate::TerminalIssuedAttemptRead::new(receipt, status),
        ))),
    }
}

/// Whether an issued-attempt lifecycle can legitimately have a submission
/// receipt. This keeps receipt integrity independent from the public read
/// projection: a terminal status may retain a previously submitted response.
#[derive(Clone, Copy, PartialEq, Eq)]
enum SubmissionReceiptRequirement {
    Absent,
    Required,
    Optional,
}

impl SubmissionReceiptRequirement {
    fn for_status(status: AttemptStatus) -> Self {
        match status {
            AttemptStatus::InProgress => Self::Absent,
            AttemptStatus::Submitted => Self::Required,
            // A normal pending-grade submission retains a receipt, while an
            // instructor force-submit can enter this terminal status without
            // fabricating a learner response. Both forms remain valid; a
            // retained receipt is still verified below.
            AttemptStatus::NeedsManualGrading
            | AttemptStatus::AutoSubmitted
            | AttemptStatus::Cleared
            | AttemptStatus::Exempt => Self::Optional,
        }
    }
}

/// Confirms a stored immutable submission receipt is the receipt for this
/// exact issued attempt. `load_submission_receipt_snapshot` has already
/// verified all receipt payload digests; this binds its immutable run and
/// presentation evidence to the broker-authorized attempt evidence.
fn validate_submission_receipt_for_issued_attempt(
    submission_receipt: Option<crate::CompletedSubmissionReceipt>,
    requirement: SubmissionReceiptRequirement,
    witness: &super::learner_work_preparation::StudentAttemptPreparationWitness,
    issued_receipt: &crate::IssuedAttemptReceiptEvidence,
) -> Result<Option<crate::ReceiptPresentationSnapshot>, StoreError> {
    match submission_receipt {
        None if requirement == SubmissionReceiptRequirement::Required => {
            Err(StoreError::Unavailable(
                "submission-backed issued attempt lacks its immutable receipt".to_string(),
            ))
        }
        None => Ok(None),
        Some(_) if requirement == SubmissionReceiptRequirement::Absent => {
            Err(StoreError::Unavailable(
                "active issued attempt unexpectedly has an immutable submission receipt"
                    .to_string(),
            ))
        }
        Some(receipt) => {
            if receipt.attempt.id != witness.attempt
                || receipt.attempt.tenant != witness.source.tenant
                || receipt.run.id != witness.run
                || receipt.run.tenant != witness.source.tenant
                || receipt.presentation != issued_receipt.presentation_snapshot().cloned()
            {
                return Err(StoreError::Unavailable(
                    "immutable submission receipt disagrees with issued evidence".to_string(),
                ));
            }
            Ok(receipt.presentation)
        }
    }
}

/// Accepts a concurrent successor-link write only when it retained the exact
/// durable descriptor this transaction produced. The id alone is not enough:
/// it would let a mismatched or checksum-invalid public successor replace the
/// receipt that replay is required to serve.
pub(super) async fn require_exact_submission_successor(
    transaction: &mut Transaction<'_, Postgres>,
    tenant: TenantId,
    predecessor: QuestionAttemptId,
    expected: Option<&ReceiptNextAttempt>,
) -> Result<(), StoreError> {
    let row = sqlx::query(
        "SELECT next_attempt_id, next_payload AS payload, next_payload_sha256 AS payload_sha256 \
         FROM submission_next_attempt WHERE tenant_id = $1 AND predecessor_attempt_id = $2",
    )
    .bind(tenant.as_uuid())
    .bind(predecessor.as_uuid())
    .fetch_optional(&mut **transaction)
    .await
    .map_err(map_sqlx_error)?
    .ok_or_else(|| {
        StoreError::Unavailable(
            "concurrent successor receipt disappeared before verification".to_string(),
        )
    })?;
    let stored_id: Option<Uuid> = row.try_get("next_attempt_id").map_err(map_sqlx_error)?;
    match (expected, stored_id) {
        (None, None) => {
            let payload: Option<serde_json::Value> =
                row.try_get("payload").map_err(map_sqlx_error)?;
            let checksum: Option<String> = row.try_get("payload_sha256").map_err(map_sqlx_error)?;
            if payload.is_some() || checksum.is_some() {
                return Err(StoreError::Unavailable(
                    "terminal successor receipt carries an unexpected descriptor".to_string(),
                ));
            }
            Ok(())
        }
        (Some(expected), Some(stored_id)) if stored_id == expected.id.as_uuid() => {
            let stored: ReceiptNextAttempt = decode_payload_row(&row)?;
            if &stored == expected {
                Ok(())
            } else {
                Err(StoreError::Unavailable(
                    "successor receipt does not match the immutable descriptor".to_string(),
                ))
            }
        }
        _ => Err(StoreError::Conflict),
    }
}

#[async_trait]
impl crate::RunStore for PostgresStore {
    async fn prepare_question_submission_impl(
        &self,
        context: TenantContext,
        actor: UserId,
        binding: LearnerWorkRoutingBinding,
        attempt: QuestionAttemptId,
        response: &StudentResponse,
        idempotency_key: &SubmissionIdempotencyKey,
    ) -> Result<crate::SubmissionPreparation, StoreError> {
        super::submission_preparation::prepare_question_submission(
            self,
            context,
            actor,
            binding,
            attempt,
            response,
            idempotency_key,
        )
        .await
    }

    async fn learner_assignment_run_items_impl(
        &self,
        context: TenantContext,
        actor: UserId,
        run: RunId,
    ) -> Result<Option<Vec<AssignmentRunItem>>, StoreError> {
        let mut transaction = self.begin_tenant(context).await?;
        if learner_projections::active_learner_run_for_read(
            &mut transaction,
            context.tenant_id(),
            actor,
            run,
        )
        .await?
        .is_none()
        {
            transaction.commit().await.map_err(map_sqlx_error)?;
            return Ok(None);
        }
        let items = load_assignment_run_items(&mut transaction, context.tenant_id(), run).await?;
        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(Some(items))
    }
    async fn start_or_resume_run_impl(
        &self,
        context: TenantContext,
        actor: UserId,
        binding: LearnerWorkRoutingBinding,
        proposed_run: RunId,
    ) -> Result<AssignmentRun, StoreError> {
        retry_transaction(|| async move {
            let mut transaction = self.begin_tenant(context).await?;
            let run = start_or_resume_run(&mut transaction, context, actor, binding, proposed_run)
                .await?;
            transaction.commit().await.map_err(map_sqlx_error)?;
            Ok(run)
        })
        .await
    }
    async fn assignment_run_items_impl(
        &self,
        context: TenantContext,
        run: RunId,
    ) -> Result<Vec<AssignmentRunItem>, StoreError> {
        let mut transaction = self.begin_tenant(context).await?;
        let exists: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM assignment_run WHERE tenant_id = $1 AND run_id = $2)",
        )
        .bind(context.tenant_id().as_uuid())
        .bind(run.as_uuid())
        .fetch_one(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;
        if !exists {
            return Err(StoreError::NotFound);
        }
        let items = load_assignment_run_items(&mut transaction, context.tenant_id(), run).await?;
        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(items)
    }
    async fn issue_or_resume_question_attempt_impl(
        &self,
        context: TenantContext,
        command: IssueQuestionAttemptCommand,
    ) -> Result<QuestionAttempt, StoreError> {
        retry_transaction(|| {
            let command = command.clone();
            async move {
                let mut transaction = self.begin_tenant(context).await?;
                let attempt = attempt_issuance::issue_or_resume_question_attempt(
                    &mut transaction,
                    context,
                    command,
                )
                .await?;
                transaction.commit().await.map_err(map_sqlx_error)?;
                Ok(attempt)
            }
        })
        .await
    }

    async fn read_issued_attempt_evidence_impl(
        &self,
        context: TenantContext,
        actor: UserId,
        binding: LearnerWorkRoutingBinding,
        attempt: QuestionAttemptId,
    ) -> Result<crate::IssuedAttemptRead, StoreError> {
        let mut transaction = self.begin_tenant(context).await?;
        // 1817 owns authorization and locks the learner-work graph before
        // this bounded immutable-evidence hydration reads any source row
        // (ASVS 1.2.4, 1.5.2, 2.2.1-2.2.3, 2.3.3, 8.2.2/8.3.1/8.4.1,
        // 11.4.3, 14.2.6, 15.4.2, and 16.5.3).
        let witness = super::learner_work_preparation::prepare_student_attempt_work(
            &mut transaction,
            context.tenant_id(),
            binding,
            actor,
            attempt,
        )
        .await?;
        let evidence = hydrate_issued_attempt_evidence(&mut transaction, &witness).await?;
        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(evidence)
    }
    async fn reserve_or_resume_prefetched_question_impl(
        &self,
        context: TenantContext,
        command: ReservePrefetchedQuestionCommand,
    ) -> Result<PrefetchedQuestionDescriptorV1, StoreError> {
        let reservation = command.reservation;
        let private_execution = command.private_execution;
        if reservation.tenant != context.tenant_id()
            || reservation.parameter_hash.trim().is_empty()
            || reservation
                .provenance
                .rendered_question_sha256
                .trim()
                .is_empty()
        {
            return Err(StoreError::InvalidRecord(
                "invalid prefetch reservation".to_string(),
            ));
        }
        reservation
            .issued_question_snapshot
            .validate_for_attempt(reservation.problem, reservation.question_version)?;
        reservation
            .issued_question_snapshot
            .validate_for_issuance_context(
                reservation.flat_grading_capability,
                reservation.webwork_grading_capability,
                reservation.qti_grading_capability,
                Some(&reservation.presentation_snapshot),
            )?;
        reservation
            .issued_question_snapshot
            .validate_native_provenance(&reservation.provenance.asset_objects)?;
        crate::validate_issued_qti_grading(
            reservation.issued_question_snapshot.question(),
            reservation.qti_grading_capability,
            private_execution.qti_grading.as_ref(),
        )?;
        validate_issued_flat_grading(
            reservation.issued_question_snapshot.question(),
            reservation.presentation_capability,
            reservation.flat_grading_capability,
            private_execution.flat_grading.as_ref(),
        )?;
        validate_issued_webwork_grading(
            reservation.issued_question_snapshot.question(),
            reservation.webwork_grading_capability,
            private_execution.webwork_grading.as_ref(),
        )?;
        validate_issued_webwork_replay(
            reservation.webwork_grading_capability,
            private_execution.webwork_replay.as_ref(),
        )?;
        let mut transaction = self.begin_tenant(context).await?;
        // The route-bound 1817 capability is the single authority and lock
        // owner for this live learner transition. It locks the course,
        // assignment, membership, enrollment, run, predecessor, and summary
        // in the same order used by submission, preventing a prefetch race
        // from introducing a second authorization model.
        let witness = super::learner_work_preparation::prepare_student_attempt_work(
            &mut transaction,
            context.tenant_id(),
            command.binding,
            command.actor,
            reservation.predecessor,
        )
        .await?;
        if witness.run != reservation.run || witness.attempt_status != AttemptStatus::InProgress {
            return Err(StoreError::Conflict);
        }
        // Hydrate projections with ordinary reads while the broker's exact
        // source locks remain held. Application code does not reacquire those
        // protected locks or repeat mutable policy evaluation.
        let run = load_postgres_run(&mut transaction, context.tenant_id(), reservation.run).await?;
        if run.completed_at.is_some() || run.score.is_some() {
            return Err(StoreError::Conflict);
        }
        let enrollment =
            load_postgres_enrollment(&mut transaction, context.tenant_id(), run.enrollment).await?;
        if witness.source.existing_enrollment != Some(run.enrollment)
            || enrollment.assignment != command.binding.assignment
            || enrollment.user != command.actor
        {
            return Err(StoreError::Unavailable(
                "prefetch source disagrees with learner-work preparation witness".to_string(),
            ));
        }
        let assignment =
            load_assignment(&mut transaction, context.tenant_id(), enrollment.assignment).await?;
        if assignment.course_id != command.binding.course {
            return Err(StoreError::Unavailable(
                "prefetch assignment disagrees with learner-work preparation witness".to_string(),
            ));
        }
        let submitted: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM submission_idempotency WHERE tenant_id = $1 AND attempt_id = $2)")
            .bind(context.tenant_id().as_uuid()).bind(reservation.predecessor.as_uuid())
            .fetch_one(&mut *transaction).await.map_err(map_sqlx_error)?;
        if submitted {
            return Err(StoreError::Conflict);
        }
        let expected = assignment
            .active_item_at(reservation.assignment_position)
            .ok_or_else(|| {
                StoreError::InvalidRecord("prefetch position is outside the assignment".to_string())
            })?;
        if expected.reference.problem != reservation.problem
            || expected.reference.version != reservation.question_version
        {
            return Err(StoreError::InvalidRecord(
                "prefetch identity does not match assignment position".to_string(),
            ));
        }
        let target_already_attempted: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM question_attempt WHERE tenant_id = $1 AND run_id = $2 AND assignment_position = $3)",
        )
        .bind(context.tenant_id().as_uuid())
        .bind(reservation.run.as_uuid())
        .bind(i32::try_from(reservation.assignment_position).map_err(|_| StoreError::InvalidRecord("prefetch position is too large".to_string()))?)
        .fetch_one(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;
        if target_already_attempted {
            return Err(StoreError::Conflict);
        }
        let existing = sqlx::query("SELECT payload, payload_sha256, presentation_descriptor_version, presentation_nonce, presentation_digest FROM question_prefetch WHERE tenant_id = $1 AND run_id = $2 AND predecessor_attempt_id = $3 AND assignment_position = $4")
            .bind(context.tenant_id().as_uuid()).bind(reservation.run.as_uuid()).bind(reservation.predecessor.as_uuid()).bind(i32::try_from(reservation.assignment_position).map_err(|_| StoreError::InvalidRecord("prefetch position is too large".to_string()))?)
            .fetch_optional(&mut *transaction).await.map_err(map_sqlx_error)?;
        if let Some(row) = existing {
            let existing: PrefetchedQuestionDescriptorV1 = decode_payload_row(&row)?;
            if decode_presentation_binding_row(&row)? != Some(existing.presentation) {
                return Err(StoreError::Unavailable(
                    "stored prefetch presentation disagrees with its columns".to_string(),
                ));
            }
            let private_matches = private_execution::prefetch_private_execution_matches(
                &mut transaction,
                context.tenant_id(),
                &reservation,
                &private_execution,
            )
            .await?;
            transaction.commit().await.map_err(map_sqlx_error)?;
            return if existing == reservation && private_matches {
                Ok(existing)
            } else {
                Err(StoreError::Conflict)
            };
        }
        let (payload, checksum) = encode_payload(&reservation)?;
        let inserted = sqlx::query("INSERT INTO question_prefetch (tenant_id, run_id, predecessor_attempt_id, predecessor_occurred_at, assignment_position, created_at, payload, payload_sha256, presentation_descriptor_version, presentation_nonce, presentation_digest) SELECT $1, $2, $3, qa.occurred_at, $4, transaction_timestamp(), $5, $6, $7, $8, $9 FROM question_attempt qa WHERE qa.tenant_id = $1 AND qa.attempt_id = $3 AND qa.run_id = $2")
            .bind(context.tenant_id().as_uuid()).bind(reservation.run.as_uuid()).bind(reservation.predecessor.as_uuid()).bind(i32::try_from(reservation.assignment_position).map_err(|_| StoreError::InvalidRecord("prefetch position is too large".to_string()))?).bind(payload).bind(checksum).bind(i16::from(reservation.presentation.descriptor_version())).bind(reservation.presentation.nonce().as_bytes().to_vec()).bind(reservation.presentation.digest().as_bytes().to_vec())
            .execute(&mut *transaction).await.map_err(map_sqlx_error)?;
        if inserted.rows_affected() != 1 {
            return Err(StoreError::Conflict);
        }
        if !private_execution::prefetch_private_execution_matches(
            &mut transaction,
            context.tenant_id(),
            &reservation,
            &private_execution,
        )
        .await?
        {
            return Err(StoreError::Conflict);
        }
        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(reservation)
    }
    async fn get_prefetched_question_impl(
        &self,
        context: TenantContext,
        actor: UserId,
        run: RunId,
        predecessor: QuestionAttemptId,
        assignment_position: u32,
    ) -> Result<Option<PrefetchedQuestionDescriptorV1>, StoreError> {
        let mut transaction = self.begin_tenant(context).await?;
        let run_record = load_run_for_update(&mut transaction, context.tenant_id(), run).await?;
        let enrollment = load_enrollment_for_update(
            &mut transaction,
            context.tenant_id(),
            run_record.enrollment,
        )
        .await?;
        let assignment =
            load_assignment(&mut transaction, context.tenant_id(), enrollment.assignment).await?;
        let decision = super::entitlement::evaluate_current(
            &mut transaction,
            context.tenant_id(),
            actor,
            assignment.course_id,
            enrollment.assignment,
        )
        .await?;
        if !matches!(decision, domain::entitlement::EntitlementDecision::Granted(ref grant) if grant.student() == enrollment.student)
        {
            return Err(StoreError::NotFound);
        }
        let row = sqlx::query("SELECT payload, payload_sha256, presentation_descriptor_version, presentation_nonce, presentation_digest FROM question_prefetch WHERE tenant_id = $1 AND run_id = $2 AND predecessor_attempt_id = $3 AND assignment_position = $4")
            .bind(context.tenant_id().as_uuid()).bind(run.as_uuid()).bind(predecessor.as_uuid()).bind(i32::try_from(assignment_position).map_err(|_| StoreError::InvalidRecord("prefetch position is too large".to_string()))?)
            .fetch_optional(&mut *transaction).await.map_err(map_sqlx_error)?;
        transaction.commit().await.map_err(map_sqlx_error)?;
        let Some(row) = row else {
            return Ok(None);
        };
        let reservation: PrefetchedQuestionDescriptorV1 = decode_payload_row(&row)?;
        if decode_presentation_binding_row(&row)? != Some(reservation.presentation) {
            return Err(StoreError::Unavailable(
                "stored prefetch presentation disagrees with its columns".to_string(),
            ));
        }
        Ok(Some(reservation))
    }
    async fn learner_get_prefetched_question_impl(
        &self,
        context: TenantContext,
        actor: UserId,
        run: RunId,
        predecessor: QuestionAttemptId,
        assignment_position: u32,
    ) -> Result<Option<PrefetchedQuestionDescriptorV1>, StoreError> {
        let mut transaction = self.begin_tenant(context).await?;
        if learner_projections::active_learner_run_for_read(
            &mut transaction,
            context.tenant_id(),
            actor,
            run,
        )
        .await?
        .is_none()
        {
            transaction.commit().await.map_err(map_sqlx_error)?;
            return Ok(None);
        }
        let row = sqlx::query("SELECT payload, payload_sha256, presentation_descriptor_version, presentation_nonce, presentation_digest FROM question_prefetch WHERE tenant_id = $1 AND run_id = $2 AND predecessor_attempt_id = $3 AND assignment_position = $4").bind(context.tenant_id().as_uuid()).bind(run.as_uuid()).bind(predecessor.as_uuid()).bind(i32::try_from(assignment_position).map_err(|_| StoreError::InvalidRecord("prefetch position is too large".to_string()))?).fetch_optional(&mut *transaction).await.map_err(map_sqlx_error)?;
        transaction.commit().await.map_err(map_sqlx_error)?;
        let Some(row) = row else { return Ok(None) };
        let reservation: PrefetchedQuestionDescriptorV1 = decode_payload_row(&row)?;
        if decode_presentation_binding_row(&row)? != Some(reservation.presentation) {
            return Err(StoreError::Unavailable(
                "stored prefetch presentation disagrees with its columns".to_string(),
            ));
        }
        Ok(Some(reservation))
    }
    async fn submission_next_attempt_impl(
        &self,
        context: TenantContext,
        actor: UserId,
        binding: LearnerWorkRoutingBinding,
        predecessor: QuestionAttemptId,
    ) -> Result<SubmissionNextAttempt, StoreError> {
        let mut transaction = self.begin_tenant(context).await?;
        if require_attempt_owner_for_read(&mut transaction, context.tenant_id(), predecessor, actor)
            .await?
            != binding
        {
            return Err(StoreError::NotFound);
        }
        let submitted: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM submission_idempotency WHERE tenant_id = $1 AND attempt_id = $2)")
            .bind(context.tenant_id().as_uuid()).bind(predecessor.as_uuid()).fetch_one(&mut *transaction).await.map_err(map_sqlx_error)?;
        if !submitted {
            return Err(StoreError::Conflict);
        }
        let next = sqlx::query(
            "SELECT next_attempt_id, next_payload AS payload, next_payload_sha256 AS payload_sha256 \
             FROM submission_next_attempt WHERE tenant_id = $1 AND predecessor_attempt_id = $2",
        )
        .bind(context.tenant_id().as_uuid())
        .bind(predecessor.as_uuid())
        .fetch_optional(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;
        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(match next {
            None => SubmissionNextAttempt::Pending,
            Some(row) => match row
                .try_get::<Option<Uuid>, _>("next_attempt_id")
                .map_err(map_sqlx_error)?
            {
                None => SubmissionNextAttempt::None,
                Some(id) => {
                    let next: ReceiptNextAttempt = decode_payload_row(&row)?;
                    if next.id != QuestionAttemptId::from_uuid(id) {
                        return Err(StoreError::Unavailable(
                            "successor receipt disagrees with its immutable link".to_string(),
                        ));
                    }
                    SubmissionNextAttempt::Issued(next)
                }
            },
        })
    }
    async fn pending_submission_for_run_impl(
        &self,
        context: TenantContext,
        actor: UserId,
        run: RunId,
    ) -> Result<Option<QuestionAttemptId>, StoreError> {
        let mut transaction = self.begin_tenant(context).await?;
        let record = load_run_for_update(&mut transaction, context.tenant_id(), run).await?;
        if load_enrollment_for_update(&mut transaction, context.tenant_id(), record.enrollment)
            .await?
            .user
            != actor
        {
            return Err(StoreError::Forbidden);
        }
        let ids: Vec<Uuid> = sqlx::query_scalar("SELECT qa.attempt_id FROM question_attempt qa JOIN submission_idempotency si ON si.tenant_id = qa.tenant_id AND si.attempt_id = qa.attempt_id LEFT JOIN submission_next_attempt sna ON sna.tenant_id = qa.tenant_id AND sna.predecessor_attempt_id = qa.attempt_id WHERE qa.tenant_id = $1 AND qa.run_id = $2 AND sna.predecessor_attempt_id IS NULL ORDER BY qa.occurred_at DESC LIMIT 2")
            .bind(context.tenant_id().as_uuid()).bind(run.as_uuid()).fetch_all(&mut *transaction).await.map_err(map_sqlx_error)?;
        transaction.commit().await.map_err(map_sqlx_error)?;
        match ids.as_slice() {
            [] => Ok(None),
            [id] => Ok(Some(QuestionAttemptId::from_uuid(*id))),
            _ => Err(StoreError::Conflict),
        }
    }
    async fn learner_pending_submission_for_run_impl(
        &self,
        context: TenantContext,
        actor: UserId,
        run: RunId,
    ) -> Result<Option<QuestionAttemptId>, StoreError> {
        let mut transaction = self.begin_tenant(context).await?;
        if learner_projections::active_learner_run_for_read(
            &mut transaction,
            context.tenant_id(),
            actor,
            run,
        )
        .await?
        .is_none()
        {
            transaction.commit().await.map_err(map_sqlx_error)?;
            return Ok(None);
        }
        let ids: Vec<Uuid> = sqlx::query_scalar("SELECT qa.attempt_id FROM question_attempt qa JOIN submission_idempotency si ON si.tenant_id = qa.tenant_id AND si.attempt_id = qa.attempt_id LEFT JOIN submission_next_attempt sna ON sna.tenant_id = qa.tenant_id AND sna.predecessor_attempt_id = qa.attempt_id WHERE qa.tenant_id = $1 AND qa.run_id = $2 AND sna.predecessor_attempt_id IS NULL ORDER BY qa.occurred_at DESC LIMIT 2").bind(context.tenant_id().as_uuid()).bind(run.as_uuid()).fetch_all(&mut *transaction).await.map_err(map_sqlx_error)?;
        transaction.commit().await.map_err(map_sqlx_error)?;
        match ids.as_slice() {
            [] => Ok(None),
            [id] => Ok(Some(QuestionAttemptId::from_uuid(*id))),
            _ => Err(StoreError::Conflict),
        }
    }
    async fn finalize_submission_next_attempt_impl(
        &self,
        context: TenantContext,
        actor: UserId,
        binding: LearnerWorkRoutingBinding,
        predecessor: QuestionAttemptId,
        next: Option<QuestionAttemptId>,
    ) -> Result<(), StoreError> {
        let mut transaction = self.begin_tenant(context).await?;
        super::learner_work_preparation::prepare_student_attempt_work(
            &mut transaction,
            context.tenant_id(),
            binding,
            actor,
            predecessor,
        )
        .await?;
        let submitted: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM submission_idempotency WHERE tenant_id = $1 AND attempt_id = $2)")
            .bind(context.tenant_id().as_uuid()).bind(predecessor.as_uuid()).fetch_one(&mut *transaction).await.map_err(map_sqlx_error)?;
        if !submitted {
            return Err(StoreError::Conflict);
        }
        // Successor receipts are immutable. The primary key serializes
        // concurrent finalizers without requiring an UPDATE grant solely for
        // SELECT FOR UPDATE; a losing insert accepts only the exact receipt.
        let (inserted, expected) = match next {
            Some(next) => {
                let row = sqlx::query(
                    "SELECT payload, payload_sha256, attempt_status AS current_attempt_status, \
                            floor(extract(epoch FROM submitted_at) * 1000)::bigint AS current_submitted_at, \
                            NULL::bigint AS current_deadline_at \
                     FROM question_attempt WHERE tenant_id = $1 AND attempt_id = $2",
                )
                .bind(context.tenant_id().as_uuid())
                .bind(next.as_uuid())
                .fetch_optional(&mut *transaction)
                .await
                .map_err(map_sqlx_error)?
                .ok_or(StoreError::NotFound)?;
                let next_attempt = decode_current_attempt_row(&row)?;
                let receipt_next = ReceiptNextAttempt::from_attempt(&next_attempt);
                let (payload, payload_sha256) = encode_payload(&receipt_next)?;
                let inserted = sqlx::query("INSERT INTO submission_next_attempt (tenant_id, predecessor_attempt_id, next_attempt_id, next_attempt_occurred_at, next_payload, next_payload_sha256) SELECT $1, $2, $3, next_attempt.occurred_at, $4, $5 FROM question_attempt next_attempt JOIN question_attempt predecessor_attempt ON predecessor_attempt.tenant_id = next_attempt.tenant_id AND predecessor_attempt.run_id = next_attempt.run_id WHERE next_attempt.tenant_id = $1 AND next_attempt.attempt_id = $3 AND predecessor_attempt.attempt_id = $2 ON CONFLICT (tenant_id, predecessor_attempt_id) DO NOTHING")
                    .bind(context.tenant_id().as_uuid()).bind(predecessor.as_uuid()).bind(next.as_uuid()).bind(payload).bind(payload_sha256).execute(&mut *transaction).await.map_err(map_sqlx_error)?
                ;
                (inserted, Some(receipt_next))
            }
            None => {
                let inserted = sqlx::query("INSERT INTO submission_next_attempt (tenant_id, predecessor_attempt_id, next_attempt_id, next_attempt_occurred_at) VALUES ($1, $2, NULL, NULL) ON CONFLICT (tenant_id, predecessor_attempt_id) DO NOTHING")
                .bind(context.tenant_id().as_uuid()).bind(predecessor.as_uuid()).execute(&mut *transaction).await.map_err(map_sqlx_error)?
                ;
                (inserted, None)
            }
        };
        if inserted.rows_affected() == 0 {
            require_exact_submission_successor(
                &mut transaction,
                context.tenant_id(),
                predecessor,
                expected.as_ref(),
            )
            .await?;
        }
        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(())
    }
    async fn list_question_attempts_impl(
        &self,
        context: TenantContext,
        run: RunId,
        page: PageRequest,
    ) -> Result<Page<QuestionAttempt>, StoreError> {
        let cursor = page.after.as_ref().map(|value| value.as_str().to_string());
        let limit = i64::from(page.size.get()) + 1;
        let mut transaction = self.begin_tenant(context).await?;
        let run_exists: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM assignment_run \
             WHERE tenant_id = $1 AND run_id = $2)",
        )
        .bind(context.tenant_id().as_uuid())
        .bind(run.as_uuid())
        .fetch_one(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;
        if !run_exists {
            return Err(StoreError::NotFound);
        }
        let rows = sqlx::query(
            "SELECT lpad(qa.assignment_position::text, 10, '0') || '/' || \
                    lpad((extract(epoch FROM qa.occurred_at) * 1000)::bigint::text, 20, '0') \
                    || '/' || qa.attempt_id::text AS stable_key, \
                    CASE WHEN si.request_contract_version = 2 THEN qa.payload ELSE COALESCE(si.payload, qa.payload) END AS payload, \
                    CASE WHEN si.request_contract_version = 2 THEN qa.payload_sha256 ELSE COALESCE(si.payload_sha256, qa.payload_sha256) END AS payload_sha256, \
                    evaluation.payload AS evaluation_payload, \
                    evaluation.payload_sha256 AS evaluation_payload_sha256, \
                    evaluation.grading_status AS evaluation_grading_status, \
                    qa.attempt_status AS current_attempt_status, \
                    floor(extract(epoch FROM qa.submitted_at) * 1000)::bigint \
                        AS current_submitted_at, \
                    floor(extract(epoch FROM timing.effective_deadline) * 1000)::bigint \
                        AS current_deadline_at \
             FROM question_attempt AS qa \
             LEFT JOIN submission_idempotency AS si \
               ON si.tenant_id = qa.tenant_id AND si.attempt_id = qa.attempt_id \
             LEFT JOIN submission_evaluation AS evaluation \
               ON evaluation.tenant_id = qa.tenant_id AND evaluation.attempt_id = qa.attempt_id \
             LEFT JOIN attempt_effective_policy_current AS current_effect ON current_effect.tenant_id=qa.tenant_id AND current_effect.attempt_id=qa.attempt_id \
             LEFT JOIN attempt_effective_policy_receipt AS timing ON timing.tenant_id=current_effect.tenant_id AND timing.attempt_id=current_effect.attempt_id AND timing.receipt_generation=current_effect.receipt_generation \
             WHERE qa.tenant_id = $1 AND qa.run_id = $2 \
               AND ($3::text IS NULL OR \
                    lpad(qa.assignment_position::text, 10, '0') || '/' || \
                    lpad((extract(epoch FROM qa.occurred_at) * 1000)::bigint::text, 20, '0') \
                    || '/' || qa.attempt_id::text > $3) \
             ORDER BY qa.assignment_position, qa.occurred_at, qa.attempt_id::text LIMIT $4",
        )
        .bind(context.tenant_id().as_uuid())
        .bind(run.as_uuid())
        .bind(cursor)
        .bind(limit)
        .fetch_all(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;
        let result = page_from_rows_with(
            rows,
            page.size.get(),
            decode_current_attempt_with_evaluation_row,
        )?;
        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(result)
    }
    async fn learner_list_question_attempts_impl(
        &self,
        context: TenantContext,
        actor: UserId,
        run: RunId,
        page: PageRequest,
    ) -> Result<Option<Page<QuestionAttempt>>, StoreError> {
        let cursor = page.after.as_ref().map(|value| value.as_str().to_string());
        let limit = i64::from(page.size.get()) + 1;
        let mut transaction = self.begin_tenant(context).await?;
        if learner_projections::active_learner_run_for_read(
            &mut transaction,
            context.tenant_id(),
            actor,
            run,
        )
        .await?
        .is_none()
        {
            transaction.commit().await.map_err(map_sqlx_error)?;
            return Ok(None);
        }
        let rows = sqlx::query("SELECT lpad(qa.assignment_position::text, 10, '0') || '/' || lpad((extract(epoch FROM qa.occurred_at) * 1000)::bigint::text, 20, '0') || '/' || qa.attempt_id::text AS stable_key, CASE WHEN si.request_contract_version = 2 THEN qa.payload ELSE COALESCE(si.payload, qa.payload) END AS payload, CASE WHEN si.request_contract_version = 2 THEN qa.payload_sha256 ELSE COALESCE(si.payload_sha256, qa.payload_sha256) END AS payload_sha256, evaluation.payload AS evaluation_payload, evaluation.payload_sha256 AS evaluation_payload_sha256, evaluation.grading_status AS evaluation_grading_status, qa.attempt_status AS current_attempt_status, floor(extract(epoch FROM qa.submitted_at) * 1000)::bigint AS current_submitted_at, floor(extract(epoch FROM timing.effective_deadline) * 1000)::bigint AS current_deadline_at FROM question_attempt AS qa LEFT JOIN submission_idempotency AS si ON si.tenant_id = qa.tenant_id AND si.attempt_id = qa.attempt_id LEFT JOIN submission_evaluation AS evaluation ON evaluation.tenant_id = qa.tenant_id AND evaluation.attempt_id = qa.attempt_id LEFT JOIN attempt_effective_policy_current AS current_effect ON current_effect.tenant_id=qa.tenant_id AND current_effect.attempt_id=qa.attempt_id LEFT JOIN attempt_effective_policy_receipt AS timing ON timing.tenant_id=current_effect.tenant_id AND timing.attempt_id=current_effect.attempt_id AND timing.receipt_generation=current_effect.receipt_generation WHERE qa.tenant_id = $1 AND qa.run_id = $2 AND ($3::text IS NULL OR lpad(qa.assignment_position::text, 10, '0') || '/' || lpad((extract(epoch FROM qa.occurred_at) * 1000)::bigint::text, 20, '0') || '/' || qa.attempt_id::text > $3) ORDER BY qa.assignment_position, qa.occurred_at, qa.attempt_id::text LIMIT $4").bind(context.tenant_id().as_uuid()).bind(run.as_uuid()).bind(cursor).bind(limit).fetch_all(&mut *transaction).await.map_err(map_sqlx_error)?;
        let result = page_from_rows_with(
            rows,
            page.size.get(),
            decode_current_attempt_with_evaluation_row,
        )?;
        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(Some(result))
    }
    async fn replay_submission_impl(
        &self,
        context: TenantContext,
        actor: UserId,
        attempt: QuestionAttemptId,
        response: &StudentResponse,
        idempotency_key: &SubmissionIdempotencyKey,
    ) -> Result<crate::SubmissionReceiptRead, StoreError> {
        let mut transaction = self.begin_tenant(context).await?;
        require_attempt_owner_for_read(&mut transaction, context.tenant_id(), attempt, actor)
            .await?;
        let record = load_submission_replay(
            &mut transaction,
            context.tenant_id(),
            attempt,
            response,
            idempotency_key,
        )
        .await?;
        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(record)
    }
    async fn submission_record_impl(
        &self,
        context: TenantContext,
        actor: UserId,
        attempt: QuestionAttemptId,
    ) -> Result<crate::SubmissionReceiptRead, StoreError> {
        let mut transaction = self.begin_tenant(context).await?;
        require_attempt_owner_for_read(&mut transaction, context.tenant_id(), attempt, actor)
            .await?;
        let record = super::submission_receipts::load_submission_record(
            &mut transaction,
            context.tenant_id(),
            attempt,
        )
        .await?;
        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(record)
    }
    async fn submit_question_attempt_impl(
        &self,
        context: TenantContext,
        command: SubmitQuestionAttemptCommand,
    ) -> Result<SubmissionRecord, StoreError> {
        retry_transaction(|| {
            let command = command.clone();
            async move {
                let mut transaction = self.begin_tenant(context).await?;
                let record = submit_question_attempt(&mut transaction, context, command).await?;
                transaction.commit().await.map_err(map_sqlx_error)?;
                Ok(record)
            }
        })
        .await
    }
    async fn force_submit_attempt_impl(
        &self,
        context: TenantContext,
        command: ForceSubmitAttemptCommand,
    ) -> Result<AttemptSupportRecord, StoreError> {
        retry_transaction(|| async move {
            let mut transaction = self.begin_tenant(context).await?;
            let record = apply_postgres_attempt_support(
                &mut transaction,
                context,
                command.action,
                command.actor,
                command.attempt,
                AttemptSupportAction::ForceSubmit,
            )
            .await?;
            transaction.commit().await.map_err(map_sqlx_error)?;
            Ok(record)
        })
        .await
    }
    async fn clear_attempt_impl(
        &self,
        context: TenantContext,
        command: ClearAttemptCommand,
    ) -> Result<AttemptSupportRecord, StoreError> {
        retry_transaction(|| async move {
            let mut transaction = self.begin_tenant(context).await?;
            let record = apply_postgres_attempt_support(
                &mut transaction,
                context,
                command.action,
                command.actor,
                command.attempt,
                AttemptSupportAction::Clear,
            )
            .await?;
            transaction.commit().await.map_err(map_sqlx_error)?;
            Ok(record)
        })
        .await
    }
}

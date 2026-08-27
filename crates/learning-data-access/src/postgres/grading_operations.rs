//! PostgreSQL atomic accepted-input persistence for automated grading.

use async_trait::async_trait;
use question_model::{
    GradingOperationAction, GradingOperationReason, GradingOperationReference,
    GradingOperationState, SubmissionEvaluationStatus,
};
use sqlx::Row;

use super::*;
use crate::{
    AcceptedSubmission, AcceptedSubmissionCommand, AcceptedSubmissionCommitError,
    AcceptedSubmissionExecution, AcceptedSubmissionExecutionClaim,
    AcceptedSubmissionExecutionDisposition, AcceptedSubmissionExecutionOutcome,
    AcceptedSubmissionExecutionStore, AcceptedSubmissionExecutionWorkerStore, AcceptedSubmissionId,
    AutomatedGradingStore, GradingExecution, GradingExecutionGeneration, GradingExecutionReceipt,
    GradingExecutionState, GradingOperation, GradingOperationRevision, GradingOperationTarget,
    JobLeaseDuration, JobLeaseToken, StoreError, TenantContext, WorkerId,
    canonical_student_response_json,
};

#[path = "grading_operations_completion.rs"]
mod grading_operations_completion;
use grading_operations_completion::*;

#[async_trait]
impl AutomatedGradingStore for PostgresStore {
    async fn accept_automated_submission(
        &self,
        context: TenantContext,
        command: AcceptedSubmissionCommand,
    ) -> Result<AcceptedSubmission, StoreError> {
        let tenant = context.tenant_id();
        // ASVS 1.5.3: every backend receives the same one typed response
        // representation. The broker hashes these exact UTF-8 bytes before
        // parsing the independently stored JSONB payload.
        let canonical_response = canonical_student_response_json(&command.response)?;
        let mut transaction = self.begin_tenant(context).await?;
        // ASVS 2.2.2 and 2.3.1: the broker derives and records all durable
        // witnesses under one locked transaction. Rust only transports the
        // bounded response and decodes the broker's canonical result.
        let row = sqlx::query(
            "SELECT result_kind, accepted_tenant_id, accepted_course_id, \
             accepted_assignment_id, accepted_attempt_id, accepted_submission_id, \
             accepted_actor_id, accepted_idempotency_key, accepted_request_sha256, \
             accepted_millis \
             FROM ple_accept_automated_submission_v1($1,$2,$3,$4,$5,$6,$7,$8)",
        )
        .bind(tenant.as_uuid())
        .bind(command.actor.as_uuid())
        .bind(command.course.as_uuid())
        .bind(command.assignment.as_uuid())
        .bind(command.attempt.as_uuid())
        .bind(command.idempotency_key.as_str())
        .bind(canonical_response)
        .bind(command.execution_job.as_uuid())
        .fetch_optional(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?
        .ok_or(StoreError::NotFound)?;
        let result_kind: String = row.try_get("result_kind").map_err(map_sqlx_error)?;
        if result_kind == "conflict" {
            return Err(StoreError::Conflict);
        }
        if result_kind == "timed_out" {
            return Err(StoreError::TimedOut);
        }
        if result_kind == "unavailable" {
            return Err(StoreError::Unavailable(
                "accepted-submission broker is unavailable".to_string(),
            ));
        }
        if result_kind != "accepted" && result_kind != "replayed" {
            return Err(StoreError::InvalidRecord(
                "invalid accepted-submission capability result".to_string(),
            ));
        }
        let request_sha256: objects::Sha256Digest =
            serde_json::from_value(serde_json::Value::String(
                row.try_get("accepted_request_sha256")
                    .map_err(map_sqlx_error)?,
            ))
            .map_err(|_| {
                StoreError::InvalidRecord("invalid stored accepted-submission digest".to_string())
            })?;
        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(AcceptedSubmission {
            tenant: TenantId::from_uuid(row.try_get("accepted_tenant_id").map_err(map_sqlx_error)?),
            course: CourseId::from_uuid(row.try_get("accepted_course_id").map_err(map_sqlx_error)?),
            assignment: AssignmentId::from_uuid(
                row.try_get("accepted_assignment_id")
                    .map_err(map_sqlx_error)?,
            ),
            attempt: QuestionAttemptId::from_uuid(
                row.try_get("accepted_attempt_id").map_err(map_sqlx_error)?,
            ),
            submission: AcceptedSubmissionId::from_uuid(
                row.try_get("accepted_submission_id")
                    .map_err(map_sqlx_error)?,
            ),
            actor: UserId::from_uuid(row.try_get("accepted_actor_id").map_err(map_sqlx_error)?),
            idempotency_key: SubmissionIdempotencyKey::parse(
                &row.try_get::<String, _>("accepted_idempotency_key")
                    .map_err(map_sqlx_error)?,
            )
            .map_err(|_| StoreError::InvalidRecord("invalid stored idempotency key".to_string()))?,
            request_sha256,
            accepted_at: ActivityTimestamp::from_unix_millis(
                row.try_get("accepted_millis").map_err(map_sqlx_error)?,
            ),
        })
    }

    async fn automated_grading_execution(
        &self,
        context: TenantContext,
        submission: AcceptedSubmissionId,
    ) -> Result<Option<GradingExecution>, StoreError> {
        let tenant = context.tenant_id();
        let mut transaction = self.begin_tenant(context).await?;
        let row = sqlx::query("SELECT submission_id, execution_generation, state, current_job_id, retry_count FROM grading_execution WHERE tenant_id=$1 AND submission_id=$2")
            .bind(tenant.as_uuid()).bind(submission.as_uuid()).fetch_optional(&mut *transaction).await.map_err(map_sqlx_error)?;
        let execution = row.map(decode_execution).transpose()?;
        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(execution)
    }

    async fn automated_grading_operation(
        &self,
        context: TenantContext,
        course: CourseId,
        reference: GradingOperationReference,
    ) -> Result<Option<GradingOperation>, StoreError> {
        let tenant = context.tenant_id();
        let mut transaction = self.begin_tenant(context).await?;
        let row = sqlx::query("SELECT assignment_id, target_kind, submission_id, requested_scoring_generation, reason, state, revision, next_action FROM grading_operation WHERE tenant_id=$1 AND course_id=$2 AND grading_operation_id=$3")
            .bind(tenant.as_uuid()).bind(course.as_uuid()).bind(i64::from(reference.number()))
            .fetch_optional(&mut *transaction).await.map_err(map_sqlx_error)?;
        let operation = row
            .map(|row| decode_operation(tenant, course, reference, &row))
            .transpose()?;
        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(operation)
    }

    async fn record_automated_grading_execution_receipt(
        &self,
        _context: TenantContext,
        _receipt: GradingExecutionReceipt,
        _resulting_evaluation: SubmissionEvaluationStatus,
    ) -> Result<(), StoreError> {
        Err(StoreError::Unavailable(
            "automated execution committer is owned by G1-W4".to_string(),
        ))
    }
}

#[async_trait]
impl AcceptedSubmissionExecutionStore for PostgresAcceptedSubmissionExecutionStore {
    async fn load_accepted_submission_for_execution(
        &self,
        context: TenantContext,
        claim: AcceptedSubmissionExecutionClaim,
    ) -> Result<AcceptedSubmissionExecution, StoreError> {
        let tenant = context.tenant_id();
        if claim.tenant != tenant {
            return Err(StoreError::Conflict);
        }
        let mut transaction = self.begin_execution_tenant(context).await?;
        // ASVS 2.3, 8.1-8.4, 14.2, and 15.4: the only answer-bearing worker
        // read uses the exact lease plus transaction-local execution role and
        // tenant fence. The general PostgresStore cannot invoke this loader.
        let row = sqlx::query(
            "SELECT * FROM public.ple_load_accepted_submission_execution_v2($1,$2,$3,$4,$5,$6)",
        )
        .bind(tenant.as_uuid())
        .bind(claim.job.as_uuid())
        .bind(claim.lease_token.as_uuid())
        .bind(claim.submission.as_uuid())
        .bind(
            i64::try_from(claim.execution_generation.as_u64()).map_err(|_| {
                StoreError::InvalidRecord("grading execution generation is too large".to_string())
            })?,
        )
        .bind(claim.worker.as_uuid())
        .fetch_optional(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?
        .ok_or(StoreError::NotFound)?;

        let returned_job: Uuid = row.try_get("worker_job_id").map_err(map_sqlx_error)?;
        let returned_lease: Uuid = row.try_get("worker_lease_token").map_err(map_sqlx_error)?;
        let returned_generation: i64 = row
            .try_get("execution_generation")
            .map_err(map_sqlx_error)?;
        let returned_state: String = row.try_get("execution_state").map_err(map_sqlx_error)?;
        if returned_job != claim.job.as_uuid()
            || returned_lease != claim.lease_token.as_uuid()
            || GradingExecutionGeneration::from_u64(u64::try_from(returned_generation).map_err(
                |_| {
                    StoreError::Unavailable(
                        "execution-load broker returned an invalid generation".to_string(),
                    )
                },
            )?) != Some(claim.execution_generation)
            || row
                .try_get::<Uuid, _>("worker_id")
                .map_err(map_sqlx_error)?
                != claim.worker.as_uuid()
            || returned_state != "running"
        {
            return Err(StoreError::Unavailable(
                "execution-load broker disagrees with the exact worker claim".to_string(),
            ));
        }

        let accepted = decode_accepted_submission_row(&row)?;
        if accepted.tenant != tenant || accepted.submission != claim.submission {
            return Err(StoreError::Unavailable(
                "execution-load broker returned a foreign accepted submission".to_string(),
            ));
        }
        let response_canonical_json: String = row
            .try_get("response_canonical_json")
            .map_err(map_sqlx_error)?;
        let response: question_model::StudentResponse =
            serde_json::from_str(&response_canonical_json).map_err(|_| {
                StoreError::Unavailable(
                    "execution-load broker returned an invalid stored response".to_string(),
                )
            })?;
        let canonical = canonical_student_response_json(&response)?;
        if canonical != response_canonical_json
            || objects::Sha256Digest::compute(canonical.as_bytes()) != accepted.request_sha256
        {
            return Err(StoreError::Unavailable(
                "execution-load broker response identity disagrees with accepted input".to_string(),
            ));
        }
        let prepared =
            super::submission_preparation::decode_prepared_accepted_submission_execution(&row)?;
        if prepared.attempt.tenant != accepted.tenant || prepared.attempt.id != accepted.attempt {
            return Err(StoreError::Unavailable(
                "execution-load broker issued evidence disagrees with accepted input".to_string(),
            ));
        }
        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(AcceptedSubmissionExecution {
            accepted,
            response,
            prepared: Box::new(prepared),
        })
    }
}

#[async_trait]
impl AcceptedSubmissionExecutionWorkerStore for PostgresAcceptedSubmissionExecutionStore {
    async fn claim_next_accepted_submission_execution(
        &self,
        worker: WorkerId,
        lease: JobLeaseDuration,
    ) -> Result<Option<AcceptedSubmissionExecutionClaim>, StoreError> {
        let lease_token = JobLeaseToken::generate()?;
        let mut transaction = self.begin_execution_worker().await?;
        // ASVS 2.3.1/2.3.3: the broker atomically selects, leases, and binds
        // one tenant-scoped execution. The returned capability is committed
        // before any grader I/O begins.
        let row = sqlx::query(
            "SELECT * FROM public.ple_claim_accepted_submission_execution_v1($1,$2,$3)",
        )
        .bind(lease_token.as_uuid())
        .bind(worker.as_uuid())
        .bind(lease.seconds())
        .fetch_optional(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;
        let Some(row) = row else {
            transaction.commit().await.map_err(map_sqlx_error)?;
            return Ok(None);
        };
        let claim = decode_worker_claim(&row, worker, lease_token)?;
        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(Some(claim))
    }

    async fn load_accepted_submission_for_execution(
        &self,
        context: TenantContext,
        claim: AcceptedSubmissionExecutionClaim,
    ) -> Result<AcceptedSubmissionExecution, StoreError> {
        <Self as AcceptedSubmissionExecutionStore>::load_accepted_submission_for_execution(
            self, context, claim,
        )
        .await
    }

    async fn commit_or_fail_accepted_submission_execution(
        &self,
        context: TenantContext,
        claim: AcceptedSubmissionExecutionClaim,
        outcome: AcceptedSubmissionExecutionOutcome,
    ) -> Result<AcceptedSubmissionExecutionDisposition, AcceptedSubmissionCommitError> {
        if context.tenant_id() != claim.tenant {
            return Err(StoreError::Conflict.into());
        }
        match outcome {
            AcceptedSubmissionExecutionOutcome::Evaluated { grade } => {
                self.complete_accepted_submission_execution(context, claim, grade)
                    .await
            }
            outcome => {
                self.fail_accepted_submission_execution(context, claim, outcome)
                    .await
            }
        }
    }
}

impl PostgresAcceptedSubmissionExecutionStore {
    async fn complete_accepted_submission_execution(
        &self,
        context: TenantContext,
        claim: AcceptedSubmissionExecutionClaim,
        grade: crate::AcceptedSubmissionGrade,
    ) -> Result<AcceptedSubmissionExecutionDisposition, AcceptedSubmissionCommitError> {
        let expected_evidence = crate::canonical_attempt_result_json(grade.evidence.result)?;
        if grade.evidence.canonical_json_version != expected_evidence.canonical_json_version
            || grade.evidence.canonical_json != expected_evidence.canonical_json
            || grade.evidence.sha256 != expected_evidence.sha256
        {
            return Err(StoreError::InvalidRecord(
                "automated result evidence is not canonical".to_string(),
            )
            .into());
        }
        let generation = execution_generation_i64(claim)?;
        let mut transaction = self.begin_execution_tenant(context).await?;
        // ASVS 2.3.1/2.3.3: this locks the exact leased execution after the
        // grader has returned. No row lock spans grader I/O.
        let row = sqlx::query(
            "SELECT * FROM public.ple_lock_accepted_submission_completion_v1($1,$2,$3,$4,$5,$6)",
        )
        .bind(claim.tenant.as_uuid())
        .bind(claim.job.as_uuid())
        .bind(claim.lease_token.as_uuid())
        .bind(claim.submission.as_uuid())
        .bind(generation)
        .bind(claim.worker.as_uuid())
        .fetch_optional(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;
        let Some(row) = row else {
            transaction
                .commit()
                .await
                .map_err(|_| AcceptedSubmissionCommitError::OutcomeUnknown)?;
            return Ok(AcceptedSubmissionExecutionDisposition::ClaimNoLongerActive);
        };
        verify_locked_claim(&row, claim)?;
        let source = decode_locked_completion_source(&row, claim, grade)?;
        let assignment_item = source
            .input
            .run_items
            .iter()
            .find(|item| item.issued_position == source.input.base_attempt.assignment_position)
            .map(|item| item.assignment_item)
            .ok_or_else(|| {
                StoreError::Unavailable(
                    "completion lock omits the accepted assignment item".to_string(),
                )
            })?;
        let plan = crate::plan_accepted_submission_completion(source.input)?;
        // Receipt evidence has one source-byte authority.  The established
        // mutable projections deliberately retain their legacy payload
        // checksums, so prepare both representations before the atomic v2
        // commit capability.
        let receipt_attempt =
            super::submission::encode_receipt_attempt_snapshot(&plan.receipt.attempt)?;
        let receipt_run = super::submission::encode_receipt_snapshot(
            "submission receipt run",
            &plan.receipt.run,
        )?;
        let receipt_summary = super::submission::encode_receipt_snapshot(
            "submission receipt summary",
            &plan.receipt.summary,
        )?;
        let receipt_feedback = super::submission::encode_feedback_snapshot(&plan.receipt.feedback)?;
        let receipt_presentation = plan
            .receipt
            .presentation
            .as_ref()
            .map(|presentation| {
                super::submission::encode_receipt_snapshot(
                    "submission receipt presentation",
                    presentation,
                )
            })
            .transpose()?;
        let attempt_current_projection = encode_current_projection(&plan.receipt.attempt)?;
        let run_current_projection = encode_current_projection(&plan.receipt.run)?;
        if receipt_attempt.version != expected_evidence.canonical_json_version
            || receipt_feedback.version != expected_evidence.canonical_json_version
            || receipt_run.version != expected_evidence.canonical_json_version
            || receipt_summary.version != expected_evidence.canonical_json_version
            || receipt_presentation
                .as_ref()
                .is_some_and(|value| value.version != expected_evidence.canonical_json_version)
        {
            return Err(StoreError::InvalidRecord(
                "completion evidence uses mixed canonical JSON versions".to_string(),
            )
            .into());
        }
        if receipt_attempt.projection != attempt_current_projection.projection
            || receipt_run.projection != run_current_projection.projection
        {
            return Err(StoreError::InvalidRecord(
                "receipt evidence disagrees with its mutable current projection".to_string(),
            )
            .into());
        }
        let statistics = encode_statistics(&plan.statistics)?;
        let recalculation_job = crate::JobId::from_uuid(Uuid::from_u128(
            claim.submission.as_uuid().as_u128() ^ u128::MAX,
        ));
        let row = sqlx::query(
            "SELECT * FROM public.ple_commit_accepted_submission_completion_v2(\
             $1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15,$16,$17,$18,$19,\
             $20,$21,$22,$23,$24,$25,$26,$27,$28,$29,$30,$31,$32,$33,$34,$35,$36,$37,$38)",
        )
        .bind(claim.tenant.as_uuid())
        .bind(claim.job.as_uuid())
        .bind(claim.lease_token.as_uuid())
        .bind(claim.submission.as_uuid())
        .bind(generation)
        .bind(claim.worker.as_uuid())
        .bind(
            i16::try_from(expected_evidence.canonical_json_version).map_err(|_| {
                StoreError::InvalidRecord("canonical JSON version is invalid".to_string())
            })?,
        )
        .bind("graded")
        .bind(expected_evidence.canonical_json)
        .bind(expected_evidence.sha256.to_string())
        .bind(receipt_attempt.source)
        .bind(Json(receipt_attempt.projection))
        .bind(receipt_attempt.sha256.to_string())
        .bind(attempt_current_projection.source)
        .bind(attempt_current_projection.sha256)
        .bind(receipt_feedback.source)
        .bind(receipt_feedback.sha256.to_string())
        .bind(receipt_run.source)
        .bind(Json(receipt_run.projection))
        .bind(receipt_run.sha256.to_string())
        .bind(run_current_projection.source)
        .bind(run_current_projection.sha256)
        .bind(
            plan.receipt
                .run
                .completed_at
                .map(|value| value.as_unix_millis()),
        )
        .bind(
            plan.enrollment
                .first_completed_at
                .map(|value| value.as_unix_millis()),
        )
        .bind(plan.enrollment.current_grade_run.map(|run| run.as_uuid()))
        .bind(plan.enrollment.best_grade_run.map(|run| run.as_uuid()))
        .bind(receipt_summary.source)
        .bind(Json(receipt_summary.projection))
        .bind(receipt_summary.sha256.to_string())
        .bind(
            receipt_presentation
                .as_ref()
                .map(|value| value.source.clone()),
        )
        .bind(
            receipt_presentation
                .as_ref()
                .map(|value| Json(value.projection.clone())),
        )
        .bind(
            receipt_presentation
                .as_ref()
                .map(|value| value.sha256.to_string()),
        )
        .bind(receipt_presentation.is_some())
        .bind(assignment_item.as_uuid())
        .bind(statistics)
        .bind(source.scoring_generation)
        .bind(recalculation_job.as_uuid())
        .bind(10_i32)
        .fetch_optional(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;
        let disposition = decode_worker_disposition(row.as_ref())?;
        transaction
            .commit()
            .await
            .map_err(|_| AcceptedSubmissionCommitError::OutcomeUnknown)?;
        Ok(disposition)
    }

    async fn fail_accepted_submission_execution(
        &self,
        context: TenantContext,
        claim: AcceptedSubmissionExecutionClaim,
        outcome: AcceptedSubmissionExecutionOutcome,
    ) -> Result<AcceptedSubmissionExecutionDisposition, AcceptedSubmissionCommitError> {
        let (failure_kind, reason) = match outcome {
            AcceptedSubmissionExecutionOutcome::DeterministicFailure { reason } => {
                ("deterministic", Some(reason_name(reason)))
            }
            AcceptedSubmissionExecutionOutcome::TransientFailure => ("transient", None),
            AcceptedSubmissionExecutionOutcome::TimedOut => ("timed_out", None),
            AcceptedSubmissionExecutionOutcome::TerminalFailure => ("terminal", None),
            AcceptedSubmissionExecutionOutcome::Evaluated { .. } => {
                return Err(StoreError::InvalidRecord(
                    "evaluated outcome requires the completion capability".to_string(),
                )
                .into());
            }
        };
        let generation = execution_generation_i64(claim)?;
        let mut transaction = self.begin_execution_tenant(context).await?;
        let row = sqlx::query(
            "SELECT * FROM public.ple_fail_accepted_submission_execution_v1($1,$2,$3,$4,$5,$6,$7,$8)",
        )
        .bind(claim.tenant.as_uuid())
        .bind(claim.job.as_uuid())
        .bind(claim.lease_token.as_uuid())
        .bind(claim.submission.as_uuid())
        .bind(generation)
        .bind(claim.worker.as_uuid())
        .bind(failure_kind)
        .bind(reason)
        .fetch_optional(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;
        let disposition = decode_worker_disposition(row.as_ref())?;
        transaction
            .commit()
            .await
            .map_err(|_| AcceptedSubmissionCommitError::OutcomeUnknown)?;
        Ok(disposition)
    }
}

fn decode_worker_claim(
    row: &sqlx::postgres::PgRow,
    expected_worker: WorkerId,
    expected_lease: JobLeaseToken,
) -> Result<AcceptedSubmissionExecutionClaim, StoreError> {
    let returned_worker: Uuid = row.try_get("worker_id").map_err(map_sqlx_error)?;
    let returned_lease: Uuid = row.try_get("worker_lease_token").map_err(map_sqlx_error)?;
    if returned_worker != expected_worker.as_uuid() || returned_lease != expected_lease.as_uuid() {
        return Err(StoreError::Unavailable(
            "execution-claim broker disagrees with the worker capability".to_string(),
        ));
    }
    let generation = row
        .try_get::<i64, _>("execution_generation")
        .map_err(map_sqlx_error)?;
    Ok(AcceptedSubmissionExecutionClaim {
        tenant: TenantId::from_uuid(row.try_get("tenant_id").map_err(map_sqlx_error)?),
        job: crate::JobId::from_uuid(row.try_get("worker_job_id").map_err(map_sqlx_error)?),
        lease_token: JobLeaseToken::from_uuid(returned_lease),
        submission: AcceptedSubmissionId::from_uuid(
            row.try_get("submission_id").map_err(map_sqlx_error)?,
        ),
        execution_generation: GradingExecutionGeneration::from_u64(
            u64::try_from(generation).map_err(|_| {
                StoreError::InvalidRecord("invalid claimed execution generation".to_string())
            })?,
        )
        .ok_or_else(|| {
            StoreError::InvalidRecord("invalid claimed execution generation".to_string())
        })?,
        worker: expected_worker,
    })
}

fn reason_name(reason: GradingOperationReason) -> &'static str {
    match reason {
        GradingOperationReason::GraderContractFailure => "grader_contract_failure",
        GradingOperationReason::GraderExecutionFailure => "grader_execution_failure",
        GradingOperationReason::IssuedEvidenceIntegrity => "issued_evidence_integrity",
        GradingOperationReason::RetryExhausted => "retry_exhausted",
        GradingOperationReason::ScoringRecalculationFailed => "scoring_recalculation_failed",
    }
}

fn decode_worker_disposition(
    row: Option<&sqlx::postgres::PgRow>,
) -> Result<AcceptedSubmissionExecutionDisposition, StoreError> {
    let Some(row) = row else {
        return Ok(AcceptedSubmissionExecutionDisposition::ClaimNoLongerActive);
    };
    let disposition: String = row.try_get("disposition").map_err(map_sqlx_error)?;
    let state: Option<String> = row
        .try_get("resulting_execution_state")
        .map_err(map_sqlx_error)?;
    let evaluation: Option<String> = row
        .try_get("resulting_evaluation_status")
        .map_err(map_sqlx_error)?;
    parse_worker_disposition(&disposition, state.as_deref(), evaluation.as_deref())
}

fn parse_worker_disposition(
    disposition: &str,
    state: Option<&str>,
    evaluation: Option<&str>,
) -> Result<AcceptedSubmissionExecutionDisposition, StoreError> {
    match (disposition, state, evaluation) {
        ("committed", Some("completed"), Some("graded")) => {
            Ok(AcceptedSubmissionExecutionDisposition::Committed)
        }
        ("rescheduled", Some("retry_wait"), Some("automated_pending")) => {
            Ok(AcceptedSubmissionExecutionDisposition::Rescheduled)
        }
        ("terminal", Some("exception"), Some("automated_exception")) => {
            Ok(AcceptedSubmissionExecutionDisposition::Terminal)
        }
        ("claim_no_longer_active", None, None) => {
            Ok(AcceptedSubmissionExecutionDisposition::ClaimNoLongerActive)
        }
        _ => Err(StoreError::Unavailable(
            "accepted-submission worker capability returned an incoherent disposition".to_string(),
        )),
    }
}

fn decode_accepted_submission_row(
    row: &sqlx::postgres::PgRow,
) -> Result<AcceptedSubmission, StoreError> {
    let request_sha256: objects::Sha256Digest = serde_json::from_value(serde_json::Value::String(
        row.try_get("accepted_request_sha256")
            .map_err(map_sqlx_error)?,
    ))
    .map_err(|_| {
        StoreError::InvalidRecord("invalid stored accepted-submission digest".to_string())
    })?;
    Ok(AcceptedSubmission {
        tenant: TenantId::from_uuid(row.try_get("accepted_tenant_id").map_err(map_sqlx_error)?),
        course: CourseId::from_uuid(row.try_get("accepted_course_id").map_err(map_sqlx_error)?),
        assignment: AssignmentId::from_uuid(
            row.try_get("accepted_assignment_id")
                .map_err(map_sqlx_error)?,
        ),
        attempt: QuestionAttemptId::from_uuid(
            row.try_get("accepted_attempt_id").map_err(map_sqlx_error)?,
        ),
        submission: AcceptedSubmissionId::from_uuid(
            row.try_get("accepted_submission_id")
                .map_err(map_sqlx_error)?,
        ),
        actor: UserId::from_uuid(row.try_get("accepted_actor_id").map_err(map_sqlx_error)?),
        idempotency_key: SubmissionIdempotencyKey::parse(
            &row.try_get::<String, _>("accepted_idempotency_key")
                .map_err(map_sqlx_error)?,
        )
        .map_err(|_| StoreError::InvalidRecord("invalid stored idempotency key".to_string()))?,
        request_sha256,
        accepted_at: ActivityTimestamp::from_unix_millis(
            row.try_get("accepted_millis").map_err(map_sqlx_error)?,
        ),
    })
}

fn decode_execution(row: sqlx::postgres::PgRow) -> Result<GradingExecution, StoreError> {
    let generation: i64 = row
        .try_get("execution_generation")
        .map_err(map_sqlx_error)?;
    let retry_count: i32 = row.try_get("retry_count").map_err(map_sqlx_error)?;
    let state: String = row.try_get("state").map_err(map_sqlx_error)?;
    let state = match state.as_str() {
        "ready" => GradingExecutionState::Ready,
        "running" => GradingExecutionState::Running,
        "completed" => GradingExecutionState::Completed,
        "exception" => GradingExecutionState::Exception,
        "retry_wait" => GradingExecutionState::RetryWait,
        "superseded" => GradingExecutionState::Superseded,
        _ => {
            return Err(StoreError::InvalidRecord(
                "invalid grading execution state".to_string(),
            ));
        }
    };
    Ok(GradingExecution {
        submission: AcceptedSubmissionId::from_uuid(
            row.try_get("submission_id").map_err(map_sqlx_error)?,
        ),
        generation: GradingExecutionGeneration::from_u64(u64::try_from(generation).map_err(
            |_| StoreError::InvalidRecord("invalid grading execution generation".to_string()),
        )?)
        .ok_or_else(|| {
            StoreError::InvalidRecord("invalid grading execution generation".to_string())
        })?,
        state,
        job: crate::JobId::from_uuid(row.try_get("current_job_id").map_err(map_sqlx_error)?),
        retry_count: u16::try_from(retry_count).map_err(|_| {
            StoreError::InvalidRecord("invalid grading execution retry count".to_string())
        })?,
    })
}

fn decode_operation(
    tenant: TenantId,
    course: CourseId,
    reference: GradingOperationReference,
    row: &sqlx::postgres::PgRow,
) -> Result<GradingOperation, StoreError> {
    let target_kind = row
        .try_get::<String, _>("target_kind")
        .map_err(map_sqlx_error)?;
    let target = match target_kind.as_str() {
        "submission" => GradingOperationTarget::SubmissionRecovery {
            submission: row
                .try_get::<Option<Uuid>, _>("submission_id")
                .map_err(map_sqlx_error)?
                .map(AcceptedSubmissionId::from_uuid)
                .ok_or_else(|| {
                    StoreError::InvalidRecord("submission target is missing its source".to_string())
                })?,
        },
        "assignment_scoring_generation" => GradingOperationTarget::AssignmentScoringGeneration {
            requested_generation: question_model::ScoringGeneration::new(
                u64::try_from(
                    row.try_get::<Option<i64>, _>("requested_scoring_generation")
                        .map_err(map_sqlx_error)?
                        .ok_or_else(|| {
                            StoreError::InvalidRecord(
                                "scoring target is missing its generation".to_string(),
                            )
                        })?,
                )
                .map_err(|_| StoreError::InvalidRecord("invalid scoring generation".to_string()))?,
            )
            .ok_or_else(|| StoreError::InvalidRecord("invalid scoring generation".to_string()))?,
        },
        _ => {
            return Err(StoreError::InvalidRecord(
                "invalid grading operation target".to_string(),
            ));
        }
    };
    let reason = match row
        .try_get::<String, _>("reason")
        .map_err(map_sqlx_error)?
        .as_str()
    {
        "grader_contract_failure" => GradingOperationReason::GraderContractFailure,
        "grader_execution_failure" => GradingOperationReason::GraderExecutionFailure,
        "issued_evidence_integrity" => GradingOperationReason::IssuedEvidenceIntegrity,
        "retry_exhausted" => GradingOperationReason::RetryExhausted,
        "scoring_recalculation_failed" => GradingOperationReason::ScoringRecalculationFailed,
        _ => {
            return Err(StoreError::InvalidRecord(
                "invalid grading operation reason".to_string(),
            ));
        }
    };
    let state = match row
        .try_get::<String, _>("state")
        .map_err(map_sqlx_error)?
        .as_str()
    {
        "actionable" => GradingOperationState::Actionable,
        "action_in_progress" => GradingOperationState::ActionInProgress,
        "completed" => GradingOperationState::Completed,
        "repair_required" => GradingOperationState::RepairRequired,
        "failed" => GradingOperationState::Failed,
        "superseded" => GradingOperationState::Superseded,
        _ => {
            return Err(StoreError::InvalidRecord(
                "invalid grading operation state".to_string(),
            ));
        }
    };
    let next_action = match row
        .try_get::<Option<String>, _>("next_action")
        .map_err(map_sqlx_error)?
        .as_deref()
    {
        None => None,
        Some("retry") => Some(GradingOperationAction::Retry),
        Some("recalculate") => Some(GradingOperationAction::Recalculate),
        Some(_) => {
            return Err(StoreError::InvalidRecord(
                "invalid grading operation action".to_string(),
            ));
        }
    };
    let revision: i64 = row.try_get("revision").map_err(map_sqlx_error)?;
    Ok(GradingOperation {
        tenant,
        course,
        assignment: AssignmentId::from_uuid(row.try_get("assignment_id").map_err(map_sqlx_error)?),
        reference,
        target,
        reason,
        state,
        revision: GradingOperationRevision::from_u64(u64::try_from(revision).map_err(|_| {
            StoreError::InvalidRecord("invalid grading operation revision".to_string())
        })?)
        .ok_or_else(|| {
            StoreError::InvalidRecord("invalid grading operation revision".to_string())
        })?,
        next_action,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dedicated_worker_handle_owns_the_private_execution_capability() {
        fn requires_execution_store<T: AcceptedSubmissionExecutionStore>() {}
        fn requires_worker_store<T: AcceptedSubmissionExecutionWorkerStore>() {}

        requires_execution_store::<PostgresAcceptedSubmissionExecutionStore>();
        requires_worker_store::<PostgresAcceptedSubmissionExecutionStore>();
    }

    #[test]
    fn worker_disposition_is_closed() {
        assert_eq!(
            parse_worker_disposition("claim_no_longer_active", None, None)
                .expect("known disposition"),
            AcceptedSubmissionExecutionDisposition::ClaimNoLongerActive
        );
        assert_eq!(
            parse_worker_disposition("committed", Some("completed"), Some("graded"))
                .expect("complete graded result"),
            AcceptedSubmissionExecutionDisposition::Committed
        );
        assert!(parse_worker_disposition("committed", Some("completed"), Some("exempt")).is_err());
        assert_eq!(
            parse_worker_disposition("rescheduled", Some("retry_wait"), Some("automated_pending"),)
                .expect("retry result"),
            AcceptedSubmissionExecutionDisposition::Rescheduled
        );
        assert_eq!(
            parse_worker_disposition("terminal", Some("exception"), Some("automated_exception"),)
                .expect("terminal result"),
            AcceptedSubmissionExecutionDisposition::Terminal
        );
        assert!(parse_worker_disposition("committed", Some("retry_wait"), Some("graded")).is_err());
        assert!(parse_worker_disposition("rescheduled", Some("retry_wait"), None).is_err());
    }
}

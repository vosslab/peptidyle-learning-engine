//! Active-attempt policy re-resolution after authoritative policy mutations.

use crate::assignment_revision_from_stored;
use crate::postgres::*;
use crate::*;
use domain::effective_assignment_policy::{AssignmentLifecycleGate, assignment_lifecycle_gate};
use question_model::{ActivityTimestamp, AssignmentId, CourseId, StudentId, TenantId};
use sqlx::{Postgres, Row, Transaction};

const ACTIVE_ATTEMPT_RECALCULATION_SELECT: &str = concat!(
    "SELECT qa.attempt_id, ",
    "floor(extract(epoch FROM qa.authored_timing_deadline)*1000)::bigint AS ",
    "authored_timing_deadline, qa.authored_timing_grace_seconds::bigint AS ",
    "authored_timing_grace_seconds, enrollment.student_id, run.run_number, ",
    "floor(extract(epoch FROM run.started_at)*1000)::bigint AS run_started_at ",
    "FROM question_attempt qa JOIN assignment_run run ON run.tenant_id=qa.tenant_id ",
    "AND run.run_id=qa.run_id JOIN enrollment enrollment ON ",
    "enrollment.tenant_id=run.tenant_id AND enrollment.enrollment_id=run.enrollment_id ",
    "WHERE qa.tenant_id=$1 AND qa.attempt_id = ANY($2) ORDER BY qa.attempt_id",
);

/// Broker-prelocked, opaque active-attempt scope for one assignment revision.
/// The UUIDs are validated before any learner or payload hydration.
pub(crate) struct ActiveAttemptPrepareWitness {
    lifecycle: question_model::AssignmentLifecycle,
    revision: AssignmentRevision,
    attempt_ids: Vec<sqlx::types::Uuid>,
}

impl ActiveAttemptPrepareWitness {
    pub(crate) fn decode(row: &sqlx::postgres::PgRow) -> Result<Self, StoreError> {
        let count: i64 = row
            .try_get("active_attempt_count")
            .map_err(map_sqlx_error)?;
        let attempt_ids: Vec<sqlx::types::Uuid> =
            row.try_get("active_attempt_ids").map_err(map_sqlx_error)?;
        validate_ordered_attempt_ids(count, &attempt_ids)?;
        Ok(Self {
            lifecycle: parse_assignment_lifecycle(
                &row.try_get::<String, _>("assignment_lifecycle")
                    .map_err(map_sqlx_error)?,
            )?,
            revision: assignment_revision_from_stored(
                row.try_get("assignment_revision").map_err(map_sqlx_error)?,
            )?,
            attempt_ids,
        })
    }
    pub(crate) fn require_revision(&self, expected: AssignmentRevision) -> Result<(), StoreError> {
        (self.revision == expected)
            .then_some(())
            .ok_or(StoreError::Conflict)
    }

    fn lifecycle(&self) -> question_model::AssignmentLifecycle {
        self.lifecycle
    }

    fn attempt_ids(&self) -> &[sqlx::types::Uuid] {
        &self.attempt_ids
    }
}

fn validate_ordered_attempt_ids(
    count: i64,
    attempt_ids: &[sqlx::types::Uuid],
) -> Result<(), StoreError> {
    let expected = usize::try_from(count).map_err(|_| StoreError::Conflict)?;
    if expected != attempt_ids.len() || attempt_ids.windows(2).any(|ids| ids[0] >= ids[1]) {
        return Err(StoreError::Conflict);
    }
    Ok(())
}

pub(crate) fn assignment_lifecycle_name(
    value: question_model::AssignmentLifecycle,
) -> &'static str {
    match value {
        question_model::AssignmentLifecycle::Draft => "draft",
        question_model::AssignmentLifecycle::Published => "published",
        question_model::AssignmentLifecycle::Closed => "closed",
        question_model::AssignmentLifecycle::Archived => "archived",
    }
}
pub(crate) fn parse_assignment_lifecycle(
    value: &str,
) -> Result<question_model::AssignmentLifecycle, StoreError> {
    match value {
        "draft" => Ok(question_model::AssignmentLifecycle::Draft),
        "published" => Ok(question_model::AssignmentLifecycle::Published),
        "closed" => Ok(question_model::AssignmentLifecycle::Closed),
        "archived" => Ok(question_model::AssignmentLifecycle::Archived),
        _ => Err(StoreError::Unavailable(
            "stored assignment lifecycle is invalid".into(),
        )),
    }
}
pub(crate) fn inert_inputs() -> Result<super::EffectivePolicyInputs, StoreError> {
    Ok(super::EffectivePolicyInputs {
        base: domain::effective_assignment_policy::BaseAssignmentPolicy {
            available_at: None,
            due_at: None,
            closes_at: None,
            time_limit_seconds: None,
            attempt_limit: None,
            late_submission: question_model::LateSubmissionPolicy::Accept,
            deadline_behavior: question_model::AssignmentDeadlineBehavior::AutoSubmit,
        },
        schedule_offsets: vec![],
        accommodations: vec![],
        individual: None,
    })
}

/// Recomputes the mutable effects for exactly the attempts locked and returned
/// by the broker. S5 is evaluated afresh from current learner authority;
/// historical enrollment and prior receipts never stand in for current facts.
pub(crate) async fn reresolve_prelocked_active_attempts(
    tx: &mut Transaction<'_, Postgres>,
    tenant: TenantId,
    course: CourseId,
    assignment: AssignmentId,
    revision: AssignmentRevision,
    witness: ActiveAttemptPrepareWitness,
) -> Result<(), StoreError> {
    witness.require_revision(revision)?;
    verify_prelocked_attempt_scope(tx, tenant, course, assignment, &witness).await?;
    let lifecycle_gate = assignment_lifecycle_gate(witness.lifecycle());
    let attempts = sqlx::query(ACTIVE_ATTEMPT_RECALCULATION_SELECT)
        .bind(tenant.as_uuid())
        .bind(witness.attempt_ids().to_vec())
        .fetch_all(&mut **tx)
        .await
        .map_err(map_sqlx_error)?;
    if attempts.len() != witness.attempt_ids().len() {
        return Err(StoreError::Conflict);
    }
    // Validate every immutable timing baseline before this policy mutation can
    // append a receipt, replace a pointer, schedule/cancel a job, or change a
    // lifecycle. Snapshot evidence establishes issuance, but re-resolution is
    // deliberately independent of it and of the current catalog.
    for row in &attempts {
        crate::postgres::runs::authored_timing::decode_authored_attempt_timing(row)?;
    }
    for row in attempts {
        let attempt =
            QuestionAttemptId::from_uuid(row.try_get("attempt_id").map_err(map_sqlx_error)?);
        let student = StudentId::from_uuid(row.try_get("student_id").map_err(map_sqlx_error)?);
        let current = sqlx::query("SELECT receipt_generation,timing_generation FROM attempt_effective_policy_current WHERE tenant_id=$1 AND attempt_id=$2 FOR UPDATE")
            .bind(tenant.as_uuid()).bind(attempt.as_uuid()).fetch_optional(&mut **tx).await.map_err(map_sqlx_error)?.ok_or_else(|| StoreError::Unavailable("active attempt is missing its effective-policy pointer".to_string()))?;
        let entitlement =
            crate::postgres::entitlement::evaluate_current_student_broker_prelocked_current_facts(
                tx, tenant, course, assignment, student,
            )
            .await?;
        let allowed = match entitlement {
            Some(domain::entitlement::EntitlementDecision::Granted(grant))
                if grant.student() == student
                    && matches!(lifecycle_gate, AssignmentLifecycleGate::Open) =>
            {
                let prior = u32::try_from(
                    row.try_get::<i64, _>("run_number")
                        .map_err(map_sqlx_error)?,
                )
                .map_err(|_| StoreError::Conflict)?
                .saturating_sub(1);
                let (decision, _) = super::resolve_granted_effective_policy_read_only(
                    tx,
                    grant,
                    domain::effective_assignment_policy::AuthorizationGate::Authorized,
                    prior,
                )
                .await?;
                match decision {
                    domain::effective_assignment_policy::EffectivePolicyDecision::Allowed {
                        policy,
                        start: domain::effective_assignment_policy::StartVerdict::MayStart { .. },
                    } => Some(policy),
                    domain::effective_assignment_policy::EffectivePolicyDecision::Allowed {
                        ..
                    }
                    | domain::effective_assignment_policy::EffectivePolicyDecision::Denied {
                        ..
                    } => None,
                }
            }
            _ => None,
        };
        let old_generation: i64 = current
            .try_get("receipt_generation")
            .map_err(map_sqlx_error)?;
        let old_timing: i64 = current
            .try_get("timing_generation")
            .map_err(map_sqlx_error)?;
        let Some(policy) = allowed else {
            // The established terminal transition removes only mutable effect
            // state; immutable receipts remain historical evidence.
            crate::postgres::assignment_timing::cancel_postgres_effective_policy_job(
                tx, tenant, attempt,
            )
            .await?;
            sqlx::query("UPDATE question_attempt SET attempt_status='auto_submitted', submitted_at=transaction_timestamp() WHERE tenant_id=$1 AND attempt_id=$2 AND attempt_status='in_progress'").bind(tenant.as_uuid()).bind(attempt.as_uuid()).execute(&mut **tx).await.map_err(map_sqlx_error)?;
            sqlx::query(
                "DELETE FROM attempt_effective_policy_current WHERE tenant_id=$1 AND attempt_id=$2",
            )
            .bind(tenant.as_uuid())
            .bind(attempt.as_uuid())
            .execute(&mut **tx)
            .await
            .map_err(map_sqlx_error)?;
            continue;
        };
        let authored =
            crate::postgres::runs::authored_timing::decode_authored_attempt_timing(&row)?;
        let timing = crate::postgres::assignment_timing::resolved_postgres_attempt_timing(
            &policy,
            ActivityTimestamp::from_unix_millis(
                row.try_get("run_started_at").map_err(map_sqlx_error)?,
            ),
            authored.deadline,
            authored.grace_seconds,
        )?;
        let now = crate::postgres::database_timestamp(tx).await?;
        if timing
            .effective_deadline
            .is_some_and(|deadline| deadline < now)
            || timing
                .auto_submit_at
                .is_some_and(|deadline| deadline <= now)
        {
            crate::postgres::assignment_timing::cancel_postgres_effective_policy_job(
                tx, tenant, attempt,
            )
            .await?;
            sqlx::query("UPDATE question_attempt SET attempt_status='auto_submitted', submitted_at=transaction_timestamp() WHERE tenant_id=$1 AND attempt_id=$2 AND attempt_status='in_progress'")
                .bind(tenant.as_uuid()).bind(attempt.as_uuid()).execute(&mut **tx).await.map_err(map_sqlx_error)?;
            sqlx::query(
                "DELETE FROM attempt_effective_policy_current WHERE tenant_id=$1 AND attempt_id=$2",
            )
            .bind(tenant.as_uuid())
            .bind(attempt.as_uuid())
            .execute(&mut **tx)
            .await
            .map_err(map_sqlx_error)?;
            continue;
        }
        let next_generation = old_generation.checked_add(1).ok_or(StoreError::Conflict)?;
        crate::postgres::effective_policy_receipts::append_sealed_effective_policy_receipt(
            tx,
            crate::postgres::effective_policy_receipts::EffectivePolicyReceiptWrite {
                tenant,
                attempt,
                assignment,
                course,
                generation: next_generation,
                policy: &policy,
                effective_deadline: timing.effective_deadline,
                effective_grace_seconds: timing.effective_grace_seconds,
                auto_submit_at: timing.auto_submit_at,
                revision,
            },
        )
        .await?;
        crate::postgres::assignment_timing::cancel_postgres_effective_policy_job(
            tx, tenant, attempt,
        )
        .await?;
        let next_timing = old_timing.checked_add(1).ok_or(StoreError::Conflict)?;
        let job =
            schedule_effective_policy_job(tx, tenant, attempt, next_timing, timing.auto_submit_at)
                .await?;
        sqlx::query("UPDATE attempt_effective_policy_current SET receipt_generation=$3,timing_generation=$4,job_id=$5,updated_at=transaction_timestamp() WHERE tenant_id=$1 AND attempt_id=$2")
            .bind(tenant.as_uuid()).bind(attempt.as_uuid()).bind(next_generation).bind(next_timing).bind(job.map(JobId::as_uuid)).execute(&mut **tx).await.map_err(map_sqlx_error)?;
    }
    Ok(())
}

/// Validates the opaque broker witness before learner identity or payload
/// hydration. The query is deliberately plain: the broker already owns the
/// assignment and exact attempt locks for this transaction.
async fn verify_prelocked_attempt_scope(
    tx: &mut Transaction<'_, Postgres>,
    tenant: TenantId,
    course: CourseId,
    assignment: AssignmentId,
    witness: &ActiveAttemptPrepareWitness,
) -> Result<(), StoreError> {
    let rows = sqlx::query(
        "SELECT qa.attempt_id, qa.attempt_status, qa.course_id, enrollment.assignment_id FROM question_attempt qa JOIN assignment_run run ON run.tenant_id=qa.tenant_id AND run.run_id=qa.run_id JOIN enrollment enrollment ON enrollment.tenant_id=run.tenant_id AND enrollment.enrollment_id=run.enrollment_id WHERE qa.tenant_id=$1 AND qa.attempt_id = ANY($2) ORDER BY qa.attempt_id",
    )
    .bind(tenant.as_uuid())
    .bind(witness.attempt_ids().to_vec())
    .fetch_all(&mut **tx)
    .await
    .map_err(map_sqlx_error)?;
    if rows.len() != witness.attempt_ids().len() {
        return Err(StoreError::Conflict);
    }
    for (row, expected_attempt) in rows.iter().zip(witness.attempt_ids()) {
        let attempt: sqlx::types::Uuid = row.try_get("attempt_id").map_err(map_sqlx_error)?;
        let status: String = row.try_get("attempt_status").map_err(map_sqlx_error)?;
        let stored_course: sqlx::types::Uuid = row.try_get("course_id").map_err(map_sqlx_error)?;
        let stored_assignment: sqlx::types::Uuid =
            row.try_get("assignment_id").map_err(map_sqlx_error)?;
        if attempt != *expected_attempt
            || status != "in_progress"
            || stored_course != course.as_uuid()
            || stored_assignment != assignment.as_uuid()
        {
            return Err(StoreError::Conflict);
        }
    }
    Ok(())
}
async fn schedule_effective_policy_job(
    tx: &mut Transaction<'_, Postgres>,
    tenant: TenantId,
    attempt: QuestionAttemptId,
    timing_generation: i64,
    auto_submit_at: Option<ActivityTimestamp>,
) -> Result<Option<JobId>, StoreError> {
    let Some(at) = auto_submit_at else {
        return Ok(None);
    };
    let job = JobId::generate()?;
    let payload = serde_json::to_value(JobPayload::AutoSubmitAttempt {
        attempt,
        timing_generation: u64::try_from(timing_generation).map_err(|_| StoreError::Conflict)?,
    })
    .map_err(|error| StoreError::InvalidRecord(error.to_string()))?;
    sqlx::query("INSERT INTO worker_job (job_id,tenant_id,payload,state,available_at,max_attempts) VALUES ($1,$2,$3,'ready',TIMESTAMPTZ 'epoch' + $4::bigint * INTERVAL '1 millisecond',10)").bind(job.as_uuid()).bind(tenant.as_uuid()).bind(payload).bind(at.as_unix_millis()).execute(&mut **tx).await.map_err(map_sqlx_error)?;
    Ok(Some(job))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn witness_ids_require_a_nonnegative_sorted_unique_exact_count() {
        let first = sqlx::types::Uuid::parse_str("00000000-0000-0000-0000-000000000001")
            .expect("fixed UUID is valid");
        let second = sqlx::types::Uuid::parse_str("00000000-0000-0000-0000-000000000002")
            .expect("fixed UUID is valid");
        assert!(validate_ordered_attempt_ids(2, &[first, second]).is_ok());
        assert!(validate_ordered_attempt_ids(-1, &[]).is_err());
        assert!(validate_ordered_attempt_ids(1, &[first, second]).is_err());
        assert!(validate_ordered_attempt_ids(2, &[second, first]).is_err());
        assert!(validate_ordered_attempt_ids(2, &[first, first]).is_err());
    }

    #[test]
    fn active_attempt_recalculation_reads_only_timing_baseline_and_live_scope() {
        for forbidden in [
            "issued_question_snapshot",
            "problem_id",
            "version_id",
            "question_definition",
        ] {
            assert!(
                !ACTIVE_ATTEMPT_RECALCULATION_SELECT.contains(forbidden),
                "active policy recalculation must not hydrate {forbidden}",
            );
        }
        assert!(ACTIVE_ATTEMPT_RECALCULATION_SELECT.contains("authored_timing_deadline"));
        assert!(ACTIVE_ATTEMPT_RECALCULATION_SELECT.contains("authored_timing_grace_seconds"));
    }
}

//! PostgreSQL ownership of the normalized effective-policy inputs.

use std::num::NonZeroU32;

use async_trait::async_trait;
use domain::effective_assignment_policy::{
    AssignmentLifecycleGate, BaseAssignmentPolicy, GroupAccommodation, GroupScheduleOffset,
    IndividualPolicyException, PolicyModificationMode, PolicyPatch, PolicyPatchSet,
    ResolveEffectivePolicyInput, ScheduleOffsetSeconds, assignment_lifecycle_gate,
    resolve_effective_policy, validate_base_assignment_policy_for_course_term,
};
use question_model::{
    ActivityTimestamp, AssignmentDeadlineBehavior, AssignmentId, CourseGroupId, CourseId,
    CourseTerm, LateSubmissionPolicy, StudentId, TenantId,
};
use sqlx::types::Uuid;
use sqlx::{Postgres, Row, Transaction};

use super::*;
use crate::*;

mod preview_resolution;

/// Recomputes the mutable effect of every active attempt while the caller
/// already holds the assignment policy lock.  S5 is evaluated afresh for each
/// attempt's current learner; historical enrollment and prior receipts never
/// stand in for current authority.
pub(super) async fn reresolve_active_attempts(
    tx: &mut Transaction<'_, Postgres>,
    tenant: TenantId,
    course: CourseId,
    assignment: AssignmentId,
    revision: AssignmentRevision,
) -> Result<(), StoreError> {
    let lifecycle: String = sqlx::query_scalar(
        "SELECT lifecycle FROM assignment WHERE tenant_id=$1 AND assignment_id=$2 FOR UPDATE",
    )
    .bind(tenant.as_uuid())
    .bind(assignment.as_uuid())
    .fetch_optional(&mut **tx)
    .await
    .map_err(map_sqlx_error)?
    .ok_or(StoreError::NotFound)?;
    let lifecycle_gate = assignment_lifecycle_gate(parse_assignment_lifecycle(&lifecycle)?);
    let attempts = sqlx::query("SELECT qa.attempt_id, qa.payload, qa.payload_sha256, qa.problem_id, qa.version_id, enrollment.student_id, run.run_number, floor(extract(epoch FROM run.started_at)*1000)::bigint AS run_started_at, floor(extract(epoch FROM qa.occurred_at)*1000)::bigint AS attempt_occurred_at FROM question_attempt qa JOIN assignment_run run ON run.tenant_id=qa.tenant_id AND run.run_id=qa.run_id JOIN enrollment enrollment ON enrollment.tenant_id=run.tenant_id AND enrollment.enrollment_id=run.enrollment_id WHERE qa.tenant_id=$1 AND enrollment.assignment_id=$3 AND qa.attempt_status='in_progress' ORDER BY qa.attempt_id FOR UPDATE OF qa")
        .bind(tenant.as_uuid()).bind(course.as_uuid()).bind(assignment.as_uuid()).fetch_all(&mut **tx).await.map_err(map_sqlx_error)?;
    for row in attempts {
        let attempt =
            QuestionAttemptId::from_uuid(row.try_get("attempt_id").map_err(map_sqlx_error)?);
        let student = StudentId::from_uuid(row.try_get("student_id").map_err(map_sqlx_error)?);
        let current = sqlx::query("SELECT receipt_generation,timing_generation FROM attempt_effective_policy_current WHERE tenant_id=$1 AND attempt_id=$2 FOR UPDATE")
            .bind(tenant.as_uuid()).bind(attempt.as_uuid()).fetch_optional(&mut **tx).await.map_err(map_sqlx_error)?.ok_or_else(|| StoreError::Unavailable("active attempt is missing its effective-policy pointer".to_string()))?;
        let entitlement =
            super::entitlement::evaluate_current_student(tx, tenant, course, assignment, student)
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
                let (decision, _) = resolve_granted_effective_policy(
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
            super::assignment_timing::cancel_postgres_effective_policy_job(tx, tenant, attempt)
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
        let attempt_record: question_model::QuestionAttempt =
            super::row_decode::decode_payload_row(&row)?;
        let published = super::transaction_context::load_published_record(
            tx,
            question_model::ProblemId::from_uuid(
                row.try_get("problem_id").map_err(map_sqlx_error)?,
            ),
            question_model::VersionId::from_uuid(
                row.try_get("version_id").map_err(map_sqlx_error)?,
            ),
        )
        .await?;
        let grace =
            super::assignment_timing::timing_policy_grace_seconds(published.question.timing_policy);
        let authored_timer = super::runs::attempt_issuance::issued_timer(
            attempt_record.timer.issued_at,
            ActivityTimestamp::from_unix_millis(
                row.try_get("run_started_at").map_err(map_sqlx_error)?,
            ),
            published.question.timing_policy,
        )?;
        let timing = super::assignment_timing::resolved_postgres_attempt_timing(
            &policy,
            ActivityTimestamp::from_unix_millis(
                row.try_get("run_started_at").map_err(map_sqlx_error)?,
            ),
            authored_timer.deadline,
            grace,
        )?;
        let now = database_timestamp(tx).await?;
        if timing
            .effective_deadline
            .is_some_and(|deadline| deadline < now)
            || timing
                .auto_submit_at
                .is_some_and(|deadline| deadline <= now)
        {
            super::assignment_timing::cancel_postgres_effective_policy_job(tx, tenant, attempt)
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
        super::effective_policy_receipts::append_sealed_effective_policy_receipt(
            tx,
            super::effective_policy_receipts::EffectivePolicyReceiptWrite {
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
        super::assignment_timing::cancel_postgres_effective_policy_job(tx, tenant, attempt).await?;
        let next_timing = old_timing.checked_add(1).ok_or(StoreError::Conflict)?;
        let job =
            schedule_effective_policy_job(tx, tenant, attempt, next_timing, timing.auto_submit_at)
                .await?;
        sqlx::query("UPDATE attempt_effective_policy_current SET receipt_generation=$3,timing_generation=$4,job_id=$5,updated_at=transaction_timestamp() WHERE tenant_id=$1 AND attempt_id=$2")
            .bind(tenant.as_uuid()).bind(attempt.as_uuid()).bind(next_generation).bind(next_timing).bind(job.map(JobId::as_uuid)).execute(&mut **tx).await.map_err(map_sqlx_error)?;
    }
    Ok(())
}

pub(super) fn assignment_lifecycle_name(
    value: question_model::AssignmentLifecycle,
) -> &'static str {
    match value {
        question_model::AssignmentLifecycle::Draft => "draft",
        question_model::AssignmentLifecycle::Published => "published",
        question_model::AssignmentLifecycle::Closed => "closed",
        question_model::AssignmentLifecycle::Archived => "archived",
    }
}

pub(super) fn parse_assignment_lifecycle(
    value: &str,
) -> Result<question_model::AssignmentLifecycle, StoreError> {
    match value {
        "draft" => Ok(question_model::AssignmentLifecycle::Draft),
        "published" => Ok(question_model::AssignmentLifecycle::Published),
        "closed" => Ok(question_model::AssignmentLifecycle::Closed),
        "archived" => Ok(question_model::AssignmentLifecycle::Archived),
        _ => Err(StoreError::Unavailable(
            "stored assignment lifecycle is invalid".to_string(),
        )),
    }
}

fn legal_lifecycle_transition(
    current: question_model::AssignmentLifecycle,
    next: question_model::AssignmentLifecycle,
) -> bool {
    use question_model::AssignmentLifecycle;
    matches!(
        (current, next),
        (
            AssignmentLifecycle::Draft,
            AssignmentLifecycle::Draft
                | AssignmentLifecycle::Published
                | AssignmentLifecycle::Archived
        ) | (
            AssignmentLifecycle::Published,
            AssignmentLifecycle::Published
                | AssignmentLifecycle::Closed
                | AssignmentLifecycle::Archived
        ) | (
            AssignmentLifecycle::Closed,
            AssignmentLifecycle::Closed
                | AssignmentLifecycle::Published
                | AssignmentLifecycle::Archived
        ) | (AssignmentLifecycle::Archived, AssignmentLifecycle::Archived)
    )
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

#[async_trait]
impl crate::EffectivePolicyStore for PostgresStore {
    async fn get_base_assignment_policy_impl(
        &self,
        context: TenantContext,
        assignment: AssignmentId,
    ) -> Result<Option<StoredBaseAssignmentPolicy>, StoreError> {
        let mut transaction = self.begin_tenant(context).await?;
        let result = load_base(&mut transaction, context.tenant_id(), assignment, false).await?;
        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(result)
    }

    async fn put_assignment_teaching_settings_impl(
        &self,
        context: TenantContext,
        command: PutAssignmentTeachingSettingsCommand,
    ) -> Result<StoredBaseAssignmentPolicy, StoreError> {
        retry_transaction(|| {
            let command = command.clone();
            async move {
            let tenant = context.tenant_id();
            let mut tx = self.begin_tenant(context).await?;
            lock_policy(&mut tx, tenant, command.assignment).await?;
            authorize_editor(&mut tx, tenant, command.course, command.assignment, command.actor).await?;
            let course_term = load_course_term_for_policy(&mut tx, tenant, command.course).await?;
            validate_base_assignment_policy_for_course_term(command.settings.base_policy, &course_term).map_err(|error| {
                StoreError::InvalidRecord(format!("invalid assignment teaching settings: {error:?}"))
            })?;
            let revision = require_revision(&mut tx, tenant, command.assignment, command.course, command.expected_revision).await?;
            let current_lifecycle: String = sqlx::query_scalar("SELECT lifecycle FROM assignment WHERE tenant_id=$1 AND assignment_id=$2 FOR UPDATE")
                .bind(tenant.as_uuid()).bind(command.assignment.as_uuid()).fetch_one(&mut *tx).await.map_err(map_sqlx_error)?;
            let previous = parse_assignment_lifecycle(&current_lifecycle)?;
            if !legal_lifecycle_transition(previous, command.settings.lifecycle) {
                return Err(StoreError::InvalidRecord("assignment lifecycle transition is invalid".to_string()));
            }
            sqlx::query("UPDATE assignment SET lifecycle=$3, instructions=$4, updated_at=transaction_timestamp() WHERE tenant_id=$1 AND assignment_id=$2")
                .bind(tenant.as_uuid()).bind(command.assignment.as_uuid())
                .bind(assignment_lifecycle_name(command.settings.lifecycle))
                .bind(command.settings.instructions.as_str())
                .execute(&mut *tx).await.map_err(map_sqlx_error)?;
            sqlx::query("INSERT INTO assignment_effective_policy_base \
                (tenant_id, assignment_id, course_id, available_at, due_at, closes_at, late_submission_policy, deadline_behavior, time_limit_seconds, attempt_limit) \
                VALUES ($1,$2,$3,to_timestamp($4::double precision / 1000),to_timestamp($5::double precision / 1000),to_timestamp($6::double precision / 1000),$7,$8,$9,$10) \
                ON CONFLICT (tenant_id, assignment_id) DO UPDATE SET course_id=EXCLUDED.course_id, available_at=EXCLUDED.available_at, due_at=EXCLUDED.due_at, closes_at=EXCLUDED.closes_at, late_submission_policy=EXCLUDED.late_submission_policy, deadline_behavior=EXCLUDED.deadline_behavior, time_limit_seconds=EXCLUDED.time_limit_seconds, attempt_limit=EXCLUDED.attempt_limit, updated_at=transaction_timestamp()")
                .bind(tenant.as_uuid()).bind(command.assignment.as_uuid()).bind(command.course.as_uuid())
                .bind(millis(command.settings.base_policy.available_at)).bind(millis(command.settings.base_policy.due_at)).bind(millis(command.settings.base_policy.closes_at))
                .bind(late_name(command.settings.base_policy.late_submission)).bind(deadline_name(command.settings.base_policy.deadline_behavior))
                .bind(command.settings.base_policy.time_limit_seconds.map(|v| i32::try_from(v.get())).transpose().map_err(|_| StoreError::InvalidRecord("time limit exceeds PostgreSQL integer".to_string()))?)
                .bind(command.settings.base_policy.attempt_limit.map(|v| i32::try_from(v.get())).transpose().map_err(|_| StoreError::InvalidRecord("attempt limit exceeds PostgreSQL integer".to_string()))?)
                .execute(&mut *tx).await.map_err(map_sqlx_error)?;
            let next = bump_revision(&mut tx, tenant, command.assignment, command.expected_revision).await?;
            tx.commit().await.map_err(map_sqlx_error)?;
            Ok(StoredBaseAssignmentPolicy { tenant, course: command.course, assignment: command.assignment, policy: command.settings.base_policy, revision: next.max(revision) })
            }
        }).await
    }

    async fn put_group_schedule_offset_impl(
        &self,
        context: TenantContext,
        command: PutGroupScheduleOffsetCommand,
    ) -> Result<AssignmentRevision, StoreError> {
        mutate_group_offset(self, context, command, false).await
    }
    async fn delete_group_schedule_offset_impl(
        &self,
        context: TenantContext,
        command: DeleteGroupScheduleOffsetCommand,
    ) -> Result<AssignmentRevision, StoreError> {
        delete_group_offset(self, context, command).await
    }
    async fn put_group_accommodation_impl(
        &self,
        context: TenantContext,
        command: PutGroupAccommodationCommand,
    ) -> Result<AssignmentRevision, StoreError> {
        mutate_accommodation(self, context, command).await
    }
    async fn delete_group_accommodation_impl(
        &self,
        context: TenantContext,
        command: DeleteGroupAccommodationCommand,
    ) -> Result<AssignmentRevision, StoreError> {
        delete_accommodation(self, context, command).await
    }
    async fn put_individual_policy_exception_impl(
        &self,
        context: TenantContext,
        command: PutIndividualPolicyExceptionCommand,
    ) -> Result<AssignmentRevision, StoreError> {
        mutate_individual(self, context, command).await
    }
    async fn delete_individual_policy_exception_impl(
        &self,
        context: TenantContext,
        command: DeleteIndividualPolicyExceptionCommand,
    ) -> Result<AssignmentRevision, StoreError> {
        delete_individual(self, context, command).await
    }

    async fn resolve_effective_policy_impl(
        &self,
        context: TenantContext,
        command: ResolveEffectivePolicyCommand,
    ) -> Result<Option<EffectivePolicyResolution>, StoreError> {
        let mut tx = self.begin_tenant(context).await?;
        let tenant = context.tenant_id();
        let assignment = sqlx::query("SELECT course_id, lifecycle, revision FROM assignment WHERE tenant_id=$1 AND assignment_id=$2")
            .bind(tenant.as_uuid()).bind(command.assignment.as_uuid()).fetch_optional(&mut *tx).await.map_err(map_sqlx_error)?;
        let Some(assignment) = assignment else {
            tx.commit().await.map_err(map_sqlx_error)?;
            return Ok(None);
        };
        let course = CourseId::from_uuid(assignment.try_get("course_id").map_err(map_sqlx_error)?);
        let lifecycle: String = assignment.try_get("lifecycle").map_err(map_sqlx_error)?;
        let gate = assignment_lifecycle_gate(parse_assignment_lifecycle(&lifecycle)?);
        let denied = !matches!(gate, AssignmentLifecycleGate::Open)
            || matches!(
                command.entitlement,
                domain::entitlement::EntitlementDecision::Denied(_)
            )
            || matches!(
                command.authorization,
                domain::effective_assignment_policy::AuthorizationGate::Denied(_)
            );
        let inputs = if denied {
            inert_inputs()?
        } else {
            let domain::entitlement::EntitlementDecision::Granted(ref grant) = command.entitlement
            else {
                unreachable!()
            };
            if grant.tenant() != tenant
                || grant.course() != course
                || grant.assignment() != command.assignment
            {
                return Err(StoreError::InvalidRecord(
                    "effective-policy entitlement does not bind this assignment".to_string(),
                ));
            }
            load_inputs(
                &mut tx,
                tenant,
                command.assignment,
                Some(grant.student()),
                Some(grant.applicable_policy_scopes()),
            )
            .await?
        };
        let now = database_timestamp(&mut tx).await?;
        let decision = resolve_effective_policy(ResolveEffectivePolicyInput {
            lifecycle: gate,
            entitlement: command.entitlement,
            authorization: command.authorization,
            now,
            prior_run_count: command.prior_run_count,
            base: inputs.base,
            group_schedule_offsets: inputs.schedule_offsets,
            group_accommodations: inputs.accommodations,
            individual_exception: inputs.individual,
        })
        .map_err(|e| {
            StoreError::InvalidRecord(format!("invalid effective policy inputs: {e:?}"))
        })?;
        let revision = AssignmentRevision::from_stored(
            assignment.try_get("revision").map_err(map_sqlx_error)?,
        )?;
        tx.commit().await.map_err(map_sqlx_error)?;
        Ok(Some(EffectivePolicyResolution {
            tenant,
            course,
            assignment: command.assignment,
            decision,
            revision,
        }))
    }

    async fn get_issued_effective_policy_receipt_impl(
        &self,
        context: TenantContext,
        attempt: QuestionAttemptId,
    ) -> Result<Option<IssuedEffectivePolicyReceipt>, StoreError> {
        let mut tx = self.begin_tenant(context).await?;
        let receipt = super::effective_policy_receipts::read_current_effective_policy_receipt(
            &mut tx,
            context.tenant_id(),
            attempt,
        )
        .await?;
        tx.commit().await.map_err(map_sqlx_error)?;
        Ok(receipt)
    }
}

async fn authorize_editor(
    tx: &mut Transaction<'_, Postgres>,
    tenant: TenantId,
    course: CourseId,
    assignment: AssignmentId,
    actor: UserId,
) -> Result<(), StoreError> {
    let assignment_course: Option<Uuid> = sqlx::query_scalar(
        "SELECT course_id FROM assignment WHERE tenant_id=$1 AND assignment_id=$2 FOR UPDATE",
    )
    .bind(tenant.as_uuid())
    .bind(assignment.as_uuid())
    .fetch_optional(&mut **tx)
    .await
    .map_err(map_sqlx_error)?;
    if assignment_course != Some(course.as_uuid())
        || !postgres_is_course_instructor(tx, tenant, course, actor).await?
    {
        return Err(StoreError::NotFound);
    }
    Ok(())
}

pub(super) async fn load_course_term_for_policy(
    tx: &mut Transaction<'_, Postgres>,
    tenant: TenantId,
    course: CourseId,
) -> Result<CourseTerm, StoreError> {
    let row = sqlx::query(
        "SELECT term_start_date::text AS term_start_date, term_end_date::text AS term_end_date, time_zone \
         FROM course WHERE tenant_id=$1 AND course_id=$2 FOR KEY SHARE",
    )
    .bind(tenant.as_uuid())
    .bind(course.as_uuid())
    .fetch_optional(&mut **tx)
    .await
    .map_err(map_sqlx_error)?
    .ok_or(StoreError::NotFound)?;
    let start_date: String = row.try_get("term_start_date").map_err(map_sqlx_error)?;
    let end_date: String = row.try_get("term_end_date").map_err(map_sqlx_error)?;
    let time_zone: String = row.try_get("time_zone").map_err(map_sqlx_error)?;
    CourseTerm::from_parts(&start_date, &end_date, &time_zone)
        .map_err(|error| StoreError::Unavailable(format!("stored course term is invalid: {error}")))
}

pub(super) async fn load_course_term_for_preview(
    tx: &mut Transaction<'_, Postgres>,
    tenant: TenantId,
    course: CourseId,
) -> Result<CourseTerm, StoreError> {
    preview_resolution::load_course_term_for_preview(tx, tenant, course).await
}

async fn require_revision(
    tx: &mut Transaction<'_, Postgres>,
    tenant: TenantId,
    assignment: AssignmentId,
    course: CourseId,
    expected: AssignmentRevision,
) -> Result<AssignmentRevision, StoreError> {
    let row = sqlx::query("SELECT course_id, revision FROM assignment WHERE tenant_id=$1 AND assignment_id=$2 FOR UPDATE").bind(tenant.as_uuid()).bind(assignment.as_uuid()).fetch_optional(&mut **tx).await.map_err(map_sqlx_error)?.ok_or(StoreError::NotFound)?;
    if row
        .try_get::<Uuid, _>("course_id")
        .map_err(map_sqlx_error)?
        != course.as_uuid()
    {
        return Err(StoreError::NotFound);
    }
    let actual = AssignmentRevision::from_stored(row.try_get("revision").map_err(map_sqlx_error)?)?;
    if actual != expected {
        return Err(StoreError::Conflict);
    }
    Ok(actual)
}

async fn bump_revision(
    tx: &mut Transaction<'_, Postgres>,
    tenant: TenantId,
    assignment: AssignmentId,
    expected: AssignmentRevision,
) -> Result<AssignmentRevision, StoreError> {
    let next = expected.next()?;
    let course: Option<Uuid> = sqlx::query_scalar("UPDATE assignment SET revision=$3, updated_at=transaction_timestamp() WHERE tenant_id=$1 AND assignment_id=$2 AND revision=$4 RETURNING course_id").bind(tenant.as_uuid()).bind(assignment.as_uuid()).bind(i64::try_from(next.value()).map_err(|_| StoreError::Conflict)?).bind(i64::try_from(expected.value()).map_err(|_| StoreError::Conflict)?).fetch_optional(&mut **tx).await.map_err(map_sqlx_error)?;
    let course = CourseId::from_uuid(course.ok_or(StoreError::Conflict)?);
    reresolve_active_attempts(tx, tenant, course, assignment, next).await?;
    Ok(next)
}

async fn lock_policy(
    tx: &mut Transaction<'_, Postgres>,
    tenant: TenantId,
    assignment: AssignmentId,
) -> Result<(), StoreError> {
    sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended($1::uuid::text || ':' || $2::uuid::text, 0))").bind(tenant.as_uuid()).bind(assignment.as_uuid()).execute(&mut **tx).await.map_err(map_sqlx_error)?;
    Ok(())
}

fn millis(value: Option<ActivityTimestamp>) -> Option<i64> {
    value.map(|timestamp| timestamp.as_unix_millis())
}
fn late_name(value: LateSubmissionPolicy) -> &'static str {
    match value {
        LateSubmissionPolicy::Accept => "accept",
        LateSubmissionPolicy::Reject => "reject",
        LateSubmissionPolicy::MarkLate => "mark_late",
    }
}
fn deadline_name(value: AssignmentDeadlineBehavior) -> &'static str {
    match value {
        AssignmentDeadlineBehavior::AutoSubmit => "auto_submit",
    }
}
fn inert_inputs() -> Result<EffectivePolicyInputs, StoreError> {
    Ok(EffectivePolicyInputs {
        base: BaseAssignmentPolicy {
            available_at: None,
            due_at: None,
            closes_at: None,
            time_limit_seconds: None,
            attempt_limit: None,
            late_submission: LateSubmissionPolicy::Accept,
            deadline_behavior: AssignmentDeadlineBehavior::AutoSubmit,
        },
        schedule_offsets: vec![],
        accommodations: vec![],
        individual: None,
    })
}

async fn load_base(
    tx: &mut Transaction<'_, Postgres>,
    tenant: TenantId,
    assignment: AssignmentId,
    lock: bool,
) -> Result<Option<StoredBaseAssignmentPolicy>, StoreError> {
    let sql = if lock {
        "SELECT course_id, floor(extract(epoch FROM available_at)*1000)::bigint AS available, floor(extract(epoch FROM due_at)*1000)::bigint AS due, floor(extract(epoch FROM closes_at)*1000)::bigint AS closes, late_submission_policy, deadline_behavior, time_limit_seconds, attempt_limit, revision FROM assignment_effective_policy_base JOIN assignment USING (tenant_id, assignment_id, course_id) WHERE assignment_effective_policy_base.tenant_id=$1 AND assignment_effective_policy_base.assignment_id=$2 FOR UPDATE"
    } else {
        "SELECT course_id, floor(extract(epoch FROM available_at)*1000)::bigint AS available, floor(extract(epoch FROM due_at)*1000)::bigint AS due, floor(extract(epoch FROM closes_at)*1000)::bigint AS closes, late_submission_policy, deadline_behavior, time_limit_seconds, attempt_limit, revision FROM assignment_effective_policy_base JOIN assignment USING (tenant_id, assignment_id, course_id) WHERE assignment_effective_policy_base.tenant_id=$1 AND assignment_effective_policy_base.assignment_id=$2"
    };
    let row = sqlx::query(sql)
        .bind(tenant.as_uuid())
        .bind(assignment.as_uuid())
        .fetch_optional(&mut **tx)
        .await
        .map_err(map_sqlx_error)?;
    row.map(|row| decode_base(&row, tenant, assignment))
        .transpose()
}

pub(super) async fn load_base_policy(
    tx: &mut Transaction<'_, Postgres>,
    tenant: TenantId,
    assignment: AssignmentId,
) -> Result<BaseAssignmentPolicy, StoreError> {
    load_base(tx, tenant, assignment, false)
        .await?
        .map(|stored| stored.policy)
        .ok_or(StoreError::NotFound)
}

fn decode_base(
    row: &sqlx::postgres::PgRow,
    tenant: TenantId,
    assignment: AssignmentId,
) -> Result<StoredBaseAssignmentPolicy, StoreError> {
    let timestamp = |name| -> Result<Option<ActivityTimestamp>, StoreError> {
        Ok::<Option<ActivityTimestamp>, StoreError>(
            row.try_get::<Option<i64>, _>(name)
                .map_err(map_sqlx_error)?
                .map(ActivityTimestamp::from_unix_millis),
        )
    };
    let limit = |name| -> Result<Option<NonZeroU32>, StoreError> {
        row.try_get::<Option<i32>, _>(name)
            .map_err(map_sqlx_error)?
            .map(|value| {
                u32::try_from(value)
                    .ok()
                    .and_then(NonZeroU32::new)
                    .ok_or_else(|| StoreError::Unavailable(format!("stored {name} is invalid")))
            })
            .transpose()
    };
    let late = match row
        .try_get::<String, _>("late_submission_policy")
        .map_err(map_sqlx_error)?
        .as_str()
    {
        "accept" => LateSubmissionPolicy::Accept,
        "reject" => LateSubmissionPolicy::Reject,
        "mark_late" => LateSubmissionPolicy::MarkLate,
        _ => {
            return Err(StoreError::Unavailable(
                "stored late policy is invalid".to_string(),
            ));
        }
    };
    let deadline = match row
        .try_get::<String, _>("deadline_behavior")
        .map_err(map_sqlx_error)?
        .as_str()
    {
        "auto_submit" => AssignmentDeadlineBehavior::AutoSubmit,
        _ => {
            return Err(StoreError::Unavailable(
                "stored deadline behavior is invalid".to_string(),
            ));
        }
    };
    Ok(StoredBaseAssignmentPolicy {
        tenant,
        course: CourseId::from_uuid(row.try_get("course_id").map_err(map_sqlx_error)?),
        assignment,
        policy: BaseAssignmentPolicy {
            available_at: timestamp("available")?,
            due_at: timestamp("due")?,
            closes_at: timestamp("closes")?,
            time_limit_seconds: limit("time_limit_seconds")?,
            attempt_limit: limit("attempt_limit")?,
            late_submission: late,
            deadline_behavior: deadline,
        },
        revision: AssignmentRevision::from_stored(
            row.try_get("revision").map_err(map_sqlx_error)?,
        )?,
    })
}

pub(super) async fn load_inputs(
    tx: &mut Transaction<'_, Postgres>,
    tenant: TenantId,
    assignment: AssignmentId,
    student: Option<StudentId>,
    scopes: Option<&domain::entitlement::ApplicablePolicyScopes>,
) -> Result<EffectivePolicyInputs, StoreError> {
    let base = load_base(tx, tenant, assignment, false)
        .await?
        .ok_or(StoreError::NotFound)?
        .policy;
    // The evaluator is the only authority that mints applicable scopes.  Do
    // not load every group modifier and hope the pure resolver filters it:
    // that would make unauthorized modifier rows observable to this layer.
    let schedule_groups = scopes.map(|value| {
        value
            .iter()
            .filter_map(|(group, purpose)| {
                question_model::GroupPurposeCapabilities::for_purpose(*purpose)
                    .schedule_scope
                    .then_some(group.as_uuid())
            })
            .collect::<Vec<_>>()
    });
    let accommodation_groups = scopes.map(|value| {
        value
            .iter()
            .filter_map(|(group, purpose)| {
                question_model::GroupPurposeCapabilities::for_purpose(*purpose)
                    .accommodation_scope
                    .then_some(group.as_uuid())
            })
            .collect::<Vec<_>>()
    });
    let offsets=sqlx::query("SELECT course_group_id, schedule_offset_seconds FROM assignment_group_schedule_offset WHERE tenant_id=$1 AND assignment_id=$2 AND ($3::uuid[] IS NULL OR course_group_id = ANY($3)) ORDER BY course_group_id").bind(tenant.as_uuid()).bind(assignment.as_uuid()).bind(schedule_groups.as_deref()).fetch_all(&mut **tx).await.map_err(map_sqlx_error)?.into_iter().map(|r|Ok(GroupScheduleOffset{group:CourseGroupId::from_uuid(r.try_get("course_group_id").map_err(map_sqlx_error)?),offset_seconds:ScheduleOffsetSeconds::try_new(r.try_get("schedule_offset_seconds").map_err(map_sqlx_error)?).map_err(|_|StoreError::Unavailable("stored schedule offset is invalid".to_string()))?})).collect::<Result<Vec<_>,StoreError>>()?;
    let accommodations=sqlx::query("SELECT course_group_id, override_kind, available_mode, floor(extract(epoch FROM available_at)*1000)::bigint AS available, due_mode, floor(extract(epoch FROM due_at)*1000)::bigint AS due, closes_mode, floor(extract(epoch FROM closes_at)*1000)::bigint AS closes, time_limit_mode, time_limit_seconds, attempt_limit_mode, attempt_limit FROM assignment_group_accommodation WHERE tenant_id=$1 AND assignment_id=$2 AND ($3::uuid[] IS NULL OR course_group_id = ANY($3)) ORDER BY course_group_id").bind(tenant.as_uuid()).bind(assignment.as_uuid()).bind(accommodation_groups.as_deref()).fetch_all(&mut **tx).await.map_err(map_sqlx_error)?.into_iter().map(|r|decode_group_accommodation(&r)).collect::<Result<Vec<_>,StoreError>>()?;
    let individual=match student { None=>None, Some(student)=>sqlx::query("SELECT override_kind, available_mode, floor(extract(epoch FROM available_at)*1000)::bigint AS available, due_mode, floor(extract(epoch FROM due_at)*1000)::bigint AS due, closes_mode, floor(extract(epoch FROM closes_at)*1000)::bigint AS closes, time_limit_mode, time_limit_seconds, attempt_limit_mode, attempt_limit FROM assignment_individual_policy_exception WHERE tenant_id=$1 AND assignment_id=$2 AND student_id=$3").bind(tenant.as_uuid()).bind(assignment.as_uuid()).bind(student.as_uuid()).fetch_optional(&mut **tx).await.map_err(map_sqlx_error)?.map(|r|decode_individual(&r,student)).transpose()?};
    Ok(EffectivePolicyInputs {
        base,
        schedule_offsets: offsets,
        accommodations,
        individual,
    })
}

/// Resolves an already-evaluated S5 grant inside the caller's transaction.
/// PostgreSQL derives lifecycle from the stored assignment row, then evaluates
/// one already-authorized S5 grant inside the caller's transaction.
pub(super) async fn resolve_granted_effective_policy(
    tx: &mut Transaction<'_, Postgres>,
    grant: domain::entitlement::EntitlementGrant,
    authorization: domain::effective_assignment_policy::AuthorizationGate,
    prior_run_count: u32,
) -> Result<
    (
        domain::effective_assignment_policy::EffectivePolicyDecision,
        AssignmentRevision,
    ),
    StoreError,
> {
    preview_resolution::resolve_granted_effective_policy_with_lock(
        tx,
        grant,
        authorization,
        prior_run_count,
        true,
    )
    .await
}

pub(super) async fn resolve_granted_effective_policy_read_only(
    tx: &mut Transaction<'_, Postgres>,
    grant: domain::entitlement::EntitlementGrant,
    authorization: domain::effective_assignment_policy::AuthorizationGate,
    prior_run_count: u32,
) -> Result<
    (
        domain::effective_assignment_policy::EffectivePolicyDecision,
        AssignmentRevision,
    ),
    StoreError,
> {
    preview_resolution::resolve_granted_effective_policy_with_lock(
        tx,
        grant,
        authorization,
        prior_run_count,
        false,
    )
    .await
}

fn mode(value: &str) -> Result<PolicyModificationMode, StoreError> {
    match value {
        "extend_only" => Ok(PolicyModificationMode::ExtendOnly),
        "explicit_override" => Ok(PolicyModificationMode::Override),
        _ => Err(StoreError::Unavailable(
            "stored policy override mode is invalid".to_string(),
        )),
    }
}
fn patch_timestamp(
    row: &sqlx::postgres::PgRow,
    mode_name: &str,
    value_name: &str,
) -> Result<PolicyPatch<ActivityTimestamp>, StoreError> {
    match (
        row.try_get::<Option<String>, _>(mode_name)
            .map_err(map_sqlx_error)?
            .as_deref(),
        row.try_get::<Option<i64>, _>(value_name)
            .map_err(map_sqlx_error)?,
    ) {
        (None, None) => Ok(PolicyPatch::Inherit),
        (Some("unrestricted"), None) => Ok(PolicyPatch::Unrestricted),
        (Some("at"), Some(v)) => Ok(PolicyPatch::Set(ActivityTimestamp::from_unix_millis(v))),
        _ => Err(StoreError::Unavailable(
            "stored policy timestamp patch is invalid".to_string(),
        )),
    }
}
fn patch_limit(
    row: &sqlx::postgres::PgRow,
    mode_name: &str,
    value_name: &str,
) -> Result<PolicyPatch<NonZeroU32>, StoreError> {
    match (
        row.try_get::<Option<String>, _>(mode_name)
            .map_err(map_sqlx_error)?
            .as_deref(),
        row.try_get::<Option<i32>, _>(value_name)
            .map_err(map_sqlx_error)?,
    ) {
        (None, None) => Ok(PolicyPatch::Inherit),
        (Some("unlimited"), None) => Ok(PolicyPatch::Unrestricted),
        (Some("value"), Some(v)) => u32::try_from(v)
            .ok()
            .and_then(NonZeroU32::new)
            .map(PolicyPatch::Set)
            .ok_or_else(|| {
                StoreError::Unavailable("stored policy limit patch is invalid".to_string())
            }),
        _ => Err(StoreError::Unavailable(
            "stored policy limit patch is invalid".to_string(),
        )),
    }
}
fn decode_patch(
    row: &sqlx::postgres::PgRow,
) -> Result<(PolicyModificationMode, PolicyPatchSet), StoreError> {
    Ok((
        mode(
            &row.try_get::<String, _>("override_kind")
                .map_err(map_sqlx_error)?,
        )?,
        PolicyPatchSet {
            available_at: patch_timestamp(row, "available_mode", "available")?,
            due_at: patch_timestamp(row, "due_mode", "due")?,
            closes_at: patch_timestamp(row, "closes_mode", "closes")?,
            time_limit_seconds: patch_limit(row, "time_limit_mode", "time_limit_seconds")?,
            attempt_limit: patch_limit(row, "attempt_limit_mode", "attempt_limit")?,
        },
    ))
}
fn decode_group_accommodation(
    row: &sqlx::postgres::PgRow,
) -> Result<GroupAccommodation, StoreError> {
    let (mode, patch) = decode_patch(row)?;
    Ok(GroupAccommodation {
        group: CourseGroupId::from_uuid(row.try_get("course_group_id").map_err(map_sqlx_error)?),
        mode,
        patch,
    })
}
fn decode_individual(
    row: &sqlx::postgres::PgRow,
    student: StudentId,
) -> Result<IndividualPolicyException, StoreError> {
    let (mode, patch) = decode_patch(row)?;
    Ok(IndividualPolicyException {
        student,
        mode,
        patch,
    })
}

async fn mutate_group_offset(
    store: &PostgresStore,
    context: TenantContext,
    command: PutGroupScheduleOffsetCommand,
    _: bool,
) -> Result<AssignmentRevision, StoreError> {
    retry_transaction(||async move{let tenant=context.tenant_id();let mut tx=store.begin_tenant(context).await?;lock_policy(&mut tx,tenant,command.assignment).await?;authorize_editor(&mut tx,tenant,command.course,command.assignment,command.actor).await?;require_revision(&mut tx,tenant,command.assignment,command.course,command.expected_revision).await?;sqlx::query("INSERT INTO assignment_group_schedule_offset (tenant_id,assignment_id,course_id,course_group_id,schedule_offset_seconds) VALUES($1,$2,$3,$4,$5) ON CONFLICT(tenant_id,assignment_id,course_group_id) DO UPDATE SET course_id=EXCLUDED.course_id,schedule_offset_seconds=EXCLUDED.schedule_offset_seconds,updated_at=transaction_timestamp()").bind(tenant.as_uuid()).bind(command.assignment.as_uuid()).bind(command.course.as_uuid()).bind(command.offset.group.as_uuid()).bind(command.offset.offset_seconds.get()).execute(&mut *tx).await.map_err(map_sqlx_error)?;let revision=bump_revision(&mut tx,tenant,command.assignment,command.expected_revision).await?;tx.commit().await.map_err(map_sqlx_error)?;Ok(revision)}).await
}
async fn delete_group_offset(
    store: &PostgresStore,
    context: TenantContext,
    command: DeleteGroupScheduleOffsetCommand,
) -> Result<AssignmentRevision, StoreError> {
    retry_transaction(||async move{let tenant=context.tenant_id();let mut tx=store.begin_tenant(context).await?;lock_policy(&mut tx,tenant,command.assignment).await?;authorize_editor(&mut tx,tenant,command.course,command.assignment,command.actor).await?;require_revision(&mut tx,tenant,command.assignment,command.course,command.expected_revision).await?;let removed=sqlx::query("DELETE FROM assignment_group_schedule_offset WHERE tenant_id=$1 AND assignment_id=$2 AND course_group_id=$3").bind(tenant.as_uuid()).bind(command.assignment.as_uuid()).bind(command.group.as_uuid()).execute(&mut *tx).await.map_err(map_sqlx_error)?;if removed.rows_affected()!=1{return Err(StoreError::NotFound)}let revision=bump_revision(&mut tx,tenant,command.assignment,command.expected_revision).await?;tx.commit().await.map_err(map_sqlx_error)?;Ok(revision)}).await
}
async fn mutate_accommodation(
    store: &PostgresStore,
    context: TenantContext,
    command: PutGroupAccommodationCommand,
) -> Result<AssignmentRevision, StoreError> {
    retry_transaction(||async move{let tenant=context.tenant_id();let mut tx=store.begin_tenant(context).await?;lock_policy(&mut tx,tenant,command.assignment).await?;authorize_editor(&mut tx,tenant,command.course,command.assignment,command.actor).await?;require_revision(&mut tx,tenant,command.assignment,command.course,command.expected_revision).await?;let values=patch_values(command.accommodation.mode,command.accommodation.patch)?;sqlx::query("INSERT INTO assignment_group_accommodation (tenant_id,assignment_id,course_id,course_group_id,override_kind,available_mode,available_at,due_mode,due_at,closes_mode,closes_at,time_limit_mode,time_limit_seconds,attempt_limit_mode,attempt_limit) VALUES($1,$2,$3,$4,$5,$6,to_timestamp($7::double precision/1000),$8,to_timestamp($9::double precision/1000),$10,to_timestamp($11::double precision/1000),$12,$13,$14,$15) ON CONFLICT(tenant_id,assignment_id,course_group_id) DO UPDATE SET override_kind=EXCLUDED.override_kind,available_mode=EXCLUDED.available_mode,available_at=EXCLUDED.available_at,due_mode=EXCLUDED.due_mode,due_at=EXCLUDED.due_at,closes_mode=EXCLUDED.closes_mode,closes_at=EXCLUDED.closes_at,time_limit_mode=EXCLUDED.time_limit_mode,time_limit_seconds=EXCLUDED.time_limit_seconds,attempt_limit_mode=EXCLUDED.attempt_limit_mode,attempt_limit=EXCLUDED.attempt_limit,updated_at=transaction_timestamp()").bind(tenant.as_uuid()).bind(command.assignment.as_uuid()).bind(command.course.as_uuid()).bind(command.accommodation.group.as_uuid()).bind(values.kind).bind(values.available_mode).bind(values.available).bind(values.due_mode).bind(values.due).bind(values.closes_mode).bind(values.closes).bind(values.time_limit_mode).bind(values.time_limit).bind(values.attempt_limit_mode).bind(values.attempt_limit).execute(&mut *tx).await.map_err(map_sqlx_error)?;let revision=bump_revision(&mut tx,tenant,command.assignment,command.expected_revision).await?;tx.commit().await.map_err(map_sqlx_error)?;Ok(revision)}).await
}
async fn delete_accommodation(
    store: &PostgresStore,
    context: TenantContext,
    command: DeleteGroupAccommodationCommand,
) -> Result<AssignmentRevision, StoreError> {
    retry_transaction(||async move{let tenant=context.tenant_id();let mut tx=store.begin_tenant(context).await?;lock_policy(&mut tx,tenant,command.assignment).await?;authorize_editor(&mut tx,tenant,command.course,command.assignment,command.actor).await?;require_revision(&mut tx,tenant,command.assignment,command.course,command.expected_revision).await?;let removed=sqlx::query("DELETE FROM assignment_group_accommodation WHERE tenant_id=$1 AND assignment_id=$2 AND course_group_id=$3").bind(tenant.as_uuid()).bind(command.assignment.as_uuid()).bind(command.group.as_uuid()).execute(&mut *tx).await.map_err(map_sqlx_error)?;if removed.rows_affected()!=1{return Err(StoreError::NotFound)}let revision=bump_revision(&mut tx,tenant,command.assignment,command.expected_revision).await?;tx.commit().await.map_err(map_sqlx_error)?;Ok(revision)}).await
}
async fn mutate_individual(
    store: &PostgresStore,
    context: TenantContext,
    command: PutIndividualPolicyExceptionCommand,
) -> Result<AssignmentRevision, StoreError> {
    retry_transaction(||async move{let tenant=context.tenant_id();let mut tx=store.begin_tenant(context).await?;lock_policy(&mut tx,tenant,command.assignment).await?;authorize_editor(&mut tx,tenant,command.course,command.assignment,command.actor).await?;require_revision(&mut tx,tenant,command.assignment,command.course,command.expected_revision).await?;let e=command.exception;let values=patch_values(e.exception.mode,e.exception.patch)?;sqlx::query("INSERT INTO assignment_individual_policy_exception (tenant_id,assignment_individual_policy_exception_id,assignment_id,course_id,student_id,override_kind,available_mode,available_at,due_mode,due_at,closes_mode,closes_at,time_limit_mode,time_limit_seconds,attempt_limit_mode,attempt_limit) VALUES($1,$2,$3,$4,$5,$6,$7,to_timestamp($8::double precision/1000),$9,to_timestamp($10::double precision/1000),$11,to_timestamp($12::double precision/1000),$13,$14,$15,$16) ON CONFLICT(tenant_id,assignment_id,student_id) DO UPDATE SET assignment_individual_policy_exception_id=EXCLUDED.assignment_individual_policy_exception_id,override_kind=EXCLUDED.override_kind,available_mode=EXCLUDED.available_mode,available_at=EXCLUDED.available_at,due_mode=EXCLUDED.due_mode,due_at=EXCLUDED.due_at,closes_mode=EXCLUDED.closes_mode,closes_at=EXCLUDED.closes_at,time_limit_mode=EXCLUDED.time_limit_mode,time_limit_seconds=EXCLUDED.time_limit_seconds,attempt_limit_mode=EXCLUDED.attempt_limit_mode,attempt_limit=EXCLUDED.attempt_limit,updated_at=transaction_timestamp()").bind(tenant.as_uuid()).bind(e.id.as_uuid()).bind(command.assignment.as_uuid()).bind(command.course.as_uuid()).bind(e.exception.student.as_uuid()).bind(values.kind).bind(values.available_mode).bind(values.available).bind(values.due_mode).bind(values.due).bind(values.closes_mode).bind(values.closes).bind(values.time_limit_mode).bind(values.time_limit).bind(values.attempt_limit_mode).bind(values.attempt_limit).execute(&mut *tx).await.map_err(map_sqlx_error)?;let revision=bump_revision(&mut tx,tenant,command.assignment,command.expected_revision).await?;tx.commit().await.map_err(map_sqlx_error)?;Ok(revision)}).await
}
async fn delete_individual(
    store: &PostgresStore,
    context: TenantContext,
    command: DeleteIndividualPolicyExceptionCommand,
) -> Result<AssignmentRevision, StoreError> {
    retry_transaction(||async move{let tenant=context.tenant_id();let mut tx=store.begin_tenant(context).await?;lock_policy(&mut tx,tenant,command.assignment).await?;authorize_editor(&mut tx,tenant,command.course,command.assignment,command.actor).await?;require_revision(&mut tx,tenant,command.assignment,command.course,command.expected_revision).await?;let removed=sqlx::query("DELETE FROM assignment_individual_policy_exception WHERE tenant_id=$1 AND assignment_id=$2 AND student_id=$3").bind(tenant.as_uuid()).bind(command.assignment.as_uuid()).bind(command.student.as_uuid()).execute(&mut *tx).await.map_err(map_sqlx_error)?;if removed.rows_affected()!=1{return Err(StoreError::NotFound)}let revision=bump_revision(&mut tx,tenant,command.assignment,command.expected_revision).await?;tx.commit().await.map_err(map_sqlx_error)?;Ok(revision)}).await
}

struct PatchValues {
    kind: &'static str,
    available_mode: Option<&'static str>,
    available: Option<i64>,
    due_mode: Option<&'static str>,
    due: Option<i64>,
    closes_mode: Option<&'static str>,
    closes: Option<i64>,
    time_limit_mode: Option<&'static str>,
    time_limit: Option<i32>,
    attempt_limit_mode: Option<&'static str>,
    attempt_limit: Option<i32>,
}
fn timestamp_columns(value: PolicyPatch<ActivityTimestamp>) -> (Option<&'static str>, Option<i64>) {
    match value {
        PolicyPatch::Inherit => (None, None),
        PolicyPatch::Unrestricted => (Some("unrestricted"), None),
        PolicyPatch::Set(v) => (Some("at"), Some(v.as_unix_millis())),
    }
}
fn limit_columns(
    value: PolicyPatch<NonZeroU32>,
) -> Result<(Option<&'static str>, Option<i32>), StoreError> {
    match value {
        PolicyPatch::Inherit => Ok((None, None)),
        PolicyPatch::Unrestricted => Ok((Some("unlimited"), None)),
        PolicyPatch::Set(v) => Ok((
            Some("value"),
            Some(i32::try_from(v.get()).map_err(|_| {
                StoreError::InvalidRecord("policy limit exceeds PostgreSQL integer".to_string())
            })?),
        )),
    }
}
fn patch_values(
    mode: PolicyModificationMode,
    patch: PolicyPatchSet,
) -> Result<PatchValues, StoreError> {
    let (available_mode, available) = timestamp_columns(patch.available_at);
    let (due_mode, due) = timestamp_columns(patch.due_at);
    let (closes_mode, closes) = timestamp_columns(patch.closes_at);
    let (time_limit_mode, time_limit) = limit_columns(patch.time_limit_seconds)?;
    let (attempt_limit_mode, attempt_limit) = limit_columns(patch.attempt_limit)?;
    Ok(PatchValues {
        kind: match mode {
            PolicyModificationMode::ExtendOnly => "extend_only",
            PolicyModificationMode::Override => "explicit_override",
        },
        available_mode,
        available,
        due_mode,
        due,
        closes_mode,
        closes,
        time_limit_mode,
        time_limit,
        attempt_limit_mode,
        attempt_limit,
    })
}

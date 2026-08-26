//! PostgreSQL ownership of the normalized effective-policy inputs.

use std::num::NonZeroU32;

use async_trait::async_trait;
use domain::effective_assignment_policy::{
    AssignmentLifecycleGate, BaseAssignmentPolicy, GroupAccommodation, GroupScheduleOffset,
    IndividualPolicyException, PolicyModificationMode, PolicyPatch, PolicyPatchSet,
    ResolveEffectivePolicyInput, ScheduleOffsetSeconds, assignment_lifecycle_gate,
    resolve_effective_policy,
};
use question_model::{
    ActivityTimestamp, AssignmentDeadlineBehavior, AssignmentId, CourseGroupId, CourseId,
    CourseTerm, LateSubmissionPolicy, StudentId, TenantId,
};
use serde_json::json;
use sqlx::{Postgres, Row, Transaction};

use super::*;
use crate::*;
use crate::{
    assignment_revision_checked_next, assignment_revision_from_stored,
    assignment_revision_to_stored,
};

mod active_attempts;
mod preview_resolution;
pub(super) use active_attempts::{
    assignment_lifecycle_name, inert_inputs, parse_assignment_lifecycle,
    reresolve_prelocked_active_attempts,
};

fn map_assignment_mutator_error(error: sqlx::Error) -> StoreError {
    if let sqlx::Error::Database(database_error) = &error
        && database_error.code().as_deref() == Some("42501")
    {
        return StoreError::NotFound;
    }
    map_sqlx_error(error)
}

#[async_trait]
impl crate::EffectivePolicyStore for PostgresStore {
    async fn get_base_assignment_policy_impl(
        &self,
        context: TenantContext,
        assignment: AssignmentId,
    ) -> Result<Option<StoredBaseAssignmentPolicy>, StoreError> {
        let mut transaction = self.begin_tenant(context).await?;
        let result = load_base(&mut transaction, context.tenant_id(), assignment).await?;
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
                prepare_policy_mutation(
                    &mut tx,
                    tenant,
                    command.course,
                    command.assignment,
                    command.actor,
                    command.expected_revision,
                )
                .await?;
                let settings = teaching_settings_payload(command.settings.clone())?;
                let returned: i64 = sqlx::query_scalar(
                    "SELECT ple_put_assignment_teaching_settings($1,$2,$3,$4,$5,$6)",
                )
                .bind(tenant.as_uuid())
                .bind(command.actor.as_uuid())
                .bind(command.course.as_uuid())
                .bind(command.assignment.as_uuid())
                .bind(assignment_revision_to_stored(command.expected_revision)?)
                .bind(settings)
                .fetch_one(&mut *tx)
                .await
                .map_err(map_assignment_mutator_error)?;
                let next = finish_policy_mutation(
                    &mut tx,
                    context,
                    command.actor,
                    command.course,
                    command.assignment,
                    command.expected_revision,
                    returned,
                )
                .await?;
                let stored = load_base(&mut tx, tenant, command.assignment)
                    .await?
                    .ok_or(StoreError::NotFound)?;
                if stored.course != command.course
                    || stored.policy != command.settings.base_policy
                    || stored.revision != next
                {
                    return Err(StoreError::Unavailable(
                        "teaching-settings capability normalization mismatch".into(),
                    ));
                }
                tx.commit().await.map_err(map_sqlx_error)?;
                Ok(stored)
            }
        })
        .await
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
            active_attempts::inert_inputs()?
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
        let revision = assignment_revision_from_stored(
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

pub(super) async fn load_course_term_for_preview(
    tx: &mut Transaction<'_, Postgres>,
    tenant: TenantId,
    course: CourseId,
) -> Result<CourseTerm, StoreError> {
    preview_resolution::load_course_term_for_preview(tx, tenant, course).await
}

async fn prepare_policy_mutation(
    tx: &mut Transaction<'_, Postgres>,
    tenant: TenantId,
    course: CourseId,
    assignment: AssignmentId,
    actor: UserId,
    expected: AssignmentRevision,
) -> Result<(), StoreError> {
    let expected = assignment_revision_to_stored(expected)?;
    let returned: i64 =
        sqlx::query_scalar("SELECT ple_prepare_assignment_mutation($1, $2, $3, $4, $5)")
            .bind(tenant.as_uuid())
            .bind(actor.as_uuid())
            .bind(course.as_uuid())
            .bind(assignment.as_uuid())
            .bind(expected)
            .fetch_one(&mut **tx)
            .await
            .map_err(map_assignment_mutator_error)?;
    if returned != expected {
        return Err(StoreError::Conflict);
    }
    Ok(())
}

async fn finish_policy_mutation(
    tx: &mut Transaction<'_, Postgres>,
    context: TenantContext,
    actor: UserId,
    course: CourseId,
    assignment: AssignmentId,
    expected: AssignmentRevision,
    returned: i64,
) -> Result<AssignmentRevision, StoreError> {
    let revision = assignment_revision_from_stored(returned)?;
    if revision != assignment_revision_checked_next(expected)? {
        return Err(StoreError::Conflict);
    }
    reresolve_post_mutation_active_attempts(tx, context, actor, course, assignment, revision)
        .await?;
    Ok(revision)
}

/// Completes the broker-governed post-mutation repair preparation.  The
/// capability locks the exact active-attempt set; its opaque witness is
/// decoded and validated before the active-attempt worker hydrates learners.
pub(super) async fn prepare_post_mutation_active_attempt_reresolution(
    tx: &mut Transaction<'_, Postgres>,
    tenant: TenantId,
    actor: UserId,
    course: CourseId,
    assignment: AssignmentId,
    revision: AssignmentRevision,
) -> Result<active_attempts::ActiveAttemptPrepareWitness, StoreError> {
    let row = sqlx::query(
        "SELECT * FROM ple_prepare_assignment_active_attempt_reresolution($1,$2,$3,$4,$5)",
    )
    .bind(tenant.as_uuid())
    .bind(actor.as_uuid())
    .bind(course.as_uuid())
    .bind(assignment.as_uuid())
    .bind(assignment_revision_to_stored(revision)?)
    .fetch_one(&mut **tx)
    .await
    .map_err(map_assignment_mutator_error)?;
    let witness = active_attempts::ActiveAttemptPrepareWitness::decode(&row)?;
    witness.require_revision(revision)?;
    Ok(witness)
}

pub(super) async fn reresolve_post_mutation_active_attempts(
    tx: &mut Transaction<'_, Postgres>,
    context: TenantContext,
    actor: UserId,
    course: CourseId,
    assignment: AssignmentId,
    revision: AssignmentRevision,
) -> Result<(), StoreError> {
    let witness = prepare_post_mutation_active_attempt_reresolution(
        tx,
        context.tenant_id(),
        actor,
        course,
        assignment,
        revision,
    )
    .await?;
    reresolve_prelocked_active_attempts(
        tx,
        context.tenant_id(),
        course,
        assignment,
        revision,
        witness,
    )
    .await
}
fn teaching_settings_payload(
    settings: question_model::AssignmentTeachingSettings,
) -> Result<serde_json::Value, StoreError> {
    let p = settings.base_policy;
    let limit = |v: Option<NonZeroU32>| {
        v.map(|v| {
            i32::try_from(v.get()).map_err(|_| {
                StoreError::InvalidRecord("policy limit exceeds PostgreSQL integer".into())
            })
        })
        .transpose()
    };
    Ok(
        json!({"lifecycle":assignment_lifecycle_name(settings.lifecycle),"instructions":settings.instructions,"basePolicy":{"availableAt":p.available_at.map(|v|v.as_unix_millis()),"dueAt":p.due_at.map(|v|v.as_unix_millis()),"closesAt":p.closes_at.map(|v|v.as_unix_millis()),"lateSubmission":late_name(p.late_submission),"deadlineBehavior":deadline_name(p.deadline_behavior),"timeLimitSeconds":limit(p.time_limit_seconds)?,"attemptLimit":limit(p.attempt_limit)?}}),
    )
}
fn late_name(value: LateSubmissionPolicy) -> &'static str {
    match value {
        LateSubmissionPolicy::Accept => "accept",
        LateSubmissionPolicy::Reject => "reject",
        LateSubmissionPolicy::MarkLate => "markLate",
    }
}
fn deadline_name(_: AssignmentDeadlineBehavior) -> &'static str {
    "autoSubmit"
}

async fn load_base(
    tx: &mut Transaction<'_, Postgres>,
    tenant: TenantId,
    assignment: AssignmentId,
) -> Result<Option<StoredBaseAssignmentPolicy>, StoreError> {
    let row = sqlx::query("SELECT course_id, floor(extract(epoch FROM available_at)*1000)::bigint AS available, floor(extract(epoch FROM due_at)*1000)::bigint AS due, floor(extract(epoch FROM closes_at)*1000)::bigint AS closes, late_submission_policy, deadline_behavior, time_limit_seconds, attempt_limit, revision FROM assignment_effective_policy_base JOIN assignment USING (tenant_id, assignment_id, course_id) WHERE assignment_effective_policy_base.tenant_id=$1 AND assignment_effective_policy_base.assignment_id=$2")
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
    load_base(tx, tenant, assignment)
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
        revision: assignment_revision_from_stored(
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
    let base = load_base(tx, tenant, assignment)
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
    preview_resolution::resolve_granted_effective_policy_read_only(
        tx,
        grant,
        authorization,
        prior_run_count,
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

fn patch_payload(
    mode: PolicyModificationMode,
    patch: PolicyPatchSet,
) -> Result<serde_json::Value, StoreError> {
    let timestamp = |v: PolicyPatch<ActivityTimestamp>| match v {
        PolicyPatch::Inherit => json!([null, null]),
        PolicyPatch::Unrestricted => json!(["unrestricted", null]),
        PolicyPatch::Set(v) => json!(["at", v.as_unix_millis()]),
    };
    let limit = |v: PolicyPatch<NonZeroU32>| -> Result<(Option<&str>, Option<i32>), StoreError> {
        match v {
            PolicyPatch::Inherit => Ok((None, None)),
            PolicyPatch::Unrestricted => Ok((Some("unlimited"), None)),
            PolicyPatch::Set(v) => Ok((
                Some("value"),
                Some(i32::try_from(v.get()).map_err(|_| {
                    StoreError::InvalidRecord("policy limit exceeds PostgreSQL integer".into())
                })?),
            )),
        }
    };
    let a = timestamp(patch.available_at);
    let d = timestamp(patch.due_at);
    let c = timestamp(patch.closes_at);
    let (tlm, tl) = limit(patch.time_limit_seconds)?;
    let (alm, al) = limit(patch.attempt_limit)?;
    Ok(
        json!({"overrideKind":match mode{PolicyModificationMode::ExtendOnly=>"extend_only",PolicyModificationMode::Override=>"explicit_override"},"availableMode":a[0],"availableAt":a[1],"dueMode":d[0],"dueAt":d[1],"closesMode":c[0],"closesAt":c[1],"timeLimitMode":tlm,"timeLimitSeconds":tl,"attemptLimitMode":alm,"attemptLimit":al}),
    )
}

async fn mutate_group_offset(
    store: &PostgresStore,
    context: TenantContext,
    command: PutGroupScheduleOffsetCommand,
    _: bool,
) -> Result<AssignmentRevision, StoreError> {
    retry_transaction(|| async move {
        let tenant = context.tenant_id();
        let mut tx = store.begin_tenant(context).await?;
        prepare_policy_mutation(
            &mut tx,
            tenant,
            command.course,
            command.assignment,
            command.actor,
            command.expected_revision,
        )
        .await?;
        let returned: i64 = sqlx::query_scalar(
            "SELECT ple_put_assignment_group_schedule_offset($1,$2,$3,$4,$5,$6,$7)",
        )
        .bind(tenant.as_uuid())
        .bind(command.actor.as_uuid())
        .bind(command.course.as_uuid())
        .bind(command.assignment.as_uuid())
        .bind(assignment_revision_to_stored(command.expected_revision)?)
        .bind(command.offset.group.as_uuid())
        .bind(command.offset.offset_seconds.get())
        .fetch_one(&mut *tx)
        .await
        .map_err(map_assignment_mutator_error)?;
        let revision = finish_policy_mutation(
            &mut tx,
            context,
            command.actor,
            command.course,
            command.assignment,
            command.expected_revision,
            returned,
        )
        .await?;
        tx.commit().await.map_err(map_sqlx_error)?;
        Ok(revision)
    })
    .await
}
async fn delete_group_offset(
    store: &PostgresStore,
    context: TenantContext,
    command: DeleteGroupScheduleOffsetCommand,
) -> Result<AssignmentRevision, StoreError> {
    retry_transaction(|| async move {
        let tenant = context.tenant_id();
        let mut tx = store.begin_tenant(context).await?;
        prepare_policy_mutation(
            &mut tx,
            tenant,
            command.course,
            command.assignment,
            command.actor,
            command.expected_revision,
        )
        .await?;
        let returned: i64 = sqlx::query_scalar(
            "SELECT ple_delete_assignment_group_schedule_offset($1,$2,$3,$4,$5,$6)",
        )
        .bind(tenant.as_uuid())
        .bind(command.actor.as_uuid())
        .bind(command.course.as_uuid())
        .bind(command.assignment.as_uuid())
        .bind(assignment_revision_to_stored(command.expected_revision)?)
        .bind(command.group.as_uuid())
        .fetch_one(&mut *tx)
        .await
        .map_err(map_assignment_mutator_error)?;
        let revision = finish_policy_mutation(
            &mut tx,
            context,
            command.actor,
            command.course,
            command.assignment,
            command.expected_revision,
            returned,
        )
        .await?;
        tx.commit().await.map_err(map_sqlx_error)?;
        Ok(revision)
    })
    .await
}
async fn mutate_accommodation(
    store: &PostgresStore,
    context: TenantContext,
    command: PutGroupAccommodationCommand,
) -> Result<AssignmentRevision, StoreError> {
    retry_transaction(|| async move {
        let tenant = context.tenant_id();
        let mut tx = store.begin_tenant(context).await?;
        prepare_policy_mutation(
            &mut tx,
            tenant,
            command.course,
            command.assignment,
            command.actor,
            command.expected_revision,
        )
        .await?;
        let returned: i64 = sqlx::query_scalar(
            "SELECT ple_put_assignment_group_accommodation($1,$2,$3,$4,$5,$6,$7)",
        )
        .bind(tenant.as_uuid())
        .bind(command.actor.as_uuid())
        .bind(command.course.as_uuid())
        .bind(command.assignment.as_uuid())
        .bind(assignment_revision_to_stored(command.expected_revision)?)
        .bind(command.accommodation.group.as_uuid())
        .bind(patch_payload(
            command.accommodation.mode,
            command.accommodation.patch,
        )?)
        .fetch_one(&mut *tx)
        .await
        .map_err(map_assignment_mutator_error)?;
        let revision = finish_policy_mutation(
            &mut tx,
            context,
            command.actor,
            command.course,
            command.assignment,
            command.expected_revision,
            returned,
        )
        .await?;
        tx.commit().await.map_err(map_sqlx_error)?;
        Ok(revision)
    })
    .await
}
async fn delete_accommodation(
    store: &PostgresStore,
    context: TenantContext,
    command: DeleteGroupAccommodationCommand,
) -> Result<AssignmentRevision, StoreError> {
    retry_transaction(|| async move {
        let tenant = context.tenant_id();
        let mut tx = store.begin_tenant(context).await?;
        prepare_policy_mutation(
            &mut tx,
            tenant,
            command.course,
            command.assignment,
            command.actor,
            command.expected_revision,
        )
        .await?;
        let returned: i64 = sqlx::query_scalar(
            "SELECT ple_delete_assignment_group_accommodation($1,$2,$3,$4,$5,$6)",
        )
        .bind(tenant.as_uuid())
        .bind(command.actor.as_uuid())
        .bind(command.course.as_uuid())
        .bind(command.assignment.as_uuid())
        .bind(assignment_revision_to_stored(command.expected_revision)?)
        .bind(command.group.as_uuid())
        .fetch_one(&mut *tx)
        .await
        .map_err(map_assignment_mutator_error)?;
        let revision = finish_policy_mutation(
            &mut tx,
            context,
            command.actor,
            command.course,
            command.assignment,
            command.expected_revision,
            returned,
        )
        .await?;
        tx.commit().await.map_err(map_sqlx_error)?;
        Ok(revision)
    })
    .await
}
async fn mutate_individual(
    store: &PostgresStore,
    context: TenantContext,
    command: PutIndividualPolicyExceptionCommand,
) -> Result<AssignmentRevision, StoreError> {
    let e = command.exception;
    retry_transaction(|| async move {
        let tenant = context.tenant_id();
        let mut tx = store.begin_tenant(context).await?;
        prepare_policy_mutation(
            &mut tx,
            tenant,
            command.course,
            command.assignment,
            command.actor,
            command.expected_revision,
        )
        .await?;
        let returned: i64 = sqlx::query_scalar(
            "SELECT ple_put_assignment_individual_exception($1,$2,$3,$4,$5,$6,$7,$8)",
        )
        .bind(tenant.as_uuid())
        .bind(command.actor.as_uuid())
        .bind(command.course.as_uuid())
        .bind(command.assignment.as_uuid())
        .bind(assignment_revision_to_stored(command.expected_revision)?)
        .bind(e.id.as_uuid())
        .bind(e.exception.student.as_uuid())
        .bind(patch_payload(e.exception.mode, e.exception.patch)?)
        .fetch_one(&mut *tx)
        .await
        .map_err(map_assignment_mutator_error)?;
        let revision = finish_policy_mutation(
            &mut tx,
            context,
            command.actor,
            command.course,
            command.assignment,
            command.expected_revision,
            returned,
        )
        .await?;
        tx.commit().await.map_err(map_sqlx_error)?;
        Ok(revision)
    })
    .await
}
async fn delete_individual(
    store: &PostgresStore,
    context: TenantContext,
    command: DeleteIndividualPolicyExceptionCommand,
) -> Result<AssignmentRevision, StoreError> {
    retry_transaction(|| async move {
        let tenant = context.tenant_id();
        let mut tx = store.begin_tenant(context).await?;
        prepare_policy_mutation(
            &mut tx,
            tenant,
            command.course,
            command.assignment,
            command.actor,
            command.expected_revision,
        )
        .await?;
        let returned: i64 = sqlx::query_scalar(
            "SELECT ple_delete_assignment_individual_exception($1,$2,$3,$4,$5,$6)",
        )
        .bind(tenant.as_uuid())
        .bind(command.actor.as_uuid())
        .bind(command.course.as_uuid())
        .bind(command.assignment.as_uuid())
        .bind(assignment_revision_to_stored(command.expected_revision)?)
        .bind(command.student.as_uuid())
        .fetch_one(&mut *tx)
        .await
        .map_err(map_assignment_mutator_error)?;
        let revision = finish_policy_mutation(
            &mut tx,
            context,
            command.actor,
            command.course,
            command.assignment,
            command.expected_revision,
            returned,
        )
        .await?;
        tx.commit().await.map_err(map_sqlx_error)?;
        Ok(revision)
    })
    .await
}

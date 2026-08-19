use async_trait::async_trait;
use domain::{
    effective_assignment_policy::{
        AssignmentLifecycleGate, AuthorizationGate, EffectivePolicyDecision,
        ResolveEffectivePolicyInput, resolve_effective_policy,
    },
    entitlement::EntitlementGrant,
};

use super::*;

#[async_trait]
impl crate::EffectivePolicyStore for MemoryStore {
    async fn get_base_assignment_policy_impl(
        &self,
        context: TenantContext,
        assignment: AssignmentId,
    ) -> Result<Option<StoredBaseAssignmentPolicy>, StoreError> {
        Ok(self
            .read_state()?
            .assignment_base_policy
            .get(&(context.tenant_id(), assignment))
            .copied())
    }

    async fn put_base_assignment_policy_impl(
        &self,
        context: TenantContext,
        command: PutBaseAssignmentPolicyCommand,
    ) -> Result<StoredBaseAssignmentPolicy, StoreError> {
        let tenant = context.tenant_id();
        let mut state = self.write_state()?;
        authorize_policy_editor(
            &state,
            tenant,
            command.course,
            command.assignment,
            command.actor,
        )?;
        let revision = require_expected_revision(
            &state,
            tenant,
            command.assignment,
            command.expected_revision,
        )?;
        let record = StoredBaseAssignmentPolicy {
            tenant,
            course: command.course,
            assignment: command.assignment,
            policy: command.policy,
            revision,
        };
        state
            .assignment_base_policy
            .insert((tenant, command.assignment), record);
        state
            .assignment_revisions
            .insert((tenant, command.assignment), revision);
        Ok(record)
    }

    async fn put_group_schedule_offset_impl(
        &self,
        context: TenantContext,
        command: PutGroupScheduleOffsetCommand,
    ) -> Result<AssignmentRevision, StoreError> {
        let tenant = context.tenant_id();
        let mut state = self.write_state()?;
        authorize_policy_editor(
            &state,
            tenant,
            command.course,
            command.assignment,
            command.actor,
        )?;
        require_group_course(&state, tenant, command.course, command.offset.group)?;
        let revision = require_expected_revision(
            &state,
            tenant,
            command.assignment,
            command.expected_revision,
        )?;
        state.assignment_group_schedule_offsets.insert(
            (tenant, command.assignment, command.offset.group),
            command.offset,
        );
        state
            .assignment_revisions
            .insert((tenant, command.assignment), revision);
        Ok(revision)
    }

    async fn delete_group_schedule_offset_impl(
        &self,
        context: TenantContext,
        command: DeleteGroupScheduleOffsetCommand,
    ) -> Result<AssignmentRevision, StoreError> {
        let tenant = context.tenant_id();
        let mut state = self.write_state()?;
        authorize_policy_editor(
            &state,
            tenant,
            command.course,
            command.assignment,
            command.actor,
        )?;
        let revision = require_expected_revision(
            &state,
            tenant,
            command.assignment,
            command.expected_revision,
        )?;
        state
            .assignment_group_schedule_offsets
            .remove(&(tenant, command.assignment, command.group))
            .ok_or(StoreError::NotFound)?;
        state
            .assignment_revisions
            .insert((tenant, command.assignment), revision);
        Ok(revision)
    }

    async fn put_group_accommodation_impl(
        &self,
        context: TenantContext,
        command: PutGroupAccommodationCommand,
    ) -> Result<AssignmentRevision, StoreError> {
        let tenant = context.tenant_id();
        let mut state = self.write_state()?;
        authorize_policy_editor(
            &state,
            tenant,
            command.course,
            command.assignment,
            command.actor,
        )?;
        require_group_course(&state, tenant, command.course, command.accommodation.group)?;
        let revision = require_expected_revision(
            &state,
            tenant,
            command.assignment,
            command.expected_revision,
        )?;
        state.assignment_group_accommodations.insert(
            (tenant, command.assignment, command.accommodation.group),
            command.accommodation,
        );
        state
            .assignment_revisions
            .insert((tenant, command.assignment), revision);
        Ok(revision)
    }

    async fn delete_group_accommodation_impl(
        &self,
        context: TenantContext,
        command: DeleteGroupAccommodationCommand,
    ) -> Result<AssignmentRevision, StoreError> {
        let tenant = context.tenant_id();
        let mut state = self.write_state()?;
        authorize_policy_editor(
            &state,
            tenant,
            command.course,
            command.assignment,
            command.actor,
        )?;
        let revision = require_expected_revision(
            &state,
            tenant,
            command.assignment,
            command.expected_revision,
        )?;
        state
            .assignment_group_accommodations
            .remove(&(tenant, command.assignment, command.group))
            .ok_or(StoreError::NotFound)?;
        state
            .assignment_revisions
            .insert((tenant, command.assignment), revision);
        Ok(revision)
    }

    async fn put_individual_policy_exception_impl(
        &self,
        context: TenantContext,
        command: PutIndividualPolicyExceptionCommand,
    ) -> Result<AssignmentRevision, StoreError> {
        let tenant = context.tenant_id();
        let mut state = self.write_state()?;
        authorize_policy_editor(
            &state,
            tenant,
            command.course,
            command.assignment,
            command.actor,
        )?;
        let revision = require_expected_revision(
            &state,
            tenant,
            command.assignment,
            command.expected_revision,
        )?;
        let student = command.exception.exception.student;
        if state.assignment_individual_policy_exceptions.iter().any(
            |((record_tenant, record_assignment, existing_student), existing)| {
                *record_tenant == tenant
                    && *record_assignment == command.assignment
                    && *existing_student != student
                    && existing.id == command.exception.id
            },
        ) {
            return Err(StoreError::Conflict);
        }
        state
            .assignment_individual_policy_exceptions
            .insert((tenant, command.assignment, student), command.exception);
        state
            .assignment_revisions
            .insert((tenant, command.assignment), revision);
        Ok(revision)
    }

    async fn delete_individual_policy_exception_impl(
        &self,
        context: TenantContext,
        command: DeleteIndividualPolicyExceptionCommand,
    ) -> Result<AssignmentRevision, StoreError> {
        let tenant = context.tenant_id();
        let mut state = self.write_state()?;
        authorize_policy_editor(
            &state,
            tenant,
            command.course,
            command.assignment,
            command.actor,
        )?;
        let revision = require_expected_revision(
            &state,
            tenant,
            command.assignment,
            command.expected_revision,
        )?;
        state
            .assignment_individual_policy_exceptions
            .remove(&(tenant, command.assignment, command.student))
            .ok_or(StoreError::NotFound)?;
        state
            .assignment_revisions
            .insert((tenant, command.assignment), revision);
        Ok(revision)
    }

    async fn resolve_effective_policy_impl(
        &self,
        context: TenantContext,
        command: ResolveEffectivePolicyCommand,
    ) -> Result<Option<EffectivePolicyResolution>, StoreError> {
        let state = self.read_state()?;
        let tenant = context.tenant_id();
        let record = match state.assignments.get(&(tenant, command.assignment)) {
            Some(record) => record,
            None => return Ok(None),
        };
        let should_short_circuit = match command.lifecycle {
            domain::effective_assignment_policy::AssignmentLifecycleGate::Denied(_) => true,
            domain::effective_assignment_policy::AssignmentLifecycleGate::Open => {
                match &command.entitlement {
                    domain::entitlement::EntitlementDecision::Denied(_) => true,
                    domain::entitlement::EntitlementDecision::Granted(_) => matches!(
                        command.authorization,
                        domain::effective_assignment_policy::AuthorizationGate::Denied(_)
                    ),
                }
            }
        };
        let inputs = if should_short_circuit {
            // Keep the store boundary observationally identical to the pure
            // resolver: denied gates are decided before grant validation or
            // any M1--M4 read. These inert values cannot affect a denial.
            inert_effective_policy_inputs()?
        } else {
            let domain::entitlement::EntitlementDecision::Granted(grant) = &command.entitlement
            else {
                unreachable!("the preceding gate check handles denied entitlement");
            };
            if grant.tenant() != tenant
                || grant.course() != record.course_id
                || grant.assignment() != command.assignment
            {
                return Err(StoreError::InvalidRecord(
                    "effective-policy entitlement does not bind this assignment".to_string(),
                ));
            }
            memory_effective_policy_inputs_for_grant(&state, tenant, command.assignment, grant)?
        };
        let decision = resolve_effective_policy(ResolveEffectivePolicyInput {
            lifecycle: command.lifecycle,
            entitlement: command.entitlement,
            authorization: command.authorization,
            now: command.now,
            prior_run_count: command.prior_run_count,
            base: inputs.base,
            group_schedule_offsets: inputs.schedule_offsets,
            group_accommodations: inputs.accommodations,
            individual_exception: inputs.individual,
        })
        .map_err(|error| {
            StoreError::InvalidRecord(format!("invalid effective policy inputs: {error:?}"))
        })?;
        Ok(Some(EffectivePolicyResolution {
            tenant,
            course: record.course_id,
            assignment: command.assignment,
            decision,
            revision: *state
                .assignment_revisions
                .get(&(tenant, command.assignment))
                .ok_or(StoreError::NotFound)?,
        }))
    }

    async fn get_issued_effective_policy_receipt_impl(
        &self,
        context: TenantContext,
        attempt: QuestionAttemptId,
    ) -> Result<Option<IssuedEffectivePolicyReceipt>, StoreError> {
        let state = self.read_state()?;
        let tenant = context.tenant_id();
        Ok(state
            .attempt_effective_policy_current
            .get(&(tenant, attempt))
            .and_then(|generation| {
                state
                    .issued_effective_policy_receipts
                    .get(&(tenant, attempt, *generation))
                    .cloned()
            }))
    }
}

fn inert_effective_policy_inputs() -> Result<crate::EffectivePolicyInputs, StoreError> {
    Ok(crate::EffectivePolicyInputs {
        base: base_policy_from_editor_timing(None)?,
        schedule_offsets: Vec::new(),
        accommodations: Vec::new(),
        individual: None,
    })
}

/// Resolves a current S5 grant against the policy rows held in this immutable
/// Memory snapshot.  S5 has already selected the applicable group scopes; this
/// helper must not rediscover course-group membership while loading M2 or M3.
pub(super) fn resolve_granted_memory_effective_policy(
    state: &State,
    tenant: TenantId,
    assignment: &AssignmentRecord,
    grant: EntitlementGrant,
    prior_run_count: u32,
) -> Result<EffectivePolicyDecision, StoreError> {
    if grant.tenant() != tenant
        || grant.course() != assignment.course_id
        || grant.assignment() != assignment.id
    {
        return Err(StoreError::InvalidRecord(
            "effective-policy entitlement does not bind this assignment".to_string(),
        ));
    }
    let inputs = memory_effective_policy_inputs_for_grant(state, tenant, assignment.id, &grant)?;
    resolve_effective_policy(ResolveEffectivePolicyInput {
        lifecycle: stored_assignment_lifecycle_gate(assignment),
        entitlement: domain::entitlement::EntitlementDecision::Granted(grant),
        authorization: AuthorizationGate::Authorized,
        now: state.authoritative_time,
        prior_run_count,
        base: inputs.base,
        group_schedule_offsets: inputs.schedule_offsets,
        group_accommodations: inputs.accommodations,
        individual_exception: inputs.individual,
    })
    .map_err(|error| {
        StoreError::InvalidRecord(format!("invalid effective policy inputs: {error:?}"))
    })
}

fn stored_assignment_lifecycle_gate(_assignment: &AssignmentRecord) -> AssignmentLifecycleGate {
    // The current Memory assignment record is persisted only after publication;
    // T1 will introduce its explicit lifecycle field and this seam then maps it
    // to the same closed domain gate used by PostgreSQL.
    AssignmentLifecycleGate::Open
}

/// Loads the S3 modifier inputs selected by an already-evaluated S5 grant.
///
/// This is the sole Memory action/read input owner: it deliberately trusts the
/// grant's exact tenant, course, assignment, student, and applicable scope
/// bindings rather than rediscovering roster or group membership.
pub(super) fn memory_effective_policy_inputs_for_grant(
    state: &State,
    tenant: TenantId,
    assignment: AssignmentId,
    grant: &EntitlementGrant,
) -> Result<crate::EffectivePolicyInputs, StoreError> {
    let base = state
        .assignment_base_policy
        .get(&(tenant, assignment))
        .ok_or(StoreError::NotFound)?
        .policy;
    let schedule_scopes = grant
        .applicable_policy_scopes()
        .iter()
        .filter_map(|(group, purpose)| {
            question_model::GroupPurposeCapabilities::for_purpose(*purpose)
                .schedule_scope
                .then_some(*group)
        })
        .collect::<std::collections::BTreeSet<_>>();
    let accommodation_scopes = grant
        .applicable_policy_scopes()
        .iter()
        .filter_map(|(group, purpose)| {
            question_model::GroupPurposeCapabilities::for_purpose(*purpose)
                .accommodation_scope
                .then_some(*group)
        })
        .collect::<std::collections::BTreeSet<_>>();
    Ok(crate::EffectivePolicyInputs {
        base,
        schedule_offsets: state
            .assignment_group_schedule_offsets
            .iter()
            .filter_map(|((record_tenant, record_assignment, group), value)| {
                (*record_tenant == tenant
                    && *record_assignment == assignment
                    && schedule_scopes.contains(group))
                .then_some(*value)
            })
            .collect(),
        accommodations: state
            .assignment_group_accommodations
            .iter()
            .filter_map(|((record_tenant, record_assignment, group), value)| {
                (*record_tenant == tenant
                    && *record_assignment == assignment
                    && accommodation_scopes.contains(group))
                .then_some(*value)
            })
            .collect(),
        individual: state
            .assignment_individual_policy_exceptions
            .get(&(tenant, assignment, grant.student()))
            .map(|value| value.exception),
    })
}

pub(super) fn base_policy_from_editor_timing(
    time_limit_seconds: Option<u32>,
) -> Result<domain::effective_assignment_policy::BaseAssignmentPolicy, StoreError> {
    Ok(domain::effective_assignment_policy::BaseAssignmentPolicy {
        available_at: None,
        due_at: None,
        closes_at: None,
        time_limit_seconds: match time_limit_seconds {
            Some(value) => Some(std::num::NonZeroU32::new(value).ok_or_else(|| {
                StoreError::InvalidRecord("assignment time limit must be nonzero".to_string())
            })?),
            None => None,
        },
        attempt_limit: None,
        late_submission: question_model::LateSubmissionPolicy::Accept,
        deadline_behavior: question_model::AssignmentDeadlineBehavior::AutoSubmit,
    })
}

pub(super) fn memory_assignment_has_run(state: &State, assignment: &AssignmentRecord) -> bool {
    state.runs.values().any(|run| {
        state
            .enrollments
            .get(&(run.tenant, run.enrollment))
            .is_some_and(|enrollment| {
                enrollment.tenant == assignment.tenant && enrollment.assignment == assignment.id
            })
    })
}

pub(super) fn memory_assignment_has_results(state: &State, assignment: &AssignmentRecord) -> bool {
    state.submissions.values().any(|submission| {
        let attempt = &submission.record.attempt;
        attempt.result.is_some()
            && state
                .runs
                .get(&(attempt.tenant, attempt.run))
                .and_then(|run| state.enrollments.get(&(run.tenant, run.enrollment)))
                .is_some_and(|enrollment| enrollment.assignment == assignment.id)
    })
}

pub(super) fn validate_memory_assignment_references(
    state: &State,
    context: TenantContext,
    assignment: &AssignmentRecord,
) -> Result<(), StoreError> {
    if !state
        .courses
        .contains_key(&(assignment.tenant, assignment.course_id))
    {
        return Err(StoreError::InvalidRecord(
            "assignment references a missing course".to_string(),
        ));
    }
    for reference in assignment.references() {
        let assignable = state
            .published
            .get(&(reference.problem, reference.version))
            .is_some_and(|record| {
                record.lifecycle.is_assignable()
                    && catalog_record_visible(state, context.tenant_id(), record)
            });
        if !assignable {
            return Err(StoreError::InvalidRecord(format!(
                "assignment references a missing, hidden, or inactive published version {}/{}",
                reference.problem, reference.version
            )));
        }
    }
    Ok(())
}

pub(super) fn store_issued_effective_policy_receipt(
    state: &mut State,
    tenant: TenantId,
    attempt: QuestionAttemptId,
    policy: domain::effective_assignment_policy::EffectiveAssignmentPolicy,
) -> Result<(), StoreError> {
    let generation = state
        .attempt_effective_policy_current
        .get(&(tenant, attempt))
        .copied()
        .unwrap_or(0)
        .checked_add(1)
        .ok_or_else(|| {
            StoreError::Unavailable("effective policy receipt generation overflow".to_string())
        })?;
    let receipt = IssuedEffectivePolicyReceipt {
        attempt,
        generation,
        policy,
    };
    for source in receipt.field_sources() {
        state.issued_effective_policy_field_sources.insert(
            (
                tenant,
                attempt,
                generation,
                source.field,
                source.source_order,
            ),
            source,
        );
    }
    state
        .issued_effective_policy_receipts
        .insert((tenant, attempt, generation), receipt);
    state
        .attempt_effective_policy_current
        .insert((tenant, attempt), generation);
    Ok(())
}

fn authorize_policy_editor(
    state: &State,
    tenant: TenantId,
    course: CourseId,
    assignment: AssignmentId,
    actor: UserId,
) -> Result<(), StoreError> {
    let record = state
        .assignments
        .get(&(tenant, assignment))
        .ok_or(StoreError::NotFound)?;
    if record.course_id != course
        || super::entitlement::current_course_role(state, tenant, course, actor)
            != Some(CourseMembershipRole::Instructor)
    {
        return Err(StoreError::NotFound);
    }
    require_course_records_accessible(state, tenant, course)
}

fn require_group_course(
    state: &State,
    tenant: TenantId,
    course: CourseId,
    group: CourseGroupId,
) -> Result<(), StoreError> {
    state
        .course_groups
        .get(&(tenant, group))
        .filter(|record| record.course == course)
        .map(|_| ())
        .ok_or(StoreError::NotFound)
}

fn require_expected_revision(
    state: &State,
    tenant: TenantId,
    assignment: AssignmentId,
    expected: AssignmentRevision,
) -> Result<AssignmentRevision, StoreError> {
    let current = *state
        .assignment_revisions
        .get(&(tenant, assignment))
        .ok_or(StoreError::NotFound)?;
    if current != expected {
        return Err(StoreError::Conflict);
    }
    current.next()
}

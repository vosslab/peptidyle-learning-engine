use async_trait::async_trait;
use domain::{
    effective_assignment_policy::{
        AssignmentLifecycleGate, AuthorizationGate, EffectivePolicyDecision,
        ResolveEffectivePolicyInput, resolve_effective_policy,
        validate_base_assignment_policy_for_course_term,
    },
    entitlement::EntitlementGrant,
};

use super::*;
use crate::assignment_revision_checked_next;

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

    async fn put_assignment_teaching_settings_impl(
        &self,
        context: TenantContext,
        command: PutAssignmentTeachingSettingsCommand,
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
        let course_term = state
            .courses
            .get(&(tenant, command.course))
            .ok_or(StoreError::NotFound)?
            .term
            .clone();
        validate_base_assignment_policy_for_course_term(command.settings.base_policy, &course_term)
            .map_err(|error| {
                StoreError::InvalidRecord(format!(
                    "invalid assignment teaching settings: {error:?}"
                ))
            })?;
        let revision = require_expected_revision(
            &state,
            tenant,
            command.assignment,
            command.expected_revision,
        )?;
        let assignment = state
            .assignments
            .get(&(tenant, command.assignment))
            .cloned()
            .ok_or(StoreError::NotFound)?;
        if !domain::effective_assignment_policy::is_legal_assignment_lifecycle_transition(
            assignment.lifecycle,
            command.settings.lifecycle,
        ) {
            return Err(StoreError::InvalidRecord(
                "illegal assignment lifecycle transition".to_string(),
            ));
        }
        let mut updated = assignment.clone();
        updated.lifecycle = command.settings.lifecycle;
        updated.instructions = command.settings.instructions.clone();
        validate_assignment(&updated)?;
        // The settings, record revision, and each mutable active-attempt
        // projection form one transaction in Memory.  Keep a complete
        // snapshot so every validation or re-resolution failure is atomic.
        let snapshot = state.clone();
        let record = StoredBaseAssignmentPolicy {
            tenant,
            course: command.course,
            assignment: command.assignment,
            policy: command.settings.base_policy,
            revision,
        };
        state
            .assignment_base_policy
            .insert((tenant, command.assignment), record);
        state
            .assignments
            .insert((tenant, command.assignment), updated);
        state
            .assignment_revisions
            .insert((tenant, command.assignment), revision);
        if let Err(error) = reresolve_active_assignment_attempts(
            &mut state,
            tenant,
            command.course,
            command.assignment,
        ) {
            *state = snapshot;
            return Err(error);
        }
        if let Err(error) = super::curriculum_adoption::advance_course_schedule_revision(
            &mut state,
            tenant,
            command.course,
        ) {
            *state = snapshot;
            return Err(error);
        }
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
        require_group_capability(
            &state,
            tenant,
            command.course,
            command.offset.group,
            |capabilities| capabilities.schedule_scope,
        )?;
        let revision = require_expected_revision(
            &state,
            tenant,
            command.assignment,
            command.expected_revision,
        )?;
        let snapshot = state.clone();
        state.assignment_group_schedule_offsets.insert(
            (tenant, command.assignment, command.offset.group),
            command.offset,
        );
        state
            .assignment_revisions
            .insert((tenant, command.assignment), revision);
        if let Err(error) = reresolve_active_assignment_attempts(
            &mut state,
            tenant,
            command.course,
            command.assignment,
        ) {
            *state = snapshot;
            return Err(error);
        }
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
        require_group_capability(
            &state,
            tenant,
            command.course,
            command.group,
            |capabilities| capabilities.schedule_scope,
        )?;
        let revision = require_expected_revision(
            &state,
            tenant,
            command.assignment,
            command.expected_revision,
        )?;
        let snapshot = state.clone();
        state
            .assignment_group_schedule_offsets
            .remove(&(tenant, command.assignment, command.group))
            .ok_or(StoreError::NotFound)?;
        state
            .assignment_revisions
            .insert((tenant, command.assignment), revision);
        if let Err(error) = reresolve_active_assignment_attempts(
            &mut state,
            tenant,
            command.course,
            command.assignment,
        ) {
            *state = snapshot;
            return Err(error);
        }
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
        require_group_capability(
            &state,
            tenant,
            command.course,
            command.accommodation.group,
            |capabilities| capabilities.accommodation_scope,
        )?;
        let revision = require_expected_revision(
            &state,
            tenant,
            command.assignment,
            command.expected_revision,
        )?;
        let snapshot = state.clone();
        state.assignment_group_accommodations.insert(
            (tenant, command.assignment, command.accommodation.group),
            command.accommodation,
        );
        state
            .assignment_revisions
            .insert((tenant, command.assignment), revision);
        if let Err(error) = reresolve_active_assignment_attempts(
            &mut state,
            tenant,
            command.course,
            command.assignment,
        ) {
            *state = snapshot;
            return Err(error);
        }
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
        require_group_capability(
            &state,
            tenant,
            command.course,
            command.group,
            |capabilities| capabilities.accommodation_scope,
        )?;
        let revision = require_expected_revision(
            &state,
            tenant,
            command.assignment,
            command.expected_revision,
        )?;
        let snapshot = state.clone();
        state
            .assignment_group_accommodations
            .remove(&(tenant, command.assignment, command.group))
            .ok_or(StoreError::NotFound)?;
        state
            .assignment_revisions
            .insert((tenant, command.assignment), revision);
        if let Err(error) = reresolve_active_assignment_attempts(
            &mut state,
            tenant,
            command.course,
            command.assignment,
        ) {
            *state = snapshot;
            return Err(error);
        }
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
        let valid_student = state.course_memberships.values().any(|membership| {
            membership.tenant == tenant
                && membership.course == command.course
                && membership.student == Some(student)
                && membership.role == CourseMembershipRole::Student
                && membership.status == crate::CourseMemberStatus::Active
        });
        if !valid_student {
            return Err(StoreError::NotFound);
        }
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
        let snapshot = state.clone();
        state
            .assignment_individual_policy_exceptions
            .insert((tenant, command.assignment, student), command.exception);
        state
            .assignment_revisions
            .insert((tenant, command.assignment), revision);
        if let Err(error) = reresolve_active_assignment_attempts(
            &mut state,
            tenant,
            command.course,
            command.assignment,
        ) {
            *state = snapshot;
            return Err(error);
        }
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
        let snapshot = state.clone();
        state
            .assignment_individual_policy_exceptions
            .remove(&(tenant, command.assignment, command.student))
            .ok_or(StoreError::NotFound)?;
        state
            .assignment_revisions
            .insert((tenant, command.assignment), revision);
        if let Err(error) = reresolve_active_assignment_attempts(
            &mut state,
            tenant,
            command.course,
            command.assignment,
        ) {
            *state = snapshot;
            return Err(error);
        }
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
        let lifecycle = stored_assignment_lifecycle_gate(record);
        let should_short_circuit = match lifecycle {
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
            lifecycle,
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
        base: domain::effective_assignment_policy::BaseAssignmentPolicy::default(),
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

fn stored_assignment_lifecycle_gate(assignment: &AssignmentRecord) -> AssignmentLifecycleGate {
    domain::effective_assignment_policy::assignment_lifecycle_gate(assignment.lifecycle)
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
    // A hypothetical exception is a preview-only resolver input.  Receipt
    // history is durable authority for a real issued attempt, so reject every
    // hypothetical field before calculating a generation or changing any map.
    for source in receipt_policy_sources(&policy) {
        if matches!(
            source,
            domain::effective_assignment_policy::PolicySource::HypotheticalIndividualException
        ) {
            return Err(StoreError::InvalidRecord(
                "hypothetical individual exceptions cannot be persisted in effective-policy receipts"
                    .to_string(),
            ));
        }
    }
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

fn receipt_policy_sources(
    policy: &domain::effective_assignment_policy::EffectiveAssignmentPolicy,
) -> [&domain::effective_assignment_policy::PolicySource; 7] {
    [
        &policy.available_at.source,
        &policy.due_at.source,
        &policy.closes_at.source,
        &policy.time_limit_seconds.source,
        &policy.attempt_limit.source,
        &policy.late_submission.source,
        &policy.deadline_behavior.source,
    ]
}

/// Replaces only the mutable current S3 receipt for active attempts after an
/// instructor changes the assignment teaching settings.  The historical
/// receipt generations remain sealed evidence.  A current entitlement or
/// lifecycle/window denial terminalizes the attempt and removes its mutable
/// current pointer instead of rewriting history.
pub(super) fn reresolve_active_assignment_attempts(
    state: &mut State,
    tenant: TenantId,
    course: CourseId,
    assignment: AssignmentId,
) -> Result<(), StoreError> {
    let record = state
        .assignments
        .get(&(tenant, assignment))
        .cloned()
        .ok_or(StoreError::NotFound)?;
    let active_attempts = state
        .attempts
        .values()
        .filter(|attempt| attempt.tenant == tenant)
        .filter(|attempt| {
            projected_attempt(state, tenant, attempt).status == AttemptStatus::InProgress
        })
        .filter_map(|attempt| {
            let run = state.runs.get(&(tenant, attempt.run))?;
            let enrollment = state.enrollments.get(&(tenant, run.enrollment))?;
            (enrollment.assignment == assignment).then_some((
                attempt.clone(),
                run.clone(),
                enrollment.clone(),
            ))
        })
        .collect::<Vec<_>>();

    for (attempt, run, enrollment) in active_attempts {
        let grant = match super::entitlement::evaluate_locked(
            state,
            tenant,
            enrollment.user,
            course,
            assignment,
        )? {
            domain::entitlement::EntitlementDecision::Granted(grant)
                if grant.student() == enrollment.student =>
            {
                Some(grant)
            }
            domain::entitlement::EntitlementDecision::Granted(_)
            | domain::entitlement::EntitlementDecision::Denied(_) => None,
        };
        let policy = match grant {
            Some(grant) => match resolve_granted_memory_effective_policy(
                state,
                tenant,
                &record,
                grant,
                run.run_number.saturating_sub(1),
            )? {
                EffectivePolicyDecision::Allowed {
                    policy,
                    start: domain::effective_assignment_policy::StartVerdict::MayStart { .. },
                } => Some(policy),
                EffectivePolicyDecision::Allowed { .. }
                | EffectivePolicyDecision::Denied { .. } => None,
            },
            None => None,
        };
        let Some(policy) = policy else {
            let mut terminal = projected_attempt(state, tenant, &attempt);
            terminal.status = AttemptStatus::AutoSubmitted;
            terminal.timer.submitted_at = Some(state.authoritative_time);
            state.attempt_current.insert((tenant, attempt.id), terminal);
            super::complete_memory_attempt_timing_job(state, tenant, attempt.id);
            state
                .attempt_effective_policy_current
                .remove(&(tenant, attempt.id));
            continue;
        };

        let timing = state
            .attempt_timing
            .get(&(tenant, attempt.id))
            .copied()
            .ok_or_else(|| {
                StoreError::Unavailable("active attempt is missing timing state".to_string())
            })?;
        let (effective_deadline, effective_grace_seconds, auto_submit_at) =
            super::runs::effective_attempt_deadline(
                &run,
                timing.authored_deadline,
                timing.authored_grace_seconds,
                &policy,
            )?;
        if effective_deadline.is_some_and(|deadline| deadline < state.authoritative_time)
            || auto_submit_at.is_some_and(|deadline| deadline <= state.authoritative_time)
        {
            let mut terminal = projected_attempt(state, tenant, &attempt);
            terminal.status = AttemptStatus::AutoSubmitted;
            terminal.timer.deadline = effective_deadline;
            terminal.timer.submitted_at = Some(state.authoritative_time);
            state.attempt_current.insert((tenant, attempt.id), terminal);
            super::complete_memory_attempt_timing_job(state, tenant, attempt.id);
            state
                .attempt_effective_policy_current
                .remove(&(tenant, attempt.id));
            continue;
        }

        super::complete_memory_attempt_timing_job(state, tenant, attempt.id);
        let next_generation = timing.generation.checked_add(1).ok_or_else(|| {
            StoreError::Unavailable("attempt timing generation overflow".to_string())
        })?;
        let job = match auto_submit_at {
            Some(available_at) => {
                let job = JobId::generate()?;
                state.jobs.insert(
                    job,
                    StoredJob {
                        tenant,
                        payload: JobPayload::AutoSubmitAttempt {
                            attempt: attempt.id,
                            timing_generation: next_generation,
                        },
                        state: JobState::Ready,
                        available_at,
                        lease_token: None,
                        lease_expires_at: None,
                        attempt_count: 0,
                        max_attempts: 10,
                        failure: None,
                    },
                );
                Some(job)
            }
            None => None,
        };
        state.attempt_timing.insert(
            (tenant, attempt.id),
            MemoryAttemptTiming {
                assignment,
                authored_deadline: timing.authored_deadline,
                authored_grace_seconds: timing.authored_grace_seconds,
                effective_deadline,
                effective_grace_seconds,
                auto_submit_at,
                generation: next_generation,
                job,
            },
        );
        let mut current = projected_attempt(state, tenant, &attempt);
        current.timer.deadline = effective_deadline;
        state.attempt_current.insert((tenant, attempt.id), current);
        store_issued_effective_policy_receipt(state, tenant, attempt.id, *policy)?;
    }
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

fn require_group_capability(
    state: &State,
    tenant: TenantId,
    course: CourseId,
    group: CourseGroupId,
    permits: impl FnOnce(question_model::GroupPurposeCapabilities) -> bool,
) -> Result<(), StoreError> {
    let record = state
        .course_groups
        .get(&(tenant, group))
        .filter(|record| record.course == course)
        .ok_or(StoreError::NotFound)?;
    permits(question_model::GroupPurposeCapabilities::for_purpose(
        record.purpose,
    ))
    .then_some(())
    .ok_or_else(|| {
        StoreError::InvalidRecord("group purpose cannot own this policy modifier".into())
    })
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
    assignment_revision_checked_next(current)
}

#[cfg(test)]
#[path = "course_policy/receipt_tests.rs"]
mod receipt_tests;

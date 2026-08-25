use async_trait::async_trait;

use super::*;
use crate::{LearnerWorkRoutingBinding, PrefetchedQuestionDescriptorV1, ReceiptNextAttempt};

mod attempt_issuance;
mod issued_contracts;
mod private_execution;
pub(super) mod submission_preparation;

pub(super) use attempt_issuance::effective_attempt_deadline;
mod learner_reads;
mod pending_submissions;

pub(super) use issued_contracts::{
    load_issued_presentation, load_issued_receipt_evidence, load_submission_record,
};

#[async_trait]
impl crate::RunStore for MemoryStore {
    async fn prepare_question_submission_impl(
        &self,
        context: TenantContext,
        actor: UserId,
        binding: LearnerWorkRoutingBinding,
        attempt: QuestionAttemptId,
        response: &StudentResponse,
        idempotency_key: &SubmissionIdempotencyKey,
    ) -> Result<crate::SubmissionPreparation, StoreError> {
        let state = self.read_state()?;
        submission_preparation::prepare_question_submission(
            &state,
            context,
            actor,
            binding,
            attempt,
            response,
            idempotency_key,
        )
    }

    async fn learner_assignment_run_items_impl(
        &self,
        context: TenantContext,
        actor: UserId,
        run: RunId,
    ) -> Result<Option<Vec<AssignmentRunItem>>, StoreError> {
        learner_reads::assignment_run_items(self, context, actor, run).await
    }
    async fn start_or_resume_run_impl(
        &self,
        context: TenantContext,
        actor: UserId,
        binding: LearnerWorkRoutingBinding,
        proposed_run: RunId,
    ) -> Result<AssignmentRun, StoreError> {
        let mut state = self.write_state()?;
        let tenant = context.tenant_id();
        // ASVS V8.2.2/V8.3.1: the route binding is not authority. Resolve the learner's current
        // membership for its asserted course before looking up the assignment,
        // so a known assignment ID cannot select a course for the caller.
        super::entitlement::active_membership_for(&state, tenant, binding.course, actor)
            .filter(|membership| {
                membership.role == CourseMembershipRole::Student && membership.student.is_some()
            })
            .ok_or(StoreError::NotFound)?;
        let assignment_id = binding.assignment;
        let assignment = assignment_record(&state, tenant, assignment_id)?;
        if assignment.course_id != binding.course {
            return Err(StoreError::NotFound);
        }
        require_course_records_accessible(&state, tenant, binding.course)?;
        // S5 decides current access first, but a grant is not yet a receipt:
        // S3 must reject an unavailable, closed, or exhausted policy without
        // leaving enrollment evidence behind.
        let domain::entitlement::EntitlementDecision::Granted(grant) =
            super::entitlement::evaluate_locked(
                &state,
                tenant,
                actor,
                binding.course,
                assignment_id,
            )?
        else {
            return Err(StoreError::NotFound);
        };
        let now = state.authoritative_time;
        let completed_prior_run_count = state
            .runs
            .values()
            .filter(|run| {
                run.tenant == tenant
                    && run.completed_at.is_some()
                    && state
                        .enrollments
                        .get(&(tenant, run.enrollment))
                        .is_some_and(|enrollment| {
                            enrollment.assignment == assignment_id
                                && enrollment.student == grant.student()
                        })
            })
            .count();
        let inputs =
            memory_effective_policy_inputs_for_grant(&state, tenant, assignment_id, &grant)?;
        let decision = domain::effective_assignment_policy::resolve_effective_policy(
            domain::effective_assignment_policy::ResolveEffectivePolicyInput {
                lifecycle: domain::effective_assignment_policy::assignment_lifecycle_gate(
                    assignment.lifecycle,
                ),
                entitlement: domain::entitlement::EntitlementDecision::Granted(grant.clone()),
                authorization: domain::effective_assignment_policy::AuthorizationGate::Authorized,
                now,
                prior_run_count: u32::try_from(completed_prior_run_count).map_err(|_| {
                    StoreError::Unavailable("run count exceeds policy range".to_string())
                })?,
                base: inputs.base,
                group_schedule_offsets: inputs.schedule_offsets,
                group_accommodations: inputs.accommodations,
                individual_exception: inputs.individual,
            },
        )
        .map_err(|error| {
            StoreError::InvalidRecord(format!("invalid effective policy inputs: {error:?}"))
        })?;
        let domain::effective_assignment_policy::EffectivePolicyDecision::Allowed {
            start: domain::effective_assignment_policy::StartVerdict::MayStart { .. },
            ..
        } = decision
        else {
            return Err(StoreError::NotFound);
        };
        let existing_enrollment = state
            .enrollments
            .values()
            .find(|enrollment| {
                enrollment.tenant == tenant
                    && enrollment.assignment == assignment_id
                    && enrollment.student == grant.student()
            })
            .cloned();
        if let Some(enrollment) = &existing_enrollment {
            if let Some(active) = state.runs.values().find(|run| {
                run.tenant == tenant
                    && run.enrollment == enrollment.id
                    && run.completed_at.is_none()
            }) {
                return Ok(active.clone());
            }
            let previous = state
                .summaries
                .get(&(tenant, enrollment.id))
                .ok_or(StoreError::NotFound)?;
            if !continued_practice_allows_run(previous, assignment.policies.continued_practice) {
                return Err(StoreError::InvalidRecord(
                    "continued-practice policy does not permit another run".to_string(),
                ));
            }
        }
        if state.runs.contains_key(&(tenant, proposed_run)) {
            return Err(StoreError::AlreadyExists);
        }
        let entitlement = super::entitlement::materialize_locked(
            &mut state,
            tenant,
            crate::MaterializeAssignmentEntitlementCommand::for_learner_action(
                actor,
                binding.course,
                assignment_id,
                question_model::EntitlementPurpose::StartRun,
            )?,
        )?;
        let crate::AssignmentEntitlementMaterialization::Granted(entitlement) = entitlement else {
            return Err(StoreError::NotFound);
        };
        let enrollment = entitlement.enrollment;
        let previous = state
            .summaries
            .get(&(tenant, enrollment.id))
            .cloned()
            .ok_or(StoreError::NotFound)?;
        let run_number = state
            .runs
            .values()
            .filter(|run| run.tenant == tenant && run.enrollment == enrollment.id)
            .map(|run| run.run_number)
            .max()
            .unwrap_or(0)
            .checked_add(1)
            .ok_or_else(|| StoreError::InvalidRecord("run number overflow".to_string()))?;
        let public_id =
            super::navigation_references::ensure_run_reference(&mut state, tenant, proposed_run)?;
        let run = AssignmentRun {
            id: proposed_run,
            reference: public_id,
            tenant,
            enrollment: enrollment.id,
            run_number,
            started_at: state.authoritative_time,
            completed_at: None,
            score: None,
            mode: match enrollment.status() {
                EnrollmentStatus::InProgress => RunMode::Assigned,
                EnrollmentStatus::Completed => RunMode::Practice,
            },
            variation: assignment.policies.variation,
        };
        let next = project_summary(
            &previous,
            summary_transition(&ActivityTransition::StartRun { run: run.clone() }),
            grade_policy(&assignment),
        )?;
        let run_items = select_assignment_run_items(&assignment, &run)?;
        state.runs.insert((tenant, run.id), run.clone());
        state.run_items.insert((tenant, run.id), run_items);
        state.summaries.insert((tenant, enrollment.id), next);
        Ok(run)
    }
    async fn assignment_run_items_impl(
        &self,
        context: TenantContext,
        run: RunId,
    ) -> Result<Vec<AssignmentRunItem>, StoreError> {
        let state = self.read_state()?;
        if !state.runs.contains_key(&(context.tenant_id(), run)) {
            return Err(StoreError::NotFound);
        }
        Ok(state
            .run_items
            .get(&(context.tenant_id(), run))
            .cloned()
            .unwrap_or_default())
    }
    async fn issue_or_resume_question_attempt_impl(
        &self,
        context: TenantContext,
        command: IssueQuestionAttemptCommand,
    ) -> Result<QuestionAttempt, StoreError> {
        attempt_issuance::issue_or_resume_question_attempt(self, context, command).await
    }
    async fn read_issued_attempt_evidence_impl(
        &self,
        context: TenantContext,
        actor: UserId,
        binding: LearnerWorkRoutingBinding,
        attempt: QuestionAttemptId,
    ) -> Result<crate::IssuedAttemptRead, StoreError> {
        let state = self.read_state()?;
        let tenant = context.tenant_id();
        // Establish the live Student route authority before the opaque attempt
        // lookup, mirroring the broker-first PostgreSQL capability (ASVS
        // 1.2.4, 1.5.2, 2.2.1-2.2.3, 2.3.3, 8.2.2/8.3.1/8.4.1, 11.4.3,
        // 14.2.6, 15.4.2, and 16.5.3).
        super::entitlement::active_membership_for(&state, tenant, binding.course, actor)
            .filter(|membership| {
                membership.role == CourseMembershipRole::Student && membership.student.is_some()
            })
            .ok_or(StoreError::NotFound)?;
        let assignment = assignment_record(&state, tenant, binding.assignment)?;
        if assignment.course_id != binding.course {
            return Err(StoreError::NotFound);
        }
        require_course_records_accessible(&state, tenant, binding.course)?;
        let domain::entitlement::EntitlementDecision::Granted(grant) =
            super::entitlement::evaluate_locked(
                &state,
                tenant,
                actor,
                binding.course,
                binding.assignment,
            )?
        else {
            return Err(StoreError::NotFound);
        };
        let record = state
            .attempts
            .get(&(tenant, attempt))
            .ok_or(StoreError::NotFound)?;
        let current_attempt = projected_attempt(&state, tenant, record);
        let run = state
            .runs
            .get(&(tenant, record.run))
            .ok_or(StoreError::NotFound)?;
        let enrollment = enrollment_record(&state, tenant, run.enrollment)?;
        if enrollment.assignment != binding.assignment
            || enrollment.user != actor
            || enrollment.student != grant.student()
        {
            return Err(StoreError::NotFound);
        }
        // The immutable issue record provides only receipt evidence. Current
        // delivery lifecycle comes from the separately projected attempt
        // state, matching PostgreSQL's relational status witness (ASVS
        // 1.5.2, 2.2.1-2.2.3, 8.2.2/8.3.1, 14.2.6, and 16.5.3).
        if record.status != AttemptStatus::InProgress
            || record.response.is_some()
            || record.result.is_some()
            || record.timer.submitted_at.is_some()
        {
            return Err(StoreError::Unavailable(
                "stored issued attempt payload does not describe issuance".to_string(),
            ));
        }
        if current_attempt.status == AttemptStatus::InProgress
            && current_attempt.timer.submitted_at.is_some()
        {
            return Err(StoreError::Unavailable(
                "in-progress attempt carries a current submission time".to_string(),
            ));
        }
        if current_attempt.status == AttemptStatus::Submitted
            && current_attempt.timer.submitted_at.is_none()
        {
            return Err(StoreError::Unavailable(
                "submitted attempt lacks its current submission time".to_string(),
            ));
        }
        let presentation = load_issued_receipt_evidence(&state, tenant, record)?;
        let presentation_binding = state.attempt_presentations.get(&(tenant, attempt)).copied();
        let grading_envelope = state
            .attempt_grading_envelopes
            .get(&(tenant, attempt))
            .cloned();
        let receipt = crate::IssuedAttemptReceiptEvidence::new(
            presentation_binding,
            presentation,
            grading_envelope,
        );
        match current_attempt.status {
            AttemptStatus::InProgress => Ok(crate::IssuedAttemptRead::Active(Box::new(
                crate::ActiveIssuedAttemptEvidence::new(receipt),
            ))),
            AttemptStatus::Submitted => {
                let stored = state.submissions.get(&(tenant, attempt)).ok_or_else(|| {
                    StoreError::Unavailable(
                        "submitted attempt lacks its immutable receipt".to_string(),
                    )
                })?;
                if stored.record.attempt.id != attempt
                    || stored.record.attempt.run != current_attempt.run
                    || stored.record.presentation != receipt.presentation_snapshot().cloned()
                {
                    return Err(StoreError::Unavailable(
                        "submitted receipt disagrees with issued evidence".to_string(),
                    ));
                }
                Ok(crate::IssuedAttemptRead::Submitted(Box::new(
                    crate::SubmittedIssuedAttemptRead::new(
                        receipt,
                        crate::SubmittedQuestionReceipt::new(stored.record.presentation.clone()),
                    ),
                )))
            }
            status => Ok(crate::IssuedAttemptRead::TerminalWithoutReceipt(Box::new(
                crate::TerminalIssuedAttemptRead::new(receipt, status),
            ))),
        }
    }
    async fn reserve_or_resume_prefetched_question_impl(
        &self,
        context: TenantContext,
        command: ReservePrefetchedQuestionCommand,
    ) -> Result<PrefetchedQuestionDescriptorV1, StoreError> {
        let mut state = self.write_state()?;
        let tenant = context.tenant_id();
        let reservation = command.reservation;
        let private_execution = command.private_execution;
        if reservation.tenant != tenant
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
        let run = state
            .runs
            .get(&(tenant, reservation.run))
            .ok_or(StoreError::NotFound)?;
        if run.completed_at.is_some() || run.score.is_some() {
            return Err(StoreError::Conflict);
        }
        let enrollment = enrollment_record(&state, tenant, run.enrollment)?;
        let predecessor = state
            .attempts
            .get(&(tenant, reservation.predecessor))
            .ok_or(StoreError::NotFound)?;
        if predecessor.run != reservation.run
            || state.submissions.contains_key(&(tenant, predecessor.id))
        {
            return Err(StoreError::Conflict);
        }
        if enrollment.assignment != command.binding.assignment {
            return Err(StoreError::NotFound);
        }
        let assignment = assignment_record(&state, tenant, command.binding.assignment)?;
        if assignment.course_id != command.binding.course {
            return Err(StoreError::NotFound);
        }
        require_course_records_accessible(&state, tenant, command.binding.course)?;
        super::entitlement::require_current_enrollment_entitlement(
            &state,
            tenant,
            command.actor,
            command.binding.course,
            command.binding.assignment,
            &enrollment,
        )?;
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
        if state.attempts.values().any(|attempt| {
            attempt.tenant == tenant
                && attempt.run == reservation.run
                && attempt.assignment_position == reservation.assignment_position
        }) {
            return Err(StoreError::Conflict);
        }
        let key = (
            tenant,
            reservation.run,
            reservation.predecessor,
            reservation.assignment_position,
        );
        if let Some(existing) = state.prefetched_questions.get(&key) {
            return if existing == &reservation
                && state.prefetched_private_execution.get(&key) == Some(&private_execution)
            {
                Ok(existing.clone())
            } else {
                Err(StoreError::Conflict)
            };
        }
        state.prefetched_questions.insert(key, reservation.clone());
        state
            .prefetched_private_execution
            .insert(key, private_execution);
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
        let state = self.read_state()?;
        let run_record = state
            .runs
            .get(&(context.tenant_id(), run))
            .ok_or(StoreError::NotFound)?;
        let enrollment = enrollment_record(&state, context.tenant_id(), run_record.enrollment)?;
        let assignment = assignment_record(&state, context.tenant_id(), enrollment.assignment)?;
        require_course_records_accessible(&state, context.tenant_id(), assignment.course_id)?;
        super::entitlement::require_current_enrollment_entitlement(
            &state,
            context.tenant_id(),
            actor,
            assignment.course_id,
            assignment.id,
            &enrollment,
        )?;
        Ok(state
            .prefetched_questions
            .get(&(context.tenant_id(), run, predecessor, assignment_position))
            .cloned())
    }
    async fn learner_get_prefetched_question_impl(
        &self,
        context: TenantContext,
        actor: UserId,
        run: RunId,
        predecessor: QuestionAttemptId,
        assignment_position: u32,
    ) -> Result<Option<PrefetchedQuestionDescriptorV1>, StoreError> {
        learner_reads::prefetched_question(
            self,
            context,
            actor,
            run,
            predecessor,
            assignment_position,
        )
        .await
    }
    async fn submission_next_attempt_impl(
        &self,
        context: TenantContext,
        actor: UserId,
        binding: LearnerWorkRoutingBinding,
        predecessor: QuestionAttemptId,
    ) -> Result<SubmissionNextAttempt, StoreError> {
        let state = self.read_state()?;
        let attempt = state
            .attempts
            .get(&(context.tenant_id(), predecessor))
            .ok_or(StoreError::NotFound)?;
        let run = state
            .runs
            .get(&(context.tenant_id(), attempt.run))
            .ok_or(StoreError::NotFound)?;
        let enrollment = enrollment_record(&state, context.tenant_id(), run.enrollment)?;
        let assignment = assignment_record(&state, context.tenant_id(), enrollment.assignment)?;
        if LearnerWorkRoutingBinding::new(assignment.course_id, assignment.id) != binding {
            return Err(StoreError::NotFound);
        }
        require_course_records_accessible(&state, context.tenant_id(), assignment.course_id)?;
        require_attempt_owner(&state, context.tenant_id(), attempt, actor)?;
        if !state
            .submissions
            .contains_key(&(context.tenant_id(), predecessor))
        {
            return Err(StoreError::Conflict);
        }
        Ok(
            match state
                .submission_next_attempts
                .get(&(context.tenant_id(), predecessor))
            {
                None => SubmissionNextAttempt::Pending,
                Some(None) => SubmissionNextAttempt::None,
                Some(Some(next)) => SubmissionNextAttempt::Issued(next.clone()),
            },
        )
    }
    async fn pending_submission_for_run_impl(
        &self,
        context: TenantContext,
        actor: UserId,
        run: RunId,
    ) -> Result<Option<QuestionAttemptId>, StoreError> {
        pending_submissions::pending_submission_for_run(self, context, actor, run).await
    }
    async fn learner_pending_submission_for_run_impl(
        &self,
        context: TenantContext,
        actor: UserId,
        run: RunId,
    ) -> Result<Option<QuestionAttemptId>, StoreError> {
        learner_reads::pending_submission_for_run(self, context, actor, run).await
    }
    async fn finalize_submission_next_attempt_impl(
        &self,
        context: TenantContext,
        actor: UserId,
        binding: LearnerWorkRoutingBinding,
        predecessor: QuestionAttemptId,
        next: Option<QuestionAttemptId>,
    ) -> Result<(), StoreError> {
        let mut state = self.write_state()?;
        let tenant = context.tenant_id();
        let attempt = state
            .attempts
            .get(&(tenant, predecessor))
            .ok_or(StoreError::NotFound)?
            .clone();
        let run = state
            .runs
            .get(&(tenant, attempt.run))
            .ok_or(StoreError::NotFound)?;
        let enrollment = enrollment_record(&state, tenant, run.enrollment)?;
        let assignment = assignment_record(&state, tenant, enrollment.assignment)?;
        if LearnerWorkRoutingBinding::new(assignment.course_id, assignment.id) != binding {
            return Err(StoreError::NotFound);
        }
        require_course_records_accessible(&state, tenant, assignment.course_id)?;
        require_attempt_owner(&state, tenant, &attempt, actor)?;
        if !state.submissions.contains_key(&(tenant, predecessor)) {
            return Err(StoreError::Conflict);
        }
        let next = if let Some(next) = next {
            let next_attempt = state
                .attempts
                .get(&(tenant, next))
                .ok_or(StoreError::NotFound)?;
            if next_attempt.run != attempt.run {
                return Err(StoreError::Conflict);
            }
            Some(ReceiptNextAttempt::from_attempt(next_attempt))
        } else {
            None
        };
        match state.submission_next_attempts.get(&(tenant, predecessor)) {
            Some(existing) if *existing != next => Err(StoreError::Conflict),
            _ => {
                state
                    .submission_next_attempts
                    .insert((tenant, predecessor), next);
                Ok(())
            }
        }
    }
    async fn list_question_attempts_impl(
        &self,
        context: TenantContext,
        run: RunId,
        page: PageRequest,
    ) -> Result<Page<QuestionAttempt>, StoreError> {
        let state = self.read_state()?;
        let run_record = state
            .runs
            .get(&(context.tenant_id(), run))
            .ok_or(StoreError::NotFound)?;
        let enrollment = enrollment_record(&state, context.tenant_id(), run_record.enrollment)?;
        let assignment = assignment_record(&state, context.tenant_id(), enrollment.assignment)?;
        require_course_records_accessible(&state, context.tenant_id(), assignment.course_id)?;
        let records = state
            .attempts
            .values()
            .filter(|attempt| attempt.tenant == context.tenant_id() && attempt.run == run)
            .map(|attempt| {
                let projected = projected_attempt(&state, context.tenant_id(), attempt);
                (
                    format!(
                        "{:010}/{:020}/{}",
                        projected.assignment_position,
                        projected.timer.issued_at.as_unix_millis(),
                        projected.id
                    ),
                    projected,
                )
            })
            .collect();
        Ok(page_records(records, &page))
    }
    async fn learner_list_question_attempts_impl(
        &self,
        context: TenantContext,
        actor: UserId,
        run: RunId,
        page: PageRequest,
    ) -> Result<Option<Page<QuestionAttempt>>, StoreError> {
        learner_reads::list_question_attempts(self, context, actor, run, page).await
    }
    async fn replay_submission_impl(
        &self,
        context: TenantContext,
        actor: UserId,
        attempt_id: QuestionAttemptId,
        response: &StudentResponse,
        idempotency_key: &SubmissionIdempotencyKey,
    ) -> Result<Option<SubmissionRecord>, StoreError> {
        let state = self.read_state()?;
        let tenant = context.tenant_id();
        let attempt = state
            .attempts
            .get(&(tenant, attempt_id))
            .ok_or(StoreError::NotFound)?;
        let run = state
            .runs
            .get(&(tenant, attempt.run))
            .ok_or(StoreError::NotFound)?;
        let enrollment = enrollment_record(&state, tenant, run.enrollment)?;
        let assignment = assignment_record(&state, tenant, enrollment.assignment)?;
        require_course_records_accessible(&state, tenant, assignment.course_id)?;
        require_attempt_owner(&state, tenant, attempt, actor)?;
        let Some(stored) = state.submissions.get(&(tenant, attempt_id)) else {
            return Ok(None);
        };
        if &stored.key != idempotency_key || &stored.response != response {
            return Err(StoreError::Conflict);
        }
        load_submission_record(&state, tenant, attempt)
    }
    async fn submission_record_impl(
        &self,
        context: TenantContext,
        actor: UserId,
        attempt_id: QuestionAttemptId,
    ) -> Result<Option<SubmissionRecord>, StoreError> {
        let state = self.read_state()?;
        let tenant = context.tenant_id();
        let attempt = state
            .attempts
            .get(&(tenant, attempt_id))
            .ok_or(StoreError::NotFound)?;
        let run = state
            .runs
            .get(&(tenant, attempt.run))
            .ok_or(StoreError::NotFound)?;
        let enrollment = enrollment_record(&state, tenant, run.enrollment)?;
        let assignment = assignment_record(&state, tenant, enrollment.assignment)?;
        require_course_records_accessible(&state, tenant, assignment.course_id)?;
        require_attempt_owner(&state, tenant, attempt, actor)?;
        load_submission_record(&state, tenant, attempt)
    }
    async fn submit_question_attempt_impl(
        &self,
        context: TenantContext,
        command: SubmitQuestionAttemptCommand,
    ) -> Result<SubmissionRecord, StoreError> {
        let mut state = self.write_state()?;
        let tenant = context.tenant_id();
        let attempt = state
            .attempts
            .get(&(tenant, command.attempt))
            .ok_or(StoreError::NotFound)?;
        let run = state
            .runs
            .get(&(tenant, attempt.run))
            .ok_or(StoreError::NotFound)?;
        let enrollment = enrollment_record(&state, tenant, run.enrollment)?;
        let assignment = assignment_record(&state, tenant, enrollment.assignment)?;
        require_course_records_accessible(&state, tenant, assignment.course_id)?;
        submit_question_attempt_locked(&mut state, context, command)
    }
    async fn force_submit_attempt_impl(
        &self,
        context: TenantContext,
        command: ForceSubmitAttemptCommand,
    ) -> Result<AttemptSupportRecord, StoreError> {
        let mut state = self.write_state()?;
        apply_memory_attempt_support(
            &mut state,
            context,
            command.action,
            command.actor,
            command.attempt,
            AttemptSupportAction::ForceSubmit,
        )
    }
    async fn clear_attempt_impl(
        &self,
        context: TenantContext,
        command: ClearAttemptCommand,
    ) -> Result<AttemptSupportRecord, StoreError> {
        let mut state = self.write_state()?;
        apply_memory_attempt_support(
            &mut state,
            context,
            command.action,
            command.actor,
            command.attempt,
            AttemptSupportAction::Clear,
        )
    }
}

pub(super) fn submit_question_attempt_locked(
    state: &mut State,
    context: TenantContext,
    command: SubmitQuestionAttemptCommand,
) -> Result<SubmissionRecord, StoreError> {
    let tenant = context.tenant_id();
    match submission_preparation::prepare_question_submission(
        state,
        context,
        command.actor,
        command.binding,
        command.attempt,
        &command.response,
        &command.idempotency_key,
    )? {
        crate::SubmissionPreparation::Replay(record) => return Ok(*record),
        crate::SubmissionPreparation::FirstEffect(_) => {}
    }
    let base = state
        .attempts
        .get(&(tenant, command.attempt))
        .cloned()
        .ok_or(StoreError::NotFound)?;
    require_attempt_owner(state, tenant, &base, command.actor)?;
    if let Some(matches_request) = state
        .submissions
        .get(&(tenant, command.attempt))
        .map(|stored| stored.key == command.idempotency_key && stored.response == command.response)
    {
        return if matches_request {
            load_submission_record(state, tenant, &base)?.ok_or_else(|| {
                StoreError::Unavailable("submission receipt disappeared during replay".to_string())
            })
        } else {
            Err(StoreError::Conflict)
        };
    }
    if projected_attempt(state, tenant, &base).status != AttemptStatus::InProgress {
        return Err(StoreError::Conflict);
    }
    // Validate the issuance-time snapshot before any receipt, attempt, or run
    // mutation. A submission only copies this owned value; it never rebuilds.
    let presentation = load_issued_presentation(state, tenant, &base)?;
    let feedback = private_feedback_record(command.feedback.clone())?;
    let mut run = state
        .runs
        .get(&(tenant, base.run))
        .cloned()
        .ok_or(StoreError::NotFound)?;
    if run.completed_at.is_some() || run.score.is_some() {
        return Err(StoreError::Conflict);
    }
    let mut enrollment = enrollment_record(state, tenant, run.enrollment)?;
    let assignment = assignment_record(state, tenant, enrollment.assignment)?;
    crate::validate_attempt_result(command.result)?;
    let submitted_at = state.authoritative_time;
    let mut submitted = projected_attempt(state, tenant, &base);
    submitted.response = Some(command.response.clone());
    submitted.status = AttemptStatus::Submitted;
    submitted.result = Some(command.result);
    submitted.timer.submitted_at = Some(submitted_at);
    let disclosure = super::feedback::current_disclosure_input(
        state,
        tenant,
        &assignment,
        command.attempt,
        submitted.timer.submitted_at,
    )?;
    let timing = state
        .attempt_timing
        .get(&(tenant, command.attempt))
        .ok_or_else(|| StoreError::Unavailable("issued timing authority is missing".to_string()))?;
    let effective_policy = timing
        .effective_deadline
        .map_or(TimingPolicy::Untimed, |_| TimingPolicy::PerQuestion {
            seconds: 1,
            grace_seconds: timing.effective_grace_seconds,
        });
    let verdict = timer_verdict(&TimerEvaluation {
        policy: effective_policy,
        timer: submitted.timer,
        evaluated_at: submitted_at,
        pause_extension_millis: 0,
    })
    .map_err(|error| StoreError::InvalidRecord(error.to_string()))?;
    if verdict == TimerVerdict::TimedOut {
        return Err(StoreError::TimedOut);
    }
    require_course_records_accessible(state, tenant, assignment.course_id)?;
    let previous = state
        .summaries
        .get(&(tenant, enrollment.id))
        .cloned()
        .ok_or(StoreError::NotFound)?;
    let mut next = project_summary(
        &previous,
        domain::scoring::RunTransition::QuestionAttemptRecorded { at: submitted_at },
        grade_policy(&assignment),
    )?;
    let run_items = state
        .run_items
        .get(&(tenant, run.id))
        .cloned()
        .ok_or_else(|| StoreError::Unavailable("run has no immutable items".to_string()))?;
    let attempts = state
        .attempts
        .values()
        .filter(|attempt| attempt.tenant == tenant && attempt.run == run.id)
        .map(|attempt| {
            if attempt.id == submitted.id {
                submitted.clone()
            } else {
                projected_attempt(state, tenant, attempt)
            }
        })
        .collect::<Vec<_>>();
    let questions = current_run_questions(&assignment, &run_items, &attempts, &submitted)?;
    let results = questions
        .iter()
        .map(|question| question.map(|question| question.result))
        .collect::<Vec<_>>();
    let submitted_item = run_items
        .iter()
        .find(|item| item.issued_position == submitted.assignment_position)
        .ok_or_else(|| {
            StoreError::Unavailable("submitted attempt has no immutable run item".to_string())
        })?;
    let submitted_assignment_item = submitted_item.assignment_item;
    let (earned_points, possible_points) = crate::current_attempt_points(
        &assignment,
        submitted_assignment_item,
        submitted.status,
        command.result,
    )?;
    let (scoring_generation, _) = state
        .assignment_scoring
        .get(&(tenant, assignment.id))
        .copied()
        .ok_or(StoreError::NotFound)?;
    let mut statistics_contributions = None;
    if let Some(score) = completed_run_score(&questions, assignment.policies.completion)? {
        next = project_summary(
            &next,
            domain::scoring::RunTransition::Completed {
                score,
                at: submitted_at,
            },
            grade_policy(&assignment),
        )?;
        run.completed_at = Some(submitted_at);
        run.score = Some(score);
        project_enrollment_completion(
            &mut enrollment,
            &previous,
            grade_policy(&assignment),
            run.id,
            score,
            submitted_at,
        );
        if run.mode == RunMode::Assigned && previous.completed_run_count == 0 {
            statistics_contributions = Some(derive_statistics_contributions(
                &run_items, &results, &attempts,
            )?);
        }
    }
    if let Some(contributions) = &statistics_contributions {
        stage_statistics_contributions(
            state,
            tenant,
            enrollment.id,
            run.id,
            submitted.id,
            contributions,
        )?;
    }
    let record = SubmissionRecord {
        attempt: submitted,
        run: run.clone(),
        summary: next.clone(),
        feedback,
        presentation,
        disclosure,
    };
    state.submissions.insert(
        (tenant, command.attempt),
        StoredSubmission {
            key: command.idempotency_key,
            response: command.response,
            record: record.clone(),
        },
    );
    state.attempt_scores.insert(
        (tenant, command.attempt),
        MemoryAttemptScore {
            assignment: assignment.id,
            assignment_item: submitted_assignment_item,
            generation: scoring_generation,
            earned_points,
            possible_points,
        },
    );
    state.runs.insert((tenant, run.id), run);
    state
        .enrollments
        .insert((tenant, enrollment.id), enrollment);
    state.summaries.insert((tenant, next.enrollment), next);
    state
        .webwork_grade_replay
        .remove(&(tenant, command.attempt));
    complete_memory_attempt_timing_job(state, tenant, command.attempt);
    Ok(record)
}

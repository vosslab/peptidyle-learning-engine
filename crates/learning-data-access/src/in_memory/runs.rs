use async_trait::async_trait;

use super::*;
use crate::{ReceiptNextAttempt, ReceiptPresentationSnapshot};

mod attempt_issuance;
mod issued_contracts;
mod learner_reads;

pub(super) use issued_contracts::{
    load_issued_flat_grading, load_issued_presentation, load_issued_webwork_grading,
    load_submission_record,
};

#[async_trait]
impl crate::RunStore for MemoryStore {
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
        assignment_id: AssignmentId,
        proposed_run: RunId,
    ) -> Result<AssignmentRun, StoreError> {
        let mut state = self.write_state()?;
        let tenant = context.tenant_id();
        let assignment = assignment_record(&state, tenant, assignment_id)?;
        require_course_records_accessible(&state, tenant, assignment.course_id)?;
        // S5 decides current access first, but a grant is not yet a receipt:
        // S3 must reject an unavailable, closed, or exhausted policy without
        // leaving enrollment evidence behind.
        let domain::entitlement::EntitlementDecision::Granted(grant) =
            super::entitlement::evaluate_locked(
                &state,
                tenant,
                actor,
                assignment.course_id,
                assignment_id,
            )?
        else {
            return Err(StoreError::NotFound);
        };
        let now = state.authoritative_time;
        let existing_run_count = state
            .runs
            .values()
            .filter(|run| {
                run.tenant == tenant
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
                lifecycle: domain::effective_assignment_policy::AssignmentLifecycleGate::Open,
                entitlement: domain::entitlement::EntitlementDecision::Granted(grant.clone()),
                authorization: domain::effective_assignment_policy::AuthorizationGate::Authorized,
                now,
                prior_run_count: u32::try_from(existing_run_count).map_err(|_| {
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
                assignment.course_id,
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
        let run_items = select_assignment_run_items(&assignment, run.id)?;
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
    async fn get_attempt_presentation_binding_impl(
        &self,
        context: TenantContext,
        actor: UserId,
        attempt: QuestionAttemptId,
    ) -> Result<Option<PresentationBindingV1>, StoreError> {
        let state = self.read_state()?;
        let tenant = context.tenant_id();
        let record = state
            .attempts
            .get(&(tenant, attempt))
            .ok_or(StoreError::NotFound)?;
        require_attempt_owner(&state, tenant, record, actor)?;
        Ok(state.attempt_presentations.get(&(tenant, attempt)).copied())
    }

    async fn get_attempt_presentation_snapshot_impl(
        &self,
        context: TenantContext,
        actor: UserId,
        attempt: QuestionAttemptId,
    ) -> Result<Option<ReceiptPresentationSnapshot>, StoreError> {
        let state = self.read_state()?;
        let tenant = context.tenant_id();
        let record = state
            .attempts
            .get(&(tenant, attempt))
            .ok_or(StoreError::NotFound)?;
        require_attempt_owner(&state, tenant, record, actor)?;
        load_issued_presentation(&state, tenant, record)
    }

    async fn get_attempt_grading_envelope_impl(
        &self,
        context: TenantContext,
        actor: UserId,
        attempt: QuestionAttemptId,
    ) -> Result<Option<QuestionEnvelope>, StoreError> {
        let state = self.read_state()?;
        let tenant = context.tenant_id();
        let record = state
            .attempts
            .get(&(tenant, attempt))
            .ok_or(StoreError::NotFound)?;
        require_attempt_owner(&state, tenant, record, actor)?;
        load_issued_presentation(&state, tenant, record)?;
        Ok(state
            .attempt_grading_envelopes
            .get(&(tenant, attempt))
            .cloned())
    }

    async fn get_attempt_flat_grading_impl(
        &self,
        context: TenantContext,
        actor: UserId,
        attempt: QuestionAttemptId,
    ) -> Result<Option<crate::IssuedFlatGradingContract>, StoreError> {
        let state = self.read_state()?;
        let tenant = context.tenant_id();
        let record = state
            .attempts
            .get(&(tenant, attempt))
            .ok_or(StoreError::NotFound)?;
        require_attempt_owner(&state, tenant, record, actor)?;
        load_issued_presentation(&state, tenant, record)?;
        load_issued_flat_grading(&state, tenant, record)
    }

    async fn get_attempt_webwork_grading_impl(
        &self,
        context: TenantContext,
        actor: UserId,
        attempt: QuestionAttemptId,
    ) -> Result<Option<crate::IssuedWebworkGradingContract>, StoreError> {
        let state = self.read_state()?;
        let tenant = context.tenant_id();
        let record = state
            .attempts
            .get(&(tenant, attempt))
            .ok_or(StoreError::NotFound)?;
        require_attempt_owner(&state, tenant, record, actor)?;
        load_issued_presentation(&state, tenant, record)?;
        load_issued_webwork_grading(&state, tenant, record)
    }

    async fn get_webwork_grade_replay_state_impl(
        &self,
        context: TenantContext,
        actor: UserId,
        attempt: QuestionAttemptId,
    ) -> Result<Option<WebworkGradeReplayStateV1>, StoreError> {
        let state = self.read_state()?;
        let tenant = context.tenant_id();
        let record = state
            .attempts
            .get(&(tenant, attempt))
            .ok_or(StoreError::NotFound)?;
        require_attempt_owner(&state, tenant, record, actor)?;
        let replay = state.webwork_grade_replay.get(&(tenant, attempt)).cloned();
        if let Some(replay) = replay.as_ref() {
            crate::validate_persisted_webwork_replay_state(
                record,
                state.attempt_presentations.get(&(tenant, attempt)).copied(),
                replay,
            )?;
        }
        Ok(replay)
    }
    async fn reserve_or_resume_prefetched_question_impl(
        &self,
        context: TenantContext,
        command: ReservePrefetchedQuestionCommand,
    ) -> Result<PrefetchedQuestion, StoreError> {
        let mut state = self.write_state()?;
        let tenant = context.tenant_id();
        let reservation = command.reservation;
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
        let assignment = assignment_record(&state, tenant, enrollment.assignment)?;
        require_course_records_accessible(&state, tenant, assignment.course_id)?;
        super::entitlement::require_current_enrollment_entitlement(
            &state,
            tenant,
            command.actor,
            assignment.course_id,
            assignment.id,
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
            return if existing == &reservation {
                Ok(existing.clone())
            } else {
                Err(StoreError::Conflict)
            };
        }
        state.prefetched_questions.insert(key, reservation.clone());
        Ok(reservation)
    }
    async fn get_prefetched_question_impl(
        &self,
        context: TenantContext,
        actor: UserId,
        run: RunId,
        predecessor: QuestionAttemptId,
        assignment_position: u32,
    ) -> Result<Option<PrefetchedQuestion>, StoreError> {
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
    ) -> Result<Option<PrefetchedQuestion>, StoreError> {
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
        let pending: Vec<_> = state
            .attempts
            .values()
            .filter(|attempt| {
                attempt.tenant == context.tenant_id()
                    && attempt.run == run
                    && state
                        .submissions
                        .contains_key(&(context.tenant_id(), attempt.id))
                    && !state
                        .submission_next_attempts
                        .contains_key(&(context.tenant_id(), attempt.id))
            })
            .map(|attempt| attempt.id)
            .take(2)
            .collect();
        match pending.as_slice() {
            [] => Ok(None),
            [id] => Ok(Some(*id)),
            _ => Err(StoreError::Conflict),
        }
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

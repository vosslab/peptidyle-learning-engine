use async_trait::async_trait;
use question_model::{
    CourseMembershipRole, IssuedAttemptCapabilityV1, SubmissionEvaluationStatus, UserRole,
};

use super::*;
use crate::{
    AcceptedSubmission, AcceptedSubmissionCommand, AcceptedSubmissionId, AutomatedGradingStore,
    GradingExecution, GradingExecutionGeneration, GradingExecutionReceipt, GradingOperation,
    GradingOperationGroup, GradingOperationGroupBy, GradingOperationRevision,
    GradingOperationTrustGeneration, InstructorGradingOperationProjection,
    InstructorGradingOperationRow, StoreError, TenantContext,
};

pub(super) fn require_instructor_operation_authority(
    state: &State,
    tenant: TenantId,
    session: crate::SessionTokenHash,
    course: CourseId,
    assignment: AssignmentId,
) -> Result<UserId, StoreError> {
    let subject = super::sessions::active_subject(
        state,
        TenantContext::from_authenticated_session(tenant),
        session,
    )
    .ok_or(StoreError::NotFound)?;
    let actor = subject.user();
    // ASVS 8.2.1-8.2.2: keep explicit role and course membership checks in the Store boundary.
    if !subject.roles().contains(&UserRole::Instructor) {
        return Err(StoreError::NotFound);
    }
    let assignment_record = assignment_record(state, tenant, assignment)?;
    if assignment_record.course_id != course
        || super::entitlement::current_course_role(state, tenant, course, actor)
            != Some(CourseMembershipRole::Instructor)
    {
        return Err(StoreError::NotFound);
    }
    require_course_records_accessible(state, tenant, course)?;
    Ok(actor)
}
pub(super) fn operation_row(
    state: &State,
    tenant: TenantId,
    operation: GradingOperation,
    group_by: GradingOperationGroupBy,
) -> Result<InstructorGradingOperationRow, StoreError> {
    let projection = InstructorGradingOperationProjection {
        reference: operation.reference,
        reason: operation.reason,
        state: operation.state,
        revision: operation.revision,
        next_action: operation.next_action,
    };
    let (_attempt, learner, enrollment_id, question_id, title, generation) = match operation.target
    {
        crate::GradingOperationTarget::SubmissionRecovery { submission } => {
            let attempt = state
                .automated_grading_executions
                .iter()
                .find_map(|((stored_tenant, attempt), execution)| {
                    (*stored_tenant == tenant && execution.submission == submission)
                        .then_some(*attempt)
                })
                .ok_or(StoreError::NotFound)?;
            let record = state
                .attempts
                .get(&(tenant, attempt))
                .ok_or(StoreError::NotFound)?;
            let run = state
                .runs
                .get(&(tenant, record.run))
                .ok_or(StoreError::NotFound)?;
            let enrollment = enrollment_record(state, tenant, run.enrollment)?;
            let published = state
                .published
                .get(&(record.problem, record.question_version))
                .ok_or(StoreError::NotFound)?;
            let issued = state
                .attempt_issued_question_snapshots
                .get(&(tenant, attempt))
                .ok_or(StoreError::NotFound)?;
            if issued.question().problem != record.problem
                || issued.question().version != record.question_version
            {
                return Err(StoreError::InvalidRecord(
                    "issued question snapshot does not match its attempt".to_string(),
                ));
            }
            let execution = state
                .automated_grading_executions
                .get(&(tenant, attempt))
                .ok_or(StoreError::NotFound)?;
            (
                attempt,
                enrollment.user,
                enrollment.id,
                published.question_id.clone(),
                issued.question().metadata.title.clone(),
                GradingOperationTrustGeneration::Execution(execution.generation),
            )
        }
        crate::GradingOperationTarget::AssignmentScoringGeneration {
            requested_generation,
        } => {
            return Ok(InstructorGradingOperationRow {
                operation: projection,
                group: GradingOperationGroup::Assignment,
                affected_learner_count: assignment_group_impact(
                    state,
                    tenant,
                    operation.assignment,
                )?,
                trust_generation: GradingOperationTrustGeneration::AssignmentScoring(
                    requested_generation,
                ),
                stable_cursor: crate::GradingOperationCursor::encode(
                    tenant,
                    operation.course,
                    operation.assignment,
                    group_by,
                    &GradingOperationGroup::Assignment,
                    operation.reference,
                ),
            });
        }
    };
    let group = match group_by {
        GradingOperationGroupBy::Question => GradingOperationGroup::Question { question_id, title },
        GradingOperationGroupBy::Learner => GradingOperationGroup::Learner {
            membership: state
                .entitlement_materializations
                .get(&(tenant, enrollment_id))
                .map(|materialization| materialization.membership)
                .and_then(|membership| {
                    state
                        .course_membership_references
                        .get(&(tenant, membership))
                })
                .copied()
                .ok_or(StoreError::NotFound)?,
            display_name: question_model::TeachingDisplayLabel::try_from(
                state
                    .accounts
                    .get(&learner)
                    .ok_or(StoreError::NotFound)?
                    .display_name
                    .clone(),
            )
            .map_err(|error| StoreError::InvalidRecord(error.to_string()))?,
        },
    };
    let stable_cursor = crate::GradingOperationCursor::encode(
        tenant,
        operation.course,
        operation.assignment,
        group_by,
        &group,
        operation.reference,
    );
    Ok(InstructorGradingOperationRow {
        operation: projection,
        affected_learner_count: operation_group_impact(
            state,
            tenant,
            operation.course,
            operation.assignment,
            &group,
        )?,
        group,
        trust_generation: generation,
        stable_cursor,
    })
}
fn assignment_group_impact(
    state: &State,
    tenant: TenantId,
    assignment: AssignmentId,
) -> Result<u32, StoreError> {
    u32::try_from(
        state
            .enrollments
            .values()
            .filter(|enrollment| enrollment.tenant == tenant && enrollment.assignment == assignment)
            .count(),
    )
    .map_err(|_| StoreError::InvalidRecord("operation impact exceeds u32".to_string()))
}
fn operation_group_impact(
    state: &State,
    tenant: TenantId,
    course: CourseId,
    assignment: AssignmentId,
    group: &GradingOperationGroup,
) -> Result<u32, StoreError> {
    let mut members = std::collections::BTreeSet::new();
    for operation in state
        .automated_grading_operations
        .values()
        .filter(|operation| {
            operation.tenant == tenant
                && operation.course == course
                && operation.assignment == assignment
        })
    {
        let crate::GradingOperationTarget::SubmissionRecovery { submission } = operation.target
        else {
            continue;
        };
        let Some(attempt) = state.automated_grading_executions.iter().find_map(
            |((stored_tenant, attempt), execution)| {
                (*stored_tenant == tenant && execution.submission == submission).then_some(*attempt)
            },
        ) else {
            continue;
        };
        let record = state
            .attempts
            .get(&(tenant, attempt))
            .ok_or(StoreError::NotFound)?;
        let run = state
            .runs
            .get(&(tenant, record.run))
            .ok_or(StoreError::NotFound)?;
        let enrollment = enrollment_record(state, tenant, run.enrollment)?;
        let membership = state
            .entitlement_materializations
            .get(&(tenant, enrollment.id))
            .map(|materialization| materialization.membership)
            .and_then(|membership| {
                state
                    .course_membership_references
                    .get(&(tenant, membership))
            })
            .copied()
            .ok_or(StoreError::NotFound)?;
        let matches = match group {
            GradingOperationGroup::Question { question_id, .. } => state
                .published
                .get(&(record.problem, record.question_version))
                .is_some_and(|published| &published.question_id == question_id),
            GradingOperationGroup::Learner {
                membership: selected,
                ..
            } => membership == *selected,
            GradingOperationGroup::Assignment => false,
        };
        if matches {
            members.insert(membership);
        }
    }
    u32::try_from(members.len())
        .map_err(|_| StoreError::InvalidRecord("operation impact exceeds u32".to_string()))
}
pub(super) fn next_operation_revision(
    revision: GradingOperationRevision,
) -> Result<GradingOperationRevision, StoreError> {
    GradingOperationRevision::from_u64(
        revision
            .as_u64()
            .checked_add(1)
            .ok_or(StoreError::Conflict)?,
    )
    .ok_or(StoreError::Conflict)
}

pub(super) fn page_rows(
    rows: Vec<InstructorGradingOperationRow>,
    tenant: TenantId,
    course: CourseId,
    assignment: AssignmentId,
    group_by: GradingOperationGroupBy,
    page: crate::PageRequest,
) -> Result<crate::Page<InstructorGradingOperationRow>, StoreError> {
    let seek = page
        .after
        .as_ref()
        .map(|cursor| {
            crate::GradingOperationCursor::decode(cursor, tenant, course, assignment, group_by)
        })
        .transpose()?;
    let start = seek.map_or(0, |seek| {
        rows.iter()
            .position(|row| {
                (
                    crate::operation_group_key(&row.group),
                    row.operation.reference,
                ) > (seek.group_key.clone(), seek.operation)
            })
            .unwrap_or(rows.len())
    });
    let end = start
        .saturating_add(usize::from(page.size.get()))
        .min(rows.len());
    let next_cursor = (end < rows.len()).then(|| rows[end - 1].stable_cursor.clone());
    Ok(crate::Page {
        items: rows[start..end].to_vec(),
        next_cursor,
    })
}

#[async_trait]
impl AutomatedGradingStore for MemoryStore {
    async fn accept_automated_submission(
        &self,
        context: TenantContext,
        command: AcceptedSubmissionCommand,
    ) -> Result<AcceptedSubmission, StoreError> {
        let mut state = self.write_state()?;
        let tenant = context.tenant_id();
        let canonical_response = crate::canonical_student_response_json(&command.response)?;
        let response_sha256 = objects::Sha256Digest::compute(canonical_response.as_bytes());
        let attempt = state
            .attempts
            .get(&(tenant, command.attempt))
            .cloned()
            .ok_or(StoreError::NotFound)?;
        let run = state
            .runs
            .get(&(tenant, attempt.run))
            .ok_or(StoreError::NotFound)?;
        let enrollment = enrollment_record(&state, tenant, run.enrollment)?;
        let assignment = assignment_record(&state, tenant, enrollment.assignment)?;
        if assignment.course_id != command.course
            || assignment.id != command.assignment
            || enrollment.user != command.actor
            || super::entitlement::current_course_role(
                &state,
                tenant,
                command.course,
                command.actor,
            ) != Some(CourseMembershipRole::Student)
        {
            return Err(StoreError::NotFound);
        }
        require_course_records_accessible(&state, tenant, command.course)?;
        if let Some(stored) = state.submissions.get(&(tenant, command.attempt)) {
            let Some(accepted) = stored.accepted_pending() else {
                return Err(StoreError::Conflict);
            };
            let metadata_matches = accepted.actor == command.actor
                && accepted.course == command.course
                && accepted.assignment == command.assignment
                && accepted.idempotency_key == command.idempotency_key
                && accepted.request_sha256 == response_sha256;
            if metadata_matches
                && stored_submission_matches_canonical(
                    &state,
                    tenant,
                    command.attempt,
                    &command.response,
                    &canonical_response,
                    response_sha256,
                )?
            {
                return Ok(accepted.clone());
            }
            return Err(StoreError::Conflict);
        }
        if projected_attempt(&state, tenant, &attempt).status
            != question_model::AttemptStatus::InProgress
        {
            return Err(StoreError::Conflict);
        }
        let accepted_at = state.authoritative_time;
        // The accepted-work record is the common durable timing witness for
        // every worker backend. Validate it before storing private response
        // material so a later evaluator cannot complete work that expired at
        // the server-owned acceptance boundary.
        let timing = state
            .attempt_timing
            .get(&(tenant, command.attempt))
            .ok_or_else(|| {
                StoreError::Unavailable("issued timing authority is missing".to_string())
            })?;
        let effective_policy = timing
            .effective_deadline
            .map_or(TimingPolicy::Untimed, |_| TimingPolicy::PerQuestion {
                seconds: 1,
                grace_seconds: timing.effective_grace_seconds,
            });
        let verdict = timer_verdict(&TimerEvaluation {
            policy: effective_policy,
            timer: projected_attempt(&state, tenant, &attempt).timer,
            evaluated_at: accepted_at,
            pause_extension_millis: 0,
        })
        .map_err(|error| StoreError::InvalidRecord(error.to_string()))?;
        if verdict == TimerVerdict::TimedOut {
            return Err(StoreError::TimedOut);
        }
        let submission = AcceptedSubmissionId::from_uuid(command.attempt.as_uuid());
        let accepted = AcceptedSubmission {
            tenant,
            course: command.course,
            assignment: command.assignment,
            attempt: command.attempt,
            submission,
            actor: command.actor,
            idempotency_key: command.idempotency_key.clone(),
            request_sha256: response_sha256,
            accepted_at,
        };
        let execution = GradingExecution {
            submission,
            generation: GradingExecutionGeneration::INITIAL,
            state: crate::GradingExecutionState::Ready,
            job: command.execution_job,
            retry_count: 0,
        };
        if state.jobs.contains_key(&command.execution_job) {
            return Err(StoreError::Conflict);
        }
        let private_response = StoredPrivateSubmissionResponse {
            canonical_text: canonical_response,
            sha256: response_sha256,
            response: command.response,
        };
        state.submissions.insert(
            (tenant, command.attempt),
            StoredSubmission {
                key: command.idempotency_key,
                state: StoredSubmissionState::AcceptedPending(accepted.clone()),
            },
        );
        state
            .private_submission_responses
            .insert((tenant, command.attempt), private_response);
        // Keep Memory's current lifecycle projection aligned with the atomic
        // PostgreSQL broker transition. The accepted response stays only in
        // `private_submission_responses`; this answer-free projection records
        // that the learner can no longer submit a second response. ASVS
        // 2.3.1/2.3.3.
        let mut current_attempt = projected_attempt(&state, tenant, &attempt);
        current_attempt.status = question_model::AttemptStatus::Submitted;
        current_attempt.timer.submitted_at = Some(accepted_at);
        state
            .attempt_current
            .insert((tenant, command.attempt), current_attempt);
        complete_memory_attempt_timing_job(&mut state, tenant, command.attempt);
        state
            .automated_grading_executions
            .insert((tenant, command.attempt), execution);
        state.automated_grading_evaluations.insert(
            (tenant, command.attempt),
            SubmissionEvaluationStatus::AutomatedPending,
        );
        state.jobs.insert(
            command.execution_job,
            StoredJob {
                tenant,
                payload: crate::JobPayload::GradeAcceptedSubmission {
                    attempt: command.attempt,
                    submission,
                    execution_generation: GradingExecutionGeneration::INITIAL,
                },
                state: crate::JobState::Ready,
                available_at: accepted_at,
                lease_token: None,
                lease_expires_at: None,
                attempt_count: 0,
                max_attempts: crate::ACCEPTED_SUBMISSION_JOB_MAX_ATTEMPTS,
                failure: None,
            },
        );
        state
            .automated_grading_execution_receipts
            .entry((tenant, command.attempt))
            .or_default()
            .push(GradingExecutionReceipt {
                submission,
                generation: GradingExecutionGeneration::INITIAL,
                resulting_state: crate::GradingExecutionState::Ready,
                worker: None,
                occurred_at: accepted_at,
            });
        Ok(accepted)
    }

    async fn automated_grading_execution(
        &self,
        context: TenantContext,
        submission: AcceptedSubmissionId,
    ) -> Result<Option<GradingExecution>, StoreError> {
        let state = self.read_state()?;
        let tenant = context.tenant_id();
        Ok(state
            .automated_grading_executions
            .iter()
            .find_map(|((stored_tenant, _), execution)| {
                (*stored_tenant == tenant && execution.submission == submission)
                    .then_some(*execution)
            }))
    }

    async fn record_automated_grading_execution_receipt(
        &self,
        context: TenantContext,
        receipt: GradingExecutionReceipt,
        resulting_evaluation: SubmissionEvaluationStatus,
    ) -> Result<(), StoreError> {
        let mut state = self.write_state()?;
        let tenant = context.tenant_id();
        let (attempt, generation_matches) = state
            .automated_grading_executions
            .iter_mut()
            .find_map(|((stored_tenant, attempt), execution)| {
                (*stored_tenant == tenant && execution.submission == receipt.submission)
                    .then_some((*attempt, execution.generation == receipt.generation))
            })
            .ok_or(StoreError::NotFound)?;
        if !generation_matches {
            return Err(StoreError::Conflict);
        }
        let execution = state
            .automated_grading_executions
            .get_mut(&(tenant, attempt))
            .ok_or(StoreError::NotFound)?;
        execution.state = receipt.resulting_state;
        state
            .automated_grading_evaluations
            .insert((tenant, attempt), resulting_evaluation);
        state
            .automated_grading_execution_receipts
            .entry((tenant, attempt))
            .or_default()
            .push(receipt);
        Ok(())
    }
}

pub(super) fn load_prepared_accepted_submission(
    state: &State,
    tenant: question_model::TenantId,
    attempt_id: question_model::QuestionAttemptId,
) -> Result<crate::PreparedQuestionSubmission, StoreError> {
    let attempt = state
        .attempts
        .get(&(tenant, attempt_id))
        .cloned()
        .ok_or(StoreError::NotFound)?;
    let issued_question_snapshot = state
        .attempt_issued_question_snapshots
        .get(&(tenant, attempt_id))
        .cloned()
        .ok_or_else(|| {
            StoreError::Unavailable("issued question snapshot is missing".to_string())
        })?;
    let flat_capability = state
        .attempt_flat_grading_capabilities
        .get(&(tenant, attempt_id))
        .copied()
        .ok_or_else(|| {
            StoreError::Unavailable("attempt flat grading capability is missing".to_string())
        })?;
    let webwork_capability = state
        .attempt_webwork_grading_capabilities
        .get(&(tenant, attempt_id))
        .copied()
        .ok_or_else(|| {
            StoreError::Unavailable("attempt WeBWorK grading capability is missing".to_string())
        })?;
    let qti_capability = state
        .attempt_qti_grading_capabilities
        .get(&(tenant, attempt_id))
        .copied()
        .ok_or_else(|| {
            StoreError::Unavailable("attempt QTI grading capability is missing".to_string())
        })?;
    super::runs::validate_issued_question_snapshot(
        &issued_question_snapshot,
        &attempt,
        flat_capability,
        webwork_capability,
        qti_capability,
        state
            .attempt_presentation_snapshots
            .get(&(tenant, attempt_id)),
    )?;
    let presentation = super::runs::load_issued_presentation(state, tenant, &attempt)?;
    let presentation_binding = state
        .attempt_presentations
        .get(&(tenant, attempt_id))
        .copied();
    let grading_envelope = state
        .attempt_grading_envelopes
        .get(&(tenant, attempt_id))
        .cloned();
    let flat_grading = super::runs::load_issued_flat_grading(state, tenant, &attempt)?;
    let webwork_grading = super::runs::load_issued_webwork_grading(state, tenant, &attempt)?;
    let issued_qti_grading =
        super::runs::load_issued_qti_grading(state, tenant, &attempt, &issued_question_snapshot)?;
    crate::validate_issued_flat_grading(
        issued_question_snapshot.question(),
        if presentation.is_some() {
            crate::PresentationCapability::EnvelopeV1
        } else {
            crate::PresentationCapability::NotApplicable
        },
        if matches!(
            attempt.issued_capability,
            IssuedAttemptCapabilityV1::FlatPresentation
        ) {
            crate::FlatGradingCapability::Required
        } else {
            crate::FlatGradingCapability::NotApplicable
        },
        flat_grading.as_ref(),
    )?;
    crate::validate_issued_webwork_grading(
        issued_question_snapshot.question(),
        if matches!(
            attempt.issued_capability,
            IssuedAttemptCapabilityV1::WebworkPresentation
        ) {
            crate::WebworkGradingCapability::Required
        } else {
            crate::WebworkGradingCapability::NotApplicable
        },
        webwork_grading.as_ref(),
    )?;
    crate::validate_issued_qti_grading(
        issued_question_snapshot.question(),
        if matches!(
            attempt.issued_capability,
            IssuedAttemptCapabilityV1::QtiPresentation
        ) {
            crate::QtiGradingCapability::Required
        } else {
            crate::QtiGradingCapability::NotApplicable
        },
        issued_qti_grading.as_ref(),
    )?;
    Ok(crate::PreparedQuestionSubmission {
        attempt,
        issued_question_snapshot,
        presentation_binding,
        presentation,
        grading_envelope,
        flat_grading,
        webwork_grading,
        issued_qti_grading,
        webwork_replay: state
            .webwork_grade_replay
            .get(&(tenant, attempt_id))
            .cloned(),
    })
}

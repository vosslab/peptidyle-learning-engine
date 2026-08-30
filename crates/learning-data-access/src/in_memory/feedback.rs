use async_trait::async_trait;

use super::*;
use crate::StudentDisclosureInput;

/// Builds the sole S4 disclosure input from the assignment-owned schedule,
/// the current sealed S3 receipt, and the Memory authoritative clock.  The
/// caller has already performed Student or Instructor authorization.
pub(super) fn current_disclosure_input(
    state: &State,
    assignment: &AssignmentRecord,
    attempt: QuestionAttemptId,
    submitted_at: Option<ActivityTimestamp>,
) -> Result<StudentDisclosureInput, StoreError> {
    if state
        .attempt_timing
        .get(&attempt)
        .map(|timing| timing.assignment)
        != Some(assignment.id)
    {
        return Err(StoreError::Unavailable(
            "current effective-policy receipt does not bind the prepared assignment".to_string(),
        ));
    }
    let generation = state
        .attempt_effective_policy_current
        .get(&attempt)
        .ok_or_else(|| {
            StoreError::Unavailable("current effective-policy receipt is missing".to_string())
        })?;
    let receipt = state
        .issued_effective_policy_receipts
        .get(&(attempt, *generation))
        .ok_or_else(|| {
            StoreError::Unavailable("current effective-policy receipt is corrupt".to_string())
        })?;
    if receipt.attempt != attempt {
        return Err(StoreError::Unavailable(
            "current effective-policy receipt does not bind the attempt".to_string(),
        ));
    }
    Ok(StudentDisclosureInput::new(
        assignment.disclosure_policy,
        receipt.policy.clone(),
        state.authoritative_time,
        submitted_at,
    ))
}

#[async_trait]
impl crate::FeedbackStore for MemoryStore {
    async fn release_attempt_feedback_impl(
        &self,
        context: ActorContext,
        command: ReleaseAttemptFeedbackCommand,
    ) -> Result<FeedbackReleaseRecord, StoreError> {
        let mut state = self.write_state()?;
        if context.user_id() != command.actor {
            return Err(StoreError::NotFound);
        }
        let attempt = state
            .attempts
            .get(&command.attempt)
            .cloned()
            .ok_or(StoreError::NotFound)?;
        let run = state.runs.get(&attempt.run).ok_or(StoreError::NotFound)?;
        let enrollment = enrollment_record(&state, run.enrollment)?;
        let assignment = assignment_record(&state, enrollment.assignment)?;
        require_course_records_accessible(&state, assignment.course_id)?;
        state
            .courses
            .get(&assignment.course_id)
            .ok_or(StoreError::NotFound)?;
        if super::entitlement::current_course_role(&state, assignment.course_id, command.actor)
            != Some(CourseMembershipRole::Instructor)
        {
            return Err(StoreError::NotFound);
        }
        if !state.submissions.contains_key(&command.attempt) {
            return Err(StoreError::NotFound);
        }
        let current = projected_attempt(&state, &attempt);
        let disclosure = current_disclosure_input(
            &state,
            &assignment,
            command.attempt,
            current.timer.submitted_at,
        )?
        .decision();
        if !disclosure.feedback_text && !disclosure.solution {
            return Err(StoreError::InvalidRecord(
                "feedback release requires currently disclosed teaching feedback".to_string(),
            ));
        }
        if let Some(existing) = state.feedback_releases.get(&command.attempt) {
            return if existing.released_by == command.actor {
                Ok(existing.clone())
            } else {
                Err(StoreError::Conflict)
            };
        }
        let record = FeedbackReleaseRecord {
            attempt: command.attempt,
            released_by: command.actor,
            released_at: state.authoritative_time,
        };
        state
            .feedback_releases
            .insert(command.attempt, record.clone());
        Ok(record)
    }
    async fn get_attempt_feedback_release_impl(
        &self,
        context: ActorContext,
        actor: UserId,
        attempt_id: QuestionAttemptId,
    ) -> Result<Option<FeedbackReleaseRecord>, StoreError> {
        let state = self.read_state()?;
        if context.user_id() != actor {
            return Err(StoreError::NotFound);
        }
        let attempt = state
            .attempts
            .get(&attempt_id)
            .ok_or(StoreError::NotFound)?;
        let run = state.runs.get(&attempt.run).ok_or(StoreError::NotFound)?;
        let enrollment = enrollment_record(&state, run.enrollment)?;
        let assignment = assignment_record(&state, enrollment.assignment)?;
        require_course_records_accessible(&state, assignment.course_id)?;
        state
            .courses
            .get(&assignment.course_id)
            .ok_or(StoreError::NotFound)?;
        let student_self = super::entitlement::require_current_enrollment_entitlement(
            &state,
            actor,
            assignment.course_id,
            assignment.id,
            &enrollment,
        )
        .is_ok();
        if !student_self
            && super::entitlement::current_course_role(&state, assignment.course_id, actor)
                != Some(CourseMembershipRole::Instructor)
        {
            return Err(StoreError::NotFound);
        }
        Ok(state.feedback_releases.get(&attempt_id).cloned())
    }
    async fn get_run_summary_page_impl(
        &self,
        context: ActorContext,
        actor: UserId,
        run_id: RunId,
        page: PageRequest,
    ) -> Result<RunSummaryPageInput, StoreError> {
        let state = self.read_state()?;
        if context.user_id() != actor {
            return Err(StoreError::NotFound);
        }
        let run = state
            .runs
            .get(&run_id)
            .cloned()
            .ok_or(StoreError::NotFound)?;
        let enrollment = enrollment_record(&state, run.enrollment)?;
        let assignment = assignment_record(&state, enrollment.assignment)?;
        require_course_records_accessible(&state, assignment.course_id)?;
        state
            .courses
            .get(&assignment.course_id)
            .ok_or(StoreError::NotFound)?;
        let student_self = super::entitlement::require_current_enrollment_entitlement(
            &state,
            actor,
            assignment.course_id,
            assignment.id,
            &enrollment,
        )
        .is_ok();
        if !student_self
            && super::entitlement::current_course_role(&state, assignment.course_id, actor)
                != Some(CourseMembershipRole::Instructor)
        {
            return Err(StoreError::NotFound);
        }
        let summary = state
            .summaries
            .get(&enrollment.id)
            .cloned()
            .ok_or(StoreError::NotFound)?;
        let after = page
            .after
            .as_ref()
            .map(|cursor| RunSummaryCursor::decode(cursor, run.id.as_uuid()))
            .transpose()?;
        let mut rows = Vec::new();
        for attempt in state
            .attempts
            .values()
            .filter(|attempt| attempt.run == run.id)
        {
            let current = projected_attempt(&state, attempt);
            if student_self && current.status == AttemptStatus::Cleared {
                continue;
            }
            if student_self {
                let assignment_item = state
                    .run_items
                    .get(&run.id)
                    .and_then(|items| {
                        items
                            .iter()
                            .find(|item| item.issued_position == attempt.assignment_position)
                    })
                    .map(|item| item.assignment_item)
                    .ok_or_else(|| {
                        StoreError::Unavailable(
                            "summary attempt has no immutable run item".to_string(),
                        )
                    })?;
                if assignment_item_is_retired(&assignment, assignment_item).ok_or_else(|| {
                    StoreError::Unavailable(
                        "summary run item has no current assignment tombstone".to_string(),
                    )
                })? {
                    continue;
                }
            }
            let key = RunSummaryCursor {
                assignment_position: attempt.assignment_position,
                attempt: attempt.id.as_uuid(),
            };
            if after.is_some_and(|cursor| key <= cursor) {
                continue;
            }
            let submitted = state
                .submissions
                .get(&attempt.id)
                .and_then(|stored| stored.completed_record_opt());
            rows.push((
                key,
                RunSummaryOutcomeInput {
                    attempt: current.id,
                    assignment_position: current.assignment_position,
                    submitted_at: current.timer.submitted_at,
                    response: current.response.clone(),
                    result: current.result,
                    disclosure: current_disclosure_input(
                        &state,
                        &assignment,
                        current.id,
                        current.timer.submitted_at,
                    )?,
                    feedback: submitted.map(|record| record.feedback.clone()),
                },
            ));
        }
        rows.sort_by_key(|(key, _)| *key);
        let take = usize::from(page.size.get());
        let has_more = rows.len() > take;
        rows.truncate(take);
        let next_cursor = has_more
            .then(|| rows.last().map(|(key, _)| key.encode(run.id.as_uuid())))
            .flatten();
        let scoring_status = state
            .assignment_scoring
            .get(&assignment.id)
            .ok_or(StoreError::NotFound)?
            .1;
        Ok(RunSummaryPageInput {
            run,
            practice_allowed: continued_practice_allows_run(
                &summary,
                assignment.policies.continued_practice,
            ),
            assignment,
            summary,
            scoring_status,
            outcomes: Page {
                items: rows.into_iter().map(|(_, item)| item).collect(),
                next_cursor,
            },
        })
    }
}

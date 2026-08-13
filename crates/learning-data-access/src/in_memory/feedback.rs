use async_trait::async_trait;

use super::*;

#[async_trait]
impl crate::FeedbackStore for MemoryStore {
    async fn release_attempt_feedback_impl(
        &self,
        context: TenantContext,
        command: ReleaseAttemptFeedbackCommand,
    ) -> Result<FeedbackReleaseRecord, StoreError> {
        let mut state = self.write_state()?;
        let tenant = context.tenant_id();
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
        require_course_records_accessible(&state, tenant, assignment.course_id)?;
        let course = state
            .courses
            .get(&(tenant, assignment.course_id))
            .ok_or(StoreError::NotFound)?;
        if course.role_for(command.actor) != Some(CourseRole::Instructor) {
            return Err(StoreError::NotFound);
        }
        if !state.submissions.contains_key(&(tenant, command.attempt)) {
            return Err(StoreError::NotFound);
        }
        let disclosure = *state
            .attempt_feedback_disclosures
            .get(&(tenant, command.attempt))
            .ok_or_else(|| {
                StoreError::Unavailable("issued feedback disclosure is missing".to_string())
            })?;
        if disclosure != question_model::run_policy::FeedbackDisclosure::OnRelease {
            return Err(StoreError::InvalidRecord(
                "feedback release requires an on-release question policy".to_string(),
            ));
        }
        if let Some(existing) = state.feedback_releases.get(&(tenant, command.attempt)) {
            return if existing.released_by == command.actor {
                Ok(existing.clone())
            } else {
                Err(StoreError::Conflict)
            };
        }
        let record = FeedbackReleaseRecord {
            tenant,
            attempt: command.attempt,
            released_by: command.actor,
            released_at: state.authoritative_time,
        };
        state
            .feedback_releases
            .insert((tenant, command.attempt), record.clone());
        Ok(record)
    }
    async fn get_attempt_feedback_release_impl(
        &self,
        context: TenantContext,
        actor: UserId,
        attempt_id: QuestionAttemptId,
    ) -> Result<Option<FeedbackReleaseRecord>, StoreError> {
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
        let course = state
            .courses
            .get(&(tenant, assignment.course_id))
            .ok_or(StoreError::NotFound)?;
        if actor != enrollment.user && course.role_for(actor) != Some(CourseRole::Instructor) {
            return Err(StoreError::NotFound);
        }
        Ok(state.feedback_releases.get(&(tenant, attempt_id)).cloned())
    }
    async fn get_run_summary_page_impl(
        &self,
        context: TenantContext,
        actor: UserId,
        run_id: RunId,
        page: PageRequest,
    ) -> Result<RunSummaryPageInput, StoreError> {
        let state = self.read_state()?;
        let tenant = context.tenant_id();
        let run = state
            .runs
            .get(&(tenant, run_id))
            .cloned()
            .ok_or(StoreError::NotFound)?;
        let enrollment = enrollment_record(&state, tenant, run.enrollment)?;
        let assignment = assignment_record(&state, tenant, enrollment.assignment)?;
        require_course_records_accessible(&state, tenant, assignment.course_id)?;
        let course = state
            .courses
            .get(&(tenant, assignment.course_id))
            .ok_or(StoreError::NotFound)?;
        if actor != enrollment.user && course.role_for(actor) != Some(CourseRole::Instructor) {
            return Err(StoreError::NotFound);
        }
        let summary = state
            .summaries
            .get(&(tenant, enrollment.id))
            .cloned()
            .ok_or(StoreError::NotFound)?;
        let after = page
            .after
            .as_ref()
            .map(|cursor| RunSummaryCursor::decode(cursor, tenant.as_uuid(), run.id.as_uuid()))
            .transpose()?;
        let mut rows = Vec::new();
        for attempt in state
            .attempts
            .values()
            .filter(|attempt| attempt.tenant == tenant && attempt.run == run.id)
        {
            let current = projected_attempt(&state, tenant, attempt);
            if actor == enrollment.user && current.status == AttemptStatus::Cleared {
                continue;
            }
            if actor == enrollment.user {
                let assignment_item = state
                    .run_items
                    .get(&(tenant, run.id))
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
                .get(&(tenant, attempt.id))
                .map(|stored| &stored.record);
            let feedback_policy = *state
                .attempt_feedback_disclosures
                .get(&(tenant, attempt.id))
                .ok_or_else(|| {
                    StoreError::Unavailable("issued feedback disclosure is missing".to_string())
                })?;
            rows.push((
                key,
                RunSummaryOutcomeInput {
                    attempt: current.id,
                    assignment_position: current.assignment_position,
                    submitted_at: current.timer.submitted_at,
                    response: current.response.clone(),
                    result: current.result,
                    feedback_policy,
                    feedback: submitted.map(|record| record.feedback.clone()),
                    release: state.feedback_releases.get(&(tenant, current.id)).cloned(),
                },
            ));
        }
        rows.sort_by_key(|(key, _)| *key);
        let take = usize::from(page.size.get());
        let has_more = rows.len() > take;
        rows.truncate(take);
        let next_cursor = has_more
            .then(|| {
                rows.last()
                    .map(|(key, _)| key.encode(tenant.as_uuid(), run.id.as_uuid()))
            })
            .flatten();
        Ok(RunSummaryPageInput {
            run,
            practice_allowed: continued_practice_allows_run(
                &summary,
                assignment.policies.continued_practice,
            ),
            assignment,
            summary,
            outcomes: Page {
                items: rows.into_iter().map(|(_, item)| item).collect(),
                next_cursor,
            },
        })
    }
}

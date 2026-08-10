use async_trait::async_trait;

use super::*;

#[async_trait]
impl crate::StatisticsStore for MemoryStore {
    async fn question_statistics_impl(
        &self,
        context: TenantContext,
        reference: ProblemVersionRef,
    ) -> Result<QuestionStatisticsDisclosure, StoreError> {
        let state = self.read_state()?;
        let visible = state
            .published
            .get(&(reference.problem, reference.version))
            .is_some_and(|record| catalog_record_visible(&state, context.tenant_id(), record));
        if !visible {
            return Ok(QuestionStatisticsDisclosure::Suppressed);
        }
        let disclosure = state
            .question_statistics
            .get(&(reference.problem, reference.version))
            .map(|aggregate| aggregate.disclose(StatisticsDisclosurePolicy::default()))
            .unwrap_or(QuestionStatisticsDisclosure::Suppressed);
        Ok(disclosure)
    }
    async fn list_gradebook_rows_impl(
        &self,
        context: TenantContext,
        course: CourseId,
        page: PageRequest,
    ) -> Result<Page<question_model::GradebookSummaryRow>, StoreError> {
        let state = self.read_state()?;
        let tenant = context.tenant_id();
        require_course_records_accessible(&state, tenant, course)?;
        let mut records = state
            .enrollments
            .iter()
            .filter_map(|((row_tenant, enrollment_id), enrollment)| {
                if *row_tenant != tenant {
                    return None;
                }
                let assignment = state.assignments.get(&(tenant, enrollment.assignment))?;
                if assignment.course_id != course {
                    return None;
                }
                let summary = state.summaries.get(&(tenant, *enrollment_id))?.clone();
                Some((
                    GradebookCursor {
                        assignment: assignment.id.as_uuid(),
                        enrollment: enrollment.id.as_uuid(),
                    },
                    question_model::GradebookSummaryRow {
                        tenant,
                        course_id: course,
                        enrollment_id: enrollment.id,
                        student_id: enrollment.student,
                        assignment_id: assignment.id,
                        assignment_title: assignment.title.clone(),
                        summary,
                    },
                ))
            })
            .collect::<Vec<_>>();
        let cursor = page
            .after
            .as_ref()
            .map(GradebookCursor::decode)
            .transpose()?;
        records.sort_by_key(|(key, _)| *key);
        let mut selected = records
            .into_iter()
            .filter(|(key, _)| cursor.is_none_or(|after| *key > after))
            .take(usize::from(page.size.get()) + 1)
            .collect::<Vec<_>>();
        let has_more = selected.len() > usize::from(page.size.get());
        if has_more {
            selected.pop();
        }
        let next_cursor = has_more.then(|| {
            selected
                .last()
                .map(|(key, _)| key.encode())
                .expect("a nonempty page precedes a following page")
        });
        Ok(Page {
            items: selected.into_iter().map(|(_, row)| row).collect(),
            next_cursor,
        })
    }
}

/// Stages all first-completed-run contributions before mutating visible
/// submission state. One rejected aggregate leaves the whole MemoryStore
/// transition unchanged.
pub(super) fn stage_statistics_contributions(
    state: &mut State,
    tenant: TenantId,
    enrollment: EnrollmentId,
    first_completed_run: RunId,
    trigger_attempt: QuestionAttemptId,
    contributions: &[StatisticsContribution],
) -> Result<(), StoreError> {
    let mut aggregate_updates = BTreeMap::new();
    let mut receipt_updates = BTreeMap::new();
    for contribution in contributions {
        let receipt_key = (
            tenant,
            enrollment,
            contribution.reference.problem,
            contribution.reference.version,
        );
        if let Some(receipt) = state.question_statistics_receipts.get(&receipt_key) {
            if receipt.first_completed_run == first_completed_run
                && receipt.attempt == trigger_attempt
                && receipt.checksum == contribution.checksum
            {
                continue;
            }
            return Err(StoreError::Conflict);
        }
        if receipt_updates.contains_key(&receipt_key) {
            return Err(StoreError::Conflict);
        }
        let aggregate_key = (
            contribution.reference.problem,
            contribution.reference.version,
        );
        let aggregate = aggregate_updates.entry(aggregate_key).or_insert_with(|| {
            state
                .question_statistics
                .get(&aggregate_key)
                .cloned()
                .unwrap_or_default()
        });
        aggregate
            .record(contribution.observation)
            .map_err(|error| StoreError::InvalidRecord(error.to_string()))?;
        receipt_updates.insert(
            receipt_key,
            StatisticsContributionReceipt {
                first_completed_run,
                attempt: trigger_attempt,
                #[cfg(test)]
                observation: contribution.observation,
                checksum: contribution.checksum,
            },
        );
    }
    state.question_statistics.extend(aggregate_updates);
    state.question_statistics_receipts.extend(receipt_updates);
    Ok(())
}

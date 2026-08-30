use async_trait::async_trait;

use super::*;

#[async_trait]
impl crate::StatisticsStore for MemoryStore {
    async fn question_statistics_impl(
        &self,
        reference: ProblemVersionRef,
    ) -> Result<QuestionStatisticsDisclosure, StoreError> {
        let state = self.read_state()?;
        let visible = state
            .published
            .get(&(reference.problem, reference.version))
            .is_some_and(|record| record.scope == question_model::PublicationScope::Public);
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
        actor: ActorContext,
        course: CourseId,
        page: PageRequest,
    ) -> Result<Page<question_model::GradebookSummaryRow>, StoreError> {
        let state = self.read_state()?;
        require_course_records_accessible(&state, course)?;
        match super::entitlement::current_course_role(&state, course, actor.user_id()) {
            Some(question_model::CourseMembershipRole::Instructor) => {}
            Some(question_model::CourseMembershipRole::Student) => {
                return Err(StoreError::Forbidden);
            }
            None => return Err(StoreError::NotFound),
        }
        let mut records = state
            .enrollments
            .iter()
            .filter_map(|(enrollment_id, enrollment)| {
                let assignment = state.assignments.get(&enrollment.assignment)?;
                if assignment.course_id != course {
                    return None;
                }
                let summary = state.summaries.get(enrollment_id)?.clone();
                let student_name = state
                    .course_memberships
                    .values()
                    .find(|membership| {
                        membership.course == course
                            && membership.student == Some(enrollment.student)
                    })
                    .and_then(|membership| state.roster_profiles.get(&(course, membership.id)))
                    .map(|profile| profile.display_name.clone())
                    .unwrap_or_else(|| "Learner".to_string());
                Some((
                    GradebookCursor {
                        assignment: assignment.id.as_uuid(),
                        enrollment: enrollment.id.as_uuid(),
                    },
                    question_model::GradebookSummaryRow {
                        course_id: course,
                        enrollment_id: enrollment.id,
                        student_id: enrollment.student,
                        student_name,
                        assignment_id: assignment.id,
                        assignment_title: assignment.title.clone(),
                        summary,
                        scoring_status: state.assignment_scoring.get(&assignment.id)?.1,
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
    enrollment: EnrollmentId,
    first_completed_run: RunId,
    contributions: &[StatisticsContribution],
) -> Result<(), StoreError> {
    let enrollment_record = state
        .enrollments
        .get(&enrollment)
        .ok_or(StoreError::NotFound)?;
    let course = state
        .assignments
        .get(&enrollment_record.assignment)
        .map(|assignment| assignment.course_id)
        .ok_or(StoreError::NotFound)?;
    let student_fingerprint = discovery_student_fingerprint(enrollment_record.student);
    let mut aggregate_updates = BTreeMap::new();
    let mut receipt_updates = BTreeMap::new();
    let mut observed_course_updates = BTreeSet::new();
    let mut student_updates = BTreeSet::new();
    for contribution in contributions {
        let receipt_key = (
            enrollment,
            contribution.reference.problem,
            contribution.reference.version,
        );
        if let Some(receipt) = state.question_statistics_receipts.get(&receipt_key) {
            if receipt.first_completed_run == first_completed_run
                && receipt.attempt == contribution.first_scored_attempt
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
        let student_key = (aggregate_key.0, aggregate_key.1, student_fingerprint);
        let independent = !state.catalog_evidence_learners.contains(&student_key)
            && !student_updates.contains(&student_key);
        if independent {
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
            observed_course_updates.insert(aggregate_key);
        }
        receipt_updates.insert(
            receipt_key,
            StatisticsContributionReceipt {
                first_completed_run,
                attempt: contribution.first_scored_attempt,
                #[cfg(test)]
                observation: contribution.observation,
                checksum: contribution.checksum,
            },
        );
        student_updates.insert(student_key);
    }
    // `append_catalog_discovery_evidence_revision` is the only later fallible
    // operation. Reserve enough sequence space before any map becomes
    // visible, so the following application phase is infallible.
    state
        .next_catalog_publication_sequence
        .checked_add(
            u64::try_from(observed_course_updates.len()).map_err(|_| StoreError::Conflict)?,
        )
        .ok_or_else(|| StoreError::Unavailable("catalog event sequence exhausted".to_string()))?;
    state.question_statistics.extend(aggregate_updates);
    state.question_statistics_receipts.extend(receipt_updates);
    state.catalog_evidence_learners.extend(student_updates);
    for reference in observed_course_updates {
        state
            .catalog_evidence_courses
            .entry(reference)
            .or_default()
            .insert(course);
        append_catalog_discovery_evidence_revision(state, reference)?;
    }
    Ok(())
}

pub(super) fn discovery_student_fingerprint(student: StudentId) -> [u8; 32] {
    let mut bytes = Vec::with_capacity(64);
    bytes.extend_from_slice(b"ple-discovery-student-v1");
    bytes.extend_from_slice(student.as_uuid().as_bytes());
    *objects::Sha256Digest::compute(&bytes).as_bytes()
}

/// Appends the current evidence projection only after both independent
/// privacy thresholds are met.  The revision remains immutable after the
/// event boundary is issued to a catalog continuation.
pub(super) fn append_catalog_discovery_evidence_revision(
    state: &mut State,
    reference: (ProblemId, VersionId),
) -> Result<(), StoreError> {
    let Some(question_model::QuestionStatisticsDisclosure::Available(view)) = state
        .question_statistics
        .get(&reference)
        .map(|aggregate| aggregate.disclose(StatisticsDisclosurePolicy::default()))
    else {
        return Ok(());
    };
    let observed_course_count = state
        .catalog_evidence_courses
        .get(&reference)
        .map_or(0_u64, |courses| courses.len() as u64);
    if observed_course_count < 2 {
        return Ok(());
    }
    let sequence = state.next_catalog_publication_sequence;
    state.next_catalog_publication_sequence = sequence
        .checked_add(1)
        .ok_or_else(|| StoreError::Unavailable("catalog event sequence exhausted".to_string()))?;
    let discrimination = view.discrimination_index.unwrap_or(0.0).max(0.0);
    let quality = ((1.0_f64 + observed_course_count as f64).ln()
        + (1.0_f64 + view.cohort_size as f64).ln()
        + discrimination)
        .mul_add(1_000_000.0, 0.0)
        .round() as i64;
    state
        .catalog_discovery_evidence_revisions
        .entry(reference)
        .or_default()
        .push(CatalogDiscoveryEvidenceRevision {
            sequence,
            quality,
            evidence: CatalogDiscoveryEvidence::Available {
                formula_version: 1,
                observed_course_count,
                independent_learner_observation_count: view.cohort_size,
                difficulty_index: view.difficulty_index,
                attempts_mean: view.attempts_mean,
                time_median_seconds_estimate: view.time_median_seconds_estimate,
                discrimination_index: view.discrimination_index,
                evidence_at: state.authoritative_time,
            },
        });
    Ok(())
}

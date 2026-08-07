//! In-memory Store backend (WP-C4, MOD-STO).

use std::collections::BTreeMap;
use std::sync::{Arc, RwLock};

use async_trait::async_trait;
use domain::run::continued_practice_allows_run;
use domain::scoring::project_summary;
use question_model::{
    AssignmentEnrollment, AssignmentId, AssignmentRun, EnrollmentId, EnrollmentStatus, GradePolicy,
    ProblemId, QuestionAttempt, QuestionAttemptId, RunId, RunMode, StudentAssignmentSummary,
    TenantId, VersionId, WorkspaceId,
};

use crate::{
    ActivityTransition, AssignmentRecord, Cursor, DraftRecord, Page, PageRequest,
    PublishedProblemRecord, Store, StoreError, TenantContext, grade_policy, summary_transition,
};

/// Memory backend used by conformance tests and pre-PostgreSQL lanes.
#[derive(Debug, Clone, Default)]
pub struct MemoryStore {
    state: Arc<RwLock<State>>,
}

/// All maps use tenant ID in their key for tenant-owned records.
#[derive(Debug, Default)]
struct State {
    drafts: BTreeMap<(TenantId, WorkspaceId), DraftRecord>,
    published: BTreeMap<(ProblemId, VersionId), PublishedProblemRecord>,
    assignments: BTreeMap<(TenantId, AssignmentId), AssignmentRecord>,
    enrollments: BTreeMap<(TenantId, EnrollmentId), AssignmentEnrollment>,
    runs: BTreeMap<(TenantId, RunId), AssignmentRun>,
    attempts: BTreeMap<(TenantId, QuestionAttemptId), QuestionAttempt>,
    summaries: BTreeMap<(TenantId, EnrollmentId), StudentAssignmentSummary>,
}

#[async_trait]
impl Store for MemoryStore {
    async fn upsert_draft(
        &self,
        context: TenantContext,
        draft: DraftRecord,
    ) -> Result<(), StoreError> {
        ensure_tenant(context, draft.tenant)?;
        if !draft.question.is_draft() {
            return Err(StoreError::InvalidRecord(
                "draft must not carry a ProblemId".to_string(),
            ));
        }
        let mut state = self.write_state()?;
        state
            .drafts
            .insert((draft.tenant, draft.question.workspace), draft);
        Ok(())
    }

    async fn get_draft(
        &self,
        context: TenantContext,
        workspace: WorkspaceId,
    ) -> Result<Option<DraftRecord>, StoreError> {
        let state = self.read_state()?;
        Ok(state.drafts.get(&(context.tenant_id(), workspace)).cloned())
    }

    async fn publish_problem(&self, record: PublishedProblemRecord) -> Result<(), StoreError> {
        if record.question.problem != Some(record.problem)
            || record.question.version != record.version
        {
            return Err(StoreError::InvalidRecord(
                "published record IDs must match its question definition".to_string(),
            ));
        }
        let mut state = self.write_state()?;
        let key = (record.problem, record.version);
        if state.published.contains_key(&key) {
            return Err(StoreError::AlreadyExists);
        }
        state.published.insert(key, record);
        Ok(())
    }

    async fn get_published_problem(
        &self,
        problem: ProblemId,
        version: VersionId,
    ) -> Result<Option<PublishedProblemRecord>, StoreError> {
        let state = self.read_state()?;
        Ok(state.published.get(&(problem, version)).cloned())
    }

    async fn list_published_problems(
        &self,
        page: PageRequest,
    ) -> Result<Page<PublishedProblemRecord>, StoreError> {
        let state = self.read_state()?;
        let records = state
            .published
            .iter()
            .map(|((problem, version), record)| (format!("{problem}/{version}"), record.clone()))
            .collect();
        Ok(page_records(records, &page))
    }

    async fn upsert_assignment(
        &self,
        context: TenantContext,
        assignment: AssignmentRecord,
    ) -> Result<(), StoreError> {
        ensure_tenant(context, assignment.tenant)?;
        let mut state = self.write_state()?;
        for reference in &assignment.problems {
            if !state
                .published
                .contains_key(&(reference.problem, reference.version))
            {
                return Err(StoreError::InvalidRecord(format!(
                    "assignment references missing published version {}/{}",
                    reference.problem, reference.version
                )));
            }
        }
        state
            .assignments
            .insert((assignment.tenant, assignment.id), assignment);
        Ok(())
    }

    async fn get_assignment(
        &self,
        context: TenantContext,
        assignment: AssignmentId,
    ) -> Result<Option<AssignmentRecord>, StoreError> {
        let state = self.read_state()?;
        Ok(state
            .assignments
            .get(&(context.tenant_id(), assignment))
            .cloned())
    }

    async fn list_assignments(
        &self,
        context: TenantContext,
        page: PageRequest,
    ) -> Result<Page<AssignmentRecord>, StoreError> {
        let state = self.read_state()?;
        let records = state
            .assignments
            .iter()
            .filter(|((tenant, _), _)| *tenant == context.tenant_id())
            .map(|((_, assignment), record)| (assignment.to_string(), record.clone()))
            .collect();
        Ok(page_records(records, &page))
    }

    async fn create_enrollment(
        &self,
        context: TenantContext,
        enrollment: AssignmentEnrollment,
    ) -> Result<(), StoreError> {
        ensure_tenant(context, enrollment.tenant)?;
        let mut state = self.write_state()?;
        if !state
            .assignments
            .contains_key(&(enrollment.tenant, enrollment.assignment))
        {
            return Err(StoreError::InvalidRecord(
                "enrollment references a missing assignment".to_string(),
            ));
        }
        let key = (enrollment.tenant, enrollment.id);
        if state.enrollments.contains_key(&key) {
            return Err(StoreError::AlreadyExists);
        }
        state.summaries.insert(
            key,
            StudentAssignmentSummary::empty(enrollment.tenant, enrollment.id),
        );
        state.enrollments.insert(key, enrollment);
        Ok(())
    }

    async fn get_enrollment(
        &self,
        context: TenantContext,
        enrollment: EnrollmentId,
    ) -> Result<Option<AssignmentEnrollment>, StoreError> {
        let state = self.read_state()?;
        Ok(state
            .enrollments
            .get(&(context.tenant_id(), enrollment))
            .cloned())
    }

    async fn apply_activity_transition(
        &self,
        context: TenantContext,
        transition: ActivityTransition,
    ) -> Result<StudentAssignmentSummary, StoreError> {
        let mut state = self.write_state()?;
        let tenant = context.tenant_id();

        let (enrollment_id, assignment, domain_transition) = match &transition {
            ActivityTransition::StartRun { run } => {
                ensure_tenant(context, run.tenant)?;
                if run.run_number == 0 || run.completed_at.is_some() || run.score.is_some() {
                    return Err(StoreError::InvalidRecord(
                        "new run must be one-based and incomplete".to_string(),
                    ));
                }
                if state.runs.contains_key(&(tenant, run.id)) {
                    return Err(StoreError::AlreadyExists);
                }
                let enrollment = enrollment_record(&state, tenant, run.enrollment)?;
                let assignment = assignment_record(&state, tenant, enrollment.assignment)?;
                let expected_mode = match enrollment.status() {
                    EnrollmentStatus::InProgress => RunMode::Assigned,
                    EnrollmentStatus::Completed => RunMode::Practice,
                };
                if run.mode != expected_mode {
                    return Err(StoreError::InvalidRecord(format!(
                        "run mode must be {expected_mode:?} for this enrollment"
                    )));
                }
                if run.variation != assignment.policies.variation {
                    return Err(StoreError::InvalidRecord(
                        "run variation must match its assignment policy".to_string(),
                    ));
                }
                if state.runs.values().any(|existing| {
                    existing.tenant == tenant
                        && existing.enrollment == run.enrollment
                        && existing.completed_at.is_none()
                }) {
                    return Err(StoreError::InvalidRecord(
                        "an enrollment cannot have two in-progress runs".to_string(),
                    ));
                }
                let expected_run_number = state
                    .runs
                    .values()
                    .filter(|existing| {
                        existing.tenant == tenant && existing.enrollment == run.enrollment
                    })
                    .map(|existing| existing.run_number)
                    .max()
                    .unwrap_or(0)
                    .checked_add(1)
                    .ok_or_else(|| StoreError::InvalidRecord("run number overflow".to_string()))?;
                if run.run_number != expected_run_number {
                    return Err(StoreError::InvalidRecord(format!(
                        "run number must be the next one-based value {expected_run_number}"
                    )));
                }
                (enrollment.id, assignment, summary_transition(&transition))
            }
            ActivityTransition::RecordQuestionAttempt { attempt } => {
                ensure_tenant(context, attempt.tenant)?;
                if state.attempts.contains_key(&(tenant, attempt.id)) {
                    return Err(StoreError::AlreadyExists);
                }
                let run = state
                    .runs
                    .get(&(tenant, attempt.run))
                    .ok_or(StoreError::NotFound)?;
                if run.completed_at.is_some() || run.score.is_some() {
                    return Err(StoreError::InvalidRecord(
                        "question attempts cannot be added to a completed run".to_string(),
                    ));
                }
                let enrollment = enrollment_record(&state, tenant, run.enrollment)?;
                let assignment = assignment_record(&state, tenant, enrollment.assignment)?;
                if !assignment.problems.iter().any(|reference| {
                    reference.problem == attempt.problem
                        && reference.version == attempt.question_version
                }) {
                    return Err(StoreError::InvalidRecord(
                        "question attempt must reference a version in its assignment".to_string(),
                    ));
                }
                (enrollment.id, assignment, summary_transition(&transition))
            }
            ActivityTransition::CompleteRun { run, .. } => {
                let run_record = state
                    .runs
                    .get(&(tenant, *run))
                    .ok_or(StoreError::NotFound)?;
                if run_record.completed_at.is_some() || run_record.score.is_some() {
                    return Err(StoreError::InvalidRecord(
                        "completed run cannot be completed again".to_string(),
                    ));
                }
                let enrollment = enrollment_record(&state, tenant, run_record.enrollment)?;
                (
                    enrollment.id,
                    assignment_record(&state, tenant, enrollment.assignment)?,
                    summary_transition(&transition),
                )
            }
        };

        let summary_key = (tenant, enrollment_id);
        let previous = state
            .summaries
            .get(&summary_key)
            .cloned()
            .ok_or(StoreError::NotFound)?;
        let grade = grade_policy(&assignment);
        if matches!(&transition, ActivityTransition::StartRun { .. })
            && !continued_practice_allows_run(&previous, assignment.policies.continued_practice)
        {
            return Err(StoreError::InvalidRecord(
                "continued-practice policy does not permit another run".to_string(),
            ));
        }
        let next = project_summary(&previous, domain_transition, grade)?;

        match transition {
            ActivityTransition::StartRun { run } => {
                state.runs.insert((tenant, run.id), run);
            }
            ActivityTransition::RecordQuestionAttempt { attempt } => {
                state.attempts.insert((tenant, attempt.id), *attempt);
            }
            ActivityTransition::CompleteRun { run, score, at } => {
                {
                    let run_record = state
                        .runs
                        .get_mut(&(tenant, run))
                        .ok_or(StoreError::NotFound)?;
                    run_record.completed_at = Some(at);
                    run_record.score = Some(score);
                }
                let enrollment = state
                    .enrollments
                    .get_mut(&summary_key)
                    .ok_or(StoreError::NotFound)?;
                project_enrollment_completion(enrollment, &previous, grade, run, score, at);
            }
        }
        state.summaries.insert(summary_key, next.clone());
        Ok(next)
    }

    async fn get_run(
        &self,
        context: TenantContext,
        run: RunId,
    ) -> Result<Option<AssignmentRun>, StoreError> {
        let state = self.read_state()?;
        Ok(state.runs.get(&(context.tenant_id(), run)).cloned())
    }

    async fn list_runs(
        &self,
        context: TenantContext,
        enrollment: EnrollmentId,
        page: PageRequest,
    ) -> Result<Page<AssignmentRun>, StoreError> {
        let state = self.read_state()?;
        if !state
            .enrollments
            .contains_key(&(context.tenant_id(), enrollment))
        {
            return Err(StoreError::NotFound);
        }
        let records = state
            .runs
            .iter()
            .filter(|((tenant, _), run)| {
                *tenant == context.tenant_id() && run.enrollment == enrollment
            })
            .map(|((_, run_id), run)| (format!("{:010}/{run_id}", run.run_number), run.clone()))
            .collect();
        Ok(page_records(records, &page))
    }

    async fn get_question_attempt(
        &self,
        context: TenantContext,
        attempt: QuestionAttemptId,
    ) -> Result<Option<QuestionAttempt>, StoreError> {
        let state = self.read_state()?;
        Ok(state.attempts.get(&(context.tenant_id(), attempt)).cloned())
    }

    async fn get_summary(
        &self,
        context: TenantContext,
        enrollment: EnrollmentId,
    ) -> Result<Option<StudentAssignmentSummary>, StoreError> {
        let state = self.read_state()?;
        Ok(state
            .summaries
            .get(&(context.tenant_id(), enrollment))
            .cloned())
    }
}

impl MemoryStore {
    /// Acquires immutable backend state.
    fn read_state(&self) -> Result<std::sync::RwLockReadGuard<'_, State>, StoreError> {
        self.state
            .read()
            .map_err(|error| StoreError::Unavailable(error.to_string()))
    }

    /// Acquires mutable backend state for one atomic operation.
    fn write_state(&self) -> Result<std::sync::RwLockWriteGuard<'_, State>, StoreError> {
        self.state
            .write()
            .map_err(|error| StoreError::Unavailable(error.to_string()))
    }
}

/// Refuses records whose direct tenant does not match authenticated context.
fn ensure_tenant(context: TenantContext, record_tenant: TenantId) -> Result<(), StoreError> {
    if context.tenant_id() == record_tenant {
        Ok(())
    } else {
        Err(StoreError::TenantMismatch)
    }
}

/// Loads one enrollment inside an already tenant-scoped state operation.
fn enrollment_record(
    state: &State,
    tenant: TenantId,
    enrollment: EnrollmentId,
) -> Result<AssignmentEnrollment, StoreError> {
    state
        .enrollments
        .get(&(tenant, enrollment))
        .cloned()
        .ok_or(StoreError::NotFound)
}

/// Maintains the enrollment pointers in the same activity transaction.
fn project_enrollment_completion(
    enrollment: &mut AssignmentEnrollment,
    previous: &StudentAssignmentSummary,
    grade: GradePolicy,
    run: RunId,
    score: f64,
    at: question_model::ActivityTimestamp,
) {
    let is_first_completion = previous.completed_run_count == 0;
    let is_new_best = previous.best_score.is_none_or(|best| score > best);

    if enrollment.first_completed_at.is_none() {
        enrollment.first_completed_at = Some(at);
    }
    if is_new_best || enrollment.best_grade_run.is_none() {
        enrollment.best_grade_run = Some(run);
    }
    enrollment.current_grade_run = match grade {
        GradePolicy::First if is_first_completion => Some(run),
        GradePolicy::First => enrollment.current_grade_run,
        GradePolicy::Latest => Some(run),
        GradePolicy::Highest if is_new_best => Some(run),
        GradePolicy::Highest => enrollment.current_grade_run,
        GradePolicy::InstructorSelected => enrollment.current_grade_run,
    };
}

/// Loads the assignment whose grade policy drives summary projection.
fn assignment_record(
    state: &State,
    tenant: TenantId,
    assignment: AssignmentId,
) -> Result<AssignmentRecord, StoreError> {
    state
        .assignments
        .get(&(tenant, assignment))
        .cloned()
        .ok_or(StoreError::NotFound)
}

/// Applies stable-key cursor paging without a positional index parameter.
fn page_records<T>(mut records: Vec<(String, T)>, request: &PageRequest) -> Page<T> {
    records.sort_by(|left, right| left.0.cmp(&right.0));
    let after = request.after.as_ref().map(Cursor::as_str);
    let mut selected: Vec<(String, T)> = records
        .into_iter()
        .filter(|(key, _)| after.is_none_or(|cursor| key.as_str() > cursor))
        .take(usize::from(request.size.get()) + 1)
        .collect();
    let has_more = selected.len() > usize::from(request.size.get());
    if has_more {
        selected.pop();
    }
    let next_cursor = if has_more {
        selected
            .last()
            .map(|(key, _)| Cursor::from_stable_key(key.clone()))
    } else {
        None
    };
    Page {
        items: selected.into_iter().map(|(_, item)| item).collect(),
        next_cursor,
    }
}

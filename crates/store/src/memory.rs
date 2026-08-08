//! In-memory Store backend (WP-C4, MOD-STO).

use std::collections::{BTreeMap, BTreeSet};
use std::sync::{Arc, RwLock};

use async_trait::async_trait;
use domain::run::continued_practice_allows_run;
use domain::scoring::project_summary;
use domain::timing::{TimerEvaluation, TimerVerdict, timer_verdict};
use question_model::run_policy::TimingPolicy;
use question_model::taxonomy::TaxonomyTerm;
use question_model::{
    ActivityTimestamp, AssignmentEnrollment, AssignmentId, AssignmentRun, AttemptResult,
    AttemptTimerRecord, CatalogLifecycle, CatalogProblemSummary, CourseId, CourseRole,
    CourseSummary, EnrollmentId, EnrollmentStatus, ProblemId, ProblemVersionRef, PublicationScope,
    QuestionAttempt, QuestionAttemptId, RunId, RunMode, StudentAssignmentSummary, StudentResponse,
    TenantId, UserId, VersionId, WorkspaceId,
};

use crate::{
    ActivityTransition, AssetAccessEvent, AssetDeliveryId, AssetDeliveryRecord, AssetDeliveryScope,
    AssetStore, AssignmentRecord, AuthorizedAssetDelivery, CatalogStore, CatalogTransition,
    CourseListScope, CourseRecord, Cursor, DraftRecord, IssueQuestionAttemptCommand, Page,
    PageRequest, PublishDraftCommand, PublishedProblemRecord, SessionLifetime, SessionRecord,
    SessionStore, SessionSubject, SessionTokenHash, Store, StoreError, SubmissionIdempotencyKey,
    SubmissionRecord, SubmitQuestionAttemptCommand, TenantContext, completed_run_score,
    ensure_tenant, grade_policy, project_enrollment_completion, summary_transition,
    validate_asset_delivery, validate_assignment, validate_course, validate_draft,
    validate_published,
};

/// Memory backend used by conformance tests and pre-PostgreSQL lanes.
#[derive(Debug, Clone, Default)]
pub struct MemoryStore {
    state: Arc<RwLock<State>>,
}

/// All maps use tenant ID in their key for tenant-owned records.
#[derive(Debug, Default)]
struct State {
    authoritative_time: ActivityTimestamp,
    sessions: BTreeMap<SessionTokenHash, StoredSession>,
    catalog_grants: BTreeSet<(TenantId, ProblemId, VersionId)>,
    drafts: BTreeMap<(TenantId, WorkspaceId), DraftRecord>,
    published: BTreeMap<(ProblemId, VersionId), PublishedProblemRecord>,
    courses: BTreeMap<(TenantId, CourseId), CourseRecord>,
    assignments: BTreeMap<(TenantId, AssignmentId), AssignmentRecord>,
    enrollments: BTreeMap<(TenantId, EnrollmentId), AssignmentEnrollment>,
    runs: BTreeMap<(TenantId, RunId), AssignmentRun>,
    attempts: BTreeMap<(TenantId, QuestionAttemptId), QuestionAttempt>,
    submissions: BTreeMap<(TenantId, QuestionAttemptId), StoredSubmission>,
    summaries: BTreeMap<(TenantId, EnrollmentId), StudentAssignmentSummary>,
    asset_deliveries: BTreeMap<AssetDeliveryId, AssetDeliveryRecord>,
    asset_access_events: Vec<AssetAccessEvent>,
}

/// Immutable first result retained for exact submission replay.
#[derive(Debug)]
struct StoredSubmission {
    key: SubmissionIdempotencyKey,
    response: StudentResponse,
    record: SubmissionRecord,
}

#[async_trait]
impl AssetStore for MemoryStore {
    async fn register_asset_delivery(
        &self,
        context: TenantContext,
        record: AssetDeliveryRecord,
    ) -> Result<(), StoreError> {
        validate_asset_delivery(&record)?;
        let mut state = self.write_state()?;
        match &record.scope {
            AssetDeliveryScope::Catalog { reference, .. } => {
                let published = state
                    .published
                    .get(&(reference.problem, reference.version))
                    .ok_or(StoreError::NotFound)?;
                if !catalog_record_visible(&state, context.tenant_id(), published) {
                    return Err(StoreError::NotFound);
                }
            }
            AssetDeliveryScope::StudentRecord { tenant, .. } => {
                ensure_tenant(context, *tenant)?;
            }
        }
        if state.asset_deliveries.contains_key(&record.id) {
            return Err(StoreError::AlreadyExists);
        }
        state.asset_deliveries.insert(record.id, record);
        Ok(())
    }

    async fn get_public_asset_delivery(
        &self,
        delivery: AssetDeliveryId,
    ) -> Result<Option<AssetDeliveryRecord>, StoreError> {
        let state = self.read_state()?;
        let Some(record) = state.asset_deliveries.get(&delivery) else {
            return Ok(None);
        };
        let AssetDeliveryScope::Catalog { reference, .. } = record.scope else {
            return Ok(None);
        };
        Ok(state
            .published
            .get(&(reference.problem, reference.version))
            .filter(|published| published.scope == PublicationScope::Public)
            .map(|_| record.clone()))
    }

    async fn authorize_asset_delivery(
        &self,
        context: TenantContext,
        actor: UserId,
        delivery: AssetDeliveryId,
    ) -> Result<AuthorizedAssetDelivery, StoreError> {
        let mut state = self.write_state()?;
        let record = state
            .asset_deliveries
            .get(&delivery)
            .cloned()
            .ok_or(StoreError::NotFound)?;
        let authorized = match &record.scope {
            AssetDeliveryScope::Catalog { reference, .. } => state
                .published
                .get(&(reference.problem, reference.version))
                .is_some_and(|published| {
                    catalog_record_visible(&state, context.tenant_id(), published)
                }),
            AssetDeliveryScope::StudentRecord {
                tenant,
                authorized_users,
            } => *tenant == context.tenant_id() && authorized_users.contains(&actor),
        };
        if !authorized {
            return Err(StoreError::NotFound);
        }
        let authorized_at = state.authoritative_time;
        state.asset_access_events.push(AssetAccessEvent {
            tenant: context.tenant_id(),
            actor,
            delivery,
            object: record.object.id,
            bucket: record.object.bucket,
            occurred_at: authorized_at,
        });
        Ok(AuthorizedAssetDelivery {
            record,
            authorized_at,
        })
    }
}

#[async_trait]
impl CatalogStore for MemoryStore {
    async fn publish_draft(
        &self,
        context: TenantContext,
        command: PublishDraftCommand,
    ) -> Result<PublishedProblemRecord, StoreError> {
        ensure_tenant(context, command.expected_draft.tenant)?;
        validate_draft(&command.expected_draft)?;
        let mut state = self.write_state()?;
        let draft_key = (
            context.tenant_id(),
            command.expected_draft.question.workspace,
        );
        match state.drafts.get(&draft_key) {
            Some(stored) if stored == &command.expected_draft => {}
            Some(_) => return Err(StoreError::Conflict),
            None => return Err(StoreError::NotFound),
        }
        let version = command.expected_draft.question.version;
        if state.published.contains_key(&(command.problem, version)) {
            return Err(StoreError::AlreadyExists);
        }

        let (authors, previous_version, derived_from) =
            if let Some(revises) = command.expected_draft.revises {
                if command.problem != revises.problem {
                    return Err(StoreError::InvalidRecord(
                        "revision must remain in its existing problem chain".to_string(),
                    ));
                }
                let base = state
                    .published
                    .get(&(revises.problem, revises.version))
                    .ok_or(StoreError::NotFound)?;
                if !catalog_record_visible(&state, context.tenant_id(), base) {
                    return Err(StoreError::NotFound);
                }
                if !base.authors.contains(&command.publisher) {
                    return Err(StoreError::Forbidden);
                }
                if state.published.values().any(|record| {
                    record.problem == revises.problem
                        && record.previous_version == Some(revises.version)
                }) {
                    return Err(StoreError::Conflict);
                }
                (
                    base.authors.clone(),
                    Some(revises.version),
                    base.derived_from,
                )
            } else {
                if state
                    .published
                    .keys()
                    .any(|(problem, _)| *problem == command.problem)
                {
                    return Err(StoreError::AlreadyExists);
                }
                if let Some(source) = command.expected_draft.derived_from {
                    let source_record = state
                        .published
                        .get(&(source.problem, source.version))
                        .ok_or(StoreError::NotFound)?;
                    if !catalog_record_visible(&state, context.tenant_id(), source_record) {
                        return Err(StoreError::NotFound);
                    }
                }
                (
                    vec![command.publisher],
                    None,
                    command.expected_draft.derived_from,
                )
            };

        let mut question = command.expected_draft.question.clone();
        question.problem = Some(command.problem);
        let record = PublishedProblemRecord {
            problem: command.problem,
            version,
            question,
            capabilities: command.capabilities,
            scope: command.scope,
            lifecycle: CatalogLifecycle::Published,
            authors,
            previous_version,
            derived_from,
            published_at: state.authoritative_time,
        };
        validate_published(&record)?;
        if record.scope == PublicationScope::Institution {
            state
                .catalog_grants
                .insert((context.tenant_id(), record.problem, record.version));
        }
        state
            .published
            .insert((record.problem, record.version), record.clone());
        state.drafts.remove(&draft_key);
        Ok(record)
    }

    async fn get_catalog_problem(
        &self,
        context: TenantContext,
        reference: ProblemVersionRef,
    ) -> Result<Option<PublishedProblemRecord>, StoreError> {
        let state = self.read_state()?;
        Ok(state
            .published
            .get(&(reference.problem, reference.version))
            .filter(|record| catalog_record_visible(&state, context.tenant_id(), record))
            .cloned())
    }

    async fn list_catalog(
        &self,
        context: TenantContext,
        page: PageRequest,
    ) -> Result<Page<CatalogProblemSummary>, StoreError> {
        let state = self.read_state()?;
        let records = state
            .published
            .iter()
            .filter(|(_, record)| {
                record.lifecycle.is_discoverable()
                    && catalog_record_visible(&state, context.tenant_id(), record)
            })
            .map(|((problem, version), record)| (format!("{problem}/{version}"), record.summary()))
            .collect();
        Ok(page_records(records, &page))
    }

    async fn list_catalog_taxonomy(
        &self,
        context: TenantContext,
        page: PageRequest,
    ) -> Result<Page<TaxonomyTerm>, StoreError> {
        let state = self.read_state()?;
        let mut distinct = BTreeMap::new();
        for record in state.published.values().filter(|record| {
            record.lifecycle.is_discoverable()
                && catalog_record_visible(&state, context.tenant_id(), record)
        }) {
            for term in &record.question.metadata.taxonomy {
                distinct
                    .entry(taxonomy_cursor_key(term))
                    .or_insert_with(|| term.clone());
            }
        }
        Ok(page_records(distinct.into_iter().collect(), &page))
    }

    async fn transition_catalog_problem(
        &self,
        context: TenantContext,
        actor: UserId,
        reference: ProblemVersionRef,
        transition: CatalogTransition,
    ) -> Result<PublishedProblemRecord, StoreError> {
        let mut state = self.write_state()?;
        let key = (reference.problem, reference.version);
        let visible = state
            .published
            .get(&key)
            .is_some_and(|record| catalog_record_visible(&state, context.tenant_id(), record));
        if !visible {
            return Err(StoreError::NotFound);
        }
        let record = state.published.get_mut(&key).ok_or(StoreError::NotFound)?;
        if !record.authors.contains(&actor) {
            return Err(StoreError::Forbidden);
        }
        record.lifecycle = match (&record.lifecycle, transition) {
            (CatalogLifecycle::Published, CatalogTransition::Deprecate { reason }) => {
                let reason = validated_deprecation_reason(reason)?;
                CatalogLifecycle::Deprecated { reason }
            }
            (CatalogLifecycle::Deprecated { reason }, CatalogTransition::Archive) => {
                CatalogLifecycle::Archived {
                    reason: reason.clone(),
                }
            }
            _ => {
                return Err(StoreError::InvalidRecord(
                    "catalog lifecycle transition is not allowed".to_string(),
                ));
            }
        };
        Ok(record.clone())
    }
}

#[derive(Debug)]
struct StoredSession {
    record: SessionRecord,
    revoked: bool,
}

#[async_trait]
impl SessionStore for MemoryStore {
    async fn create_session(
        &self,
        token_hash: SessionTokenHash,
        subject: SessionSubject,
        lifetime: SessionLifetime,
    ) -> Result<SessionRecord, StoreError> {
        let mut state = self.write_state()?;
        if state.sessions.contains_key(&token_hash) {
            return Err(StoreError::AlreadyExists);
        }
        let created_at = state.authoritative_time;
        let lifetime_millis = i64::from(lifetime.as_seconds()) * 1_000;
        let expires_at = ActivityTimestamp::from_unix_millis(
            created_at
                .as_unix_millis()
                .checked_add(lifetime_millis)
                .ok_or_else(|| StoreError::InvalidRecord("session expiry overflow".to_string()))?,
        );
        let record = SessionRecord {
            token_hash,
            subject,
            created_at,
            expires_at,
        };
        state.sessions.insert(
            token_hash,
            StoredSession {
                record: record.clone(),
                revoked: false,
            },
        );
        Ok(record)
    }

    async fn resolve_session(
        &self,
        token_hash: SessionTokenHash,
    ) -> Result<Option<SessionRecord>, StoreError> {
        let state = self.read_state()?;
        let now = state.authoritative_time;
        Ok(state.sessions.get(&token_hash).and_then(|stored| {
            (!stored.revoked && stored.record.expires_at > now).then(|| stored.record.clone())
        }))
    }

    async fn revoke_session(&self, token_hash: SessionTokenHash) -> Result<(), StoreError> {
        let mut state = self.write_state()?;
        if let Some(stored) = state.sessions.get_mut(&token_hash) {
            stored.revoked = true;
        }
        Ok(())
    }
}

#[async_trait]
impl Store for MemoryStore {
    async fn upsert_draft(
        &self,
        context: TenantContext,
        draft: DraftRecord,
    ) -> Result<(), StoreError> {
        ensure_tenant(context, draft.tenant)?;
        validate_draft(&draft)?;
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

    async fn get_published_problem(
        &self,
        problem: ProblemId,
        version: VersionId,
    ) -> Result<Option<PublishedProblemRecord>, StoreError> {
        let state = self.read_state()?;
        Ok(state
            .published
            .get(&(problem, version))
            .filter(|record| record.scope == PublicationScope::Public)
            .cloned())
    }

    async fn list_published_problems(
        &self,
        page: PageRequest,
    ) -> Result<Page<PublishedProblemRecord>, StoreError> {
        let state = self.read_state()?;
        let records = state
            .published
            .iter()
            .filter(|(_, record)| {
                record.scope == PublicationScope::Public && record.lifecycle.is_discoverable()
            })
            .map(|((problem, version), record)| (format!("{problem}/{version}"), record.clone()))
            .collect();
        Ok(page_records(records, &page))
    }

    async fn upsert_course(
        &self,
        context: TenantContext,
        course: CourseRecord,
    ) -> Result<(), StoreError> {
        ensure_tenant(context, course.tenant)?;
        validate_course(&course)?;
        self.write_state()?
            .courses
            .insert((course.tenant, course.id), course);
        Ok(())
    }

    async fn get_course(
        &self,
        context: TenantContext,
        course: CourseId,
    ) -> Result<Option<CourseRecord>, StoreError> {
        let state = self.read_state()?;
        Ok(state.courses.get(&(context.tenant_id(), course)).cloned())
    }

    async fn list_courses(
        &self,
        context: TenantContext,
        scope: CourseListScope,
        page: PageRequest,
    ) -> Result<Page<CourseSummary>, StoreError> {
        let state = self.read_state()?;
        let records = state
            .courses
            .iter()
            .filter_map(|((tenant, course_id), record)| {
                if *tenant != context.tenant_id() {
                    return None;
                }
                let role = match scope {
                    CourseListScope::Member(user) => record.role_for(user)?,
                    CourseListScope::TenantAdministrator => CourseRole::Administrator,
                };
                Some((course_id.to_string(), record.summary(role)))
            })
            .collect();
        Ok(page_records(records, &page))
    }

    async fn upsert_assignment(
        &self,
        context: TenantContext,
        assignment: AssignmentRecord,
    ) -> Result<(), StoreError> {
        ensure_tenant(context, assignment.tenant)?;
        validate_assignment(&assignment)?;
        let mut state = self.write_state()?;
        if !state
            .courses
            .contains_key(&(assignment.tenant, assignment.course_id))
        {
            return Err(StoreError::InvalidRecord(
                "assignment references a missing course".to_string(),
            ));
        }
        for reference in &assignment.problems {
            let assignable = state
                .published
                .get(&(reference.problem, reference.version))
                .is_some_and(|record| {
                    record.lifecycle.is_assignable()
                        && catalog_record_visible(&state, context.tenant_id(), record)
                });
            if !assignable {
                return Err(StoreError::InvalidRecord(format!(
                    "assignment references a missing, hidden, or inactive published version {}/{}",
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
        course: CourseId,
        page: PageRequest,
    ) -> Result<Page<AssignmentRecord>, StoreError> {
        let state = self.read_state()?;
        if !state.courses.contains_key(&(context.tenant_id(), course)) {
            return Err(StoreError::NotFound);
        }
        let records = state
            .assignments
            .iter()
            .filter(|((tenant, _), record)| {
                *tenant == context.tenant_id() && record.course_id == course
            })
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
        let assignment = state
            .assignments
            .get(&(enrollment.tenant, enrollment.assignment))
            .ok_or_else(|| {
                StoreError::InvalidRecord("enrollment references a missing assignment".to_string())
            })?;
        let course = state
            .courses
            .get(&(enrollment.tenant, assignment.course_id))
            .ok_or(StoreError::NotFound)?;
        if course.role_for(enrollment.user) != Some(CourseRole::Student) {
            return Err(StoreError::InvalidRecord(
                "enrollment user must be a student member of the assignment course".to_string(),
            ));
        }
        let key = (enrollment.tenant, enrollment.id);
        if state.enrollments.contains_key(&key) {
            return Err(StoreError::AlreadyExists);
        }
        if state.enrollments.values().any(|existing| {
            existing.tenant == enrollment.tenant
                && existing.assignment == enrollment.assignment
                && existing.user == enrollment.user
        }) {
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

    async fn start_or_resume_run(
        &self,
        context: TenantContext,
        actor: UserId,
        assignment_id: AssignmentId,
        proposed_run: RunId,
    ) -> Result<AssignmentRun, StoreError> {
        let mut state = self.write_state()?;
        let tenant = context.tenant_id();
        let enrollment = state
            .enrollments
            .values()
            .find(|enrollment| {
                enrollment.tenant == tenant
                    && enrollment.assignment == assignment_id
                    && enrollment.user == actor
            })
            .cloned()
            .ok_or(StoreError::NotFound)?;
        if let Some(active) = state.runs.values().find(|run| {
            run.tenant == tenant && run.enrollment == enrollment.id && run.completed_at.is_none()
        }) {
            return Ok(active.clone());
        }
        if state.runs.contains_key(&(tenant, proposed_run)) {
            return Err(StoreError::AlreadyExists);
        }
        let assignment = assignment_record(&state, tenant, assignment_id)?;
        let previous = state
            .summaries
            .get(&(tenant, enrollment.id))
            .cloned()
            .ok_or(StoreError::NotFound)?;
        if !continued_practice_allows_run(&previous, assignment.policies.continued_practice) {
            return Err(StoreError::InvalidRecord(
                "continued-practice policy does not permit another run".to_string(),
            ));
        }
        let run_number = state
            .runs
            .values()
            .filter(|run| run.tenant == tenant && run.enrollment == enrollment.id)
            .map(|run| run.run_number)
            .max()
            .unwrap_or(0)
            .checked_add(1)
            .ok_or_else(|| StoreError::InvalidRecord("run number overflow".to_string()))?;
        let run = AssignmentRun {
            id: proposed_run,
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
        state.runs.insert((tenant, run.id), run.clone());
        state.summaries.insert((tenant, enrollment.id), next);
        Ok(run)
    }

    async fn issue_or_resume_question_attempt(
        &self,
        context: TenantContext,
        command: IssueQuestionAttemptCommand,
    ) -> Result<QuestionAttempt, StoreError> {
        let mut state = self.write_state()?;
        let tenant = context.tenant_id();
        let run = state
            .runs
            .get(&(tenant, command.run))
            .cloned()
            .ok_or(StoreError::NotFound)?;
        if run.completed_at.is_some() || run.score.is_some() {
            return Err(StoreError::InvalidRecord(
                "a completed run cannot issue another question".to_string(),
            ));
        }
        let enrollment = enrollment_record(&state, tenant, run.enrollment)?;
        if enrollment.user != command.actor {
            return Err(StoreError::Forbidden);
        }
        let assignment = assignment_record(&state, tenant, enrollment.assignment)?;
        validate_assignment_position(&assignment, &command)?;

        let unresolved = state
            .attempts
            .values()
            .filter(|attempt| {
                attempt.tenant == tenant
                    && attempt.run == run.id
                    && !state.submissions.contains_key(&(tenant, attempt.id))
            })
            .max_by_key(|attempt| (attempt.timer.issued_at, attempt.id));
        if let Some(active) = unresolved {
            if active.assignment_position == command.assignment_position {
                return Ok(active.clone());
            }
            return Err(StoreError::InvalidRecord(
                "another question attempt is already active in this run".to_string(),
            ));
        }
        let latest_for_position = state
            .attempts
            .values()
            .filter(|attempt| {
                attempt.tenant == tenant
                    && attempt.run == run.id
                    && attempt.assignment_position == command.assignment_position
            })
            .max_by_key(|attempt| (attempt.timer.issued_at, attempt.id));
        if latest_for_position.is_some_and(|latest| {
            state
                .submissions
                .get(&(tenant, latest.id))
                .and_then(|submission| submission.record.attempt.result)
                .is_some_and(|result| result.correct)
        }) {
            return Err(StoreError::InvalidRecord(
                "a correct question position cannot be retried".to_string(),
            ));
        }
        if state.attempts.contains_key(&(tenant, command.attempt)) {
            return Err(StoreError::AlreadyExists);
        }
        if command.parameter_hash.trim().is_empty()
            || command
                .provenance
                .rendered_question_sha256
                .trim()
                .is_empty()
        {
            return Err(StoreError::InvalidRecord(
                "issued attempt hashes must not be empty".to_string(),
            ));
        }
        let question = state
            .published
            .get(&(command.problem, command.question_version))
            .ok_or(StoreError::NotFound)?;
        let timer = issued_timer(
            state.authoritative_time,
            &run,
            question.question.timing_policy,
        )?;
        let attempt = QuestionAttempt {
            id: command.attempt,
            tenant,
            run: run.id,
            problem: command.problem,
            question_version: command.question_version,
            assignment_position: command.assignment_position,
            seed: command.seed,
            parameter_hash: command.parameter_hash,
            response: None,
            result: None,
            timer,
            provenance: command.provenance,
        };
        state.attempts.insert((tenant, attempt.id), attempt.clone());
        Ok(attempt)
    }

    async fn list_question_attempts(
        &self,
        context: TenantContext,
        run: RunId,
        page: PageRequest,
    ) -> Result<Page<QuestionAttempt>, StoreError> {
        let state = self.read_state()?;
        if !state.runs.contains_key(&(context.tenant_id(), run)) {
            return Err(StoreError::NotFound);
        }
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

    async fn replay_submission(
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
        require_attempt_owner(&state, tenant, attempt, actor)?;
        let Some(stored) = state.submissions.get(&(tenant, attempt_id)) else {
            return Ok(None);
        };
        if &stored.key != idempotency_key || &stored.response != response {
            return Err(StoreError::Conflict);
        }
        Ok(Some(stored.record.clone()))
    }

    async fn submit_question_attempt(
        &self,
        context: TenantContext,
        command: SubmitQuestionAttemptCommand,
    ) -> Result<SubmissionRecord, StoreError> {
        let mut state = self.write_state()?;
        let tenant = context.tenant_id();
        let base = state
            .attempts
            .get(&(tenant, command.attempt))
            .cloned()
            .ok_or(StoreError::NotFound)?;
        require_attempt_owner(&state, tenant, &base, command.actor)?;
        if let Some(stored) = state.submissions.get(&(tenant, command.attempt)) {
            if stored.key == command.idempotency_key && stored.response == command.response {
                return Ok(stored.record.clone());
            }
            return Err(StoreError::Conflict);
        }

        let mut run = state
            .runs
            .get(&(tenant, base.run))
            .cloned()
            .ok_or(StoreError::NotFound)?;
        if run.completed_at.is_some() || run.score.is_some() {
            return Err(StoreError::Conflict);
        }
        let mut enrollment = enrollment_record(&state, tenant, run.enrollment)?;
        let assignment = assignment_record(&state, tenant, enrollment.assignment)?;
        let question = state
            .published
            .get(&(base.problem, base.question_version))
            .ok_or(StoreError::NotFound)?;
        crate::validate_attempt_result(command.result)?;
        let submitted_at = state.authoritative_time;
        let mut submitted = base.clone();
        submitted.response = Some(command.response.clone());
        submitted.result = Some(command.result);
        submitted.timer.submitted_at = Some(submitted_at);
        let verdict = timer_verdict(&TimerEvaluation {
            policy: question.question.timing_policy,
            timer: submitted.timer,
            evaluated_at: submitted_at,
            pause_extension_millis: 0,
        })
        .map_err(|error| StoreError::InvalidRecord(error.to_string()))?;
        if verdict == TimerVerdict::TimedOut {
            return Err(StoreError::TimedOut);
        }

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
        let results = current_results_by_position(&state, tenant, &run, &assignment, &submitted);
        if let Some(score) = completed_run_score(&results, assignment.policies.completion)? {
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
        }
        let record = SubmissionRecord {
            attempt: submitted,
            run: run.clone(),
            summary: next.clone(),
        };
        state.submissions.insert(
            (tenant, command.attempt),
            StoredSubmission {
                key: command.idempotency_key,
                response: command.response,
                record: record.clone(),
            },
        );
        state.runs.insert((tenant, run.id), run);
        state
            .enrollments
            .insert((tenant, enrollment.id), enrollment);
        state.summaries.insert((tenant, next.enrollment), next);
        Ok(record)
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
        Ok(state
            .attempts
            .get(&(context.tenant_id(), attempt))
            .map(|record| projected_attempt(&state, context.tenant_id(), record)))
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

fn validate_assignment_position(
    assignment: &AssignmentRecord,
    command: &IssueQuestionAttemptCommand,
) -> Result<(), StoreError> {
    let position = usize::try_from(command.assignment_position)
        .map_err(|_| StoreError::InvalidRecord("assignment position is too large".to_string()))?;
    let expected = assignment.problems.get(position).ok_or_else(|| {
        StoreError::InvalidRecord("question position is outside the assignment".to_string())
    })?;
    if expected.problem != command.problem || expected.version != command.question_version {
        return Err(StoreError::InvalidRecord(
            "question identity does not match its assignment position".to_string(),
        ));
    }
    Ok(())
}

fn issued_timer(
    issued_at: ActivityTimestamp,
    run: &AssignmentRun,
    policy: TimingPolicy,
) -> Result<AttemptTimerRecord, StoreError> {
    let deadline = match policy {
        TimingPolicy::Untimed => None,
        TimingPolicy::PerQuestion { seconds, .. } => {
            Some(add_seconds(issued_at, seconds, "question deadline")?)
        }
        TimingPolicy::PerAttempt { seconds, .. } => {
            let deadline = add_seconds(run.started_at, seconds, "run deadline")?;
            if deadline < issued_at {
                return Err(StoreError::TimedOut);
            }
            Some(deadline)
        }
    };
    Ok(AttemptTimerRecord {
        issued_at,
        deadline,
        submitted_at: None,
    })
}

fn add_seconds(
    timestamp: ActivityTimestamp,
    seconds: u32,
    description: &str,
) -> Result<ActivityTimestamp, StoreError> {
    timestamp
        .as_unix_millis()
        .checked_add(i64::from(seconds) * 1_000)
        .map(ActivityTimestamp::from_unix_millis)
        .ok_or_else(|| StoreError::InvalidRecord(format!("{description} overflow")))
}

fn require_attempt_owner(
    state: &State,
    tenant: TenantId,
    attempt: &QuestionAttempt,
    actor: UserId,
) -> Result<(), StoreError> {
    let run = state
        .runs
        .get(&(tenant, attempt.run))
        .ok_or(StoreError::NotFound)?;
    let enrollment = enrollment_record(state, tenant, run.enrollment)?;
    if enrollment.user == actor {
        Ok(())
    } else {
        Err(StoreError::NotFound)
    }
}

fn projected_attempt(
    state: &State,
    tenant: TenantId,
    attempt: &QuestionAttempt,
) -> QuestionAttempt {
    state
        .submissions
        .get(&(tenant, attempt.id))
        .map_or_else(|| attempt.clone(), |stored| stored.record.attempt.clone())
}

fn current_results_by_position(
    state: &State,
    tenant: TenantId,
    run: &AssignmentRun,
    assignment: &AssignmentRecord,
    current: &QuestionAttempt,
) -> Vec<Option<AttemptResult>> {
    let mut latest: Vec<Option<(ActivityTimestamp, QuestionAttemptId, AttemptResult)>> =
        vec![None; assignment.problems.len()];
    for base in state
        .attempts
        .values()
        .filter(|attempt| attempt.tenant == tenant && attempt.run == run.id)
    {
        let projected = if base.id == current.id {
            current.clone()
        } else {
            projected_attempt(state, tenant, base)
        };
        let (Some(submitted_at), Some(result)) = (projected.timer.submitted_at, projected.result)
        else {
            continue;
        };
        let Ok(position) = usize::try_from(projected.assignment_position) else {
            continue;
        };
        let Some(slot) = latest.get_mut(position) else {
            continue;
        };
        if slot
            .as_ref()
            .is_none_or(|(at, id, _)| (submitted_at, projected.id) > (*at, *id))
        {
            *slot = Some((submitted_at, projected.id, result));
        }
    }
    latest
        .into_iter()
        .map(|entry| entry.map(|(_, _, result)| result))
        .collect()
}

impl MemoryStore {
    /// Sets the stub backend clock used by session tests and local development.
    pub fn set_authoritative_time(&self, now: ActivityTimestamp) -> Result<(), StoreError> {
        self.write_state()?.authoritative_time = now;
        Ok(())
    }

    /// Returns protected asset access events for conformance assertions.
    pub fn asset_access_events(&self) -> Result<Vec<AssetAccessEvent>, StoreError> {
        Ok(self.read_state()?.asset_access_events.clone())
    }

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

fn catalog_record_visible(
    state: &State,
    tenant: TenantId,
    record: &PublishedProblemRecord,
) -> bool {
    record.scope == PublicationScope::Public
        || state
            .catalog_grants
            .contains(&(tenant, record.problem, record.version))
}

fn validated_deprecation_reason(reason: String) -> Result<String, StoreError> {
    const MAX_REASON_CHARS: usize = 1_000;
    let reason = reason.trim();
    if reason.is_empty() {
        return Err(StoreError::InvalidRecord(
            "deprecation requires a nonempty reason".to_string(),
        ));
    }
    if reason.chars().count() > MAX_REASON_CHARS {
        return Err(StoreError::InvalidRecord(format!(
            "deprecation reason must contain at most {MAX_REASON_CHARS} characters"
        )));
    }
    Ok(reason.to_string())
}

fn taxonomy_cursor_key(term: &TaxonomyTerm) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";

    let mut key = String::with_capacity((term.scheme.len() + term.code.len()) * 2 + 1);
    for byte in term.scheme.bytes() {
        key.push(char::from(HEX[usize::from(byte >> 4)]));
        key.push(char::from(HEX[usize::from(byte & 0x0f)]));
    }
    key.push('/');
    for byte in term.code.bytes() {
        key.push(char::from(HEX[usize::from(byte >> 4)]));
        key.push(char::from(HEX[usize::from(byte & 0x0f)]));
    }
    key
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

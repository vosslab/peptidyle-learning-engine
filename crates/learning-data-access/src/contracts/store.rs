use super::*;

/// Portable persistence failure with no SQL type in its variants.
#[derive(Debug, Clone, PartialEq)]
pub enum StoreError {
    /// Requested record is absent in the active tenant or shared catalog.
    NotFound,
    /// Immutable identity already exists.
    AlreadyExists,
    /// A tenant-owned record disagrees with authenticated context.
    TenantMismatch,
    /// Stored state changed after a caller validated its expected value.
    Conflict,
    /// PostgreSQL aborted the whole transaction due to a serialization or deadlock conflict.
    RetryableTransaction,
    /// Authenticated identity lacks ownership or role for the operation.
    Forbidden,
    /// Record shape violates a model invariant.
    InvalidRecord(String),
    /// Pure activity projection rejected the transition.
    RunModel(RunModelError),
    /// The database-authoritative timer no longer accepts this response.
    TimedOut,
    /// Backend state is temporarily unavailable.
    Unavailable(String),
}

impl std::fmt::Display for StoreError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotFound => write!(formatter, "record not found"),
            Self::AlreadyExists => write!(formatter, "immutable record already exists"),
            Self::TenantMismatch => write!(formatter, "record tenant does not match context"),
            Self::Conflict => write!(formatter, "record changed before the operation committed"),
            Self::RetryableTransaction => write!(formatter, "transaction must be retried"),
            Self::Forbidden => write!(formatter, "operation is not authorized"),
            Self::InvalidRecord(message) => write!(formatter, "invalid record: {message}"),
            Self::RunModel(error) => write!(formatter, "activity transition rejected: {error}"),
            Self::TimedOut => write!(formatter, "question attempt timed out"),
            Self::Unavailable(message) => write!(formatter, "store unavailable: {message}"),
        }
    }
}

impl std::error::Error for StoreError {}

impl From<RunModelError> for StoreError {
    fn from(error: RunModelError) -> Self {
        Self::RunModel(error)
    }
}

/// Catalog operations that require visibility, ownership, and atomic publish.
#[async_trait]
pub trait CatalogStore: Send + Sync {
    /// Validates the stored draft expectation and atomically publishes it.
    async fn publish_draft(
        &self,
        context: TenantContext,
        actor: UserId,
        command: PublishDraftCommand,
    ) -> Result<PublishedProblemRecord, StoreError>;

    /// Resolves an exact visible version, including deprecated or archived ones.
    async fn get_catalog_problem(
        &self,
        context: TenantContext,
        reference: ProblemVersionRef,
    ) -> Result<Option<PublishedProblemRecord>, StoreError>;

    /// Resolves a copyable catalog reference under the caller's visibility.
    /// A stable reference selects the latest assignable version; an exact
    /// reference never silently upgrades to another version.
    async fn resolve_catalog_problem(
        &self,
        context: TenantContext,
        reference: question_model::ProblemDisplayRef,
    ) -> Result<Option<PublishedProblemRecord>, StoreError>;

    /// Lists discoverable hot metadata in stable cursor order.
    async fn list_catalog(
        &self,
        context: TenantContext,
        page: PageRequest,
    ) -> Result<Page<CatalogProblemSummary>, StoreError>;

    /// Lists distinct controlled taxonomy terms in stable cursor order.
    async fn list_catalog_taxonomy(
        &self,
        context: TenantContext,
        page: PageRequest,
    ) -> Result<Page<TaxonomyTerm>, StoreError>;

    /// Searches hot discoverable metadata and returns rows plus server-side
    /// facets from one normalized-query snapshot. Implementations must reject
    /// a cursor issued for a different normalized query and must never load
    /// `problem_version_payload` merely to browse or aggregate.
    async fn search_catalog(
        &self,
        context: TenantContext,
        query: CatalogSearchQuery,
    ) -> Result<CatalogSearchPage, StoreError>;

    /// Returns a safe exact immutable catalog-detail projection. This default
    /// retains compatibility for focused test stores while production stores
    /// may use a hot metadata projection instead of loading source bindings.
    async fn get_catalog_detail(
        &self,
        context: TenantContext,
        reference: ProblemVersionRef,
    ) -> Result<Option<CatalogProblemDetail>, StoreError> {
        Ok(self
            .get_catalog_problem(context, reference)
            .await?
            .map(|record| CatalogProblemDetail {
                summary: record.summary(),
                prompt: record.question.prompt,
                statistics: question_model::CatalogStatisticsStatus::Unavailable,
            }))
    }

    /// Applies an author-owned, one-way post-publication transition.
    async fn transition_catalog_problem(
        &self,
        context: TenantContext,
        actor: UserId,
        reference: ProblemVersionRef,
        transition: CatalogTransition,
    ) -> Result<PublishedProblemRecord, StoreError>;
}

/// Private catalog bridge from an exact visible version to its source bytes.
///
/// This trait is intentionally not part of any browser DTO or public asset
/// delivery API. A foreign tenant receives `None` before an object store is
/// consulted, which keeps source-object existence tenant-isolated.
#[async_trait]
pub trait CatalogSourceStore: Send + Sync {
    /// Resolves the exact source binding for one visible immutable version.
    async fn catalog_source_artifact(
        &self,
        context: TenantContext,
        reference: ProblemVersionRef,
    ) -> Result<Option<PublishedSourceArtifact>, StoreError>;
}

/// Persistence operations consumed by catalog, course, run, and worker lanes.
/// Complete persistence contract consumed by server and worker lanes.
#[async_trait]
pub trait Store:
    StatisticsStore
    + AuthoringStore
    + CourseStore
    + CourseAssignmentStore
    + AssignmentPolicyStore
    + RunStore
    + FeedbackStore
    + ActivityStore
{
    /// Delegates to the focused [`StatisticsStore`] capability.
    async fn question_statistics(
        &self,
        _context: TenantContext,
        _reference: ProblemVersionRef,
    ) -> Result<QuestionStatisticsDisclosure, StoreError> {
        StatisticsStore::question_statistics_impl(self, _context, _reference).await
    }

    /// Delegates to the focused [`StatisticsStore`] capability.
    async fn list_gradebook_rows(
        &self,
        context: TenantContext,
        course: CourseId,
        page: PageRequest,
    ) -> Result<Page<GradebookSummaryRow>, StoreError> {
        StatisticsStore::list_gradebook_rows_impl(self, context, course, page).await
    }

    /// Delegates to the focused [`AuthoringStore`] capability.
    async fn upsert_draft(
        &self,
        context: TenantContext,
        actor: UserId,
        expected_revision: Option<WorkspaceDraftRevision>,
        draft: DraftRecord,
    ) -> Result<WorkspaceDraft, StoreError> {
        AuthoringStore::upsert_draft_impl(self, context, actor, expected_revision, draft).await
    }

    /// Delegates to the focused [`AuthoringStore`] capability.
    async fn get_draft(
        &self,
        context: TenantContext,
        actor: UserId,
        workspace: WorkspaceId,
    ) -> Result<Option<WorkspaceDraft>, StoreError> {
        AuthoringStore::get_draft_impl(self, context, actor, workspace).await
    }

    /// Delegates to the focused [`AuthoringStore`] capability.
    async fn list_drafts(
        &self,
        context: TenantContext,
        actor: UserId,
        page: PageRequest,
    ) -> Result<Page<WorkspaceDraftSummary>, StoreError> {
        AuthoringStore::list_drafts_impl(self, context, actor, page).await
    }

    /// Delegates to the focused [`AuthoringStore`] capability.
    async fn delete_draft(
        &self,
        context: TenantContext,
        actor: UserId,
        workspace: WorkspaceId,
        expected_revision: WorkspaceDraftRevision,
    ) -> Result<bool, StoreError> {
        AuthoringStore::delete_draft_impl(self, context, actor, workspace, expected_revision).await
    }

    /// Delegates to the focused [`AuthoringStore`] capability.
    async fn grant_draft_collaborator(
        &self,
        context: TenantContext,
        actor: UserId,
        workspace: WorkspaceId,
        collaborator: UserId,
    ) -> Result<(), StoreError> {
        AuthoringStore::grant_draft_collaborator_impl(self, context, actor, workspace, collaborator)
            .await
    }

    /// Delegates to the focused [`AuthoringStore`] capability.
    async fn get_published_problem(
        &self,
        problem: ProblemId,
        version: VersionId,
    ) -> Result<Option<PublishedProblemRecord>, StoreError> {
        AuthoringStore::get_published_problem_impl(self, problem, version).await
    }

    /// Delegates to the focused [`AuthoringStore`] capability.
    async fn list_published_problems(
        &self,
        page: PageRequest,
    ) -> Result<Page<PublishedProblemRecord>, StoreError> {
        AuthoringStore::list_published_problems_impl(self, page).await
    }

    /// Delegates to the focused [`CourseStore`] capability.
    async fn upsert_course(
        &self,
        context: TenantContext,
        course: CourseRecord,
    ) -> Result<(), StoreError> {
        CourseStore::upsert_course_impl(self, context, course).await
    }

    /// Delegates to the focused [`CourseStore`] capability.
    async fn get_course(
        &self,
        context: TenantContext,
        course: CourseId,
    ) -> Result<Option<CourseRecord>, StoreError> {
        CourseStore::get_course_impl(self, context, course).await
    }

    /// Delegates to the focused [`CourseStore`] capability.
    async fn list_courses(
        &self,
        context: TenantContext,
        scope: CourseListScope,
        page: PageRequest,
    ) -> Result<Page<CourseSummary>, StoreError> {
        CourseStore::list_courses_impl(self, context, scope, page).await
    }

    /// Delegates to the focused [`CourseStore`] capability.
    async fn put_course_group(
        &self,
        context: TenantContext,
        command: PutCourseGroupCommand,
    ) -> Result<StoredCourseGroup, StoreError> {
        CourseStore::put_course_group_impl(self, context, command).await
    }

    /// Delegates to the focused [`CourseStore`] capability.
    async fn get_course_group(
        &self,
        context: TenantContext,
        group: CourseGroupId,
    ) -> Result<Option<StoredCourseGroup>, StoreError> {
        CourseStore::get_course_group_impl(self, context, group).await
    }

    /// Creates a non-editor assignment with the internal explicit Untimed
    /// policy. This method is not used by the browser editor contract.
    async fn create_untimed_assignment(
        &self,
        context: TenantContext,
        assignment: AssignmentRecord,
    ) -> Result<StoredAssignment, StoreError> {
        CourseAssignmentStore::create_untimed_assignment_impl(self, context, assignment).await
    }

    /// Atomically creates an assignment and its editor-owned run timing.
    async fn create_assignment_with_timing(
        &self,
        context: TenantContext,
        assignment: AssignmentRecord,
        assignment_timing: question_model::AssignmentRunTiming,
    ) -> Result<StoredAssignment, StoreError> {
        CourseAssignmentStore::create_assignment_with_timing_impl(
            self,
            context,
            assignment,
            assignment_timing,
        )
        .await
    }

    /// Replaces non-timing fields while retaining the stored timer in internal
    /// workflows that do not own timing.
    async fn replace_assignment_preserving_timing(
        &self,
        context: TenantContext,
        course: CourseId,
        assignment: AssignmentId,
        expected_revision: AssignmentRevision,
        update: AssignmentUpdate,
    ) -> Result<StoredAssignment, StoreError> {
        CourseAssignmentStore::replace_assignment_preserving_timing_impl(
            self,
            context,
            course,
            assignment,
            expected_revision,
            update,
        )
        .await
    }

    /// Atomically replaces an assignment definition and its run timing.
    async fn replace_assignment_with_timing(
        &self,
        context: TenantContext,
        course: CourseId,
        assignment: AssignmentId,
        expected_revision: AssignmentRevision,
        update: AssignmentEditorUpdate,
    ) -> Result<StoredAssignment, StoreError> {
        CourseAssignmentStore::replace_assignment_with_timing_impl(
            self,
            context,
            course,
            assignment,
            expected_revision,
            update,
        )
        .await
    }

    /// Delegates to the focused [`CourseAssignmentStore`] capability.
    async fn delete_and_regrade_assignment_item(
        &self,
        context: TenantContext,
        command: DeleteAndRegradeAssignmentItemCommand,
    ) -> Result<StoredAssignment, StoreError> {
        CourseAssignmentStore::delete_and_regrade_assignment_item_impl(self, context, command).await
    }

    /// Delegates to the focused [`CourseAssignmentStore`] capability.
    async fn get_assignment_for_edit(
        &self,
        context: TenantContext,
        assignment: AssignmentId,
    ) -> Result<Option<StoredAssignment>, StoreError> {
        CourseAssignmentStore::get_assignment_for_edit_impl(self, context, assignment).await
    }

    /// Delegates to the focused [`CourseAssignmentStore`] capability.
    async fn get_assignment(
        &self,
        context: TenantContext,
        assignment: AssignmentId,
    ) -> Result<Option<AssignmentRecord>, StoreError> {
        CourseAssignmentStore::get_assignment_impl(self, context, assignment).await
    }

    /// Delegates to the focused [`CourseAssignmentStore`] capability.
    async fn list_assignments(
        &self,
        context: TenantContext,
        course: CourseId,
        page: PageRequest,
    ) -> Result<Page<AssignmentRecord>, StoreError> {
        CourseAssignmentStore::list_assignments_impl(self, context, course, page).await
    }

    /// Delegates to the focused [`CourseAssignmentStore`] capability.
    async fn create_enrollment(
        &self,
        context: TenantContext,
        enrollment: AssignmentEnrollment,
    ) -> Result<(), StoreError> {
        CourseAssignmentStore::create_enrollment_impl(self, context, enrollment).await
    }

    /// Delegates to the focused [`CourseAssignmentStore`] capability.
    async fn get_enrollment(
        &self,
        context: TenantContext,
        enrollment: EnrollmentId,
    ) -> Result<Option<AssignmentEnrollment>, StoreError> {
        CourseAssignmentStore::get_enrollment_impl(self, context, enrollment).await
    }

    /// Delegates to the focused [`AssignmentPolicyStore`] capability.
    async fn get_assignment_timing(
        &self,
        context: TenantContext,
        assignment: AssignmentId,
    ) -> Result<Option<StoredAssignmentTiming>, StoreError> {
        AssignmentPolicyStore::get_assignment_timing_impl(self, context, assignment).await
    }

    /// Delegates to the focused [`AssignmentPolicyStore`] capability.
    async fn update_assignment_timing(
        &self,
        context: TenantContext,
        command: UpdateAssignmentTimingCommand,
    ) -> Result<StoredAssignmentTiming, StoreError> {
        AssignmentPolicyStore::update_assignment_timing_impl(self, context, command).await
    }

    /// Delegates to the focused [`AssignmentPolicyStore`] capability.
    async fn set_assignment_policy_exception(
        &self,
        context: TenantContext,
        command: SetAssignmentPolicyExceptionCommand,
    ) -> Result<StoredAssignmentPolicyException, StoreError> {
        AssignmentPolicyStore::set_assignment_policy_exception_impl(self, context, command).await
    }

    /// Delegates to the focused [`AssignmentPolicyStore`] capability.
    async fn delete_assignment_policy_exception(
        &self,
        context: TenantContext,
        command: DeleteAssignmentPolicyExceptionCommand,
    ) -> Result<AssignmentRevision, StoreError> {
        AssignmentPolicyStore::delete_assignment_policy_exception_impl(self, context, command).await
    }

    /// Delegates to the focused [`AssignmentPolicyStore`] capability.
    async fn get_assignment_policy_exception(
        &self,
        context: TenantContext,
        assignment: AssignmentId,
        exception: AssignmentPolicyExceptionId,
    ) -> Result<Option<StoredAssignmentPolicyException>, StoreError> {
        AssignmentPolicyStore::get_assignment_policy_exception_impl(
            self, context, assignment, exception,
        )
        .await
    }

    /// Delegates to the focused [`AssignmentPolicyStore`] capability.
    async fn resolve_assignment_timing(
        &self,
        context: TenantContext,
        assignment: AssignmentId,
        student: StudentId,
    ) -> Result<Option<ResolvedAssignmentTiming>, StoreError> {
        AssignmentPolicyStore::resolve_assignment_timing_impl(self, context, assignment, student)
            .await
    }

    /// Delegates to the focused [`AssignmentPolicyStore`] capability.
    async fn get_attempt_resolved_timing(
        &self,
        context: TenantContext,
        attempt: QuestionAttemptId,
    ) -> Result<Option<ResolvedAttemptTiming>, StoreError> {
        AssignmentPolicyStore::get_attempt_resolved_timing_impl(self, context, attempt).await
    }

    /// Delegates to the focused [`RunStore`] capability.
    async fn start_or_resume_run(
        &self,
        context: TenantContext,
        actor: UserId,
        assignment: AssignmentId,
        proposed_run: RunId,
    ) -> Result<AssignmentRun, StoreError> {
        RunStore::start_or_resume_run_impl(self, context, actor, assignment, proposed_run).await
    }

    /// Delegates to the focused [`RunStore`] capability.
    async fn assignment_run_items(
        &self,
        context: TenantContext,
        run: RunId,
    ) -> Result<Vec<AssignmentRunItem>, StoreError> {
        RunStore::assignment_run_items_impl(self, context, run).await
    }

    /// Delegates to the focused [`RunStore`] capability.
    async fn issue_or_resume_question_attempt(
        &self,
        context: TenantContext,
        command: IssueQuestionAttemptCommand,
    ) -> Result<QuestionAttempt, StoreError> {
        RunStore::issue_or_resume_question_attempt_impl(self, context, command).await
    }

    /// Delegates to the focused [`RunStore`] capability.
    async fn get_attempt_presentation_binding(
        &self,
        context: TenantContext,
        actor: UserId,
        attempt: QuestionAttemptId,
    ) -> Result<Option<PresentationBindingV1>, StoreError> {
        RunStore::get_attempt_presentation_binding_impl(self, context, actor, attempt).await
    }

    /// Reads the private answer-free WeBWorK replay state for one owned attempt.
    async fn get_webwork_grade_replay_state(
        &self,
        context: TenantContext,
        actor: UserId,
        attempt: QuestionAttemptId,
    ) -> Result<Option<WebworkGradeReplayStateV1>, StoreError> {
        RunStore::get_webwork_grade_replay_state_impl(self, context, actor, attempt).await
    }

    /// Delegates to the focused [`RunStore`] capability.
    async fn reserve_or_resume_prefetched_question(
        &self,
        context: TenantContext,
        command: ReservePrefetchedQuestionCommand,
    ) -> Result<PrefetchedQuestion, StoreError> {
        RunStore::reserve_or_resume_prefetched_question_impl(self, context, command).await
    }

    /// Delegates to the focused [`RunStore`] capability.
    async fn get_prefetched_question(
        &self,
        context: TenantContext,
        actor: UserId,
        run: RunId,
        predecessor: QuestionAttemptId,
        assignment_position: u32,
    ) -> Result<Option<PrefetchedQuestion>, StoreError> {
        RunStore::get_prefetched_question_impl(
            self,
            context,
            actor,
            run,
            predecessor,
            assignment_position,
        )
        .await
    }

    /// Delegates to the focused [`RunStore`] capability.
    async fn submission_next_attempt(
        &self,
        context: TenantContext,
        actor: UserId,
        predecessor: QuestionAttemptId,
    ) -> Result<SubmissionNextAttempt, StoreError> {
        RunStore::submission_next_attempt_impl(self, context, actor, predecessor).await
    }

    /// Delegates to the focused [`RunStore`] capability.
    async fn pending_submission_for_run(
        &self,
        context: TenantContext,
        actor: UserId,
        run: RunId,
    ) -> Result<Option<QuestionAttemptId>, StoreError> {
        RunStore::pending_submission_for_run_impl(self, context, actor, run).await
    }

    /// Delegates to the focused [`RunStore`] capability.
    async fn finalize_submission_next_attempt(
        &self,
        context: TenantContext,
        actor: UserId,
        predecessor: QuestionAttemptId,
        next: Option<QuestionAttemptId>,
    ) -> Result<(), StoreError> {
        RunStore::finalize_submission_next_attempt_impl(self, context, actor, predecessor, next)
            .await
    }

    /// Delegates to the focused [`RunStore`] capability.
    async fn list_question_attempts(
        &self,
        context: TenantContext,
        run: RunId,
        page: PageRequest,
    ) -> Result<Page<QuestionAttempt>, StoreError> {
        RunStore::list_question_attempts_impl(self, context, run, page).await
    }

    /// Delegates to the focused [`RunStore`] capability.
    async fn replay_submission(
        &self,
        context: TenantContext,
        actor: UserId,
        attempt: QuestionAttemptId,
        response: &StudentResponse,
        idempotency_key: &SubmissionIdempotencyKey,
    ) -> Result<Option<SubmissionRecord>, StoreError> {
        RunStore::replay_submission_impl(self, context, actor, attempt, response, idempotency_key)
            .await
    }

    /// Delegates to the focused [`RunStore`] capability.
    async fn submit_question_attempt(
        &self,
        context: TenantContext,
        command: SubmitQuestionAttemptCommand,
    ) -> Result<SubmissionRecord, StoreError> {
        RunStore::submit_question_attempt_impl(self, context, command).await
    }

    /// Delegates to the focused [`RunStore`] capability.
    async fn force_submit_attempt(
        &self,
        context: TenantContext,
        command: ForceSubmitAttemptCommand,
    ) -> Result<AttemptSupportRecord, StoreError> {
        RunStore::force_submit_attempt_impl(self, context, command).await
    }

    /// Delegates to the focused [`RunStore`] capability.
    async fn clear_attempt(
        &self,
        context: TenantContext,
        command: ClearAttemptCommand,
    ) -> Result<AttemptSupportRecord, StoreError> {
        RunStore::clear_attempt_impl(self, context, command).await
    }

    /// Delegates to the focused [`FeedbackStore`] capability.
    async fn release_attempt_feedback(
        &self,
        context: TenantContext,
        command: ReleaseAttemptFeedbackCommand,
    ) -> Result<FeedbackReleaseRecord, StoreError> {
        FeedbackStore::release_attempt_feedback_impl(self, context, command).await
    }

    /// Delegates to the focused [`FeedbackStore`] capability.
    async fn get_attempt_feedback_release(
        &self,
        context: TenantContext,
        actor: UserId,
        attempt: QuestionAttemptId,
    ) -> Result<Option<FeedbackReleaseRecord>, StoreError> {
        FeedbackStore::get_attempt_feedback_release_impl(self, context, actor, attempt).await
    }

    /// Delegates to the focused [`FeedbackStore`] capability.
    async fn get_run_summary_page(
        &self,
        context: TenantContext,
        actor: UserId,
        run: RunId,
        page: PageRequest,
    ) -> Result<RunSummaryPageInput, StoreError> {
        FeedbackStore::get_run_summary_page_impl(self, context, actor, run, page).await
    }

    /// Delegates to the focused [`ActivityStore`] capability.
    async fn apply_activity_transition(
        &self,
        context: TenantContext,
        transition: ActivityTransition,
    ) -> Result<StudentAssignmentSummary, StoreError> {
        ActivityStore::apply_activity_transition_impl(self, context, transition).await
    }

    /// Delegates to the focused [`ActivityStore`] capability.
    async fn get_run(
        &self,
        context: TenantContext,
        run: RunId,
    ) -> Result<Option<AssignmentRun>, StoreError> {
        ActivityStore::get_run_impl(self, context, run).await
    }

    /// Delegates to the focused [`ActivityStore`] capability.
    async fn list_runs(
        &self,
        context: TenantContext,
        enrollment: EnrollmentId,
        page: PageRequest,
    ) -> Result<Page<AssignmentRun>, StoreError> {
        ActivityStore::list_runs_impl(self, context, enrollment, page).await
    }

    /// Delegates to the focused [`ActivityStore`] capability.
    async fn get_question_attempt(
        &self,
        context: TenantContext,
        attempt: QuestionAttemptId,
    ) -> Result<Option<QuestionAttempt>, StoreError> {
        ActivityStore::get_question_attempt_impl(self, context, attempt).await
    }

    /// Delegates to the focused [`ActivityStore`] capability.
    async fn get_summary(
        &self,
        context: TenantContext,
        enrollment: EnrollmentId,
    ) -> Result<Option<StudentAssignmentSummary>, StoreError> {
        ActivityStore::get_summary_impl(self, context, enrollment).await
    }
}

#[async_trait]
impl<T> Store for T where
    T: StatisticsStore
        + AuthoringStore
        + CourseStore
        + CourseAssignmentStore
        + AssignmentPolicyStore
        + RunStore
        + FeedbackStore
        + ActivityStore
{
}

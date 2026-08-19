use async_trait::async_trait;

use super::*;

/// Focused persistence capability composed by [`Store`].
#[async_trait]
pub trait StatisticsStore: Send + Sync {
    /// Returns only the globally k-anonymous metrics visible for one catalog
    /// version.  This has no contribution-write counterpart: submission
    /// completion owns that server-only capability.
    async fn question_statistics_impl(
        &self,
        _context: TenantContext,
        _reference: ProblemVersionRef,
    ) -> Result<QuestionStatisticsDisclosure, StoreError> {
        Ok(QuestionStatisticsDisclosure::Suppressed)
    }

    /// Lists compact gradebook rows for one tenant-owned course.
    ///
    /// The stable cursor is the backend-owned `(assignment_id, enrollment_id)`
    /// key. Implementations read assignment, enrollment, and maintained
    /// summary rows only; they do not scan run or attempt history.
    async fn list_gradebook_rows_impl(
        &self,
        context: TenantContext,
        course: CourseId,
        page: PageRequest,
    ) -> Result<Page<GradebookSummaryRow>, StoreError>;
}

/// Focused persistence capability composed by [`Store`].
#[async_trait]
pub trait AuthoringStore: Send + Sync {
    /// Creates or replaces a tenant-owned editable draft for an authorized actor.
    ///
    /// The first save must pass `None` and atomically establishes `actor` as
    /// owner. Later saves require the exact revision returned by a prior read
    /// or save, so a stale browser tab cannot overwrite newer author work.
    async fn upsert_draft_impl(
        &self,
        context: TenantContext,
        actor: UserId,
        expected_revision: Option<WorkspaceDraftRevision>,
        draft: DraftRecord,
    ) -> Result<WorkspaceDraft, StoreError>;

    /// Reads a draft only when `actor` has an explicit workspace binding.
    async fn get_draft_impl(
        &self,
        context: TenantContext,
        actor: UserId,
        workspace: WorkspaceId,
    ) -> Result<Option<WorkspaceDraft>, StoreError>;

    /// Lists compact private workspace-draft summaries visible to `actor` in
    /// tenant-bound cursor order.
    async fn list_drafts_impl(
        &self,
        context: TenantContext,
        actor: UserId,
        page: PageRequest,
    ) -> Result<Page<WorkspaceDraftSummary>, StoreError>;

    /// Removes only one unversioned draft in the active tenant.
    ///
    /// The caller supplies the revision obtained from a successful read or
    /// save.  The implementation compares that revision and verifies owner
    /// authority in the same removal operation, so a stale tab cannot delete
    /// newer author work.
    ///
    /// `false` deliberately covers an absent or foreign-tenant workspace, so
    /// callers do not gain an existence oracle through deletion.
    async fn delete_draft_impl(
        &self,
        context: TenantContext,
        actor: UserId,
        workspace: WorkspaceId,
        expected_revision: WorkspaceDraftRevision,
    ) -> Result<bool, StoreError>;

    /// Adds a collaborator to an existing workspace.
    ///
    /// Only the persisted owner may grant this access. Repeating the same
    /// grant is idempotent so an interrupted invitation retry is harmless.
    async fn grant_draft_collaborator_impl(
        &self,
        context: TenantContext,
        actor: UserId,
        workspace: WorkspaceId,
        collaborator: UserId,
    ) -> Result<(), StoreError>;

    /// Resolves one exact globally public version.
    ///
    /// Tenant-visible institution content is intentionally absent; use
    /// [`CatalogStore::get_catalog_problem`] when a session tenant is known.
    async fn get_published_problem_impl(
        &self,
        problem: ProblemId,
        version: VersionId,
    ) -> Result<Option<PublishedProblemRecord>, StoreError>;

    /// Lists globally public, discoverable versions in stable cursor order.
    ///
    /// Tenant-visible institution content is intentionally absent; use
    /// [`CatalogStore::list_catalog`] when a session tenant is known.
    async fn list_published_problems_impl(
        &self,
        page: PageRequest,
    ) -> Result<Page<PublishedProblemRecord>, StoreError>;
}

/// Focused persistence capability composed by [`Store`].
#[async_trait]
pub trait CourseStore: Send + Sync {
    /// Atomically creates a tenant-owned course and its mandatory initial
    /// instructor membership.
    async fn create_course_impl(
        &self,
        context: TenantContext,
        command: CreateCourseCommand,
    ) -> Result<(), StoreError>;

    /// Reads one course inside the active tenant for authorization checks.
    async fn get_course_impl(
        &self,
        context: TenantContext,
        course: CourseId,
    ) -> Result<Option<CourseRecord>, StoreError>;

    /// Reads the sole current direct-authority episode for one user. Historical
    /// episodes remain available only through their immutable receipts.
    async fn get_current_course_membership_impl(
        &self,
        context: TenantContext,
        course: CourseId,
        user: UserId,
    ) -> Result<Option<CourseMembershipRecord>, StoreError>;

    /// Lists courses visible to a member or sysadmin.
    async fn list_courses_impl(
        &self,
        context: TenantContext,
        scope: CourseListScope,
        page: PageRequest,
    ) -> Result<Page<CourseSummary>, StoreError>;

    /// Creates or conditionally replaces one instructor-owned course group.
    /// Membership edits immediately re-resolve active attempts for every
    /// assignment exception that targets this group.
    async fn put_course_group_impl(
        &self,
        context: TenantContext,
        command: PutCourseGroupCommand,
    ) -> Result<StoredCourseGroup, StoreError>;

    /// Reads one current course group inside the active tenant.
    async fn get_course_group_impl(
        &self,
        context: TenantContext,
        group: CourseGroupId,
    ) -> Result<Option<StoredCourseGroup>, StoreError>;
}

/// Focused persistence capability composed by [`Store`].
#[async_trait]
pub trait CourseAssignmentStore: Send + Sync {
    /// Creates a non-editor assignment with the explicitly chosen Untimed
    /// policy. Internal provisioning uses this narrower command; browser
    /// editor requests always carry timing on the wire.
    async fn create_untimed_assignment_impl(
        &self,
        context: TenantContext,
        assignment: AssignmentRecord,
    ) -> Result<StoredAssignment, StoreError> {
        self.create_assignment_with_timing_impl(
            context,
            assignment,
            question_model::AssignmentRunTiming::default(),
        )
        .await
    }

    /// Creates the definition and its editor-owned run timing in one commit,
    /// returning its initial strong revision token.
    async fn create_assignment_with_timing_impl(
        &self,
        context: TenantContext,
        assignment: AssignmentRecord,
        assignment_timing: question_model::AssignmentRunTiming,
    ) -> Result<StoredAssignment, StoreError>;

    /// Replaces non-timing assignment fields while retaining the currently
    /// persisted timing choice for internal workflows that never own timing.
    async fn replace_assignment_preserving_timing_impl(
        &self,
        context: TenantContext,
        course: CourseId,
        assignment: AssignmentId,
        expected_revision: AssignmentRevision,
        update: AssignmentUpdate,
    ) -> Result<StoredAssignment, StoreError> {
        let current = self
            .get_assignment_for_edit_impl(context, assignment)
            .await?
            .ok_or(StoreError::NotFound)?;
        self.replace_assignment_with_timing_impl(
            context,
            course,
            assignment,
            expected_revision,
            AssignmentEditorUpdate {
                assignment: update,
                assignment_timing: current.assignment_timing,
            },
        )
        .await
    }

    /// Replaces definition and editor timing under one shared revision.
    async fn replace_assignment_with_timing_impl(
        &self,
        context: TenantContext,
        course: CourseId,
        assignment: AssignmentId,
        expected_revision: AssignmentRevision,
        update: AssignmentEditorUpdate,
    ) -> Result<StoredAssignment, StoreError>;

    /// Replaces one fixed item for future runs under the assignment's strong
    /// revision token. The command identifies the stable assignment-owned slot
    /// and supplies an exact publication already resolved from a Question ID;
    /// the persisted slot's assignment-authored settings remain in force.
    async fn replace_assignment_fixed_item_impl(
        &self,
        context: TenantContext,
        command: ReplaceAssignmentFixedItemCommand,
    ) -> Result<StoredAssignment, StoreError>;

    /// Inserts one fresh fixed item before learner evidence exists. The
    /// command's item carries its requested visible position and exact
    /// server-resolved immutable publication.
    async fn add_assignment_fixed_item_impl(
        &self,
        context: TenantContext,
        command: AddAssignmentFixedItemCommand,
    ) -> Result<StoredAssignment, StoreError>;

    /// Removes one fixed item before learner evidence exists. The focused
    /// post-evidence workflow remains Delete and Regrade.
    async fn remove_assignment_fixed_item_impl(
        &self,
        context: TenantContext,
        command: RemoveAssignmentFixedItemCommand,
    ) -> Result<StoredAssignment, StoreError>;

    /// Retires one fixed item or selection candidate and recalculates all current grades.
    ///
    /// The command is rejected while an affected attempt is in progress. Submitted
    /// evidence remains protected; future runs omit the retired identity.
    async fn delete_and_regrade_assignment_item_impl(
        &self,
        context: TenantContext,
        command: DeleteAndRegradeAssignmentItemCommand,
    ) -> Result<StoredAssignment, StoreError>;

    /// Reads one assignment and its current revision for an authenticated edit.
    async fn get_assignment_for_edit_impl(
        &self,
        context: TenantContext,
        assignment: AssignmentId,
    ) -> Result<Option<StoredAssignment>, StoreError>;

    /// Reads one assignment inside the active tenant.
    async fn get_assignment_impl(
        &self,
        context: TenantContext,
        assignment: AssignmentId,
    ) -> Result<Option<AssignmentRecord>, StoreError>;

    /// Lists assignments inside the active tenant in stable cursor order.
    async fn list_assignments_impl(
        &self,
        context: TenantContext,
        course: CourseId,
        page: PageRequest,
    ) -> Result<Page<AssignmentRecord>, StoreError>;

    /// Reads one enrollment inside the active tenant.
    async fn get_enrollment_impl(
        &self,
        context: TenantContext,
        enrollment: EnrollmentId,
    ) -> Result<Option<AssignmentEnrollment>, StoreError>;
}

/// Focused persistence capability composed by [`Store`].
#[async_trait]
pub trait EffectivePolicyStore: Send + Sync {
    /// Reads the assignment-owned M1 base policy and its shared revision.
    async fn get_base_assignment_policy_impl(
        &self,
        context: TenantContext,
        assignment: AssignmentId,
    ) -> Result<Option<StoredBaseAssignmentPolicy>, StoreError>;

    async fn put_base_assignment_policy_impl(
        &self,
        context: TenantContext,
        command: PutBaseAssignmentPolicyCommand,
    ) -> Result<StoredBaseAssignmentPolicy, StoreError>;

    async fn put_group_schedule_offset_impl(
        &self,
        context: TenantContext,
        command: PutGroupScheduleOffsetCommand,
    ) -> Result<AssignmentRevision, StoreError>;

    async fn delete_group_schedule_offset_impl(
        &self,
        context: TenantContext,
        command: DeleteGroupScheduleOffsetCommand,
    ) -> Result<AssignmentRevision, StoreError>;

    async fn put_group_accommodation_impl(
        &self,
        context: TenantContext,
        command: PutGroupAccommodationCommand,
    ) -> Result<AssignmentRevision, StoreError>;

    async fn delete_group_accommodation_impl(
        &self,
        context: TenantContext,
        command: DeleteGroupAccommodationCommand,
    ) -> Result<AssignmentRevision, StoreError>;

    async fn put_individual_policy_exception_impl(
        &self,
        context: TenantContext,
        command: PutIndividualPolicyExceptionCommand,
    ) -> Result<AssignmentRevision, StoreError>;

    async fn delete_individual_policy_exception_impl(
        &self,
        context: TenantContext,
        command: DeleteIndividualPolicyExceptionCommand,
    ) -> Result<AssignmentRevision, StoreError>;

    /// Resolves M1--M4 from an S5 decision; implementations may not derive
    /// group applicability, membership, or audience outside that decision.
    async fn resolve_effective_policy_impl(
        &self,
        context: TenantContext,
        command: ResolveEffectivePolicyCommand,
    ) -> Result<Option<EffectivePolicyResolution>, StoreError>;

    /// Reads the immutable effective-policy receipt for one issued attempt.
    async fn get_issued_effective_policy_receipt_impl(
        &self,
        context: TenantContext,
        attempt: QuestionAttemptId,
    ) -> Result<Option<IssuedEffectivePolicyReceipt>, StoreError>;
}

/// Focused persistence capability composed by [`Store`].
#[async_trait]
pub trait RunStore: Send + Sync {
    /// Browser learner capability for immutable items of a currently active
    /// enrollment. Historical instructor inspection uses a separate capability.
    async fn learner_assignment_run_items_impl(
        &self,
        context: TenantContext,
        actor: UserId,
        run: RunId,
    ) -> Result<Option<Vec<AssignmentRunItem>>, StoreError>;
    /// Starts the next run or returns the enrollment's existing active run.
    ///
    /// The backend owns the timestamp, one-based run number, mode, policy,
    /// and compact-summary transition. The proposed ID is used only when a new
    /// run is actually inserted.
    async fn start_or_resume_run_impl(
        &self,
        context: TenantContext,
        actor: UserId,
        assignment: AssignmentId,
        proposed_run: RunId,
    ) -> Result<AssignmentRun, StoreError>;

    /// Reads the immutable selected questions and issued order frozen at run start.
    async fn assignment_run_items_impl(
        &self,
        context: TenantContext,
        run: RunId,
    ) -> Result<Vec<AssignmentRunItem>, StoreError>;

    /// Issues a fresh question or returns the run's unresolved instance.
    ///
    /// Storage supplies the authoritative issue time and deadline and permits
    /// at most one unresolved question in a run.
    async fn issue_or_resume_question_attempt_impl(
        &self,
        context: TenantContext,
        command: IssueQuestionAttemptCommand,
    ) -> Result<QuestionAttempt, StoreError>;

    /// Reads the server-only presentation binding for one owned attempt.
    async fn get_attempt_presentation_binding_impl(
        &self,
        context: TenantContext,
        actor: UserId,
        attempt: QuestionAttemptId,
    ) -> Result<Option<PresentationBindingV1>, StoreError>;

    /// Reads the answer-free snapshot frozen when one owned attempt issued its
    /// presentation. A presentation-bearing attempt without this snapshot is
    /// unavailable authority, never a request to rebuild from current state.
    async fn get_attempt_presentation_snapshot_impl(
        &self,
        context: TenantContext,
        actor: UserId,
        attempt: QuestionAttemptId,
    ) -> Result<Option<ReceiptPresentationSnapshot>, StoreError>;

    /// Reads the exact server-only answer-free envelope frozen with one owned
    /// presentation-bearing attempt. Its durable response IDs are used only
    /// for first-submit validation and private grading, never browser output.
    async fn get_attempt_grading_envelope_impl(
        &self,
        context: TenantContext,
        actor: UserId,
        attempt: QuestionAttemptId,
    ) -> Result<Option<question_model::QuestionEnvelope>, StoreError>;

    /// Reads immutable private flat-question grading authority for one owned
    /// attempt. A missing or corrupt required contract is unavailable rather
    /// than an invitation to reread mutable publication state.
    async fn get_attempt_flat_grading_impl(
        &self,
        context: TenantContext,
        actor: UserId,
        attempt: QuestionAttemptId,
    ) -> Result<Option<crate::IssuedFlatGradingContract>, StoreError>;

    /// Reads immutable server-only WebWork grading authority for one owned
    /// attempt. A missing required contract fails closed instead of asking the
    /// current catalog or renderer to reconstruct it.
    async fn get_attempt_webwork_grading_impl(
        &self,
        context: TenantContext,
        actor: UserId,
        attempt: QuestionAttemptId,
    ) -> Result<Option<crate::IssuedWebworkGradingContract>, StoreError>;

    /// Reads the private answer-free WeBWorK replay state for one owned attempt.
    async fn get_webwork_grade_replay_state_impl(
        &self,
        context: TenantContext,
        actor: UserId,
        attempt: QuestionAttemptId,
    ) -> Result<Option<WebworkGradeReplayStateV1>, StoreError>;

    /// Reserves a key-free future variant for an owned unresolved predecessor.
    /// This operation never creates a question attempt or starts a timer.
    async fn reserve_or_resume_prefetched_question_impl(
        &self,
        context: TenantContext,
        command: ReservePrefetchedQuestionCommand,
    ) -> Result<PrefetchedQuestion, StoreError>;

    /// Finds a reservation selected by trusted server sequencing. Promotion
    /// remains atomic in `issue_or_resume_question_attempt`.
    async fn get_prefetched_question_impl(
        &self,
        context: TenantContext,
        actor: UserId,
        run: RunId,
        predecessor: QuestionAttemptId,
        assignment_position: u32,
    ) -> Result<Option<PrefetchedQuestion>, StoreError>;

    /// Browser learner capability for a reservation in an active enrollment.
    async fn learner_get_prefetched_question_impl(
        &self,
        context: TenantContext,
        actor: UserId,
        run: RunId,
        predecessor: QuestionAttemptId,
        assignment_position: u32,
    ) -> Result<Option<PrefetchedQuestion>, StoreError>;

    /// Reads the immutable next-attempt result for an owned submission.
    async fn submission_next_attempt_impl(
        &self,
        context: TenantContext,
        actor: UserId,
        predecessor: QuestionAttemptId,
    ) -> Result<SubmissionNextAttempt, StoreError>;

    /// Returns the sole owned committed submission in a run whose successor
    /// receipt has not yet been finalized. Ambiguity is a conflict, never a
    /// route-level guess.
    async fn pending_submission_for_run_impl(
        &self,
        context: TenantContext,
        actor: UserId,
        run: RunId,
    ) -> Result<Option<QuestionAttemptId>, StoreError>;

    /// Browser learner capability for an active enrollment's recovery state.
    async fn learner_pending_submission_for_run_impl(
        &self,
        context: TenantContext,
        actor: UserId,
        run: RunId,
    ) -> Result<Option<QuestionAttemptId>, StoreError>;

    /// Finalizes a no-successor receipt after the server has checked current
    /// run state. Repeating the exact decision is safe; a different decision
    /// conflicts rather than rewriting an earlier receipt.
    async fn finalize_submission_next_attempt_impl(
        &self,
        context: TenantContext,
        actor: UserId,
        predecessor: QuestionAttemptId,
        next: Option<QuestionAttemptId>,
    ) -> Result<(), StoreError>;

    /// Lists browser-safe attempt projections for one run in stable cursor order.
    async fn list_question_attempts_impl(
        &self,
        context: TenantContext,
        run: RunId,
        page: PageRequest,
    ) -> Result<Page<QuestionAttempt>, StoreError>;

    /// Browser learner capability for attempts in an active enrollment.
    async fn learner_list_question_attempts_impl(
        &self,
        context: TenantContext,
        actor: UserId,
        run: RunId,
        page: PageRequest,
    ) -> Result<Option<Page<QuestionAttempt>>, StoreError>;

    /// Returns a prior exact submission before invoking a grading backend again.
    ///
    /// A changed response or key for an already submitted attempt is a conflict.
    async fn replay_submission_impl(
        &self,
        context: TenantContext,
        actor: UserId,
        attempt: QuestionAttemptId,
        response: &StudentResponse,
        idempotency_key: &SubmissionIdempotencyKey,
    ) -> Result<Option<SubmissionRecord>, StoreError>;

    /// Reads the immutable receipt for one owned submitted attempt.
    ///
    /// This does not accept an idempotency key because it is a receipt read,
    /// not a retry authorization. Implementations must return the persisted
    /// receipt only and fail closed when a required receipt payload is absent
    /// or corrupt; they must not reconstruct it from current catalog state.
    async fn submission_record_impl(
        &self,
        context: TenantContext,
        actor: UserId,
        attempt: QuestionAttemptId,
    ) -> Result<Option<SubmissionRecord>, StoreError>;

    /// Atomically records the first response, grade event, run completion, and summary.
    ///
    /// The backend supplies the submission timestamp and applies the authored
    /// timing policy. Exact retries return the first committed record.
    async fn submit_question_attempt_impl(
        &self,
        context: TenantContext,
        command: SubmitQuestionAttemptCommand,
    ) -> Result<SubmissionRecord, StoreError>;

    /// Closes an active question without inventing a response or score.
    ///
    /// Only a persisted direct course instructor may perform this action.
    /// The attempt becomes `needs_manual_grading` and exact action retries
    /// return the original minimal audit record.
    async fn force_submit_attempt_impl(
        &self,
        context: TenantContext,
        command: ForceSubmitAttemptCommand,
    ) -> Result<AttemptSupportRecord, StoreError>;

    /// Excludes an attempt from current scoring while retaining raw evidence.
    ///
    /// A submitted evaluation triggers generation-fenced assignment
    /// recalculation; exact action retries never enqueue a duplicate job.
    async fn clear_attempt_impl(
        &self,
        context: TenantContext,
        command: ClearAttemptCommand,
    ) -> Result<AttemptSupportRecord, StoreError>;
}

/// Focused persistence capability composed by [`Store`].
#[async_trait]
pub trait FeedbackStore: Send + Sync {
    /// Atomically records an authorized instructor release of an existing
    /// first-grade feedback record. The original receipt is never rewritten.
    async fn release_attempt_feedback_impl(
        &self,
        context: TenantContext,
        command: ReleaseAttemptFeedbackCommand,
    ) -> Result<FeedbackReleaseRecord, StoreError>;

    /// Reads the current release state for one attempt after proving the actor
    /// owns that educational record or directly instructs its course.
    async fn get_attempt_feedback_release_impl(
        &self,
        context: TenantContext,
        actor: UserId,
        attempt: QuestionAttemptId,
    ) -> Result<Option<FeedbackReleaseRecord>, StoreError>;

    /// Reads the bounded, private material for one run-summary projection.
    ///
    /// The actor must own the enrollment or directly instruct its course. A
    /// failed authorization is deliberately indistinguishable from absence.
    /// Implementations use a stable `(assignment_position, attempt_id)` cursor
    /// and never consult question envelopes or re-run an adapter.
    async fn get_run_summary_page_impl(
        &self,
        context: TenantContext,
        actor: UserId,
        run: RunId,
        page: PageRequest,
    ) -> Result<RunSummaryPageInput, StoreError>;
}

/// Focused persistence capability composed by [`Store`].
#[async_trait]
pub trait ActivityStore: Send + Sync {
    /// Reads retained history for a current direct course instructor. This is
    /// intentionally distinct from the active-learner browser capability.
    async fn instructor_get_enrollment_impl(
        &self,
        context: TenantContext,
        actor: UserId,
        enrollment: EnrollmentId,
    ) -> Result<Option<AssignmentEnrollment>, StoreError>;
    /// Reads one enrollment only for its currently active learner owner.
    async fn learner_get_enrollment_impl(
        &self,
        context: TenantContext,
        actor: UserId,
        enrollment: EnrollmentId,
    ) -> Result<Option<AssignmentEnrollment>, StoreError>;
    /// Reads one enrollment for a current learner and assignment by active
    /// membership identity, including course visibility gates.
    async fn learner_get_enrollment_for_assignment_impl(
        &self,
        context: TenantContext,
        actor: UserId,
        assignment: AssignmentId,
    ) -> Result<Option<AssignmentEnrollment>, StoreError>;
    /// Reads one run only for its currently active learner owner. This is a
    /// distinct browser capability; historical instructor reads use separate APIs.
    async fn learner_get_run_impl(
        &self,
        context: TenantContext,
        actor: UserId,
        run: RunId,
    ) -> Result<Option<AssignmentRun>, StoreError>;
    /// Atomically writes activity and its compact summary projection.
    async fn apply_activity_transition_impl(
        &self,
        context: TenantContext,
        transition: ActivityTransition,
    ) -> Result<StudentAssignmentSummary, StoreError>;

    /// Reads one run inside the active tenant.
    async fn get_run_impl(
        &self,
        context: TenantContext,
        run: RunId,
    ) -> Result<Option<AssignmentRun>, StoreError>;

    /// Lists runs for one enrollment in stable cursor order.
    async fn list_runs_impl(
        &self,
        context: TenantContext,
        enrollment: EnrollmentId,
        page: PageRequest,
    ) -> Result<Page<AssignmentRun>, StoreError>;

    /// Lists retained runs only for a current direct course instructor.
    async fn instructor_list_runs_impl(
        &self,
        context: TenantContext,
        actor: UserId,
        enrollment: EnrollmentId,
        page: PageRequest,
    ) -> Result<Option<Page<AssignmentRun>>, StoreError>;

    /// Browser learner capability for an active enrollment's run list.
    async fn learner_list_runs_impl(
        &self,
        context: TenantContext,
        actor: UserId,
        enrollment: EnrollmentId,
        page: PageRequest,
    ) -> Result<Option<Page<AssignmentRun>>, StoreError>;

    /// Reads one question attempt inside the active tenant.
    async fn get_question_attempt_impl(
        &self,
        context: TenantContext,
        attempt: QuestionAttemptId,
    ) -> Result<Option<QuestionAttempt>, StoreError>;

    /// Browser learner capability for an active enrollment's attempt.
    async fn learner_get_question_attempt_impl(
        &self,
        context: TenantContext,
        actor: UserId,
        attempt: QuestionAttemptId,
    ) -> Result<Option<QuestionAttempt>, StoreError>;

    /// Reads the transactionally maintained summary for one enrollment.
    async fn get_summary_impl(
        &self,
        context: TenantContext,
        enrollment: EnrollmentId,
    ) -> Result<Option<StudentAssignmentSummary>, StoreError>;

    /// Browser learner capability for an active enrollment's summary.
    async fn learner_get_summary_impl(
        &self,
        context: TenantContext,
        actor: UserId,
        enrollment: EnrollmentId,
    ) -> Result<Option<StudentAssignmentSummary>, StoreError>;
}

use super::*;

/// Persistence operations consumed by catalog, course, run, and worker lanes.
/// Complete persistence contract consumed by server and worker lanes.
#[async_trait]
pub trait Store:
    StatisticsStore
    + AssetStore
    + AuthoringStore
    + CourseStore
    + CourseAssignmentStore
    + EntitlementStore
    + EffectivePolicyStore
    + RunStore
    + FeedbackStore
    + ActivityStore
    + NavigationReferenceStore
    + AccountPresentationStore
{
    /// Lists the current learner-visible assignment definitions for one
    /// course.  This is non-mutating and uses the same entitlement evaluator
    /// as receipt materialization.
    async fn list_learner_entitled_assignments(
        &self,
        context: TenantContext,
        learner: UserId,
        course: CourseId,
        page: PageRequest,
    ) -> Result<Page<AssignmentRecord>, StoreError> {
        EntitlementStore::list_learner_entitled_assignments_impl(
            self, context, learner, course, page,
        )
        .await
    }

    /// Evaluates present learner authority without materializing a receipt.
    async fn evaluate_assignment_entitlement(
        &self,
        context: TenantContext,
        learner: UserId,
        course: CourseId,
        assignment: AssignmentId,
    ) -> Result<domain::entitlement::EntitlementDecision, StoreError> {
        EntitlementStore::evaluate_assignment_entitlement_impl(
            self, context, learner, course, assignment,
        )
        .await
    }

    /// Explicit instructor issue of a learner receipt. Learner start, attempt,
    /// submission, and replay must materialize only inside their owning action.
    async fn issue_assignment_entitlement(
        &self,
        context: TenantContext,
        command: MaterializeAssignmentEntitlementCommand,
    ) -> Result<AssignmentEntitlementMaterialization, StoreError> {
        EntitlementStore::issue_assignment_entitlement_impl(self, context, command).await
    }

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
    async fn create_course(
        &self,
        context: TenantContext,
        command: CreateCourseCommand,
    ) -> Result<(), StoreError> {
        CourseStore::create_course_impl(self, context, command).await
    }

    /// Delegates to the focused [`CourseStore`] capability.
    async fn get_course(
        &self,
        context: TenantContext,
        course: CourseId,
    ) -> Result<Option<CourseRecord>, StoreError> {
        CourseStore::get_course_impl(self, context, course).await
    }

    /// Delegates to the canonical current-membership authority query.
    async fn get_current_course_membership(
        &self,
        context: TenantContext,
        course: CourseId,
        user: UserId,
    ) -> Result<Option<CourseMembershipRecord>, StoreError> {
        CourseStore::get_current_course_membership_impl(self, context, course, user).await
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

    /// Atomically creates an assignment and its explicit base policy.
    async fn create_assignment(
        &self,
        context: TenantContext,
        command: CreateAssignmentCommand,
    ) -> Result<StoredAssignment, StoreError> {
        CourseAssignmentStore::create_assignment_impl(self, context, command).await
    }

    /// Atomically replaces content fields while preserving teaching settings.
    async fn replace_assignment(
        &self,
        context: TenantContext,
        command: ReplaceAssignmentCommand,
    ) -> Result<StoredAssignment, StoreError> {
        CourseAssignmentStore::replace_assignment_impl(self, context, command).await
    }

    /// Atomically replaces an unissued complete assignment definition.
    async fn replace_unissued_assignment_definition(
        &self,
        context: TenantContext,
        command: ReplaceUnissuedAssignmentDefinitionCommand,
    ) -> Result<ReplaceUnissuedAssignmentDefinitionOutcome, StoreError> {
        CourseAssignmentStore::replace_unissued_assignment_definition_impl(self, context, command)
            .await
    }

    /// Replaces one fixed assignment item through the focused
    /// revision-checked replacement capability.
    async fn replace_assignment_fixed_item(
        &self,
        context: TenantContext,
        command: ReplaceAssignmentFixedItemCommand,
    ) -> Result<StoredAssignment, StoreError> {
        CourseAssignmentStore::replace_assignment_fixed_item_impl(self, context, command).await
    }

    /// Inserts one fresh fixed item through the focused revision-checked
    /// pre-evidence editing capability.
    async fn add_assignment_fixed_item(
        &self,
        context: TenantContext,
        command: AddAssignmentFixedItemCommand,
    ) -> Result<StoredAssignment, StoreError> {
        CourseAssignmentStore::add_assignment_fixed_item_impl(self, context, command).await
    }

    /// Removes one fixed item through the focused revision-checked
    /// pre-evidence editing capability.
    async fn remove_assignment_fixed_item(
        &self,
        context: TenantContext,
        command: RemoveAssignmentFixedItemCommand,
    ) -> Result<StoredAssignment, StoreError> {
        CourseAssignmentStore::remove_assignment_fixed_item_impl(self, context, command).await
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
    async fn get_enrollment(
        &self,
        context: TenantContext,
        enrollment: EnrollmentId,
    ) -> Result<Option<AssignmentEnrollment>, StoreError> {
        CourseAssignmentStore::get_enrollment_impl(self, context, enrollment).await
    }

    /// Browser learner capability; storage proves current active membership.
    async fn learner_get_enrollment(
        &self,
        context: TenantContext,
        actor: UserId,
        enrollment: EnrollmentId,
    ) -> Result<Option<AssignmentEnrollment>, StoreError> {
        ActivityStore::learner_get_enrollment_impl(self, context, actor, enrollment).await
    }

    /// Resolves one active learner enrollment for an assignment by assignment
    /// identity, including course visibility and active-membership checks.
    async fn learner_get_enrollment_for_assignment(
        &self,
        context: TenantContext,
        actor: UserId,
        assignment: AssignmentId,
    ) -> Result<Option<AssignmentEnrollment>, StoreError> {
        ActivityStore::learner_get_enrollment_for_assignment_impl(self, context, actor, assignment)
            .await
    }

    /// Historical instructor capability; the Store rechecks direct course
    /// membership instead of accepting a coarse platform role.
    async fn instructor_get_enrollment(
        &self,
        context: TenantContext,
        actor: UserId,
        enrollment: EnrollmentId,
    ) -> Result<Option<AssignmentEnrollment>, StoreError> {
        ActivityStore::instructor_get_enrollment_impl(self, context, actor, enrollment).await
    }

    /// Delegates to the focused [`EffectivePolicyStore`] capability.
    async fn get_base_assignment_policy(
        &self,
        context: TenantContext,
        assignment: AssignmentId,
    ) -> Result<Option<StoredBaseAssignmentPolicy>, StoreError> {
        EffectivePolicyStore::get_base_assignment_policy_impl(self, context, assignment).await
    }

    /// Delegates to the focused [`EffectivePolicyStore`] capability.
    async fn put_assignment_teaching_settings(
        &self,
        context: TenantContext,
        command: PutAssignmentTeachingSettingsCommand,
    ) -> Result<StoredBaseAssignmentPolicy, StoreError> {
        EffectivePolicyStore::put_assignment_teaching_settings_impl(self, context, command).await
    }

    /// Delegates to the focused [`EffectivePolicyStore`] capability.
    async fn put_group_schedule_offset(
        &self,
        context: TenantContext,
        command: PutGroupScheduleOffsetCommand,
    ) -> Result<AssignmentRevision, StoreError> {
        EffectivePolicyStore::put_group_schedule_offset_impl(self, context, command).await
    }

    /// Delegates to the focused [`EffectivePolicyStore`] capability.
    async fn delete_group_schedule_offset(
        &self,
        context: TenantContext,
        command: DeleteGroupScheduleOffsetCommand,
    ) -> Result<AssignmentRevision, StoreError> {
        EffectivePolicyStore::delete_group_schedule_offset_impl(self, context, command).await
    }

    /// Delegates to the focused [`EffectivePolicyStore`] capability.
    async fn put_group_accommodation(
        &self,
        context: TenantContext,
        command: PutGroupAccommodationCommand,
    ) -> Result<AssignmentRevision, StoreError> {
        EffectivePolicyStore::put_group_accommodation_impl(self, context, command).await
    }

    /// Delegates to the focused [`EffectivePolicyStore`] capability.
    async fn delete_group_accommodation(
        &self,
        context: TenantContext,
        command: DeleteGroupAccommodationCommand,
    ) -> Result<AssignmentRevision, StoreError> {
        EffectivePolicyStore::delete_group_accommodation_impl(self, context, command).await
    }

    /// Delegates to the focused [`EffectivePolicyStore`] capability.
    async fn put_individual_policy_exception(
        &self,
        context: TenantContext,
        command: PutIndividualPolicyExceptionCommand,
    ) -> Result<AssignmentRevision, StoreError> {
        EffectivePolicyStore::put_individual_policy_exception_impl(self, context, command).await
    }

    /// Delegates to the focused [`EffectivePolicyStore`] capability.
    async fn delete_individual_policy_exception(
        &self,
        context: TenantContext,
        command: DeleteIndividualPolicyExceptionCommand,
    ) -> Result<AssignmentRevision, StoreError> {
        EffectivePolicyStore::delete_individual_policy_exception_impl(self, context, command).await
    }

    /// Delegates to the focused [`EffectivePolicyStore`] capability.
    async fn resolve_effective_policy(
        &self,
        context: TenantContext,
        command: ResolveEffectivePolicyCommand,
    ) -> Result<Option<EffectivePolicyResolution>, StoreError> {
        EffectivePolicyStore::resolve_effective_policy_impl(self, context, command).await
    }

    /// Delegates to the focused [`EffectivePolicyStore`] capability.
    async fn get_issued_effective_policy_receipt(
        &self,
        context: TenantContext,
        attempt: QuestionAttemptId,
    ) -> Result<Option<IssuedEffectivePolicyReceipt>, StoreError> {
        EffectivePolicyStore::get_issued_effective_policy_receipt_impl(self, context, attempt).await
    }

    /// Delegates to the focused [`RunStore`] capability.
    ///
    /// `binding` is a routing assertion verified by the Store, not an
    /// authority or authorization grant.
    async fn start_or_resume_run(
        &self,
        context: TenantContext,
        actor: UserId,
        binding: LearnerWorkRoutingBinding,
        proposed_run: RunId,
    ) -> Result<AssignmentRun, StoreError> {
        RunStore::start_or_resume_run_impl(self, context, actor, binding, proposed_run).await
    }

    /// Delegates to the focused [`RunStore`] capability.
    async fn assignment_run_items(
        &self,
        context: TenantContext,
        run: RunId,
    ) -> Result<Vec<AssignmentRunItem>, StoreError> {
        RunStore::assignment_run_items_impl(self, context, run).await
    }

    async fn learner_assignment_run_items(
        &self,
        context: TenantContext,
        actor: UserId,
        run: RunId,
    ) -> Result<Option<Vec<AssignmentRunItem>>, StoreError> {
        RunStore::learner_assignment_run_items_impl(self, context, actor, run).await
    }

    /// Delegates to the focused [`RunStore`] capability.
    async fn issue_or_resume_question_attempt(
        &self,
        context: TenantContext,
        command: IssueQuestionAttemptCommand,
    ) -> Result<QuestionAttempt, StoreError> {
        RunStore::issue_or_resume_question_attempt_impl(self, context, command).await
    }

    /// Reads one coherent server-only issued evidence aggregate through the
    /// explicit learner-work route binding.
    async fn read_issued_attempt_evidence(
        &self,
        context: TenantContext,
        actor: UserId,
        binding: LearnerWorkRoutingBinding,
        attempt: QuestionAttemptId,
    ) -> Result<IssuedAttemptRead, StoreError> {
        RunStore::read_issued_attempt_evidence_impl(self, context, actor, binding, attempt).await
    }

    /// Prepares one exact submission without holding storage locks across grading.
    async fn prepare_question_submission(
        &self,
        context: TenantContext,
        actor: UserId,
        binding: LearnerWorkRoutingBinding,
        attempt: QuestionAttemptId,
        response: &StudentResponse,
        idempotency_key: &SubmissionIdempotencyKey,
    ) -> Result<SubmissionPreparation, StoreError> {
        RunStore::prepare_question_submission_impl(
            self,
            context,
            actor,
            binding,
            attempt,
            response,
            idempotency_key,
        )
        .await
    }

    /// Delegates to the focused [`RunStore`] capability.
    async fn reserve_or_resume_prefetched_question(
        &self,
        context: TenantContext,
        command: ReservePrefetchedQuestionCommand,
    ) -> Result<PrefetchedQuestionDescriptorV1, StoreError> {
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
    ) -> Result<Option<PrefetchedQuestionDescriptorV1>, StoreError> {
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

    async fn learner_get_prefetched_question(
        &self,
        context: TenantContext,
        actor: UserId,
        run: RunId,
        predecessor: QuestionAttemptId,
        assignment_position: u32,
    ) -> Result<Option<PrefetchedQuestionDescriptorV1>, StoreError> {
        RunStore::learner_get_prefetched_question_impl(
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
        binding: LearnerWorkRoutingBinding,
        predecessor: QuestionAttemptId,
    ) -> Result<SubmissionNextAttempt, StoreError> {
        RunStore::submission_next_attempt_impl(self, context, actor, binding, predecessor).await
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

    async fn learner_pending_submission_for_run(
        &self,
        context: TenantContext,
        actor: UserId,
        run: RunId,
    ) -> Result<Option<QuestionAttemptId>, StoreError> {
        RunStore::learner_pending_submission_for_run_impl(self, context, actor, run).await
    }

    /// Browser learner capability for the current enrollment's attempt list.
    async fn learner_list_question_attempts(
        &self,
        context: TenantContext,
        actor: UserId,
        run: RunId,
        page: PageRequest,
    ) -> Result<Option<Page<QuestionAttempt>>, StoreError> {
        RunStore::learner_list_question_attempts_impl(self, context, actor, run, page).await
    }

    /// Delegates to the focused [`RunStore`] capability.
    async fn finalize_submission_next_attempt(
        &self,
        context: TenantContext,
        actor: UserId,
        binding: LearnerWorkRoutingBinding,
        predecessor: QuestionAttemptId,
        next: Option<QuestionAttemptId>,
    ) -> Result<(), StoreError> {
        RunStore::finalize_submission_next_attempt_impl(
            self,
            context,
            actor,
            binding,
            predecessor,
            next,
        )
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

    /// Reads an owned immutable submission receipt without retry credentials.
    ///
    /// A missing receipt means the attempt has not submitted. A corrupt or
    /// incomplete receipt is unavailable authority, never a request to rebuild
    /// from current catalog state.
    async fn submission_record(
        &self,
        context: TenantContext,
        actor: UserId,
        attempt: QuestionAttemptId,
    ) -> Result<Option<SubmissionRecord>, StoreError> {
        RunStore::submission_record_impl(self, context, actor, attempt).await
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

    /// Browser learner capability; checks current membership in storage.
    async fn learner_get_run(
        &self,
        context: TenantContext,
        actor: UserId,
        run: RunId,
    ) -> Result<Option<AssignmentRun>, StoreError> {
        ActivityStore::learner_get_run_impl(self, context, actor, run).await
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

    /// Historical instructor capability; storage rechecks direct course membership.
    async fn instructor_list_runs(
        &self,
        context: TenantContext,
        actor: UserId,
        enrollment: EnrollmentId,
        page: PageRequest,
    ) -> Result<Option<Page<AssignmentRun>>, StoreError> {
        ActivityStore::instructor_list_runs_impl(self, context, actor, enrollment, page).await
    }

    async fn learner_list_runs(
        &self,
        context: TenantContext,
        actor: UserId,
        enrollment: EnrollmentId,
        page: PageRequest,
    ) -> Result<Option<Page<AssignmentRun>>, StoreError> {
        ActivityStore::learner_list_runs_impl(self, context, actor, enrollment, page).await
    }

    /// Delegates to the focused [`ActivityStore`] capability.
    async fn get_question_attempt(
        &self,
        context: TenantContext,
        attempt: QuestionAttemptId,
    ) -> Result<Option<QuestionAttempt>, StoreError> {
        ActivityStore::get_question_attempt_impl(self, context, attempt).await
    }

    async fn learner_get_question_attempt(
        &self,
        context: TenantContext,
        actor: UserId,
        attempt: QuestionAttemptId,
    ) -> Result<Option<QuestionAttempt>, StoreError> {
        ActivityStore::learner_get_question_attempt_impl(self, context, actor, attempt).await
    }

    /// Delegates to the focused [`ActivityStore`] capability.
    async fn get_summary(
        &self,
        context: TenantContext,
        enrollment: EnrollmentId,
    ) -> Result<Option<StudentAssignmentSummary>, StoreError> {
        ActivityStore::get_summary_impl(self, context, enrollment).await
    }

    async fn learner_get_summary(
        &self,
        context: TenantContext,
        actor: UserId,
        enrollment: EnrollmentId,
    ) -> Result<Option<LearnerAssignmentSummarySnapshot>, StoreError> {
        ActivityStore::learner_get_summary_impl(self, context, actor, enrollment).await
    }
}

#[async_trait]
impl<T> Store for T where
    T: StatisticsStore
        + AssetStore
        + AuthoringStore
        + CourseStore
        + CourseAssignmentStore
        + EntitlementStore
        + EffectivePolicyStore
        + RunStore
        + FeedbackStore
        + ActivityStore
        + NavigationReferenceStore
        + AccountPresentationStore
{
}

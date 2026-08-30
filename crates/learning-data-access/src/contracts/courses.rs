use super::*;

mod selection;
pub(crate) use selection::{select_assignment_group_candidates, select_assignment_run_items};

/// Course aggregate. Direct access lives exclusively in
/// [`CourseMembershipRecord`], never in this aggregate.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CourseRecord {
    /// Durable course identity.
    pub id: CourseId,
    /// Human-facing course or section title.
    pub title: String,
    /// Required inclusive term bounds and authoritative scheduling zone.
    pub term: question_model::CourseTerm,
}

/// Atomic course-provisioning command.
///
/// A course has no usable orphan state: the first instructor membership is
/// created in the same transaction. Later roster changes use their dedicated
/// lifecycle commands and may not replace this relationship wholesale.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CreateCourseCommand {
    pub course: CourseRecord,
    /// Closed server-owned authority for the initial direct Instructor.
    pub authority: CourseCreationAuthority,
}

/// Authority accepted by atomic course provisioning.
///
/// Browser and ordinary application DTOs never carry this value. The server
/// derives these variants from its authenticated request. Base Course
/// installation uses a separate private, non-Clone client and command.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CourseCreationAuthority {
    /// A currently approved instructor with the exact authenticated session.
    ApprovedInstructor {
        actor: UserId,
        session: SessionTokenHash,
    },
    /// A currently authenticated platform Sysadmin.
    Sysadmin {
        actor: UserId,
        session: SessionTokenHash,
    },
}

/// Intended initial direct-Instructor membership for a future course creation.
///
/// This is the SD1 target contract, not accepted runtime authority. The
/// request boundary supplies the authenticated `ActorContext` separately; the
/// later Store transaction verifies that actor for the selected mode and
/// verifies the target's current Instructor approval before creating exactly
/// this ordinary direct membership.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CourseCreationIntent {
    /// An approved Instructor creates a course for their own direct membership.
    DirectApprovedInstructor { initial_instructor: UserId },
    /// A Sysadmin provisions a course for an explicitly approved Instructor.
    ///
    /// The Sysadmin is an audit and authorization actor only; this intent
    /// grants the initial membership exclusively to `initial_instructor`.
    SysadminOnBehalfOfApprovedInstructor { initial_instructor: UserId },
}

impl CourseCreationIntent {
    /// Returns the sole account that receives the first direct membership.
    pub const fn initial_instructor(self) -> UserId {
        match self {
            Self::DirectApprovedInstructor { initial_instructor }
            | Self::SysadminOnBehalfOfApprovedInstructor { initial_instructor } => {
                initial_instructor
            }
        }
    }
}

/// Exact course-and-actor input for a future Store authorization lookup.
///
/// This names a lookup target rather than an authorization result. A Store
/// transaction resolves current approval and direct membership before it
/// permits the requested operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CourseInstructorAuthorizationScope {
    pub course: CourseId,
    pub actor: UserId,
}

/// Exact course, actor, and Student-record input for a future Store ownership
/// lookup.
///
/// The `student` identity stays distinct from the account identity. The Store
/// resolves this active Student membership episode before it permits access.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StudentCourseRecordAuthorizationScope {
    pub course: CourseId,
    pub actor: UserId,
    pub membership: question_model::CourseMembershipId,
    pub student: StudentId,
}

/// Canonical course access relationship.  Each reinvitation creates a new
/// episode; a revoked row remains immutable evidence for receipts created
/// during that episode.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CourseMembershipRecord {
    pub id: question_model::CourseMembershipId,
    pub course: CourseId,
    pub user: UserId,
    /// Learner identity for student episodes; instructors deliberately have
    /// no synthetic learner identity.
    pub student: Option<StudentId>,
    pub role: CourseMembershipRole,
    /// Active course-local export key. This is membership authority, not
    /// presentation profile data, and becomes reusable after revocation.
    pub roster_id: Option<crate::CourseRosterId>,
    pub status: crate::CourseMemberStatus,
    pub joined_at: ActivityTimestamp,
    pub revoked_at: Option<ActivityTimestamp>,
}

impl CourseRecord {
    /// Returns the browser projection for one direct course member.
    pub fn summary(
        &self,
        role: CourseMembershipRole,
        reference: question_model::CourseReference,
    ) -> CourseSummary {
        CourseSummary {
            id: self.id,
            reference,
            title: self.title.clone(),
            term: self.term.clone(),
            role,
        }
    }
}

/// Explicit scope for course-list authorization.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CourseListScope {
    /// Return only courses carrying a direct membership for this user.
    Member(UserId),
}

/// Course-owned assignment that references shared immutable content.
#[derive(Debug, Clone, PartialEq)]
pub struct AssignmentRecord {
    /// Durable assignment identity.
    pub id: AssignmentId,
    /// Course containing the assignment.
    pub course_id: CourseId,
    /// Human-facing assignment title.
    pub title: String,
    /// Instructor-controlled delivery state. Draft is never learner-visible.
    pub lifecycle: question_model::AssignmentLifecycle,
    /// Learner-facing instructions owned by the assignment policy slice.
    pub instructions: question_model::AssignmentInstructions,
    /// Explicit current audience.  Course-wide and group-scoped delivery are
    /// different contracts; absence is not a compatible default.
    pub audience: question_model::AssignmentAudience,
    /// Stable ordered fixed items selected for the assignment.
    pub items: Vec<question_model::AssignmentItem>,
    /// Random-selection groups with pinned immutable candidates.
    pub selection_groups: Vec<question_model::AssignmentSelectionGroup>,
    /// Assignment-owned learner-facing disclosure schedule, evaluated only by
    /// the server-side learner projection path.
    pub disclosure_policy: question_model::StudentDisclosurePolicy,
    /// Four independent run policies.
    pub policies: RunPolicies,
}

/// Editable assignment definition together with its server-managed revision.
#[derive(Debug, Clone, PartialEq)]
pub struct StoredAssignment {
    pub record: AssignmentRecord,
    pub revision: AssignmentRevision,
    /// The assignment-owned base policy held under the same revision.
    pub base_policy: question_model::BaseAssignmentPolicy,
    /// Generation matched by current computed score rows.
    pub scoring_generation: ScoringGeneration,
    /// Whether scores for this generation may be presented.
    pub scoring_status: ScoringStatus,
}

/// Complete server-owned defaults for a newly persisted assignment draft.
///
/// The browser supplies only a title. This value makes the remaining initial
/// aggregate explicit and shared by every Store implementation.
#[derive(Debug, Clone, PartialEq)]
pub struct NewAssignmentDraft {
    pub record: AssignmentRecord,
    pub base_policy: question_model::BaseAssignmentPolicy,
    pub scoring_generation: ScoringGeneration,
    pub scoring_status: ScoringStatus,
}

/// Creates the one authoritative incomplete Draft aggregate.
pub fn new_assignment_draft(
    course: CourseId,
    assignment: AssignmentId,
    title: String,
) -> NewAssignmentDraft {
    NewAssignmentDraft {
        record: AssignmentRecord {
            id: assignment,
            course_id: course,
            title,
            lifecycle: question_model::AssignmentLifecycle::Draft,
            instructions: question_model::AssignmentInstructions::default(),
            audience: question_model::AssignmentAudience::CourseWide,
            items: Vec::new(),
            selection_groups: Vec::new(),
            disclosure_policy: question_model::StudentDisclosurePolicy::default(),
            policies: RunPolicies {
                completion: question_model::CompletionRequirement::AnswerAll,
                grade: question_model::GradePolicy::Highest,
                continued_practice: question_model::ContinuedPractice::Unlimited,
                variation: question_model::VariationPolicy::NewSeeds,
            },
        },
        base_policy: question_model::BaseAssignmentPolicy::default(),
        scoring_generation: ScoringGeneration::INITIAL,
        scoring_status: ScoringStatus::Current,
    }
}

/// Questions-owned fields for the current definition slice. Draft and
/// Archived assignments may persist this slice without entries.
#[derive(Debug, Clone, PartialEq)]
pub struct AssignmentContentUpdate {
    pub title: String,
    pub items: Vec<question_model::AssignmentItem>,
    pub selection_groups: Vec<question_model::AssignmentSelectionGroup>,
}

/// Policies-owned fields, including the resolved absolute teaching settings.
#[derive(Debug, Clone, PartialEq)]
pub struct AssignmentPoliciesUpdate {
    pub audience: question_model::AssignmentAudience,
    pub disclosure_policy: question_model::StudentDisclosurePolicy,
    pub policies: RunPolicies,
    pub teaching_settings: question_model::AssignmentTeachingSettings,
}

/// Editable assignment fields supplied after the server has bound identity and
/// course ownership from the authenticated route.
#[derive(Debug, Clone, PartialEq)]
pub struct AssignmentUpdate {
    pub title: String,
    pub audience: question_model::AssignmentAudience,
    pub items: Vec<question_model::AssignmentItem>,
    pub selection_groups: Vec<question_model::AssignmentSelectionGroup>,
    pub disclosure_policy: question_model::StudentDisclosurePolicy,
    pub policies: RunPolicies,
}

/// Authenticated instructor command that creates an assignment definition and
/// its base policy in the same persistence transaction.
///
/// Authority deliberately belongs to the command rather than a caller-supplied
/// scope: the Store derives its authorization from the actor and exact course
/// relationship.
#[derive(Debug, Clone, PartialEq)]
pub struct CreateAssignmentCommand {
    pub actor: UserId,
    pub assignment: AssignmentRecord,
    pub base_policy: question_model::BaseAssignmentPolicy,
}

/// Authenticated instructor command that creates a persisted incomplete Draft.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CreateAssignmentDraftCommand {
    pub actor: UserId,
    pub course: CourseId,
    pub assignment: AssignmentId,
    pub title: String,
}

/// Revision-checked update of exactly the Questions-owned assignment slice.
#[derive(Debug, Clone, PartialEq)]
pub struct ReplaceAssignmentContentCommand {
    pub actor: UserId,
    pub course: CourseId,
    pub assignment: AssignmentId,
    pub expected_revision: AssignmentRevision,
    pub update: AssignmentContentUpdate,
}

/// Result of a complete Questions-page save.
#[derive(Debug, Clone, PartialEq)]
pub enum ReplaceAssignmentContentOutcome {
    /// The content slice replaced one aggregate revision.
    Replaced(Box<StoredAssignment>),
    /// The expected aggregate revision no longer matches current state.
    RevisionConflict,
    /// Learner evidence was issued before this structural replacement committed.
    Issued,
}

/// Revision-checked update of exactly the Policies-owned assignment slice.
#[derive(Debug, Clone, PartialEq)]
pub struct ReplaceAssignmentPoliciesCommand {
    pub actor: UserId,
    pub course: CourseId,
    pub assignment: AssignmentId,
    pub expected_revision: AssignmentRevision,
    pub update: AssignmentPoliciesUpdate,
}

/// Result of a complete Policies-page save.
#[derive(Debug, Clone, PartialEq)]
pub enum ReplaceAssignmentPoliciesOutcome {
    /// The policy slice replaced one aggregate revision.
    Replaced(Box<StoredAssignment>),
    /// The expected aggregate revision no longer matches current state.
    RevisionConflict,
}

/// Authenticated instructor command that atomically replaces an assignment's
/// editable definition fields.
#[derive(Debug, Clone, PartialEq)]
pub struct ReplaceAssignmentCommand {
    pub actor: UserId,
    pub course: CourseId,
    pub assignment: AssignmentId,
    pub expected_revision: AssignmentRevision,
    pub update: AssignmentUpdate,
}

/// Revision-checked instructor command behind the Delete and Regrade action.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DeleteAndRegradeAssignmentItemCommand {
    /// Authenticated direct Instructor authorized inside the write boundary.
    pub actor: UserId,
    pub course: CourseId,
    pub assignment: AssignmentId,
    pub item: AssignmentItemId,
    pub expected_revision: AssignmentRevision,
}

/// Revision-checked replacement of one fixed item for future assignment runs.
///
/// The server resolves `replacement` from an instructor-selected Question ID
/// before it builds this command. `current_item` is the stable
/// assignment-owned slot identity; storage changes that slot's exact immutable
/// publication while preserving its position, points, delivery state, and
/// scoring mode. Issued run evidence retains its own frozen reference.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReplaceAssignmentFixedItemCommand {
    /// Authenticated direct Instructor authorized inside the write boundary.
    pub actor: UserId,
    /// Course that authorizes the edit.
    pub course: CourseId,
    /// Assignment whose future definition will change.
    pub assignment: AssignmentId,
    /// Stable assignment-owned fixed-item slot selected for replacement.
    pub current_item: AssignmentItemId,
    /// Strong revision token read by the instructor before replacement.
    pub expected_revision: AssignmentRevision,
    /// Exact immutable publication resolved from the selected Question ID.
    pub replacement: ProblemVersionRef,
}

/// Revision-checked insertion of one fixed item before the assignment has
/// learner evidence.
///
/// The server mints `item.id` and resolves its exact immutable publication
/// from an instructor-selected Question ID. `item.position` is the requested
/// visible future-run position; storage shifts later fixed items as needed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AddAssignmentFixedItemCommand {
    /// Authenticated direct Instructor authorized inside the write boundary.
    pub actor: UserId,
    /// Course that authorizes the edit.
    pub course: CourseId,
    /// Assignment whose future definition gains an item.
    pub assignment: AssignmentId,
    /// Strong revision token read by the instructor before insertion.
    pub expected_revision: AssignmentRevision,
    /// Fresh server-minted item and its requested visible position.
    pub item: question_model::AssignmentItem,
}
/// Revision-checked removal of one fixed item before the assignment has
/// learner evidence.
///
/// This ordinary editor workflow removes the current fixed item from future
/// delivery. [`DeleteAndRegradeAssignmentItemCommand`] remains the explicit
/// destructive workflow once protected evidence requires retirement and score
/// recalculation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RemoveAssignmentFixedItemCommand {
    /// Authenticated direct Instructor authorized inside the write boundary.
    pub actor: UserId,
    /// Course that authorizes the edit.
    pub course: CourseId,
    /// Assignment whose future definition loses an item.
    pub assignment: AssignmentId,
    /// Existing fixed item selected for removal.
    pub item: AssignmentItemId,
    /// Strong revision token read by the instructor before removal.
    pub expected_revision: AssignmentRevision,
}
impl AssignmentRecord {
    /// Derives the current publication readiness from the future-run definition.
    pub fn publication_readiness(&self) -> question_model::AssignmentPublicationReadiness {
        question_model::AssignmentPublicationReadiness::from_definition(
            &self.items,
            &self.selection_groups,
        )
    }

    /// Replaces only the Questions-owned slice of this aggregate.
    pub fn with_content_update(&self, update: AssignmentContentUpdate) -> Self {
        Self {
            title: update.title,
            items: update.items,
            selection_groups: update.selection_groups,
            ..self.clone()
        }
    }

    /// Replaces only the Policies-owned slice of this aggregate.
    pub fn with_policies_update(&self, update: AssignmentPoliciesUpdate) -> Self {
        Self {
            lifecycle: update.teaching_settings.lifecycle,
            instructions: update.teaching_settings.instructions,
            audience: update.audience,
            disclosure_policy: update.disclosure_policy,
            policies: update.policies,
            ..self.clone()
        }
    }

    /// Every pinned immutable reference in the current assignment definition.
    pub fn references(&self) -> impl Iterator<Item = ProblemVersionRef> + '_ {
        self.items.iter().map(|item| item.reference).chain(
            self.selection_groups
                .iter()
                .flat_map(|group| group.candidates.iter().map(|candidate| candidate.reference)),
        )
    }

    /// Active fixed items in current future-run order.
    pub fn active_items(&self) -> impl Iterator<Item = &question_model::AssignmentItem> {
        self.items
            .iter()
            .filter(|item| item.delivery_state == question_model::AssignmentDeliveryState::Active)
    }

    /// Immutable content that may be delivered by a future run.
    pub fn active_references(&self) -> impl Iterator<Item = ProblemVersionRef> + '_ {
        self.active_items()
            .map(|item| item.reference)
            .chain(self.selection_groups.iter().flat_map(|group| {
                group.candidates.iter().filter_map(|candidate| {
                    (candidate.delivery_state == question_model::AssignmentDeliveryState::Active)
                        .then_some(candidate.reference)
                })
            }))
    }

    /// Resolves one active fixed item by its future-run position.
    pub fn active_item_at(&self, position: u32) -> Option<&question_model::AssignmentItem> {
        self.active_items().find(|item| item.position == position)
    }

    /// Builds the browser-safe assignment projection from server-resolved
    /// Question-ID display data.
    pub fn summary(
        &self,
        reference: question_model::AssignmentReference,
        items: Vec<question_model::AssignmentItemSummary>,
        selection_groups: Vec<question_model::AssignmentSelectionGroupSummary>,
    ) -> AssignmentSummary {
        AssignmentSummary {
            id: self.id,
            reference,
            course_id: self.course_id,
            title: self.title.clone(),
            items,
            selection_groups,
            disclosure_policy: self.disclosure_policy,
            policies: self.policies,
        }
    }
}

/// Applies current assignment scoring to one normalized backend result.
pub(crate) fn current_attempt_points(
    assignment: &AssignmentRecord,
    assignment_item: AssignmentItemId,
    status: AttemptStatus,
    result: AttemptResult,
) -> Result<(f64, f64), StoreError> {
    validate_attempt_result(result)?;
    if matches!(status, AttemptStatus::Cleared | AttemptStatus::Exempt) {
        return Ok((0.0, 0.0));
    }
    let (points, mode) = if let Some(item) = assignment
        .items
        .iter()
        .find(|item| item.id == assignment_item)
    {
        (item.points_possible, item.scoring_mode)
    } else if let Some(group) = assignment.selection_groups.iter().find(|group| {
        group
            .candidates
            .iter()
            .any(|candidate| candidate.id == assignment_item)
    }) {
        let candidate = group
            .candidates
            .iter()
            .find(|candidate| candidate.id == assignment_item)
            .expect("selection group was found through this candidate");
        (
            group.points_per_item,
            if candidate.delivery_state == AssignmentDeliveryState::Retired {
                question_model::AssignmentScoringMode::Excluded
            } else {
                question_model::AssignmentScoringMode::Normal
            },
        )
    } else {
        return Err(StoreError::InvalidRecord(
            "run item no longer resolves to a current scoring definition".to_string(),
        ));
    };
    let credit = result.points_earned / result.points_possible;
    let possible_points = points.scaled() as f64 / 10_000.0;
    let (earned, possible) = match mode {
        question_model::AssignmentScoringMode::Normal => {
            (credit * possible_points, possible_points)
        }
        question_model::AssignmentScoringMode::FullCredit => (possible_points, possible_points),
        question_model::AssignmentScoringMode::ExtraCredit => (credit * possible_points, 0.0),
        question_model::AssignmentScoringMode::Excluded => (0.0, 0.0),
    };
    Ok((
        score_precision::round_for_persistence(earned),
        score_precision::round_for_persistence(possible),
    ))
}

#[cfg(any(test, feature = "test-support"))]
pub(crate) fn assignment_item_is_retired(
    assignment: &AssignmentRecord,
    assignment_item: AssignmentItemId,
) -> Option<bool> {
    assignment
        .items
        .iter()
        .find(|item| item.id == assignment_item)
        .map(|item| item.delivery_state == AssignmentDeliveryState::Retired)
        .or_else(|| {
            assignment
                .selection_groups
                .iter()
                .flat_map(|group| group.candidates.iter())
                .find(|candidate| candidate.id == assignment_item)
                .map(|candidate| candidate.delivery_state == AssignmentDeliveryState::Retired)
        })
}

/// Publishes the first completion and replaces computed score fields and run pointers.
pub(crate) fn recalculated_enrollment_projection(
    mut enrollment: AssignmentEnrollment,
    mut summary: StudentAssignmentSummary,
    grade_policy: GradePolicy,
    mut completed_runs: Vec<domain::scoring::CompletedRunScore>,
    first_completed_at: Option<question_model::ActivityTimestamp>,
) -> Result<(AssignmentEnrollment, StudentAssignmentSummary), StoreError> {
    completed_runs.sort_by_key(|run| run.run_number);
    let selected = domain::scoring::score(
        &completed_runs,
        grade_policy,
        (grade_policy == GradePolicy::InstructorSelected)
            .then_some(enrollment.current_grade_run)
            .flatten(),
    )
    .map_err(|error| StoreError::InvalidRecord(error.to_string()))?;
    let best = domain::scoring::score(&completed_runs, GradePolicy::Highest, None)
        .map_err(|error| StoreError::InvalidRecord(error.to_string()))?;
    summary.completed_run_count = u32::try_from(completed_runs.len())
        .map_err(|_| StoreError::InvalidRecord("too many completed runs".to_string()))?;
    summary.latest_score = completed_runs.last().map(|run| run.score);
    summary.best_score = best.map(|selection| selection.score);
    summary.current_score = selected.map(|selection| selection.score);
    if enrollment.first_completed_at.is_none() {
        enrollment.first_completed_at = first_completed_at;
    }
    enrollment.best_grade_run = best.map(|selection| selection.run);
    enrollment.current_grade_run = selected.map(|selection| selection.run);
    Ok((enrollment, summary))
}

#[cfg(test)]
mod tests;

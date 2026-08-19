use super::*;

/// Tenant-owned course. Direct access lives exclusively in
/// [`CourseMembershipRecord`], never in this aggregate.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CourseRecord {
    /// Durable course identity.
    pub id: CourseId,
    /// Direct RLS boundary.
    pub tenant: TenantId,
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
    pub initial_instructor: UserId,
}

/// Canonical course access relationship.  Each reinvitation creates a new
/// episode; a revoked row remains immutable evidence for receipts created
/// during that episode.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CourseMembershipRecord {
    pub id: question_model::CourseMembershipId,
    pub tenant: TenantId,
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
            tenant: self.tenant,
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

/// Tenant-owned assignment that references shared immutable content.
#[derive(Debug, Clone, PartialEq)]
pub struct AssignmentRecord {
    /// Durable assignment identity.
    pub id: AssignmentId,
    /// Direct RLS boundary.
    pub tenant: TenantId,
    /// Tenant-owned course containing the assignment.
    pub course_id: CourseId,
    /// Human-facing assignment title.
    pub title: String,
    /// Explicit current audience.  Course-wide and group-scoped delivery are
    /// different contracts; absence is not a compatible default.
    pub audience: question_model::AssignmentAudience,
    /// Stable ordered fixed items selected for the assignment.
    pub items: Vec<question_model::AssignmentItem>,
    /// Random-selection groups with pinned immutable candidates.
    pub selection_groups: Vec<question_model::AssignmentSelectionGroup>,
    /// Assignment-owned learner-facing disclosure schedule, evaluated only by
    /// the server-side learner projection path.
    pub disclosure_policy: question_model::LearnerDisclosurePolicy,
    /// Four independent run policies.
    pub policies: RunPolicies,
}

/// Editable assignment definition together with its server-managed revision.
#[derive(Debug, Clone, PartialEq)]
pub struct StoredAssignment {
    pub record: AssignmentRecord,
    pub revision: AssignmentRevision,
    /// Editor-only whole-run timer choice held under the same revision.
    pub assignment_timing: question_model::AssignmentRunTiming,
    /// Generation matched by current computed score rows.
    pub scoring_generation: ScoringGeneration,
    /// Whether scores for this generation may be presented.
    pub scoring_status: ScoringStatus,
}

/// Editable assignment fields supplied after the server has bound identity and
/// course ownership from the authenticated route.
#[derive(Debug, Clone, PartialEq)]
pub struct AssignmentUpdate {
    pub title: String,
    pub audience: question_model::AssignmentAudience,
    pub items: Vec<question_model::AssignmentItem>,
    pub selection_groups: Vec<question_model::AssignmentSelectionGroup>,
    pub disclosure_policy: question_model::LearnerDisclosurePolicy,
    pub policies: RunPolicies,
}

/// One editor save expressed as one revision-checked persistence operation.
///
/// `assignment_timing` is deliberately narrower than the effective-policy
/// resolver: a normal editor save must not clear schedule or accommodation
/// settings owned by their dedicated policy workflow.
#[derive(Debug, Clone, PartialEq)]
pub struct AssignmentEditorUpdate {
    pub assignment: AssignmentUpdate,
    pub assignment_timing: question_model::AssignmentRunTiming,
}

/// Revision-checked instructor command behind the Delete and Regrade action.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DeleteAndRegradeAssignmentItemCommand {
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
    /// Tenant-owned course that authorizes the edit.
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
    /// Tenant-owned course that authorizes the edit.
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
    /// Tenant-owned course that authorizes the edit.
    pub course: CourseId,
    /// Assignment whose future definition loses an item.
    pub assignment: AssignmentId,
    /// Existing fixed item selected for removal.
    pub item: AssignmentItemId,
    /// Strong revision token read by the instructor before removal.
    pub expected_revision: AssignmentRevision,
}
impl AssignmentRecord {
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
            tenant: self.tenant,
            course_id: self.course_id,
            title: self.title.clone(),
            items,
            selection_groups,
            disclosure_policy: self.disclosure_policy,
            policies: self.policies,
        }
    }
}

/// Freezes current fixed items and deterministic group selections for one new run.
pub(crate) fn select_assignment_run_items(
    assignment: &AssignmentRecord,
    run: RunId,
) -> Result<Vec<AssignmentRunItem>, StoreError> {
    enum Source<'a> {
        Fixed(&'a AssignmentItem),
        Group(&'a AssignmentSelectionGroup),
    }
    let mut sources = assignment
        .active_items()
        .map(|item| (item.position, Source::Fixed(item)))
        .chain(
            assignment
                .selection_groups
                .iter()
                .map(|group| (group.position, Source::Group(group))),
        )
        .collect::<Vec<_>>();
    sources.sort_by_key(|(position, _)| *position);
    let mut selected = Vec::new();
    for (source_position, source) in sources {
        match source {
            Source::Fixed(item) => {
                selected.push((item.id, source_position, item.reference, None, None))
            }
            Source::Group(group) => {
                let seed = assignment_selection_seed(run, group);
                let mut candidates = group
                    .candidates
                    .iter()
                    .filter(|candidate| candidate.delivery_state == AssignmentDeliveryState::Active)
                    .map(|candidate| (assignment_selection_rank(seed, candidate.id), candidate))
                    .collect::<Vec<_>>();
                candidates.sort_by_key(|(rank, candidate)| (*rank, candidate.id));
                candidates.truncate(usize::try_from(group.draw_count).map_err(|_| {
                    StoreError::InvalidRecord("selection draw count is too large".to_string())
                })?);
                if group.ordering == SelectionOrdering::CandidateOrder {
                    candidates.sort_by_key(|(_, candidate)| (candidate.position, candidate.id));
                }
                for (_, candidate) in candidates {
                    selected.push((
                        candidate.id,
                        source_position,
                        candidate.reference,
                        Some(group.id),
                        Some(seed),
                    ));
                }
            }
        }
    }
    selected
        .into_iter()
        .enumerate()
        .map(
            |(
                issued_position,
                (assignment_item, source_position, reference, selection_group, selection_seed),
            )| {
                Ok(AssignmentRunItem {
                    run,
                    assignment_item,
                    source_position,
                    issued_position: u32::try_from(issued_position).map_err(|_| {
                        StoreError::InvalidRecord("too many selected run items".to_string())
                    })?,
                    reference,
                    selection_group,
                    selection_seed,
                })
            },
        )
        .collect()
}

fn assignment_selection_seed(run: RunId, group: &AssignmentSelectionGroup) -> u64 {
    let mut bytes = Vec::with_capacity(34);
    bytes.extend_from_slice(run.as_uuid().as_bytes());
    bytes.extend_from_slice(group.id.as_uuid().as_bytes());
    bytes.extend_from_slice(&group.algorithm_version.to_be_bytes());
    let digest = Sha256Digest::compute(&bytes);
    let mut seed = [0_u8; 8];
    seed.copy_from_slice(&digest.as_bytes()[..8]);
    u64::from_be_bytes(seed) & 9_007_199_254_740_991
}

fn assignment_selection_rank(seed: u64, candidate: AssignmentItemId) -> u64 {
    let mut bytes = Vec::with_capacity(24);
    bytes.extend_from_slice(&seed.to_be_bytes());
    bytes.extend_from_slice(candidate.as_uuid().as_bytes());
    let digest = Sha256Digest::compute(&bytes);
    let mut rank = [0_u8; 8];
    rank.copy_from_slice(&digest.as_bytes()[..8]);
    u64::from_be_bytes(rank)
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
mod assignment_selection_tests {
    use super::*;
    use question_model::{
        AssignmentScoringMode, AssignmentSelectionCandidate, AttemptTimerRecord,
        ImplementationVersion, PointValue, ProblemVersionRef,
    };

    fn id(value: u128) -> Uuid {
        Uuid::from_u128(value)
    }

    #[test]
    fn run_selection_is_reproducible_and_freezes_expanded_order() {
        let reference = |value| ProblemVersionRef {
            problem: ProblemId::from_uuid(id(10 + value)),
            version: VersionId::from_uuid(id(20 + value)),
        };
        let assignment = AssignmentRecord {
            id: AssignmentId::from_uuid(id(1)),
            tenant: TenantId::from_uuid(id(2)),
            course_id: CourseId::from_uuid(id(3)),
            title: "Selection fixture".to_string(),
            audience: question_model::AssignmentAudience::CourseWide,
            items: vec![AssignmentItem {
                id: AssignmentItemId::from_uuid(id(30)),
                reference: reference(0),
                position: 0,
                points_possible: PointValue::from_whole(1),
                delivery_state: AssignmentDeliveryState::Active,
                scoring_mode: AssignmentScoringMode::Normal,
            }],
            selection_groups: vec![AssignmentSelectionGroup {
                id: question_model::AssignmentSelectionGroupId::from_uuid(id(31)),
                position: 1,
                draw_count: 2,
                points_per_item: PointValue::from_whole(2),
                ordering: SelectionOrdering::Randomized,
                algorithm_version: 1,
                candidates: (1..=4)
                    .map(|value| AssignmentSelectionCandidate {
                        id: AssignmentItemId::from_uuid(id(40 + value)),
                        position: u32::try_from(value - 1).expect("fixture position"),
                        reference: reference(value),
                        delivery_state: if value == 4 {
                            AssignmentDeliveryState::Retired
                        } else {
                            AssignmentDeliveryState::Active
                        },
                    })
                    .collect(),
            }],
            disclosure_policy: question_model::LearnerDisclosurePolicy::default(),
            policies: RunPolicies {
                completion: question_model::CompletionRequirement::AnswerAll,
                grade: GradePolicy::Highest,
                continued_practice: question_model::ContinuedPractice::Unlimited,
                variation: question_model::VariationPolicy::NewSeeds,
            },
        };
        let run = RunId::from_uuid(id(100));
        let first = select_assignment_run_items(&assignment, run).expect("valid selection");
        let replay = select_assignment_run_items(&assignment, run).expect("repeat selection");

        assert_eq!(first, replay);
        assert_eq!(first.len(), 3);
        assert_eq!(
            first
                .iter()
                .map(|item| item.issued_position)
                .collect::<Vec<_>>(),
            vec![0, 1, 2]
        );
        assert!(first[0].selection_group.is_none());
        assert!(first[1..].iter().all(|item| item.selection_seed.is_some()));
        assert!(
            first
                .iter()
                .all(|item| item.assignment_item != AssignmentItemId::from_uuid(id(44)))
        );
        let next = select_assignment_run_items(&assignment, RunId::from_uuid(id(101)))
            .expect("next run selection");
        assert_ne!(first[1].selection_seed, next[1].selection_seed);
    }

    #[test]
    fn current_attempt_points_apply_every_scoring_mode_and_attempt_exclusion() {
        let reference = ProblemVersionRef {
            problem: ProblemId::from_uuid(id(200)),
            version: VersionId::from_uuid(id(201)),
        };
        let modes = [
            AssignmentScoringMode::Normal,
            AssignmentScoringMode::FullCredit,
            AssignmentScoringMode::ExtraCredit,
            AssignmentScoringMode::Excluded,
        ];
        let assignment = AssignmentRecord {
            id: AssignmentId::from_uuid(id(202)),
            tenant: TenantId::from_uuid(id(203)),
            course_id: CourseId::from_uuid(id(204)),
            title: "Scoring modes".to_string(),
            audience: question_model::AssignmentAudience::CourseWide,
            items: modes
                .into_iter()
                .enumerate()
                .map(|(position, scoring_mode)| AssignmentItem {
                    id: AssignmentItemId::from_uuid(id(210 + position as u128)),
                    reference,
                    position: u32::try_from(position).expect("fixture position"),
                    points_possible: PointValue::from_whole(2),
                    delivery_state: AssignmentDeliveryState::Active,
                    scoring_mode,
                })
                .collect(),
            selection_groups: Vec::new(),
            disclosure_policy: question_model::LearnerDisclosurePolicy::default(),
            policies: RunPolicies {
                completion: question_model::CompletionRequirement::AnswerAll,
                grade: GradePolicy::Highest,
                continued_practice: question_model::ContinuedPractice::Unlimited,
                variation: question_model::VariationPolicy::NewSeeds,
            },
        };
        let result = |credit: f64| AttemptResult {
            correct: credit == 1.0,
            points_earned: credit,
            points_possible: 1.0,
        };

        assert_eq!(
            current_attempt_points(
                &assignment,
                assignment.items[0].id,
                AttemptStatus::Submitted,
                result(-0.5),
            ),
            Ok((-1.0, 2.0)),
            "normal scoring retains negative credit"
        );
        assert_eq!(
            current_attempt_points(
                &assignment,
                assignment.items[1].id,
                AttemptStatus::Submitted,
                result(-0.5),
            ),
            Ok((2.0, 2.0)),
            "full credit ignores the normalized result"
        );
        assert_eq!(
            current_attempt_points(
                &assignment,
                assignment.items[2].id,
                AttemptStatus::Submitted,
                result(1.25),
            ),
            Ok((2.5, 0.0)),
            "extra credit changes only the numerator"
        );
        assert_eq!(
            current_attempt_points(
                &assignment,
                assignment.items[3].id,
                AttemptStatus::Submitted,
                result(1.0),
            ),
            Ok((0.0, 0.0)),
            "excluded items change neither numerator nor denominator"
        );
        assert_eq!(
            current_attempt_points(
                &assignment,
                assignment.items[0].id,
                AttemptStatus::Cleared,
                result(1.0),
            ),
            Ok((0.0, 0.0)),
            "cleared attempts are absent from current scoring"
        );
        assert_eq!(
            current_attempt_points(
                &assignment,
                assignment.items[0].id,
                AttemptStatus::Submitted,
                result(4.000_000_000_000_3),
            ),
            Ok((8.0, 2.0)),
            "computed points are rounded before persistence"
        );
    }

    #[test]
    fn completed_run_score_is_rounded_before_persistence() {
        let questions = vec![Some(CurrentRunQuestion {
            assignment_item: AssignmentItemId::from_uuid(id(250)),
            result: AttemptResult {
                correct: false,
                points_earned: 1.0,
                points_possible: 3.0,
            },
            earned_points: 1.0,
            possible_points: 3.0,
        })];

        assert_eq!(
            completed_run_score(&questions, question_model::CompletionRequirement::AnswerAll),
            Ok(Some(0.3333))
        );
    }

    #[test]
    fn selected_group_items_complete_from_the_immutable_delivered_order() {
        let tenant = TenantId::from_uuid(id(300));
        let run = RunId::from_uuid(id(301));
        let reference = |value| ProblemVersionRef {
            problem: ProblemId::from_uuid(id(310 + value)),
            version: VersionId::from_uuid(id(320 + value)),
        };
        let assignment = AssignmentRecord {
            id: AssignmentId::from_uuid(id(302)),
            tenant,
            course_id: CourseId::from_uuid(id(303)),
            title: "Selected completion".to_string(),
            audience: question_model::AssignmentAudience::CourseWide,
            items: Vec::new(),
            selection_groups: vec![AssignmentSelectionGroup {
                id: question_model::AssignmentSelectionGroupId::from_uuid(id(304)),
                position: 0,
                draw_count: 2,
                points_per_item: PointValue::from_whole(2),
                ordering: SelectionOrdering::CandidateOrder,
                algorithm_version: 1,
                candidates: (0..2)
                    .map(|position| AssignmentSelectionCandidate {
                        id: AssignmentItemId::from_uuid(id(330 + u128::from(position))),
                        position,
                        reference: reference(u128::from(position)),
                        delivery_state: AssignmentDeliveryState::Active,
                    })
                    .collect(),
            }],
            disclosure_policy: question_model::LearnerDisclosurePolicy::default(),
            policies: RunPolicies {
                completion: question_model::CompletionRequirement::AnswerAll,
                grade: GradePolicy::Highest,
                continued_practice: question_model::ContinuedPractice::Unlimited,
                variation: question_model::VariationPolicy::NewSeeds,
            },
        };
        let run_items = select_assignment_run_items(&assignment, run).expect("selected run items");
        let attempts = run_items
            .iter()
            .enumerate()
            .map(|(index, item)| QuestionAttempt {
                id: QuestionAttemptId::from_uuid(id(340 + index as u128)),
                tenant,
                run,
                problem: item.reference.problem,
                question_version: item.reference.version,
                assignment_position: item.issued_position,
                seed: u64::try_from(index).expect("fixture seed"),
                parameter_hash: format!("selected-{index}"),
                response: Some(StudentResponse::Numeric { value: 1.0 }),
                status: AttemptStatus::Submitted,
                result: Some(AttemptResult {
                    correct: true,
                    points_earned: 1.0,
                    points_possible: 1.0,
                }),
                timer: AttemptTimerRecord {
                    issued_at: ActivityTimestamp::from_unix_millis(index as i64),
                    deadline: None,
                    submitted_at: Some(ActivityTimestamp::from_unix_millis(index as i64 + 1)),
                },
                provenance: AttemptProvenance {
                    adapter: ImplementationVersion {
                        id: "native".to_string(),
                        version: "1".to_string(),
                    },
                    renderer: None,
                    generator: None,
                    source_artifact: None,
                    asset_objects: Vec::new(),
                    grading: ImplementationVersion {
                        id: "native".to_string(),
                        version: "1".to_string(),
                    },
                    rendered_question_sha256: format!("selected-render-{index}"),
                },
                issued_capability: question_model::IssuedAttemptCapabilityV1::NotApplicable,
            })
            .collect::<Vec<_>>();
        let questions = current_run_questions(
            &assignment,
            &run_items,
            &attempts,
            attempts.last().expect("selected current attempt"),
        )
        .expect("selected questions resolve");

        assert_eq!(questions.len(), 2);
        assert_eq!(
            completed_run_score(&questions, question_model::CompletionRequirement::AnswerAll),
            Ok(Some(1.0))
        );
        assert!(questions.iter().all(|question| {
            question.is_some_and(|question| {
                question.earned_points == 2.0 && question.possible_points == 2.0
            })
        }));
    }

    fn immutable_assignment_fixture() -> AssignmentRecord {
        let reference = |value: u128| ProblemVersionRef {
            problem: ProblemId::from_uuid(id(10 + value)),
            version: VersionId::from_uuid(id(20 + value)),
        };
        let item = |id_value, reference, position| AssignmentItem {
            id: AssignmentItemId::from_uuid(id(id_value)),
            reference,
            position,
            points_possible: PointValue::from_whole(1),
            delivery_state: AssignmentDeliveryState::Active,
            scoring_mode: AssignmentScoringMode::Normal,
        };
        AssignmentRecord {
            id: AssignmentId::from_uuid(id(1)),
            tenant: TenantId::from_uuid(id(2)),
            course_id: CourseId::from_uuid(id(3)),
            title: "Immutable item fixture".to_string(),
            audience: question_model::AssignmentAudience::CourseWide,
            items: vec![item(30, reference(1), 0), item(31, reference(2), 1)],
            selection_groups: vec![AssignmentSelectionGroup {
                id: question_model::AssignmentSelectionGroupId::from_uuid(id(40)),
                position: 2,
                draw_count: 1,
                points_per_item: PointValue::from_whole(1),
                ordering: SelectionOrdering::CandidateOrder,
                algorithm_version: 1,
                candidates: vec![AssignmentSelectionCandidate {
                    id: AssignmentItemId::from_uuid(id(41)),
                    position: 0,
                    reference: reference(3),
                    delivery_state: AssignmentDeliveryState::Active,
                }],
            }],
            disclosure_policy: question_model::LearnerDisclosurePolicy::default(),
            policies: RunPolicies {
                completion: question_model::CompletionRequirement::AnswerAll,
                grade: GradePolicy::Highest,
                continued_practice: question_model::ContinuedPractice::Unlimited,
                variation: question_model::VariationPolicy::NewSeeds,
            },
        }
    }

    fn ordinary_update(record: &AssignmentRecord) -> AssignmentUpdate {
        AssignmentUpdate {
            title: record.title.clone(),
            audience: record.audience.clone(),
            items: record.items.clone(),
            selection_groups: record.selection_groups.clone(),
            disclosure_policy: record.disclosure_policy,
            policies: record.policies,
        }
    }

    #[test]
    fn ordinary_assignment_save_rejects_content_identity_changes() {
        let record = immutable_assignment_fixture();
        let mut changed_reference = ordinary_update(&record);
        changed_reference.items[0].reference.version = VersionId::from_uuid(id(99));
        let mut removed = ordinary_update(&record);
        removed.items.pop();
        let mut added = ordinary_update(&record);
        let mut fresh = added.items[0].clone();
        fresh.id = AssignmentItemId::from_uuid(id(98));
        added.items.push(fresh);
        let mut candidate_substitution = ordinary_update(&record);
        candidate_substitution.selection_groups[0].candidates[0]
            .reference
            .problem = ProblemId::from_uuid(id(97));

        for update in [changed_reference, removed, added, candidate_substitution] {
            assert!(matches!(
                ensure_assignment_update_preserves_references(&record, &update),
                Err(StoreError::InvalidRecord(_))
            ));
        }
    }

    #[test]
    fn ordinary_assignment_save_allows_reordering_and_authored_settings() {
        let record = immutable_assignment_fixture();
        let mut update = ordinary_update(&record);
        update.items.swap(0, 1);
        update.items[0].position = 0;
        update.items[1].position = 1;
        update.items[0].points_possible = PointValue::from_whole(4);
        update.items[1].scoring_mode = AssignmentScoringMode::ExtraCredit;
        update.selection_groups[0].position = 4;
        update.selection_groups[0].candidates[0].position = 3;

        assert_eq!(
            ensure_assignment_update_preserves_references(&record, &update),
            Ok(())
        );
    }
}

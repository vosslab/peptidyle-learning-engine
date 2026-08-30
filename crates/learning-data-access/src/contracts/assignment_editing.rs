//! Revision-checked assignment editing invariants and transitions.
//!
//! Assignment records own the persisted definition. This module owns the
//! focused editing operations that preserve immutable publication bindings or
//! deliberately retire an item and force score recalculation.

use super::*;

/// Confirms that an ordinary full-assignment save preserves the exact fixed
/// item and selection-candidate identity-to-publication mappings. The save
/// may reorder those identities and update assignment-authored settings.
/// Focused revision-checked commands make content substitutions, additions,
/// and removals explicit.
pub fn ensure_assignment_update_preserves_references(
    current: &AssignmentRecord,
    update: &AssignmentUpdate,
) -> Result<(), StoreError> {
    let fixed_references = |items: &[question_model::AssignmentItem]| {
        items
            .iter()
            .map(|item| (item.id, item.reference))
            .collect::<std::collections::BTreeMap<_, _>>()
    };
    let candidate_references = |groups: &[question_model::AssignmentSelectionGroup]| {
        groups
            .iter()
            .flat_map(|group| group.candidates.iter())
            .map(|candidate| (candidate.id, candidate.reference))
            .collect::<std::collections::BTreeMap<_, _>>()
    };
    let same_identity_set = |persisted: &std::collections::BTreeMap<_, _>,
                             edited: &std::collections::BTreeMap<_, _>,
                             persisted_count: usize,
                             edited_count: usize,
                             kind: &str| {
        (persisted.len() == persisted_count
            && edited.len() == edited_count
            && persisted == edited)
            .then_some(())
            .ok_or_else(|| {
                StoreError::InvalidRecord(format!(
                    "an ordinary assignment save preserves every {kind} identity and immutable publication; use a focused revision-checked item command"
                ))
            })
    };

    let current_fixed = fixed_references(&current.items);
    let updated_fixed = fixed_references(&update.items);
    same_identity_set(
        &current_fixed,
        &updated_fixed,
        current.items.len(),
        update.items.len(),
        "fixed item",
    )?;
    let current_candidate = candidate_references(&current.selection_groups);
    let updated_candidate = candidate_references(&update.selection_groups);
    let current_candidate_count = current
        .selection_groups
        .iter()
        .map(|group| group.candidates.len())
        .sum();
    let updated_candidate_count = update
        .selection_groups
        .iter()
        .map(|group| group.candidates.len())
        .sum();
    same_identity_set(
        &current_candidate,
        &updated_candidate,
        current_candidate_count,
        updated_candidate_count,
        "selection candidate",
    )
}

/// Produces the explicit retirement update used by Delete and Regrade.
pub(crate) fn delete_and_regrade_update(
    stored: &StoredAssignment,
    target: AssignmentItemId,
) -> Result<Option<AssignmentUpdate>, StoreError> {
    let mut update = AssignmentUpdate {
        title: stored.record.title.clone(),
        audience: stored.record.audience.clone(),
        items: stored.record.items.clone(),
        selection_groups: stored.record.selection_groups.clone(),
        disclosure_policy: stored.record.disclosure_policy,
        policies: stored.record.policies,
    };
    if let Some(item) = update.items.iter_mut().find(|item| item.id == target) {
        if item.delivery_state == AssignmentDeliveryState::Retired
            && item.scoring_mode == question_model::AssignmentScoringMode::Excluded
        {
            return Ok(None);
        }
        item.delivery_state = AssignmentDeliveryState::Retired;
        item.scoring_mode = question_model::AssignmentScoringMode::Excluded;
        return Ok(Some(update));
    }
    if let Some(candidate) = update
        .selection_groups
        .iter_mut()
        .flat_map(|group| group.candidates.iter_mut())
        .find(|candidate| candidate.id == target)
    {
        if candidate.delivery_state == AssignmentDeliveryState::Retired {
            return Ok(None);
        }
        candidate.delivery_state = AssignmentDeliveryState::Retired;
        return Ok(Some(update));
    }
    Err(StoreError::NotFound)
}

/// Reports whether an update changes the score-relevant assignment definition.
pub(crate) fn assignment_scoring_changed(
    previous: &AssignmentRecord,
    replacement: &AssignmentRecord,
) -> bool {
    let fixed = |assignment: &AssignmentRecord| {
        assignment
            .items
            .iter()
            .map(|item| {
                (
                    item.id,
                    (item.points_possible, item.delivery_state, item.scoring_mode),
                )
            })
            .collect::<std::collections::BTreeMap<_, _>>()
    };
    let groups = |assignment: &AssignmentRecord| {
        assignment
            .selection_groups
            .iter()
            .map(|group| {
                (
                    group.id,
                    (
                        group.points_per_item,
                        group
                            .candidates
                            .iter()
                            .map(|candidate| (candidate.id, candidate.delivery_state))
                            .collect::<std::collections::BTreeMap<_, _>>(),
                    ),
                )
            })
            .collect::<std::collections::BTreeMap<_, _>>()
    };
    previous.policies.completion != replacement.policies.completion
        || previous.policies.grade != replacement.policies.grade
        || fixed(previous) != fixed(replacement)
        || groups(previous) != groups(replacement)
}

/// Reports whether a Questions-slice save changes work that is immutable once
/// learner evidence exists.
///
/// The Questions command owns fixed-item points, delivery, scoring, identity,
/// publication, and order, plus every pool's draw and per-item-point semantics.
/// A title remains presentation-only. Keeping this issued-work fence in the
/// shared Store contract prevents the in-memory and PostgreSQL implementations
/// from making different decisions for the same authenticated command.
///
/// ASVS 2.2.2, 2.3.1-2.3.3, 8.1.2, 8.2.3, 8.3.1, and 15.3.3: the trusted
/// service layer allowlists the Questions-owned fields and rejects their
/// mutation after issuance rather than trusting browser state or recalculating
/// already-issued evidence from a rewritten definition.
pub(crate) fn assignment_content_changes_issued_work(
    current: &AssignmentRecord,
    replacement: &AssignmentRecord,
) -> bool {
    let fixed_shape = |items: &[question_model::AssignmentItem]| {
        items
            .iter()
            .map(|item| {
                (
                    item.id,
                    item.reference,
                    item.position,
                    item.points_possible,
                    item.delivery_state,
                    item.scoring_mode,
                )
            })
            .collect::<Vec<_>>()
    };
    let group_shape = |groups: &[question_model::AssignmentSelectionGroup]| {
        groups
            .iter()
            .map(|group| {
                (
                    group.id,
                    group.position,
                    group.draw_count,
                    group.points_per_item,
                    group.ordering,
                    group.algorithm,
                    group
                        .candidates
                        .iter()
                        .map(|candidate| {
                            (
                                candidate.id,
                                candidate.reference,
                                candidate.position,
                                candidate.delivery_state,
                            )
                        })
                        .collect::<Vec<_>>(),
                )
            })
            .collect::<Vec<_>>()
    };

    fixed_shape(&current.items) != fixed_shape(&replacement.items)
        || group_shape(&current.selection_groups) != group_shape(&replacement.selection_groups)
}

#[cfg(test)]
mod structural_content_tests {
    use super::*;
    use question_model::{
        AssignmentAudience, AssignmentDeliveryState, AssignmentId, AssignmentInstructions,
        AssignmentItem, AssignmentItemId, AssignmentLifecycle, AssignmentScoringMode,
        AssignmentSelectionCandidate, AssignmentSelectionGroup, AssignmentSelectionGroupId,
        GradePolicy, PointValue, PoolDrawAlgorithm, ProblemId, ProblemVersionRef, RunPolicies,
        SelectionOrdering, StudentDisclosurePolicy, VersionId,
    };
    use uuid::Uuid;

    fn assignment() -> AssignmentRecord {
        AssignmentRecord {
            id: AssignmentId::from_uuid(Uuid::from_u128(1)),
            course_id: question_model::CourseId::from_uuid(Uuid::from_u128(3)),
            title: "Structural content fixture".to_string(),
            lifecycle: AssignmentLifecycle::Draft,
            instructions: AssignmentInstructions::default(),
            audience: AssignmentAudience::CourseWide,
            items: vec![AssignmentItem {
                id: AssignmentItemId::from_uuid(Uuid::from_u128(4)),
                reference: ProblemVersionRef {
                    problem: ProblemId::from_uuid(Uuid::from_u128(5)),
                    version: VersionId::from_uuid(Uuid::from_u128(6)),
                },
                position: 0,
                points_possible: PointValue::from_whole(1),
                delivery_state: AssignmentDeliveryState::Active,
                scoring_mode: AssignmentScoringMode::Normal,
            }],
            selection_groups: vec![AssignmentSelectionGroup {
                id: AssignmentSelectionGroupId::from_uuid(Uuid::from_u128(7)),
                position: 1,
                draw_count: 1,
                points_per_item: PointValue::from_whole(1),
                ordering: SelectionOrdering::CandidateOrder,
                algorithm: PoolDrawAlgorithm::V1,
                candidates: vec![AssignmentSelectionCandidate {
                    id: AssignmentItemId::from_uuid(Uuid::from_u128(8)),
                    position: 0,
                    reference: ProblemVersionRef {
                        problem: ProblemId::from_uuid(Uuid::from_u128(9)),
                        version: VersionId::from_uuid(Uuid::from_u128(10)),
                    },
                    delivery_state: AssignmentDeliveryState::Active,
                }],
            }],
            disclosure_policy: StudentDisclosurePolicy::default(),
            policies: RunPolicies {
                completion: question_model::CompletionRequirement::AnswerAll,
                grade: GradePolicy::Highest,
                continued_practice: question_model::ContinuedPractice::Unlimited,
                variation: question_model::VariationPolicy::NewSeeds,
            },
        }
    }

    #[test]
    fn issued_work_fence_allows_title_only_but_rejects_questions_owned_semantics() {
        let current = assignment();

        let mut title_only = current.clone();
        title_only.title = "Presentation-only revision".to_string();
        assert!(!assignment_content_changes_issued_work(
            &current,
            &title_only
        ));

        let mut fixed_points = current.clone();
        fixed_points.items[0].points_possible = PointValue::from_whole(2);
        assert!(assignment_content_changes_issued_work(
            &current,
            &fixed_points
        ));

        let mut fixed_retirement = current.clone();
        fixed_retirement.items[0].delivery_state = AssignmentDeliveryState::Retired;
        assert!(assignment_content_changes_issued_work(
            &current,
            &fixed_retirement
        ));

        let mut fixed_scoring = current.clone();
        fixed_scoring.items[0].scoring_mode = AssignmentScoringMode::Excluded;
        assert!(assignment_content_changes_issued_work(
            &current,
            &fixed_scoring
        ));

        let mut pool_points = current.clone();
        pool_points.selection_groups[0].points_per_item = PointValue::from_whole(2);
        assert!(assignment_content_changes_issued_work(
            &current,
            &pool_points
        ));

        let mut candidate_retirement = current.clone();
        candidate_retirement.selection_groups[0].candidates[0].delivery_state =
            AssignmentDeliveryState::Retired;
        assert!(assignment_content_changes_issued_work(
            &current,
            &candidate_retirement
        ));
    }
}

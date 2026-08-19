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

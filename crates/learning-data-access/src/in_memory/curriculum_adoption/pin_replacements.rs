//! Reauthorization and replacement of immutable source publication pins.

use std::collections::BTreeSet;

use question_model::curriculum_adoption::CurriculumSemanticPayload;
use question_model::{
    AssignmentDefinitionSourceView, CurriculumPinPosition, CurriculumPinReplacements,
    CurriculumSourceView, ReplacementQuestionChoices, UnavailablePinRecoveryAction,
};

use crate::curriculum_adoption::{
    PositionedPin, ResolvedPinReplacement, SemanticPlannerError, first_unavailable_pin,
    positioned_pins, substitute_resolved_pins,
};
use crate::in_memory::{MemoryStore, State, catalog_record_visible};
use crate::{StoreError, TenantId, UserId};

pub(crate) fn validate_destination_pins(
    state: &State,
    tenant: TenantId,
    payload: &CurriculumSemanticPayload,
) -> Result<(), CurriculumPinPosition> {
    unavailable_destination_pin(state, tenant, payload)
        .expect("validated qmodel semantic positions remain bounded")
        .map(PositionedPin::position)
        .map_or(Ok(()), Err)
}

pub(crate) fn unavailable_destination_pin(
    state: &State,
    tenant: TenantId,
    payload: &CurriculumSemanticPayload,
) -> Result<Option<PositionedPin>, StoreError> {
    first_unavailable_pin(payload, |reference| {
        pin_authorized(state, tenant, reference)
    })
    .map_err(semantic_error)
}

pub(crate) fn source_snapshot_with_replacements(
    state: &State,
    store: &MemoryStore,
    tenant: TenantId,
    actor: UserId,
    source: CurriculumSourceView,
    replacements: &CurriculumPinReplacements,
) -> Result<super::super::reusable_curriculum::ReusableSourceSnapshot, StoreError> {
    let snapshot = super::super::reusable_curriculum::curriculum_source_snapshot(
        state, tenant, actor, source,
    )?;
    let payload = apply_pin_replacements(state, store, tenant, &snapshot.payload, replacements)?;
    Ok(super::super::reusable_curriculum::ReusableSourceSnapshot { payload })
}

pub(crate) fn assignment_source_snapshot_with_replacements(
    state: &State,
    store: &MemoryStore,
    tenant: TenantId,
    actor: UserId,
    source: AssignmentDefinitionSourceView,
    replacements: &CurriculumPinReplacements,
) -> Result<super::super::reusable_curriculum::ReusableSourceSnapshot, StoreError> {
    let snapshot = super::super::reusable_curriculum::curriculum_assignment_source_snapshot(
        state, tenant, actor, source,
    )?;
    let payload = apply_pin_replacements(state, store, tenant, &snapshot.payload, replacements)?;
    Ok(super::super::reusable_curriculum::ReusableSourceSnapshot { payload })
}

pub(crate) fn apply_pin_replacements(
    state: &State,
    store: &MemoryStore,
    tenant: TenantId,
    payload: &CurriculumSemanticPayload,
    replacements: &CurriculumPinReplacements,
) -> Result<CurriculumSemanticPayload, StoreError> {
    let positions = positioned_pins(payload)
        .map_err(semantic_error)?
        .into_iter()
        .map(PositionedPin::position)
        .collect::<BTreeSet<_>>();
    if replacements
        .as_slice()
        .iter()
        .any(|replacement| !positions.contains(&replacement.position))
    {
        return Err(StoreError::InvalidRecord(
            "pin replacement does not identify a matching source pin".into(),
        ));
    }
    let resolved = replacements
        .as_slice()
        .iter()
        .map(|replacement| {
            let reference = super::super::reusable_curriculum::resolve_public_replacement(
                state,
                store,
                tenant,
                &replacement.question,
            )?;
            Ok(ResolvedPinReplacement {
                position: replacement.position,
                reference,
            })
        })
        .collect::<Result<Vec<_>, StoreError>>()?;
    substitute_resolved_pins(payload, &resolved).map_err(semantic_error)
}

fn pin_authorized(
    state: &State,
    tenant: TenantId,
    reference: question_model::ProblemVersionRef,
) -> bool {
    state
        .published
        .get(&(reference.problem, reference.version))
        .is_some_and(|record| {
            record.lifecycle.is_assignable() && catalog_record_visible(state, tenant, record)
        })
}

pub(crate) fn pin_correction(
    state: &State,
    tenant: TenantId,
    payload: &CurriculumSemanticPayload,
) -> Result<Option<UnavailablePinRecoveryAction>, StoreError> {
    let Err(position) = validate_destination_pins(state, tenant, payload) else {
        return Ok(None);
    };
    let candidates = replacement_question_choices(state, tenant)?;
    Ok(Some(
        UnavailablePinRecoveryAction::SelectReplacementQuestion {
            position,
            candidates,
        },
    ))
}

pub(crate) fn replacement_question_choices(
    state: &State,
    tenant: TenantId,
) -> Result<ReplacementQuestionChoices, StoreError> {
    let candidates = state
        .published
        .values()
        .filter(|record| {
            super::super::reusable_curriculum::replacement_candidate_selectable(
                state, tenant, record,
            )
        })
        .map(|record| record.question_id.clone())
        .take(question_model::MAX_ASSIGNMENT_CANDIDATES_PER_SELECTION_GROUP)
        .collect::<Vec<_>>();
    let candidates = ReplacementQuestionChoices::new(candidates)
        .map_err(|error| StoreError::InvalidRecord(error.to_string()))?;
    Ok(candidates)
}

fn semantic_error(error: SemanticPlannerError) -> StoreError {
    StoreError::InvalidRecord(error.to_string())
}

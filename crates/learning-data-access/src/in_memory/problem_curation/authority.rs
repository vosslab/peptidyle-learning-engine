//! Active-session role resolution for the Memory curation conformance model.

use question_model::{ProblemCollectionVisibility, UserRole};

use super::StoredProblemCollection;
use crate::{SessionTokenHash, StoreError, TenantContext, UserId};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(super) enum CurationPrincipal {
    Instructor(UserId),
    Sysadmin(UserId),
}

pub(super) fn curation_principal(
    state: &crate::in_memory::State,
    context: TenantContext,
    session: SessionTokenHash,
) -> Result<CurationPrincipal, StoreError> {
    let subject = crate::in_memory::sessions::active_subject(state, context, session)
        .ok_or(StoreError::NotFound)?;
    // A dual-role session intentionally uses the Instructor path, preserving its personal state.
    if subject.roles().contains(&UserRole::Instructor) {
        match approved_instructor(state, subject.user()) {
            Ok(actor) => return Ok(CurationPrincipal::Instructor(actor)),
            Err(_) if subject.roles().contains(&UserRole::Sysadmin) => {
                return Ok(CurationPrincipal::Sysadmin(subject.user()));
            }
            Err(error) => return Err(error),
        }
    }
    subject
        .roles()
        .contains(&UserRole::Sysadmin)
        .then_some(CurationPrincipal::Sysadmin(subject.user()))
        .ok_or(StoreError::Forbidden)
}

pub(super) fn require_instructor(
    state: &crate::in_memory::State,
    context: TenantContext,
    session: SessionTokenHash,
) -> Result<UserId, StoreError> {
    match curation_principal(state, context, session)? {
        CurationPrincipal::Instructor(actor) => Ok(actor),
        CurationPrincipal::Sysadmin(_) => Err(StoreError::Forbidden),
    }
}

fn approved_instructor(
    state: &crate::in_memory::State,
    actor: UserId,
) -> Result<UserId, StoreError> {
    let approval = state
        .instructor_approvals
        .get(&actor)
        .ok_or(StoreError::Forbidden)?;
    domain::teaching_authority::validate_instructor_approval(
        &approval.approval,
        state.authoritative_time,
    )
    .map_err(|error| {
        StoreError::InvalidRecord(format!("invalid instructor approval: {error:?}"))
    })?;
    (approval.approval.user == actor && approval.approval.revoked_at.is_none())
        .then_some(actor)
        .ok_or(StoreError::Forbidden)
}

pub(super) fn can_read_collection(
    principal: CurationPrincipal,
    collection: &StoredProblemCollection,
) -> bool {
    match principal {
        CurationPrincipal::Instructor(actor) => {
            collection.owner == actor
                || collection.visibility == ProblemCollectionVisibility::Institution
        }
        CurationPrincipal::Sysadmin(_) => {
            collection.visibility == ProblemCollectionVisibility::Institution
        }
    }
}

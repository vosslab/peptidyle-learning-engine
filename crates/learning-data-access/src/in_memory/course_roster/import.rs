//! In-memory staged roster-import transaction.

use std::collections::{BTreeMap, BTreeSet};

use objects::Sha256Digest;
use question_model::{CourseId, TenantId, UserId};

use super::{
    CourseInvitation, CourseInvitationId, CourseInvitationStatus, CourseMemberStatus,
    bump_roster_revision, roster_policy, timestamp_after_seconds,
};
use crate::in_memory::State;
use crate::{
    CommitCourseRosterImport, CommittedCourseRosterImport, CourseRosterImportId,
    CourseRosterImportPreview, CourseRosterImportRow, CourseRosterImportRowInput,
    CourseRosterImportState, RosterIdempotencyKey, RosterImportRowStatus, StageCourseRosterImport,
    StoreError,
};

#[derive(Debug, Clone)]
pub(in crate::in_memory) struct StoredCourseRosterImport {
    pub(super) preview: CourseRosterImportPreview,
    normalized_digest: Sha256Digest,
    stage_idempotency_key: RosterIdempotencyKey,
    normalized_rows: Vec<CourseRosterImportRowInput>,
    commit_idempotency_key: Option<RosterIdempotencyKey>,
    committed: Option<CommittedCourseRosterImport>,
}

pub(super) fn stage(
    state: &mut State,
    tenant: TenantId,
    _actor: UserId,
    command: StageCourseRosterImport,
) -> Result<CourseRosterImportPreview, StoreError> {
    cleanup_expired(state);
    validate_rows(&command.rows)?;
    let current = roster_policy(state, tenant, command.course);
    if current.revision != command.expected_roster_revision {
        return Err(StoreError::Conflict);
    }
    let receipt_key = (tenant, command.course, command.idempotency_key.clone());
    if let Some(import) = state.roster_import_idempotency.get(&receipt_key).copied() {
        let stored = state
            .roster_imports
            .get(&(tenant, command.course, import))
            .ok_or_else(|| {
                StoreError::Unavailable("roster import receipt is inconsistent".to_string())
            })?;
        if stored.normalized_digest == command.normalized_digest
            && stored.normalized_rows == command.rows
            && stored.preview.roster_revision == command.expected_roster_revision
        {
            return Ok(stored.preview.clone());
        }
        return Err(StoreError::Conflict);
    }
    let import = CourseRosterImportId::generate()?;
    let preview = CourseRosterImportPreview {
        id: import,
        course: command.course,
        roster_revision: current.revision,
        revision: crate::RosterImportRevision::INITIAL,
        state: CourseRosterImportState::Preview,
        expires_at: timestamp_after_seconds(
            state.authoritative_time,
            command.lifetime.as_seconds(),
        )?,
        rows: classify_rows(state, tenant, command.course, &command.rows),
    };
    state.roster_import_idempotency.insert(receipt_key, import);
    state.roster_imports.insert(
        (tenant, command.course, import),
        StoredCourseRosterImport {
            preview: preview.clone(),
            normalized_digest: command.normalized_digest,
            stage_idempotency_key: command.idempotency_key,
            normalized_rows: command.rows,
            commit_idempotency_key: None,
            committed: None,
        },
    );
    Ok(preview)
}

pub(super) fn commit(
    state: &mut State,
    tenant: TenantId,
    actor: UserId,
    command: CommitCourseRosterImport,
) -> Result<CommittedCourseRosterImport, StoreError> {
    cleanup_expired(state);
    let key = (tenant, command.course, command.import);
    let stored = state
        .roster_imports
        .get(&key)
        .cloned()
        .ok_or(StoreError::NotFound)?;
    if let Some(committed) = stored.committed {
        return (stored.commit_idempotency_key.as_ref() == Some(&command.idempotency_key))
            .then_some(committed)
            .ok_or(StoreError::Conflict);
    }
    if stored.preview.state != CourseRosterImportState::Preview
        || stored.preview.revision != command.expected_import_revision
        || stored.preview.expires_at <= state.authoritative_time
        || roster_policy(state, tenant, command.course).revision != stored.preview.roster_revision
    {
        return Err(StoreError::Conflict);
    }
    let ready = stored
        .preview
        .rows
        .iter()
        .filter(|row| row.status == RosterImportRowStatus::ReadyToInvite)
        .map(|row| (row.row_number, row))
        .collect::<BTreeMap<_, _>>();
    let bindings = command
        .invitations
        .iter()
        .map(|binding| (binding.row_number, binding))
        .collect::<BTreeMap<_, _>>();
    if bindings.len() != command.invitations.len()
        || ready.keys().copied().collect::<BTreeSet<_>>()
            != bindings.keys().copied().collect::<BTreeSet<_>>()
    {
        return Err(StoreError::InvalidRecord(
            "roster import invitation set does not match ready rows".to_string(),
        ));
    }
    let mut invitations = Vec::with_capacity(ready.len());
    for (row_number, row) in ready {
        let binding = bindings[&row_number];
        let email = row
            .email
            .clone()
            .ok_or_else(|| StoreError::Unavailable("ready roster row has no email".to_string()))?;
        let roster_id = row.roster_id.clone().ok_or_else(|| {
            StoreError::Unavailable("ready roster row has no roster ID".to_string())
        })?;
        if state.invitation_by_hash.contains_key(&binding.token_hash)
            || state.invitation_idempotency.contains_key(&(
                tenant,
                command.course,
                binding.idempotency_key.clone(),
            ))
        {
            return Err(StoreError::Conflict);
        }
        let invitation = CourseInvitation {
            id: CourseInvitationId::generate()?,
            tenant,
            course: command.course,
            email,
            roster_id,
            invited_by: actor,
            status: CourseInvitationStatus::Pending,
            created_at: state.authoritative_time,
            expires_at: timestamp_after_seconds(
                state.authoritative_time,
                binding.lifetime.as_seconds(),
            )?,
            claimed_by: None,
        };
        state
            .invitation_by_hash
            .insert(binding.token_hash, (tenant, command.course, invitation.id));
        state.invitation_idempotency.insert(
            (tenant, command.course, binding.idempotency_key.clone()),
            (invitation.id, binding.token_hash),
        );
        state.course_invitations.insert(
            (tenant, command.course, invitation.id),
            super::StoredCourseInvitation {
                record: invitation.clone(),
            },
        );
        invitations.push((row_number, invitation));
    }
    let roster_revision = bump_roster_revision(
        state,
        tenant,
        command.course,
        Some(stored.preview.roster_revision),
    )?;
    let import_revision = stored.preview.revision.next()?;
    let committed = CommittedCourseRosterImport {
        import: command.import,
        import_revision,
        roster_revision,
        invitations,
    };
    let stored = state
        .roster_imports
        .get_mut(&key)
        .ok_or(StoreError::NotFound)?;
    stored.preview.state = CourseRosterImportState::Committed;
    stored.preview.revision = import_revision;
    stored.commit_idempotency_key = Some(command.idempotency_key);
    stored.committed = Some(committed.clone());
    Ok(committed)
}

fn validate_rows(rows: &[CourseRosterImportRowInput]) -> Result<(), StoreError> {
    if rows.is_empty() || rows.len() > crate::MAX_ROSTER_IMPORT_ROWS {
        return Err(StoreError::InvalidRecord(
            "roster import row count is invalid".to_string(),
        ));
    }
    let mut numbers = BTreeSet::new();
    for row in rows {
        row.validate_shape()?;
        if !numbers.insert(row.row_number) {
            return Err(StoreError::InvalidRecord(
                "roster import row number is duplicated".to_string(),
            ));
        }
    }
    Ok(())
}

fn classify_rows(
    state: &State,
    tenant: TenantId,
    course: CourseId,
    inputs: &[CourseRosterImportRowInput],
) -> Vec<CourseRosterImportRow> {
    let mut email_counts = BTreeMap::new();
    let mut roster_counts = BTreeMap::new();
    for row in inputs {
        if let Some(email) = &row.email {
            *email_counts
                .entry(email.normalized().to_string())
                .or_insert(0_u16) += 1;
        }
        if let Some(roster_id) = &row.roster_id {
            *roster_counts.entry(roster_id.clone()).or_insert(0_u16) += 1;
        }
    }
    let policy = roster_policy(state, tenant, course);
    inputs
        .iter()
        .map(|input| {
            let status = match (&input.email, &input.roster_id) {
                (Some(email), Some(roster_id))
                    if email_counts[email.normalized()] > 1 || roster_counts[roster_id] > 1 =>
                {
                    RosterImportRowStatus::Duplicate
                }
                (Some(email), Some(_)) if !policy.validates(email) => {
                    RosterImportRowStatus::Invalid
                }
                (Some(email), Some(roster_id)) => {
                    classify_valid_row(state, tenant, course, email, roster_id)
                }
                _ => RosterImportRowStatus::Invalid,
            };
            CourseRosterImportRow {
                row_number: input.row_number,
                email: input.email.clone(),
                roster_id: input.roster_id.clone(),
                status,
            }
        })
        .collect()
}

fn classify_valid_row(
    state: &State,
    tenant: TenantId,
    course: CourseId,
    email: &crate::AuthenticationEmail,
    roster_id: &crate::CourseRosterId,
) -> RosterImportRowStatus {
    let matching_members = state
        .roster_members
        .values()
        .filter(|member| {
            member.tenant == tenant
                && member.course == course
                && member.status == CourseMemberStatus::Active
                && (member
                    .roster_email
                    .as_ref()
                    .is_some_and(|stored| stored.normalized() == email.normalized())
                    || member.roster_id.as_ref() == Some(roster_id))
        })
        .collect::<Vec<_>>();
    if !matching_members.is_empty() {
        return if matching_members.len() == 1
            && matching_members[0]
                .roster_email
                .as_ref()
                .is_some_and(|stored| stored.normalized() == email.normalized())
            && matching_members[0].roster_id.as_ref() == Some(roster_id)
        {
            RosterImportRowStatus::AlreadyMember
        } else {
            RosterImportRowStatus::Invalid
        };
    }
    let matching_invitations = state
        .course_invitations
        .values()
        .filter(|stored| {
            stored.record.tenant == tenant
                && stored.record.course == course
                && stored.record.status == CourseInvitationStatus::Pending
                && stored.record.expires_at > state.authoritative_time
                && (stored.record.email.normalized() == email.normalized()
                    || &stored.record.roster_id == roster_id)
        })
        .collect::<Vec<_>>();
    if !matching_invitations.is_empty() {
        return if matching_invitations.len() == 1
            && matching_invitations[0].record.email.normalized() == email.normalized()
            && &matching_invitations[0].record.roster_id == roster_id
        {
            RosterImportRowStatus::AlreadyPending
        } else {
            RosterImportRowStatus::Invalid
        };
    }
    RosterImportRowStatus::ReadyToInvite
}

fn cleanup_expired(state: &mut State) {
    let now = state.authoritative_time;
    let expired = state
        .roster_imports
        .iter()
        .filter(|(_, stored)| {
            stored.preview.state == CourseRosterImportState::Preview
                && stored.preview.expires_at <= now
        })
        .map(|(key, stored)| (*key, stored.stage_idempotency_key.clone()))
        .collect::<Vec<_>>();
    for ((tenant, course, import), idempotency_key) in expired {
        state.roster_imports.remove(&(tenant, course, import));
        state
            .roster_import_idempotency
            .remove(&(tenant, course, idempotency_key));
    }
}

//! In-memory reference implementation of invitation-delivery fencing.

use async_trait::async_trait;

use super::course_roster::{delivery_provenance, require_roster_read_authority};
use super::{MemoryStore, State};
use crate::{
    ClaimedCourseInvitationDelivery, CompleteCourseInvitationDelivery, CourseInvitationDelivery,
    CourseInvitationDeliveryId, CourseInvitationDeliveryLeaseId,
    CourseInvitationDeliveryOutcomeCode, CourseInvitationDeliveryState,
    CourseInvitationDeliveryStore, CourseInvitationDeliveryWorkerStore, CourseInvitationId,
    CourseInvitationStatus, MAX_COURSE_INVITATION_DELIVERY_ATTEMPTS, SessionTokenHash, StoreError,
    TenantContext,
};
use question_model::{ActivityTimestamp, CourseId, TenantId};

pub(super) fn create_pending(
    state: &mut State,
    tenant: TenantId,
    course: CourseId,
    invitation: CourseInvitationId,
) -> Result<CourseInvitationDelivery, StoreError> {
    let key = (tenant, course, invitation);
    if let Some(delivery) = state.invitation_deliveries.get(&key) {
        return Ok(delivery.clone());
    }
    let now = state.authoritative_time;
    let delivery = CourseInvitationDelivery {
        tenant,
        course,
        invitation,
        id: CourseInvitationDeliveryId::generate()?,
        state: CourseInvitationDeliveryState::Pending,
        attempt_count: 0,
        next_attempt_at: now,
        last_attempt_at: None,
        lease: None,
        lease_expires_at: None,
        dispatch_started_at: None,
        outcome_code: None,
        created_at: now,
        updated_at: now,
        accepted_at: None,
        terminal_at: None,
    };
    state.invitation_deliveries.insert(key, delivery.clone());
    Ok(delivery)
}

pub(super) fn cancel_for_invitation(
    state: &mut State,
    tenant: TenantId,
    course: CourseId,
    invitation: CourseInvitationId,
) {
    if let Some(delivery) = state
        .invitation_deliveries
        .get_mut(&(tenant, course, invitation))
        && matches!(
            delivery.state,
            CourseInvitationDeliveryState::Pending | CourseInvitationDeliveryState::RetryableFailed
        )
    {
        let dispatch_started = delivery.dispatch_started_at.is_some();
        delivery.state = if dispatch_started {
            CourseInvitationDeliveryState::Ambiguous
        } else {
            CourseInvitationDeliveryState::Cancelled
        };
        delivery.outcome_code = Some(if dispatch_started {
            CourseInvitationDeliveryOutcomeCode::AmbiguousTransport
        } else {
            CourseInvitationDeliveryOutcomeCode::Cancelled
        });
        delivery.lease = None;
        delivery.lease_expires_at = None;
        delivery.dispatch_started_at = None;
        delivery.updated_at = state.authoritative_time;
        delivery.terminal_at = Some(state.authoritative_time);
    }
}

#[async_trait]
impl CourseInvitationDeliveryStore for MemoryStore {
    async fn course_invitation_delivery_state(
        &self,
        context: TenantContext,
        session: SessionTokenHash,
        course: CourseId,
        invitation: CourseInvitationId,
    ) -> Result<Option<CourseInvitationDeliveryState>, StoreError> {
        let state = self.read_state()?;
        require_roster_read_authority(&state, context, session, course)?;
        Ok(state
            .invitation_deliveries
            .get(&(context.tenant_id(), course, invitation))
            .map(|item| item.state))
    }
}

#[async_trait]
impl CourseInvitationDeliveryWorkerStore for MemoryStore {
    async fn prepare_course_invitation_delivery(
        &self,
        delivery: CourseInvitationDeliveryId,
        lease: CourseInvitationDeliveryLeaseId,
    ) -> Result<Option<crate::PreparedCourseInvitationDelivery>, StoreError> {
        let mut state = self.write_state()?;
        let now = state.authoritative_time;
        let Some(key) = state
            .invitation_deliveries
            .iter()
            .find_map(|(key, record)| {
                (record.id == delivery && record.lease == Some(lease)).then_some(*key)
            })
        else {
            return Ok(None);
        };
        let (tenant, course, invitation_id) = key;
        let active_lease = state
            .invitation_deliveries
            .get(&(tenant, course, invitation_id))
            .is_some_and(|record| {
                record.lease_expires_at.is_some_and(|until| until > now)
                    && record.dispatch_started_at.is_none()
            });
        if !active_lease {
            return Ok(None);
        }
        let invitation = state
            .course_invitations
            .get(&(tenant, course, invitation_id))
            .ok_or(StoreError::NotFound)?;
        if invitation.record.status != CourseInvitationStatus::Pending
            || invitation.record.expires_at <= now
        {
            cancel_for_invitation(&mut state, tenant, course, invitation_id);
            return Ok(None);
        }
        let delivery_email = invitation.record.email.delivery().to_string();
        let roster_id = invitation.record.roster_id.clone();
        let idempotency_key = state
            .invitation_idempotency
            .iter()
            .find_map(|((entry_tenant, entry_course, entry_key), (id, _))| {
                (*entry_tenant == tenant && *entry_course == course && *id == invitation_id)
                    .then_some(entry_key.clone())
            })
            .ok_or_else(|| {
                StoreError::Unavailable(
                    "invitation idempotency receipt is inconsistent".to_string(),
                )
            })?;
        let record = state
            .invitation_deliveries
            .get_mut(&(tenant, course, invitation_id))
            .ok_or(StoreError::NotFound)?;
        record.dispatch_started_at = Some(now);
        let reissuance = match delivery_provenance(&state, tenant, course, invitation_id)? {
            Some((import, row_number, commit_idempotency_key)) => {
                crate::InvitationDeliveryReissuance::Import {
                    tenant,
                    course,
                    import,
                    row_number,
                    commit_idempotency_key,
                }
            }
            None => crate::InvitationDeliveryReissuance::Single {
                tenant,
                course,
                roster_id,
                idempotency_key: idempotency_key.clone(),
            },
        };
        Ok(Some(crate::PreparedCourseInvitationDelivery {
            delivery,
            lease,
            delivery_email,
            expected_token_hash: state
                .invitation_idempotency
                .get(&(tenant, course, idempotency_key.clone()))
                .ok_or(StoreError::NotFound)?
                .1,
            reissuance,
        }))
    }
    async fn claim_due_course_invitation_deliveries(
        &self,
        maximum: u16,
        lease_duration_seconds: u32,
    ) -> Result<Vec<ClaimedCourseInvitationDelivery>, StoreError> {
        if maximum == 0
            || maximum > 100
            || lease_duration_seconds == 0
            || lease_duration_seconds > 900
        {
            return Err(StoreError::InvalidRecord(
                "invitation delivery claim arguments are invalid".to_string(),
            ));
        }
        let mut state = self.write_state()?;
        let now = state.authoritative_time;
        let expiry = now
            .as_unix_millis()
            .checked_add(i64::from(lease_duration_seconds) * 1000)
            .map(ActivityTimestamp::from_unix_millis)
            .ok_or(StoreError::Conflict)?;
        let expired = state
            .invitation_deliveries
            .iter()
            .filter_map(|(key, delivery)| {
                let invitation = state.course_invitations.get(key)?;
                (invitation.record.status == CourseInvitationStatus::Pending
                    && invitation.record.expires_at <= now
                    && matches!(
                        delivery.state,
                        CourseInvitationDeliveryState::Pending
                            | CourseInvitationDeliveryState::RetryableFailed
                    ))
                .then_some(*key)
            })
            .collect::<Vec<_>>();
        for (tenant, course, invitation) in expired {
            cancel_for_invitation(&mut state, tenant, course, invitation);
        }
        let expired_leases = state
            .invitation_deliveries
            .iter()
            .filter_map(|(key, delivery)| {
                (matches!(
                    delivery.state,
                    CourseInvitationDeliveryState::Pending
                        | CourseInvitationDeliveryState::RetryableFailed
                ) && delivery.lease_expires_at.is_some_and(|until| until <= now))
                .then_some(*key)
            })
            .collect::<Vec<_>>();
        for key in expired_leases {
            let delivery = state
                .invitation_deliveries
                .get_mut(&key)
                .ok_or(StoreError::NotFound)?;
            if delivery.dispatch_started_at.is_some() {
                delivery.state = CourseInvitationDeliveryState::Ambiguous;
                delivery.outcome_code =
                    Some(CourseInvitationDeliveryOutcomeCode::AmbiguousTransport);
                delivery.terminal_at = Some(now);
            }
            delivery.lease = None;
            delivery.lease_expires_at = None;
            delivery.dispatch_started_at = None;
            delivery.updated_at = now;
        }
        let exhausted = state
            .invitation_deliveries
            .iter()
            .filter_map(|(key, delivery)| {
                (matches!(
                    delivery.state,
                    CourseInvitationDeliveryState::Pending
                        | CourseInvitationDeliveryState::RetryableFailed
                ) && delivery.attempt_count >= MAX_COURSE_INVITATION_DELIVERY_ATTEMPTS
                    && delivery.lease.is_none())
                .then_some(*key)
            })
            .collect::<Vec<_>>();
        for key in exhausted {
            let delivery = state
                .invitation_deliveries
                .get_mut(&key)
                .ok_or(StoreError::NotFound)?;
            delivery.state = CourseInvitationDeliveryState::PermanentFailed;
            delivery.outcome_code = Some(CourseInvitationDeliveryOutcomeCode::PermanentFailure);
            delivery.terminal_at = Some(now);
            delivery.updated_at = now;
        }
        let keys = state
            .invitation_deliveries
            .iter()
            .filter_map(|(key, delivery)| {
                let invitation = state.course_invitations.get(key)?;
                let eligible = invitation.record.status == CourseInvitationStatus::Pending
                    && invitation.record.expires_at > now
                    && matches!(
                        delivery.state,
                        CourseInvitationDeliveryState::Pending
                            | CourseInvitationDeliveryState::RetryableFailed
                    )
                    && delivery.next_attempt_at <= now
                    && delivery.attempt_count < MAX_COURSE_INVITATION_DELIVERY_ATTEMPTS
                    && delivery
                        .lease_expires_at
                        .is_none_or(|leased_until| leased_until <= now);
                eligible.then_some(*key)
            })
            .take(usize::from(maximum))
            .collect::<Vec<_>>();
        let mut claimed = Vec::with_capacity(keys.len());
        for key in keys {
            let lease = CourseInvitationDeliveryLeaseId::generate()?;
            let delivery = state
                .invitation_deliveries
                .get_mut(&key)
                .ok_or(StoreError::NotFound)?;
            delivery.lease = Some(lease);
            delivery.lease_expires_at = Some(expiry);
            delivery.dispatch_started_at = None;
            delivery.attempt_count = delivery
                .attempt_count
                .checked_add(1)
                .ok_or(StoreError::Conflict)?;
            delivery.last_attempt_at = Some(now);
            delivery.updated_at = now;
            claimed.push(ClaimedCourseInvitationDelivery {
                delivery: delivery.clone(),
                lease,
            });
        }
        Ok(claimed)
    }

    async fn complete_course_invitation_delivery(
        &self,
        delivery_id: CourseInvitationDeliveryId,
        lease: CourseInvitationDeliveryLeaseId,
        completion: CompleteCourseInvitationDelivery,
    ) -> Result<bool, StoreError> {
        let mut state = self.write_state()?;
        let now = state.authoritative_time;
        let key = state
            .invitation_deliveries
            .iter()
            .find_map(|(key, delivery)| (delivery.id == delivery_id).then_some(*key));
        let Some(key) = key else {
            return Ok(false);
        };
        let invitation = state
            .course_invitations
            .get(&key)
            .ok_or(StoreError::NotFound)?;
        let active = invitation.record.status == CourseInvitationStatus::Pending
            && invitation.record.expires_at > now;
        let record = state
            .invitation_deliveries
            .get_mut(&key)
            .ok_or(StoreError::NotFound)?;
        if record.lease != Some(lease)
            || record.lease_expires_at.is_none_or(|until| until <= now)
            || record.dispatch_started_at.is_none()
        {
            return Ok(false);
        }
        record.lease = None;
        record.lease_expires_at = None;
        record.dispatch_started_at = None;
        record.updated_at = now;
        if !active {
            record.state = CourseInvitationDeliveryState::Ambiguous;
            record.outcome_code = Some(CourseInvitationDeliveryOutcomeCode::AmbiguousTransport);
            record.terminal_at = Some(now);
            return Ok(true);
        }
        match completion {
            CompleteCourseInvitationDelivery::AcceptedByProvider => {
                record.state = CourseInvitationDeliveryState::AcceptedByProvider;
                record.outcome_code = Some(CourseInvitationDeliveryOutcomeCode::Accepted);
                record.accepted_at = Some(now);
                record.terminal_at = Some(now);
            }
            CompleteCourseInvitationDelivery::RetryableFailed { next_attempt_at } => {
                if record.attempt_count >= MAX_COURSE_INVITATION_DELIVERY_ATTEMPTS {
                    record.state = CourseInvitationDeliveryState::PermanentFailed;
                    record.outcome_code =
                        Some(CourseInvitationDeliveryOutcomeCode::PermanentFailure);
                    record.terminal_at = Some(now);
                } else {
                    record.state = CourseInvitationDeliveryState::RetryableFailed;
                    record.outcome_code =
                        Some(CourseInvitationDeliveryOutcomeCode::TemporaryFailure);
                    record.next_attempt_at = next_attempt_at;
                }
            }
            CompleteCourseInvitationDelivery::Ambiguous => {
                record.state = CourseInvitationDeliveryState::Ambiguous;
                record.outcome_code = Some(CourseInvitationDeliveryOutcomeCode::AmbiguousTransport);
                record.terminal_at = Some(now);
            }
            CompleteCourseInvitationDelivery::PermanentFailed => {
                record.state = CourseInvitationDeliveryState::PermanentFailed;
                record.outcome_code = Some(CourseInvitationDeliveryOutcomeCode::PermanentFailure);
                record.terminal_at = Some(now);
            }
        }
        Ok(true)
    }

    async fn revalidate_course_invitation_delivery_lease(
        &self,
        delivery: CourseInvitationDeliveryId,
        lease: CourseInvitationDeliveryLeaseId,
    ) -> Result<bool, StoreError> {
        let state = self.read_state()?;
        Ok(state.invitation_deliveries.iter().any(|(key, record)| {
            record.id == delivery
                && record.lease == Some(lease)
                && record
                    .lease_expires_at
                    .is_some_and(|until| until > state.authoritative_time)
                && record.dispatch_started_at.is_some()
                && state.course_invitations.get(key).is_some_and(|invitation| {
                    invitation.record.status == CourseInvitationStatus::Pending
                        && invitation.record.expires_at > state.authoritative_time
                })
        }))
    }
}

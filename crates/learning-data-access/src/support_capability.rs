//! Exact-course Sysadmin support-capability issuance and revocation.

use async_trait::async_trait;
use question_model::{AccountReference, CourseInstanceReference, Timestamp};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{CourseRosterEntry, SessionTokenHash, StoreError};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SupportOperationKind {
    CourseRosterSupport,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SupportMinimumProjection {
    CourseRoster,
}

/// Course, operation, projection, and expiry are server derived.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct IssueSupportCapabilityInput {
    pub sysadmin_reference: AccountReference,
    pub purpose: String,
}

impl IssueSupportCapabilityInput {
    pub fn validate(&self) -> Result<(), StoreError> {
        if self.purpose != self.purpose.trim()
            || !(1..=1_000).contains(&self.purpose.chars().count())
        {
            return Err(StoreError::InvalidRecord(
                "Support purpose is invalid".to_string(),
            ));
        }
        Ok(())
    }
}

/// Safe receipt; it contains no roster, membership, email, or private ID data.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SupportCapabilityReceipt {
    pub capability_id: Uuid,
    pub course_reference: CourseInstanceReference,
    pub sysadmin_reference: AccountReference,
    pub operation_kind: SupportOperationKind,
    pub minimum_projection: SupportMinimumProjection,
    pub purpose: String,
    pub expires_at: Timestamp,
    pub revoked_at: Option<Timestamp>,
}

#[async_trait]
pub trait SupportCapabilityStore: Send + Sync {
    async fn issue_course_roster_support(
        &self,
        token: SessionTokenHash,
        course: CourseInstanceReference,
        input: IssueSupportCapabilityInput,
    ) -> Result<SupportCapabilityReceipt, StoreError>;
    async fn revoke_course_roster_support(
        &self,
        token: SessionTokenHash,
        course: CourseInstanceReference,
        capability_id: Uuid,
    ) -> Result<SupportCapabilityReceipt, StoreError>;
    async fn read_course_roster_support(
        &self,
        token: SessionTokenHash,
        capability_id: Uuid,
    ) -> Result<Vec<CourseRosterEntry>, StoreError>;
}

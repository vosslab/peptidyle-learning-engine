//! Sysadmin-authorized Instructor Account management records.

use async_trait::async_trait;
use question_model::{AccountReference, Timestamp};
use serde::{Deserialize, Serialize};

use crate::{SessionTokenHash, StoreError};

/// The current lifecycle state of an Instructor Account.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum InstructorAccountState {
    /// The Instructor Account can authenticate and use its established authority.
    Active,
    /// The Instructor Account remains durable but cannot authenticate.
    Deactivated,
    /// A terminal Account State that this projection never creates.
    Closed,
}

/// Browser-safe Instructor Accounts row. It intentionally omits email, credentials,
/// Course, Student Record, and any other FERPA-bearing relation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InstructorAccountSummary {
    /// Canonical opaque Account Reference.
    pub reference: AccountReference,
    /// Current Account State derived from the immutable event history.
    pub state: InstructorAccountState,
    /// Most recent successful credential verification/session creation, if any.
    pub last_successful_sign_in: Option<Timestamp>,
}

/// Normalized email supplied only to Create Instructor Account.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CreateInstructorAccountInput {
    /// Normalized Instructor Authentication Email; never returned by this Store.
    pub normalized_email: String,
}

impl CreateInstructorAccountInput {
    /// Matches the existing database Create Instructor Account normalization contract.
    pub fn validate(&self) -> Result<(), StoreError> {
        let email = &self.normalized_email;
        if !(3..=320).contains(&email.len()) || email != &email.trim().to_lowercase() {
            return Err(StoreError::InvalidRecord(
                "Instructor Authentication Email is invalid".to_string(),
            ));
        }
        Ok(())
    }
}

/// Required human-readable reason for Deactivate Instructor Account.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DeactivateInstructorAccountInput {
    /// Bounded local reason retained in the immutable Account State Event.
    pub reason: String,
}

impl DeactivateInstructorAccountInput {
    /// Rejects only the established Account State Event reason shapes.
    pub fn validate(&self) -> Result<(), StoreError> {
        let reason = self.reason.trim();
        if reason.len() > 1_000 || reason.is_empty() || reason != self.reason {
            return Err(StoreError::InvalidRecord(
                "Deactivate Instructor Account reason is invalid".to_string(),
            ));
        }
        Ok(())
    }
}

/// Sysadmin-only Store boundary for Instructor Accounts.
#[async_trait]
pub trait InstructorAccountStore: Send + Sync {
    /// Lists only the intentional browser-safe Instructor Account projection.
    async fn list_instructor_accounts(
        &self,
        session_token_hash: SessionTokenHash,
    ) -> Result<Vec<InstructorAccountSummary>, StoreError>;

    /// Creates one Active Instructor Account using the established atomic procedure.
    async fn create_instructor_account(
        &self,
        session_token_hash: SessionTokenHash,
        input: CreateInstructorAccountInput,
    ) -> Result<InstructorAccountSummary, StoreError>;

    /// Appends a Deactivated Account State Event and revokes active sessions.
    async fn deactivate_instructor_account(
        &self,
        session_token_hash: SessionTokenHash,
        reference: AccountReference,
        input: DeactivateInstructorAccountInput,
    ) -> Result<InstructorAccountSummary, StoreError>;

    /// Appends an Active Account State Event for an existing deactivated Instructor Account.
    async fn reactivate_instructor_account(
        &self,
        session_token_hash: SessionTokenHash,
        reference: AccountReference,
    ) -> Result<InstructorAccountSummary, StoreError>;
}

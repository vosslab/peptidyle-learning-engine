//! Sysadmin-authorized Instructor Account management records.

use async_trait::async_trait;
use question_model::{AccountId, AccountTimeZone, Timestamp};
use serde::{Deserialize, Serialize};

use crate::{
    AuthenticationEmail, NumberedPageRequest, PageNumber, PageSize, ProvidedAvatarId,
    SessionTokenHash, StoreError,
};

/// The current lifecycle state of an Instructor Account.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
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
    /// Canonical Account ID.
    pub id: AccountId,
    /// Current Account State derived from the immutable event history.
    pub state: InstructorAccountState,
    /// Most recent successful credential verification/session creation, if any.
    pub last_successful_sign_in: Option<Timestamp>,
    /// Cross-account projection permits only a static provided-avatar ID.
    /// Generic and private Profile-image choices both remain absent here.
    pub provided_avatar_id: Option<ProvidedAvatarId>,
}

/// One numbered page of closed Instructor Account rows with the authenticated Sysadmin's display
/// context and the independent state-facet counts.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InstructorAccountList {
    /// The intentional browser-safe target-account rows in the resolved page.
    pub items: Vec<InstructorAccountSummary>,
    /// One-based requested page after clamping to the matching result set.
    pub page_number: PageNumber,
    /// The validated requested page size.
    pub page_size: PageSize,
    /// Number of matching rows before page slicing.
    pub total_count: u64,
    /// Exact self-owned IANA zone for the authenticated Sysadmin viewer.
    pub display_time_zone: AccountTimeZone,
    pub state_choices: Vec<InstructorAccountStateChoice>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InstructorAccountStateChoice {
    pub key: InstructorAccountState,
    pub count: i64,
}

/// Sysadmin list constraints for one numbered page of the complete matching collection.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstructorAccountBrowse {
    /// Exact Account ID/email, or a display-name substring; empty lists all matches.
    pub query: Option<String>,
    /// One Account State, or every state when absent.
    pub state: Option<InstructorAccountState>,
    pub sort: InstructorAccountSort,
    /// One-based page request with the ordinary 100-row maximum.
    pub page: NumberedPageRequest,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum InstructorAccountSort {
    IdAscending,
    IdDescending,
}

/// Account record created by a Sysadmin after outside vetting.
///
/// The delivery destination stays server-only so API responses never disclose
/// Authentication Email.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CreatedInstructorAccount {
    pub account: InstructorAccountSummary,
    pub setup_email_destination: AuthenticationEmail,
}

/// Account setup supplied only to Create Instructor Account.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CreateInstructorAccountInput {
    /// Normalized Instructor Authentication Email; never returned by this Store.
    pub normalized_email: String,
    pub first_name: String,
    pub last_name: String,
    pub affiliation: String,
}

impl CreateInstructorAccountInput {
    /// Matches the existing database Create Instructor Account normalization contract.
    pub fn validate(&self) -> Result<(), StoreError> {
        let email = AuthenticationEmail::parse(&self.normalized_email).map_err(|_| {
            StoreError::InvalidRecord("Instructor Authentication Email is invalid".to_string())
        })?;
        if email.normalized() != self.normalized_email {
            return Err(StoreError::InvalidRecord(
                "Instructor Authentication Email is invalid".to_string(),
            ));
        }
        for (label, value, maximum) in [
            ("first name", &self.first_name, 100usize),
            ("last name", &self.last_name, 100usize),
            ("affiliation", &self.affiliation, 300usize),
        ] {
            if value != value.trim()
                || value.is_empty()
                || value.chars().count() > maximum
                || value.chars().any(char::is_control)
            {
                return Err(StoreError::InvalidRecord(format!(
                    "Instructor {label} is invalid"
                )));
            }
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
    /// Lists browser-safe rows with only the authenticated Sysadmin's display zone.
    ///
    /// A non-empty query matches an Instructor Account ID or Authentication Email
    /// exactly, or a Profile display name by case-insensitive substring. State keeps
    /// one Account State. The result is filtered and ordered before the requested
    /// numbered page is sliced; the returned rows still omit email and name.
    async fn list_instructor_accounts(
        &self,
        session_token_hash: SessionTokenHash,
        browse: InstructorAccountBrowse,
    ) -> Result<InstructorAccountList, StoreError>;

    /// Creates one Active Instructor Account using the established atomic procedure.
    async fn create_instructor_account(
        &self,
        session_token_hash: SessionTokenHash,
        input: CreateInstructorAccountInput,
    ) -> Result<CreatedInstructorAccount, StoreError>;

    /// Finds the private setup-email destination for one current Instructor.
    /// Only the installed active Sysadmin session can request it.
    async fn instructor_setup_email_destination(
        &self,
        session_token_hash: SessionTokenHash,
        account_id: AccountId,
    ) -> Result<AuthenticationEmail, StoreError>;

    /// Appends a Deactivated Account State Event and revokes active sessions.
    async fn deactivate_instructor_account(
        &self,
        session_token_hash: SessionTokenHash,
        account_id: AccountId,
        input: DeactivateInstructorAccountInput,
    ) -> Result<InstructorAccountSummary, StoreError>;

    /// Appends an Active Account State Event for an existing deactivated Instructor Account.
    async fn reactivate_instructor_account(
        &self,
        session_token_hash: SessionTokenHash,
        account_id: AccountId,
    ) -> Result<InstructorAccountSummary, StoreError>;
}

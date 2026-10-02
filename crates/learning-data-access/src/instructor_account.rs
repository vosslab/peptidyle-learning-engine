//! Sysadmin-authorized Instructor Account management records.

use async_trait::async_trait;
use question_model::{AccountId, AccountTimeZone, Timestamp};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{ProvidedAvatarId, SessionTokenHash, StoreError};

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

/// Closed Instructor Account rows with the authenticated Sysadmin's display context.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InstructorAccountList {
    /// The intentional browser-safe target-account rows.
    pub accounts: Vec<InstructorAccountSummary>,
    /// Exact self-owned IANA zone for the authenticated Sysadmin viewer.
    pub display_time_zone: AccountTimeZone,
    /// Account ID of the last row when another row exists past this page.
    pub next_cursor: Option<AccountId>,
}

/// Sysadmin list constraints. A page size scans one server page; None returns the filtered list.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstructorAccountBrowse {
    /// Name or email. Empty and missing both list without a name match.
    pub query: Option<String>,
    /// One Account State, or every state when absent.
    pub state: Option<InstructorAccountState>,
    /// Return rows after this Account ID. The first page has no cursor.
    pub after_account_id: Option<AccountId>,
    /// 50, 100, or 250. None is the complete filtered list.
    pub page_size: Option<i32>,
}

/// Normalized email supplied only to Create Instructor Account.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CreateInstructorAccountInput {
    /// Normalized Instructor Authentication Email; never returned by this Store.
    pub normalized_email: String,
    /// Immutable completed identity check for this exact normalized email.
    ///
    /// The Store derives the approving Sysadmin from the authenticated session;
    /// this opaque ID only binds that completed decision to the candidate.
    pub vetting_decision_id: InstructorIdentityVettingDecisionId,
}

/// Opaque durable ID for an immutable completed Instructor identity check.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct InstructorIdentityVettingDecisionId(Uuid);

impl InstructorIdentityVettingDecisionId {
    /// Reconstitutes the private audit identity returned by the trusted Store.
    pub fn from_uuid(value: Uuid) -> Self {
        Self(value)
    }

    /// Returns the opaque audit identity for a later trusted Store operation.
    pub fn as_uuid(self) -> Uuid {
        self.0
    }
}

/// Candidate identity supplied only to record completed human vetting.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CompleteInstructorIdentityVettingInput {
    /// Exact normalized private Authentication Email for the vetted candidate.
    pub normalized_email: String,
    /// The Sysadmin-verified, immutable display identity for a later narrow
    /// Instructor-only endorsement projection.  It is not an Account or
    /// Profile field and is never returned by this vetting boundary.
    pub verified_instructor_display_name: String,
}

impl CompleteInstructorIdentityVettingInput {
    /// Accepts only the canonical lookup form; the Store never accepts an actor or role.
    pub fn validate(&self) -> Result<(), StoreError> {
        let email = &self.normalized_email;
        if !(3..=320).contains(&email.len()) || email != &email.trim().to_lowercase() {
            return Err(StoreError::InvalidRecord(
                "Instructor identity vetting email is invalid".to_string(),
            ));
        }
        let display_name = &self.verified_instructor_display_name;
        if display_name != display_name.trim()
            || !(1..=200).contains(&display_name.chars().count())
            || display_name.chars().any(char::is_control)
        {
            return Err(StoreError::InvalidRecord(
                "Verified Instructor display name is invalid".to_string(),
            ));
        }
        Ok(())
    }
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
    /// Records one immutable completed identity-vetting fact for a candidate.
    ///
    /// The authenticated Sysadmin is derived from `session_token_hash`; callers
    /// cannot provide an approving Account or User Role.
    async fn complete_instructor_identity_vetting(
        &self,
        session_token_hash: SessionTokenHash,
        input: CompleteInstructorIdentityVettingInput,
    ) -> Result<InstructorIdentityVettingDecisionId, StoreError>;

    /// Lists browser-safe rows with only the authenticated Sysadmin's display zone.
    ///
    /// A non-empty query matches an Instructor authentication email exactly or a
    /// vetted display name by case-insensitive substring. State keeps one Account
    /// State. Page size 50, 100, or 250 returns at most that many rows after the
    /// previous Account ID. The returned rows still omit the email and the name.
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
    ) -> Result<InstructorAccountSummary, StoreError>;

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

#[cfg(test)]
mod tests {
    use super::{CreateInstructorAccountInput, InstructorIdentityVettingDecisionId};

    #[test]
    fn create_instructor_account_requires_a_completed_vetting_decision() {
        let missing = serde_json::from_str::<CreateInstructorAccountInput>(
            r#"{"normalizedEmail":"ada@university.edu"}"#,
        );
        assert!(
            missing.is_err(),
            "creation without a vetting decision must fail"
        );

        let decision = "11111111-1111-4111-8111-111111111111";
        let input: CreateInstructorAccountInput = serde_json::from_str(&format!(
            r#"{{"normalizedEmail":"ada@university.edu","vettingDecisionId":"{decision}"}}"#
        ))
        .expect("vetted creation input");
        input.validate().expect("normalized email is acceptable");
        assert_eq!(
            input.vetting_decision_id,
            InstructorIdentityVettingDecisionId::from_uuid(
                uuid::Uuid::parse_str(decision).expect("decision id")
            )
        );

        let unnormalized: CreateInstructorAccountInput = serde_json::from_str(&format!(
            r#"{{"normalizedEmail":"Ada@University.EDU","vettingDecisionId":"{decision}"}}"#
        ))
        .expect("field shape is valid");
        assert!(unnormalized.validate().is_err());
    }
}

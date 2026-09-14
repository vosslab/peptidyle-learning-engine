//! Authenticated Account time-zone preference boundary.

use async_trait::async_trait;
use question_model::AccountTimeZone;

use crate::{SessionTokenHash, StoreError};

/// Narrow authenticated-self read used by trusted scheduling and display paths.
#[async_trait]
pub trait AccountTimeZoneStore: Send + Sync {
    /// Loads the caller's Account-owned exact IANA zone; no Account ID is accepted.
    async fn authenticated_account_time_zone(
        &self,
        session_token_hash: SessionTokenHash,
    ) -> Result<AccountTimeZone, StoreError>;

    /// Loads only an active Student caller's Account-owned display zone.
    async fn authenticated_student_time_zone(
        &self,
        session_token_hash: SessionTokenHash,
    ) -> Result<AccountTimeZone, StoreError>;

    /// Replaces only an active Student caller's own display zone.
    async fn update_authenticated_student_time_zone(
        &self,
        session_token_hash: SessionTokenHash,
        time_zone: AccountTimeZone,
    ) -> Result<AccountTimeZone, StoreError>;
}

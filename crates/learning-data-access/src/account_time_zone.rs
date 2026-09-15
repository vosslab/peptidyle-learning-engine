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

    /// Replaces only the authenticated Account's own exact IANA preference.
    ///
    /// The subject is always derived from the installed server-side session;
    /// this boundary intentionally accepts no Account or role identifier.
    async fn update_authenticated_account_time_zone(
        &self,
        session_token_hash: SessionTokenHash,
        time_zone: AccountTimeZone,
    ) -> Result<AccountTimeZone, StoreError>;
}

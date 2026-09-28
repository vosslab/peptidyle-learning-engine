//! Authenticated Account appearance-preference persistence boundary.

use async_trait::async_trait;
use question_model::{DisplayMode, Theme};

use crate::{SessionTokenHash, StoreError};

/// The preferences that resolve global Account appearance.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AccountAppearance {
    /// NULL storage follows the browser's current color preference.
    pub display_mode_preference: Option<DisplayMode>,
    /// Present only for an authenticated Instructor Account.
    pub personal_theme: Option<Theme>,
}

/// Session-authorized current Account appearance persistence.
#[async_trait]
pub trait AccountAppearanceStore: Send + Sync {
    /// Reads only the authenticated Account's own appearance state.
    async fn authenticated_account_appearance(
        &self,
        session_token_hash: SessionTokenHash,
    ) -> Result<AccountAppearance, StoreError>;

    /// Sets or clears only the authenticated Account's display-mode preference.
    async fn update_authenticated_account_display_mode_preference(
        &self,
        session_token_hash: SessionTokenHash,
        display_mode_preference: Option<DisplayMode>,
    ) -> Result<Option<DisplayMode>, StoreError>;

    /// Sets only the authenticated Instructor Account's personal Theme.
    async fn update_authenticated_instructor_personal_theme(
        &self,
        session_token_hash: SessionTokenHash,
        theme: Theme,
    ) -> Result<Theme, StoreError>;
}

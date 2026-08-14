//! Account-owned presentation preferences.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::{AccountSessionTokenHash, StoreError};

/// User-selected contrast treatment. Standard preserves the selected course palette.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum PresentationContrast {
    #[default]
    Standard,
    Increased,
}

/// Presentation preferences contain no course content and apply across account sessions.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AccountPresentationPreference {
    pub contrast: PresentationContrast,
}

#[async_trait]
pub trait AccountPresentationStore: Send + Sync {
    async fn account_presentation(
        &self,
        session_token_hash: AccountSessionTokenHash,
    ) -> Result<AccountPresentationPreference, StoreError>;

    async fn save_account_presentation(
        &self,
        session_token_hash: AccountSessionTokenHash,
        preference: AccountPresentationPreference,
    ) -> Result<AccountPresentationPreference, StoreError>;
}

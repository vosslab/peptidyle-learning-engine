use super::MemoryStore;
use crate::{
    AccountPresentationPreference, AccountPresentationStore, AccountSessionTokenHash, StoreError,
};
use async_trait::async_trait;

#[async_trait]
impl AccountPresentationStore for MemoryStore {
    async fn account_presentation(
        &self,
        session_token_hash: AccountSessionTokenHash,
    ) -> Result<AccountPresentationPreference, StoreError> {
        let state = self.read_state()?;
        let user = state
            .account_sessions
            .get(&session_token_hash)
            .filter(|session| session.expires_at > state.authoritative_time)
            .map(|session| session.user)
            .ok_or(StoreError::NotFound)?;
        Ok(state
            .account_presentation
            .get(&user)
            .copied()
            .unwrap_or_default())
    }

    async fn save_account_presentation(
        &self,
        session_token_hash: AccountSessionTokenHash,
        preference: AccountPresentationPreference,
    ) -> Result<AccountPresentationPreference, StoreError> {
        let mut state = self.write_state()?;
        let user = state
            .account_sessions
            .get(&session_token_hash)
            .filter(|session| session.expires_at > state.authoritative_time)
            .map(|session| session.user)
            .ok_or(StoreError::NotFound)?;
        state.account_presentation.insert(user, preference);
        Ok(preference)
    }
}

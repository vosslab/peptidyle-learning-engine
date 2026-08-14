use async_trait::async_trait;
use sqlx::Row;

use super::{PostgresStore, map_sqlx_error};
use crate::{
    AccountPresentationPreference, AccountPresentationStore, AccountSessionTokenHash,
    PresentationContrast, StoreError,
};

#[async_trait]
impl AccountPresentationStore for PostgresStore {
    async fn account_presentation(
        &self,
        session_token_hash: AccountSessionTokenHash,
    ) -> Result<AccountPresentationPreference, StoreError> {
        let mut transaction = self.begin_auth().await?;
        let contrast = sqlx::query("SELECT ple_account_presentation_get($1) AS contrast")
            .bind(session_token_hash.as_bytes().to_vec())
            .fetch_one(&mut *transaction)
            .await
            .map_err(map_sqlx_error)?
            .try_get::<Option<String>, _>("contrast")
            .map_err(map_sqlx_error)?
            .ok_or(StoreError::NotFound)?;
        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(AccountPresentationPreference {
            contrast: decode_contrast(&contrast)?,
        })
    }

    async fn save_account_presentation(
        &self,
        session_token_hash: AccountSessionTokenHash,
        preference: AccountPresentationPreference,
    ) -> Result<AccountPresentationPreference, StoreError> {
        let mut transaction = self.begin_auth().await?;
        let contrast = sqlx::query("SELECT ple_account_presentation_save($1, $2) AS contrast")
            .bind(session_token_hash.as_bytes().to_vec())
            .bind(encode_contrast(preference.contrast))
            .fetch_one(&mut *transaction)
            .await
            .map_err(map_sqlx_error)?
            .try_get::<Option<String>, _>("contrast")
            .map_err(map_sqlx_error)?
            .ok_or(StoreError::NotFound)?;
        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(AccountPresentationPreference {
            contrast: decode_contrast(&contrast)?,
        })
    }
}

fn encode_contrast(value: PresentationContrast) -> &'static str {
    match value {
        PresentationContrast::Standard => "standard",
        PresentationContrast::Increased => "increased",
    }
}

fn decode_contrast(value: &str) -> Result<PresentationContrast, StoreError> {
    match value {
        "standard" => Ok(PresentationContrast::Standard),
        "increased" => Ok(PresentationContrast::Increased),
        _ => Err(StoreError::Unavailable(
            "stored presentation contrast is invalid".to_string(),
        )),
    }
}

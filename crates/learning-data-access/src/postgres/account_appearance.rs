//! PostgreSQL persistence for authenticated Account appearance preferences.

use async_trait::async_trait;
use question_model::{DisplayMode, Theme};
use sqlx::{Postgres, Row, Transaction};

use super::{Pool, connection::map_sqlx_error};
use crate::{AccountAppearance, AccountAppearanceStore, SessionTokenHash, StoreError};

/// PostgreSQL Store for Account appearance derived from an installed session.
#[derive(Clone)]
pub struct PostgresAccountAppearanceStore {
    pool: Pool,
}

impl PostgresAccountAppearanceStore {
    /// Binds the attested API pool to self-only appearance procedures.
    pub fn new(pool: Pool) -> Self {
        Self { pool }
    }

    async fn begin(
        &self,
        token: SessionTokenHash,
    ) -> Result<Transaction<'_, Postgres>, StoreError> {
        let mut tx = self.pool.begin().await.map_err(map_sqlx_error)?;
        sqlx::query("SET LOCAL ROLE ple_auth")
            .execute(&mut *tx)
            .await
            .map_err(map_sqlx_error)?;
        let session = sqlx::query(
            "SELECT session_id FROM ple_api.resolve_and_install_session(decode($1, 'hex'))",
        )
        .bind(token.to_string())
        .fetch_optional(&mut *tx)
        .await
        .map_err(map_sqlx_error)?;
        if session.is_none() {
            return Err(StoreError::Forbidden);
        }
        sqlx::query("SET LOCAL ROLE ple_app")
            .execute(&mut *tx)
            .await
            .map_err(map_sqlx_error)?;
        Ok(tx)
    }

    fn display_mode(value: Option<String>) -> Result<Option<DisplayMode>, StoreError> {
        value
            .map(|value| match value.as_str() {
                "light" => Ok(DisplayMode::Light),
                "dark" => Ok(DisplayMode::Dark),
                _ => Err(StoreError::InvalidRecord(
                    "database returned an invalid display-mode preference".to_string(),
                )),
            })
            .transpose()
    }

    fn theme(value: Option<String>) -> Result<Option<Theme>, StoreError> {
        value
            .map(|value| {
                value.parse().map_err(|_| {
                    StoreError::InvalidRecord(
                        "database returned an invalid personal Theme".to_string(),
                    )
                })
            })
            .transpose()
    }
}

#[async_trait]
impl AccountAppearanceStore for PostgresAccountAppearanceStore {
    async fn authenticated_account_appearance(
        &self,
        token: SessionTokenHash,
    ) -> Result<AccountAppearance, StoreError> {
        let mut tx = self.begin(token).await?;
        // ASVS 1.2.4, 2.2.1--2.2.2, 8.2.2, and 8.3.1: the closed result is
        // read through a parameter-free procedure whose subject comes only
        // from the installed authenticated session.
        let row = sqlx::query(
            "SELECT display_mode_preference, personal_theme FROM ple_api.current_account_appearance()",
        )
        .fetch_optional(&mut *tx)
        .await
        .map_err(map_sqlx_error)?
        .ok_or(StoreError::NotFound)?;
        let appearance = AccountAppearance {
            display_mode_preference: Self::display_mode(
                row.try_get("display_mode_preference")
                    .map_err(map_sqlx_error)?,
            )?,
            personal_theme: Self::theme(row.try_get("personal_theme").map_err(map_sqlx_error)?)?,
        };
        tx.commit().await.map_err(map_sqlx_error)?;
        Ok(appearance)
    }

    async fn update_authenticated_account_display_mode_preference(
        &self,
        token: SessionTokenHash,
        display_mode_preference: Option<DisplayMode>,
    ) -> Result<Option<DisplayMode>, StoreError> {
        let mut tx = self.begin(token).await?;
        // ASVS 1.2.4 and 2.2.1--2.2.2: bind the closed optional value while
        // PostgreSQL independently derives the Account from the installed session.
        let value: Option<String> = display_mode_preference.map(|mode| match mode {
            DisplayMode::Light => "light".to_string(),
            DisplayMode::Dark => "dark".to_string(),
        });
        let saved = Self::display_mode(
            sqlx::query_scalar("SELECT ple_api.update_current_account_display_mode_preference($1)")
                .bind(value)
                .fetch_one(&mut *tx)
                .await
                .map_err(map_sqlx_error)?,
        )?;
        tx.commit().await.map_err(map_sqlx_error)?;
        Ok(saved)
    }

    async fn update_authenticated_instructor_personal_theme(
        &self,
        token: SessionTokenHash,
        theme: Theme,
    ) -> Result<Theme, StoreError> {
        let mut tx = self.begin(token).await?;
        // ASVS 1.2.4 and 2.2.1--2.2.2: bind a closed Theme value; the SQL
        // procedure derives and verifies the active Instructor itself.
        let saved: String =
            sqlx::query_scalar("SELECT ple_api.update_current_instructor_personal_theme($1)")
                .bind(theme.as_str())
                .fetch_one(&mut *tx)
                .await
                .map_err(map_sqlx_error)?;
        let saved = Self::theme(Some(saved))?.ok_or_else(|| {
            StoreError::InvalidRecord("database returned no personal Theme".to_string())
        })?;
        tx.commit().await.map_err(map_sqlx_error)?;
        Ok(saved)
    }
}

//! PostgreSQL persistence for Sysadmin Instructor Account management.

use async_trait::async_trait;
use question_model::{AccountReference, Timestamp};
use sqlx::{Postgres, Row, Transaction};

use super::{Pool, connection::map_sqlx_error};
use crate::{
    CreateInstructorAccountInput, DeactivateInstructorAccountInput, InstructorAccountState,
    InstructorAccountStore, InstructorAccountSummary, SessionTokenHash, StoreError,
};

/// PostgreSQL Store for the deliberate Sysadmin-only Instructor Accounts surface.
#[derive(Clone)]
pub struct PostgresInstructorAccountStore {
    pool: Pool,
}

impl PostgresInstructorAccountStore {
    /// Binds the attested API pool to M16 procedures.
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
}

#[async_trait]
impl InstructorAccountStore for PostgresInstructorAccountStore {
    async fn list_instructor_accounts(
        &self,
        token: SessionTokenHash,
    ) -> Result<Vec<InstructorAccountSummary>, StoreError> {
        let mut tx = self.begin(token).await?;
        let rows = sqlx::query(
            "SELECT reference_number, state, \
             (extract(epoch FROM last_successful_sign_in) * 1000)::bigint \
             AS last_successful_sign_in_millis \
             FROM ple_api.list_live_demo_instructor_accounts()",
        )
        .fetch_all(&mut *tx)
        .await
        .map_err(map_sqlx_error)?;
        let records = rows
            .iter()
            .map(decode_summary)
            .collect::<Result<Vec<_>, _>>()?;
        tx.commit().await.map_err(map_sqlx_error)?;
        Ok(records)
    }

    async fn create_instructor_account(
        &self,
        token: SessionTokenHash,
        input: CreateInstructorAccountInput,
    ) -> Result<InstructorAccountSummary, StoreError> {
        input.validate()?;
        let mut tx = self.begin(token).await?;
        let row = sqlx::query(
            "SELECT reference_number, state, \
             (extract(epoch FROM last_successful_sign_in) * 1000)::bigint \
             AS last_successful_sign_in_millis \
             FROM ple_api.create_live_demo_instructor_account($1)",
        )
        .bind(input.normalized_email)
        .fetch_one(&mut *tx)
        .await
        .map_err(map_sqlx_error)?;
        let record = decode_summary(&row)?;
        tx.commit().await.map_err(map_sqlx_error)?;
        Ok(record)
    }

    async fn deactivate_instructor_account(
        &self,
        token: SessionTokenHash,
        reference: AccountReference,
        input: DeactivateInstructorAccountInput,
    ) -> Result<InstructorAccountSummary, StoreError> {
        input.validate()?;
        self.change_state(token, reference, "deactivated", Some(input.reason))
            .await
    }

    async fn reactivate_instructor_account(
        &self,
        token: SessionTokenHash,
        reference: AccountReference,
    ) -> Result<InstructorAccountSummary, StoreError> {
        self.change_state(token, reference, "active", None).await
    }
}

impl PostgresInstructorAccountStore {
    async fn change_state(
        &self,
        token: SessionTokenHash,
        reference: AccountReference,
        state: &'static str,
        reason: Option<String>,
    ) -> Result<InstructorAccountSummary, StoreError> {
        let mut tx = self.begin(token).await?;
        let row = sqlx::query(
            "SELECT reference_number, state, \
             (extract(epoch FROM last_successful_sign_in) * 1000)::bigint \
             AS last_successful_sign_in_millis \
             FROM ple_api.change_live_demo_instructor_account_state($1, $2, $3)",
        )
        .bind(i64::from(reference.number()))
        .bind(state)
        .bind(reason)
        .fetch_optional(&mut *tx)
        .await
        .map_err(map_sqlx_error)?;
        let record = row
            .as_ref()
            .map(decode_summary)
            .transpose()?
            .ok_or(StoreError::NotFound)?;
        tx.commit().await.map_err(map_sqlx_error)?;
        Ok(record)
    }
}

fn decode_summary(row: &sqlx::postgres::PgRow) -> Result<InstructorAccountSummary, StoreError> {
    let reference_number: i64 = row.try_get("reference_number").map_err(map_sqlx_error)?;
    let reference = u64::try_from(reference_number)
        .ok()
        .and_then(AccountReference::new)
        .ok_or_else(|| invalid("Account Reference"))?;
    let state = match row
        .try_get::<String, _>("state")
        .map_err(map_sqlx_error)?
        .as_str()
    {
        "active" => InstructorAccountState::Active,
        "deactivated" => InstructorAccountState::Deactivated,
        "closed" => InstructorAccountState::Closed,
        _ => return Err(invalid("Account State")),
    };
    let last_successful_sign_in = row
        .try_get::<Option<i64>, _>("last_successful_sign_in_millis")
        .map_err(map_sqlx_error)?
        .map(Timestamp::from_unix_millis);
    Ok(InstructorAccountSummary {
        reference,
        state,
        last_successful_sign_in,
    })
}

fn invalid(label: &str) -> StoreError {
    StoreError::InvalidRecord(format!("database returned an invalid {label}"))
}

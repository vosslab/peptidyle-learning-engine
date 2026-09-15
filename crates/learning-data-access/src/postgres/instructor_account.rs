//! PostgreSQL persistence for Sysadmin Instructor Account management.

use async_trait::async_trait;
use question_model::{AccountReference, AccountTimeZone, Timestamp};
use sqlx::{Postgres, Row, Transaction};

use super::{Pool, connection::map_sqlx_error};
use crate::{
    CompleteInstructorIdentityVettingInput, CreateInstructorAccountInput,
    DeactivateInstructorAccountInput, InstructorAccountList, InstructorAccountState,
    InstructorAccountStore, InstructorAccountSummary, InstructorIdentityVettingDecisionReference,
    ProvidedAvatarId, SessionTokenHash, StoreError,
};

/// PostgreSQL Store for the deliberate Sysadmin-only Instructor Accounts surface.
#[derive(Clone)]
pub struct PostgresInstructorAccountStore {
    pool: Pool,
}

impl PostgresInstructorAccountStore {
    /// Binds the attested API pool to Instructor Account procedures.
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
    async fn complete_instructor_identity_vetting(
        &self,
        token: SessionTokenHash,
        input: CompleteInstructorIdentityVettingInput,
    ) -> Result<InstructorIdentityVettingDecisionReference, StoreError> {
        input.validate()?;
        let mut tx = self.begin(token).await?;
        // ASVS 8.2.1 and 8.3.1: PostgreSQL derives the active Sysadmin from
        // the installed session; this adapter supplies no actor or role field.
        let decision_id: uuid::Uuid =
            sqlx::query_scalar("SELECT ple_api.complete_instructor_identity_vetting($1, $2)")
                .bind(input.normalized_email)
                .bind(input.verified_instructor_display_name)
                .fetch_one(&mut *tx)
                .await
                .map_err(map_sqlx_error)?;
        tx.commit().await.map_err(map_sqlx_error)?;
        Ok(InstructorIdentityVettingDecisionReference::from_uuid(
            decision_id,
        ))
    }

    async fn list_instructor_accounts(
        &self,
        token: SessionTokenHash,
    ) -> Result<InstructorAccountList, StoreError> {
        let mut tx = self.begin(token).await?;
        let rows = sqlx::query(
            "SELECT public_reference, state, provided_avatar_id, \
             (extract(epoch FROM last_successful_sign_in) * 1000)::bigint \
             AS last_successful_sign_in_millis \
             FROM ple_api.list_instructor_account_avatar_summaries()",
        )
        .fetch_all(&mut *tx)
        .await
        .map_err(map_sqlx_error)?;
        let records = rows
            .iter()
            .map(decode_summary)
            .collect::<Result<Vec<_>, _>>()?;
        // ASVS 8.2.2 and 8.3.1: this function derives the display preference
        // from the installed session rather than accepting a target Account ID.
        let display_time_zone =
            sqlx::query_scalar::<_, Option<String>>("SELECT ple_api.current_account_time_zone()")
                .fetch_one(&mut *tx)
                .await
                .map_err(map_sqlx_error)?
                .ok_or(StoreError::NotFound)
                .and_then(|value| {
                    AccountTimeZone::parse(&value).map_err(|_| {
                        StoreError::InvalidRecord(
                            "database returned an invalid Account time zone".to_string(),
                        )
                    })
                })?;
        tx.commit().await.map_err(map_sqlx_error)?;
        Ok(InstructorAccountList {
            accounts: records,
            display_time_zone,
        })
    }

    async fn create_instructor_account(
        &self,
        token: SessionTokenHash,
        input: CreateInstructorAccountInput,
    ) -> Result<InstructorAccountSummary, StoreError> {
        input.validate()?;
        let mut tx = self.begin(token).await?;
        let public_reference = sqlx::query_scalar(
            "SELECT public_reference \
             FROM ple_api.create_instructor_account($1, $2)",
        )
        .bind(input.normalized_email)
        .bind(input.vetting_decision_reference.as_uuid())
        .fetch_one(&mut *tx)
        .await
        .map_err(map_sqlx_error)?;
        let record = summary_for_reference(&mut tx, public_reference)
            .await?
            .ok_or(StoreError::NotFound)?;
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
        let public_reference = sqlx::query_scalar(
            "SELECT public_reference \
             FROM ple_api.change_instructor_account_state($1, $2, $3)",
        )
        .bind(reference.as_string())
        .bind(state)
        .bind(reason)
        .fetch_optional(&mut *tx)
        .await
        .map_err(map_sqlx_error)?;
        let public_reference = public_reference.ok_or(StoreError::NotFound)?;
        let record = summary_for_reference(&mut tx, public_reference)
            .await?
            .ok_or(StoreError::NotFound)?;
        tx.commit().await.map_err(map_sqlx_error)?;
        Ok(record)
    }
}

async fn summary_for_reference(
    tx: &mut Transaction<'_, Postgres>,
    public_reference: String,
) -> Result<Option<InstructorAccountSummary>, StoreError> {
    let row = sqlx::query(
        "SELECT public_reference, state, provided_avatar_id, \
         (extract(epoch FROM last_successful_sign_in) * 1000)::bigint \
         AS last_successful_sign_in_millis \
         FROM ple_api.list_instructor_account_avatar_summaries() \
         WHERE public_reference = $1",
    )
    .bind(public_reference)
    .fetch_optional(&mut **tx)
    .await
    .map_err(map_sqlx_error)?;
    row.as_ref().map(decode_summary).transpose()
}

fn decode_summary(row: &sqlx::postgres::PgRow) -> Result<InstructorAccountSummary, StoreError> {
    let reference = AccountReference::new(
        row.try_get::<String, _>("public_reference")
            .map_err(map_sqlx_error)?,
    )
    .map_err(|_| invalid("Account Reference"))?;
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
    let provided_avatar_id = row
        .try_get::<Option<String>, _>("provided_avatar_id")
        .map_err(map_sqlx_error)?
        .map(ProvidedAvatarId::parse)
        .transpose()?;
    Ok(InstructorAccountSummary {
        reference,
        state,
        last_successful_sign_in,
        provided_avatar_id,
    })
}

fn invalid(label: &str) -> StoreError {
    StoreError::InvalidRecord(format!("database returned an invalid {label}"))
}

//! PostgreSQL persistence for Sysadmin Instructor Account management.

use async_trait::async_trait;
use question_model::{AccountId, AccountTimeZone, Timestamp};
use sqlx::{Postgres, Row, Transaction};

use super::{Pool, connection::map_sqlx_error};
use crate::{
    AuthenticationEmail, CreateInstructorAccountInput, CreatedInstructorAccount,
    DeactivateInstructorAccountInput, InstructorAccountBrowse, InstructorAccountList,
    InstructorAccountState, InstructorAccountStore, InstructorAccountSummary, ProvidedAvatarId,
    SessionTokenHash, StoreError,
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
    async fn list_instructor_accounts(
        &self,
        token: SessionTokenHash,
        browse: InstructorAccountBrowse,
    ) -> Result<InstructorAccountList, StoreError> {
        let query = normalize_find_query(browse.query.as_deref())?;
        let page_size = normalize_page_size(browse.page_size)?;
        let state = browse.state.map(state_name);
        let after_account_id = browse.after_account_id.map(|id| id.as_string());
        let mut tx = self.begin(token).await?;
        let rows = sqlx::query(
            "SELECT account_id, state, provided_avatar_id, \
             (extract(epoch FROM last_successful_sign_in) * 1000)::bigint \
             AS last_successful_sign_in_millis \
             FROM ple_api.list_instructor_account_avatar_summaries($1, $2, $3, $4)",
        )
        .bind(query)
        .bind(state)
        .bind(after_account_id)
        .bind(page_size)
        .fetch_all(&mut *tx)
        .await
        .map_err(map_sqlx_error)?;
        let mut records = rows
            .iter()
            .map(decode_summary)
            .collect::<Result<Vec<_>, _>>()?;
        let next_cursor = match page_size {
            Some(limit) => {
                let limit =
                    usize::try_from(limit).map_err(|_| invalid("Instructor Account list"))?;
                if records.len() > limit {
                    records.truncate(limit);
                    records.last().map(|account| account.id.clone())
                } else {
                    None
                }
            }
            None => None,
        };
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
            next_cursor,
        })
    }

    async fn create_instructor_account(
        &self,
        token: SessionTokenHash,
        input: CreateInstructorAccountInput,
    ) -> Result<CreatedInstructorAccount, StoreError> {
        input.validate()?;
        let mut tx = self.begin(token).await?;
        let account_id = sqlx::query_scalar(
            "SELECT account_id \
             FROM ple_api.create_instructor_account($1, $2, $3, $4)",
        )
        .bind(input.normalized_email)
        .bind(input.first_name)
        .bind(input.last_name)
        .bind(input.affiliation)
        .fetch_one(&mut *tx)
        .await
        .map_err(map_sqlx_error)?;
        let record = summary_for_account(&mut tx, account_id)
            .await?
            .ok_or(StoreError::NotFound)?;
        let delivery_email = sqlx::query_scalar::<_, String>(
            "SELECT ple_api.instructor_setup_email_destination($1)",
        )
        .bind(record.id.as_string())
        .fetch_one(&mut *tx)
        .await
        .map_err(map_sqlx_error)?;
        let setup_email_destination = AuthenticationEmail::parse(&delivery_email)
            .map_err(|_| invalid("Instructor Authentication Email"))?;
        tx.commit().await.map_err(map_sqlx_error)?;
        Ok(CreatedInstructorAccount {
            account: record,
            setup_email_destination,
        })
    }

    async fn instructor_setup_email_destination(
        &self,
        token: SessionTokenHash,
        account_id: AccountId,
    ) -> Result<AuthenticationEmail, StoreError> {
        let mut tx = self.begin(token).await?;
        let delivery_email = sqlx::query_scalar::<_, String>(
            "SELECT ple_api.instructor_setup_email_destination($1)",
        )
        .bind(account_id.as_string())
        .fetch_optional(&mut *tx)
        .await
        .map_err(map_sqlx_error)?
        .ok_or(StoreError::NotFound)?;
        let destination = AuthenticationEmail::parse(&delivery_email)
            .map_err(|_| invalid("Instructor Authentication Email"))?;
        tx.commit().await.map_err(map_sqlx_error)?;
        Ok(destination)
    }

    async fn deactivate_instructor_account(
        &self,
        token: SessionTokenHash,
        id: AccountId,
        input: DeactivateInstructorAccountInput,
    ) -> Result<InstructorAccountSummary, StoreError> {
        input.validate()?;
        self.change_state(token, id, "deactivated", Some(input.reason))
            .await
    }

    async fn reactivate_instructor_account(
        &self,
        token: SessionTokenHash,
        id: AccountId,
    ) -> Result<InstructorAccountSummary, StoreError> {
        self.change_state(token, id, "active", None).await
    }
}

impl PostgresInstructorAccountStore {
    async fn change_state(
        &self,
        token: SessionTokenHash,
        id: AccountId,
        state: &'static str,
        reason: Option<String>,
    ) -> Result<InstructorAccountSummary, StoreError> {
        let mut tx = self.begin(token).await?;
        let account_id = sqlx::query_scalar(
            "SELECT account_id \
             FROM ple_api.change_instructor_account_state($1, $2, $3)",
        )
        .bind(id.as_string())
        .bind(state)
        .bind(reason)
        .fetch_optional(&mut *tx)
        .await
        .map_err(map_sqlx_error)?;
        let account_id = account_id.ok_or(StoreError::NotFound)?;
        let record = summary_for_account(&mut tx, account_id)
            .await?
            .ok_or(StoreError::NotFound)?;
        tx.commit().await.map_err(map_sqlx_error)?;
        Ok(record)
    }
}

async fn summary_for_account(
    tx: &mut Transaction<'_, Postgres>,
    account_id: String,
) -> Result<Option<InstructorAccountSummary>, StoreError> {
    let row = sqlx::query(
        "SELECT account_id, state, provided_avatar_id, \
         (extract(epoch FROM last_successful_sign_in) * 1000)::bigint \
         AS last_successful_sign_in_millis \
         FROM ple_api.list_instructor_account_avatar_summaries() \
         WHERE account_id = $1",
    )
    .bind(account_id)
    .fetch_optional(&mut **tx)
    .await
    .map_err(map_sqlx_error)?;
    row.as_ref().map(decode_summary).transpose()
}

fn state_name(state: InstructorAccountState) -> &'static str {
    match state {
        InstructorAccountState::Active => "active",
        InstructorAccountState::Deactivated => "deactivated",
        InstructorAccountState::Closed => "closed",
    }
}

fn normalize_page_size(page_size: Option<i32>) -> Result<Option<i32>, StoreError> {
    match page_size {
        None | Some(50 | 100 | 250) => Ok(page_size),
        Some(_) => Err(invalid("Instructor Account list")),
    }
}

fn normalize_find_query(query: Option<&str>) -> Result<Option<String>, StoreError> {
    let Some(query) = query else {
        return Ok(None);
    };
    let trimmed = query.trim();
    if trimmed.is_empty() {
        return Ok(None);
    }
    if trimmed.chars().count() > 320 || trimmed.chars().any(char::is_control) {
        return Err(invalid("Instructor Account search"));
    }
    Ok(Some(trimmed.to_string()))
}

fn decode_summary(row: &sqlx::postgres::PgRow) -> Result<InstructorAccountSummary, StoreError> {
    let account_id = AccountId::new(
        row.try_get::<String, _>("account_id")
            .map_err(map_sqlx_error)?,
    )
    .map_err(|_| invalid("Account ID"))?;
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
        id: account_id,
        state,
        last_successful_sign_in,
        provided_avatar_id,
    })
}

fn invalid(label: &str) -> StoreError {
    StoreError::InvalidRecord(format!("database returned an invalid {label}"))
}

//! PostgreSQL persistence for browser-bound email-code sign-in ceremonies.

use async_trait::async_trait;
use question_model::UserRole;
use sqlx::{Postgres, Row, Transaction};

use super::{
    Pool,
    connection::{map_sqlx_error, parse_account_id},
};
use crate::authentication_ceremony::{EmailAuthenticationStart, PreparedEmailAuthentication};
use crate::{
    AuthenticatedAccount, AuthenticationCeremonyLifetime, AuthenticationCeremonyStore,
    AuthenticationEmail, AuthenticationSecretHash, EmailAuthenticationChallengeId,
    EmailAuthenticationCode, StoreError,
};

/// Dedicated least-privilege adapter for email-code start and completion.
#[derive(Clone)]
pub struct PostgresAuthenticationCeremonyStore {
    pool: Pool,
}

impl PostgresAuthenticationCeremonyStore {
    /// Binds the attested API pool to the authentication-ceremony procedures.
    pub fn new(pool: Pool) -> Self {
        Self { pool }
    }

    async fn begin(&self) -> Result<Transaction<'_, Postgres>, StoreError> {
        let mut transaction = self.pool.begin().await.map_err(map_sqlx_error)?;
        sqlx::query("SET LOCAL ROLE ple_auth")
            .execute(&mut *transaction)
            .await
            .map_err(map_sqlx_error)?;
        Ok(transaction)
    }
}

#[async_trait]
impl AuthenticationCeremonyStore for PostgresAuthenticationCeremonyStore {
    async fn start_email_authentication_challenge(
        &self,
        email: AuthenticationEmail,
        _: EmailAuthenticationCode,
        _: AuthenticationSecretHash,
        _: AuthenticationCeremonyLifetime,
    ) -> Result<EmailAuthenticationStart, StoreError> {
        let mut transaction = self.begin().await?;
        let row = sqlx::query(
            "SELECT delivery_email, eligible FROM ple_api.prepare_email_authentication_challenge($1)",
        )
        .bind(email.normalized())
        .fetch_one(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;
        transaction.commit().await.map_err(map_sqlx_error)?;
        let eligible = row.try_get::<bool, _>("eligible").map_err(map_sqlx_error)?;
        let destination = row
            .try_get::<Option<String>, _>("delivery_email")
            .map_err(map_sqlx_error)?;
        match (eligible, destination) {
            (true, Some(destination)) => AuthenticationEmail::parse(&destination)
                .map(|destination| {
                    EmailAuthenticationStart::Eligible(PreparedEmailAuthentication { destination })
                })
                .map_err(|_| {
                    StoreError::Unavailable(
                        "database returned an invalid authentication email".to_string(),
                    )
                }),
            (false, Some(destination)) => AuthenticationEmail::parse(&destination)
                .map(|destination| {
                    EmailAuthenticationStart::Covered(PreparedEmailAuthentication { destination })
                })
                .map_err(|_| {
                    StoreError::Unavailable(
                        "database returned an invalid authentication email".to_string(),
                    )
                }),
            (false, None) => Ok(EmailAuthenticationStart::RateLimited),
            (true, None) => Err(StoreError::Unavailable(
                "database marked an email delivery eligible without a destination".to_string(),
            )),
        }
    }

    async fn commit_email_authentication_challenge(
        &self,
        challenge: EmailAuthenticationChallengeId,
        email: AuthenticationEmail,
        proof_hash: AuthenticationSecretHash,
        browser_binding_hash: AuthenticationSecretHash,
        lifetime: AuthenticationCeremonyLifetime,
    ) -> Result<bool, StoreError> {
        let mut transaction = self.begin().await?;
        let committed = sqlx::query_scalar::<_, bool>(
            "SELECT ple_api.commit_email_authentication_challenge(\
                $1, $2, decode($3, 'hex'), decode($4, 'hex'), $5\
             )",
        )
        .bind(challenge.as_uuid())
        .bind(email.normalized())
        .bind(hex(&proof_hash.as_bytes()))
        .bind(hex(&browser_binding_hash.as_bytes()))
        .bind(i64::from(lifetime.as_seconds()))
        .fetch_one(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;
        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(committed)
    }

    async fn consume_email_authentication_challenge(
        &self,
        challenge: EmailAuthenticationChallengeId,
        proof_hash: AuthenticationSecretHash,
        browser_binding_hash: AuthenticationSecretHash,
    ) -> Result<Option<AuthenticatedAccount>, StoreError> {
        let mut transaction = self.begin().await?;
        let row = sqlx::query(
            "SELECT account_id, user_role \
             FROM ple_api.consume_email_authentication_challenge(\
                $1, decode($2, 'hex'), decode($3, 'hex')\
             )",
        )
        .bind(challenge.as_uuid())
        .bind(hex(&proof_hash.as_bytes()))
        .bind(hex(&browser_binding_hash.as_bytes()))
        .fetch_optional(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;
        transaction.commit().await.map_err(map_sqlx_error)?;
        row.map(|row| {
            let account = parse_account_id(row.try_get("account_id").map_err(map_sqlx_error)?)?;
            let user_role = match row
                .try_get::<String, _>("user_role")
                .map_err(map_sqlx_error)?
                .as_str()
            {
                "student" => UserRole::Student,
                "instructor" => UserRole::Instructor,
                _ => {
                    return Err(StoreError::Unavailable(
                        "database returned an invalid email-code role".to_string(),
                    ));
                }
            };
            Ok(AuthenticatedAccount { account, user_role })
        })
        .transpose()
    }

    async fn authenticate_passkey(
        &self,
        _ceremony: crate::PasskeyCeremonyId,
        _credential_id_hash: AuthenticationSecretHash,
        _browser_binding_hash: AuthenticationSecretHash,
    ) -> Result<Option<AuthenticatedAccount>, StoreError> {
        Err(StoreError::Unavailable(
            "passkey authentication is not implemented by the email ceremony store".to_string(),
        ))
    }
}

fn hex(value: &[u8; 32]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut output = String::with_capacity(64);
    for byte in value {
        output.push(HEX[usize::from(byte >> 4)] as char);
        output.push(HEX[usize::from(byte & 0x0f)] as char);
    }
    output
}

//! PostgreSQL implementation of atomic reusable Question Pool creation.

use async_trait::async_trait;
use question_model::QuestionPoolId;
use sqlx::{Postgres, Row, Transaction};

use super::{Pool, connection::map_sqlx_error};
use crate::{
    CreateQuestionPoolError, CreateQuestionPoolInput, CreatedQuestionPool,
    QuestionPoolCreationStore, SessionTokenHash, StoreError,
};

const QUESTION_POOL_PUBLIC_ID_UNIQUE: &str = "question_pool_public_question_pool_id_key";
const PUBLIC_ID_COLLISION_SQLSTATE: &str = "QP001";

/// PostgreSQL Store for the one trusted Pool-create capability.
#[derive(Clone)]
pub struct PostgresQuestionPoolCreationStore {
    pool: Pool,
}

impl PostgresQuestionPoolCreationStore {
    /// Binds the attested API pool to the narrow Pool-create procedure.
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
impl QuestionPoolCreationStore for PostgresQuestionPoolCreationStore {
    async fn create_question_pool(
        &self,
        session_token_hash: SessionTokenHash,
        input: CreateQuestionPoolInput,
    ) -> Result<CreatedQuestionPool, CreateQuestionPoolError> {
        input.validate().map_err(CreateQuestionPoolError::Store)?;
        let member_question_ids = input
            .members
            .iter()
            .map(|member| member.published_question_id.as_str().to_owned())
            .collect::<Vec<_>>();
        let member_revision_numbers = input
            .members
            .iter()
            .map(|member| i32::try_from(member.revision_number.get()))
            .collect::<Result<Vec<_>, _>>()
            .map_err(|_| {
                CreateQuestionPoolError::Store(StoreError::InvalidRecord(
                    "Question Revision Number exceeds PostgreSQL integer".to_owned(),
                ))
            })?;
        let mut tx = self
            .begin(session_token_hash)
            .await
            .map_err(CreateQuestionPoolError::Store)?;
        let row = sqlx::query(
            "SELECT question_pool_id, question_pool_edit_number \
             FROM ple_api.create_question_pool($1, $2, $3, $4, $5, $6)",
        )
        .bind(input.question_pool_id.as_str())
        .bind(member_question_ids)
        .bind(member_revision_numbers)
        .bind(input.interchangeability_attested)
        // ASVS 1.2.4: Pool text remains query data, never SQL source.
        .bind(input.title)
        .bind(input.description)
        .fetch_one(&mut *tx)
        .await
        .map_err(map_create_question_pool_error)?;
        let question_pool_id = row
            .try_get::<String, _>("question_pool_id")
            .map_err(map_sqlx_error)
            .and_then(|value| {
                value.parse::<QuestionPoolId>().map_err(|_| {
                    StoreError::InvalidRecord(
                        "Question Pool creation returned an invalid public identity".to_owned(),
                    )
                })
            })
            .map_err(CreateQuestionPoolError::Store)?;
        if question_pool_id != input.question_pool_id {
            return Err(CreateQuestionPoolError::Store(StoreError::InvalidRecord(
                "Question Pool creation returned an unexpected public identity".to_owned(),
            )));
        }
        let edit_number = row
            .try_get::<i64, _>("question_pool_edit_number")
            .map_err(map_sqlx_error)
            .and_then(|value| {
                u64::try_from(value).map_err(|_| {
                    StoreError::InvalidRecord(
                        "Question Pool creation returned an invalid Edit Number".to_owned(),
                    )
                })
            })
            .map_err(CreateQuestionPoolError::Store)?;
        if edit_number != 1 {
            return Err(CreateQuestionPoolError::Store(StoreError::InvalidRecord(
                "Question Pool creation must return Edit Number 1".to_owned(),
            )));
        }
        tx.commit()
            .await
            .map_err(map_sqlx_error)
            .map_err(CreateQuestionPoolError::Store)?;
        Ok(CreatedQuestionPool {
            question_pool_id,
            edit_number,
        })
    }
}

fn map_create_question_pool_error(error: sqlx::Error) -> CreateQuestionPoolError {
    if let sqlx::Error::Database(database_error) = &error
        && is_question_pool_identity_collision(
            database_error.code().as_deref(),
            database_error.constraint(),
        )
    {
        return CreateQuestionPoolError::IdentityCollision;
    }
    if let sqlx::Error::Database(database_error) = &error
        && database_error.code().as_deref() == Some("23505")
    {
        return CreateQuestionPoolError::Store(StoreError::InvalidRecord(
            "Question Pool creation violates a database uniqueness invariant".to_owned(),
        ));
    }
    CreateQuestionPoolError::Store(map_sqlx_error(error))
}

fn is_question_pool_identity_collision(code: Option<&str>, constraint: Option<&str>) -> bool {
    code == Some(PUBLIC_ID_COLLISION_SQLSTATE)
        || (code == Some("23505") && constraint == Some(QUESTION_POOL_PUBLIC_ID_UNIQUE))
}

#[cfg(test)]
mod tests {
    use super::is_question_pool_identity_collision;

    #[test]
    fn recognizes_only_the_shared_public_id_collision_signal() {
        assert!(is_question_pool_identity_collision(Some("QP001"), None));
        assert!(is_question_pool_identity_collision(
            Some("23505"),
            Some("question_pool_public_question_pool_id_key"),
        ));
        assert!(!is_question_pool_identity_collision(
            Some("23505"),
            Some("unrelated_unique_constraint"),
        ));
        assert!(!is_question_pool_identity_collision(Some("23503"), None));
    }
}

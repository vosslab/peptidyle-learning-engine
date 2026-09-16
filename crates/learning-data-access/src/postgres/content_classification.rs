//! Installed-session PostgreSQL reads for the global classification vocabulary.

use async_trait::async_trait;
use sqlx::Row;
use uuid::Uuid;

use super::{Pool, connection::map_sqlx_error};
use crate::{ContentClassificationItem, ContentClassificationStore, SessionTokenHash, StoreError};

#[derive(Clone)]
pub struct PostgresContentClassificationStore {
    pool: Pool,
}

impl PostgresContentClassificationStore {
    pub fn new(pool: Pool) -> Self {
        Self { pool }
    }

    async fn read(
        &self,
        token: SessionTokenHash,
        query: &'static str,
        parent: Option<Uuid>,
    ) -> Result<Vec<ContentClassificationItem>, StoreError> {
        let mut tx = self.pool.begin().await.map_err(map_sqlx_error)?;
        sqlx::query("SET LOCAL ROLE ple_auth")
            .execute(&mut *tx)
            .await
            .map_err(map_sqlx_error)?;
        // ASVS 8.3.1: install current database session authority, never caller roles.
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
        // ASVS 1.2.4: only fixed queries and bound UUID values reach SQL.
        let query = sqlx::query(query);
        let query = if let Some(parent) = parent {
            query.bind(parent)
        } else {
            query
        };
        let rows = query.fetch_all(&mut *tx).await.map_err(map_sqlx_error)?;
        let items = rows
            .iter()
            .map(|row| {
                Ok(ContentClassificationItem {
                    uuid: row.try_get("uuid").map_err(map_sqlx_error)?,
                    name: row.try_get("name").map_err(map_sqlx_error)?,
                })
            })
            .collect::<Result<Vec<_>, StoreError>>()?;
        tx.commit().await.map_err(map_sqlx_error)?;
        Ok(items)
    }
}

#[async_trait]
impl ContentClassificationStore for PostgresContentClassificationStore {
    async fn list_disciplines(
        &self,
        token: SessionTokenHash,
    ) -> Result<Vec<ContentClassificationItem>, StoreError> {
        self.read(
            token,
            "SELECT discipline_uuid AS uuid, name FROM ple_api.list_content_disciplines()",
            None,
        )
        .await
    }
    async fn list_subjects(
        &self,
        token: SessionTokenHash,
        discipline_uuid: Uuid,
    ) -> Result<Vec<ContentClassificationItem>, StoreError> {
        self.read(
            token,
            "SELECT subject_uuid AS uuid, name FROM ple_api.list_content_subjects($1)",
            Some(discipline_uuid),
        )
        .await
    }
    async fn list_topics(
        &self,
        token: SessionTokenHash,
        subject_uuid: Uuid,
    ) -> Result<Vec<ContentClassificationItem>, StoreError> {
        self.read(
            token,
            "SELECT topic_uuid AS uuid, name FROM ple_api.list_content_topics($1)",
            Some(subject_uuid),
        )
        .await
    }
    async fn list_subtopics(
        &self,
        token: SessionTokenHash,
        topic_uuid: Uuid,
    ) -> Result<Vec<ContentClassificationItem>, StoreError> {
        self.read(
            token,
            "SELECT subtopic_uuid AS uuid, name FROM ple_api.list_content_subtopics($1)",
            Some(topic_uuid),
        )
        .await
    }
}

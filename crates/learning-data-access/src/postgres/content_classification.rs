//! Installed-session PostgreSQL reads for the global classification vocabulary.

use async_trait::async_trait;
use sqlx::{Postgres, Row, Transaction};
use uuid::Uuid;

use super::{Pool, connection::map_sqlx_error};
use crate::{
    ContentClassificationItem, ContentClassificationStore, ContentDiscipline,
    ContentDisciplineAdministrationStore, ContentDisciplineDiscoveryStore, SessionTokenHash,
    StoreError,
};

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
        id_column: &'static str,
        parent: Option<Uuid>,
    ) -> Result<Vec<ContentClassificationItem>, StoreError> {
        let mut tx = self.begin(token).await?;
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
            .map(|row| decode_classification_item(row, id_column))
            .collect::<Result<Vec<_>, StoreError>>()?;
        tx.commit().await.map_err(map_sqlx_error)?;
        Ok(items)
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
        Ok(tx)
    }

    async fn discipline_for_uuid(
        tx: &mut Transaction<'_, Postgres>,
        discipline_uuid: Uuid,
    ) -> Result<ContentDiscipline, StoreError> {
        let row = sqlx::query(
            "SELECT content_discipline_id, name, is_retired \
             FROM ple_api.get_content_discipline($1)",
        )
        .bind(discipline_uuid)
        .fetch_optional(&mut **tx)
        .await
        .map_err(map_sqlx_error)?
        .ok_or(StoreError::NotFound)?;
        decode_discipline(&row)
    }

    async fn change_discipline(
        &self,
        token: SessionTokenHash,
        discipline_uuid: Uuid,
        statement: &'static str,
        name: Option<String>,
    ) -> Result<ContentDiscipline, StoreError> {
        let mut tx = self.begin(token).await?;
        let query = sqlx::query(statement).bind(discipline_uuid);
        if let Some(name) = name {
            query.bind(name).execute(&mut *tx).await
        } else {
            query.execute(&mut *tx).await
        }
        .map_err(map_sqlx_error)?;
        let item = Self::discipline_for_uuid(&mut tx, discipline_uuid).await?;
        tx.commit().await.map_err(map_sqlx_error)?;
        Ok(item)
    }
}

fn decode_classification_item(
    row: &sqlx::postgres::PgRow,
    id_column: &'static str,
) -> Result<ContentClassificationItem, StoreError> {
    Ok(ContentClassificationItem {
        uuid: row.try_get(id_column).map_err(map_sqlx_error)?,
        name: row.try_get("name").map_err(map_sqlx_error)?,
    })
}

fn decode_discipline(row: &sqlx::postgres::PgRow) -> Result<ContentDiscipline, StoreError> {
    Ok(ContentDiscipline {
        uuid: row
            .try_get("content_discipline_id")
            .map_err(map_sqlx_error)?,
        name: row.try_get("name").map_err(map_sqlx_error)?,
        is_retired: row.try_get("is_retired").map_err(map_sqlx_error)?,
    })
}

#[async_trait]
impl ContentClassificationStore for PostgresContentClassificationStore {
    async fn list_disciplines(
        &self,
        token: SessionTokenHash,
    ) -> Result<Vec<ContentClassificationItem>, StoreError> {
        self.read(
            token,
            "SELECT content_discipline_id, name FROM ple_api.list_content_disciplines()",
            "content_discipline_id",
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
            "SELECT content_subject_id, name FROM ple_api.list_content_subjects($1)",
            "content_subject_id",
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
            "SELECT content_topic_id, name FROM ple_api.list_content_topics($1)",
            "content_topic_id",
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
            "SELECT content_subtopic_id, name FROM ple_api.list_content_subtopics($1)",
            "content_subtopic_id",
            Some(topic_uuid),
        )
        .await
    }
}

#[async_trait]
impl ContentDisciplineDiscoveryStore for PostgresContentClassificationStore {
    async fn list_disciplines_including_retired(
        &self,
        token: SessionTokenHash,
    ) -> Result<Vec<ContentDiscipline>, StoreError> {
        let mut tx = self.begin(token).await?;
        let rows = sqlx::query(
            "SELECT content_discipline_id, name, is_retired \
             FROM ple_api.list_content_disciplines_including_retired()",
        )
        .fetch_all(&mut *tx)
        .await
        .map_err(map_sqlx_error)?;
        let items = rows
            .iter()
            .map(decode_discipline)
            .collect::<Result<Vec<_>, StoreError>>()?;
        tx.commit().await.map_err(map_sqlx_error)?;
        Ok(items)
    }
}

#[async_trait]
impl ContentDisciplineAdministrationStore for PostgresContentClassificationStore {
    async fn create_discipline(
        &self,
        token: SessionTokenHash,
        name: String,
    ) -> Result<ContentDiscipline, StoreError> {
        let mut tx = self.begin(token).await?;
        let discipline_uuid = sqlx::query_scalar("SELECT ple_api.create_content_discipline($1)")
            .bind(name)
            .fetch_one(&mut *tx)
            .await
            .map_err(map_sqlx_error)?;
        let item = Self::discipline_for_uuid(&mut tx, discipline_uuid).await?;
        tx.commit().await.map_err(map_sqlx_error)?;
        Ok(item)
    }
    async fn rename_discipline(
        &self,
        token: SessionTokenHash,
        discipline_uuid: Uuid,
        name: String,
    ) -> Result<ContentDiscipline, StoreError> {
        self.change_discipline(
            token,
            discipline_uuid,
            "SELECT ple_api.rename_content_discipline($1, $2)",
            Some(name),
        )
        .await
    }
    async fn retire_discipline(
        &self,
        token: SessionTokenHash,
        discipline_uuid: Uuid,
    ) -> Result<ContentDiscipline, StoreError> {
        self.change_discipline(
            token,
            discipline_uuid,
            "SELECT ple_api.retire_content_discipline($1)",
            None,
        )
        .await
    }
    async fn restore_discipline(
        &self,
        token: SessionTokenHash,
        discipline_uuid: Uuid,
    ) -> Result<ContentDiscipline, StoreError> {
        self.change_discipline(
            token,
            discipline_uuid,
            "SELECT ple_api.restore_content_discipline($1)",
            None,
        )
        .await
    }
}

//! Migrator-only PostgreSQL adapter for explicit existing-Revision maintenance.

use std::time::Duration;

use async_trait::async_trait;
use objects::{ObjectAddress, ObjectDataClass, ObjectRecord, ObjectStorageArea};
use question_model::{PublishedQuestionRevisionTuple, QuestionRevisionNumber};
use serde_json::Value;
use sqlx::{Postgres, Row, Transaction};

use super::{Pool, connection::map_sqlx_error};
use crate::{
    PreparedQuestionThumbnailPublication, PublishedQuestionThumbnail,
    QuestionThumbnailBackfillEvidence, QuestionThumbnailBackfillRegistrationError,
    QuestionThumbnailBackfillRevision, QuestionThumbnailBackfillStore, StoreError,
};

/// Binds only the explicitly supplied migration pool; no session or API role.
#[derive(Clone)]
pub struct PostgresQuestionThumbnailBackfillStore {
    pool: Pool,
}

impl PostgresQuestionThumbnailBackfillStore {
    pub fn new(pool: Pool) -> Self {
        Self { pool }
    }

    async fn begin(&self) -> Result<Transaction<'_, Postgres>, StoreError> {
        let mut transaction = self.pool.begin().await.map_err(map_sqlx_error)?;
        sqlx::query("SET LOCAL ROLE ple_private_owner")
            .execute(&mut *transaction)
            .await
            .map_err(map_sqlx_error)?;
        sqlx::query("SET LOCAL statement_timeout = '5s'")
            .execute(&mut *transaction)
            .await
            .map_err(map_sqlx_error)?;
        Ok(transaction)
    }

    async fn read(
        &self,
        revision: Option<&PublishedQuestionRevisionTuple>,
    ) -> Result<Vec<QuestionThumbnailBackfillRevision>, StoreError> {
        let mut transaction = self.begin().await?;
        // ASVS 1.2.4: exact identities stay bound values, including maintenance.
        let rows =
            sqlx::query("SELECT * FROM ple_private.question_thumbnail_backfill_snapshot($1, $2)")
                .bind(revision.map(|tuple| tuple.published_question_id.as_str()))
                .bind(revision.map(revision_number).transpose()?)
                .fetch_all(&mut *transaction)
                .await
                .map_err(map_sqlx_error)?;
        let result = rows.into_iter().map(decode_revision).collect();
        transaction.rollback().await.map_err(map_sqlx_error)?;
        result
    }
}

#[async_trait]
impl QuestionThumbnailBackfillStore for PostgresQuestionThumbnailBackfillStore {
    async fn snapshot(&self) -> Result<Vec<QuestionThumbnailBackfillRevision>, StoreError> {
        self.read(None).await
    }

    async fn load_revision(
        &self,
        revision: &PublishedQuestionRevisionTuple,
    ) -> Result<QuestionThumbnailBackfillEvidence, StoreError> {
        self.read(Some(revision))
            .await?
            .into_iter()
            .next()
            .ok_or(StoreError::NotFound)?
            .evidence
    }

    async fn register(
        &self,
        revision: &PublishedQuestionRevisionTuple,
        source: &ObjectRecord,
        thumbnail: &PreparedQuestionThumbnailPublication,
    ) -> Result<bool, QuestionThumbnailBackfillRegistrationError> {
        use QuestionThumbnailBackfillRegistrationError::{NotCommitted, Unknown};
        thumbnail.validate(source).map_err(NotCommitted)?;
        if source.published_question_revision_tuple.as_ref() != Some(revision) {
            return Err(NotCommitted(invalid()));
        }
        let revision_number = revision_number(revision).map_err(NotCommitted)?;
        let record = &thumbnail.object_record;
        let size = i64::try_from(record.size_bytes).map_err(|_| NotCommitted(invalid()))?;
        let source_address =
            serde_json::to_value(&source.address).map_err(|_| NotCommitted(invalid()))?;
        let thumbnail_address =
            serde_json::to_value(&record.address).map_err(|_| NotCommitted(invalid()))?;
        let mut transaction = tokio::time::timeout(Duration::from_secs(8), self.begin())
            .await
            .map_err(|_| NotCommitted(StoreError::TimedOut))?
            .map_err(NotCommitted)?;
        let result = tokio::time::timeout(Duration::from_secs(6), sqlx::query_scalar::<_, bool>(
            "SELECT ple_private.register_existing_question_thumbnail($1,$2,$3,$4,$5,$6,$7,$8,$9,$10)",
        )
        .bind(revision.published_question_id.as_str())
        .bind(revision_number)
        .bind(source.id.as_uuid())
        .bind(source.sha256.as_bytes().to_vec())
        .bind(source_address)
        .bind(record.id.as_uuid())
        .bind(thumbnail_address)
        .bind(record.sha256.as_bytes().to_vec())
        .bind(size)
        .bind(record.created_at.as_unix_millis())
        .fetch_one(&mut *transaction)).await;
        let created = match result {
            Ok(Ok(created)) => created,
            error => {
                let error = match error {
                    Ok(Err(error)) => map_sqlx_error(error),
                    _ => StoreError::TimedOut,
                };
                return Err(if confirmed_rollback(transaction).await {
                    NotCommitted(error)
                } else {
                    Unknown(error)
                });
            }
        };
        if !created {
            // The successful function returned before either candidate insert.
            // No COMMIT is sent; this candidate can never be the winner.
            let _ = confirmed_rollback(transaction).await;
            return Ok(false);
        }
        // A lost COMMIT response retains potentially registered immutable bytes.
        tokio::time::timeout(Duration::from_secs(5), transaction.commit())
            .await
            .map_err(|_| Unknown(StoreError::TimedOut))?
            .map_err(map_sqlx_error)
            .map_err(Unknown)?;
        Ok(true)
    }
}

async fn confirmed_rollback(transaction: Transaction<'_, Postgres>) -> bool {
    matches!(
        tokio::time::timeout(Duration::from_secs(2), transaction.rollback()).await,
        Ok(Ok(()))
    )
}

fn revision_number(revision: &PublishedQuestionRevisionTuple) -> Result<i32, StoreError> {
    i32::try_from(revision.revision_number.get()).map_err(|_| invalid())
}

fn decode_revision(
    row: sqlx::postgres::PgRow,
) -> Result<QuestionThumbnailBackfillRevision, StoreError> {
    let revision = PublishedQuestionRevisionTuple {
        published_question_id: row
            .try_get::<String, _>("published_question_id")
            .map_err(map_sqlx_error)?
            .parse()
            .map_err(|_| invalid())?,
        revision_number: u32::try_from(
            row.try_get::<i32, _>("revision_number")
                .map_err(map_sqlx_error)?,
        )
        .ok()
        .and_then(|number| QuestionRevisionNumber::new(number).ok())
        .ok_or_else(invalid)?,
    };
    let evidence = decode_evidence(&row, &revision);
    Ok(QuestionThumbnailBackfillRevision { revision, evidence })
}

fn decode_evidence(
    row: &sqlx::postgres::PgRow,
    revision: &PublishedQuestionRevisionTuple,
) -> Result<QuestionThumbnailBackfillEvidence, StoreError> {
    let binding = row
        .try_get::<Option<Value>, _>("source_binding")
        .map_err(map_sqlx_error)?
        .ok_or_else(invalid)?;
    let source: ObjectRecord = serde_json::from_value(
        row.try_get::<Option<Value>, _>("source_record")
            .map_err(map_sqlx_error)?
            .ok_or_else(invalid)?,
    )
    .map_err(|_| invalid())?;
    let expected_address = ObjectAddress::QuestionSource {
        published_question_revision_tuple: revision.clone(),
        object_id: source.id,
    };
    if binding["backend"] != "webwork"
        || !matches!(
            binding["question_format"].as_str(),
            Some("webworkPg" | "webworkPgml")
        )
        || binding["published_question_id"] != revision.published_question_id.as_str()
        || binding["revision_number"].as_u64() != Some(u64::from(revision.revision_number.get()))
        || binding["source_object_record_id"] != source.id.to_string()
        || binding["source_object_checksum"] != source.sha256.to_string()
        || source.address != expected_address
        || source.published_question_revision_tuple.as_ref() != Some(revision)
        || source.storage_area != ObjectStorageArea::PrivateContent
        || source.data_class != ObjectDataClass::QuestionSource
        || source.media_type != "text/x-wework-pg"
    {
        return Err(invalid());
    }
    let webwork_pg_path = binding["webwork_pg_path"]
        .as_str()
        .ok_or_else(invalid)?
        .to_owned();
    let thumbnail = row
        .try_get::<Option<Value>, _>("thumbnail")
        .map_err(map_sqlx_error)?
        .map(|value| {
            let object_record =
                serde_json::from_value(value["objectRecord"].clone()).map_err(|_| invalid())?;
            if value["intrinsicWidth"] != 256 || value["intrinsicHeight"] != 256 {
                return Err(invalid());
            }
            let thumbnail = PublishedQuestionThumbnail {
                object_record,
                intrinsic_width: 256,
                intrinsic_height: 256,
            };
            PreparedQuestionThumbnailPublication {
                object_record: thumbnail.object_record.clone(),
                source_checksum: source.sha256,
                intrinsic_width: thumbnail.intrinsic_width,
                intrinsic_height: thumbnail.intrinsic_height,
            }
            .validate(&source)?;
            Ok(thumbnail)
        })
        .transpose()?;
    Ok(QuestionThumbnailBackfillEvidence {
        source_record: source,
        webwork_pg_path,
        thumbnail,
    })
}

fn invalid() -> StoreError {
    StoreError::InvalidRecord("Existing Question Revision artifact evidence is invalid".into())
}

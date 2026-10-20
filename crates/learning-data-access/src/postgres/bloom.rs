//! Initial Bloom reads and atomic pair writes for the API background task.

use serde::Deserialize;
use sqlx::{Postgres, Transaction};

use super::{Pool, connection::map_sqlx_error};
use crate::StoreError;
use question_model::{
    ObjectId, PublishedQuestionId, PublishedQuestionRevisionTuple, QuestionRevisionNumber,
    SourceObjectChecksum,
};

/// One immutable Published Question source.
#[derive(Clone, Deserialize)]
pub struct BloomSource {
    /// Immutable Question lineage identity.
    pub question_id: PublishedQuestionId,
    /// Exact published revision containing the source.
    pub revision: u32,
    /// Backend used to interpret this source.
    pub backend: String,
    /// Stored source object identity.
    pub object_id: ObjectId,
    /// Expected source content checksum.
    pub checksum: SourceObjectChecksum,
    /// Renderer path for a WeBWorK source, when applicable.
    pub webwork_pg_path: Option<String>,
}

impl BloomSource {
    /// Validate the revision number and bind it to the immutable Question identity.
    pub fn revision_tuple(&self) -> Result<PublishedQuestionRevisionTuple, StoreError> {
        Ok(PublishedQuestionRevisionTuple {
            published_question_id: self.question_id.clone(),
            revision_number: QuestionRevisionNumber::new(self.revision)
                .map_err(|e| StoreError::InvalidRecord(e.to_string()))?,
        })
    }
}

/// Existing metadata concurrency tokens plus the evidence needed by JEV.
#[derive(Deserialize)]
pub struct BloomCandidate {
    /// Target kind: question or pool.
    pub kind: String,
    /// Question or Pool identity.
    pub id: String,
    /// Question revision; zero for a Pool.
    pub revision: i32,
    /// Metadata edit token captured before classification.
    pub edit_number: i64,
    /// Pool membership edit token; zero for a Question.
    pub membership_edit_number: i64,
    /// Current target title supplied as evidence.
    pub title: String,
    /// Current target description supplied as evidence.
    pub description: String,
    /// Exact source bindings for Question evidence; empty for Pools.
    pub sources: Vec<serde_json::Value>,
    /// Calculated Pool Cognitive Process, absent if members have none.
    pub cognitive: Option<String>,
    /// Calculated Pool Knowledge Dimension, absent if members have none.
    pub knowledge: Option<String>,
}

/// Uses the existing API login; source access and writes remain in fixed SQL functions.
#[derive(Clone)]
pub struct PostgresBloomStore {
    pool: Pool,
}

impl PostgresBloomStore {
    /// Share the existing API database pool.
    pub fn new(pool: Pool) -> Self {
        Self { pool }
    }

    async fn begin(&self) -> Result<Transaction<'_, Postgres>, StoreError> {
        let mut tx = self.pool.begin().await.map_err(map_sqlx_error)?;
        sqlx::query("SET LOCAL ROLE ple_app")
            .execute(&mut *tx)
            .await
            .map_err(map_sqlx_error)?;
        sqlx::query("SET LOCAL statement_timeout = '5s'")
            .execute(&mut *tx)
            .await
            .map_err(map_sqlx_error)?;
        Ok(tx)
    }

    /// Read a blank Question or the current member modes for a Pool.
    pub async fn candidate(
        &self,
        kind: &str,
        id: &str,
        revision: i32,
    ) -> Result<Option<BloomCandidate>, StoreError> {
        let mut tx = self.begin().await?;
        let row = sqlx::query_scalar::<_, serde_json::Value>(
            "SELECT * FROM ple_api.bloom_classification_candidate($1,$2,$3)",
        )
        .bind(kind)
        .bind(id)
        .bind(revision)
        .fetch_optional(&mut *tx)
        .await
        .map_err(map_sqlx_error)?;
        tx.commit().await.map_err(map_sqlx_error)?;
        row.map(|value| {
            serde_json::from_value(value)
                .map_err(|_| StoreError::InvalidRecord("Bloom candidate is invalid".into()))
        })
        .transpose()
    }

    /// Save both dimensions using the existing metadata and membership edit tokens.
    /// Return false when the target changed or the values already match.
    pub async fn save(
        &self,
        target: &BloomCandidate,
        cognitive: Option<&str>,
        knowledge: Option<&str>,
    ) -> Result<bool, StoreError> {
        let mut tx = self.begin().await?;
        let saved =
            sqlx::query_scalar("SELECT ple_api.save_bloom_classification($1,$2,$3,$4,$5,$6,$7)")
                .bind(&target.kind)
                .bind(&target.id)
                .bind(target.revision)
                .bind(target.edit_number)
                .bind(target.membership_edit_number)
                .bind(cognitive)
                .bind(knowledge)
                .fetch_one(&mut *tx)
                .await
                .map_err(map_sqlx_error)?;
        tx.commit().await.map_err(map_sqlx_error)?;
        Ok(saved)
    }
}

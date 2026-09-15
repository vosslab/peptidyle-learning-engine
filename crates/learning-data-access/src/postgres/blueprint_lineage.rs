//! PostgreSQL adapter for the Blueprint fork transaction.

use async_trait::async_trait;
use question_model::{
    BlueprintCourseReference, BlueprintMetadataEtag, BlueprintRevision, BlueprintRevisionReference,
    RequestChecksum, Timestamp,
};
use sqlx::{Postgres, Row, Transaction};

use super::{Pool, connection::map_sqlx_error};
use crate::{
    BlueprintForkSource, BlueprintLineageStore, ForkBlueprintCourseReceipt, SessionTokenHash,
    StoreError,
};

/// PostgreSQL implementation of the closed Blueprint fork boundary.
#[derive(Clone)]
pub struct PostgresBlueprintLineageStore {
    pool: Pool,
}

impl PostgresBlueprintLineageStore {
    pub fn new(pool: Pool) -> Self {
        Self { pool }
    }

    async fn begin(
        &self,
        session: SessionTokenHash,
    ) -> Result<Transaction<'_, Postgres>, StoreError> {
        let mut transaction = self.pool.begin().await.map_err(map_sqlx_error)?;
        sqlx::query("SET LOCAL ROLE ple_auth")
            .execute(&mut *transaction)
            .await
            .map_err(map_sqlx_error)?;
        let resolved = sqlx::query(
            "SELECT session_id FROM ple_api.resolve_and_install_session(decode($1, 'hex'))",
        )
        .bind(session.to_string())
        .fetch_optional(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;
        if resolved.is_none() {
            return Err(StoreError::Forbidden);
        }
        sqlx::query("SET LOCAL ROLE ple_app")
            .execute(&mut *transaction)
            .await
            .map_err(map_sqlx_error)?;
        Ok(transaction)
    }
}

#[async_trait]
impl BlueprintLineageStore for PostgresBlueprintLineageStore {
    async fn fork_blueprint_course(
        &self,
        session: SessionTokenHash,
        source: BlueprintForkSource,
        request_checksum: RequestChecksum,
    ) -> Result<ForkBlueprintCourseReceipt, StoreError> {
        let mut transaction = self.begin(session).await?;
        let actor = current_actor(&mut transaction).await?;
        let row = sqlx::query(
            "SELECT public_reference, blueprint_revision_number, metadata_etag, \
             (EXTRACT(EPOCH FROM accepted_at) * 1000)::bigint AS accepted_at_millis \\
             FROM ple_api.fork_blueprint_course($1, $2, $3, $4)",
        )
        .bind(random_uuid()?)
        .bind(source.blueprint_revision.reference.as_string())
        .bind(
            i64::try_from(source.blueprint_revision.revision.value()).map_err(|_| {
                StoreError::InvalidRecord("Blueprint Revision is invalid".to_string())
            })?,
        )
        .bind(request_checksum.into_bytes().to_vec())
        .fetch_one(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;
        let public_reference: String = row.try_get("public_reference").map_err(map_sqlx_error)?;
        let revision_number: i64 = row
            .try_get("blueprint_revision_number")
            .map_err(map_sqlx_error)?;
        let accepted_at_millis: i64 = row.try_get("accepted_at_millis").map_err(map_sqlx_error)?;
        let receipt = ForkBlueprintCourseReceipt {
            blueprint_revision: BlueprintRevisionReference {
                reference: blueprint_reference(public_reference)?,
                revision: blueprint_revision(revision_number)?,
            },
            source,
            metadata_etag: BlueprintMetadataEtag::from_uuid(
                row.try_get("metadata_etag").map_err(map_sqlx_error)?,
            ),
            actor,
            request_checksum,
            accepted_at: Timestamp::from_unix_millis(accepted_at_millis),
        };
        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(receipt)
    }
}

async fn current_actor(
    transaction: &mut Transaction<'_, Postgres>,
) -> Result<question_model::AccountId, StoreError> {
    let row = sqlx::query("SELECT ple_api.current_session_account_id() AS account_id")
        .fetch_optional(&mut **transaction)
        .await
        .map_err(map_sqlx_error)?
        .ok_or(StoreError::Forbidden)?;
    Ok(question_model::AccountId::from_uuid(
        row.try_get("account_id").map_err(map_sqlx_error)?,
    ))
}

fn blueprint_reference(value: String) -> Result<BlueprintCourseReference, StoreError> {
    value
        .parse()
        .map_err(|_| StoreError::InvalidRecord("Blueprint reference is invalid".to_string()))
}

fn blueprint_revision(value: i64) -> Result<BlueprintRevision, StoreError> {
    let number = u64::try_from(value)
        .map_err(|_| StoreError::InvalidRecord("Blueprint Revision is invalid".to_string()))?;
    BlueprintRevision::new(number)
        .ok_or_else(|| StoreError::InvalidRecord("Blueprint Revision is invalid".to_string()))
}

fn random_uuid() -> Result<uuid::Uuid, StoreError> {
    crate::random_uuid::random_uuid_v4(|_| {
        StoreError::Unavailable("Blueprint fork UUID randomness unavailable".to_string())
    })
}

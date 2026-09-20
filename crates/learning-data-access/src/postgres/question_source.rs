//! PostgreSQL persistence for Draft Question Source Bindings.

use async_trait::async_trait;
use objects::{ObjectAddress, ObjectDataClass, ObjectRecord, ObjectStorageArea, Sha256Checksum};
use question_model::{ObjectId, Timestamp};
use serde::Serialize;
use sqlx::{Postgres, Row, Transaction, types::Json};

use super::Pool;
use super::connection::map_sqlx_error;
use crate::{
    DraftQuestionPublicationSource, DraftQuestionPublicationSourceStore,
    DraftQuestionSourceBindingInput, DraftQuestionSourceBindingStore, DraftQuestionUuid,
    ExistingQuestionRevisionPublicationError, ExistingQuestionRevisionPublicationInput,
    ExistingQuestionRevisionPublicationStore, NewQuestionLineagePublicationError,
    NewQuestionLineagePublicationInput, NewQuestionLineagePublicationStore, SessionTokenHash,
    StoreError,
};
use question_model::WorkspaceId;

// This legacy uniqueness boundary also conclusively identifies a freshly
// minted Question ID collision. New code serializes candidate allocation and
// reports the same outcome with the dedicated QP001 SQLSTATE.
const PUBLISHED_QUESTION_PRIMARY_KEY: &str = "published_question_pkey";

/// PostgreSQL implementation of the session-authorized Draft Question Source Binding Store.
#[derive(Clone)]
pub struct PostgresDraftQuestionSourceBindingStore {
    pool: Pool,
}

#[async_trait]
impl DraftQuestionPublicationSourceStore for PostgresDraftQuestionSourceBindingStore {
    async fn load_draft_question_publication_source(
        &self,
        session_token_hash: SessionTokenHash,
        draft_question_uuid: DraftQuestionUuid,
        expected_draft_question_edit_number: crate::DraftQuestionEditNumber,
        workspace: WorkspaceId,
    ) -> Result<DraftQuestionPublicationSource, StoreError> {
        let mut transaction = self
            .begin_authenticated_application_transaction(session_token_hash)
            .await?;
        // ASVS 1.2.4, 2.2.2, 2.3.1, 8.2.1-8.2.3, and 8.3.1: values
        // remain parameters, while PostgreSQL rechecks current Instructor and
        // workspace authority plus the exact Draft Question Edit Number.
        let row =
            sqlx::query("SELECT * FROM ple_api.load_draft_question_publication_source($1, $2, $3)")
                .bind(draft_question_uuid.as_uuid())
                .bind(expected_draft_question_edit_number.as_postgres_bigint())
                .bind(workspace.as_uuid())
                .fetch_one(&mut *transaction)
                .await
                .map_err(map_sqlx_error)?;
        transaction.commit().await.map_err(map_sqlx_error)?;

        let object_id = ObjectId::from_uuid(row.try_get("object_id").map_err(map_sqlx_error)?);
        let Json(object_address): Json<ObjectAddress> =
            row.try_get("object_address").map_err(map_sqlx_error)?;
        let checksum: Vec<u8> = row.try_get("sha256").map_err(map_sqlx_error)?;
        let checksum: [u8; 32] = checksum.try_into().map_err(|_| {
            StoreError::InvalidRecord(
                "Draft Question Source Object Record checksum has invalid width".to_string(),
            )
        })?;
        let size_bytes: i64 = row.try_get("size_bytes").map_err(map_sqlx_error)?;
        let size_bytes = u64::try_from(size_bytes).map_err(|_| {
            StoreError::InvalidRecord(
                "Draft Question Source Object Record size is negative".to_string(),
            )
        })?;
        let created_at_millis: i64 = row.try_get("created_at_millis").map_err(map_sqlx_error)?;
        let source_record = ObjectRecord {
            id: object_id,
            storage_area: ObjectStorageArea::PrivateContent,
            data_class: ObjectDataClass::AuthoringContent,
            address: object_address,
            sha256: Sha256Checksum::from_bytes(checksum),
            size_bytes,
            media_type: row.try_get("media_type").map_err(map_sqlx_error)?,
            question_revision_tuple: None,
            created_at: Timestamp::from_unix_millis(created_at_millis),
        };
        Ok(DraftQuestionPublicationSource { source_record })
    }
}

impl PostgresDraftQuestionSourceBindingStore {
    /// Binds the already-attested API pool to private Draft Question Source Binding persistence.
    pub fn new(pool: Pool) -> Self {
        Self { pool }
    }

    async fn begin_authenticated_application_transaction(
        &self,
        token_hash: SessionTokenHash,
    ) -> Result<Transaction<'_, Postgres>, StoreError> {
        let mut transaction = self.pool.begin().await.map_err(map_sqlx_error)?;
        sqlx::query("SET LOCAL ROLE ple_auth")
            .execute(&mut *transaction)
            .await
            .map_err(map_sqlx_error)?;
        let session = sqlx::query(
            "SELECT session_id FROM ple_api.resolve_and_install_session(decode($1, 'hex'))",
        )
        .bind(token_hash.to_string())
        .fetch_optional(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;
        if session.is_none() {
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
impl DraftQuestionSourceBindingStore for PostgresDraftQuestionSourceBindingStore {
    async fn bind_draft_question_source(
        &self,
        session_token_hash: SessionTokenHash,
        input: DraftQuestionSourceBindingInput,
    ) -> Result<crate::DraftQuestionEditNumber, StoreError> {
        input.validate()?;
        let question_format = wire_string(&input.question_format, "Question Format")?;
        let question_type = wire_string(&input.question_type, "Question Type")?;

        let mut transaction = self
            .begin_authenticated_application_transaction(session_token_hash)
            .await?;
        // ASVS 1.2.4, 2.2.2, 2.3.1, 8.2.1, and 8.3.1: every value is
        // parameterized; the database resolves the authenticated session and
        // authorizes the exact workspace/Draft Question/Edit Number/object relationship in one
        // transaction before it creates or confirms an immutable record.
        let committed_edit: i64 = sqlx::query_scalar(
            "SELECT ple_api.bind_draft_question_source(\
                $1, $2, $3, $4, $5, $6, $7, $8, $9\
             )",
        )
        .bind(input.draft_question_uuid.as_uuid())
        .bind(
            input
                .expected_draft_question_edit_number
                .as_postgres_bigint(),
        )
        .bind(input.workspace.as_uuid())
        .bind(input.question_backend.as_str())
        .bind(question_format)
        .bind(question_type)
        .bind(input.webwork_pg_path)
        .bind(input.source_object_id.as_uuid())
        .bind(input.source_object_checksum.as_str())
        .fetch_one(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;
        transaction.commit().await.map_err(map_sqlx_error)?;
        crate::DraftQuestionEditNumber::new(u64::try_from(committed_edit).map_err(|_| {
            StoreError::InvalidRecord("committed Draft Question Edit Number is invalid".to_owned())
        })?)
    }
}

#[async_trait]
impl NewQuestionLineagePublicationStore for PostgresDraftQuestionSourceBindingStore {
    async fn publish_new_question_lineage(
        &self,
        session_token_hash: SessionTokenHash,
        input: NewQuestionLineagePublicationInput,
    ) -> Result<question_model::QuestionRevisionTuple, NewQuestionLineagePublicationError> {
        input
            .validate()
            .map_err(NewQuestionLineagePublicationError::Store)?;
        let question_revision_tuple = input.question_revision_tuple();
        let object_record = &input.question_source_object_record;
        let object_address = serde_json::to_value(&object_record.address).map_err(|_| {
            NewQuestionLineagePublicationError::Store(StoreError::InvalidRecord(
                "Question Publication Object Address cannot be encoded".to_string(),
            ))
        })?;
        let size_bytes = i64::try_from(object_record.size_bytes).map_err(|_| {
            NewQuestionLineagePublicationError::Store(StoreError::InvalidRecord(
                "Question Publication source size exceeds PostgreSQL bigint".to_string(),
            ))
        })?;
        let question_authorship: Vec<&str> = input
            .question_authorship
            .authors
            .iter()
            .map(|author| author.display_name.as_str())
            .collect();
        let question_authorship = serde_json::to_value(question_authorship).map_err(|_| {
            NewQuestionLineagePublicationError::Store(StoreError::InvalidRecord(
                "Question Authorship cannot be encoded".to_string(),
            ))
        })?;
        let initial_shared_tags: Vec<&str> = input
            .initial_shared_tags
            .iter()
            .map(|tag| tag.as_str())
            .collect();
        let question_license = wire_string(&input.question_license, "Question License")
            .map_err(NewQuestionLineagePublicationError::Store)?;
        let mut transaction = self
            .begin_authenticated_application_transaction(session_token_hash)
            .await
            .map_err(NewQuestionLineagePublicationError::Store)?;
        // ASVS 1.2.4, 2.2.2, 2.3.1, 2.3.3, 5.3.2, 8.2.1-8.2.3,
        // and 8.3.1: all values are parameters. The database rechecks current
        // Instructor/workspace authority, locks the exact Draft Question Edit
        // Number, derives the target Object Address, and commits the complete
        // Published Question aggregate in one transaction.
        sqlx::query(
            "SELECT ple_api.publish_new_question_lineage(\
                $1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, $18, $19, $20, $21, $22\
             )",
        )
        .bind(input.draft_question_uuid.as_uuid())
        .bind(
            input
                .expected_draft_question_edit_number
                .as_postgres_bigint(),
        )
        .bind(input.workspace.as_uuid())
        .bind(question_id_for_persistence(&input.question_id))
        .bind(object_record.id.as_uuid())
        .bind(object_address)
        .bind(object_record.sha256.as_bytes().to_vec())
        .bind(size_bytes)
        .bind(&object_record.media_type)
        .bind(object_record.created_at.as_unix_millis())
        .bind(question_authorship)
        .bind(initial_shared_tags)
        .bind(input.discipline_uuid)
        .bind(input.subject_uuid)
        .bind(input.topic_uuid)
        .bind(input.subtopic_uuid)
        .bind(question_license)
        .bind(input.question_revision_reason.as_str())
        .bind(input.question_ownership_event_id)
        .bind(input.question_publication_event_id)
        .bind(input.question_availability_event_id)
        .bind(encode_prepared_question_image(input.hotspot_question_image.as_ref()).map_err(NewQuestionLineagePublicationError::Store)?)
        .execute(&mut *transaction)
        .await
        .map_err(map_new_question_lineage_publication_error)?;
        transaction
            .commit()
            .await
            .map_err(map_sqlx_error)
            .map_err(NewQuestionLineagePublicationError::Store)?;
        Ok(question_revision_tuple)
    }
}

#[async_trait]
impl ExistingQuestionRevisionPublicationStore for PostgresDraftQuestionSourceBindingStore {
    async fn publish_question_revision(
        &self,
        session_token_hash: SessionTokenHash,
        input: ExistingQuestionRevisionPublicationInput,
    ) -> Result<question_model::QuestionRevisionTuple, ExistingQuestionRevisionPublicationError>
    {
        input
            .validate()
            .map_err(ExistingQuestionRevisionPublicationError::Store)?;
        let question_revision_tuple = input
            .question_revision_tuple()
            .map_err(ExistingQuestionRevisionPublicationError::Store)?;
        let object_record = &input.question_source_object_record;
        let object_address = serde_json::to_value(&object_record.address).map_err(|_| {
            ExistingQuestionRevisionPublicationError::Store(StoreError::InvalidRecord(
                "Question Revision Publication Object Address cannot be encoded".to_string(),
            ))
        })?;
        let size_bytes = i64::try_from(object_record.size_bytes).map_err(|_| {
            ExistingQuestionRevisionPublicationError::Store(StoreError::InvalidRecord(
                "Question Revision Publication source size exceeds PostgreSQL bigint".to_string(),
            ))
        })?;
        let mut transaction = self
            .begin_authenticated_application_transaction(session_token_hash)
            .await
            .map_err(ExistingQuestionRevisionPublicationError::Store)?;
        // ASVS 1.2.4, 2.2.2, 2.3.1-2.3.4, and 8.2.1-8.3.1: parameters
        // carry the browser-selected exact parent only. PostgreSQL locks the
        // lineage, repeats current owner and parent checks, and returns a
        // retryable conflict before it can register a stale successor.
        let row = sqlx::query(
            "SELECT ple_api.publish_question_revision(\
                $1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14\
             ) AS revision_number",
        )
        .bind(input.draft_question_uuid.as_uuid())
        .bind(
            input
                .expected_draft_question_edit_number
                .as_postgres_bigint(),
        )
        .bind(input.workspace.as_uuid())
        .bind(question_id_for_persistence(
            &input.parent_question_revision_tuple.question_id,
        ))
        .bind(
            i32::try_from(input.parent_question_revision_tuple.revision_number.get()).map_err(
                |_| {
                    ExistingQuestionRevisionPublicationError::Store(StoreError::InvalidRecord(
                        "Question parent Revision Number is invalid".to_string(),
                    ))
                },
            )?,
        )
        .bind(object_record.id.as_uuid())
        .bind(object_address)
        .bind(object_record.sha256.as_bytes().to_vec())
        .bind(size_bytes)
        .bind(&object_record.media_type)
        .bind(object_record.created_at.as_unix_millis())
        .bind(input.question_revision_reason.as_str())
        .bind(input.question_publication_event_id)
        .bind(
            encode_prepared_question_image(input.hotspot_question_image.as_ref())
                .map_err(ExistingQuestionRevisionPublicationError::Store)?,
        )
        .fetch_one(&mut *transaction)
        .await
        .map_err(map_existing_question_revision_publication_error)?;
        let revision_number: i32 = row
            .try_get("revision_number")
            .map_err(map_sqlx_error)
            .map_err(ExistingQuestionRevisionPublicationError::Store)?;
        let revision_number = u32::try_from(revision_number)
            .ok()
            .and_then(|value| question_model::QuestionRevisionNumber::new(value).ok())
            .ok_or_else(|| {
                ExistingQuestionRevisionPublicationError::Store(StoreError::InvalidRecord(
                    "Question Revision Publication returned an invalid revision".to_string(),
                ))
            })?;
        if revision_number != question_revision_tuple.revision_number {
            return Err(ExistingQuestionRevisionPublicationError::Store(
                StoreError::InvalidRecord(
                    "Question Revision Publication returned an unexpected revision".to_string(),
                ),
            ));
        }
        transaction
            .commit()
            .await
            .map_err(map_sqlx_error)
            .map_err(ExistingQuestionRevisionPublicationError::Store)?;
        Ok(question_revision_tuple)
    }
}

fn encode_prepared_question_image(
    asset: Option<&crate::PreparedQuestionImagePublication>,
) -> Result<Option<serde_json::Value>, StoreError> {
    asset.map(|asset| {
        let record = &asset.restricted_source_record;
        let address = serde_json::to_value(&record.address)
            .map_err(|_| StoreError::InvalidRecord("Publication asset address cannot be encoded".into()))?;
        Ok(serde_json::json!({
            "questionImageAssetId": asset.question_image_asset_id, "sourceObjectId": record.id, "sourceObjectAddress": address,
            "checksum": record.sha256.to_string(), "byteLength": record.size_bytes,
            "mediaType": record.media_type, "createdAtMillis": record.created_at.as_unix_millis(),
            "publicObjectId": asset.public_object_id, "intrinsicWidth": asset.intrinsic_width,
            "intrinsicHeight": asset.intrinsic_height, "deliveryId": asset.delivery_id, "jobId": asset.job_id,
        }))
    }).transpose()
}

fn map_existing_question_revision_publication_error(
    error: sqlx::Error,
) -> ExistingQuestionRevisionPublicationError {
    if let sqlx::Error::Database(database_error) = &error
        && database_error.code().as_deref() == Some("PQR01")
    {
        // This procedure's dedicated SQLSTATE is raised before any insert.
        // It proves the transaction rolled back, which lets the coordinator
        // remove only its just-written target object without inspecting a
        // database message or treating a genuine serialization failure as
        // conclusive.
        return ExistingQuestionRevisionPublicationError::Stale;
    }
    ExistingQuestionRevisionPublicationError::Store(map_sqlx_error(error))
}

fn map_new_question_lineage_publication_error(
    error: sqlx::Error,
) -> NewQuestionLineagePublicationError {
    if let sqlx::Error::Database(database_error) = &error
        && database_error.code().as_deref() == Some("QP001")
    {
        return NewQuestionLineagePublicationError::IdentityCollision;
    }
    if let sqlx::Error::Database(database_error) = &error
        && database_error.code().as_deref() == Some("23505")
    {
        return if is_published_question_identity_collision(
            database_error.code().as_deref(),
            database_error.constraint(),
        ) {
            NewQuestionLineagePublicationError::IdentityCollision
        } else {
            // Do not expose a PostgreSQL constraint name beyond this adapter.
            // A different uniqueness violation is not an ID-allocation race.
            NewQuestionLineagePublicationError::Store(StoreError::InvalidRecord(
                "Question Publication violates a database uniqueness invariant".to_string(),
            ))
        };
    }
    NewQuestionLineagePublicationError::Store(map_sqlx_error(error))
}

fn is_published_question_identity_collision(code: Option<&str>, constraint: Option<&str>) -> bool {
    code == Some("QP001")
        || (code == Some("23505") && constraint == Some(PUBLISHED_QUESTION_PRIMARY_KEY))
}

fn question_id_for_persistence(question_id: &question_model::QuestionId) -> &str {
    question_id.as_str()
}

fn wire_string(value: &impl Serialize, label: &str) -> Result<String, StoreError> {
    match serde_json::to_value(value) {
        Ok(serde_json::Value::String(value)) => Ok(value),
        _ => Err(StoreError::InvalidRecord(format!(
            "{label} must have one scalar canonical wire value"
        ))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    #[test]
    fn only_explicit_question_id_outcomes_are_identity_collisions() {
        assert!(is_published_question_identity_collision(
            Some("23505"),
            Some(PUBLISHED_QUESTION_PRIMARY_KEY),
        ));
        assert!(is_published_question_identity_collision(
            Some("QP001"),
            None,
        ));
        assert!(!is_published_question_identity_collision(
            Some("23505"),
            Some("question_publication_event_question_id_revision_number_key"),
        ));
        assert!(!is_published_question_identity_collision(
            Some("23505"),
            None
        ));
        assert!(!is_published_question_identity_collision(
            Some("23503"),
            Some(PUBLISHED_QUESTION_PRIMARY_KEY),
        ));
    }

    #[test]
    fn new_lineage_publication_binds_the_canonical_database_question_id() {
        let question_id = question_model::QuestionId::from_str("ABCD-XEFG")
            .expect("canonical Question ID is accepted at the model boundary");

        assert_eq!(question_id_for_persistence(&question_id), "ABCD-XEFG");
    }
}

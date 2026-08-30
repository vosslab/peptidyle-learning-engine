//! PostgreSQL flat-question source and grader-only grading persistence.

use async_trait::async_trait;
use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use objects::{ObjectRecord, Sha256Digest};
use question_model::DraftQuestionSource;
use question_model::WorkspaceId;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::postgres::PgRow;
use sqlx::{Postgres, Row, Transaction};

use crate::{
    ActorContext, FlatQuestionGradingPayload, FlatQuestionStore, StoreError,
    UpsertFlatQuestionCommand, WorkspaceDraftRevision, WorkspaceFlatQuestionSource,
};

use super::{PostgresGraderStore, PostgresStore, map_sqlx_error};

pub(super) const FLAT_SOURCE_PAYLOAD_SIZE_BYTES: usize = 65_536;

/// Persisted representation for protected current and published grading rows.
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct FlatQuestionGradingAnswerKeyPayload {
    public_sha256: String,
    payload_sha256: String,
    payload_base64: String,
}

impl std::fmt::Debug for FlatQuestionGradingAnswerKeyPayload {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("FlatQuestionGradingAnswerKeyPayload([redacted])")
    }
}

#[cfg(feature = "postgres")]
#[async_trait]
impl FlatQuestionStore for PostgresStore {
    async fn upsert_flat_question(
        &self,
        actor: ActorContext,
        command: UpsertFlatQuestionCommand,
    ) -> Result<WorkspaceFlatQuestionSource, StoreError> {
        ensure_upsert_inputs(&command)?;
        let actor_user = actor.user_id();
        let source_family = flat_source_family(&command.draft.question.source)?;
        let title = command.draft.question.metadata.title.clone();
        let definition = serde_json::to_value(&command.draft)
            .map_err(|error| StoreError::InvalidRecord(error.to_string()))?;
        let workspace = command.draft.question.workspace;
        let mut transaction = self.begin_actor(actor).await?;
        let current: Option<i64> = sqlx::query_scalar(
            "SELECT revision::bigint FROM ple_private.workspace_draft_question \
             WHERE workspace_id = $1 FOR UPDATE",
        )
        .bind(workspace.as_uuid())
        .fetch_optional(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;
        let revision = match current {
            Some(value) => {
                let current = WorkspaceDraftRevision::from_stored(value)?;
                if command.expected_revision != Some(current) {
                    return Err(StoreError::Conflict);
                }
                let next = current.next()?;
                sqlx::query(
                    "UPDATE ple_private.workspace_draft_question \
                     SET title = $2, definition = $3, revision = $4, \
                         updated_at = transaction_timestamp() \
                     WHERE workspace_id = $1",
                )
                .bind(workspace.as_uuid())
                .bind(title)
                .bind(definition)
                .bind(i32::try_from(next.value()).map_err(|_| {
                    StoreError::Unavailable("workspace draft revision limit reached".to_string())
                })?)
                .execute(&mut *transaction)
                .await
                .map_err(map_sqlx_error)?;
                next
            }
            None => {
                if command.expected_revision.is_some() {
                    return Err(StoreError::Conflict);
                }
                let (can_access, owns_workspace): (bool, bool) = sqlx::query_as(
                    "SELECT ple_api.current_actor_can_access_workspace($1), \
                            ple_api.current_actor_owns_workspace($1)",
                )
                .bind(workspace.as_uuid())
                .fetch_one(&mut *transaction)
                .await
                .map_err(map_sqlx_error)?;
                if can_access && !owns_workspace {
                    return Err(StoreError::Forbidden);
                }
                if !can_access {
                    sqlx::query(
                        "INSERT INTO ple_private.authoring_workspace \
                         (workspace_id, owner_user_id, created_at) \
                         VALUES ($1, $2, transaction_timestamp())",
                    )
                    .bind(workspace.as_uuid())
                    .bind(actor_user.as_uuid())
                    .execute(&mut *transaction)
                    .await
                    .map_err(map_sqlx_error)?;
                }
                sqlx::query(
                    "INSERT INTO ple_private.workspace_draft_question \
                     (draft_id, workspace_id, revision, title, definition, created_at, updated_at) \
                     VALUES ($1, $2, 1, $3, $4, transaction_timestamp(), transaction_timestamp())",
                )
                .bind(workspace.as_uuid())
                .bind(workspace.as_uuid())
                .bind(title)
                .bind(definition)
                .execute(&mut *transaction)
                .await
                .map_err(map_sqlx_error)?;
                WorkspaceDraftRevision::INITIAL
            }
        };

        let source_payload = command.source.clone();
        let source = WorkspaceFlatQuestionSource::new(
            command.draft.question.workspace,
            revision,
            source_family,
            source_payload.clone(),
            command.canonical_source_sha256.clone(),
            command.public_binding_sha256.clone(),
        )?;
        let source_record = serde_json::to_value(&source_payload).map_err(|error| {
            StoreError::Unavailable(format!(
                "serialized flat-question source record is invalid: {error}"
            ))
        })?;
        if serde_json::to_vec(&source_record)
            .map_err(|error| StoreError::Unavailable(error.to_string()))?
            .len()
            > FLAT_SOURCE_PAYLOAD_SIZE_BYTES
        {
            return Err(StoreError::InvalidRecord(
                "flat-question source payload exceeds metadata size limit".to_string(),
            ));
        }
        sqlx::query(
            "INSERT INTO ple_private.workspace_flat_question_source \
             (workspace_id, draft_revision, source_family, source_record, \
              canonical_source_sha256, public_binding_sha256, created_at, updated_at) \
             VALUES ($1, $2, $3, $4, $5, $6, transaction_timestamp(), transaction_timestamp()) \
             ON CONFLICT (workspace_id) DO UPDATE \
             SET draft_revision = EXCLUDED.draft_revision, source_family = EXCLUDED.source_family, \
                 source_record = EXCLUDED.source_record, \
                 canonical_source_sha256 = EXCLUDED.canonical_source_sha256, \
                 public_binding_sha256 = EXCLUDED.public_binding_sha256, \
                 updated_at = transaction_timestamp()",
        )
        .bind(workspace.as_uuid())
        .bind(i64::try_from(revision.value()).map_err(|_| {
            StoreError::Unavailable(
                "workspace draft revision does not fit database integer".to_string(),
            )
        })?)
        .bind(&source.source_family)
        .bind(source_record)
        .bind(&source.canonical_source_sha256)
        .bind(&source.public_binding_sha256)
        .execute(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;

        let (grading_payload, grading_checksum) = encode_flat_question_grading_payload(&command.grading)?;
        let updated = sqlx::query(
            "UPDATE ple_private.workspace_flat_question_grading \
             SET draft_revision = $2, public_binding_sha256 = $3, payload = $4, \
                 payload_sha256 = $5, updated_at = transaction_timestamp() \
             WHERE workspace_id = $1",
        )
        .bind(workspace.as_uuid())
        .bind(i64::try_from(revision.value()).map_err(|_| {
            StoreError::Unavailable("workspace draft revision does not fit database integer".to_string())
        })?)
        .bind(&source.public_binding_sha256)
        .bind(&grading_payload)
        .bind(&grading_checksum)
        .execute(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;
        if updated.rows_affected() == 0 {
            sqlx::query(
                "INSERT INTO ple_private.workspace_flat_question_grading \
                 (workspace_id, draft_revision, public_binding_sha256, payload, payload_sha256, created_at, updated_at) \
                 VALUES ($1, $2, $3, $4, $5, transaction_timestamp(), transaction_timestamp())",
            )
            .bind(workspace.as_uuid())
            .bind(i64::try_from(revision.value()).map_err(|_| {
                StoreError::Unavailable("workspace draft revision does not fit database integer".to_string())
            })?)
            .bind(&source.public_binding_sha256)
            .bind(grading_payload)
            .bind(grading_checksum)
            .execute(&mut *transaction)
            .await
            .map_err(map_sqlx_error)?;
        }

        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(source)
    }

    async fn flat_question_source(
        &self,
        actor: ActorContext,
        workspace: WorkspaceId,
    ) -> Result<Option<WorkspaceFlatQuestionSource>, StoreError> {
        let mut transaction = self.begin_actor(actor).await?;
        let row = sqlx::query(
            "SELECT draft_revision::bigint AS draft_revision, source_family, source_record, \
                    canonical_source_sha256, public_binding_sha256 \
             FROM ple_private.workspace_flat_question_source \
             WHERE workspace_id = $1",
        )
        .bind(workspace.as_uuid())
        .fetch_optional(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;
        let row = match row {
            Some(row) => row,
            None => {
                transaction.commit().await.map_err(map_sqlx_error)?;
                return Ok(None);
            }
        };
        let source_record: ObjectRecord = serde_json::from_value(
            row.try_get("source_record").map_err(map_sqlx_error)?,
        )
        .map_err(|error| StoreError::Unavailable(error.to_string()))?;
        let source = WorkspaceFlatQuestionSource::new(
            workspace,
            WorkspaceDraftRevision::from_stored(
                row.try_get("draft_revision").map_err(map_sqlx_error)?,
            )?,
            row.try_get("source_family").map_err(map_sqlx_error)?,
            source_record,
            row.try_get("canonical_source_sha256").map_err(map_sqlx_error)?,
            row.try_get("public_binding_sha256").map_err(map_sqlx_error)?,
        )?;
        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(Some(source))
    }
}

#[cfg(feature = "postgres")]
#[async_trait]
impl crate::FlatQuestionGradingStore for PostgresGraderStore {
    async fn flat_question_published_grading(
        &self,
        reference: question_model::ProblemVersionRef,
    ) -> Result<Option<FlatQuestionGradingPayload>, StoreError> {
        let mut transaction = self.begin_grader().await?;
        let row = sqlx::query(
            "SELECT payload AS key_payload, payload_sha256 AS key_sha256 \
             FROM ple_private.published_flat_question_grading \
             WHERE problem_id = $1 AND version_id = $2",
        )
        .bind(reference.problem.as_uuid())
        .bind(reference.version.as_uuid())
        .fetch_optional(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;
        let material = row
            .as_ref()
            .map(decode_flat_question_grading_payload)
            .transpose()?;
        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(material)
    }
}

pub(super) fn ensure_upsert_inputs(command: &UpsertFlatQuestionCommand) -> Result<(), StoreError> {
    crate::flat_question::validate_upsert_flat_question_command(command)
}

fn flat_source_family(source: &DraftQuestionSource) -> Result<String, StoreError> {
    match source {
        DraftQuestionSource::Native { family }
            if grading::flat_question::is_flat_question_family(family) =>
        {
            Ok(family.clone())
        }
        DraftQuestionSource::Native { .. } => Err(StoreError::InvalidRecord(
            "flat-question source family is unsupported".to_string(),
        )),
        _ => Err(StoreError::InvalidRecord(
            "flat-question sources require native draft source".to_string(),
        )),
    }
}

pub(super) async fn stage_flat_question_grading(
    transaction: &mut Transaction<'_, Postgres>,
    source: &WorkspaceFlatQuestionSource,
    _draft_payload_sha256: &str,
    _source_payload_sha256: &str,
    grading: &FlatQuestionGradingPayload,
) -> Result<(), StoreError> {
    let (grading_payload, grading_checksum) = encode_flat_question_grading_payload(grading)?;
    let revision = i64::try_from(source.workspace_revision.value()).map_err(|_| {
        StoreError::Unavailable("workspace draft revision does not fit database integer".to_string())
    })?;
    let updated = sqlx::query(
        "UPDATE ple_private.workspace_flat_question_grading \
         SET draft_revision = $2, public_binding_sha256 = $3, payload = $4, \
             payload_sha256 = $5, updated_at = transaction_timestamp() \
         WHERE workspace_id = $1",
    )
    .bind(source.workspace.as_uuid())
    .bind(revision)
    .bind(&source.public_binding_sha256)
    .bind(&grading_payload)
    .bind(&grading_checksum)
    .execute(&mut **transaction)
    .await
    .map_err(map_sqlx_error)?;
    if updated.rows_affected() == 0 {
        sqlx::query(
            "INSERT INTO ple_private.workspace_flat_question_grading \
             (workspace_id, draft_revision, public_binding_sha256, payload, payload_sha256, created_at, updated_at) \
             VALUES ($1, $2, $3, $4, $5, transaction_timestamp(), transaction_timestamp())",
        )
        .bind(source.workspace.as_uuid())
        .bind(revision)
        .bind(&source.public_binding_sha256)
        .bind(grading_payload)
        .bind(grading_checksum)
        .execute(&mut **transaction)
        .await
        .map_err(map_sqlx_error)?;
    }
    Ok(())
}

fn decode_flat_question_grading_payload(
    row: &PgRow,
) -> Result<FlatQuestionGradingPayload, StoreError> {
    let payload: Value = row.try_get("key_payload").map_err(map_sqlx_error)?;
    let checksum: String = row.try_get("key_sha256").map_err(map_sqlx_error)?;
    decode_flat_question_grading_payload_parts(payload, checksum)
}

/// Decodes one grader-only answer-key envelope without relying on a database row.
///
/// The persisted JSONB envelope preserves the canonical private payload bytes,
/// whose original formatting cannot safely be reconstructed from JSONB.
fn decode_flat_question_grading_payload_parts(
    payload: Value,
    row_checksum: String,
) -> Result<FlatQuestionGradingPayload, StoreError> {
    let stored: FlatQuestionGradingAnswerKeyPayload =
        serde_json::from_value(payload).map_err(|_| {
            StoreError::Unavailable(
                "stored flat-question grading payload has invalid answer-key structure".to_string(),
            )
        })?;
    let payload_bytes = STANDARD
        .decode(stored.payload_base64.as_bytes())
        .map_err(|_| {
            StoreError::Unavailable(
                "stored flat-question grading payload is not base64".to_string(),
            )
        })?;
    if Sha256Digest::compute(&payload_bytes).to_string() != stored.payload_sha256 {
        return Err(StoreError::Unavailable(
            "stored flat-question grading payload checksum mismatch".to_string(),
        ));
    }
    if row_checksum != stored.payload_sha256 {
        return Err(StoreError::Unavailable(
            "stored flat-question grading payload checksum mismatch".to_string(),
        ));
    }
    let grading = FlatQuestionGradingPayload::from_canonical_bytes(payload_bytes)?;
    if grading.public_binding_sha256() != stored.public_sha256.as_str() {
        Err(StoreError::Unavailable(
            "stored flat-question grading payload binding mismatch".to_string(),
        ))
    } else {
        Ok(grading)
    }
}

// Catalog publication calls this once its PostgreSQL promotion branch lands.
// Keep the envelope construction next to its verifier so JSONB never has to
// reproduce the private canonical JSON bytes.
#[allow(dead_code)]
pub(super) fn encode_flat_question_grading_payload(
    grading: &FlatQuestionGradingPayload,
) -> Result<(Value, String), StoreError> {
    let value = serde_json::to_value(FlatQuestionGradingAnswerKeyPayload {
        public_sha256: grading.public_binding_sha256().to_string(),
        payload_sha256: grading.sha256().to_string(),
        payload_base64: STANDARD.encode(grading.bytes()),
    })
    .map_err(|error| {
        StoreError::Unavailable(format!(
            "flat-question grader payload encoding failed: {error}"
        ))
    })?;
    Ok((value, grading.sha256().to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::flat_question::FlatQuestionGradingPayload;

    const FIXTURE: &str = r#"{"format":"pleFlatQuestion","version":2,"title":"Favorite color","prompt":"What is my favorite color?","response":{"kind":"singleChoice","choices":[{"id":"blue","text":"Blue"},{"id":"red","text":"Red"}],"correctChoice":"blue"},"points":1.0,"attemptPolicy":{"maxAttempts":null},"timingPolicy":{"kind":"untimed"},"license":{"kind":"cc0"},"language":"en-US"}"#;

    fn grading() -> FlatQuestionGradingPayload {
        let private =
            adapter_native::flat_question::FlatQuestionDocument::parse(FIXTURE.as_bytes())
                .expect("fixture should parse")
                .compile(question_model::WorkspaceId::from_uuid(uuid::Uuid::nil()))
                .expect("fixture should compile")
                .into_parts()
                .1;
        FlatQuestionGradingPayload::from_private(&private)
            .expect("compiled private material should persist")
    }

    #[test]
    fn postgres_staging_accepts_every_closed_flat_family_and_rejects_other_native_sources() {
        for family in [
            grading::flat_question::FLAT_SINGLE_CHOICE_V2_FAMILY,
            grading::flat_question::FLAT_SINGLE_CHOICE_V2_FAMILY,
            grading::flat_question::FLAT_MULTIPLE_ANSWER_FAMILY,
            grading::flat_question::FLAT_FILL_IN_FAMILY,
            grading::flat_question::FLAT_MULTI_FILL_IN_FAMILY,
            grading::flat_question::FLAT_NUMERIC_FAMILY,
            grading::flat_question::FLAT_MATCHING_FAMILY,
            grading::flat_question::FLAT_ORDERING_FAMILY,
            grading::flat_question::FLAT_HOTSPOT_FAMILY,
        ] {
            assert_eq!(
                flat_source_family(&DraftQuestionSource::Native {
                    family: family.to_string(),
                })
                .expect("closed flat family"),
                family
            );
        }
        assert!(
            flat_source_family(&DraftQuestionSource::Native {
                family: "unreviewed_native_family".to_string(),
            })
            .is_err()
        );
    }

    #[test]
    fn grading_payload_answer_key_roundtrip_preserves_bytes() {
        let payload = grading();
        let (encoded, checksum) = encode_flat_question_grading_payload(&payload)
            .expect("encoding should preserve expected JSON object");
        let decoded = decode_flat_question_grading_payload_parts(encoded, checksum)
            .expect("decoding should preserve grading payload bytes");
        assert_eq!(
            decoded.public_binding_sha256(),
            payload.public_binding_sha256()
        );
        assert_eq!(decoded.bytes(), payload.bytes());
    }

    #[test]
    fn grading_payload_invalid_checksum_is_rejected() {
        let payload = grading();
        let (encoded, checksum) =
            encode_flat_question_grading_payload(&payload).expect("encoding should succeed");
        assert!(
            decode_flat_question_grading_payload_parts(encoded, format!("{checksum}0")).is_err()
        );
    }

    #[test]
    fn grading_answer_key_envelope_debug_is_redacted() {
        let payload = grading();
        let private_digest = payload.sha256().to_string();
        let public_digest = payload.public_binding_sha256().to_string();
        let encoded = FlatQuestionGradingAnswerKeyPayload {
            public_sha256: public_digest.clone(),
            payload_sha256: private_digest.clone(),
            payload_base64: STANDARD.encode(payload.bytes()),
        };

        let debug = format!("{encoded:?}");
        assert_eq!(debug, "FlatQuestionGradingAnswerKeyPayload([redacted])");
        assert!(!debug.contains(&public_digest));
        assert!(!debug.contains(&private_digest));
        assert!(!debug.contains(&encoded.payload_base64));
    }
}

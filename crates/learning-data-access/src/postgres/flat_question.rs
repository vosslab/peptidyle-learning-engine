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
    FlatQuestionGradingPayload, FlatQuestionStore, StoreError, TenantContext,
    UpsertFlatQuestionCommand, WorkspaceDraftRevision, WorkspaceFlatQuestionSource, ensure_tenant,
};

use super::{
    PostgresGraderStore, PostgresStore, decode_payload_row_named, encode_payload, map_sqlx_error,
};

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
        context: TenantContext,
        actor: crate::UserId,
        command: UpsertFlatQuestionCommand,
    ) -> Result<WorkspaceFlatQuestionSource, StoreError> {
        ensure_upsert_inputs(&command)?;
        ensure_tenant(context, command.draft.tenant)?;
        let source_family = flat_source_family(&command.draft.question.source)?;
        let (draft_payload, draft_checksum) = encode_payload(&command.draft)?;
        let mut transaction = self.begin_tenant(context).await?;
        let current: Option<i64> = sqlx::query(
            "SELECT revision FROM workspace_draft \
             WHERE tenant_id = $1 AND workspace_id = $2 FOR UPDATE",
        )
        .bind(command.draft.tenant.as_uuid())
        .bind(command.draft.question.workspace.as_uuid())
        .fetch_optional(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?
        .map(|row| row.try_get("revision").map_err(map_sqlx_error))
        .transpose()?;
        let revision = match current {
            Some(value) => {
                let current = WorkspaceDraftRevision::from_stored(value)?;
                let role: Option<String> = sqlx::query_scalar(
                    "SELECT role FROM workspace_draft_access \
                     WHERE tenant_id = $1 AND workspace_id = $2 AND user_id = $3",
                )
                .bind(command.draft.tenant.as_uuid())
                .bind(command.draft.question.workspace.as_uuid())
                .bind(actor.as_uuid())
                .fetch_optional(&mut *transaction)
                .await
                .map_err(map_sqlx_error)?;
                if !matches!(role.as_deref(), Some("owner") | Some("collaborator")) {
                    return Err(StoreError::Forbidden);
                }
                if command.expected_revision != Some(current) {
                    return Err(StoreError::Conflict);
                }
                let next = current.next()?;
                sqlx::query(
                    "UPDATE workspace_draft SET payload = $3, payload_sha256 = $4, \
                     revision = $5, updated_at = transaction_timestamp() \
                     WHERE tenant_id = $1 AND workspace_id = $2",
                )
                .bind(command.draft.tenant.as_uuid())
                .bind(command.draft.question.workspace.as_uuid())
                .bind(draft_payload)
                .bind(&draft_checksum)
                .bind(i64::try_from(next.value()).map_err(|_| {
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
                sqlx::query(
                    "INSERT INTO workspace_draft \
                     (tenant_id, workspace_id, payload, payload_sha256, revision) \
                     VALUES ($1, $2, $3, $4, $5)",
                )
                .bind(command.draft.tenant.as_uuid())
                .bind(command.draft.question.workspace.as_uuid())
                .bind(draft_payload)
                .bind(&draft_checksum)
                .bind(
                    i64::try_from(WorkspaceDraftRevision::INITIAL.value()).map_err(|_| {
                        StoreError::Unavailable(
                            "workspace draft revision limit reached".to_string(),
                        )
                    })?,
                )
                .execute(&mut *transaction)
                .await
                .map_err(map_sqlx_error)?;
                sqlx::query(
                    "INSERT INTO workspace_draft_access \
                     (tenant_id, workspace_id, user_id, role) VALUES ($1, $2, $3, 'owner')",
                )
                .bind(command.draft.tenant.as_uuid())
                .bind(command.draft.question.workspace.as_uuid())
                .bind(actor.as_uuid())
                .execute(&mut *transaction)
                .await
                .map_err(map_sqlx_error)?;
                WorkspaceDraftRevision::INITIAL
            }
        };

        let source_payload = command.source.clone();
        let source = WorkspaceFlatQuestionSource::new(
            command.draft.tenant,
            command.draft.question.workspace,
            revision,
            source_family,
            source_payload.clone(),
            command.canonical_source_sha256.clone(),
            command.public_binding_sha256.clone(),
        )?;
        let (payload_json, payload_checksum) = encode_payload(&source_payload)?;
        let payload_bytes = serde_json::to_vec(&payload_json).map_err(|error| {
            StoreError::Unavailable(format!(
                "serialized flat-question source payload is invalid: {error}"
            ))
        })?;
        if payload_bytes.len() > FLAT_SOURCE_PAYLOAD_SIZE_BYTES {
            return Err(StoreError::InvalidRecord(
                "flat-question source payload exceeds metadata size limit".to_string(),
            ));
        }

        // Updating a draft activates the schema trigger that deletes the old
        // source binding. The app role has no UPDATE grant on that binding.
        sqlx::query(
            "INSERT INTO workspace_flat_question_source \
             (tenant_id, workspace_id, draft_revision, draft_payload_sha256, \
              source_object_id, source_payload, source_payload_sha256, \
              canonical_source_sha256, public_binding_sha256) \
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)",
        )
        .bind(command.draft.tenant.as_uuid())
        .bind(command.draft.question.workspace.as_uuid())
        .bind(i64::try_from(revision.value()).map_err(|_| {
            StoreError::Unavailable(
                "workspace draft revision does not fit database integer".to_string(),
            )
        })?)
        .bind(&draft_checksum)
        .bind(source_payload.id.as_uuid())
        .bind(payload_json)
        .bind(&payload_checksum)
        .bind(&source.canonical_source_sha256)
        .bind(&source.public_binding_sha256)
        .execute(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;

        stage_flat_question_grading(
            &mut transaction,
            &source,
            &draft_checksum,
            &payload_checksum,
            &command.grading,
        )
        .await?;

        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(source)
    }

    async fn flat_question_source(
        &self,
        context: TenantContext,
        actor: crate::UserId,
        workspace: WorkspaceId,
    ) -> Result<Option<WorkspaceFlatQuestionSource>, StoreError> {
        let mut transaction = self.begin_tenant(context).await?;
        let has_access: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM workspace_draft_access\n              WHERE tenant_id = $1 AND workspace_id = $2 AND user_id = $3)",
        )
        .bind(context.tenant_id().as_uuid())
        .bind(workspace.as_uuid())
        .bind(actor.as_uuid())
        .fetch_one(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;
        if !has_access {
            transaction.commit().await.map_err(map_sqlx_error)?;
            return Ok(None);
        }
        let row = sqlx::query(
            "SELECT d.payload, d.payload_sha256, d.revision,\n                    s.source_payload, s.source_payload_sha256,\n                    s.canonical_source_sha256, s.public_binding_sha256\n             FROM workspace_flat_question_source AS s\n             JOIN workspace_draft AS d\n               ON d.tenant_id = s.tenant_id AND d.workspace_id = s.workspace_id\n             WHERE s.tenant_id = $1 AND s.workspace_id = $2",
        )
        .bind(context.tenant_id().as_uuid())
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
        let source = decode_workspace_flat_source_row(context.tenant_id(), workspace, &row)?;
        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(Some(source))
    }
}

#[cfg(feature = "postgres")]
#[async_trait]
impl crate::FlatQuestionGradingStore for PostgresGraderStore {
    async fn flat_question_published_grading(
        &self,
        context: TenantContext,
        reference: question_model::ProblemVersionRef,
    ) -> Result<Option<FlatQuestionGradingPayload>, StoreError> {
        let mut transaction = self.begin_grader_tenant(context).await?;
        let row = sqlx::query(
            "SELECT key_payload, key_sha256 \
             FROM ple_flat_question_grading_material($1, $2, $3)",
        )
        .bind(context.tenant_id().as_uuid())
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
    draft_payload_sha256: &str,
    source_payload_sha256: &str,
    grading: &FlatQuestionGradingPayload,
) -> Result<(), StoreError> {
    let (grading_payload, grading_checksum) = encode_flat_question_grading_payload(grading)?;
    let staged: bool = sqlx::query_scalar(
        "SELECT ple_stage_flat_question_grading(\
            $1, $2, $3, $4, $5, $6, $7, $8, $9, $10)",
    )
    .bind(source.tenant.as_uuid())
    .bind(source.workspace.as_uuid())
    .bind(
        i64::try_from(source.workspace_revision.value()).map_err(|_| {
            StoreError::Unavailable(
                "workspace draft revision does not fit database integer".to_string(),
            )
        })?,
    )
    .bind(draft_payload_sha256)
    .bind(source.source_record.id.as_uuid())
    .bind(source_payload_sha256)
    .bind(&source.canonical_source_sha256)
    .bind(&source.public_binding_sha256)
    .bind(grading_payload)
    .bind(grading_checksum)
    .fetch_one(&mut **transaction)
    .await
    .map_err(map_sqlx_error)?;
    if !staged {
        return Err(StoreError::Conflict);
    }
    Ok(())
}

fn decode_workspace_flat_source_row(
    tenant: question_model::TenantId,
    workspace: WorkspaceId,
    row: &PgRow,
) -> Result<WorkspaceFlatQuestionSource, StoreError> {
    let draft_revision =
        WorkspaceDraftRevision::from_stored(row.try_get("revision").map_err(map_sqlx_error)?)?;
    let draft: crate::DraftRecord = decode_payload_row_named(row, "payload", "payload_sha256")?;
    let source_record: ObjectRecord =
        decode_payload_row_named(row, "source_payload", "source_payload_sha256")?;
    let source_family = match draft.question.source {
        DraftQuestionSource::Native { family } => family,
        _ => {
            return Err(StoreError::InvalidRecord(
                "flat-question source family is not native".to_string(),
            ));
        }
    };
    WorkspaceFlatQuestionSource::new(
        tenant,
        workspace,
        draft_revision,
        source_family,
        source_record,
        row.try_get("canonical_source_sha256")
            .map_err(map_sqlx_error)?,
        row.try_get("public_binding_sha256")
            .map_err(map_sqlx_error)?,
    )
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

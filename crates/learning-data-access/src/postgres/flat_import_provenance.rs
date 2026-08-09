//! PostgreSQL persistence for QTI-profile-to-flat private provenance.

use async_trait::async_trait;
use objects::{ObjectCategory, ObjectKey, ObjectRecord, Sha256Digest};
use question_model::{
    ActivityTimestamp, DraftQuestionSource, ObjectId, ProblemVersionRef, UserId, WorkspaceId,
    WorkspaceImportId,
};
use sqlx::postgres::PgRow;
use sqlx::{Postgres, Row, Transaction};
use uuid::Uuid;

use crate::{
    FlatImportChoiceMapPayload, FlatImportIntegrityDigests, FlatImportProvenanceStore,
    FlatImportPublicationPromotion, PersistedFlatImportProfile, PublishedFlatImportOrigin,
    QtiProfileFlatConversionCommand, QtiProfileImportEvidence, StoreError, TenantContext,
    UpsertFlatQuestionCommand, WorkspaceDraftRevision, WorkspaceFlatImportOrigin,
    WorkspaceFlatQuestionSource, ensure_tenant,
};

use super::flat_question::{
    FLAT_SOURCE_PAYLOAD_SIZE_BYTES, ensure_upsert_inputs, stage_flat_question_grading,
};
use super::{PostgresStore, encode_payload, map_sqlx_error, retry_transaction};

struct CommittedProfileEvidence {
    source_item_identifier: String,
    profile: PersistedFlatImportProfile,
    digests: FlatImportIntegrityDigests,
}

#[async_trait]
impl FlatImportProvenanceStore for PostgresStore {
    async fn stage_qti_profile_import_evidence(
        &self,
        context: TenantContext,
        evidence: QtiProfileImportEvidence,
    ) -> Result<(), StoreError> {
        ensure_tenant(context, evidence.persistence_parts().import.tenant)?;
        retry_transaction(|| {
            let evidence = evidence.clone();
            async move {
                let mut transaction = self.begin_tenant(context).await?;
                let staged = stage_qti_profile_import_evidence(&mut transaction, &evidence).await?;
                if !staged {
                    return Err(StoreError::Conflict);
                }
                transaction.commit().await.map_err(map_sqlx_error)
            }
        })
        .await
    }

    async fn convert_qti_profile_item_to_flat(
        &self,
        context: TenantContext,
        actor: UserId,
        command: QtiProfileFlatConversionCommand,
    ) -> Result<WorkspaceFlatQuestionSource, StoreError> {
        let command = QtiProfileFlatConversionCommand::new(
            command.expected_revision,
            command.draft,
            command.source,
            command.canonical_source_sha256,
            command.public_binding_sha256,
            command.grading,
            command.origin,
        )?;
        ensure_tenant(context, command.draft.tenant)?;
        if command.origin.acknowledged_by() != actor {
            return Err(StoreError::InvalidRecord(
                "flat-import acknowledgement actor must match the authenticated actor".to_string(),
            ));
        }
        retry_transaction(|| {
            let command = command.clone();
            async move { self.convert_in_transaction(context, actor, command).await }
        })
        .await
    }

    async fn workspace_flat_import_origin(
        &self,
        context: TenantContext,
        actor: UserId,
        workspace: WorkspaceId,
    ) -> Result<Option<WorkspaceFlatImportOrigin>, StoreError> {
        let mut transaction = self.begin_tenant(context).await?;
        let origin =
            read_workspace_flat_import_origin(&mut transaction, context, actor, workspace).await?;
        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(origin)
    }
}

async fn stage_qti_profile_import_evidence(
    transaction: &mut Transaction<'_, Postgres>,
    evidence: &QtiProfileImportEvidence,
) -> Result<bool, StoreError> {
    let parts = evidence.persistence_parts();
    sqlx::query_scalar(
        "SELECT ple_stage_qti_profile_evidence(\
            $1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15)",
    )
    .bind(parts.import.tenant.as_uuid())
    .bind(parts.import.workspace.as_uuid())
    .bind(parts.import.import.as_uuid())
    .bind(parts.source_item_identifier)
    .bind(parts.source_item_identifier)
    .bind(parts.profile.profile_id())
    .bind(parts.profile.profile_version())
    .bind(parts.profile.mapping_version())
    .bind(parts.digests.profile_report_sha256.to_string())
    .bind(parts.digests.normalized_item_sha256.to_string())
    .bind(parts.digests.public_mapping_sha256.to_string())
    .bind(parts.digests.private_mapping_sha256.to_string())
    .bind(parts.digests.mapping_sha256.to_string())
    .bind(parts.digests.warning_sha256.to_string())
    .bind(parts.digests.choice_map_sha256.to_string())
    .fetch_one(&mut **transaction)
    .await
    .map_err(map_sqlx_error)
}

impl PostgresStore {
    async fn convert_in_transaction(
        &self,
        context: TenantContext,
        actor: UserId,
        command: QtiProfileFlatConversionCommand,
    ) -> Result<WorkspaceFlatQuestionSource, StoreError> {
        let mut transaction = self.begin_tenant(context).await?;
        let upsert = UpsertFlatQuestionCommand {
            expected_revision: command.expected_revision,
            draft: command.draft.clone(),
            source: command.source.clone(),
            canonical_source_sha256: command.canonical_source_sha256.clone(),
            public_binding_sha256: command.public_binding_sha256.clone(),
            grading: command.grading.clone(),
        };
        ensure_upsert_inputs(&upsert)?;
        let source_family = match &command.draft.question.source {
            DraftQuestionSource::Native { family } => family.clone(),
            _ => {
                return Err(StoreError::InvalidRecord(
                    "flat-import conversion requires native draft source".to_string(),
                ));
            }
        };
        let (draft_payload, draft_checksum) = encode_payload(&command.draft)?;
        let (source_payload, source_payload_checksum) = encode_payload(&command.source)?;
        let source_payload_bytes = serde_json::to_vec(&source_payload).map_err(|error| {
            StoreError::Unavailable(format!(
                "serialized flat-question source payload is invalid: {error}"
            ))
        })?;
        if source_payload_bytes.len() > FLAT_SOURCE_PAYLOAD_SIZE_BYTES {
            return Err(StoreError::InvalidRecord(
                "flat-question source payload exceeds metadata size limit".to_string(),
            ));
        }

        // This draft row is the serialization lock for conversion and later
        // publication. The protected functions repeat this same first lock.
        let current: Option<i64> = sqlx::query_scalar(
            "SELECT revision FROM workspace_draft \
             WHERE tenant_id = $1 AND workspace_id = $2 FOR UPDATE",
        )
        .bind(command.draft.tenant.as_uuid())
        .bind(command.draft.question.workspace.as_uuid())
        .fetch_optional(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;

        let (revision, update_existing) = match current {
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
                (current.next()?, true)
            }
            None => {
                if command.expected_revision.is_some() {
                    return Err(StoreError::Conflict);
                }
                let inserted = sqlx::query(
                    "INSERT INTO workspace_draft \
                     (tenant_id, workspace_id, payload, payload_sha256, revision) \
                     VALUES ($1, $2, $3, $4, $5) \
                     ON CONFLICT (tenant_id, workspace_id) DO NOTHING",
                )
                .bind(command.draft.tenant.as_uuid())
                .bind(command.draft.question.workspace.as_uuid())
                .bind(draft_payload.clone())
                .bind(&draft_checksum)
                .bind(revision_i64(WorkspaceDraftRevision::INITIAL)?)
                .execute(&mut *transaction)
                .await
                .map_err(map_sqlx_error)?;
                require_first_draft_inserted(inserted.rows_affected())?;
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
                (WorkspaceDraftRevision::INITIAL, false)
            }
        };

        // The broker takes the committed registry FOR KEY SHARE and returns
        // only the answer-free, closed profile evidence.
        let evidence = read_committed_profile_evidence(&mut transaction, &command.origin)
            .await?
            .ok_or(StoreError::Conflict)?;
        validate_committed_profile_evidence(&command.origin, &evidence)?;

        if update_existing {
            sqlx::query(
                "UPDATE workspace_draft SET payload = $3, payload_sha256 = $4, \
                 revision = $5, updated_at = transaction_timestamp() \
                 WHERE tenant_id = $1 AND workspace_id = $2",
            )
            .bind(command.draft.tenant.as_uuid())
            .bind(command.draft.question.workspace.as_uuid())
            .bind(draft_payload)
            .bind(&draft_checksum)
            .bind(revision_i64(revision)?)
            .execute(&mut *transaction)
            .await
            .map_err(map_sqlx_error)?;
        }

        let source = WorkspaceFlatQuestionSource::new(
            command.draft.tenant,
            command.draft.question.workspace,
            revision,
            source_family,
            command.source.clone(),
            command.canonical_source_sha256.clone(),
            command.public_binding_sha256.clone(),
        )?;
        sqlx::query(
            "INSERT INTO workspace_flat_question_source \
             (tenant_id, workspace_id, draft_revision, draft_payload_sha256, \
              source_object_id, source_payload, source_payload_sha256, \
              canonical_source_sha256, public_binding_sha256) \
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)",
        )
        .bind(command.draft.tenant.as_uuid())
        .bind(command.draft.question.workspace.as_uuid())
        .bind(revision_i64(revision)?)
        .bind(&draft_checksum)
        .bind(command.source.id.as_uuid())
        .bind(source_payload)
        .bind(&source_payload_checksum)
        .bind(&command.canonical_source_sha256)
        .bind(&command.public_binding_sha256)
        .execute(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;

        // Origin replacement locks current origin before current source. Stage
        // grading only afterward so its source lock cannot invert that order;
        // a later refusal still rolls this entire transaction back.
        let replaced =
            replace_workspace_flat_import_origin(&mut transaction, actor, &command.origin).await?;
        if !replaced {
            return Err(StoreError::Conflict);
        }
        stage_flat_question_grading(
            &mut transaction,
            &source,
            &draft_checksum,
            &source_payload_checksum,
            &command.grading,
        )
        .await?;
        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(source)
    }
}

fn require_first_draft_inserted(rows_affected: u64) -> Result<(), StoreError> {
    if rows_affected != 1 {
        return Err(StoreError::Conflict);
    }
    Ok(())
}

async fn read_committed_profile_evidence(
    transaction: &mut Transaction<'_, Postgres>,
    origin: &WorkspaceFlatImportOrigin,
) -> Result<Option<CommittedProfileEvidence>, StoreError> {
    let import = origin.import();
    let row = sqlx::query(
        "SELECT source_item_identifier, profile_id, profile_version, mapping_version, \
                profile_report_sha256, normalized_item_sha256, public_mapping_sha256, \
                private_mapping_sha256, mapping_sha256, warning_sha256, choice_map_sha256 \
         FROM ple_read_committed_qti_profile_evidence($1, $2, $3, $4)",
    )
    .bind(import.tenant.as_uuid())
    .bind(import.workspace.as_uuid())
    .bind(import.import.as_uuid())
    .bind(origin.source_item_identifier())
    .fetch_optional(&mut **transaction)
    .await
    .map_err(map_sqlx_error)?;
    row.as_ref().map(decode_committed_profile_row).transpose()
}

fn decode_committed_profile_row(row: &PgRow) -> Result<CommittedProfileEvidence, StoreError> {
    let profile_id: String = row.try_get("profile_id").map_err(map_sqlx_error)?;
    let profile_version: String = row.try_get("profile_version").map_err(map_sqlx_error)?;
    let mapping_version: String = row.try_get("mapping_version").map_err(map_sqlx_error)?;
    Ok(CommittedProfileEvidence {
        source_item_identifier: row
            .try_get("source_item_identifier")
            .map_err(map_sqlx_error)?,
        profile: PersistedFlatImportProfile::from_stored(
            &profile_id,
            &profile_version,
            &mapping_version,
        )?,
        digests: FlatImportIntegrityDigests {
            normalized_item_sha256: decode_digest_column(row, "normalized_item_sha256")?,
            profile_report_sha256: decode_digest_column(row, "profile_report_sha256")?,
            public_mapping_sha256: decode_digest_column(row, "public_mapping_sha256")?,
            private_mapping_sha256: decode_digest_column(row, "private_mapping_sha256")?,
            mapping_sha256: decode_digest_column(row, "mapping_sha256")?,
            warning_sha256: decode_digest_column(row, "warning_sha256")?,
            choice_map_sha256: decode_digest_column(row, "choice_map_sha256")?,
        },
    })
}

fn validate_committed_profile_evidence(
    origin: &WorkspaceFlatImportOrigin,
    stored: &CommittedProfileEvidence,
) -> Result<(), StoreError> {
    if stored.source_item_identifier != origin.source_item_identifier()
        || stored.profile != origin.profile()
        || stored.digests != origin.digests()
    {
        return Err(StoreError::Conflict);
    }
    Ok(())
}

async fn replace_workspace_flat_import_origin(
    transaction: &mut Transaction<'_, Postgres>,
    actor: UserId,
    origin: &WorkspaceFlatImportOrigin,
) -> Result<bool, StoreError> {
    let import = origin.import();
    let archive = origin.source_archive();
    let parts = origin.persistence_parts();
    sqlx::query_scalar(
        "SELECT ple_replace_workspace_flat_import_origin(\
            $1, $2, $3, $4, $5, $6, $7, $8, $9, $10, \
            to_timestamp($11::double precision / 1000.0), \
            $12, $13, $14, $15, $16, $17, $18, $19, $20, $21, $22, $23, $24, $25, \
            to_timestamp($26::double precision / 1000.0), $27)",
    )
    .bind(import.tenant.as_uuid())
    .bind(import.workspace.as_uuid())
    .bind(actor.as_uuid())
    .bind(import.import.as_uuid())
    .bind(archive.id.as_uuid())
    .bind(archive.sha256.to_string())
    .bind(object_size_i64(archive)?)
    .bind(&archive.media_type)
    .bind(&archive.license)
    .bind(&archive.provenance)
    .bind(archive.created_at.as_unix_millis())
    .bind(parts.source_item_identifier)
    .bind(parts.profile.profile_id())
    .bind(parts.profile.profile_version())
    .bind(parts.profile.mapping_version())
    .bind(parts.conversion_version)
    .bind(parts.digests.normalized_item_sha256.to_string())
    .bind(parts.digests.profile_report_sha256.to_string())
    .bind(parts.digests.public_mapping_sha256.to_string())
    .bind(parts.digests.private_mapping_sha256.to_string())
    .bind(parts.digests.mapping_sha256.to_string())
    .bind(parts.digests.warning_sha256.to_string())
    .bind(parts.digests.choice_map_sha256.to_string())
    .bind(parts.mapped_canonical_source_sha256.to_string())
    .bind(parts.acknowledged_by.as_uuid())
    .bind(parts.acknowledged_at.as_unix_millis())
    .bind(parts.choice_map.bytes())
    .fetch_one(&mut **transaction)
    .await
    .map_err(map_sqlx_error)
}

pub(super) async fn read_workspace_flat_import_origin(
    transaction: &mut Transaction<'_, Postgres>,
    context: TenantContext,
    actor: UserId,
    workspace: WorkspaceId,
) -> Result<Option<WorkspaceFlatImportOrigin>, StoreError> {
    let row = sqlx::query(
        "SELECT import_id, source_archive_object_id, source_archive_sha256, \
                source_archive_size_bytes, source_archive_media_type, source_archive_license, \
                source_archive_provenance, \
                floor(extract(epoch FROM source_archive_created_at) * 1000)::bigint \
                    AS source_archive_created_at_millis, \
                source_item_identifier, profile_id, profile_version, mapping_version, \
                conversion_version, normalized_item_sha256, profile_report_sha256, \
                public_mapping_sha256, private_mapping_sha256, mapping_sha256, \
                warning_sha256, choice_map_sha256, mapped_canonical_source_sha256, \
                acknowledged_by, \
                floor(extract(epoch FROM acknowledged_at) * 1000)::bigint \
                    AS acknowledged_at_millis, \
                choice_map_payload \
         FROM ple_read_workspace_flat_import_origin($1, $2, $3)",
    )
    .bind(context.tenant_id().as_uuid())
    .bind(workspace.as_uuid())
    .bind(actor.as_uuid())
    .fetch_optional(&mut **transaction)
    .await
    .map_err(map_sqlx_error)?;
    row.as_ref()
        .map(|row| decode_workspace_origin_row(context, workspace, row))
        .transpose()
}

fn decode_workspace_origin_row(
    context: TenantContext,
    workspace: WorkspaceId,
    row: &PgRow,
) -> Result<WorkspaceFlatImportOrigin, StoreError> {
    let import_id: Uuid = row.try_get("import_id").map_err(map_sqlx_error)?;
    let archive_id: Uuid = row
        .try_get("source_archive_object_id")
        .map_err(map_sqlx_error)?;
    let import_id = WorkspaceImportId::from_uuid(import_id);
    let archive_id = ObjectId::from_uuid(archive_id);
    let import = crate::QtiImportRef {
        tenant: context.tenant_id(),
        workspace,
        import: import_id,
    };
    let key = ObjectKey::WorkspaceSource {
        tenant: context.tenant_id(),
        workspace,
        import: import_id,
        object: archive_id,
    };
    let archive_size: i64 = row
        .try_get("source_archive_size_bytes")
        .map_err(map_sqlx_error)?;
    let archive = ObjectRecord {
        id: archive_id,
        bucket: key.bucket(),
        key,
        sha256: decode_digest_column(row, "source_archive_sha256")?,
        size_bytes: u64::try_from(archive_size).map_err(|_| {
            StoreError::InvalidRecord(
                "stored flat-import source archive size is invalid".to_string(),
            )
        })?,
        media_type: row
            .try_get("source_archive_media_type")
            .map_err(map_sqlx_error)?,
        category: ObjectCategory::Source,
        version: None,
        license: row
            .try_get("source_archive_license")
            .map_err(map_sqlx_error)?,
        provenance: row
            .try_get("source_archive_provenance")
            .map_err(map_sqlx_error)?,
        created_at: ActivityTimestamp::from_unix_millis(
            row.try_get("source_archive_created_at_millis")
                .map_err(map_sqlx_error)?,
        ),
    };
    let profile_id: String = row.try_get("profile_id").map_err(map_sqlx_error)?;
    let profile_version: String = row.try_get("profile_version").map_err(map_sqlx_error)?;
    let mapping_version: String = row.try_get("mapping_version").map_err(map_sqlx_error)?;
    let choice_map: Vec<u8> = row.try_get("choice_map_payload").map_err(map_sqlx_error)?;
    WorkspaceFlatImportOrigin::new(
        import,
        row.try_get("source_item_identifier")
            .map_err(map_sqlx_error)?,
        PersistedFlatImportProfile::from_stored(&profile_id, &profile_version, &mapping_version)?,
        crate::FlatImportConversionVersion::new(
            row.try_get::<String, _>("conversion_version")
                .map_err(map_sqlx_error)?,
        )?,
        archive,
        FlatImportIntegrityDigests {
            normalized_item_sha256: decode_digest_column(row, "normalized_item_sha256")?,
            profile_report_sha256: decode_digest_column(row, "profile_report_sha256")?,
            public_mapping_sha256: decode_digest_column(row, "public_mapping_sha256")?,
            private_mapping_sha256: decode_digest_column(row, "private_mapping_sha256")?,
            mapping_sha256: decode_digest_column(row, "mapping_sha256")?,
            warning_sha256: decode_digest_column(row, "warning_sha256")?,
            choice_map_sha256: decode_digest_column(row, "choice_map_sha256")?,
        },
        decode_digest_column(row, "mapped_canonical_source_sha256")?,
        UserId::from_uuid(row.try_get("acknowledged_by").map_err(map_sqlx_error)?),
        ActivityTimestamp::from_unix_millis(
            row.try_get("acknowledged_at_millis")
                .map_err(map_sqlx_error)?,
        ),
        FlatImportChoiceMapPayload::from_canonical_bytes(choice_map)?,
    )
}

pub(super) async fn promote_flat_import_origin(
    transaction: &mut Transaction<'_, Postgres>,
    context: TenantContext,
    actor: UserId,
    workspace: WorkspaceId,
    reference: ProblemVersionRef,
    current: &WorkspaceFlatImportOrigin,
    promotion: &FlatImportPublicationPromotion,
) -> Result<bool, StoreError> {
    let exact = FlatImportPublicationPromotion::new(
        current,
        reference,
        promotion.published_archive().clone(),
    )?;
    if exact != *promotion {
        return Err(StoreError::Conflict);
    }
    let published = PublishedFlatImportOrigin::from_current(
        current,
        reference,
        promotion.published_archive().clone(),
    )?;
    let import = current.import();
    let source_archive = current.source_archive();
    let parts = published.persistence_parts();
    let published_archive = published.published_archive();
    sqlx::query_scalar(
        "SELECT ple_promote_flat_import_origin(\
            $1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, \
            $16, $17, $18, $19, $20, $21, $22, \
            to_timestamp($23::double precision / 1000.0), \
            $24, $25, $26, $27, $28, $29, \
            to_timestamp($30::double precision / 1000.0))",
    )
    .bind(context.tenant_id().as_uuid())
    .bind(workspace.as_uuid())
    .bind(actor.as_uuid())
    .bind(reference.problem.as_uuid())
    .bind(reference.version.as_uuid())
    .bind(import.import.as_uuid())
    .bind(source_archive.id.as_uuid())
    .bind(source_archive.sha256.to_string())
    .bind(parts.source_item_identifier)
    .bind(parts.profile.profile_id())
    .bind(parts.profile.profile_version())
    .bind(parts.profile.mapping_version())
    .bind(parts.conversion_version)
    .bind(parts.digests.normalized_item_sha256.to_string())
    .bind(parts.digests.profile_report_sha256.to_string())
    .bind(parts.digests.public_mapping_sha256.to_string())
    .bind(parts.digests.private_mapping_sha256.to_string())
    .bind(parts.digests.mapping_sha256.to_string())
    .bind(parts.digests.warning_sha256.to_string())
    .bind(parts.digests.choice_map_sha256.to_string())
    .bind(parts.mapped_canonical_source_sha256.to_string())
    .bind(parts.acknowledged_by.as_uuid())
    .bind(parts.acknowledged_at.as_unix_millis())
    .bind(published_archive.id.as_uuid())
    .bind(published_archive.sha256.to_string())
    .bind(object_size_i64(published_archive)?)
    .bind(&published_archive.media_type)
    .bind(&published_archive.license)
    .bind(&published_archive.provenance)
    .bind(published_archive.created_at.as_unix_millis())
    .fetch_one(&mut **transaction)
    .await
    .map_err(map_sqlx_error)
}

fn decode_digest_column(row: &PgRow, column: &'static str) -> Result<Sha256Digest, StoreError> {
    let value: String = row.try_get(column).map_err(map_sqlx_error)?;
    decode_stored_digest(&value, column)
}

fn decode_stored_digest(value: &str, field: &str) -> Result<Sha256Digest, StoreError> {
    let value = value.trim_end();
    if value.len() != 64
        || !value
            .bytes()
            .all(|byte| matches!(byte, b'0'..=b'9' | b'a'..=b'f'))
    {
        return Err(StoreError::InvalidRecord(format!(
            "stored flat-import {field} digest is invalid"
        )));
    }
    let mut bytes = [0_u8; 32];
    for (index, output) in bytes.iter_mut().enumerate() {
        let offset = index * 2;
        *output = u8::from_str_radix(&value[offset..offset + 2], 16).map_err(|_| {
            StoreError::InvalidRecord(format!("stored flat-import {field} digest is invalid"))
        })?;
    }
    Ok(Sha256Digest::from_bytes(bytes))
}

fn revision_i64(revision: WorkspaceDraftRevision) -> Result<i64, StoreError> {
    i64::try_from(revision.value()).map_err(|_| {
        StoreError::Unavailable(
            "workspace draft revision does not fit database integer".to_string(),
        )
    })
}

fn object_size_i64(record: &ObjectRecord) -> Result<i64, StoreError> {
    i64::try_from(record.size_bytes).map_err(|_| {
        StoreError::InvalidRecord("object size does not fit database integer".to_string())
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stored_character_digest_decodes_without_padding() {
        let digest = Sha256Digest::compute(b"flat import provenance");
        let stored = format!("{}   ", digest);
        assert_eq!(
            decode_stored_digest(&stored, "test").expect("valid digest should decode"),
            digest
        );
    }

    #[test]
    fn malformed_stored_digest_is_a_safe_record_error() {
        let error = decode_stored_digest(&"z".repeat(64), "test")
            .expect_err("non-hex stored digest must fail");
        assert!(matches!(error, StoreError::InvalidRecord(_)));
    }

    #[test]
    fn losing_first_draft_insert_is_a_conflict() {
        assert_eq!(require_first_draft_inserted(0), Err(StoreError::Conflict));
        assert_eq!(require_first_draft_inserted(1), Ok(()));
    }
}

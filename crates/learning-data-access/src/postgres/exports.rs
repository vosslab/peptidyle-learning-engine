//! PostgreSQL assignment-export creation, projection, and worker commit.

use async_trait::async_trait;
use question_model::{AssignmentId, CourseId, ObjectId, TenantId, UserId};
use sqlx::Row;

use super::course_roster::{lock_course_roster_cross_product, require_course_instructor};
use super::{
    PostgresStore, decode_payload_row, encode_payload, load_assignment_for_share, map_sqlx_error,
};
use crate::{
    AssetDeliveryId, AssetDeliveryRecord, AssetDeliveryScope, CreateAssignmentExport,
    ExportArtifactKind, ExportArtifactRecord, ExportCommitDisposition, ExportId, ExportJobCommit,
    ExportJobStore, JobId, JobPayload, StoreError, StudentExportArtifactView, StudentExportJob,
    StudentExportState, StudentExportView, TenantContext,
};

#[async_trait]
impl ExportJobStore for PostgresStore {
    async fn create_assignment_export(
        &self,
        context: TenantContext,
        session: crate::SessionTokenHash,
        request: CreateAssignmentExport,
    ) -> Result<StudentExportView, StoreError> {
        if !(1..=20).contains(&request.max_attempts) {
            return Err(StoreError::InvalidRecord(
                "job max attempts must be between 1 and 20".to_string(),
            ));
        }
        let export = ExportId::generate()?;
        let manifest = fresh_export_object_id()?;
        let job = JobId::generate()?;
        let mut transaction = self.begin_tenant(context).await?;
        let assignment =
            load_assignment_for_share(&mut transaction, context.tenant_id(), request.assignment)
                .await?;
        let records_accessible: bool =
            sqlx::query_scalar("SELECT public.ple_course_records_accessible($1, $2)")
                .bind(context.tenant_id().as_uuid())
                .bind(assignment.course_id.as_uuid())
                .fetch_one(&mut *transaction)
                .await
                .map_err(map_sqlx_error)?;
        if !records_accessible {
            return Err(StoreError::NotFound);
        }
        // Course roster mutation takes this same row lock. Holding it while
        // resolving the session and inserting the job prevents a removed
        // instructor from winning a membership-change race.
        lock_course_roster_cross_product(
            &mut transaction,
            context.tenant_id(),
            assignment.course_id,
        )
        .await?;
        let requested_by =
            require_course_instructor(&mut transaction, session, assignment.course_id).await?;
        let mut expected = Vec::new();
        for kind in ExportArtifactKind::ALL {
            expected.push((kind, fresh_export_object_id()?));
        }
        let frozen = StudentExportJob {
            id: export,
            tenant: context.tenant_id(),
            assignment: assignment.id,
            course: assignment.course_id,
            title: assignment.title.clone(),
            requested_by,
            manifest,
            problems: assignment.active_references().collect(),
            expected_artifacts: expected.clone(),
        };
        let (frozen_payload, frozen_checksum) = encode_payload(&frozen)?;
        let payload = serde_json::to_value(JobPayload::Export {
            delivery_object: manifest,
        })
        .map_err(|error| {
            StoreError::InvalidRecord(format!("job payload serialization failed: {error}"))
        })?;
        sqlx::query(
            "INSERT INTO worker_job (job_id, tenant_id, payload, state, max_attempts) \
             VALUES ($1, $2, $3, 'ready', $4)",
        )
        .bind(job.as_uuid())
        .bind(context.tenant_id().as_uuid())
        .bind(payload)
        .bind(i32::from(request.max_attempts))
        .execute(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;
        sqlx::query(
            "INSERT INTO student_export_request \
             (export_id, tenant_id, course_id, assignment_id, requester_id, job_id, manifest_object_id, \
              frozen_payload, frozen_payload_sha256, state) \
             VALUES ($1,$2,$3,$4,$5,$6,$7,$8,decode($9,'hex'),'queued')",
        )
        .bind(export.as_uuid())
        .bind(context.tenant_id().as_uuid())
        .bind(assignment.course_id.as_uuid())
        .bind(assignment.id.as_uuid())
        .bind(requested_by.as_uuid())
        .bind(job.as_uuid())
        .bind(manifest.as_uuid())
        .bind(frozen_payload)
        .bind(frozen_checksum)
        .execute(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;
        for (kind, object) in expected {
            sqlx::query(
                "INSERT INTO student_export_artifact (export_id, kind, object_id) VALUES ($1,$2,$3)",
            )
            .bind(export.as_uuid())
            .bind(export_artifact_kind_db(kind))
            .bind(object.as_uuid())
            .execute(&mut *transaction)
            .await
            .map_err(map_sqlx_error)?;
        }
        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(StudentExportView {
            id: export,
            assignment: assignment.id,
            state: StudentExportState::Queued,
            artifacts: None,
        })
    }

    async fn get_assignment_export(
        &self,
        context: TenantContext,
        export: ExportId,
    ) -> Result<Option<StudentExportView>, StoreError> {
        get_postgres_export_view(self, context, export, None).await
    }

    async fn get_assignment_export_for_requester(
        &self,
        context: TenantContext,
        export: ExportId,
        requester: UserId,
    ) -> Result<Option<StudentExportView>, StoreError> {
        get_postgres_export_view(self, context, export, Some(requester)).await
    }

    async fn load_export_job(
        &self,
        context: TenantContext,
        manifest: ObjectId,
    ) -> Result<Option<StudentExportJob>, StoreError> {
        let mut transaction = self.begin_tenant(context).await?;
        let row = sqlx::query(
            "SELECT frozen_payload, frozen_payload_sha256 FROM student_export_request \
             WHERE manifest_object_id = $1",
        )
        .bind(manifest.as_uuid())
        .fetch_optional(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;
        let result = row.as_ref().map(decode_payload_row).transpose()?;
        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(result)
    }

    async fn commit_export_effect(
        &self,
        context: TenantContext,
        commit: ExportJobCommit,
    ) -> Result<ExportCommitDisposition, StoreError> {
        validate_export_artifacts(context.tenant_id(), &commit.artifacts)?;
        let mut transaction = self.begin_tenant(context).await?;
        let request_row = sqlx::query(
            "SELECT requester_id, course_id FROM student_export_request \
             WHERE job_id = $1 AND manifest_object_id = $2",
        )
        .bind(commit.job.as_uuid())
        .bind(commit.manifest.as_uuid())
        .fetch_optional(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?
        .ok_or(StoreError::NotFound)?;
        let requester = UserId::from_uuid(
            request_row
                .try_get("requester_id")
                .map_err(map_sqlx_error)?,
        );
        let course = CourseId::from_uuid(request_row.try_get("course_id").map_err(map_sqlx_error)?);
        let mut artifacts = Vec::with_capacity(commit.artifacts.len());
        for artifact in &commit.artifacts {
            let delivery = AssetDeliveryRecord {
                id: AssetDeliveryId::from_object(artifact.object.id),
                object: artifact.object.clone(),
                intrinsic_width: None,
                intrinsic_height: None,
                scope: AssetDeliveryScope::StudentRecord {
                    tenant: context.tenant_id(),
                    course,
                    authorized_users: vec![requester],
                },
                publication: crate::AssetPublication::Ready,
                pending_source: None,
            };
            // The requester and course come only from the frozen export row.
            // The broker verifies the exact typed delivery while committing
            // the closed four-artifact bundle under the active lease.
            let object = serde_json::to_value(&artifact.object).map_err(|error| {
                StoreError::InvalidRecord(format!("export object serialization failed: {error}"))
            })?;
            let (delivery_payload, delivery_sha256) = encode_payload(&delivery)?;
            artifacts.push(serde_json::json!({
                "kind": export_artifact_kind_db(artifact.kind),
                "object": artifact.object.id.as_uuid().to_string(),
                "filename": artifact.filename,
                "mediaType": artifact.object.media_type,
                "objectRecord": object,
                "delivery": delivery_payload.0,
                "deliverySha256": delivery_sha256,
            }));
        }
        let disposition: Option<String> =
            sqlx::query_scalar("SELECT ple_commit_export_job($1,$2,$3,$4,$5)")
                .bind(context.tenant_id().as_uuid())
                .bind(commit.job.as_uuid())
                .bind(commit.lease.as_uuid())
                .bind(commit.manifest.as_uuid())
                .bind(serde_json::Value::Array(artifacts))
                .fetch_one(&mut *transaction)
                .await
                .map_err(map_sqlx_error)?;
        let result = match disposition.as_deref() {
            Some("committed") => ExportCommitDisposition::Committed,
            Some("already_committed") => ExportCommitDisposition::AlreadyCommitted,
            _ => return Err(StoreError::Conflict),
        };
        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(result)
    }
}

fn fresh_export_object_id() -> Result<ObjectId, StoreError> {
    crate::random_uuid::random_uuid_v4(|error| {
        StoreError::Unavailable(format!("export object ID randomness unavailable: {error}"))
    })
    .map(ObjectId::from_uuid)
}

fn export_artifact_kind_db(kind: ExportArtifactKind) -> &'static str {
    match kind {
        ExportArtifactKind::Docx => "docx",
        ExportArtifactKind::Pdf => "pdf",
        ExportArtifactKind::AccessibleDocx => "accessibleDocx",
        ExportArtifactKind::AccessiblePdf => "accessiblePdf",
    }
}

fn export_artifact_kind_from_db(value: &str) -> Result<ExportArtifactKind, StoreError> {
    match value {
        "docx" => Ok(ExportArtifactKind::Docx),
        "pdf" => Ok(ExportArtifactKind::Pdf),
        "accessibleDocx" => Ok(ExportArtifactKind::AccessibleDocx),
        "accessiblePdf" => Ok(ExportArtifactKind::AccessiblePdf),
        _ => Err(StoreError::Unavailable(
            "unknown stored export artifact kind".to_string(),
        )),
    }
}

fn validate_export_artifacts(
    tenant: TenantId,
    artifacts: &[ExportArtifactRecord],
) -> Result<(), StoreError> {
    if artifacts.len() != 4 {
        return Err(StoreError::InvalidRecord(
            "an export effect must contain exactly four artifacts".to_string(),
        ));
    }
    let mut kinds = std::collections::BTreeSet::new();
    let mut objects = std::collections::BTreeSet::new();
    for artifact in artifacts {
        let expected_filename = match artifact.kind {
            ExportArtifactKind::Docx => "exam.docx",
            ExportArtifactKind::Pdf => "exam.pdf",
            ExportArtifactKind::AccessibleDocx => "exam-accessible.docx",
            ExportArtifactKind::AccessiblePdf => "exam-accessible.pdf",
        };
        if !kinds.insert(artifact.kind)
            || !objects.insert(artifact.object.id)
            || artifact.filename != expected_filename
            || artifact.object.media_type != artifact.kind.media_type()
            || !matches!(&artifact.object.key, objects::ObjectKey::StudentRecord { tenant: key_tenant, object }
                if *key_tenant == tenant && *object == artifact.object.id)
        {
            return Err(StoreError::InvalidRecord(
                "export artifact does not match its closed private output contract".to_string(),
            ));
        }
    }
    Ok(())
}

async fn get_postgres_export_view(
    store: &PostgresStore,
    context: TenantContext,
    export: ExportId,
    requester: Option<UserId>,
) -> Result<Option<StudentExportView>, StoreError> {
    let mut transaction = store.begin_tenant(context).await?;
    let row = sqlx::query(
        "SELECT assignment_id, state FROM student_export_request WHERE export_id = $1 \
         AND ($2::uuid IS NULL OR requester_id = $2)",
    )
    .bind(export.as_uuid())
    .bind(requester.map(|value| value.as_uuid()))
    .fetch_optional(&mut *transaction)
    .await
    .map_err(map_sqlx_error)?;
    let view = if let Some(row) = row {
        let state = match row
            .try_get::<String, _>("state")
            .map_err(map_sqlx_error)?
            .as_str()
        {
            "queued" => StudentExportState::Queued,
            "ready" => StudentExportState::Ready,
            "failed" => StudentExportState::Failed,
            _ => {
                return Err(StoreError::Unavailable(
                    "unknown stored export state".to_string(),
                ));
            }
        };
        let artifacts = if state == StudentExportState::Ready {
            let rows = sqlx::query(
                "SELECT kind, filename, media_type, delivery_id FROM student_export_artifact \
                 WHERE export_id = $1 ORDER BY kind",
            )
            .bind(export.as_uuid())
            .fetch_all(&mut *transaction)
            .await
            .map_err(map_sqlx_error)?;
            Some(
                rows.into_iter()
                    .map(|artifact| {
                        Ok(StudentExportArtifactView {
                            kind: export_artifact_kind_from_db(
                                &artifact
                                    .try_get::<String, _>("kind")
                                    .map_err(map_sqlx_error)?,
                            )?,
                            filename: artifact.try_get("filename").map_err(map_sqlx_error)?,
                            media_type: artifact.try_get("media_type").map_err(map_sqlx_error)?,
                            delivery: AssetDeliveryId::from_object(ObjectId::from_uuid(
                                artifact.try_get("delivery_id").map_err(map_sqlx_error)?,
                            )),
                        })
                    })
                    .collect::<Result<Vec<_>, StoreError>>()?,
            )
        } else {
            None
        };
        Some(StudentExportView {
            id: export,
            assignment: AssignmentId::from_uuid(
                row.try_get("assignment_id").map_err(map_sqlx_error)?,
            ),
            state,
            artifacts,
        })
    } else {
        None
    };
    transaction.commit().await.map_err(map_sqlx_error)?;
    Ok(view)
}

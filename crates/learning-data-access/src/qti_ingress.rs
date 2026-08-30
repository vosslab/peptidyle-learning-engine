//! Atomic workspace QTI queueing and coarse import status.

use async_trait::async_trait;
use objects::{Bucket, ObjectCategory, ObjectKey, ObjectRecord, workspace_qti_archive_object_id};
use question_model::{WorkspaceId, WorkspaceImportId};
use uuid::Uuid;

use crate::{
    EnqueueJob, JobId, JobPayload, QTI_PROFILE_ARCHIVE_MEDIA_TYPE, QtiImportRef, QtiImportRegistry,
    ActorContext, StoreError, flat_import_provenance::MAX_QTI_PROFILE_ARCHIVE_BYTES,
};

const QTI_IMPORT_JOB_ID_DOMAIN: &[u8] = b"ple-qti-import-job/v1\0";

/// Server-owned request to queue the one deterministic job for an import.
///
/// `source` must be the authoritative record returned by the immutable object
/// write or by an independent replay read. This Store validates its complete
/// typed shape but does not own object bytes or perform the object-store read.
#[derive(Clone, PartialEq, Eq)]
pub struct QueueQtiImportCommand {
    pub reference: QtiImportRef,
    pub source: ObjectRecord,
    pub max_attempts: u16,
}

/// Coarse server-side lifecycle derived from the bound job and committed import.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QtiImportApiState {
    Queued,
    Processing,
    Ready,
    Failed,
}

/// Private route-service view of one durable QTI request.
///
/// This intentionally has no serialization or debug implementation. A ready
/// registry contains private object metadata and must be projected into the
/// separate answer-free HTTP report DTO by the server.
#[derive(Clone, PartialEq, Eq)]
pub struct QtiImportApiView {
    pub reference: QtiImportRef,
    pub state: QtiImportApiState,
    pub registry: Option<QtiImportRegistry>,
}

/// Atomic queueing boundary for author-uploaded QTI archives.
#[async_trait]
pub trait QtiImportApiStore: Send + Sync {
    /// Rechecks workspace access and inserts or exactly replays one job.
    async fn queue_qti_import(
        &self,
        actor: ActorContext,
        command: QueueQtiImportCommand,
    ) -> Result<QtiImportApiView, StoreError>;

    /// Resolves status only for an actor currently bound to the workspace.
    async fn qti_import_view(
        &self,
        actor: ActorContext,
        workspace: WorkspaceId,
        import: WorkspaceImportId,
    ) -> Result<Option<QtiImportApiView>, StoreError>;
}

/// Derives the sole durable queue identity for one private QTI import.
pub fn qti_import_job_id(reference: QtiImportRef) -> JobId {
    let mut input = Vec::with_capacity(QTI_IMPORT_JOB_ID_DOMAIN.len() + 48);
    input.extend_from_slice(QTI_IMPORT_JOB_ID_DOMAIN);
    input.extend_from_slice(reference.workspace.as_uuid().as_bytes());
    input.extend_from_slice(reference.import.as_uuid().as_bytes());
    let digest = objects::Sha256Digest::compute(&input);
    let mut bytes = [0_u8; 16];
    bytes.copy_from_slice(&digest.as_bytes()[..16]);
    JobId::from_uuid(Uuid::from_bytes(bytes))
}

pub(crate) fn validate_queue_qti_import(command: &QueueQtiImportCommand) -> Result<(), StoreError> {
    let ObjectKey::WorkspaceSource {
        workspace,
        import,
        object,
    } = command.source.key
    else {
        return Err(StoreError::InvalidRecord(
            "QTI ingress requires a workspace import archive".to_string(),
        ));
    };
    if workspace != command.reference.workspace
        || import != command.reference.import
        || object != command.source.id
        || object
            != workspace_qti_archive_object_id(
                command.reference.workspace,
                command.reference.import,
            )
        || command.source.bucket != Bucket::PrivateContent
        || command.source.key.bucket() != Bucket::PrivateContent
        || command.source.category != ObjectCategory::Source
        || command.source.key.category() != ObjectCategory::Source
        || command.source.version.is_some()
        || command.source.key.version_id().is_some()
        || command.source.media_type != QTI_PROFILE_ARCHIVE_MEDIA_TYPE
        || command.source.size_bytes == 0
        || command.source.size_bytes > MAX_QTI_PROFILE_ARCHIVE_BYTES
        || !crate::publication_validation::flat_import_archive_annotations_are_valid(
            &command.source,
        )
    {
        return Err(StoreError::InvalidRecord(
            "QTI ingress archive metadata is invalid".to_string(),
        ));
    }
    EnqueueJob {
        payload: JobPayload::QtiImport {
            workspace: command.reference.workspace,
            import: command.reference.import,
            source_object: command.source.id,
        },
        max_attempts: command.max_attempts,
    }
    .validate()
}

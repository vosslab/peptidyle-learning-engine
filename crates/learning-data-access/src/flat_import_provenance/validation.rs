//! Structural validation kept separate from the provenance data model.

use objects::{
    Bucket, ObjectCategory, ObjectKey, ObjectRecord, published_import_archive_object_id,
};
use question_model::{DraftQuestionSource, ProblemVersionRef};

use super::{
    FlatQuestionGradingPayload, MAX_QTI_PROFILE_ARCHIVE_BYTES, QTI_PROFILE_ARCHIVE_MEDIA_TYPE,
    QtiImportRef, StoreError, WorkspaceFlatImportOrigin,
};
use crate::DraftRecord;

pub(super) fn validate_source_item_identifier(value: &str) -> Result<(), StoreError> {
    if value.trim().is_empty() || value.chars().count() > super::MAX_SOURCE_ITEM_IDENTIFIER_CHARS {
        return Err(StoreError::InvalidRecord(
            "flat-import source item identifier is invalid".to_string(),
        ));
    }
    Ok(())
}

pub(super) fn validate_workspace_archive(
    record: &ObjectRecord,
    reference: QtiImportRef,
) -> Result<(), StoreError> {
    let ObjectKey::WorkspaceSource {
        tenant,
        workspace,
        import,
        object,
    } = record.key
    else {
        return Err(StoreError::InvalidRecord(
            "flat-import origin requires a workspace import archive".to_string(),
        ));
    };
    if tenant != reference.tenant
        || workspace != reference.workspace
        || import != reference.import
        || object != record.id
        || record.bucket != Bucket::Content
        || record.key.bucket() != Bucket::Content
        || record.category != ObjectCategory::Source
        || record.key.category() != ObjectCategory::Source
        || record.version.is_some()
        || record.media_type != QTI_PROFILE_ARCHIVE_MEDIA_TYPE
        || record.size_bytes == 0
        || record.size_bytes > MAX_QTI_PROFILE_ARCHIVE_BYTES
        || !crate::publication_validation::flat_import_archive_annotations_are_valid(record)
    {
        return Err(StoreError::InvalidRecord(
            "flat-import workspace archive metadata is invalid".to_string(),
        ));
    }
    Ok(())
}

pub(super) fn validate_published_archive(
    current: &WorkspaceFlatImportOrigin,
    reference: ProblemVersionRef,
    record: &ObjectRecord,
) -> Result<(), StoreError> {
    let ObjectKey::PublishedImportArchive {
        tenant,
        problem,
        version,
        import,
        object,
    } = record.key
    else {
        return Err(StoreError::InvalidRecord(
            "flat-import publication requires a published archive key".to_string(),
        ));
    };
    let expected_object = published_import_archive_object_id(
        current.import.tenant,
        reference.problem,
        reference.version,
        current.import.import,
        current.source_archive.sha256,
    );
    if tenant != current.import.tenant
        || problem != reference.problem
        || version != reference.version
        || import != current.import.import
        || object != expected_object
        || record.id != expected_object
        || record.bucket != Bucket::Content
        || record.category != ObjectCategory::Source
        || record.version != Some(reference.version)
        || record.media_type != QTI_PROFILE_ARCHIVE_MEDIA_TYPE
        || record.size_bytes != current.source_archive.size_bytes
        || record.sha256 != current.source_archive.sha256
        || !crate::publication_validation::flat_import_archive_annotations_are_valid(record)
    {
        return Err(StoreError::InvalidRecord(
            "flat-import published archive metadata is invalid".to_string(),
        ));
    }
    Ok(())
}

pub(super) fn validate_conversion_inputs(
    draft: &DraftRecord,
    source: &ObjectRecord,
    canonical_source_sha256: &str,
    public_binding_sha256: &str,
    grading: &FlatQuestionGradingPayload,
    origin: &WorkspaceFlatImportOrigin,
) -> Result<(), StoreError> {
    if draft.tenant != origin.import.tenant || draft.question.workspace != origin.import.workspace {
        return Err(StoreError::InvalidRecord(
            "flat-import origin does not match the draft workspace".to_string(),
        ));
    }
    if !matches!(
        &draft.question.source,
        DraftQuestionSource::Native { family } if family == "flat_single_choice_v1"
    ) {
        return Err(StoreError::InvalidRecord(
            "flat-import conversion requires the flat single-choice family".to_string(),
        ));
    }
    crate::flat_question::validate_workspace_flat_source_record(
        &draft.tenant,
        &draft.question.workspace,
        source,
    )?;
    if source.sha256.to_string() != canonical_source_sha256
        || source.sha256 != origin.evidence.mapped_canonical_source_sha256
        || grading.public_binding_sha256() != public_binding_sha256
    {
        return Err(StoreError::InvalidRecord(
            "flat-import conversion source binding is invalid".to_string(),
        ));
    }
    let private = grading.decode_private()?;
    private.validate_for_draft(&draft.question).map_err(|_| {
        StoreError::InvalidRecord("flat-import grading binding is invalid".to_string())
    })?;
    Ok(())
}

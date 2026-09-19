//! Row decode helpers for Blueprint Course summaries, content, and metadata.

use question_model::{
    BlueprintAvailability, BlueprintCourseId, BlueprintCourseReadAccess, BlueprintEditNumber,
    BlueprintMetadataState, BlueprintRevision, BlueprintRevisionReference, Timestamp,
};
use serde_json::Value;
use sqlx::Row;
use sqlx::types::Json;

use super::super::connection::map_sqlx_error;
use super::classification::decode_classification;
use crate::{
    StoreError, StoredBlueprintCourse, StoredBlueprintCourseContent, StoredBlueprintCourseSummary,
};

pub(super) fn decode_summary(
    row: &sqlx::postgres::PgRow,
) -> Result<StoredBlueprintCourseSummary, StoreError> {
    Ok(StoredBlueprintCourseSummary {
        classification: decode_classification(row)?,
        total_adoptions: u64::try_from(
            row.try_get::<i64, _>("total_adoptions")
                .map_err(map_sqlx_error)?,
        )
        .map_err(|_| invalid("Blueprint adoption count"))?,
        total_students_ever_enrolled: u64::try_from(
            row.try_get::<i64, _>("total_students_ever_enrolled")
                .map_err(map_sqlx_error)?,
        )
        .map_err(|_| invalid("Blueprint enrollment count"))?,
        id: reference(row.try_get("blueprint_course_id").map_err(map_sqlx_error)?)?,
        short_name: row.try_get("short_name").map_err(map_sqlx_error)?,
        long_name: row.try_get("long_name").map_err(map_sqlx_error)?,
        availability: availability_value(row.try_get("availability").map_err(map_sqlx_error)?)?,
        blueprint_edit_number: blueprint_edit_number(
            row.try_get("blueprint_edit_number")
                .map_err(map_sqlx_error)?,
        ),
        current_revision: revision(
            row.try_get("current_blueprint_revision_number")
                .map_err(map_sqlx_error)?,
        )?,
        read_access: read_access(row.try_get("is_owner").map_err(map_sqlx_error)?),
    })
}

pub(super) fn decode_course(
    row: &sqlx::postgres::PgRow,
) -> Result<StoredBlueprintCourse, StoreError> {
    let fork_source_blueprint_course_id: Option<String> = row
        .try_get("fork_source_blueprint_course_id")
        .map_err(map_sqlx_error)?;
    let fork_source_revision: Option<i64> = row
        .try_get("fork_source_revision_number")
        .map_err(map_sqlx_error)?;
    let fork_source = match (fork_source_blueprint_course_id, fork_source_revision) {
        (Some(source), Some(number)) => Some(BlueprintRevisionReference {
            blueprint_course_id: reference(source)?,
            revision: revision(number)?,
        }),
        (None, None) => None,
        _ => return Err(invalid("fork origin")),
    };
    let Json(encoded): Json<Value> = row.try_get("content").map_err(map_sqlx_error)?;
    let content = decode_revision_content(
        encoded,
        &row.try_get::<Vec<u8>, _>("content_checksum")
            .map_err(map_sqlx_error)?,
    )?;
    Ok(StoredBlueprintCourse {
        classification: decode_classification(row)?,
        id: reference(row.try_get("blueprint_course_id").map_err(map_sqlx_error)?)?,
        short_name: row.try_get("short_name").map_err(map_sqlx_error)?,
        long_name: row.try_get("long_name").map_err(map_sqlx_error)?,
        availability: availability_value(row.try_get("availability").map_err(map_sqlx_error)?)?,
        blueprint_edit_number: blueprint_edit_number(
            row.try_get("blueprint_edit_number")
                .map_err(map_sqlx_error)?,
        ),
        current_revision: revision(
            row.try_get("current_blueprint_revision_number")
                .map_err(map_sqlx_error)?,
        )?,
        read_access: read_access(row.try_get("is_owner").map_err(map_sqlx_error)?),
        content,
        fork_source,
    })
}

pub(in crate::postgres) fn encode_content(
    content: &StoredBlueprintCourseContent,
) -> Result<Value, StoreError> {
    serde_json::to_value(content).map_err(|_| invalid("Blueprint Content"))
}

pub(super) fn decode_stored_content(
    encoded: Value,
) -> Result<StoredBlueprintCourseContent, StoreError> {
    serde_json::from_value(encoded).map_err(|_| invalid("Blueprint Content"))
}

/// Decode the exact retained content through the ordinary checksum boundary.
pub(in crate::postgres) fn decode_revision_content(
    encoded: Value,
    expected: &[u8],
) -> Result<StoredBlueprintCourseContent, StoreError> {
    let content = decode_stored_content(encoded)?;
    verify_content_checksum(&content, expected)?;
    Ok(content)
}

pub(super) fn verify_content_checksum(
    content: &StoredBlueprintCourseContent,
    expected: &[u8],
) -> Result<(), StoreError> {
    (expected == content.checksum()?.as_bytes())
        .then_some(())
        .ok_or_else(|| invalid("Blueprint Content Checksum"))
}

pub(super) fn decode_metadata_state(
    row: &sqlx::postgres::PgRow,
) -> Result<BlueprintMetadataState, StoreError> {
    Ok(BlueprintMetadataState {
        classification: decode_classification(row)?,
        short_name: row.try_get("short_name").map_err(map_sqlx_error)?,
        long_name: row.try_get("long_name").map_err(map_sqlx_error)?,
        availability: availability_value(row.try_get("availability").map_err(map_sqlx_error)?)?,
        blueprint_edit_number: blueprint_edit_number(
            row.try_get("blueprint_edit_number")
                .map_err(map_sqlx_error)?,
        ),
    })
}

pub(super) fn read_access(is_owner: bool) -> BlueprintCourseReadAccess {
    if is_owner {
        BlueprintCourseReadAccess::BlueprintCourseOwner
    } else {
        BlueprintCourseReadAccess::ActiveInstructor
    }
}

pub(super) fn availability_value(value: String) -> Result<BlueprintAvailability, StoreError> {
    match value.as_str() {
        "private" => Ok(BlueprintAvailability::Private),
        "public" => Ok(BlueprintAvailability::Public),
        "archived" => Ok(BlueprintAvailability::Archived),
        _ => Err(invalid("Blueprint availability")),
    }
}

pub(super) fn reference(value: String) -> Result<BlueprintCourseId, StoreError> {
    value
        .parse()
        .map_err(|_| invalid("Blueprint Course Reference"))
}
pub(super) fn revision(value: i64) -> Result<BlueprintRevision, StoreError> {
    u64::try_from(value)
        .ok()
        .and_then(BlueprintRevision::new)
        .ok_or_else(|| invalid("Blueprint Revision"))
}
pub(super) fn blueprint_edit_number(value: i64) -> BlueprintEditNumber {
    BlueprintEditNumber::from_edit_number(value)
}
pub(super) fn revision_number(value: BlueprintRevision) -> Result<i64, StoreError> {
    i64::try_from(value.value()).map_err(|_| invalid("Blueprint Revision"))
}
pub(super) fn timestamp(value: i64) -> Result<Timestamp, StoreError> {
    Ok(Timestamp::from_unix_millis(value))
}
pub(super) fn invalid_input(error: question_model::BlueprintCourseValidationError) -> StoreError {
    StoreError::InvalidRecord(format!("Blueprint Course request is invalid: {error}"))
}
pub(super) fn invalid(label: &str) -> StoreError {
    StoreError::InvalidRecord(format!("database returned an invalid {label}"))
}

//! PostgreSQL external-tool payload and persisted-binding validation.

use objects::Sha256Digest;
use question_model::{ObjectId, ProblemId, StudentResponse, VersionId};
use sqlx::Row;
use sqlx::postgres::PgRow;

use super::super::map_sqlx_error;
use crate::{ExternalToolBinding, StoreError, StudentWorkRoutingBinding};

pub(super) fn postgres_validate_external_response(
    response: &StudentResponse,
    binding: &ExternalToolBinding,
) -> Result<(), StoreError> {
    if !matches!(response, StudentResponse::ExternalTool {}) {
        return Err(StoreError::InvalidRecord(
            "external-tool exchange requires the external marker response".to_string(),
        ));
    }
    binding.validate()?;
    let canonical = serde_json::to_vec(response).map_err(|error| {
        StoreError::InvalidRecord(format!("external response encoding failed: {error}"))
    })?;
    if Sha256Digest::compute(&canonical) != binding.response_sha256 {
        return Err(StoreError::Conflict);
    }
    Ok(())
}

pub(super) fn postgres_external_binding(row: &PgRow) -> Result<ExternalToolBinding, StoreError> {
    let response: Vec<u8> = row.try_get("response_sha256").map_err(map_sqlx_error)?;
    let response: [u8; 32] = response.try_into().map_err(|_| {
        StoreError::InvalidRecord("stored external response checksum is malformed".to_string())
    })?;
    Ok(ExternalToolBinding {
        provider: row.try_get("provider").map_err(map_sqlx_error)?,
        problem: ProblemId::from_uuid(row.try_get("problem_id").map_err(map_sqlx_error)?),
        version: VersionId::from_uuid(row.try_get("version_id").map_err(map_sqlx_error)?),
        seed: row.try_get::<i64, _>("seed").map_err(map_sqlx_error)? as u64,
        source_object: ObjectId::from_uuid(
            row.try_get("source_object_id").map_err(map_sqlx_error)?,
        ),
        source_sha256: row.try_get("source_sha256").map_err(map_sqlx_error)?,
        integration_profile: row.try_get("integration_profile").map_err(map_sqlx_error)?,
        response_sha256: Sha256Digest::from_bytes(response),
    })
}

pub(super) fn postgres_stored_course_matches(
    row: &PgRow,
    student_work_binding: StudentWorkRoutingBinding,
) -> Result<bool, StoreError> {
    Ok(row
        .try_get::<uuid::Uuid, _>("course_id")
        .map_err(map_sqlx_error)?
        == student_work_binding.course.as_uuid())
}

pub(super) fn postgres_binding_matches(
    stored: &ExternalToolBinding,
    requested: &ExternalToolBinding,
) -> bool {
    stored.provider == requested.provider
        && stored.problem == requested.problem
        && stored.version == requested.version
        && stored.seed == requested.seed
        && stored.source_object == requested.source_object
        && stored.source_sha256 == requested.source_sha256
        && stored.integration_profile == requested.integration_profile
}

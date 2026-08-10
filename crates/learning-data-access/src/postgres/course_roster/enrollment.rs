//! PostgreSQL creation of the learner enrollment and its empty gradebook projection.

use question_model::{
    AssignmentEnrollment, AssignmentId, EnrollmentId, StudentAssignmentSummary, StudentId,
    TenantId, UserId,
};
use sqlx::types::Uuid;
use sqlx::{Postgres, Transaction};

use super::super::{encode_payload, map_sqlx_error};
use crate::StoreError;

pub(super) async fn insert_missing_enrollment(
    transaction: &mut Transaction<'_, Postgres>,
    tenant: TenantId,
    assignment: AssignmentId,
    user: UserId,
    student: StudentId,
) -> Result<(), StoreError> {
    let existing: Option<(Uuid, Uuid)> = sqlx::query_as(
        "SELECT enrollment_id, student_id FROM enrollment \
         WHERE tenant_id = $1 AND assignment_id = $2 AND user_id = $3",
    )
    .bind(tenant.as_uuid())
    .bind(assignment.as_uuid())
    .bind(user.as_uuid())
    .fetch_optional(&mut **transaction)
    .await
    .map_err(map_sqlx_error)?;
    if let Some((enrollment, stored_student)) = existing {
        if stored_student != student.as_uuid() {
            return Err(StoreError::Conflict);
        }
        let summary_exists: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM student_assignment_summary \
             WHERE tenant_id = $1 AND enrollment_id = $2)",
        )
        .bind(tenant.as_uuid())
        .bind(enrollment)
        .fetch_one(&mut **transaction)
        .await
        .map_err(map_sqlx_error)?;
        return summary_exists
            .then_some(())
            .ok_or_else(|| StoreError::Unavailable("enrollment summary is missing".to_string()));
    }

    let enrollment_id = EnrollmentId::from_uuid(random_uuid("enrollment ID")?);
    let enrollment = AssignmentEnrollment {
        id: enrollment_id,
        tenant,
        assignment,
        user,
        student,
        first_completed_at: None,
        current_grade_run: None,
        best_grade_run: None,
    };
    let summary = StudentAssignmentSummary::empty(tenant, enrollment_id);
    let (enrollment_payload, enrollment_checksum) = encode_payload(&enrollment)?;
    let (summary_payload, summary_checksum) = encode_payload(&summary)?;
    sqlx::query(
        "INSERT INTO enrollment \
         (tenant_id, enrollment_id, assignment_id, user_id, student_id, payload, payload_sha256) \
         VALUES ($1, $2, $3, $4, $5, $6, $7)",
    )
    .bind(tenant.as_uuid())
    .bind(enrollment_id.as_uuid())
    .bind(assignment.as_uuid())
    .bind(user.as_uuid())
    .bind(student.as_uuid())
    .bind(enrollment_payload)
    .bind(enrollment_checksum)
    .execute(&mut **transaction)
    .await
    .map_err(map_sqlx_error)?;
    sqlx::query(
        "INSERT INTO student_assignment_summary \
         (tenant_id, enrollment_id, payload, payload_sha256) VALUES ($1, $2, $3, $4)",
    )
    .bind(tenant.as_uuid())
    .bind(enrollment_id.as_uuid())
    .bind(summary_payload)
    .bind(summary_checksum)
    .execute(&mut **transaction)
    .await
    .map_err(map_sqlx_error)?;
    Ok(())
}

fn random_uuid(label: &str) -> Result<Uuid, StoreError> {
    let mut bytes = [0_u8; 16];
    getrandom::fill(&mut bytes).map_err(|error| {
        StoreError::Unavailable(format!("{label} randomness unavailable: {error}"))
    })?;
    Ok(Uuid::from_bytes(bytes))
}

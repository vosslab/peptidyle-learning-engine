//! PostgreSQL enrollment, run, summary, and catalog-record helpers.

use super::*;

#[cfg(feature = "postgres")]
pub(crate) async fn load_enrollment_for_update(
    transaction: &mut Transaction<'_, Postgres>,
    tenant: TenantId,
    enrollment: EnrollmentId,
) -> Result<AssignmentEnrollment, StoreError> {
    let row = sqlx::query(
        "SELECT enrollment_id, tenant_id, assignment_id, user_id, student_id, \
                floor(extract(epoch FROM first_completed_at) * 1000)::bigint \
                    AS first_completed_at_millis, \
                current_grade_run_id, best_grade_run_id FROM enrollment \
         WHERE tenant_id = $1 AND enrollment_id = $2 FOR UPDATE",
    )
    .bind(tenant.as_uuid())
    .bind(enrollment.as_uuid())
    .fetch_optional(&mut **transaction)
    .await
    .map_err(map_sqlx_error)?
    .ok_or(StoreError::NotFound)?;
    decode_postgres_enrollment_row(&row)
}
/// Decodes the normalized mutable enrollment state.  Enrollment provenance and
/// entitlement scopes are separate immutable relations, so this deliberately
/// has no payload or checksum compatibility path.
#[cfg(feature = "postgres")]
pub(crate) fn decode_postgres_enrollment_row(
    row: &PgRow,
) -> Result<AssignmentEnrollment, StoreError> {
    Ok(AssignmentEnrollment {
        id: EnrollmentId::from_uuid(row.try_get("enrollment_id").map_err(map_sqlx_error)?),
        tenant: TenantId::from_uuid(row.try_get("tenant_id").map_err(map_sqlx_error)?),
        assignment: AssignmentId::from_uuid(row.try_get("assignment_id").map_err(map_sqlx_error)?),
        user: UserId::from_uuid(row.try_get("user_id").map_err(map_sqlx_error)?),
        student: StudentId::from_uuid(row.try_get("student_id").map_err(map_sqlx_error)?),
        first_completed_at: row
            .try_get::<Option<i64>, _>("first_completed_at_millis")
            .map_err(map_sqlx_error)?
            .map(ActivityTimestamp::from_unix_millis),
        current_grade_run: row
            .try_get::<Option<Uuid>, _>("current_grade_run_id")
            .map_err(map_sqlx_error)?
            .map(RunId::from_uuid),
        best_grade_run: row
            .try_get::<Option<Uuid>, _>("best_grade_run_id")
            .map_err(map_sqlx_error)?
            .map(RunId::from_uuid),
    })
}

/// Loads the relational enrollment representation without taking an update
/// lock.  Readers that mutate the enrollment use [`load_enrollment_for_update`].
#[cfg(feature = "postgres")]
pub(crate) async fn load_postgres_enrollment(
    transaction: &mut Transaction<'_, Postgres>,
    tenant: TenantId,
    enrollment: EnrollmentId,
) -> Result<AssignmentEnrollment, StoreError> {
    let row = sqlx::query(
        "SELECT enrollment_id, tenant_id, assignment_id, user_id, student_id, \
                floor(extract(epoch FROM first_completed_at) * 1000)::bigint \
                    AS first_completed_at_millis, \
                current_grade_run_id, best_grade_run_id FROM enrollment \
         WHERE tenant_id = $1 AND enrollment_id = $2",
    )
    .bind(tenant.as_uuid())
    .bind(enrollment.as_uuid())
    .fetch_optional(&mut **transaction)
    .await
    .map_err(map_sqlx_error)?
    .ok_or(StoreError::NotFound)?;
    decode_postgres_enrollment_row(&row)
}

#[cfg(feature = "postgres")]
pub(crate) async fn load_run_for_update(
    transaction: &mut Transaction<'_, Postgres>,
    tenant: TenantId,
    run: RunId,
) -> Result<AssignmentRun, StoreError> {
    let row = sqlx::query(
        "SELECT payload, payload_sha256 FROM assignment_run \
         WHERE tenant_id = $1 AND run_id = $2 FOR UPDATE",
    )
    .bind(tenant.as_uuid())
    .bind(run.as_uuid())
    .fetch_optional(&mut **transaction)
    .await
    .map_err(map_sqlx_error)?
    .ok_or(StoreError::NotFound)?;
    decode_payload_row(&row)
}

/// Loads one immutable run projection without taking a mutation lock.
/// Runtime transitions that change run state use [`load_run_for_update`].
#[cfg(feature = "postgres")]
pub(crate) async fn load_postgres_run(
    transaction: &mut Transaction<'_, Postgres>,
    tenant: TenantId,
    run: RunId,
) -> Result<AssignmentRun, StoreError> {
    let row = sqlx::query(
        "SELECT payload, payload_sha256 FROM assignment_run \
         WHERE tenant_id = $1 AND run_id = $2",
    )
    .bind(tenant.as_uuid())
    .bind(run.as_uuid())
    .fetch_optional(&mut **transaction)
    .await
    .map_err(map_sqlx_error)?
    .ok_or(StoreError::NotFound)?;
    decode_payload_row(&row)
}

#[cfg(feature = "postgres")]
pub(crate) async fn load_summary_for_update(
    transaction: &mut Transaction<'_, Postgres>,
    tenant: TenantId,
    enrollment: EnrollmentId,
) -> Result<StudentAssignmentSummary, StoreError> {
    let row = sqlx::query(
        "SELECT tenant_id, enrollment_id, current_score, best_score, latest_score, \
                completed_run_count, total_question_attempts, \
                floor(extract(epoch FROM last_activity_at) * 1000)::bigint \
                    AS last_activity_at_millis \
         FROM student_assignment_summary \
         WHERE tenant_id = $1 AND enrollment_id = $2 FOR UPDATE",
    )
    .bind(tenant.as_uuid())
    .bind(enrollment.as_uuid())
    .fetch_optional(&mut **transaction)
    .await
    .map_err(map_sqlx_error)?
    .ok_or(StoreError::NotFound)?;
    decode_summary_row(&row)
}

#[cfg(feature = "postgres")]
pub(crate) async fn store_summary(
    transaction: &mut Transaction<'_, Postgres>,
    summary: &StudentAssignmentSummary,
) -> Result<(), StoreError> {
    let updated = sqlx::query(
        "UPDATE student_assignment_summary \
         SET current_score = $3, best_score = $4, latest_score = $5, \
             completed_run_count = $6, total_question_attempts = $7, \
             last_activity_at = to_timestamp($8::double precision / 1000), \
             updated_at = transaction_timestamp() \
         WHERE tenant_id = $1 AND enrollment_id = $2",
    )
    .bind(summary.tenant.as_uuid())
    .bind(summary.enrollment.as_uuid())
    .bind(summary.current_score)
    .bind(summary.best_score)
    .bind(summary.latest_score)
    .bind(i64::from(summary.completed_run_count))
    .bind(i64::try_from(summary.total_question_attempts).map_err(|_| StoreError::Conflict)?)
    .bind(summary.last_activity_at.map(|value| value.as_unix_millis()))
    .execute(&mut **transaction)
    .await
    .map_err(map_sqlx_error)?;
    if updated.rows_affected() != 1 {
        return Err(StoreError::NotFound);
    }
    Ok(())
}

#[cfg(feature = "postgres")]
pub(crate) async fn insert_problem_version(
    transaction: &mut Transaction<'_, Postgres>,
    record: &PublishedProblemRecord,
    content_sha256: &str,
) -> Result<(), StoreError> {
    let backend = question_backend_name(QuestionBackend::from(&record.question.source));
    let (lifecycle, lifecycle_reason) = catalog_lifecycle_parts(&record.lifecycle);
    let derived_from_problem = record.derived_from.map(|source| source.problem.as_uuid());
    let derived_from_version = record.derived_from.map(|source| source.version.as_uuid());
    sqlx::query(
        "INSERT INTO problem_version \
         (problem_id, version_id, content_sha256, workspace_id, title, \
          backend, capabilities, metadata, \
          publication_scope, lifecycle, lifecycle_reason, author_ids, public_byline, \
          derived_from_problem_id, derived_from_version_id) \
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15)",
    )
    .bind(record.problem.as_uuid())
    .bind(record.version.as_uuid())
    .bind(content_sha256)
    .bind(record.question.workspace.as_uuid())
    .bind(&record.question.metadata.title)
    .bind(backend)
    .bind(Json(record.capabilities.clone()))
    .bind(Json(record.question.metadata.clone()))
    .bind(publication_scope_name(record.scope))
    .bind(lifecycle)
    .bind(lifecycle_reason)
    .bind(Json(record.author_ids.clone()))
    .bind(
        record
            .byline
            .names
            .iter()
            .map(|name| name.as_str())
            .collect::<Vec<_>>(),
    )
    .bind(derived_from_problem)
    .bind(derived_from_version)
    .execute(&mut **transaction)
    .await
    .map_err(map_sqlx_error)?;
    Ok(())
}

/// Persists a server-only source binding in the same transaction that makes
/// its immutable version visible.
#[cfg(feature = "postgres")]
pub(crate) async fn insert_published_source_artifact(
    transaction: &mut Transaction<'_, Postgres>,
    artifact: &PublishedSourceArtifact,
) -> Result<(), StoreError> {
    let (payload, checksum) = encode_payload(artifact)?;
    sqlx::query(
        "INSERT INTO published_source_artifact \
         (problem_id, version_id, backend, object_id, payload, payload_sha256) \
         VALUES ($1, $2, $3, $4, $5, $6)",
    )
    .bind(artifact.reference.problem.as_uuid())
    .bind(artifact.reference.version.as_uuid())
    .bind(question_backend_name(artifact.backend))
    .bind(artifact.object.id.as_uuid())
    .bind(payload)
    .bind(checksum)
    .execute(&mut **transaction)
    .await
    .map_err(map_sqlx_error)?;
    Ok(())
}

/// Inserts a QTI candidate asset while the containing publication transaction
/// is still private. The ordinary asset API deliberately cannot do this: it
/// only accepts assets for an already visible version.
#[cfg(feature = "postgres")]
pub(crate) async fn insert_catalog_asset_delivery(
    transaction: &mut Transaction<'_, Postgres>,
    record: &AssetDeliveryRecord,
) -> Result<(), StoreError> {
    validate_asset_delivery(record)?;
    let AssetDeliveryScope::Catalog { asset, reference } = record.scope else {
        return Err(StoreError::InvalidRecord(
            "QTI promotion assets must be catalog assets".to_string(),
        ));
    };
    let (payload, checksum) = encode_payload(record)?;
    sqlx::query(
        "INSERT INTO asset_delivery \
         (delivery_id, delivery_kind, tenant_id, object_id, problem_id, version_id, \
          asset_id, payload, payload_sha256) \
         VALUES ($1, 'catalog', NULL, $2, $3, $4, $5, $6, $7)",
    )
    .bind(record.id.as_uuid())
    .bind(record.object.id.as_uuid())
    .bind(reference.problem.as_uuid())
    .bind(reference.version.as_uuid())
    .bind(asset.as_uuid())
    .bind(payload)
    .bind(checksum)
    .execute(&mut **transaction)
    .await
    .map_err(map_sqlx_error)?;
    Ok(())
}

#[cfg(feature = "postgres")]
pub(crate) fn question_backend_name(backend: QuestionBackend) -> &'static str {
    match backend {
        QuestionBackend::Native => "native",
        QuestionBackend::Webwork => "webwork",
        QuestionBackend::Qti => "qti",
        QuestionBackend::H5p => "h5p",
        QuestionBackend::Imathas => "imathas",
    }
}

//! PostgreSQL staged roster-import transaction.

use std::collections::BTreeSet;

use question_model::{ActivityTimestamp, CourseId, TenantId};
use sqlx::postgres::PgRow;
use sqlx::{Postgres, Row, Transaction};

use super::{CourseRosterImportPreview, PostgresStore, decode_invitation, map_sqlx_error};
use crate::{
    AuthenticationEmail, CommitCourseRosterImport, CommittedCourseRosterImport,
    CourseRosterImportId, CourseRosterImportRow, CourseRosterImportRowInput,
    CourseRosterImportState, RosterImportRevision, RosterImportRowStatus, RosterRevision,
    SessionTokenHash, StageCourseRosterImport, StoreError, TenantContext,
};

pub(super) async fn stage(
    store: &PostgresStore,
    context: TenantContext,
    session: SessionTokenHash,
    command: StageCourseRosterImport,
) -> Result<CourseRosterImportPreview, StoreError> {
    validate_rows(&command.rows)?;
    let tenant = context.tenant_id();
    let mut transaction = store.begin_tenant(context).await?;
    let import = CourseRosterImportId::generate()?;
    let rows = serde_json::Value::Array(
        command
            .rows
            .iter()
            .map(|row| {
                serde_json::json!({
                    "row_number": row.row_number,
                    "normalized_email": row.email.as_ref().map(AuthenticationEmail::normalized),
                    "delivery_email": row.email.as_ref().map(AuthenticationEmail::delivery),
                    "roster_id": row.roster_id.as_ref().map(|value| value.as_str()),
                })
            })
            .collect(),
    );
    let witness = sqlx::query(
        "SELECT tenant_id, actor_id, course_id, roster_import_id, roster_revision \
         FROM public.ple_stage_course_roster_import_v1($1,$2,$3,$4,$5,$6,$7,$8,$9)",
    )
    .bind(tenant.as_uuid())
    .bind(session.to_string())
    .bind(command.course.as_uuid())
    .bind(import.as_uuid())
    .bind(
        i64::try_from(command.expected_roster_revision.value())
            .map_err(|_| StoreError::Conflict)?,
    )
    .bind(command.normalized_digest.as_bytes().as_slice())
    .bind(command.idempotency_key.as_str())
    .bind(i64::from(command.lifetime.as_seconds()))
    .bind(sqlx::types::Json(rows))
    .fetch_optional(&mut *transaction)
    .await
    .map_err(map_sqlx_error)?
    .ok_or(StoreError::NotFound)?;
    let staged_import = CourseRosterImportId::from_uuid(
        witness
            .try_get("roster_import_id")
            .map_err(map_sqlx_error)?,
    );
    validate_stage_witness(
        &witness,
        tenant,
        command.course,
        staged_import,
        command.expected_roster_revision,
    )?;
    let preview = load_preview(&mut transaction, tenant, command.course, staged_import).await?;
    if preview.roster_revision != command.expected_roster_revision {
        return Err(StoreError::Unavailable(
            "roster import stage returned an invalid revision".into(),
        ));
    }
    transaction.commit().await.map_err(map_sqlx_error)?;
    Ok(preview)
}

pub(super) async fn commit(
    store: &PostgresStore,
    context: TenantContext,
    session: SessionTokenHash,
    command: CommitCourseRosterImport,
) -> Result<CommittedCourseRosterImport, StoreError> {
    let tenant = context.tenant_id();
    let mut transaction = store.begin_tenant(context).await?;
    let bindings = serde_json::Value::Array(
        command
            .invitations
            .iter()
            .map(|binding| {
                serde_json::json!({
                    "row_number": binding.row_number,
                    "token_hex": encode_hex(binding.token_hash.as_bytes()),
                    "idempotency_key": binding.idempotency_key.as_str(),
                    "lifetime": binding.lifetime.as_seconds(),
                })
            })
            .collect(),
    );
    let witness = sqlx::query(
        "SELECT tenant_id, actor_id, course_id, roster_import_id, import_revision, roster_revision \
         FROM public.ple_commit_course_roster_import_v1($1,$2,$3,$4,$5,$6,$7)",
    )
    .bind(tenant.as_uuid())
    .bind(session.to_string())
    .bind(command.course.as_uuid())
    .bind(command.import.as_uuid())
    .bind(
        i64::try_from(command.expected_import_revision.value())
            .map_err(|_| StoreError::Conflict)?,
    )
    .bind(command.idempotency_key.as_str())
    .bind(sqlx::types::Json(bindings))
    .fetch_optional(&mut *transaction)
    .await
    .map_err(map_sqlx_error)?
    .ok_or(StoreError::NotFound)?;
    validate_commit_witness(&witness, tenant, command.course, command.import)?;
    let result = load_committed_from_witness(
        &mut transaction,
        tenant,
        command.course,
        command.import,
        &witness,
    )
    .await?;
    transaction.commit().await.map_err(map_sqlx_error)?;
    Ok(result)
}

fn validate_stage_witness(
    row: &PgRow,
    tenant: TenantId,
    course: CourseId,
    import: CourseRosterImportId,
    expected_revision: RosterRevision,
) -> Result<(), StoreError> {
    let returned_tenant: uuid::Uuid = row.try_get("tenant_id").map_err(map_sqlx_error)?;
    let returned_actor: uuid::Uuid = row.try_get("actor_id").map_err(map_sqlx_error)?;
    let returned_course: uuid::Uuid = row.try_get("course_id").map_err(map_sqlx_error)?;
    let returned_import: uuid::Uuid = row.try_get("roster_import_id").map_err(map_sqlx_error)?;
    let returned_revision =
        RosterRevision::from_stored(row.try_get("roster_revision").map_err(map_sqlx_error)?)?;
    if returned_tenant != tenant.as_uuid()
        || returned_actor.is_nil()
        || returned_course != course.as_uuid()
        || returned_import != import.as_uuid()
        || returned_revision != expected_revision
    {
        return Err(StoreError::Unavailable(
            "roster import stage returned an invalid authority witness".to_string(),
        ));
    }
    Ok(())
}

fn validate_commit_witness(
    row: &PgRow,
    tenant: TenantId,
    course: CourseId,
    import: CourseRosterImportId,
) -> Result<(), StoreError> {
    let returned_tenant: uuid::Uuid = row.try_get("tenant_id").map_err(map_sqlx_error)?;
    let returned_actor: uuid::Uuid = row.try_get("actor_id").map_err(map_sqlx_error)?;
    let returned_course: uuid::Uuid = row.try_get("course_id").map_err(map_sqlx_error)?;
    let returned_import: uuid::Uuid = row.try_get("roster_import_id").map_err(map_sqlx_error)?;
    let import_revision =
        RosterImportRevision::from_stored(row.try_get("import_revision").map_err(map_sqlx_error)?)?;
    let roster_revision =
        RosterRevision::from_stored(row.try_get("roster_revision").map_err(map_sqlx_error)?)?;
    if returned_tenant != tenant.as_uuid()
        || returned_actor.is_nil()
        || returned_course != course.as_uuid()
        || returned_import != import.as_uuid()
        || import_revision.value() < 2
        || roster_revision.value() < 2
    {
        return Err(StoreError::Unavailable(
            "roster import commit returned an invalid authority witness".to_string(),
        ));
    }
    Ok(())
}

async fn load_committed_from_witness(
    transaction: &mut Transaction<'_, Postgres>,
    tenant: TenantId,
    course: CourseId,
    import: CourseRosterImportId,
    witness: &PgRow,
) -> Result<CommittedCourseRosterImport, StoreError> {
    let rows = sqlx::query(
        "SELECT roster_import_row_number, invitation_id, normalized_email, delivery_email, \
                roster_id, invited_by, status, claimed_user_id, \
                floor(extract(epoch FROM created_at) * 1000)::bigint AS created_at_millis, \
                floor(extract(epoch FROM expires_at) * 1000)::bigint AS expires_at_millis \
         FROM course_invitation \
         WHERE tenant_id = $1 AND course_id = $2 AND roster_import_id = $3 \
         ORDER BY roster_import_row_number",
    )
    .bind(tenant.as_uuid())
    .bind(course.as_uuid())
    .bind(import.as_uuid())
    .fetch_all(&mut **transaction)
    .await
    .map_err(map_sqlx_error)?;
    Ok(CommittedCourseRosterImport {
        import,
        import_revision: RosterImportRevision::from_stored(
            witness.try_get("import_revision").map_err(map_sqlx_error)?,
        )?,
        roster_revision: RosterRevision::from_stored(
            witness.try_get("roster_revision").map_err(map_sqlx_error)?,
        )?,
        invitations: rows
            .iter()
            .map(|row| {
                Ok((
                    u16::try_from(
                        row.try_get::<i32, _>("roster_import_row_number")
                            .map_err(map_sqlx_error)?,
                    )
                    .map_err(|_| {
                        StoreError::Unavailable("stored roster row number is invalid".to_string())
                    })?,
                    decode_invitation(row, tenant, course)?,
                ))
            })
            .collect::<Result<Vec<_>, StoreError>>()?,
    })
}

fn encode_hex(bytes: [u8; 32]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn validate_rows(rows: &[CourseRosterImportRowInput]) -> Result<(), StoreError> {
    if rows.is_empty() || rows.len() > crate::MAX_ROSTER_IMPORT_ROWS {
        return Err(StoreError::InvalidRecord(
            "roster import row count is invalid".to_string(),
        ));
    }
    let mut numbers = BTreeSet::new();
    for row in rows {
        row.validate_shape()?;
        if !numbers.insert(row.row_number) {
            return Err(StoreError::InvalidRecord(
                "roster import row number is duplicated".to_string(),
            ));
        }
    }
    Ok(())
}

async fn load_preview(
    transaction: &mut Transaction<'_, Postgres>,
    tenant: TenantId,
    course: CourseId,
    import: CourseRosterImportId,
) -> Result<CourseRosterImportPreview, StoreError> {
    let header = sqlx::query(
        "SELECT roster_revision, revision, status, \
                floor(extract(epoch FROM expires_at) * 1000)::bigint AS expires_at_millis \
         FROM course_roster_import \
         WHERE tenant_id = $1 AND course_id = $2 AND roster_import_id = $3",
    )
    .bind(tenant.as_uuid())
    .bind(course.as_uuid())
    .bind(import.as_uuid())
    .fetch_optional(&mut **transaction)
    .await
    .map_err(map_sqlx_error)?
    .ok_or(StoreError::NotFound)?;
    let rows = sqlx::query(
        "SELECT row_number, normalized_email, delivery_email, roster_id, row_status \
         FROM course_roster_import_row \
         WHERE tenant_id = $1 AND course_id = $2 AND roster_import_id = $3 \
         ORDER BY row_number",
    )
    .bind(tenant.as_uuid())
    .bind(course.as_uuid())
    .bind(import.as_uuid())
    .fetch_all(&mut **transaction)
    .await
    .map_err(map_sqlx_error)?;
    Ok(CourseRosterImportPreview {
        id: import,
        course,
        roster_revision: RosterRevision::from_stored(
            header.try_get("roster_revision").map_err(map_sqlx_error)?,
        )?,
        revision: RosterImportRevision::from_stored(
            header.try_get("revision").map_err(map_sqlx_error)?,
        )?,
        state: import_state(
            &header
                .try_get::<String, _>("status")
                .map_err(map_sqlx_error)?,
        )?,
        expires_at: ActivityTimestamp::from_unix_millis(
            header
                .try_get("expires_at_millis")
                .map_err(map_sqlx_error)?,
        ),
        rows: rows
            .iter()
            .map(decode_preview_row)
            .collect::<Result<Vec<_>, _>>()?,
    })
}

fn decode_preview_row(row: &PgRow) -> Result<CourseRosterImportRow, StoreError> {
    let normalized: Option<String> = row.try_get("normalized_email").map_err(map_sqlx_error)?;
    let delivery: Option<String> = row.try_get("delivery_email").map_err(map_sqlx_error)?;
    let email = match (normalized, delivery) {
        (Some(normalized), Some(delivery)) => {
            let email = AuthenticationEmail::parse(&delivery)
                .map_err(|_| StoreError::Unavailable("stored roster email is invalid".into()))?;
            if email.normalized() != normalized {
                return Err(StoreError::Unavailable(
                    "stored roster email normalization is invalid".into(),
                ));
            }
            Some(email)
        }
        (None, None) => None,
        _ => {
            return Err(StoreError::Unavailable(
                "stored roster email is incomplete".into(),
            ));
        }
    };
    Ok(CourseRosterImportRow {
        row_number: u16::try_from(
            row.try_get::<i32, _>("row_number")
                .map_err(map_sqlx_error)?,
        )
        .map_err(|_| StoreError::Unavailable("stored roster row number is invalid".into()))?,
        email,
        roster_id: row
            .try_get::<Option<String>, _>("roster_id")
            .map_err(map_sqlx_error)?
            .map(|value| {
                crate::CourseRosterId::parse(&value)
                    .map_err(|_| StoreError::Unavailable("stored roster ID is invalid".into()))
            })
            .transpose()?,
        status: parse_status(
            &row.try_get::<String, _>("row_status")
                .map_err(map_sqlx_error)?,
        )?,
    })
}

fn parse_status(value: &str) -> Result<RosterImportRowStatus, StoreError> {
    match value {
        "ready_to_invite" => Ok(RosterImportRowStatus::ReadyToInvite),
        "already_member" => Ok(RosterImportRowStatus::AlreadyMember),
        "already_pending" => Ok(RosterImportRowStatus::AlreadyPending),
        "duplicate" => Ok(RosterImportRowStatus::Duplicate),
        "invalid" => Ok(RosterImportRowStatus::Invalid),
        _ => Err(StoreError::Unavailable(
            "stored roster import row status is invalid".into(),
        )),
    }
}

fn import_state(value: &str) -> Result<CourseRosterImportState, StoreError> {
    match value {
        "preview" => Ok(CourseRosterImportState::Preview),
        "committed" => Ok(CourseRosterImportState::Committed),
        _ => Err(StoreError::Unavailable(
            "stored roster import state is invalid".into(),
        )),
    }
}

//! PostgreSQL staged roster-import transaction.

use std::collections::{BTreeMap, BTreeSet};

use question_model::{ActivityTimestamp, CourseId, TenantId};
use sqlx::postgres::PgRow;
use sqlx::{Postgres, Row, Transaction};

use super::{
    CourseInvitationId, CourseRosterImportPreview, PostgresStore, bump_revision, decode_invitation,
    load_policy, lock_course_roster_cross_product, map_sqlx_error, require_manager,
};
use crate::{
    AuthenticationEmail, CommitCourseRosterImport, CommittedCourseRosterImport,
    CourseRosterImportId, CourseRosterImportRow, CourseRosterImportRowInput,
    CourseRosterImportState, RosterImportRevision, RosterImportRowStatus, RosterRevision,
    SessionTokenHash, StageCourseRosterImport, StoreError, TenantContext,
};

const IMPORT_CLEANUP_BATCH: i64 = 128;

pub(super) async fn stage(
    store: &PostgresStore,
    context: TenantContext,
    session: SessionTokenHash,
    command: StageCourseRosterImport,
) -> Result<CourseRosterImportPreview, StoreError> {
    validate_rows(&command.rows)?;
    let tenant = context.tenant_id();
    let mut transaction = store.begin_tenant(context).await?;
    cleanup_expired(&mut transaction).await?;
    require_manager(&mut transaction, session, command.course).await?;
    lock_course_roster_cross_product(&mut transaction, tenant, command.course).await?;
    let actor = require_manager(&mut transaction, session, command.course).await?;
    let policy = load_policy(&mut transaction, tenant, command.course, true).await?;
    if policy.revision != command.expected_roster_revision {
        return Err(StoreError::Conflict);
    }
    if let Some(row) = sqlx::query(
        "SELECT roster_import_id, normalized_digest, roster_revision \
         FROM course_roster_import \
         WHERE tenant_id = $1 AND course_id = $2 AND stage_idempotency_key = $3 \
         FOR UPDATE",
    )
    .bind(tenant.as_uuid())
    .bind(command.course.as_uuid())
    .bind(command.idempotency_key.as_str())
    .fetch_optional(&mut *transaction)
    .await
    .map_err(map_sqlx_error)?
    {
        let digest: Vec<u8> = row.try_get("normalized_digest").map_err(map_sqlx_error)?;
        let roster_revision =
            RosterRevision::from_stored(row.try_get("roster_revision").map_err(map_sqlx_error)?)?;
        if digest.as_slice() != command.normalized_digest.as_bytes()
            || roster_revision != command.expected_roster_revision
        {
            return Err(StoreError::Conflict);
        }
        let import = CourseRosterImportId::from_uuid(
            row.try_get("roster_import_id").map_err(map_sqlx_error)?,
        );
        let preview = load_preview(&mut transaction, tenant, command.course, import).await?;
        transaction.commit().await.map_err(map_sqlx_error)?;
        return Ok(preview);
    }

    let import = CourseRosterImportId::generate()?;
    let classified = classify_rows(
        &mut transaction,
        tenant,
        command.course,
        &policy,
        &command.rows,
    )
    .await?;
    let row = sqlx::query(
        "INSERT INTO course_roster_import \
         (tenant_id, course_id, roster_import_id, normalized_digest, \
          stage_idempotency_key, roster_revision, created_by, expires_at) \
         VALUES ($1, $2, $3, $4, $5, $6, $7, \
                 transaction_timestamp() + ($8::bigint * interval '1 second')) \
         RETURNING floor(extract(epoch FROM expires_at) * 1000)::bigint AS expires_at_millis",
    )
    .bind(tenant.as_uuid())
    .bind(command.course.as_uuid())
    .bind(import.as_uuid())
    .bind(command.normalized_digest.as_bytes().as_slice())
    .bind(command.idempotency_key.as_str())
    .bind(
        i64::try_from(command.expected_roster_revision.value())
            .map_err(|_| StoreError::Conflict)?,
    )
    .bind(actor.as_uuid())
    .bind(i64::from(command.lifetime.as_seconds()))
    .fetch_one(&mut *transaction)
    .await
    .map_err(map_sqlx_error)?;
    for preview_row in &classified {
        sqlx::query(
            "INSERT INTO course_roster_import_row \
             (tenant_id, course_id, roster_import_id, row_number, normalized_email, \
              delivery_email, roster_id, row_status) \
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8)",
        )
        .bind(tenant.as_uuid())
        .bind(command.course.as_uuid())
        .bind(import.as_uuid())
        .bind(i32::from(preview_row.row_number))
        .bind(
            preview_row
                .email
                .as_ref()
                .map(AuthenticationEmail::normalized),
        )
        .bind(
            preview_row
                .email
                .as_ref()
                .map(AuthenticationEmail::delivery),
        )
        .bind(preview_row.roster_id.as_ref().map(|value| value.as_str()))
        .bind(status_name(preview_row.status))
        .execute(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;
    }
    let preview = CourseRosterImportPreview {
        id: import,
        course: command.course,
        roster_revision: command.expected_roster_revision,
        revision: RosterImportRevision::INITIAL,
        state: CourseRosterImportState::Preview,
        expires_at: ActivityTimestamp::from_unix_millis(
            row.try_get("expires_at_millis").map_err(map_sqlx_error)?,
        ),
        rows: classified,
    };
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
    cleanup_expired(&mut transaction).await?;
    require_manager(&mut transaction, session, command.course).await?;
    lock_course_roster_cross_product(&mut transaction, tenant, command.course).await?;
    let actor = require_manager(&mut transaction, session, command.course).await?;
    let current_policy = load_policy(&mut transaction, tenant, command.course, true).await?;
    let row = sqlx::query(
        "SELECT roster_revision, committed_roster_revision, revision, status, \
                commit_idempotency_key, expires_at > transaction_timestamp() AS active \
         FROM course_roster_import \
         WHERE tenant_id = $1 AND course_id = $2 AND roster_import_id = $3 FOR UPDATE",
    )
    .bind(tenant.as_uuid())
    .bind(command.course.as_uuid())
    .bind(command.import.as_uuid())
    .fetch_optional(&mut *transaction)
    .await
    .map_err(map_sqlx_error)?
    .ok_or(StoreError::NotFound)?;
    let status: String = row.try_get("status").map_err(map_sqlx_error)?;
    let stored_commit_key: Option<String> = row
        .try_get("commit_idempotency_key")
        .map_err(map_sqlx_error)?;
    if status == "committed" {
        if stored_commit_key.as_deref() != Some(command.idempotency_key.as_str()) {
            return Err(StoreError::Conflict);
        }
        let result = load_committed(
            &mut transaction,
            tenant,
            command.course,
            command.import,
            &row,
        )
        .await?;
        transaction.commit().await.map_err(map_sqlx_error)?;
        return Ok(result);
    }
    let import_revision =
        RosterImportRevision::from_stored(row.try_get("revision").map_err(map_sqlx_error)?)?;
    let roster_revision =
        RosterRevision::from_stored(row.try_get("roster_revision").map_err(map_sqlx_error)?)?;
    let active: bool = row.try_get("active").map_err(map_sqlx_error)?;
    if status != "preview"
        || !active
        || import_revision != command.expected_import_revision
        || current_policy.revision != roster_revision
    {
        return Err(StoreError::Conflict);
    }
    let ready_rows = sqlx::query(
        "SELECT row_number, normalized_email, delivery_email, roster_id \
         FROM course_roster_import_row \
         WHERE tenant_id = $1 AND course_id = $2 AND roster_import_id = $3 \
           AND row_status = 'ready_to_invite' ORDER BY row_number",
    )
    .bind(tenant.as_uuid())
    .bind(command.course.as_uuid())
    .bind(command.import.as_uuid())
    .fetch_all(&mut *transaction)
    .await
    .map_err(map_sqlx_error)?;
    let bindings = command
        .invitations
        .iter()
        .map(|binding| (binding.row_number, binding))
        .collect::<BTreeMap<_, _>>();
    let ready_numbers = ready_rows
        .iter()
        .map(|row| {
            row.try_get::<i32, _>("row_number")
                .map_err(map_sqlx_error)
                .and_then(|value| {
                    u16::try_from(value).map_err(|_| {
                        StoreError::Unavailable("stored roster row number is invalid".to_string())
                    })
                })
        })
        .collect::<Result<BTreeSet<_>, StoreError>>()?;
    if bindings.len() != command.invitations.len()
        || bindings.keys().copied().collect::<BTreeSet<_>>() != ready_numbers
    {
        return Err(StoreError::InvalidRecord(
            "roster import invitation set does not match ready rows".to_string(),
        ));
    }
    let mut invitations = Vec::with_capacity(ready_rows.len());
    for ready in ready_rows {
        let row_number = u16::try_from(
            ready
                .try_get::<i32, _>("row_number")
                .map_err(map_sqlx_error)?,
        )
        .map_err(|_| StoreError::Unavailable("stored roster row number is invalid".to_string()))?;
        let binding = bindings[&row_number];
        let normalized: String = ready.try_get("normalized_email").map_err(map_sqlx_error)?;
        let delivery: String = ready.try_get("delivery_email").map_err(map_sqlx_error)?;
        let roster_id: String = ready.try_get("roster_id").map_err(map_sqlx_error)?;
        let invitation_id = CourseInvitationId::generate()?;
        let inserted = sqlx::query(
            "INSERT INTO course_invitation \
             (tenant_id, course_id, invitation_id, token_hash, normalized_email, \
              delivery_email, roster_id, invited_by, idempotency_key, expires_at, \
              roster_import_id, roster_import_row_number) \
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, \
                     transaction_timestamp() + ($10::bigint * interval '1 second'), $11, $12) \
             RETURNING invitation_id, normalized_email, delivery_email, roster_id, invited_by, \
                       status, claimed_user_id, \
                       floor(extract(epoch FROM created_at) * 1000)::bigint AS created_at_millis, \
                       floor(extract(epoch FROM expires_at) * 1000)::bigint AS expires_at_millis",
        )
        .bind(tenant.as_uuid())
        .bind(command.course.as_uuid())
        .bind(invitation_id.as_uuid())
        .bind(binding.token_hash.as_bytes().as_slice())
        .bind(&normalized)
        .bind(&delivery)
        .bind(&roster_id)
        .bind(actor.as_uuid())
        .bind(binding.idempotency_key.as_str())
        .bind(i64::from(binding.lifetime.as_seconds()))
        .bind(command.import.as_uuid())
        .bind(i32::from(row_number))
        .fetch_one(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;
        invitations.push((
            row_number,
            decode_invitation(&inserted, tenant, command.course)?,
        ));
    }
    let committed_roster_revision = bump_revision(
        &mut transaction,
        tenant,
        command.course,
        Some(roster_revision),
    )
    .await?;
    let committed_import_revision = import_revision.next()?;
    let updated = sqlx::query(
        "UPDATE course_roster_import \
         SET status = 'committed', revision = $4, commit_idempotency_key = $5, \
             committed_roster_revision = $6, committed_at = transaction_timestamp() \
         WHERE tenant_id = $1 AND course_id = $2 AND roster_import_id = $3 \
           AND status = 'preview'",
    )
    .bind(tenant.as_uuid())
    .bind(command.course.as_uuid())
    .bind(command.import.as_uuid())
    .bind(i64::try_from(committed_import_revision.value()).map_err(|_| StoreError::Conflict)?)
    .bind(command.idempotency_key.as_str())
    .bind(i64::try_from(committed_roster_revision.value()).map_err(|_| StoreError::Conflict)?)
    .execute(&mut *transaction)
    .await
    .map_err(map_sqlx_error)?;
    if updated.rows_affected() != 1 {
        return Err(StoreError::Conflict);
    }
    let result = CommittedCourseRosterImport {
        import: command.import,
        import_revision: committed_import_revision,
        roster_revision: committed_roster_revision,
        invitations,
    };
    transaction.commit().await.map_err(map_sqlx_error)?;
    Ok(result)
}

async fn cleanup_expired(transaction: &mut Transaction<'_, Postgres>) -> Result<(), StoreError> {
    sqlx::query(
        "DELETE FROM course_roster_import WHERE (tenant_id, course_id, roster_import_id) IN ( \
             SELECT tenant_id, course_id, roster_import_id FROM course_roster_import \
             WHERE status = 'preview' AND expires_at <= transaction_timestamp() \
             ORDER BY expires_at LIMIT $1)",
    )
    .bind(IMPORT_CLEANUP_BATCH)
    .execute(&mut **transaction)
    .await
    .map_err(map_sqlx_error)?;
    Ok(())
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

async fn classify_rows(
    transaction: &mut Transaction<'_, Postgres>,
    tenant: TenantId,
    course: CourseId,
    policy: &crate::CourseEnrollmentPolicy,
    inputs: &[CourseRosterImportRowInput],
) -> Result<Vec<CourseRosterImportRow>, StoreError> {
    let members = sqlx::query(
        "SELECT roster_email_normalized, roster_id FROM course_roster_member \
         WHERE tenant_id = $1 AND course_id = $2 AND status = 'active'",
    )
    .bind(tenant.as_uuid())
    .bind(course.as_uuid())
    .fetch_all(&mut **transaction)
    .await
    .map_err(map_sqlx_error)?;
    let invitations = sqlx::query(
        "SELECT normalized_email, roster_id FROM course_invitation \
         WHERE tenant_id = $1 AND course_id = $2 AND status = 'pending' \
           AND expires_at > transaction_timestamp()",
    )
    .bind(tenant.as_uuid())
    .bind(course.as_uuid())
    .fetch_all(&mut **transaction)
    .await
    .map_err(map_sqlx_error)?;
    let mut email_counts = BTreeMap::new();
    let mut roster_counts = BTreeMap::new();
    for row in inputs {
        if let Some(email) = &row.email {
            *email_counts
                .entry(email.normalized().to_string())
                .or_insert(0_u16) += 1;
        }
        if let Some(roster_id) = &row.roster_id {
            *roster_counts
                .entry(roster_id.as_str().to_string())
                .or_insert(0_u16) += 1;
        }
    }
    inputs
        .iter()
        .map(|input| {
            let status = match (&input.email, &input.roster_id) {
                (Some(email), Some(roster_id))
                    if email_counts[email.normalized()] > 1
                        || roster_counts[roster_id.as_str()] > 1 =>
                {
                    RosterImportRowStatus::Duplicate
                }
                (Some(email), Some(_)) if !policy.validates(email) => {
                    RosterImportRowStatus::Invalid
                }
                (Some(email), Some(roster_id)) => classify_existing(
                    &members,
                    &invitations,
                    email.normalized(),
                    roster_id.as_str(),
                )?,
                _ => RosterImportRowStatus::Invalid,
            };
            Ok(CourseRosterImportRow {
                row_number: input.row_number,
                email: input.email.clone(),
                roster_id: input.roster_id.clone(),
                status,
            })
        })
        .collect()
}

fn classify_existing(
    members: &[PgRow],
    invitations: &[PgRow],
    email: &str,
    roster_id: &str,
) -> Result<RosterImportRowStatus, StoreError> {
    let member_matches = matching_rows(members, "roster_email_normalized", email, roster_id)?;
    if !member_matches.is_empty() {
        return Ok(
            if member_matches == vec![(email.to_string(), roster_id.to_string())] {
                RosterImportRowStatus::AlreadyMember
            } else {
                RosterImportRowStatus::Invalid
            },
        );
    }
    let invitation_matches = matching_rows(invitations, "normalized_email", email, roster_id)?;
    if !invitation_matches.is_empty() {
        return Ok(
            if invitation_matches == vec![(email.to_string(), roster_id.to_string())] {
                RosterImportRowStatus::AlreadyPending
            } else {
                RosterImportRowStatus::Invalid
            },
        );
    }
    Ok(RosterImportRowStatus::ReadyToInvite)
}

fn matching_rows(
    rows: &[PgRow],
    email_column: &str,
    email: &str,
    roster_id: &str,
) -> Result<Vec<(String, String)>, StoreError> {
    rows.iter()
        .filter_map(|row| {
            let stored_email = row.try_get::<Option<String>, _>(email_column);
            let stored_roster = row.try_get::<Option<String>, _>("roster_id");
            match (stored_email, stored_roster) {
                (Ok(Some(stored_email)), Ok(Some(stored_roster)))
                    if stored_email == email || stored_roster == roster_id =>
                {
                    Some(Ok((stored_email, stored_roster)))
                }
                (Ok(_), Ok(_)) => None,
                (Err(error), _) | (_, Err(error)) => Some(Err(map_sqlx_error(error))),
            }
        })
        .collect()
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

async fn load_committed(
    transaction: &mut Transaction<'_, Postgres>,
    tenant: TenantId,
    course: CourseId,
    import: CourseRosterImportId,
    header: &PgRow,
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
            header.try_get("revision").map_err(map_sqlx_error)?,
        )?,
        roster_revision: RosterRevision::from_stored(
            header
                .try_get::<Option<i64>, _>("committed_roster_revision")
                .map_err(map_sqlx_error)?
                .ok_or_else(|| {
                    StoreError::Unavailable("committed roster revision is missing".into())
                })?,
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
                        StoreError::Unavailable("stored roster row number is invalid".into())
                    })?,
                    decode_invitation(row, tenant, course)?,
                ))
            })
            .collect::<Result<Vec<_>, StoreError>>()?,
    })
}

fn status_name(status: RosterImportRowStatus) -> &'static str {
    match status {
        RosterImportRowStatus::ReadyToInvite => "ready_to_invite",
        RosterImportRowStatus::AlreadyMember => "already_member",
        RosterImportRowStatus::AlreadyPending => "already_pending",
        RosterImportRowStatus::Duplicate => "duplicate",
        RosterImportRowStatus::Invalid => "invalid",
    }
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

//! PostgreSQL course-roster authority and concealment predicates.

use question_model::{CourseId, TenantId, UserId};
use sqlx::types::Uuid;
use sqlx::{Postgres, Transaction};

use super::super::map_sqlx_error;
use crate::{CourseRosterSupportAction, SessionTokenHash, StoreError};

pub(super) async fn require_course(
    transaction: &mut Transaction<'_, Postgres>,
    tenant: TenantId,
    course: CourseId,
) -> Result<(), StoreError> {
    let exists: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM course \
         WHERE tenant_id = $1 AND course_id = $2 \
         AND public.ple_course_records_accessible(tenant_id, course_id))",
    )
    .bind(tenant.as_uuid())
    .bind(course.as_uuid())
    .fetch_one(&mut **transaction)
    .await
    .map_err(map_sqlx_error)?;
    exists.then_some(()).ok_or(StoreError::NotFound)
}

pub(in crate::postgres) async fn require_course_instructor(
    transaction: &mut Transaction<'_, Postgres>,
    session: SessionTokenHash,
    course: CourseId,
) -> Result<UserId, StoreError> {
    let actor: Option<Uuid> =
        sqlx::query_scalar("SELECT public.ple_course_roster_actor($1, $2, true)")
            .bind(session.to_string())
            .bind(course.as_uuid())
            .fetch_one(&mut **transaction)
            .await
            .map_err(map_sqlx_error)?;
    if let Some(actor) = actor {
        return Ok(UserId::from_uuid(actor));
    }
    concealed_course_authority_error(transaction, session, course).await
}

pub(in crate::postgres) async fn precheck_course_roster_authority(
    transaction: &mut Transaction<'_, Postgres>,
    session: SessionTokenHash,
    course: CourseId,
) -> Result<UserId, StoreError> {
    let actor: Option<Uuid> =
        sqlx::query_scalar("SELECT public.ple_course_roster_support_precheck($1, $2)")
            .bind(session.to_string())
            .bind(course.as_uuid())
            .fetch_one(&mut **transaction)
            .await
            .map_err(map_sqlx_error)?;
    if let Some(actor) = actor {
        return Ok(UserId::from_uuid(actor));
    }
    concealed_course_authority_error(transaction, session, course).await
}

pub(in crate::postgres) async fn require_audited_course_roster_actor(
    transaction: &mut Transaction<'_, Postgres>,
    session: SessionTokenHash,
    course: CourseId,
    action: CourseRosterSupportAction,
) -> Result<UserId, StoreError> {
    let action = match action {
        CourseRosterSupportAction::ListRoster => "listRoster",
        CourseRosterSupportAction::CreateInvitation => "createInvitation",
        CourseRosterSupportAction::ReplaceEnrollmentPolicy => "replaceEnrollmentPolicy",
        CourseRosterSupportAction::RevokeMember => "revokeMember",
        CourseRosterSupportAction::RevokeInvitation => "revokeInvitation",
        CourseRosterSupportAction::StageImport => "stageImport",
        CourseRosterSupportAction::CommitImport => "commitImport",
    };
    let actor: Option<Uuid> =
        sqlx::query_scalar("SELECT public.ple_course_roster_support_actor($1, $2, $3)")
            .bind(session.to_string())
            .bind(course.as_uuid())
            .bind(action)
            .fetch_one(&mut **transaction)
            .await
            .map_err(map_sqlx_error)?;
    if let Some(actor) = actor {
        return Ok(UserId::from_uuid(actor));
    }
    concealed_course_authority_error(transaction, session, course).await
}

async fn concealed_course_authority_error(
    transaction: &mut Transaction<'_, Postgres>,
    session: SessionTokenHash,
    course: CourseId,
) -> Result<UserId, StoreError> {
    let course_visible: Option<Uuid> =
        sqlx::query_scalar("SELECT public.ple_course_roster_actor($1, $2, false)")
            .bind(session.to_string())
            .bind(course.as_uuid())
            .fetch_one(&mut **transaction)
            .await
            .map_err(map_sqlx_error)?;
    if course_visible.is_some() {
        Err(StoreError::Forbidden)
    } else {
        Err(StoreError::NotFound)
    }
}

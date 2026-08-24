//! Live source authorization before private rehearsal hydration.

use question_model::{
    AssignmentId, AssignmentReference, CourseId, CourseMembershipId, TeachingOperationRevision,
    TenantId, UserId,
};
use sqlx::{Postgres, Row, Transaction};

use super::super::*;

pub(super) struct LockedSource {
    pub(super) assignment: AssignmentId,
    pub(super) owner: CourseMembershipId,
}

/// Plain authorization for read-only receipt projection.  Mutations must use
/// the broker prepare witnesses below; Rust never attempts to recreate their
/// lock ordering.
pub(super) async fn lock_source(
    tx: &mut Transaction<'_, Postgres>,
    tenant: TenantId,
    actor: UserId,
    course: CourseId,
    assignment: AssignmentReference,
    revision: TeachingOperationRevision,
) -> Result<LockedSource, StoreError> {
    let assignment_id = sqlx::query_scalar::<_, sqlx::types::Uuid>("SELECT assignment_id FROM assignment WHERE tenant_id=$1 AND course_id=$2 AND public_id=$3 AND revision=$4")
        .bind(tenant.as_uuid()).bind(course.as_uuid()).bind(i64::from(assignment.number())).bind(i64::try_from(revision.value()).map_err(|_| StoreError::InvalidRecord("teaching revision exceeds database range".into()))?)
        .fetch_optional(&mut **tx).await.map_err(map_sqlx_error)?.map(AssignmentId::from_uuid).ok_or(StoreError::NotFound)?;
    let owner = sqlx::query_scalar::<_, sqlx::types::Uuid>("SELECT course_membership_id FROM course_member WHERE tenant_id=$1 AND course_id=$2 AND user_id=$3 AND role='instructor' AND status='active'")
        .bind(tenant.as_uuid()).bind(course.as_uuid()).bind(actor.as_uuid()).fetch_optional(&mut **tx).await.map_err(map_sqlx_error)?.map(CourseMembershipId::from_uuid).ok_or(StoreError::NotFound)?;
    Ok(LockedSource {
        assignment: assignment_id,
        owner,
    })
}

pub(super) struct OperationWitness {
    pub(super) assignment: AssignmentId,
    pub(super) owner: CourseMembershipId,
    pub(super) run: question_model::RehearsalRunId,
}
pub(super) struct StartWitness {
    pub(super) assignment: AssignmentId,
    pub(super) owner: CourseMembershipId,
    /// The broker-locked derived learner membership, if the start uses a
    /// derived subject.  It remains an opaque source witness; no learner
    /// identity or preview content is projected here.
    pub(super) derived_membership: Option<CourseMembershipId>,
}
impl StartWitness {
    pub(super) const fn source(&self) -> LockedSource {
        LockedSource {
            assignment: self.assignment,
            owner: self.owner,
        }
    }
}
pub(super) async fn prepare_start(
    tx: &mut Transaction<'_, Postgres>,
    tenant: TenantId,
    actor: UserId,
    course: CourseId,
    assignment: AssignmentReference,
    revision: TeachingOperationRevision,
    derived_membership: Option<CourseMembershipId>,
) -> Result<StartWitness, StoreError> {
    let row = sqlx::query("SELECT assignment_id, direct_instructor_membership_id, derived_membership_id, latest_rehearsal_run_id, latest_rehearsal_reference, latest_assignment_revision FROM ple_prepare_rehearsal_start($1,$2,$3,$4,$5,$6)")
        .bind(tenant.as_uuid()).bind(actor.as_uuid()).bind(course.as_uuid()).bind(i32::try_from(assignment.number()).map_err(|_| StoreError::InvalidRecord("assignment reference exceeds database range".into()))?).bind(i64::try_from(revision.value()).map_err(|_| StoreError::InvalidRecord("teaching revision exceeds database range".into()))?).bind(derived_membership.map(|membership| membership.as_uuid())).fetch_optional(&mut **tx).await.map_err(map_sqlx_error)?.ok_or(StoreError::NotFound)?;
    let latest_reference: Option<i64> = row
        .try_get("latest_rehearsal_reference")
        .map_err(map_sqlx_error)?;
    let latest_revision: Option<i64> = row
        .try_get("latest_assignment_revision")
        .map_err(map_sqlx_error)?;
    let latest_reference = latest_reference
        .map(|value| {
            question_model::RehearsalReference::new(u64::try_from(value).map_err(|_| {
                StoreError::InvalidRecord("invalid prepared rehearsal reference".into())
            })?)
            .ok_or_else(|| StoreError::InvalidRecord("invalid prepared rehearsal reference".into()))
        })
        .transpose()?;
    let latest_revision = latest_revision
        .map(|value| {
            TeachingOperationRevision::new(u64::try_from(value).map_err(|_| {
                StoreError::InvalidRecord("invalid prepared rehearsal revision".into())
            })?)
            .ok_or_else(|| StoreError::InvalidRecord("invalid prepared rehearsal revision".into()))
        })
        .transpose()?;
    let latest_run: Option<sqlx::types::Uuid> = row
        .try_get("latest_rehearsal_run_id")
        .map_err(map_sqlx_error)?;
    if latest_run.is_some() != latest_reference.is_some()
        || latest_run.is_some() != latest_revision.is_some()
    {
        return Err(StoreError::InvalidRecord(
            "invalid prepared latest rehearsal witness".into(),
        ));
    }
    Ok(StartWitness {
        assignment: AssignmentId::from_uuid(row.try_get("assignment_id").map_err(map_sqlx_error)?),
        owner: CourseMembershipId::from_uuid(
            row.try_get("direct_instructor_membership_id")
                .map_err(map_sqlx_error)?,
        ),
        derived_membership: row
            .try_get::<Option<sqlx::types::Uuid>, _>("derived_membership_id")
            .map_err(map_sqlx_error)?
            .map(CourseMembershipId::from_uuid),
    })
}
impl OperationWitness {
    pub(super) const fn source(&self) -> LockedSource {
        LockedSource {
            assignment: self.assignment,
            owner: self.owner,
        }
    }
}

pub(super) async fn prepare_operation(
    tx: &mut Transaction<'_, Postgres>,
    tenant: TenantId,
    locator: crate::RehearsalLocator,
) -> Result<OperationWitness, StoreError> {
    let row = sqlx::query("SELECT assignment_id, direct_instructor_membership_id, rehearsal_run_id FROM ple_prepare_rehearsal_operation($1,$2,$3,$4,$5,$6)")
        .bind(tenant.as_uuid()).bind(locator.actor.as_uuid()).bind(locator.course.as_uuid())
        .bind(i32::try_from(locator.assignment.number()).map_err(|_| StoreError::InvalidRecord("assignment reference exceeds database range".into()))?)
        .bind(i64::try_from(locator.revision.value()).map_err(|_| StoreError::InvalidRecord("teaching revision exceeds database range".into()))?)
        .bind(i64::from(locator.rehearsal.number())).fetch_optional(&mut **tx).await.map_err(map_sqlx_error)?.ok_or(StoreError::NotFound)?;
    Ok(OperationWitness {
        assignment: AssignmentId::from_uuid(row.try_get("assignment_id").map_err(map_sqlx_error)?),
        owner: CourseMembershipId::from_uuid(
            row.try_get("direct_instructor_membership_id")
                .map_err(map_sqlx_error)?,
        ),
        run: question_model::RehearsalRunId::from_uuid(
            row.try_get("rehearsal_run_id").map_err(map_sqlx_error)?,
        ),
    })
}

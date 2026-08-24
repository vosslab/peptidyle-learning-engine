//! Closed row decoders for rehearsal persistence.

use domain::rehearsal::persistence::{decode_persisted_subject, restore_subject_fingerprint};
use question_model::{
    ActivityTimestamp, AssignmentId, AssignmentReference, CourseId, CourseMembershipId,
    RehearsalEvidenceDigest, RehearsalLifecycle, RehearsalReference, RehearsalRunId,
    RehearsalRunReceipt, TeachingOperationRevision, TenantId, UserId,
};
use sqlx::{Row, postgres::PgRow};

use super::super::*;

/// Private run facts after closed decoding.  Deliberately not `Debug` because
/// the subject remains persistence-private until the answer-free receipt is made.
pub(super) struct HydratedRun {
    pub(super) id: RehearsalRunId,
    pub(super) receipt: RehearsalRunReceipt,
    pub(super) owner: CourseMembershipId,
    pub(super) course: CourseId,
    pub(super) tenant: TenantId,
    pub(super) head: domain::RehearsalEvidenceHead,
    pub(super) subject_fingerprint: domain::RehearsalSubjectFingerprint,
}

/// Non-private locator and ownership facts decoded before protected JSON.
pub(super) struct RunLocator {
    pub(super) id: RehearsalRunId,
    pub(super) reference: RehearsalReference,
    pub(super) assignment: AssignmentReference,
    pub(super) revision: TeachingOperationRevision,
    pub(super) lifecycle: RehearsalLifecycle,
    pub(super) started_at: ActivityTimestamp,
    pub(super) updated_at: ActivityTimestamp,
    pub(super) assignment_id: AssignmentId,
    pub(super) owner: CourseMembershipId,
    pub(super) actor: UserId,
    pub(super) course: CourseId,
    pub(super) tenant: TenantId,
    pub(super) head: domain::RehearsalEvidenceHead,
}

pub(super) fn decode_locator(row: &PgRow) -> Result<RunLocator, StoreError> {
    let lifecycle = decode_lifecycle(row.try_get("lifecycle").map_err(map_sqlx_error)?)?;
    let terminal: Option<i64> = row.try_get("terminal_at_millis").map_err(map_sqlx_error)?;
    if lifecycle.is_active() != terminal.is_none() {
        return invalid("rehearsal terminal timestamp shape is invalid");
    }
    let digest: Vec<u8> = row
        .try_get("evidence_head_digest")
        .map_err(map_sqlx_error)?;
    let length: i64 = row.try_get("evidence_length").map_err(map_sqlx_error)?;
    Ok(RunLocator {
        id: RehearsalRunId::from_uuid(row.try_get("rehearsal_run_id").map_err(map_sqlx_error)?),
        reference: positive_reference(row.try_get("rehearsal_reference").map_err(map_sqlx_error)?)?,
        assignment: positive_assignment(
            row.try_get("assignment_reference")
                .map_err(map_sqlx_error)?,
        )?,
        revision: positive_revision(row.try_get("assignment_revision").map_err(map_sqlx_error)?)?,
        lifecycle,
        started_at: timestamp(row.try_get("started_at_millis").map_err(map_sqlx_error)?)?,
        updated_at: timestamp(row.try_get("updated_at_millis").map_err(map_sqlx_error)?)?,
        assignment_id: AssignmentId::from_uuid(
            row.try_get("assignment_id").map_err(map_sqlx_error)?,
        ),
        owner: CourseMembershipId::from_uuid(
            row.try_get("direct_instructor_membership_id")
                .map_err(map_sqlx_error)?,
        ),
        actor: UserId::from_uuid(row.try_get("actor_id").map_err(map_sqlx_error)?),
        course: CourseId::from_uuid(row.try_get("course_id").map_err(map_sqlx_error)?),
        tenant: TenantId::from_uuid(row.try_get("tenant_id").map_err(map_sqlx_error)?),
        head: domain::RehearsalEvidenceHead::from_persisted(
            RehearsalEvidenceDigest::from_bytes(digest.try_into().map_err(|_| {
                StoreError::InvalidRecord("invalid rehearsal evidence head".into())
            })?),
            u32::try_from(length).map_err(|_| {
                StoreError::InvalidRecord("invalid rehearsal evidence length".into())
            })?,
        ),
    })
}

pub(super) fn decode_authorized_run(
    locator: RunLocator,
    subject_payload: &serde_json::Value,
    fingerprint_bytes: &[u8],
) -> Result<HydratedRun, StoreError> {
    let subject = decode_persisted_subject(subject_payload)
        .map_err(|_| StoreError::InvalidRecord("invalid rehearsal subject payload".into()))?;
    let fingerprint = restore_subject_fingerprint(fingerprint_bytes)
        .map_err(|_| StoreError::InvalidRecord("invalid rehearsal subject fingerprint".into()))?;
    let computed = domain::fingerprint_resolved_preview_subject(
        locator.assignment,
        locator.revision,
        &subject,
    )
    .map_err(|_| StoreError::InvalidRecord("invalid rehearsal subject binding".into()))?;
    if computed != fingerprint {
        return invalid("rehearsal subject fingerprint mismatch");
    }
    Ok(HydratedRun {
        id: locator.id,
        receipt: RehearsalRunReceipt {
            rehearsal: locator.reference,
            assignment: locator.assignment,
            revision: locator.revision,
            lifecycle: locator.lifecycle,
            subject,
            started_at: locator.started_at,
            updated_at: locator.updated_at,
        },
        owner: locator.owner,
        course: locator.course,
        tenant: locator.tenant,
        head: locator.head,
        subject_fingerprint: fingerprint,
    })
}

fn positive_reference(value: i64) -> Result<RehearsalReference, StoreError> {
    RehearsalReference::new(
        u64::try_from(value)
            .map_err(|_| StoreError::InvalidRecord("invalid rehearsal reference".into()))?,
    )
    .ok_or_else(|| StoreError::InvalidRecord("invalid rehearsal reference".into()))
}
fn positive_assignment(value: i32) -> Result<AssignmentReference, StoreError> {
    AssignmentReference::new(u64::from(
        u32::try_from(value)
            .map_err(|_| StoreError::InvalidRecord("invalid assignment reference".into()))?,
    ))
    .ok_or_else(|| StoreError::InvalidRecord("invalid assignment reference".into()))
}
fn positive_revision(value: i64) -> Result<TeachingOperationRevision, StoreError> {
    TeachingOperationRevision::new(
        u64::try_from(value)
            .map_err(|_| StoreError::InvalidRecord("invalid teaching revision".into()))?,
    )
    .ok_or_else(|| StoreError::InvalidRecord("invalid teaching revision".into()))
}
fn timestamp(value: i64) -> Result<ActivityTimestamp, StoreError> {
    Ok(ActivityTimestamp::from_unix_millis(value))
}
fn decode_lifecycle(value: String) -> Result<RehearsalLifecycle, StoreError> {
    match value.as_str() {
        "active" => Ok(RehearsalLifecycle::Active),
        "completed" => Ok(RehearsalLifecycle::Completed),
        "discardedByInstructor" => Ok(RehearsalLifecycle::DiscardedByInstructor),
        "discardedByNewSubject" => Ok(RehearsalLifecycle::DiscardedByNewSubject),
        "discardedStaleRevision" => Ok(RehearsalLifecycle::DiscardedStaleRevision),
        "discardedSourceContextRemoved" => Ok(RehearsalLifecycle::DiscardedSourceContextRemoved),
        _ => Err(StoreError::InvalidRecord(
            "invalid rehearsal lifecycle".into(),
        )),
    }
}
fn invalid<T>(message: &str) -> Result<T, StoreError> {
    Err(StoreError::InvalidRecord(message.into()))
}

//! Checked authority witness for one session-bound Student roster revocation.

use question_model::{CourseId, TenantId, UserId};
use sqlx::types::Uuid;
use sqlx::{Postgres, Row, Transaction};

use super::super::map_sqlx_error;
use crate::{CourseMemberId, RevokeCourseMember, RosterRevision, SessionTokenHash, StoreError};

#[derive(Debug, Clone, Copy)]
struct ExpectedWitness {
    tenant: TenantId,
    actor: UserId,
    course: CourseId,
    member: CourseMemberId,
    revision: RosterRevision,
}

#[derive(Debug, Clone, Copy)]
struct RawWitness {
    tenant: Option<Uuid>,
    actor: Option<Uuid>,
    course: Option<Uuid>,
    member: Option<Uuid>,
    was_revoked: Option<bool>,
    revision: Option<i64>,
}

impl RawWitness {
    fn validate(self, expected: ExpectedWitness) -> Result<RosterRevision, StoreError> {
        let invalid = || {
            StoreError::Unavailable(
                "course roster revocation returned an invalid authority witness".to_string(),
            )
        };
        let tenant = self.tenant.ok_or_else(invalid)?;
        let actor = self.actor.ok_or_else(invalid)?;
        let course = self.course.ok_or_else(invalid)?;
        let member = self.member.ok_or_else(invalid)?;
        let was_revoked = self.was_revoked.ok_or_else(invalid)?;
        let revision = RosterRevision::from_stored(self.revision.ok_or_else(invalid)?)
            .map_err(|_| invalid())?;
        if tenant != expected.tenant.as_uuid()
            || actor != expected.actor.as_uuid()
            || course != expected.course.as_uuid()
            || member != expected.member.as_uuid()
            || revision
                != if was_revoked {
                    expected.revision
                } else {
                    expected.revision.next().map_err(|_| invalid())?
                }
        {
            return Err(invalid());
        }
        Ok(revision)
    }
}

pub(super) async fn revoke_course_student(
    transaction: &mut Transaction<'_, Postgres>,
    tenant: TenantId,
    actor: UserId,
    session: SessionTokenHash,
    command: RevokeCourseMember,
) -> Result<RosterRevision, StoreError> {
    let expected = ExpectedWitness {
        tenant,
        actor,
        course: command.course,
        member: command.member,
        revision: command.expected_revision,
    };
    let row = sqlx::query(
        "SELECT tenant_id,actor_id,course_id,course_membership_id,was_revoked,roster_revision \
           FROM public.ple_revoke_course_student_as_roster_actor_v1($1,$2,$3,$4,$5)",
    )
    .bind(tenant.as_uuid())
    .bind(session.to_string())
    .bind(command.course.as_uuid())
    .bind(command.member.as_uuid())
    .bind(i64::try_from(command.expected_revision.value()).map_err(|_| StoreError::Conflict)?)
    .fetch_optional(&mut **transaction)
    .await
    .map_err(map_sqlx_error)?
    .ok_or(StoreError::NotFound)?;
    RawWitness {
        tenant: row.try_get("tenant_id").map_err(map_sqlx_error)?,
        actor: row.try_get("actor_id").map_err(map_sqlx_error)?,
        course: row.try_get("course_id").map_err(map_sqlx_error)?,
        member: row
            .try_get("course_membership_id")
            .map_err(map_sqlx_error)?,
        was_revoked: row.try_get("was_revoked").map_err(map_sqlx_error)?,
        revision: row.try_get("roster_revision").map_err(map_sqlx_error)?,
    }
    .validate(expected)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn uuid(value: u128) -> Uuid {
        Uuid::from_u128(value)
    }

    fn expected() -> ExpectedWitness {
        ExpectedWitness {
            tenant: TenantId::from_uuid(uuid(1)),
            actor: UserId::from_uuid(uuid(2)),
            course: CourseId::from_uuid(uuid(3)),
            member: CourseMemberId::from_uuid(uuid(4)),
            revision: RosterRevision::from_stored(7).expect("positive revision"),
        }
    }

    fn raw(was_revoked: bool, revision: i64) -> RawWitness {
        RawWitness {
            tenant: Some(uuid(1)),
            actor: Some(uuid(2)),
            course: Some(uuid(3)),
            member: Some(uuid(4)),
            was_revoked: Some(was_revoked),
            revision: Some(revision),
        }
    }

    #[test]
    fn accepts_exact_active_transition_witness() {
        assert_eq!(raw(false, 8).validate(expected()).unwrap().value(), 8);
    }

    #[test]
    fn accepts_exact_replay_witness() {
        assert_eq!(raw(true, 7).validate(expected()).unwrap().value(), 7);
    }

    #[test]
    fn rejects_witness_with_wrong_actor_or_revision_semantics() {
        let mut wrong_actor = raw(false, 8);
        wrong_actor.actor = Some(uuid(9));
        assert!(matches!(
            wrong_actor.validate(expected()),
            Err(StoreError::Unavailable(_))
        ));
        assert!(matches!(
            raw(false, 7).validate(expected()),
            Err(StoreError::Unavailable(_))
        ));
        assert!(matches!(
            raw(true, 8).validate(expected()),
            Err(StoreError::Unavailable(_))
        ));
    }

    #[test]
    fn rejects_missing_or_nonpositive_witness_fields() {
        let mut missing = raw(false, 8);
        missing.member = None;
        assert!(matches!(
            missing.validate(expected()),
            Err(StoreError::Unavailable(_))
        ));
        assert!(matches!(
            raw(false, 0).validate(expected()),
            Err(StoreError::Unavailable(_))
        ));
    }
}

//! Typed boundary for direct-Instructor Student activation.

use question_model::{CourseId, CourseMembershipId, StudentId, TenantId, UserId};
use sqlx::types::Uuid;
use sqlx::{Postgres, Row, Transaction};

use super::super::{course_roster_decode::decode_member, map_sqlx_error};
use crate::{
    ClaimedCourseMembership, CourseMemberStatus, RosterRevision, StoreError, UpsertCourseMember,
};

#[derive(Debug, Clone, Copy)]
struct ExpectedWitness {
    tenant: TenantId,
    actor: UserId,
    course: CourseId,
    target: UserId,
    candidate_membership: CourseMembershipId,
}

#[derive(Debug, Clone, Copy)]
struct RawWitness {
    tenant: Option<Uuid>,
    actor: Option<Uuid>,
    direct_instructor_membership: Option<Uuid>,
    course: Option<Uuid>,
    target: Option<Uuid>,
    student: Option<Uuid>,
    membership: Option<Uuid>,
    created: Option<bool>,
    revision: Option<i64>,
}

#[derive(Debug, Clone, Copy)]
struct ValidatedWitness {
    direct_instructor_membership: CourseMembershipId,
    student: StudentId,
    membership: CourseMembershipId,
    roster_revision: RosterRevision,
}

impl RawWitness {
    fn validate(self, expected: ExpectedWitness) -> Result<ValidatedWitness, StoreError> {
        let invalid = || {
            StoreError::Unavailable(
                "course roster mutation returned an invalid authority witness".to_string(),
            )
        };
        let tenant = self.tenant.ok_or_else(invalid)?;
        let actor = self.actor.ok_or_else(invalid)?;
        let direct_membership = self.direct_instructor_membership.ok_or_else(invalid)?;
        let course = self.course.ok_or_else(invalid)?;
        let target = self.target.ok_or_else(invalid)?;
        let student = self.student.ok_or_else(invalid)?;
        let membership = self.membership.ok_or_else(invalid)?;
        let created = self.created.ok_or_else(invalid)?;
        let revision = self.revision.ok_or_else(invalid)?;
        if tenant != expected.tenant.as_uuid()
            || actor != expected.actor.as_uuid()
            || course != expected.course.as_uuid()
            || target != expected.target.as_uuid()
            || direct_membership.is_nil()
            || student.is_nil()
            || membership.is_nil()
            || (created && membership != expected.candidate_membership.as_uuid())
            || (!created && membership == expected.candidate_membership.as_uuid())
        {
            return Err(invalid());
        }
        Ok(ValidatedWitness {
            direct_instructor_membership: CourseMembershipId::from_uuid(direct_membership),
            student: StudentId::from_uuid(student),
            membership: CourseMembershipId::from_uuid(membership),
            roster_revision: RosterRevision::from_stored(revision)?,
        })
    }
}

pub(super) fn candidate_student_id() -> Result<StudentId, StoreError> {
    random_uuid("student ID").map(StudentId::from_uuid)
}

pub(super) fn candidate_membership_id() -> Result<CourseMembershipId, StoreError> {
    random_uuid("course membership ID").map(CourseMembershipId::from_uuid)
}

pub(super) async fn upsert_course_student(
    transaction: &mut Transaction<'_, Postgres>,
    tenant: TenantId,
    actor: UserId,
    command: &UpsertCourseMember,
    candidate_student: StudentId,
    candidate_membership: CourseMembershipId,
) -> Result<ClaimedCourseMembership, StoreError> {
    let display_name = crate::validated_account_display_name(&command.display_name)
        .map_err(|error| StoreError::InvalidRecord(error.to_string()))?;
    let expected = ExpectedWitness {
        tenant,
        actor,
        course: command.course,
        target: command.user,
        candidate_membership,
    };
    let row = sqlx::query(
        "SELECT tenant_id,actor_id,direct_instructor_membership_id,course_id,target_user_id,\
                student_id,course_membership_id,created,roster_revision \
           FROM public.ple_upsert_course_student_as_instructor_v1(\
                $1,$2,$3,$4,$5,$6,$7,$8,$9,$10)",
    )
    .bind(tenant.as_uuid())
    .bind(actor.as_uuid())
    .bind(command.course.as_uuid())
    .bind(command.user.as_uuid())
    .bind(candidate_student.as_uuid())
    .bind(candidate_membership.as_uuid())
    .bind(&display_name)
    .bind(
        command
            .roster_contact
            .as_ref()
            .map(|contact| contact.email.normalized()),
    )
    .bind(
        command
            .roster_contact
            .as_ref()
            .map(|contact| contact.email.delivery()),
    )
    .bind(
        command
            .roster_contact
            .as_ref()
            .map(|contact| contact.roster_id.as_str()),
    )
    .fetch_one(&mut **transaction)
    .await
    .map_err(map_sqlx_error)?;
    let witness = RawWitness {
        tenant: row.try_get("tenant_id").map_err(map_sqlx_error)?,
        actor: row.try_get("actor_id").map_err(map_sqlx_error)?,
        direct_instructor_membership: row
            .try_get("direct_instructor_membership_id")
            .map_err(map_sqlx_error)?,
        course: row.try_get("course_id").map_err(map_sqlx_error)?,
        target: row.try_get("target_user_id").map_err(map_sqlx_error)?,
        student: row.try_get("student_id").map_err(map_sqlx_error)?,
        membership: row
            .try_get("course_membership_id")
            .map_err(map_sqlx_error)?,
        created: row.try_get("created").map_err(map_sqlx_error)?,
        revision: row.try_get("roster_revision").map_err(map_sqlx_error)?,
    }
    .validate(expected)?;

    let actor_membership: Option<Uuid> = sqlx::query_scalar(
        "SELECT course_membership_id FROM course_member \
          WHERE tenant_id=$1 AND course_id=$2 AND course_membership_id=$3 \
            AND user_id=$4 AND role='instructor' AND status='active'",
    )
    .bind(tenant.as_uuid())
    .bind(command.course.as_uuid())
    .bind(witness.direct_instructor_membership.as_uuid())
    .bind(actor.as_uuid())
    .fetch_optional(&mut **transaction)
    .await
    .map_err(map_sqlx_error)?;
    if actor_membership.is_none() {
        return Err(StoreError::Unavailable(
            "course roster mutation authority witness could not be hydrated".to_string(),
        ));
    }

    let member_row = sqlx::query(
        "SELECT membership.course_membership_id AS record_id,membership.user_id,\
                membership.student_id,profile.display_name,\
                profile.roster_email_normalized AS normalized_email,\
                profile.roster_email_delivery AS delivery_email,membership.roster_id,\
                membership.status,\
                floor(extract(epoch FROM membership.joined_at)*1000)::bigint AS created_at_millis,\
                floor(extract(epoch FROM membership.revoked_at)*1000)::bigint AS revoked_at_millis \
           FROM course_member AS membership \
           JOIN course_roster_profile AS profile \
             ON profile.tenant_id=membership.tenant_id \
            AND profile.course_id=membership.course_id \
            AND profile.course_membership_id=membership.course_membership_id \
          WHERE membership.tenant_id=$1 AND membership.course_id=$2 \
            AND membership.user_id=$3 AND membership.student_id=$4 \
            AND membership.course_membership_id=$5 \
            AND membership.role='student' AND membership.status='active'",
    )
    .bind(tenant.as_uuid())
    .bind(command.course.as_uuid())
    .bind(command.user.as_uuid())
    .bind(witness.student.as_uuid())
    .bind(witness.membership.as_uuid())
    .fetch_optional(&mut **transaction)
    .await
    .map_err(map_sqlx_error)?
    .ok_or_else(|| {
        StoreError::Unavailable(
            "course roster mutation result could not be hydrated exactly".to_string(),
        )
    })?;
    let member = decode_member(&member_row, tenant, command.course)?;
    if member.user != command.user
        || member.student != witness.student
        || member.id.as_uuid() != witness.membership.as_uuid()
        || member.status != CourseMemberStatus::Active
    {
        return Err(StoreError::Unavailable(
            "course roster mutation result disagrees with its witness".to_string(),
        ));
    }
    Ok(ClaimedCourseMembership {
        tenant,
        course: command.course,
        member,
        roster_revision: witness.roster_revision,
    })
}

fn random_uuid(label: &str) -> Result<Uuid, StoreError> {
    crate::random_uuid::random_uuid_v4(|error| {
        StoreError::Unavailable(format!("{label} randomness unavailable: {error}"))
    })
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
            target: UserId::from_uuid(uuid(4)),
            candidate_membership: CourseMembershipId::from_uuid(uuid(6)),
        }
    }

    fn valid_raw() -> RawWitness {
        RawWitness {
            tenant: Some(uuid(1)),
            actor: Some(uuid(2)),
            direct_instructor_membership: Some(uuid(7)),
            course: Some(uuid(3)),
            target: Some(uuid(4)),
            student: Some(uuid(5)),
            membership: Some(uuid(6)),
            created: Some(true),
            revision: Some(2),
        }
    }

    #[test]
    fn witness_accepts_exact_created_shape() {
        let witness = valid_raw().validate(expected()).expect("exact witness");
        assert_eq!(witness.student.as_uuid(), uuid(5));
        assert_eq!(witness.membership.as_uuid(), uuid(6));
        assert_eq!(witness.roster_revision.value(), 2);
    }

    #[test]
    fn witness_rejects_null_and_foreign_bindings() {
        for malformed in [
            RawWitness {
                tenant: None,
                ..valid_raw()
            },
            RawWitness {
                actor: None,
                ..valid_raw()
            },
            RawWitness {
                direct_instructor_membership: None,
                ..valid_raw()
            },
            RawWitness {
                course: None,
                ..valid_raw()
            },
            RawWitness {
                target: None,
                ..valid_raw()
            },
            RawWitness {
                student: None,
                ..valid_raw()
            },
            RawWitness {
                membership: None,
                ..valid_raw()
            },
            RawWitness {
                created: None,
                ..valid_raw()
            },
            RawWitness {
                revision: None,
                ..valid_raw()
            },
            RawWitness {
                tenant: Some(uuid(20)),
                ..valid_raw()
            },
            RawWitness {
                actor: Some(uuid(20)),
                ..valid_raw()
            },
            RawWitness {
                course: Some(uuid(20)),
                ..valid_raw()
            },
            RawWitness {
                target: Some(uuid(20)),
                ..valid_raw()
            },
        ] {
            assert!(malformed.validate(expected()).is_err());
        }
    }

    #[test]
    fn witness_rejects_invalid_revision_and_created_shape() {
        assert!(
            RawWitness {
                revision: Some(0),
                ..valid_raw()
            }
            .validate(expected())
            .is_err()
        );
        assert!(
            RawWitness {
                membership: Some(uuid(99)),
                ..valid_raw()
            }
            .validate(expected())
            .is_err()
        );
        assert!(
            RawWitness {
                created: Some(false),
                ..valid_raw()
            }
            .validate(expected())
            .is_err()
        );
        assert!(
            RawWitness {
                direct_instructor_membership: Some(Uuid::nil()),
                ..valid_raw()
            }
            .validate(expected())
            .is_err()
        );
    }

    #[test]
    fn witness_accepts_existing_and_reactivated_student_shapes() {
        let replay = RawWitness {
            student: Some(uuid(40)),
            membership: Some(uuid(41)),
            created: Some(false),
            ..valid_raw()
        };
        assert!(replay.validate(expected()).is_ok());
        let reactivated = RawWitness {
            student: Some(uuid(40)),
            membership: Some(uuid(6)),
            ..valid_raw()
        };
        assert!(reactivated.validate(expected()).is_ok());
    }
}

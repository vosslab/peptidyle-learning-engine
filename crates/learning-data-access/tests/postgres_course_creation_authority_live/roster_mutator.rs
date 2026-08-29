//! Store-level behavior oracle for the direct-Instructor roster capability.

use learning_data_access::postgres::PostgresStore;
use learning_data_access::{
    AuthenticationEmail, CourseRosterContact, CourseRosterId, CourseRosterStore, StoreError,
    TenantContext, UpsertCourseMember,
};
use question_model::{CourseId, TenantId, UserId};

use super::*;

#[derive(Clone, Copy)]
struct RosterFixture {
    tenant: TenantId,
    course: CourseId,
    other_course: CourseId,
    instructor: UserId,
    other_instructor: UserId,
    outsider: UserId,
    sysadmin_only: UserId,
    pending_invitation: Uuid,
}

fn command(course: CourseId, user: UserId, name: &str) -> UpsertCourseMember {
    UpsertCourseMember {
        course,
        user,
        display_name: name.to_owned(),
        roster_contact: None,
    }
}

async fn seed(pool: &PgPool) -> RosterFixture {
    let fixture = RosterFixture {
        tenant: TenantId::from_uuid(id()),
        course: CourseId::from_uuid(id()),
        other_course: CourseId::from_uuid(id()),
        instructor: UserId::from_uuid(id()),
        other_instructor: UserId::from_uuid(id()),
        outsider: UserId::from_uuid(id()),
        sysadmin_only: UserId::from_uuid(id()),
        pending_invitation: id(),
    };
    let mut transaction = pool.begin().await.expect("roster fixture transaction");
    sqlx::query("SELECT set_config('ple.tenant_id',$1,true)")
        .bind(fixture.tenant.to_string())
        .execute(&mut *transaction)
        .await
        .expect("roster fixture tenant");
    for (course, title, instructor) in [
        (fixture.course, "Roster broker course", fixture.instructor),
        (
            fixture.other_course,
            "Other roster broker course",
            fixture.other_instructor,
        ),
    ] {
        sqlx::query(
            "INSERT INTO course(tenant_id,course_id,title,term_start_date,term_end_date,time_zone) \
             VALUES($1,$2,$3,DATE '2026-08-24',DATE '2026-12-18','America/Chicago')",
        )
        .bind(fixture.tenant.as_uuid())
        .bind(course.as_uuid())
        .bind(title)
        .execute(&mut *transaction)
        .await
        .expect("roster fixture course");
        sqlx::query(
            "INSERT INTO course_roster_state(tenant_id,course_id) VALUES($1,$2) \
             ON CONFLICT ON CONSTRAINT course_roster_state_pkey DO NOTHING",
        )
        .bind(fixture.tenant.as_uuid())
        .bind(course.as_uuid())
        .execute(&mut *transaction)
        .await
        .expect("roster fixture state");
        sqlx::query(
            "INSERT INTO course_member(tenant_id,course_id,course_membership_id,user_id,role,\
                                        student_id,status,joined_at) \
             VALUES($1,$2,$3,$4,'instructor',NULL,'active',transaction_timestamp())",
        )
        .bind(fixture.tenant.as_uuid())
        .bind(course.as_uuid())
        .bind(id())
        .bind(instructor.as_uuid())
        .execute(&mut *transaction)
        .await
        .expect("direct Instructor fixture");
    }
    let mut invitation_token = [0_u8; 32];
    getrandom::fill(&mut invitation_token).expect("invitation token fixture randomness");
    sqlx::query(
        "INSERT INTO ple_account(user_id,normalized_email,delivery_email,display_name,platform_roles) \
         VALUES($1,$2,$2,'Sysadmin without course membership','[\"sysadmin\"]'::jsonb)",
    )
    .bind(fixture.sysadmin_only.as_uuid())
    .bind(format!(
        "sysadmin-only-{}@example.edu",
        fixture.sysadmin_only.as_uuid().simple()
    ))
    .execute(&mut *transaction)
    .await
    .expect("Sysadmin-only account fixture");
    sqlx::query(
        "INSERT INTO course_invitation(tenant_id,course_id,invitation_id,token_hash,\
             normalized_email,delivery_email,roster_id,invited_by,idempotency_key,expires_at) \
         VALUES($1,$2,$3,$4,$5,$5,'pending-1',$6,'pending-roster-oracle',\
                transaction_timestamp()+interval '1 day')",
    )
    .bind(fixture.tenant.as_uuid())
    .bind(fixture.course.as_uuid())
    .bind(fixture.pending_invitation)
    .bind(invitation_token.to_vec())
    .bind("pending-roster-oracle@example.edu")
    .bind(fixture.instructor.as_uuid())
    .execute(&mut *transaction)
    .await
    .expect("pending invitation fixture");
    sqlx::query(
        "INSERT INTO course_invitation_delivery(tenant_id,course_id,invitation_id,delivery_id) \
         VALUES($1,$2,$3,$4)",
    )
    .bind(fixture.tenant.as_uuid())
    .bind(fixture.course.as_uuid())
    .bind(fixture.pending_invitation)
    .bind(id())
    .execute(&mut *transaction)
    .await
    .expect("pending invitation delivery fixture");
    transaction.commit().await.expect("commit roster fixture");
    fixture
}

async fn revision(pool: &PgPool, fixture: RosterFixture) -> i64 {
    sqlx::query_scalar(
        "SELECT revision FROM course_roster_state WHERE tenant_id=$1 AND course_id=$2",
    )
    .bind(fixture.tenant.as_uuid())
    .bind(fixture.course.as_uuid())
    .fetch_one(pool)
    .await
    .expect("roster revision")
}

async fn invitation_snapshot(pool: &PgPool, fixture: RosterFixture) -> (String, String, i64) {
    let invitation: String = sqlx::query_scalar(
        "SELECT row_to_json(i)::text FROM course_invitation i \
         WHERE tenant_id=$1 AND course_id=$2 AND invitation_id=$3",
    )
    .bind(fixture.tenant.as_uuid())
    .bind(fixture.course.as_uuid())
    .bind(fixture.pending_invitation)
    .fetch_one(pool)
    .await
    .expect("pending invitation snapshot");
    let delivery: String = sqlx::query_scalar(
        "SELECT row_to_json(d)::text FROM course_invitation_delivery d \
         WHERE tenant_id=$1 AND course_id=$2 AND invitation_id=$3",
    )
    .bind(fixture.tenant.as_uuid())
    .bind(fixture.course.as_uuid())
    .bind(fixture.pending_invitation)
    .fetch_one(pool)
    .await
    .expect("pending invitation delivery snapshot");
    let audits: i64 =
        sqlx::query_scalar("SELECT count(*) FROM audit_event WHERE tenant_id=$1 AND course_id=$2")
            .bind(fixture.tenant.as_uuid())
            .bind(fixture.course.as_uuid())
            .fetch_one(pool)
            .await
            .expect("ordinary roster audit count");
    (invitation, delivery, audits)
}

async fn effect_counts(pool: &PgPool, fixture: RosterFixture, user: UserId) -> (i64, i64, i64) {
    sqlx::query_as(
        "SELECT (SELECT count(*) FROM tenant_student_identity \
                  WHERE tenant_id=$1 AND user_id=$3),\
                (SELECT count(*) FROM course_member \
                  WHERE tenant_id=$1 AND course_id=$2 AND user_id=$3),\
                (SELECT count(*) FROM course_roster_profile p JOIN course_member m \
                  ON m.tenant_id=p.tenant_id AND m.course_id=p.course_id \
                 AND m.course_membership_id=p.course_membership_id \
                  WHERE m.tenant_id=$1 AND m.course_id=$2 AND m.user_id=$3)",
    )
    .bind(fixture.tenant.as_uuid())
    .bind(fixture.course.as_uuid())
    .bind(user.as_uuid())
    .fetch_one(pool)
    .await
    .expect("roster effect counts")
}

#[tokio::test]
#[ignore = "requires the private acceptance runtime workspace"]
async fn direct_instructor_roster_capability_is_atomic_replay_safe_and_narrow() {
    let pool = pool().await;
    let fixture = seed(&pool).await;
    let store = PostgresStore::new(pool.clone());
    let context = TenantContext::from_authenticated_session(fixture.tenant);
    let invitation_before = invitation_snapshot(&pool, fixture).await;
    assert_eq!(revision(&pool, fixture).await, 1);

    let learner = UserId::from_uuid(id());
    let learner_command = command(fixture.course, learner, "Canonical learner");
    let (first, second) = tokio::join!(
        store.upsert_course_member(context, fixture.instructor, learner_command.clone()),
        store.upsert_course_member(context, fixture.instructor, learner_command.clone()),
    );
    let first = first.expect("direct Instructor activation");
    let second = second.expect("concurrent replay");
    assert_eq!(first, second);
    assert_eq!(first.roster_revision.value(), 2);
    assert_eq!(revision(&pool, fixture).await, 2);
    assert_eq!(effect_counts(&pool, fixture, learner).await, (1, 1, 1));
    let divergent = store
        .upsert_course_member(
            context,
            fixture.instructor,
            command(fixture.course, learner, "Divergent retry"),
        )
        .await
        .expect("divergent replay returns canonical profile");
    assert_eq!(divergent.member.display_name, "Canonical learner");
    assert_eq!(divergent.roster_revision.value(), 2);
    assert_eq!(invitation_snapshot(&pool, fixture).await, invitation_before);

    for actor in [
        fixture.outsider,
        fixture.other_instructor,
        fixture.sysadmin_only,
    ] {
        let target = UserId::from_uuid(id());
        let before = (
            revision(&pool, fixture).await,
            effect_counts(&pool, fixture, target).await,
            invitation_snapshot(&pool, fixture).await,
        );
        let result = store
            .upsert_course_member(
                context,
                actor,
                command(fixture.course, target, "Unauthorized learner"),
            )
            .await;
        assert!(matches!(
            result,
            Err(StoreError::Forbidden | StoreError::NotFound)
        ));
        assert_eq!(
            (
                revision(&pool, fixture).await,
                effect_counts(&pool, fixture, target).await,
                invitation_snapshot(&pool, fixture).await,
            ),
            before
        );
    }
    let foreign_target = UserId::from_uuid(id());
    let foreign = store
        .upsert_course_member(
            TenantContext::from_authenticated_session(TenantId::from_uuid(id())),
            fixture.instructor,
            command(fixture.course, foreign_target, "Foreign learner"),
        )
        .await;
    assert!(matches!(
        foreign,
        Err(StoreError::Forbidden | StoreError::NotFound)
    ));
    assert_eq!(
        effect_counts(&pool, fixture, foreign_target).await,
        (0, 0, 0)
    );

    let instructor_conflict = store
        .upsert_course_member(
            context,
            fixture.instructor,
            command(
                fixture.course,
                fixture.instructor,
                "Instructor cannot become Student",
            ),
        )
        .await;
    assert_eq!(instructor_conflict, Err(StoreError::Conflict));
    assert_eq!(revision(&pool, fixture).await, 2);

    let mut revoke = pool.begin().await.expect("revocation fixture transaction");
    sqlx::query("SELECT set_config('ple.tenant_id',$1,true)")
        .bind(fixture.tenant.to_string())
        .execute(&mut *revoke)
        .await
        .expect("revocation fixture tenant");
    sqlx::query(
        "UPDATE course_member SET status='revoked',revoked_at=transaction_timestamp() \
         WHERE tenant_id=$1 AND course_id=$2 AND user_id=$3 AND status='active'",
    )
    .bind(fixture.tenant.as_uuid())
    .bind(fixture.course.as_uuid())
    .bind(learner.as_uuid())
    .execute(&mut *revoke)
    .await
    .expect("revoke first immutable Student episode");
    revoke
        .commit()
        .await
        .expect("commit Student revocation fixture");
    let reactivated = store
        .upsert_course_member(
            context,
            fixture.instructor,
            command(fixture.course, learner, "Reactivated learner"),
        )
        .await
        .expect("reactivate Student with the same learner identity");
    assert_eq!(reactivated.member.student, first.member.student);
    assert_ne!(reactivated.member.id, first.member.id);
    assert_eq!(reactivated.roster_revision.value(), 3);
    assert_eq!(effect_counts(&pool, fixture, learner).await, (1, 2, 2));

    let contact_owner = UserId::from_uuid(id());
    let contact = CourseRosterContact {
        email: AuthenticationEmail::parse("contact-owner@example.edu")
            .expect("contact fixture email"),
        roster_id: CourseRosterId::parse("contact-1").expect("contact fixture roster ID"),
    };
    store
        .upsert_course_member(
            context,
            fixture.instructor,
            UpsertCourseMember {
                roster_contact: Some(contact.clone()),
                ..command(fixture.course, contact_owner, "Contact owner")
            },
        )
        .await
        .expect("contact-bearing Student");
    let conflict_target = UserId::from_uuid(id());
    let before_conflict = (
        revision(&pool, fixture).await,
        effect_counts(&pool, fixture, conflict_target).await,
        invitation_snapshot(&pool, fixture).await,
    );
    let conflict = store
        .upsert_course_member(
            context,
            fixture.instructor,
            UpsertCourseMember {
                roster_contact: Some(contact),
                ..command(fixture.course, conflict_target, "Contact conflict")
            },
        )
        .await;
    assert_eq!(conflict, Err(StoreError::AlreadyExists));
    assert_eq!(
        (
            revision(&pool, fixture).await,
            effect_counts(&pool, fixture, conflict_target).await,
            invitation_snapshot(&pool, fixture).await,
        ),
        before_conflict
    );

    explicit_rollback_and_direct_dml_denial(&pool, fixture).await;
}

async fn explicit_rollback_and_direct_dml_denial(pool: &PgPool, fixture: RosterFixture) {
    let rollback_target = id();
    let mut transaction = app(pool, fixture.tenant.as_uuid()).await;
    sqlx::query("SELECT * FROM public.ple_upsert_course_student_as_instructor_v1($1,$2,$3,$4,$5,$6,$7,NULL,NULL,NULL)")
        .bind(fixture.tenant.as_uuid()).bind(fixture.instructor.as_uuid())
        .bind(fixture.course.as_uuid()).bind(rollback_target).bind(id()).bind(id())
        .bind("Rolled back learner").fetch_one(&mut *transaction).await
        .expect("capability result remains inside caller transaction");
    transaction
        .rollback()
        .await
        .expect("explicit capability rollback");
    assert_eq!(
        effect_counts(pool, fixture, UserId::from_uuid(rollback_target)).await,
        (0, 0, 0)
    );

    for statement in [
        "SELECT 1 FROM course WHERE tenant_id=$1 AND course_id=$2 FOR UPDATE",
        "UPDATE course_roster_state SET revision=revision+1 WHERE tenant_id=$1 AND course_id=$2",
    ] {
        let mut transaction = app(pool, fixture.tenant.as_uuid()).await;
        let denied = sqlx::query(statement)
            .bind(fixture.tenant.as_uuid())
            .bind(fixture.course.as_uuid())
            .execute(&mut *transaction)
            .await;
        assert!(denied.is_err(), "ple_app direct SQL must remain denied");
        transaction
            .rollback()
            .await
            .expect("rollback denied direct SQL");
    }
    let mut transaction = app(pool, fixture.tenant.as_uuid()).await;
    let denied_insert = sqlx::query(
        "INSERT INTO course_member(tenant_id,course_id,course_membership_id,user_id,role,\
                                    student_id,status,joined_at) \
         VALUES($1,$2,$3,$4,'student',$5,'active',transaction_timestamp())",
    )
    .bind(fixture.tenant.as_uuid())
    .bind(fixture.course.as_uuid())
    .bind(id())
    .bind(id())
    .bind(id())
    .execute(&mut *transaction)
    .await;
    assert!(
        denied_insert.is_err(),
        "ple_app direct membership INSERT must remain denied"
    );
    transaction
        .rollback()
        .await
        .expect("rollback denied membership INSERT");
}

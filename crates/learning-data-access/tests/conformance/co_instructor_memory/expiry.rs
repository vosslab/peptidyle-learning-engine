//! Memory-only co-instructor invitation expiry oracle.

use super::*;

/// PostgreSQL proves the same lifecycle with live SQL time.
pub(crate) async fn exercise_memory_co_instructor_expiry(store: &MemoryStore) {
    let tenant = TenantId::from_uuid(uuid(730_800));
    let context = TenantContext::from_authenticated_session(tenant);
    let admin = UserId::from_uuid(uuid(730_801));
    let instructor = UserId::from_uuid(uuid(730_802));
    let target = UserId::from_uuid(uuid(730_803));
    let course = CourseId::from_uuid(uuid(730_804));
    let session = SessionTokenHash::compute(b"t2-expiry-admin");
    let instructor_session = SessionTokenHash::compute(b"t2-expiry-instructor");
    let target_session = SessionTokenHash::compute(b"t2-expiry-target");
    let expired_session = SessionTokenHash::compute(b"t2-expired-target");
    let start = ActivityTimestamp::from_unix_millis(50_000);
    store
        .set_authoritative_time(start)
        .expect("expiry start clock");
    store
        .create_session(
            session,
            SessionSubject::new(tenant, admin, "Expiry admin", vec![UserRole::Sysadmin])
                .expect("admin subject"),
            SessionLifetime::from_seconds(3_600).expect("lifetime"),
        )
        .await
        .expect("admin session");
    super::create_account(store, target, 804).await;
    store
        .create_session(
            target_session,
            SessionSubject::new(tenant, target, "Expiry target", vec![UserRole::Instructor])
                .expect("target subject"),
            SessionLifetime::from_seconds(2_592_001).expect("long target lifetime"),
        )
        .await
        .expect("target session");
    store
        .create_session(
            instructor_session,
            SessionSubject::new(
                tenant,
                instructor,
                "Expiry instructor",
                vec![UserRole::Instructor],
            )
            .expect("instructor subject"),
            SessionLifetime::from_seconds(3_600).expect("lifetime"),
        )
        .await
        .expect("instructor session");
    store
        .create_session(
            expired_session,
            SessionSubject::new(tenant, target, "Expired target", vec![UserRole::Instructor])
                .expect("expired target subject"),
            SessionLifetime::from_seconds(3_600).expect("short target lifetime"),
        )
        .await
        .expect("expired target session");
    let course_creation_authority =
        sysadmin_course_creation_authority(store, tenant, course, instructor).await;
    store
        .create_course(
            context,
            CreateCourseCommand {
                course: CourseRecord {
                    id: course,
                    tenant,
                    title: "Expiry course".to_string(),
                    term: question_model::CourseTerm::from_parts(
                        "2026-08-24",
                        "2026-12-18",
                        "America/Chicago",
                    )
                    .expect("term"),
                },
                authority: course_creation_authority,
            },
        )
        .await
        .expect("course");
    store
        .approve_instructor_account(
            context,
            ApproveInstructorAccount {
                session,
                target,
                expected_revision: None,
            },
        )
        .await
        .expect("approval");
    let invitation = store
        .create_co_instructor_invitation(
            context,
            CreateCoInstructorInvitation {
                session: instructor_session,
                actor: instructor,
                course,
                target,
            },
        )
        .await
        .expect("invitation");
    store
        .set_authoritative_time(invitation.invitation.expires_at)
        .expect("expiry clock");
    let page = PageRequest::first(PageSize::new(10).expect("page"));
    assert!(
        store
            .list_pending_co_instructor_invitations(context, target_session, page.clone())
            .await
            .expect("expired pending list")
            .items
            .is_empty()
    );
    assert!(matches!(
        store
            .accept_co_instructor_invitation(
                context,
                learning_data_access::RespondToCoInstructorInvitation {
                    session: target_session,
                    actor: target,
                    invitation: invitation.invitation.id,
                    expected_revision: invitation.revision,
                },
            )
            .await,
        Err(StoreError::InvalidRecord(_))
    ));
    assert!(matches!(
        store
            .list_pending_co_instructor_invitations(context, expired_session, page)
            .await,
        Err(StoreError::NotFound)
    ));
    assert!(
        store
            .get_current_course_membership(context, course, target)
            .await
            .expect("expired membership")
            .is_none()
    );
}

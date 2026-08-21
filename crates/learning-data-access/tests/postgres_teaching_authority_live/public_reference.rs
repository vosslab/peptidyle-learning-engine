//! Public-reference projection oracle for the live teaching-authority suite.

use super::*;
use question_model::teaching_operations::{
    CoInstructorTargetSearchQuery, CoInstructorTargetSearchRequest, TeachingPageSize,
};

#[tokio::test]
#[ignore = "requires the disposable PostgreSQL 17 database baseline"]
async fn postgres_co_instructor_target_search_oracle() {
    let url = std::env::var("PLE_TEST_DATABASE_URL")
        .expect("PLE_TEST_DATABASE_URL must name the disposable acceptance database");
    let pool = lazy_pool(&url).expect("disposable PostgreSQL URL");
    verify_application_schema(&pool)
        .await
        .expect("migrated schema");
    let store = PostgresStore::with_question_id_secret(pool.clone(), [0x59; 32]);
    let tenant = TenantId::from_uuid(id());
    let context = TenantContext::from_authenticated_session(tenant);
    let sysadmin = UserId::from_uuid(id());
    let instructor = UserId::from_uuid(id());
    let target = UserId::from_uuid(id());
    let target_marker = target.as_uuid().simple().to_string();
    let target_display = format!("Candidate Elm {}", &target_marker[..12]);
    create_account(&store, sysadmin).await;
    create_account_named(&store, target, &target_display).await;
    let sysadmin_session = session(&store, tenant, sysadmin, vec![UserRole::Sysadmin]).await;
    let course = CourseId::from_uuid(id());
    store
        .create_course(
            context,
            CreateCourseCommand {
                course: CourseRecord {
                    id: course,
                    tenant,
                    title: "T2 target-search course".into(),
                    term: CourseTerm::from_parts("2026-08-24", "2026-12-18", "America/Chicago")
                        .expect("fixture term"),
                },
                initial_instructor: instructor,
            },
        )
        .await
        .expect("course and direct instructor persist");
    approval(&store, context, sysadmin_session, target).await;
    assert_eq!(
        target_search_count_for_app(&pool, tenant, instructor, course, &target_marker[..12]).await,
        1,
        "ple_app can execute the bounded display-only target-search broker"
    );
    let request = || CoInstructorTargetSearchRequest {
        query: CoInstructorTargetSearchQuery::try_from(target_marker[..12].to_owned())
            .expect("two-character bounded query"),
        after: None,
        size: TeachingPageSize::try_from(10).expect("bounded target page"),
    };
    let first = store
        .search_course_co_instructor_targets(context, instructor, course, request())
        .await
        .expect("direct instructor searches approved candidates");
    assert_eq!(first.targets.len(), 1, "active approval is discoverable");
    assert_eq!(
        String::from(first.targets[0].account.display.clone()),
        target_display,
        "search returns a display projection, not an account identity"
    );
    let invitation = store
        .create_co_instructor_invitation(
            context,
            CreateCoInstructorInvitation {
                actor: instructor,
                course,
                target,
            },
        )
        .await
        .expect("pending invitation suppresses a repeated target");
    assert!(
        store
            .search_course_co_instructor_targets(context, instructor, course, request())
            .await
            .expect("pending target search")
            .targets
            .is_empty(),
        "pending invitation targets never reappear"
    );
    store
        .accept_co_instructor_invitation(
            context,
            RespondToCoInstructorInvitation {
                actor: target,
                invitation: invitation.invitation.id,
                expected_revision: invitation.revision,
            },
        )
        .await
        .expect("target accepts invitation into a direct instructor membership");
    assert!(
        store
            .search_course_co_instructor_targets(context, instructor, course, request())
            .await
            .expect("direct instructor exclusion search")
            .targets
            .is_empty(),
        "active direct instructors never reappear as invitation targets"
    );
}

#[tokio::test]
#[ignore = "requires the disposable PostgreSQL 17 database baseline"]
async fn postgres_teaching_authority_is_target_bound_atomic_and_least_privilege() {
    let url = std::env::var("PLE_TEST_DATABASE_URL")
        .expect("PLE_TEST_DATABASE_URL must name the disposable acceptance database");
    let pool = lazy_pool(&url).expect("disposable PostgreSQL URL");
    verify_application_schema(&pool)
        .await
        .expect("migrated schema");
    let store = PostgresStore::with_question_id_secret(pool.clone(), [0x54; 32]);
    let tenant = TenantId::from_uuid(id());
    let foreign_tenant = TenantId::from_uuid(id());
    let context = TenantContext::from_authenticated_session(tenant);
    let sysadmin = UserId::from_uuid(id());
    let instructor = UserId::from_uuid(id());
    let student = UserId::from_uuid(id());
    let target = UserId::from_uuid(id());
    let other = UserId::from_uuid(id());
    create_account(&store, sysadmin).await;
    create_account(&store, instructor).await;
    create_account(&store, target).await;
    create_account(&store, student).await;
    create_account(&store, other).await;
    let sysadmin_session = session(&store, tenant, sysadmin, vec![UserRole::Sysadmin]).await;
    let instructor_session = session(&store, tenant, instructor, vec![UserRole::Instructor]).await;
    let target_session = session(&store, tenant, target, vec![UserRole::Instructor]).await;
    let other_session = session(&store, tenant, other, vec![UserRole::Instructor]).await;
    let foreign_sysadmin =
        session(&store, foreign_tenant, sysadmin, vec![UserRole::Sysadmin]).await;
    let course = CourseId::from_uuid(id());
    store
        .create_course(
            context,
            CreateCourseCommand {
                course: CourseRecord {
                    id: course,
                    tenant,
                    title: "T2 authority live course".into(),
                    term: CourseTerm::from_parts("2026-08-24", "2026-12-18", "America/Chicago")
                        .expect("fixture term"),
                },
                initial_instructor: instructor,
            },
        )
        .await
        .expect("course and direct instructor persist");
    store
        .upsert_course_member(
            context,
            UpsertCourseMember {
                course,
                user: student,
                display_name: "T2 live student target".into(),
                roster_contact: None,
            },
        )
        .await
        .expect("student membership persists");
    let student_membership = store
        .get_current_course_membership(context, course, student)
        .await
        .expect("student membership lookup")
        .expect("active student membership");
    let student_reference = store
        .course_membership_reference(context, instructor, course, student_membership.id)
        .await
        .expect("student membership reference")
        .expect("student membership has reference");
    let target_view = store
        .resolve_active_student_target_reference(context, instructor, course, student_reference)
        .await
        .expect("exact-course Instructor resolves active Student target")
        .expect("active Student target");
    assert_eq!(target_view.membership, student_membership.id);
    assert_eq!(target_view.user, student);
    assert_eq!(
        target_view.student,
        student_membership.student.expect("student identity")
    );
    assert_eq!(
        store
            .active_student_membership_reference_view(
                context,
                instructor,
                course,
                target_view.student,
            )
            .await
            .expect("exact-course Instructor reverses active Student identity")
            .expect("active Student reference view")
            .reference,
        student_reference,
    );
    assert_eq!(
        store
            .resolve_active_student_target_reference(context, instructor, course, student_reference)
            .await
            .expect("repeated target resolution"),
        Some(target_view),
    );

    let approval = store
        .approve_instructor_account(
            context,
            ApproveInstructorAccount {
                session: sysadmin_session,
                target,
                expected_revision: None,
            },
        )
        .await
        .expect("active tenant sysadmin approves existing target");
    assert_eq!(approval.revision, InstructorApprovalRevision::INITIAL);
    assert!(
        approved_for_invitation(&pool, target).await,
        "a committed global approval is immediately visible through ple_app's brokered \
         eligibility call"
    );
    assert!(
        approval_target_exists_for_app(&pool, target).await,
        "ple_app resolves an existing invitation target only through the broker"
    );
    assert!(
        !approval_target_exists_for_app(&pool, UserId::from_uuid(id())).await,
        "the broker preserves the Store's missing-target distinction"
    );
    insert_approved_invitation_for_app(&pool, tenant, course, target, instructor).await;
    assert!(
        store
            .approve_instructor_account(
                context,
                ApproveInstructorAccount {
                    session: instructor_session,
                    target: other,
                    expected_revision: None,
                }
            )
            .await
            .is_err(),
        "non-sysadmin cannot approve"
    );
    assert!(
        store
            .approve_instructor_account(
                context,
                ApproveInstructorAccount {
                    session: foreign_sysadmin,
                    target: other,
                    expected_revision: None,
                }
            )
            .await
            .is_err(),
        "a session cannot operate under a mismatched tenant context"
    );
    assert_eq!(
        store
            .approve_instructor_account(
                context,
                ApproveInstructorAccount {
                    session: sysadmin_session,
                    target: UserId::from_uuid(id()),
                    expected_revision: None,
                }
            )
            .await,
        Err(StoreError::NotFound),
        "missing targets do not become approvals"
    );
    assert_eq!(
        store
            .approve_instructor_account(
                context,
                ApproveInstructorAccount {
                    session: sysadmin_session,
                    target,
                    expected_revision: None,
                }
            )
            .await,
        Err(StoreError::Conflict),
        "create cannot be replayed without its revision"
    );

    let reapproved = store
        .approve_instructor_account(
            context,
            ApproveInstructorAccount {
                session: sysadmin_session,
                target,
                expected_revision: Some(approval.revision),
            },
        )
        .await
        .expect("revisioned reapproval");
    let revoked = store
        .revoke_instructor_approval(
            context,
            RevokeInstructorApproval {
                session: sysadmin_session,
                target,
                expected_revision: reapproved.revision,
            },
        )
        .await
        .expect("revisioned revoke");
    assert!(revoked.approval.revoked_at.is_some());
    assert_eq!(
        store
            .revoke_instructor_approval(
                context,
                RevokeInstructorApproval {
                    session: sysadmin_session,
                    target,
                    expected_revision: reapproved.revision,
                }
            )
            .await,
        Err(StoreError::Conflict),
        "stale approval revision conflicts"
    );
    let approved = store
        .approve_instructor_account(
            context,
            ApproveInstructorAccount {
                session: sysadmin_session,
                target,
                expected_revision: Some(revoked.revision),
            },
        )
        .await
        .expect("reapproval restores only invitation eligibility");

    assert_eq!(
        store
            .list_courses(
                context,
                CourseListScope::Member(target),
                PageRequest::first(PageSize::new(10).expect("page size"))
            )
            .await
            .expect("course list")
            .items
            .len(),
        0,
        "approval alone grants neither course nor platform authority"
    );
    let invitation = store
        .create_co_instructor_invitation(
            context,
            CreateCoInstructorInvitation {
                actor: instructor,
                course,
                target,
            },
        )
        .await
        .expect("direct instructor invites approved target without email");
    assert_eq!(invitation.revision, CoInstructorInvitationRevision::INITIAL);
    let target_reference = store
        .own_account_reference(context, target_session)
        .await
        .expect("active same-tenant target receives only its own public account projection");
    assert_eq!(
        target_reference.display_name,
        "T2 live co-instructor candidate"
    );
    assert_eq!(
        store
            .resolve_account_reference_for_operator(
                context,
                sysadmin_session,
                target_reference.reference
            )
            .await
            .expect("persisted same-tenant sysadmin resolves account locator"),
        Some(target),
    );
    assert!(
        store
            .resolve_account_reference_for_operator(
                context,
                instructor_session,
                target_reference.reference
            )
            .await
            .is_err(),
        "non-sysadmin sessions cannot resolve account locators"
    );
    assert_eq!(
        store
            .resolve_approved_account_reference_for_course(
                context,
                instructor,
                course,
                target_reference.reference,
            )
            .await
            .expect("exact-course direct instructor resolves current approved target"),
        Some(target),
    );
    let invitation_reference = store
        .co_instructor_invitation_reference(context, instructor, course, invitation.invitation.id)
        .await
        .expect("exact-course direct instructor mints invitation locator")
        .expect("matching invitation locator");
    let invitation_views = store
        .list_course_co_instructor_invitation_reference_views(
            context,
            instructor,
            course,
            PageRequest::first(PageSize::new(10).expect("page size")),
        )
        .await
        .expect("direct Instructor invitation projection");
    assert_eq!(invitation_views.items.len(), 1);
    assert_eq!(invitation_views.items[0].reference, invitation_reference);
    assert_eq!(invitation_views.items[0].target, target_reference.reference);
    assert_eq!(
        invitation_views.items[0].target_display_name,
        target_reference.display_name
    );
    let pending_reference_views = store
        .list_pending_co_instructor_invitation_reference_views(
            context,
            target_session,
            PageRequest::first(PageSize::new(10).expect("page size")),
        )
        .await
        .expect("target pending invitation projection");
    assert_eq!(pending_reference_views.items.len(), 1);
    assert_eq!(
        pending_reference_views.items[0].reference,
        invitation_reference
    );
    assert_eq!(
        pending_reference_views.items[0].course_title,
        "T2 authority live course"
    );
    assert_eq!(
        store
            .resolve_pending_co_instructor_invitation_reference(
                context,
                target_session,
                invitation_reference,
            )
            .await
            .expect("active target session resolves its pending invitation"),
        Some(invitation.invitation.id),
    );
    assert_eq!(
        store
            .resolve_pending_co_instructor_invitation_reference(
                context,
                other_session,
                invitation_reference,
            )
            .await
            .expect("other active session receives no target invitation"),
        None,
    );
    let replay = store
        .create_co_instructor_invitation(
            context,
            CreateCoInstructorInvitation {
                actor: instructor,
                course,
                target,
            },
        )
        .await
        .expect("pending invite is idempotent");
    assert_eq!(replay.invitation.id, invitation.invitation.id);
    assert_eq!(
        target_session_subject_for_app(&pool, tenant, target_session).await,
        Some(target.as_uuid()),
        "the target-session broker exposes only the active target subject"
    );
    assert_eq!(
        target_session_subject_for_app(&pool, tenant, other_session).await,
        Some(other.as_uuid()),
        "each presented session resolves only its own subject"
    );
    let pending = store
        .list_pending_co_instructor_invitations(
            context,
            target_session,
            PageRequest::first(PageSize::new(10).expect("page size")),
        )
        .await
        .expect("target pending list");
    assert_eq!(
        pending.items.len(),
        1,
        "target sees exactly its pending invitation"
    );
    assert!(
        store
            .list_pending_co_instructor_invitations(
                context,
                other_session,
                PageRequest::first(PageSize::new(10).expect("page size"))
            )
            .await
            .expect("other pending list")
            .items
            .is_empty(),
        "other accounts cannot enumerate target invitations"
    );
    assert_eq!(
        store
            .accept_co_instructor_invitation(
                context,
                RespondToCoInstructorInvitation {
                    actor: other,
                    invitation: invitation.invitation.id,
                    expected_revision: invitation.revision,
                }
            )
            .await,
        Err(StoreError::NotFound),
        "wrong targets receive no invitation existence signal"
    );

    let _initial = store
        .get_current_course_membership(context, course, instructor)
        .await
        .expect("initial instructor lookup")
        .expect("initial instructor");
    let roster_revision_before_acceptance = store
        .list_course_instructor_membership_reference_views(
            context,
            instructor,
            course,
            PageRequest::first(PageSize::new(10).expect("page size")),
        )
        .await
        .expect("direct Instructor roster before acceptance")
        .roster_revision;
    let accepted = store
        .accept_co_instructor_invitation(
            context,
            RespondToCoInstructorInvitation {
                actor: target,
                invitation: invitation.invitation.id,
                expected_revision: invitation.revision,
            },
        )
        .await
        .expect("target accepts its current approved invitation");
    assert_eq!(accepted.user, target);
    let target_membership = store
        .get_current_course_membership(context, course, target)
        .await
        .expect("accepted membership lookup")
        .expect("direct instructor membership");
    let target_membership_reference = store
        .course_membership_reference(context, instructor, course, target_membership.id)
        .await
        .expect("exact-course instructor mints membership locator")
        .expect("matching membership locator");
    let beyond_end_instructor_page = store
        .list_course_instructor_membership_reference_views(
            context,
            instructor,
            course,
            PageRequest::after(
                Cursor::parse(format!("{:010}", target_membership_reference.number()))
                    .expect("stable direct Instructor cursor"),
                PageSize::new(10).expect("page size"),
            ),
        )
        .await
        .expect("beyond-end direct Instructor page");
    assert!(
        beyond_end_instructor_page.page.items.is_empty()
            && beyond_end_instructor_page.page.next_cursor.is_none(),
        "the PostgreSQL broker returns an ordinary empty page beyond the final direct Instructor"
    );
    assert_eq!(
        beyond_end_instructor_page.roster_revision, accepted.roster_revision,
        "the empty PostgreSQL page preserves the shared removal token"
    );
    let membership_views = store
        .list_course_membership_reference_views(
            context,
            instructor,
            course,
            PageRequest::first(PageSize::new(10).expect("page size")),
        )
        .await
        .expect("direct Instructor membership projection");
    assert!(
        membership_views
            .items
            .iter()
            .any(|view| view.reference == target_membership_reference
                && view.display_name == target_reference.display_name)
    );
    assert_eq!(
        store
            .resolve_course_membership_reference(
                context,
                instructor,
                course,
                target_membership_reference,
            )
            .await
            .expect("exact-course instructor resolves matching membership locator"),
        Some(target_membership.id),
    );
    assert!(
        target_membership.student.is_none(),
        "direct instructors never receive student identity"
    );
    assert_eq!(
        accepted.roster_revision.value(),
        roster_revision_before_acceptance.value() + 1,
        "acceptance advances the current roster revision once"
    );
    let accepted_replay = store
        .accept_co_instructor_invitation(
            context,
            RespondToCoInstructorInvitation {
                actor: target,
                invitation: invitation.invitation.id,
                expected_revision: invitation.revision,
            },
        )
        .await
        .expect("accepted invitation replay is idempotent");
    assert_eq!(accepted_replay.roster_revision, accepted.roster_revision);

    let next = store
        .create_co_instructor_invitation(
            context,
            CreateCoInstructorInvitation {
                actor: instructor,
                course,
                target: other,
            },
        )
        .await;
    assert!(
        next.is_err(),
        "unapproved accounts cannot receive invitations"
    );
    let removal = RemoveDirectInstructorMembership {
        actor: instructor,
        course,
        membership: accepted.membership,
        expected_roster_revision: accepted.roster_revision,
    };
    store
        .remove_direct_instructor_membership(context, removal)
        .await
        .expect("one of two instructors may leave");
    assert_eq!(
        store
            .remove_direct_instructor_membership(context, removal)
            .await,
        Err(StoreError::NotFound),
        "removed membership no longer exists as active direct authority"
    );

    assert_eq!(
        invitation_count_for_app(&pool, None).await,
        0,
        "ple_app without tenant context sees no invitations"
    );
    assert_eq!(
        invitation_count_for_app(&pool, Some(foreign_tenant)).await,
        0,
        "foreign tenant RLS sees no invitations"
    );
    approval_function_guards(&pool).await;
    let invitation_rows: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM course_instructor_invitation WHERE tenant_id=$1 AND course_id=$2",
    )
    .bind(tenant.as_uuid())
    .bind(course.as_uuid())
    .fetch_one(&pool)
    .await
    .expect("invitation count");
    assert_eq!(
        invitation_rows, 1,
        "accepted replay created no duplicate invitation"
    );
    assert!(approved.approval.revoked_at.is_none());
}

//! Public conformance for T2 co-instructor authority; PostgreSQL reuses this helper later.

use super::*;
use learning_data_access::{
    AccountIdentityStore, ApproveInstructorAccount, CoInstructorInvitationRevision,
    CreateCoInstructorInvitation, EmailAuthenticationPurpose, EmailChallengeLifetime,
    InstructorApprovalRevision, RevokeInstructorApproval, TeachingAuthorityReferenceStore,
    TeachingAuthorityStore,
};

#[path = "co_instructor_memory/expiry.rs"]
mod expiry;

pub(crate) use expiry::exercise_memory_co_instructor_expiry;

pub(crate) async fn exercise_co_instructor_authority_contract<S>(store: &S)
where
    S: Store
        + AccountIdentityStore
        + CourseRosterStore
        + SessionStore
        + TeachingAuthorityStore
        + TeachingAuthorityReferenceStore,
{
    let tenant = TenantId::from_uuid(uuid(730_000));
    let context = TenantContext::from_authenticated_session(tenant);
    let sysadmin = UserId::from_uuid(uuid(730_001));
    let instructor = UserId::from_uuid(uuid(730_002));
    let target = UserId::from_uuid(uuid(730_003));
    let outsider = UserId::from_uuid(uuid(730_004));
    let other_target = UserId::from_uuid(uuid(730_006));
    let course = CourseId::from_uuid(uuid(730_005));
    let admin_session = SessionTokenHash::compute(b"t2-admin");
    let instructor_session = SessionTokenHash::compute(b"t2-instructor");
    let non_admin_session = SessionTokenHash::compute(b"t2-non-admin");
    let target_session = SessionTokenHash::compute(b"t2-target");
    let other_target_session = SessionTokenHash::compute(b"t2-other-target");
    let foreign_session = SessionTokenHash::compute(b"t2-foreign");
    let course_creation_authority =
        sysadmin_course_creation_authority(store, tenant, course, instructor).await;
    store
        .create_session(
            admin_session,
            SessionSubject::new(tenant, sysadmin, "T2 operator", vec![UserRole::Sysadmin])
                .expect("operator subject"),
            SessionLifetime::from_seconds(3_600).expect("lifetime"),
        )
        .await
        .expect("operator session");
    store
        .create_session(
            instructor_session,
            SessionSubject::new(
                tenant,
                instructor,
                "T2 course instructor",
                vec![UserRole::Instructor],
            )
            .expect("instructor subject"),
            SessionLifetime::from_seconds(3_600).expect("lifetime"),
        )
        .await
        .expect("instructor session");
    store
        .create_session(
            non_admin_session,
            SessionSubject::new(tenant, outsider, "T2 non-operator", vec![UserRole::Student])
                .expect("non-operator subject"),
            SessionLifetime::from_seconds(3_600).expect("lifetime"),
        )
        .await
        .expect("non-operator session");
    create_account(store, instructor, 0).await;
    create_account(store, target, 1).await;
    create_account(store, other_target, 2).await;
    store
        .create_session(
            target_session,
            SessionSubject::new(tenant, target, "T2 target", vec![UserRole::Instructor])
                .expect("target subject"),
            SessionLifetime::from_seconds(3_600).expect("lifetime"),
        )
        .await
        .expect("target session");
    store
        .create_session(
            other_target_session,
            SessionSubject::new(
                tenant,
                other_target,
                "T2 other target",
                vec![UserRole::Instructor],
            )
            .expect("other target subject"),
            SessionLifetime::from_seconds(3_600).expect("lifetime"),
        )
        .await
        .expect("other target session");
    store
        .create_session(
            foreign_session,
            SessionSubject::new(
                TenantId::from_uuid(uuid(730_007)),
                target,
                "T2 foreign target",
                vec![UserRole::Instructor],
            )
            .expect("foreign target subject"),
            SessionLifetime::from_seconds(3_600).expect("lifetime"),
        )
        .await
        .expect("foreign target session");
    store
        .create_course(
            context,
            CreateCourseCommand {
                course: CourseRecord {
                    id: course,
                    tenant,
                    title: "Co-instructor conformance".to_string(),
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
    assert!(matches!(
        store
            .approve_instructor_account(
                context,
                ApproveInstructorAccount {
                    session: SessionTokenHash::compute(b"not-admin"),
                    target,
                    expected_revision: None,
                },
            )
            .await,
        Err(StoreError::NotFound)
    ));
    assert!(matches!(
        store
            .approve_instructor_account(
                context,
                ApproveInstructorAccount {
                    session: non_admin_session,
                    target,
                    expected_revision: None,
                },
            )
            .await,
        Err(StoreError::Forbidden)
    ));
    let approval = store
        .approve_instructor_account(
            context,
            ApproveInstructorAccount {
                session: admin_session,
                target,
                expected_revision: None,
            },
        )
        .await
        .expect("approval");
    assert_eq!(approval.approval.approved_by, sysadmin);
    assert_eq!(approval.revision, InstructorApprovalRevision::INITIAL);
    let target_reference = store
        .own_account_reference(context, target_session)
        .await
        .expect("active same-tenant session receives its own account locator");
    assert_eq!(target_reference.display_name, "T2 target");
    let search = question_model::CoInstructorTargetSearchRequest {
        query: question_model::CoInstructorTargetSearchQuery::try_from("target".to_owned())
            .expect("bounded nonempty discovery query"),
        after: None,
        size: question_model::TeachingPageSize::try_from(1).expect("small page"),
    };
    assert!(matches!(
        store
            .search_course_co_instructor_targets(context, outsider, course, search.clone())
            .await,
        Err(StoreError::NotFound)
    ));
    let target_search = store
        .search_course_co_instructor_targets(context, instructor, course, search.clone())
        .await
        .expect("direct Instructor receives bounded approved-target discovery");
    assert_eq!(target_search.targets.len(), 1);
    assert_eq!(
        target_search.targets[0].account.reference,
        target_reference.reference
    );
    assert_eq!(
        target_search.targets[0].account.display.as_str(),
        "T2 target"
    );
    assert_eq!(
        target_search.targets[0].approval.state,
        question_model::InstructorApprovalStateView::Approved
    );
    assert!(
        target_search.next_cursor.is_none(),
        "the one approved match has no browse continuation"
    );
    assert!(matches!(
        store
            .search_course_co_instructor_targets(
                context,
                instructor,
                course,
                question_model::CoInstructorTargetSearchRequest {
                    query: question_model::CoInstructorTargetSearchQuery::try_from(
                        "target".to_owned()
                    )
                    .expect("query"),
                    after: Some(String::new()),
                    size: question_model::TeachingPageSize::try_from(1).expect("small page"),
                },
            )
            .await,
        Err(StoreError::InvalidRecord(_))
    ));
    assert_eq!(
        store
            .resolve_account_reference_for_operator(
                context,
                admin_session,
                target_reference.reference
            )
            .await
            .expect("active sysadmin resolves an account locator"),
        Some(target),
    );
    assert!(
        store
            .resolve_account_reference_for_operator(
                context,
                non_admin_session,
                target_reference.reference,
            )
            .await
            .is_err(),
        "ordinary sessions cannot enumerate account locators"
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
            .expect("direct instructor resolves an approved account for the exact course"),
        Some(target),
    );
    assert!(
        store
            .get_current_course_membership(context, course, target)
            .await
            .expect("membership before invitation")
            .is_none(),
        "approval alone does not create course authority"
    );
    assert!(matches!(
        store
            .create_co_instructor_invitation(
                context,
                CreateCoInstructorInvitation {
                    actor: outsider,
                    course,
                    target
                },
            )
            .await,
        Err(StoreError::NotFound)
    ));
    assert!(
        matches!(
            store
                .create_co_instructor_invitation(
                    context,
                    CreateCoInstructorInvitation {
                        actor: sysadmin,
                        course,
                        target
                    },
                )
                .await,
            Err(StoreError::NotFound)
        ),
        "Sysadmin has no ambient course authority"
    );
    assert!(
        matches!(
            store
                .create_co_instructor_invitation(
                    context,
                    CreateCoInstructorInvitation {
                        actor: instructor,
                        course,
                        target: other_target
                    },
                )
                .await,
            Err(StoreError::Forbidden)
        ),
        "existing but unapproved target cannot be invited"
    );
    let page = PageRequest::first(PageSize::new(10).expect("page"));
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
        .expect("invitation");
    assert!(
        store
            .search_course_co_instructor_targets(context, instructor, course, search.clone())
            .await
            .expect("pending invitation target search")
            .targets
            .is_empty(),
        "a current pending invitation target is not offered again"
    );
    assert_eq!(invitation.revision, CoInstructorInvitationRevision::INITIAL);
    let instructor_invitation_views = store
        .list_course_co_instructor_invitation_reference_views(
            context,
            instructor,
            course,
            page.clone(),
        )
        .await
        .expect("direct Instructor invitation references");
    assert_eq!(instructor_invitation_views.items.len(), 1);
    let instructor_invitation_view = &instructor_invitation_views.items[0];
    assert_eq!(
        instructor_invitation_view.target,
        target_reference.reference
    );
    assert_eq!(instructor_invitation_view.target_display_name, "T2 target");
    assert_eq!(
        instructor_invitation_view.target_approval_state,
        question_model::teaching_operations::InstructorApprovalStateView::Approved
    );
    assert_eq!(
        instructor_invitation_view.target_approval_revision,
        approval.revision
    );
    assert_eq!(
        instructor_invitation_view.state,
        question_model::CoInstructorInvitationState::Pending
    );
    assert_eq!(instructor_invitation_view.revision, invitation.revision);
    assert_eq!(
        store
            .resolve_pending_course_co_instructor_invitation_reference(
                context,
                instructor,
                course,
                instructor_invitation_view.reference,
            )
            .await
            .expect("authorized pending course resolver"),
        Some(invitation.invitation.id)
    );
    assert!(matches!(
        store
            .list_course_co_instructor_invitation_reference_views(
                context,
                outsider,
                course,
                page.clone(),
            )
            .await,
        Err(StoreError::NotFound)
    ));
    assert!(matches!(
        store
            .resolve_pending_course_co_instructor_invitation_reference(
                context,
                outsider,
                course,
                instructor_invitation_view.reference,
            )
            .await,
        Err(StoreError::NotFound)
    ));
    let pending_views = store
        .list_pending_co_instructor_invitation_reference_views(
            context,
            target_session,
            page.clone(),
        )
        .await
        .expect("target pending invitation references");
    assert_eq!(pending_views.items.len(), 1);
    assert_eq!(
        pending_views.items[0].reference,
        instructor_invitation_view.reference
    );
    assert_eq!(
        pending_views.items[0].course_title,
        "Co-instructor conformance"
    );
    assert!(matches!(
        store
            .list_pending_co_instructor_invitation_reference_views(
                context,
                non_admin_session,
                page.clone(),
            )
            .await,
        Ok(page) if page.items.is_empty()
    ));
    assert_eq!(
        store
            .create_co_instructor_invitation(
                context,
                CreateCoInstructorInvitation {
                    actor: instructor,
                    course,
                    target
                },
            )
            .await
            .expect("deterministic pending replay"),
        invitation
    );
    assert_eq!(
        store
            .list_pending_co_instructor_invitations(context, target_session, page.clone())
            .await
            .expect("target pending list")
            .items,
        vec![invitation.clone()]
    );
    assert!(
        store
            .list_pending_co_instructor_invitations(context, non_admin_session, page.clone())
            .await
            .expect("wrong-session pending list")
            .items
            .is_empty(),
        "another same-tenant session cannot enumerate the target invitation"
    );
    assert!(matches!(
        store
            .list_pending_co_instructor_invitations(context, foreign_session, page.clone())
            .await,
        Err(StoreError::NotFound)
    ));
    assert!(matches!(
        store
            .list_course_co_instructor_invitations(context, outsider, course, page.clone())
            .await,
        Err(StoreError::NotFound)
    ));
    assert!(matches!(
        store
            .accept_co_instructor_invitation(
                context,
                learning_data_access::RespondToCoInstructorInvitation {
                    session: non_admin_session,
                    actor: other_target,
                    invitation: invitation.invitation.id,
                    expected_revision: invitation.revision,
                },
            )
            .await,
        Err(StoreError::NotFound)
    ));
    let accepted = store
        .accept_co_instructor_invitation(
            context,
            learning_data_access::RespondToCoInstructorInvitation {
                session: target_session,
                actor: target,
                invitation: invitation.invitation.id,
                expected_revision: invitation.revision,
            },
        )
        .await
        .expect("target acceptance");
    assert_eq!(accepted.user, target);
    let instructor_memberships = store
        .list_course_instructor_membership_reference_views(
            context,
            instructor,
            course,
            page.clone(),
        )
        .await
        .expect("active direct Instructor references");
    assert_eq!(
        instructor_memberships.roster_revision, accepted.roster_revision,
        "the page carries the exact course-wide removal token"
    );
    assert!(instructor_memberships.page.items.iter().any(|view| {
        view.account == target_reference.reference && view.account_display_name == "T2 target"
    }));
    let final_instructor_membership_reference = instructor_memberships
        .page
        .items
        .iter()
        .map(|view| view.membership)
        .max_by_key(|reference| reference.number())
        .expect("at least one active direct Instructor membership reference");
    let beyond_end = store
        .list_course_instructor_membership_reference_views(
            context,
            instructor,
            course,
            PageRequest::after(
                Cursor::parse(format!(
                    "{:010}",
                    final_instructor_membership_reference.number()
                ))
                .expect("stable direct Instructor cursor"),
                PageSize::new(10).expect("page size"),
            ),
        )
        .await
        .expect("beyond-end direct Instructor page");
    assert!(
        beyond_end.page.items.is_empty() && beyond_end.page.next_cursor.is_none(),
        "a beyond-end direct Instructor cursor returns an ordinary empty page"
    );
    assert_eq!(
        beyond_end.roster_revision, accepted.roster_revision,
        "an empty direct Instructor page retains the shared removal token"
    );
    assert_eq!(
        store
            .resolve_pending_course_co_instructor_invitation_reference(
                context,
                instructor,
                course,
                instructor_invitation_view.reference,
            )
            .await
            .expect("authorized course resolver"),
        None,
        "accepted invitations cannot be revoked through the pending resolver"
    );
    let membership_views = store
        .list_course_membership_reference_views(context, instructor, course, page.clone())
        .await
        .expect("direct Instructor membership references");
    let target_membership_view = membership_views
        .items
        .iter()
        .find(|view| view.display_name == "T2 target")
        .expect("target membership reference");
    assert_eq!(
        target_membership_view.role,
        question_model::CourseMembershipRole::Instructor
    );
    assert_eq!(
        target_membership_view.status,
        learning_data_access::CourseMemberStatus::Active
    );
    assert!(matches!(
        store
            .list_course_membership_reference_views(context, outsider, course, page.clone())
            .await,
        Err(StoreError::NotFound)
    ));
    let replay = store
        .accept_co_instructor_invitation(
            context,
            learning_data_access::RespondToCoInstructorInvitation {
                session: target_session,
                actor: target,
                invitation: invitation.invitation.id,
                expected_revision: invitation.revision,
            },
        )
        .await
        .expect("accepted replay");
    assert_eq!(replay.membership, accepted.membership);
    assert!(matches!(
        store
            .remove_direct_instructor_membership(
                context,
                learning_data_access::RemoveDirectInstructorMembership {
                    actor: instructor,
                    course,
                    membership: accepted.membership,
                    expected_roster_revision: accepted.roster_revision,
                },
            )
            .await,
        Ok(())
    ));
    let active_after_removal = store
        .list_course_instructor_membership_reference_views(
            context,
            instructor,
            course,
            page.clone(),
        )
        .await
        .expect("active direct Instructor references after removal");
    assert!(
        active_after_removal
            .page
            .items
            .iter()
            .all(|view| view.account != target_reference.reference)
    );
    assert_eq!(
        store
            .search_course_co_instructor_targets(context, instructor, course, search.clone())
            .await
            .expect("former direct Instructor returns to eligible search")
            .targets,
        vec![question_model::CoInstructorTargetView {
            account: question_model::TeachingAccountView {
                reference: target_reference.reference,
                display: question_model::TeachingDisplayLabel::try_from("T2 target".to_owned())
                    .expect("safe display"),
            },
            approval: question_model::AccountApprovalView {
                state: question_model::InstructorApprovalStateView::Approved,
                revision: question_model::TeachingOperationRevision::new(1).expect("revision"),
            },
        }],
        "a removed direct Instructor may be discovered again only while globally approved"
    );
    let remaining = store
        .get_current_course_membership(context, course, instructor)
        .await
        .expect("remaining instructor")
        .expect("initial instructor remains");
    let remaining_revision = store
        .list_course_roster(context, instructor_session, course, page.clone())
        .await
        .expect("remaining roster")
        .policy
        .revision;
    assert!(
        matches!(
            store
                .remove_direct_instructor_membership(
                    context,
                    learning_data_access::RemoveDirectInstructorMembership {
                        actor: instructor,
                        course,
                        membership: remaining.id,
                        expected_roster_revision: remaining_revision,
                    },
                )
                .await,
            Err(StoreError::Conflict)
        ),
        "the final active Instructor is not removable"
    );
    let other_approval = store
        .approve_instructor_account(
            context,
            ApproveInstructorAccount {
                session: admin_session,
                target: other_target,
                expected_revision: None,
            },
        )
        .await
        .expect("second approval");
    let first_search_page = store
        .search_course_co_instructor_targets(context, instructor, course, search.clone())
        .await
        .expect("first bounded target-search page");
    assert_eq!(first_search_page.targets.len(), 1);
    let second_search_page = store
        .search_course_co_instructor_targets(
            context,
            instructor,
            course,
            question_model::CoInstructorTargetSearchRequest {
                query: question_model::CoInstructorTargetSearchQuery::try_from("target".to_owned())
                    .expect("query"),
                after: first_search_page.next_cursor.clone(),
                size: question_model::TeachingPageSize::try_from(1).expect("small page"),
            },
        )
        .await
        .expect("stable continuation page");
    assert_eq!(second_search_page.targets.len(), 1);
    assert_ne!(
        first_search_page.targets[0].account.reference,
        second_search_page.targets[0].account.reference
    );
    assert!(second_search_page.next_cursor.is_none());
    let safe_search_json = serde_json::to_string(&first_search_page).expect("safe search JSON");
    assert!(!safe_search_json.contains("@example.edu"));
    assert!(!safe_search_json.contains("730_"));
    let declined = store
        .create_co_instructor_invitation(
            context,
            CreateCoInstructorInvitation {
                actor: instructor,
                course,
                target: other_target,
            },
        )
        .await
        .expect("second invitation");
    assert!(matches!(
        store
            .decline_co_instructor_invitation(
                context,
                learning_data_access::RespondToCoInstructorInvitation {
                    session: target_session,
                    actor: target,
                    invitation: declined.invitation.id,
                    expected_revision: declined.revision,
                },
            )
            .await,
        Err(StoreError::NotFound)
    ));
    store
        .decline_co_instructor_invitation(
            context,
            learning_data_access::RespondToCoInstructorInvitation {
                session: other_target_session,
                actor: other_target,
                invitation: declined.invitation.id,
                expected_revision: declined.revision,
            },
        )
        .await
        .expect("target decline");
    assert!(matches!(
        store
            .decline_co_instructor_invitation(
                context,
                learning_data_access::RespondToCoInstructorInvitation {
                    session: other_target_session,
                    actor: other_target,
                    invitation: declined.invitation.id,
                    expected_revision: declined.revision,
                },
            )
            .await,
        Err(StoreError::Conflict)
    ));
    let revoked = store
        .create_co_instructor_invitation(
            context,
            CreateCoInstructorInvitation {
                actor: instructor,
                course,
                target: other_target,
            },
        )
        .await
        .expect("revocable invitation");
    store
        .revoke_co_instructor_invitation(
            context,
            learning_data_access::RevokeCoInstructorInvitation {
                actor: instructor,
                course,
                invitation: revoked.invitation.id,
                expected_revision: revoked.revision,
            },
        )
        .await
        .expect("instructor revoke");
    assert!(matches!(
        store
            .revoke_co_instructor_invitation(
                context,
                learning_data_access::RevokeCoInstructorInvitation {
                    actor: instructor,
                    course,
                    invitation: revoked.invitation.id,
                    expected_revision: revoked.revision,
                },
            )
            .await,
        Err(StoreError::Conflict)
    ));
    let pending_after_approval = store
        .create_co_instructor_invitation(
            context,
            CreateCoInstructorInvitation {
                actor: instructor,
                course,
                target: other_target,
            },
        )
        .await
        .expect("approval revocation invitation");
    store
        .revoke_instructor_approval(
            context,
            RevokeInstructorApproval {
                session: admin_session,
                target: other_target,
                expected_revision: other_approval.revision,
            },
        )
        .await
        .expect("second approval revocation");
    let after_revocation = store
        .search_course_co_instructor_targets(context, instructor, course, search.clone())
        .await
        .expect("revoked approval is removed from discovery");
    assert!(
        after_revocation
            .targets
            .iter()
            .all(|target| target.account.reference
                != second_search_page.targets[0].account.reference),
        "revocation removes the formerly approved target without exposing identity data"
    );
    assert!(matches!(
        store
            .accept_co_instructor_invitation(
                context,
                learning_data_access::RespondToCoInstructorInvitation {
                    session: other_target_session,
                    actor: other_target,
                    invitation: pending_after_approval.invitation.id,
                    expected_revision: pending_after_approval.revision,
                },
            )
            .await,
        Err(StoreError::InvalidRecord(_))
    ));
    assert!(
        store
            .get_current_course_membership(context, course, other_target)
            .await
            .expect("atomic denied membership")
            .is_none()
    );
    let foreign_context =
        TenantContext::from_authenticated_session(TenantId::from_uuid(uuid(730_007)));
    assert!(matches!(
        store
            .list_pending_co_instructor_invitations(foreign_context, target_session, page.clone())
            .await,
        Err(StoreError::NotFound)
    ));
    assert!(matches!(
        store
            .list_course_co_instructor_invitations(foreign_context, instructor, course, page)
            .await,
        Err(StoreError::NotFound)
    ));
    let revocation = store
        .revoke_instructor_approval(
            context,
            RevokeInstructorApproval {
                session: admin_session,
                target,
                expected_revision: approval.revision,
            },
        )
        .await
        .expect("approval revocation");
    assert!(revocation.approval.revoked_at.is_some());
    assert!(matches!(
        store
            .revoke_instructor_approval(
                context,
                RevokeInstructorApproval {
                    session: admin_session,
                    target,
                    expected_revision: approval.revision,
                },
            )
            .await,
        Err(StoreError::Conflict)
    ));
}

async fn create_account<S>(store: &S, user: UserId, suffix: u128)
where
    S: AccountIdentityStore,
{
    let token = EmailChallengeSecretHash::compute(format!("t2-account-{suffix}").as_bytes());
    let binding = BrowserBindingHash::compute(format!("t2-binding-{suffix}").as_bytes());
    store
        .begin_email_authentication(BeginEmailAuthentication {
            id: EmailChallengeId::from_uuid(uuid(730_100 + suffix)),
            token_hash: token,
            browser_binding: binding,
            email_rate_limit_key: AuthenticationRateLimitKey::compute(
                format!("t2-rate-{suffix}").as_bytes(),
            ),
            email: AuthenticationEmail::parse(&format!("t2-{suffix}@example.edu")).expect("email"),
            purpose: EmailAuthenticationPurpose::SignInOrRegister,
            lifetime: EmailChallengeLifetime::from_seconds(600).expect("lifetime"),
        })
        .await
        .expect("account challenge");
    store
        .complete_email_authentication(CompleteEmailAuthentication {
            token_hash: token,
            browser_binding: binding,
            proposed_user: user,
            proposed_display_name: "T2 target".to_string(),
        })
        .await
        .expect("account");
}

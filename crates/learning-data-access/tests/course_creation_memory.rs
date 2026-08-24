//! Black-box authority and aggregate behavior for Memory course creation.

use learning_data_access::in_memory::MemoryStore;
use learning_data_access::{
    AccountIdentityStore, ApproveInstructorAccount, AuthenticationEmail,
    AuthenticationRateLimitKey, BeginEmailAuthentication, BrowserBindingHash,
    CompleteEmailAuthentication, CourseCreationAuthority, CourseGradeSchemeRevision,
    CourseGradebookStore, CourseListScope, CourseMemberStatus, CourseRecord, CourseRosterContact,
    CourseRosterId, CourseRosterStore, CourseSignupPosture, CreateCourseCommand,
    EmailAuthenticationPurpose, EmailChallengeId, EmailChallengeLifetime, EmailChallengeSecretHash,
    PageRequest, PageSize, SessionLifetime, SessionStore, SessionSubject, SessionTokenHash, Store,
    StoreError, TeachingAuthorityStore, TenantContext, UpsertCourseMember,
};
use question_model::{ActivityTimestamp, CourseGradeMode, CourseId, TenantId, UserId, UserRole};
use uuid::Uuid;

fn fixture_uuid(value: u128) -> Uuid {
    Uuid::from_u128(value)
}

fn context(tenant: TenantId) -> TenantContext {
    TenantContext::from_authenticated_session(tenant)
}

fn course(tenant: TenantId, id: CourseId, title: &str) -> CourseRecord {
    CourseRecord {
        id,
        tenant,
        title: title.to_string(),
        term: question_model::CourseTerm::from_parts("2026-08-24", "2026-12-18", "America/Chicago")
            .expect("fixed fixture term"),
    }
}

fn page() -> PageRequest {
    PageRequest::first(PageSize::new(10).expect("small page size"))
}

async fn session(
    store: &MemoryStore,
    tenant: TenantId,
    user: UserId,
    roles: Vec<UserRole>,
    token: &[u8],
    lifetime: u32,
) -> SessionTokenHash {
    let token = SessionTokenHash::compute(token);
    store
        .create_session(
            token,
            SessionSubject::new(tenant, user, "Course creation fixture", roles)
                .expect("valid fixture subject"),
            SessionLifetime::from_seconds(lifetime).expect("positive fixture lifetime"),
        )
        .await
        .expect("fixture session persists");
    token
}

async fn account(store: &MemoryStore, user: UserId, suffix: u128) {
    let token = EmailChallengeSecretHash::compute(format!("course-create-{suffix}").as_bytes());
    let binding = BrowserBindingHash::compute(format!("course-binding-{suffix}").as_bytes());
    store
        .begin_email_authentication(BeginEmailAuthentication {
            id: EmailChallengeId::from_uuid(fixture_uuid(100_000 + suffix)),
            token_hash: token,
            browser_binding: binding,
            email_rate_limit_key: AuthenticationRateLimitKey::compute(
                format!("course-rate-{suffix}").as_bytes(),
            ),
            email: AuthenticationEmail::parse(&format!("course-{suffix}@example.edu"))
                .expect("fixture email"),
            purpose: EmailAuthenticationPurpose::SignInOrRegister,
            lifetime: EmailChallengeLifetime::from_seconds(600)
                .expect("fixture challenge lifetime"),
        })
        .await
        .expect("fixture account challenge");
    store
        .complete_email_authentication(CompleteEmailAuthentication {
            token_hash: token,
            browser_binding: binding,
            proposed_user: user,
            proposed_display_name: "Course creation instructor".to_string(),
        })
        .await
        .expect("fixture account");
}

async fn approved_instructor(
    store: &MemoryStore,
    tenant: TenantId,
    sysadmin: UserId,
    instructor: UserId,
) -> (SessionTokenHash, SessionTokenHash) {
    let sysadmin_session = session(
        store,
        tenant,
        sysadmin,
        vec![UserRole::Sysadmin],
        b"course-creation-sysadmin",
        3_600,
    )
    .await;
    let instructor_session = session(
        store,
        tenant,
        instructor,
        vec![UserRole::Instructor],
        b"course-creation-instructor",
        3_600,
    )
    .await;
    account(store, instructor, instructor.as_uuid().as_u128()).await;
    store
        .approve_instructor_account(
            context(tenant),
            ApproveInstructorAccount {
                session: sysadmin_session,
                target: instructor,
                expected_revision: None,
            },
        )
        .await
        .expect("Sysadmin approves fixture instructor");
    (sysadmin_session, instructor_session)
}

async fn assert_no_course_aggregate(
    store: &MemoryStore,
    tenant: TenantId,
    course_id: CourseId,
    actor: UserId,
) {
    assert!(
        store
            .get_course(context(tenant), course_id)
            .await
            .expect("course lookup")
            .is_none(),
        "denied creation leaves no course record"
    );
    assert!(
        store
            .get_current_course_membership(context(tenant), course_id, actor)
            .await
            .expect("membership lookup")
            .is_none(),
        "denied creation leaves no instructor membership"
    );
    assert!(
        store
            .list_courses(context(tenant), CourseListScope::Member(actor), page())
            .await
            .expect("course list")
            .items
            .is_empty(),
        "denied creation leaves no discoverable course aggregate"
    );
}

#[tokio::test]
async fn approved_instructor_creation_exposes_complete_initial_course_aggregate() {
    let store = MemoryStore::default();
    let tenant = TenantId::from_uuid(fixture_uuid(1));
    let sysadmin = UserId::from_uuid(fixture_uuid(2));
    let instructor = UserId::from_uuid(fixture_uuid(3));
    let course_id = CourseId::from_uuid(fixture_uuid(4));
    let (_, instructor_session) = approved_instructor(&store, tenant, sysadmin, instructor).await;

    store
        .create_course(
            context(tenant),
            CreateCourseCommand {
                course: course(tenant, course_id, "Molecular genetics"),
                authority: CourseCreationAuthority::ApprovedInstructor {
                    actor: instructor,
                    session: instructor_session,
                },
            },
        )
        .await
        .expect("approved Instructor creates course");

    let membership = store
        .get_current_course_membership(context(tenant), course_id, instructor)
        .await
        .expect("membership lookup")
        .expect("initial instructor membership");
    assert_eq!(membership.status, CourseMemberStatus::Active);
    let roster = store
        .list_course_roster(context(tenant), instructor_session, course_id, page())
        .await
        .expect("initial roster reads immediately");
    assert!(roster.entries.items.is_empty());
    assert_eq!(roster.policy.course, course_id);
    assert_eq!(
        roster.policy.signup_posture,
        CourseSignupPosture::InvitationOnly
    );
    let scheme = store
        .course_grade_scheme(context(tenant), instructor_session, course_id)
        .await
        .expect("initial grade scheme reads immediately");
    assert_eq!(scheme.course, course_id);
    assert_eq!(scheme.revision, CourseGradeSchemeRevision::INITIAL);
    assert_eq!(scheme.scheme.mode, CourseGradeMode::TotalPoints);
}

#[tokio::test]
async fn sysadmin_creation_uses_authenticated_actor_and_provisions_initial_instructor() {
    let store = MemoryStore::default();
    let tenant = TenantId::from_uuid(fixture_uuid(10));
    let sysadmin = UserId::from_uuid(fixture_uuid(11));
    let initial_instructor = UserId::from_uuid(fixture_uuid(12));
    let course_id = CourseId::from_uuid(fixture_uuid(13));
    let sysadmin_session = session(
        &store,
        tenant,
        sysadmin,
        vec![UserRole::Sysadmin],
        b"sysadmin-creation",
        3_600,
    )
    .await;

    store
        .create_course(
            context(tenant),
            CreateCourseCommand {
                course: course(tenant, course_id, "Protein structure"),
                authority: CourseCreationAuthority::Sysadmin {
                    actor: sysadmin,
                    session: sysadmin_session,
                },
            },
        )
        .await
        .expect("Sysadmin creates course");

    assert!(
        store
            .get_current_course_membership(context(tenant), course_id, sysadmin)
            .await
            .expect("Sysadmin membership lookup")
            .is_some(),
        "the authenticated Sysadmin is the initial instructor; a caller cannot name another one"
    );
    assert!(
        store
            .get_current_course_membership(context(tenant), course_id, initial_instructor)
            .await
            .expect("unrelated user lookup")
            .is_none()
    );
}

#[tokio::test]
async fn denied_authority_variants_leave_no_course_aggregate() {
    let store = MemoryStore::default();
    let tenant = TenantId::from_uuid(fixture_uuid(20));
    let sysadmin = UserId::from_uuid(fixture_uuid(21));
    let instructor = UserId::from_uuid(fixture_uuid(22));
    let stranger = UserId::from_uuid(fixture_uuid(23));
    let (sysadmin_session, instructor_session) =
        approved_instructor(&store, tenant, sysadmin, instructor).await;
    let stranger_session = session(
        &store,
        tenant,
        stranger,
        vec![UserRole::Instructor],
        b"unapproved-instructor",
        3_600,
    )
    .await;

    let unapproved_course = CourseId::from_uuid(fixture_uuid(24));
    assert_eq!(
        store
            .create_course(
                context(tenant),
                CreateCourseCommand {
                    course: course(tenant, unapproved_course, "Unapproved"),
                    authority: CourseCreationAuthority::ApprovedInstructor {
                        actor: stranger,
                        session: stranger_session,
                    },
                },
            )
            .await,
        Err(StoreError::Forbidden)
    );
    assert_no_course_aggregate(&store, tenant, unapproved_course, stranger).await;

    let mismatched_course = CourseId::from_uuid(fixture_uuid(25));
    assert_eq!(
        store
            .create_course(
                context(tenant),
                CreateCourseCommand {
                    course: course(tenant, mismatched_course, "Mismatched"),
                    authority: CourseCreationAuthority::ApprovedInstructor {
                        actor: stranger,
                        session: instructor_session,
                    },
                },
            )
            .await,
        Err(StoreError::Forbidden)
    );
    assert_no_course_aggregate(&store, tenant, mismatched_course, stranger).await;

    let expired_course = CourseId::from_uuid(fixture_uuid(26));
    let expired_session = session(
        &store,
        tenant,
        instructor,
        vec![UserRole::Instructor],
        b"expired-approved-instructor",
        1,
    )
    .await;
    store
        .set_authoritative_time(ActivityTimestamp::from_unix_millis(1_001))
        .expect("advance authoritative time past fixture session");
    assert_eq!(
        store
            .create_course(
                context(tenant),
                CreateCourseCommand {
                    course: course(tenant, expired_course, "Expired"),
                    authority: CourseCreationAuthority::ApprovedInstructor {
                        actor: instructor,
                        session: expired_session,
                    },
                },
            )
            .await,
        Err(StoreError::NotFound)
    );
    assert_no_course_aggregate(&store, tenant, expired_course, instructor).await;

    let mismatched_sysadmin_course = CourseId::from_uuid(fixture_uuid(27));
    assert_eq!(
        store
            .create_course(
                context(tenant),
                CreateCourseCommand {
                    course: course(tenant, mismatched_sysadmin_course, "Mismatched Sysadmin"),
                    authority: CourseCreationAuthority::Sysadmin {
                        actor: instructor,
                        session: sysadmin_session,
                    },
                },
            )
            .await,
        Err(StoreError::Forbidden)
    );
    assert_no_course_aggregate(&store, tenant, mismatched_sysadmin_course, instructor).await;
}

#[tokio::test]
async fn duplicate_and_concurrent_course_creation_have_deterministic_one_winner_semantics() {
    let store = MemoryStore::default();
    let tenant = TenantId::from_uuid(fixture_uuid(30));
    let sysadmin = UserId::from_uuid(fixture_uuid(31));
    let course_id = CourseId::from_uuid(fixture_uuid(32));
    let session = session(
        &store,
        tenant,
        sysadmin,
        vec![UserRole::Sysadmin],
        b"race-sysadmin",
        3_600,
    )
    .await;
    let command = CreateCourseCommand {
        course: course(tenant, course_id, "Concurrent systems biology"),
        authority: CourseCreationAuthority::Sysadmin {
            actor: sysadmin,
            session,
        },
    };

    let (left, right) = tokio::join!(
        store.create_course(context(tenant), command.clone()),
        store.create_course(context(tenant), command.clone())
    );
    assert!(
        matches!(
            (left, right),
            (Ok(()), Err(StoreError::AlreadyExists)) | (Err(StoreError::AlreadyExists), Ok(()))
        ),
        "exactly one concurrent request creates the aggregate"
    );
    assert_eq!(
        store.create_course(context(tenant), command).await,
        Err(StoreError::AlreadyExists),
        "a later duplicate has the same explicit conflict outcome"
    );
    assert!(
        store
            .get_current_course_membership(context(tenant), course_id, sysadmin)
            .await
            .expect("membership lookup")
            .is_some(),
        "the winning aggregate contains its initial instructor"
    );
}

#[tokio::test]
async fn roster_upsert_requires_exact_direct_instructor_and_rolls_back_conflicts() {
    let store = MemoryStore::default();
    let tenant = TenantId::from_uuid(fixture_uuid(40));
    let direct_instructor = UserId::from_uuid(fixture_uuid(41));
    let other_instructor = UserId::from_uuid(fixture_uuid(42));
    let sysadmin_only = UserId::from_uuid(fixture_uuid(43));
    let outsider = UserId::from_uuid(fixture_uuid(44));
    let course_id = CourseId::from_uuid(fixture_uuid(45));
    let other_course = CourseId::from_uuid(fixture_uuid(46));
    let direct_session = session(
        &store,
        tenant,
        direct_instructor,
        vec![UserRole::Sysadmin],
        b"direct-roster-instructor",
        3_600,
    )
    .await;
    let other_session = session(
        &store,
        tenant,
        other_instructor,
        vec![UserRole::Sysadmin],
        b"other-roster-instructor",
        3_600,
    )
    .await;
    session(
        &store,
        tenant,
        sysadmin_only,
        vec![UserRole::Sysadmin],
        b"roster-sysadmin-only",
        3_600,
    )
    .await;
    for (course_id, title, actor, actor_session) in [
        (
            course_id,
            "Direct roster authority",
            direct_instructor,
            direct_session,
        ),
        (
            other_course,
            "Other roster authority",
            other_instructor,
            other_session,
        ),
    ] {
        store
            .create_course(
                context(tenant),
                CreateCourseCommand {
                    course: course(tenant, course_id, title),
                    authority: CourseCreationAuthority::Sysadmin {
                        actor,
                        session: actor_session,
                    },
                },
            )
            .await
            .expect("direct Instructor course fixture");
    }

    let learner = UserId::from_uuid(fixture_uuid(47));
    let learner_command = UpsertCourseMember {
        course: course_id,
        user: learner,
        display_name: "Canonical Memory learner".to_string(),
        roster_contact: None,
    };
    let (first, replay) = tokio::join!(
        store.upsert_course_member(context(tenant), direct_instructor, learner_command.clone()),
        store.upsert_course_member(context(tenant), direct_instructor, learner_command.clone())
    );
    let first = first.expect("direct Instructor roster activation");
    let replay = replay.expect("concurrent-equivalent roster replay");
    assert_eq!(first, replay);
    assert_eq!(first.roster_revision.value(), 2);
    let divergent = store
        .upsert_course_member(
            context(tenant),
            direct_instructor,
            UpsertCourseMember {
                display_name: "Divergent retry".to_string(),
                ..learner_command
            },
        )
        .await
        .expect("divergent replay returns canonical profile");
    assert_eq!(divergent.member.display_name, "Canonical Memory learner");
    assert_eq!(divergent.roster_revision.value(), 2);

    for actor in [outsider, other_instructor, sysadmin_only] {
        let target = UserId::from_uuid(fixture_uuid(100 + actor.as_uuid().as_u128()));
        assert!(
            matches!(
                store
                    .upsert_course_member(
                        context(tenant),
                        actor,
                        UpsertCourseMember {
                            course: course_id,
                            user: target,
                            display_name: "Unauthorized learner".to_string(),
                            roster_contact: None,
                        },
                    )
                    .await,
                Err(StoreError::Forbidden | StoreError::NotFound)
            ),
            "ordinary roster activation requires exact direct Instructor authority"
        );
        assert!(
            store
                .get_current_course_membership(context(tenant), course_id, target)
                .await
                .expect("unauthorized target membership lookup")
                .is_none()
        );
    }

    let contact_owner = UserId::from_uuid(fixture_uuid(50));
    let contact = CourseRosterContact {
        email: AuthenticationEmail::parse("memory-contact@example.edu")
            .expect("Memory contact fixture email"),
        roster_id: CourseRosterId::parse("memory-contact-1")
            .expect("Memory contact fixture roster ID"),
    };
    let contact_record = store
        .upsert_course_member(
            context(tenant),
            direct_instructor,
            UpsertCourseMember {
                course: course_id,
                user: contact_owner,
                display_name: "Memory contact owner".to_string(),
                roster_contact: Some(contact.clone()),
            },
        )
        .await
        .expect("contact-bearing Memory learner");
    let conflict_target = UserId::from_uuid(fixture_uuid(51));
    assert_eq!(
        store
            .upsert_course_member(
                context(tenant),
                direct_instructor,
                UpsertCourseMember {
                    course: course_id,
                    user: conflict_target,
                    display_name: "Memory contact conflict".to_string(),
                    roster_contact: Some(contact),
                },
            )
            .await,
        Err(StoreError::Conflict)
    );
    assert!(
        store
            .get_current_course_membership(context(tenant), course_id, conflict_target)
            .await
            .expect("conflicting target membership lookup")
            .is_none()
    );
    let roster = store
        .list_course_roster(context(tenant), direct_session, course_id, page())
        .await
        .expect("direct Instructor roster projection");
    assert_eq!(roster.policy.revision, contact_record.roster_revision);
    assert!(
        store
            .roster_support_audits()
            .expect("Memory audit state")
            .is_empty()
    );
}

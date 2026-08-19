//! Passwordless-account and atomic course-roster behavior.

use super::*;

#[path = "enrollment/account_authentication.rs"]
mod account_authentication;

async fn create_roster_session(
    store: &MemoryStore,
    tenant: TenantId,
    user: UserId,
    roles: Vec<UserRole>,
    token: &[u8],
) -> SessionTokenHash {
    let token_hash = SessionTokenHash::compute(token);
    store
        .create_session(
            token_hash,
            SessionSubject::new(tenant, user, "Roster instructor", roles).expect("session subject"),
            SessionLifetime::from_seconds(3_600).expect("session lifetime"),
        )
        .await
        .expect("roster session");
    token_hash
}

#[tokio::test]
async fn memory_invitation_claim_reconciles_both_assignment_creation_orders() {
    let store = MemoryStore::default();
    store
        .set_authoritative_time(ActivityTimestamp::from_unix_millis(10_000))
        .expect("fixture clock");
    let tenant = TenantId::from_uuid(uuid(121_000));
    let context = TenantContext::from_authenticated_session(tenant);
    let instructor = UserId::from_uuid(uuid(121_001));
    let learner = UserId::from_uuid(uuid(121_002));
    let course = CourseId::from_uuid(uuid(121_003));
    store
        .create_course(
            context,
            CreateCourseCommand {
                course: CourseRecord {
                    id: course,
                    tenant,
                    title: "Receipt materialization".to_string(),
                    term: question_model::CourseTerm::from_parts(
                        "2026-08-24",
                        "2026-12-18",
                        "America/Chicago",
                    )
                    .expect("explicit fixture course term"),
                },
                initial_instructor: instructor,
            },
        )
        .await
        .expect("course");
    let instructor_session = create_roster_session(
        &store,
        tenant,
        instructor,
        vec![UserRole::Instructor],
        b"roster-cross-product-instructor",
    )
    .await;

    let first_version = publish_assignment_version(
        &store,
        context,
        tenant,
        instructor,
        121_010,
        PublicationScope::Public,
    )
    .await;
    let first_assignment = AssignmentId::from_uuid(uuid(121_020));
    store
        .create_untimed_assignment(
            context,
            AssignmentRecord {
                id: first_assignment,
                tenant,
                course_id: course,
                title: "Created before claim".to_string(),
                audience: question_model::AssignmentAudience::CourseWide,
                items: fixed_items(vec![first_version]),
                selection_groups: Vec::new(),
                policies: policies(),
            },
        )
        .await
        .expect("first assignment");

    let token_hash = CourseInvitationSecretHash::compute(b"course-invitation");
    store
        .create_course_invitation(
            context,
            instructor_session,
            CreateCourseInvitation {
                course,
                email: AuthenticationEmail::parse("learner@mail.roosevelt.edu")
                    .expect("valid invitation email"),
                roster_id: CourseRosterId::parse("900123456").expect("valid roster ID"),
                token_hash,
                idempotency_key: RosterIdempotencyKey::parse("invite-121-002")
                    .expect("valid idempotency key"),
                lifetime: CourseInvitationLifetime::from_seconds(7 * 24 * 60 * 60)
                    .expect("bounded invitation lifetime"),
            },
        )
        .await
        .expect("invitation");
    let claimed = store
        .claim_course_invitation(ClaimCourseInvitation {
            token_hash,
            user: learner,
            verified_email: AuthenticationEmail::parse("learner@mail.roosevelt.edu")
                .expect("verified email"),
            display_name: "Course Learner".to_string(),
        })
        .await
        .expect("atomic invitation claim");
    assert_eq!(claimed.member.user, learner);

    let second_version = publish_assignment_version(
        &store,
        context,
        tenant,
        instructor,
        121_030,
        PublicationScope::Public,
    )
    .await;
    let second_assignment = AssignmentId::from_uuid(uuid(121_040));
    store
        .create_untimed_assignment(
            context,
            AssignmentRecord {
                id: second_assignment,
                tenant,
                course_id: course,
                title: "Created after claim".to_string(),
                audience: question_model::AssignmentAudience::CourseWide,
                items: fixed_items(vec![second_version]),
                selection_groups: Vec::new(),
                policies: policies(),
            },
        )
        .await
        .expect("second assignment");

    let gradebook = store
        .list_gradebook_rows(
            context,
            course,
            PageRequest::first(PageSize::new(20).unwrap()),
        )
        .await
        .expect("summary-only gradebook remains readable");
    assert!(
        gradebook.items.is_empty(),
        "roster and assignment writes must not eagerly materialize receipts"
    );
    store
        .start_or_resume_run(
            context,
            learner,
            first_assignment,
            RunId::from_uuid(uuid(121_041)),
        )
        .await
        .expect("first learner action materializes one receipt");
    store
        .start_or_resume_run(
            context,
            learner,
            second_assignment,
            RunId::from_uuid(uuid(121_042)),
        )
        .await
        .expect("second learner action materializes its own receipt");
    let gradebook = store
        .list_gradebook_rows(
            context,
            course,
            PageRequest::first(PageSize::new(20).unwrap()),
        )
        .await
        .expect("materialized gradebook remains readable");
    assert_eq!(gradebook.items.len(), 2);
    assert_eq!(
        gradebook
            .items
            .iter()
            .map(|row| row.assignment_id)
            .collect::<std::collections::BTreeSet<_>>(),
        std::collections::BTreeSet::from([first_assignment, second_assignment])
    );
    assert!(
        gradebook.items.iter().all(|row| {
            row.student_id == claimed.member.student
                && row.summary.current_score.is_none()
                && row.summary.completed_run_count == 0
                && row.summary.total_question_attempts == 0
        }),
        "each enrollment has its required empty summary"
    );
    let export = store
        .create_manual_grade_export(
            context,
            instructor_session,
            CreateManualGradeExport {
                course,
                assignment: first_assignment,
            },
        )
        .await
        .expect("manual export should use the protected roster mapping");
    assert_eq!(export.rows.len(), 1);
    assert_eq!(export.rows[0].roster_id.as_str(), "900123456");
    assert_eq!(
        export.rows[0].roster_email.normalized(),
        "learner@mail.roosevelt.edu"
    );
    assert_eq!(export.rows[0].display_name, "Course Learner");
    assert_eq!(export.rows[0].current_score, None);

    store
        .revoke_course_member(
            context,
            instructor_session,
            RevokeCourseMember {
                course,
                member: claimed.member.id,
                expected_revision: claimed.roster_revision,
            },
        )
        .await
        .expect("revocation preserves records while removing learner authority");
    assert_eq!(
        store
            .start_or_resume_run(
                context,
                learner,
                first_assignment,
                RunId::from_uuid(uuid(121_050)),
            )
            .await,
        Err(StoreError::NotFound),
        "a retained enrollment cannot outlive its revoked course membership"
    );
    assert_eq!(
        store
            .list_gradebook_rows(
                context,
                course,
                PageRequest::first(PageSize::new(20).unwrap())
            )
            .await
            .expect("instructor retains historical gradebook authority")
            .items
            .len(),
        2
    );
}

#[tokio::test]
async fn memory_course_allows_only_one_live_invitation_per_email() {
    let store = MemoryStore::default();
    store
        .set_authoritative_time(ActivityTimestamp::from_unix_millis(20_000))
        .expect("fixture clock");
    let tenant = TenantId::from_uuid(uuid(122_000));
    let context = TenantContext::from_authenticated_session(tenant);
    let instructor = UserId::from_uuid(uuid(122_001));
    let course = CourseId::from_uuid(uuid(122_002));
    store
        .create_course(
            context,
            CreateCourseCommand {
                course: CourseRecord {
                    id: course,
                    tenant,
                    title: "Pending invitation identity".to_string(),
                    term: question_model::CourseTerm::from_parts(
                        "2026-08-24",
                        "2026-12-18",
                        "America/Chicago",
                    )
                    .expect("explicit fixture course term"),
                },
                initial_instructor: instructor,
            },
        )
        .await
        .expect("course");
    let instructor_session = create_roster_session(
        &store,
        tenant,
        instructor,
        vec![UserRole::Instructor],
        b"duplicate-email-instructor",
    )
    .await;
    let email = AuthenticationEmail::parse("learner@example.edu").expect("valid email");
    store
        .create_course_invitation(
            context,
            instructor_session,
            CreateCourseInvitation {
                course,
                email: email.clone(),
                roster_id: CourseRosterId::parse("900122001").expect("valid roster ID"),
                token_hash: CourseInvitationSecretHash::compute(b"first-invitation"),
                idempotency_key: RosterIdempotencyKey::parse("first-invitation")
                    .expect("valid idempotency key"),
                lifetime: CourseInvitationLifetime::from_seconds(86_400).expect("bounded lifetime"),
            },
        )
        .await
        .expect("first invitation");

    assert_eq!(
        store
            .create_course_invitation(
                context,
                instructor_session,
                CreateCourseInvitation {
                    course,
                    email,
                    roster_id: CourseRosterId::parse("900122002").expect("valid roster ID"),
                    token_hash: CourseInvitationSecretHash::compute(b"second-invitation"),
                    idempotency_key: RosterIdempotencyKey::parse("second-invitation")
                        .expect("valid idempotency key"),
                    lifetime: CourseInvitationLifetime::from_seconds(86_400)
                        .expect("bounded lifetime"),
                },
            )
            .await,
        Err(StoreError::AlreadyExists),
        "a second live invitation could otherwise overwrite the first roster identity when claimed"
    );
}

#[path = "enrollment/roster_support_and_delivery.rs"]
mod roster_support_and_delivery;

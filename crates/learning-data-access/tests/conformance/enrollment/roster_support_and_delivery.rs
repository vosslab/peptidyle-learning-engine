use super::*;

#[tokio::test]
async fn memory_sysadmin_roster_support_is_narrow_and_audited() {
    let store = MemoryStore::default();
    store
        .set_authoritative_time(ActivityTimestamp::from_unix_millis(30_000))
        .expect("fixture clock");
    let tenant = TenantId::from_uuid(uuid(123_000));
    let context = TenantContext::from_authenticated_session(tenant);
    let instructor = UserId::from_uuid(uuid(123_001));
    let sysadmin = UserId::from_uuid(uuid(123_002));
    let course = CourseId::from_uuid(uuid(123_003));
    let course_creation_authority =
        sysadmin_course_creation_authority(&store, tenant, course, instructor).await;
    store
        .create_course(
            context,
            CreateCourseCommand {
                course: CourseRecord {
                    id: course,
                    tenant,
                    title: "Sysadmin roster authority".to_string(),
                    term: question_model::CourseTerm::from_parts(
                        "2026-08-24",
                        "2026-12-18",
                        "America/Chicago",
                    )
                    .expect("explicit fixture course term"),
                },
                authority: course_creation_authority,
            },
        )
        .await
        .expect("course");
    let sysadmin_session = create_roster_session(
        &store,
        tenant,
        sysadmin,
        vec![UserRole::Sysadmin],
        b"sysadmin-roster-session",
    )
    .await;

    let invitation_command = CreateCourseInvitation {
        course,
        email: AuthenticationEmail::parse("sysadmin-invited@example.edu").expect("valid email"),
        roster_id: CourseRosterId::parse("900123003").expect("valid roster ID"),
        token_hash: CourseInvitationSecretHash::compute(b"sysadmin-invitation"),
        idempotency_key: RosterIdempotencyKey::parse("sysadmin-invitation")
            .expect("valid idempotency key"),
        lifetime: CourseInvitationLifetime::from_seconds(86_400).expect("bounded lifetime"),
    };
    let invitation = store
        .create_course_invitation(context, sysadmin_session, invitation_command.clone())
        .await
        .expect("sysadmin may perform narrow roster support without course membership");
    assert_eq!(invitation.invited_by, sysadmin);
    assert_eq!(
        store
            .create_course_invitation(context, sysadmin_session, invitation_command)
            .await
            .expect("Sysadmin exact replay remains an audited successful response"),
        invitation
    );
    let roster = store
        .list_course_roster(
            context,
            sysadmin_session,
            course,
            PageRequest::first(PageSize::new(20).expect("page size")),
        )
        .await
        .expect("sysadmin may inspect the roster being supported");
    assert_eq!(roster.entries.items.len(), 1);
    let audits = store
        .roster_support_audits()
        .expect("roster support audit evidence");
    assert_eq!(
        audits.iter().map(|audit| audit.action).collect::<Vec<_>>(),
        vec![
            CourseRosterSupportAction::CreateInvitation,
            CourseRosterSupportAction::CreateInvitation,
            CourseRosterSupportAction::ListRoster,
        ]
    );
    assert!(
        audits
            .iter()
            .all(|audit| audit.actor == sysadmin && audit.course == course),
        "every support disclosure or change is actor/course bound"
    );

    let forged_actor_session = SessionTokenHash::compute(b"unknown-roster-session");
    assert_eq!(
        store
            .list_course_roster(
                context,
                forged_actor_session,
                course,
                PageRequest::first(PageSize::new(20).expect("page size")),
            )
            .await,
        Err(StoreError::NotFound),
        "an actor UUID without a persisted session has no roster authority"
    );
}

#[tokio::test]
async fn memory_roster_import_previews_then_commits_exactly_the_ready_rows() {
    let store = MemoryStore::default();
    store
        .set_authoritative_time(ActivityTimestamp::from_unix_millis(40_000))
        .expect("fixture clock");
    let tenant = TenantId::from_uuid(uuid(124_000));
    let context = TenantContext::from_authenticated_session(tenant);
    let instructor = UserId::from_uuid(uuid(124_001));
    let course = CourseId::from_uuid(uuid(124_002));
    let course_creation_authority =
        sysadmin_course_creation_authority(&store, tenant, course, instructor).await;
    store
        .create_course(
            context,
            CreateCourseCommand {
                course: CourseRecord {
                    id: course,
                    tenant,
                    title: "Bulk roster preview".to_string(),
                    term: question_model::CourseTerm::from_parts(
                        "2026-08-24",
                        "2026-12-18",
                        "America/Chicago",
                    )
                    .expect("explicit fixture course term"),
                },
                authority: course_creation_authority,
            },
        )
        .await
        .expect("course");
    let session = create_roster_session(
        &store,
        tenant,
        instructor,
        vec![UserRole::Instructor],
        b"bulk-roster-instructor",
    )
    .await;
    let shared_email =
        AuthenticationEmail::parse("duplicate@example.edu").expect("valid duplicate email");
    let rows = vec![
        CourseRosterImportRowInput {
            row_number: 2,
            email: Some(
                AuthenticationEmail::parse("ready@example.edu").expect("valid ready email"),
            ),
            roster_id: Some(CourseRosterId::parse("900124001").expect("valid ready roster ID")),
        },
        CourseRosterImportRowInput {
            row_number: 3,
            email: Some(shared_email.clone()),
            roster_id: Some(CourseRosterId::parse("900124002").expect("valid duplicate roster ID")),
        },
        CourseRosterImportRowInput {
            row_number: 4,
            email: Some(shared_email),
            roster_id: Some(CourseRosterId::parse("900124003").expect("valid duplicate roster ID")),
        },
        CourseRosterImportRowInput {
            row_number: 5,
            email: None,
            roster_id: None,
        },
    ];
    let preview = store
        .stage_course_roster_import(
            context,
            session,
            StageCourseRosterImport {
                course,
                expected_roster_revision: RosterRevision::INITIAL,
                normalized_digest: Sha256Digest::compute(b"normalized-roster-preview"),
                idempotency_key: RosterIdempotencyKey::parse("stage-roster-124")
                    .expect("valid stage key"),
                rows,
                lifetime: CourseRosterImportLifetime::from_seconds(3_600)
                    .expect("bounded preview lifetime"),
            },
        )
        .await
        .expect("preview should be staged");
    assert_eq!(
        preview
            .rows
            .iter()
            .map(|row| row.status)
            .collect::<Vec<_>>(),
        vec![
            RosterImportRowStatus::ReadyToInvite,
            RosterImportRowStatus::Duplicate,
            RosterImportRowStatus::Duplicate,
            RosterImportRowStatus::Invalid,
        ]
    );

    let commit = CommitCourseRosterImport {
        course,
        import: preview.id,
        expected_import_revision: preview.revision,
        idempotency_key: RosterIdempotencyKey::parse("commit-roster-124")
            .expect("valid commit key"),
        invitations: vec![RosterImportInvitation {
            row_number: 2,
            token_hash: CourseInvitationSecretHash::compute(b"bulk-ready-token"),
            idempotency_key: RosterIdempotencyKey::parse("bulk-ready-row-2")
                .expect("valid row key"),
            lifetime: CourseInvitationLifetime::from_seconds(86_400)
                .expect("bounded invitation lifetime"),
        }],
    };
    let committed = store
        .commit_course_roster_import(context, session, commit.clone())
        .await
        .expect("ready rows should commit atomically");
    assert_eq!(committed.invitations.len(), 1);
    assert_eq!(committed.invitations[0].0, 2);
    assert_eq!(committed.roster_revision.value(), 2);
    let delivery = store
        .course_invitation_delivery_state(context, session, course, committed.invitations[0].1.id)
        .await
        .expect("delivery lookup")
        .expect("one delivery per committed invitation");
    assert_eq!(delivery, CourseInvitationDeliveryState::Pending);
    assert_eq!(
        store
            .commit_course_roster_import(context, session, commit)
            .await
            .expect("same commit key is idempotent"),
        committed
    );
    let claimed = store
        .claim_due_course_invitation_deliveries(10, 60)
        .await
        .expect("pending delivery claim");
    assert_eq!(claimed.len(), 1);
    assert!(
        !store
            .complete_course_invitation_delivery(
                claimed[0].delivery.id,
                claimed[0].lease,
                CompleteCourseInvitationDelivery::Ambiguous,
            )
            .await
            .expect("unprepared lease completion is fenced")
    );
    let prepared = store
        .prepare_course_invitation_delivery(claimed[0].delivery.id, claimed[0].lease)
        .await
        .expect("prepared bulk delivery")
        .expect("current lease can prepare once");
    assert_eq!(
        prepared.expected_token_hash,
        CourseInvitationSecretHash::compute(b"bulk-ready-token")
    );
    assert!(matches!(
        prepared.reissuance,
        InvitationDeliveryReissuance::Import {
            import,
            row_number: 2,
            ref commit_idempotency_key,
            ..
        } if import == preview.id
            && commit_idempotency_key == &RosterIdempotencyKey::parse("commit-roster-124").expect("commit key")
    ));
    assert!(
        store
            .prepare_course_invitation_delivery(claimed[0].delivery.id, claimed[0].lease)
            .await
            .expect("second prepare query")
            .is_none(),
        "one lease cannot retrieve protected reissuance input twice"
    );
    assert!(
        store
            .revoke_course_invitation(
                context,
                session,
                RevokeCourseInvitation {
                    course,
                    invitation: committed.invitations[0].1.id,
                    expected_revision: committed.roster_revision,
                },
            )
            .await
            .is_ok(),
        "revocation after prepare persists the ambiguous delivery fence"
    );
    assert!(
        !store
            .complete_course_invitation_delivery(
                claimed[0].delivery.id,
                claimed[0].lease,
                CompleteCourseInvitationDelivery::AcceptedByProvider,
            )
            .await
            .expect("late completion after revocation is fenced")
    );
    assert_eq!(
        store
            .course_invitation_delivery_state(
                context,
                session,
                course,
                committed.invitations[0].1.id
            )
            .await
            .expect("revoked prepared delivery lookup"),
        Some(CourseInvitationDeliveryState::Ambiguous)
    );

    let expiry_invitation = store
        .create_course_invitation(
            context,
            session,
            CreateCourseInvitation {
                course,
                email: AuthenticationEmail::parse("expiry@example.edu").expect("expiry email"),
                roster_id: CourseRosterId::parse("900124099").expect("expiry roster ID"),
                token_hash: CourseInvitationSecretHash::compute(b"expiry-token"),
                idempotency_key: RosterIdempotencyKey::parse("expiry-delivery")
                    .expect("expiry key"),
                lifetime: CourseInvitationLifetime::from_seconds(1).expect("short lifetime"),
            },
        )
        .await
        .expect("expiry invitation");
    let expiry_claim = store
        .claim_due_course_invitation_deliveries(10, 60)
        .await
        .expect("expiry delivery claim")
        .pop()
        .expect("one expiry delivery");
    store
        .prepare_course_invitation_delivery(expiry_claim.delivery.id, expiry_claim.lease)
        .await
        .expect("expiry prepare")
        .expect("active lease prepares");
    store
        .set_authoritative_time(ActivityTimestamp::from_unix_millis(42_000))
        .expect("advance past invitation expiry");
    assert!(
        store
            .complete_course_invitation_delivery(
                expiry_claim.delivery.id,
                expiry_claim.lease,
                CompleteCourseInvitationDelivery::AcceptedByProvider,
            )
            .await
            .expect("late completion after expiry")
    );
    assert_eq!(
        store
            .course_invitation_delivery_state(context, session, course, expiry_invitation.id)
            .await
            .expect("expired prepared delivery lookup"),
        Some(CourseInvitationDeliveryState::Ambiguous)
    );

    let roster = store
        .list_course_roster(
            context,
            session,
            course,
            PageRequest::first(PageSize::new(20).expect("page size")),
        )
        .await
        .expect("committed roster should be readable");
    assert!(roster.entries.items.is_empty());
    assert_eq!(roster.policy.revision.value(), 4);
}

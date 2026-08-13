//! Passwordless-account and atomic course-roster behavior.

use super::*;

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
async fn memory_email_authentication_is_browser_bound_and_single_use() {
    let store = MemoryStore::default();
    store
        .set_authoritative_time(ActivityTimestamp::from_unix_millis(1_000))
        .expect("fixture clock");
    let email = AuthenticationEmail::parse("Student@mail.roosevelt.edu").expect("valid email");
    let token_hash = EmailChallengeSecretHash::compute(b"one-time-email-token");
    let browser_binding = BrowserBindingHash::compute(b"issuing-browser");
    store
        .begin_email_authentication(BeginEmailAuthentication {
            id: EmailChallengeId::from_uuid(uuid(120_001)),
            token_hash,
            browser_binding,
            email_rate_limit_key: AuthenticationRateLimitKey::compute(b"one-time-email-limit"),
            email: email.clone(),
            purpose: EmailAuthenticationPurpose::SignInOrRegister,
            lifetime: EmailChallengeLifetime::from_seconds(600).expect("bounded lifetime"),
        })
        .await
        .expect("challenge should persist");

    let command = CompleteEmailAuthentication {
        token_hash,
        browser_binding,
        proposed_user: UserId::from_uuid(uuid(120_002)),
        proposed_display_name: "Course Learner".to_string(),
    };
    assert_eq!(
        store
            .complete_email_authentication(CompleteEmailAuthentication {
                browser_binding: BrowserBindingHash::compute(b"forwarded-browser"),
                ..command.clone()
            })
            .await,
        Err(StoreError::NotFound),
        "a forwarded link must not authenticate a different browser"
    );
    let completed = store
        .complete_email_authentication(command.clone())
        .await
        .expect("issuing browser should complete authentication");
    assert!(completed.created);
    assert_eq!(completed.account.email, email);
    assert_eq!(
        store.complete_email_authentication(command).await,
        Err(StoreError::NotFound),
        "the token is consumed exactly once"
    );
}

#[tokio::test]
async fn memory_email_completion_and_account_session_are_atomic() {
    let store = MemoryStore::default();
    store
        .set_authoritative_time(ActivityTimestamp::from_unix_millis(2_000))
        .expect("fixture clock");
    let email = AuthenticationEmail::parse("atomic@example.edu").expect("valid email");
    let challenge_hash = EmailChallengeSecretHash::compute(b"atomic-email-token");
    let browser_binding = BrowserBindingHash::compute(b"atomic-browser");
    store
        .begin_email_authentication(BeginEmailAuthentication {
            id: EmailChallengeId::from_uuid(uuid(120_010)),
            token_hash: challenge_hash,
            browser_binding,
            email_rate_limit_key: AuthenticationRateLimitKey::compute(b"atomic-email-limit"),
            email: email.clone(),
            purpose: EmailAuthenticationPurpose::SignInOrRegister,
            lifetime: EmailChallengeLifetime::from_seconds(600).expect("bounded lifetime"),
        })
        .await
        .expect("challenge should persist");

    let user = UserId::from_uuid(uuid(120_011));
    let session_hash = AccountSessionTokenHash::compute(b"atomic-account-session");
    let completed = store
        .complete_email_authentication_and_create_session(
            CompleteEmailAuthenticationAndCreateSession {
                authentication: CompleteEmailAuthentication {
                    token_hash: challenge_hash,
                    browser_binding,
                    proposed_user: user,
                    proposed_display_name: "Atomic Learner".to_string(),
                },
                session_token_hash: session_hash,
                session_lifetime: AccountSessionLifetime::from_seconds(900)
                    .expect("bounded lifetime"),
            },
        )
        .await
        .expect("challenge and account session should commit together");
    assert_eq!(completed.authentication.account.email, email);
    assert_eq!(completed.session.user, user);
    assert_eq!(
        store
            .resolve_account_session(session_hash)
            .await
            .expect("account session lookup")
            .expect("active account session")
            .user,
        user
    );
    assert_eq!(
        store
            .complete_email_authentication(CompleteEmailAuthentication {
                token_hash: challenge_hash,
                browser_binding,
                proposed_user: user,
                proposed_display_name: "Atomic Learner".to_string(),
            })
            .await,
        Err(StoreError::NotFound),
        "successful account proof issuance consumes the challenge"
    );
}

#[tokio::test]
async fn memory_account_course_context_derives_tenant_and_role_from_membership() {
    let store = MemoryStore::default();
    let user = UserId::from_uuid(uuid(120_030));
    let tenant_a = TenantId::from_uuid(uuid(120_031));
    let tenant_b = TenantId::from_uuid(uuid(120_032));
    let course_a = CourseId::from_uuid(uuid(120_033));
    let course_b = CourseId::from_uuid(uuid(120_034));
    for (tenant, course, title, role) in [
        (
            tenant_a,
            course_a,
            "Biochemistry",
            CourseMembershipRole::Instructor,
        ),
        (
            tenant_b,
            course_b,
            "Genetics",
            CourseMembershipRole::Student,
        ),
    ] {
        let mut members = vec![CourseMembership { user, role }];
        if role == CourseMembershipRole::Student {
            members.push(CourseMembership {
                user: UserId::from_uuid(uuid(120_036)),
                role: CourseMembershipRole::Instructor,
            });
        }
        store
            .upsert_course(
                TenantContext::from_authenticated_session(tenant),
                CourseRecord {
                    id: course,
                    tenant,
                    title: title.to_string(),
                    members,
                },
            )
            .await
            .expect("course context fixture");
    }

    let first = store
        .list_account_course_contexts(
            user,
            PageRequest::first(PageSize::new(1).expect("page size")),
        )
        .await
        .expect("first account course page");
    assert_eq!(first.items.len(), 1);
    let second = store
        .list_account_course_contexts(
            user,
            PageRequest::after(
                first.next_cursor.expect("continuation cursor"),
                PageSize::new(1).expect("page size"),
            ),
        )
        .await
        .expect("second account course page");
    assert_eq!(second.items.len(), 1);
    assert_eq!(
        store
            .resolve_account_course_context(user, course_b)
            .await
            .expect("course context lookup")
            .expect("student context"),
        learning_data_access::AccountCourseContext {
            tenant: tenant_b,
            course: course_b,
            title: "Genetics".to_string(),
            role: CourseMembershipRole::Student,
        }
    );
    assert_eq!(
        store
            .resolve_account_course_context(UserId::from_uuid(uuid(120_035)), course_b)
            .await
            .expect("nonmember lookup"),
        None
    );
}

#[tokio::test]
async fn memory_discoverable_passkey_lookup_never_requires_browser_user_identity() {
    let store = MemoryStore::default();
    store
        .set_authoritative_time(ActivityTimestamp::from_unix_millis(3_000))
        .expect("fixture clock");
    let user = UserId::from_uuid(uuid(120_020));
    let challenge_hash = EmailChallengeSecretHash::compute(b"passkey-account-token");
    let browser_binding = BrowserBindingHash::compute(b"passkey-account-browser");
    store
        .begin_email_authentication(BeginEmailAuthentication {
            id: EmailChallengeId::from_uuid(uuid(120_021)),
            token_hash: challenge_hash,
            browser_binding,
            email_rate_limit_key: AuthenticationRateLimitKey::compute(b"passkey-account-limit"),
            email: AuthenticationEmail::parse("passkey@example.edu").expect("valid email"),
            purpose: EmailAuthenticationPurpose::SignInOrRegister,
            lifetime: EmailChallengeLifetime::from_seconds(600).expect("bounded lifetime"),
        })
        .await
        .expect("challenge");
    store
        .complete_email_authentication(CompleteEmailAuthentication {
            token_hash: challenge_hash,
            browser_binding,
            proposed_user: user,
            proposed_display_name: "Passkey Learner".to_string(),
        })
        .await
        .expect("account");

    let passkey = store
        .insert_passkey(RegisterPasskey {
            id: PasskeyId::from_uuid(uuid(120_022)),
            user,
            credential_id_hash: CredentialIdHash::compute(b"discoverable-credential"),
            label: validated_passkey_label("Laptop").expect("label"),
            credential: WebauthnState::new(br#"{"credential":"state"}"#.to_vec())
                .expect("credential state"),
        })
        .await
        .expect("passkey");
    assert_eq!(
        store
            .get_active_passkey_by_credential_id_hash(passkey.credential_id_hash)
            .await
            .expect("credential lookup"),
        Some(passkey.clone())
    );

    let completed = store
        .complete_passkey_authentication_and_create_session(
            CompletePasskeyAuthenticationAndCreateSession {
                passkey: PasskeyRecord {
                    credential: WebauthnState::new(br#"{"credential":"updated"}"#.to_vec())
                        .expect("updated credential state"),
                    ..passkey.clone()
                },
                session_token_hash: AccountSessionTokenHash::compute(b"passkey-account-session"),
                session_lifetime: AccountSessionLifetime::from_seconds(900)
                    .expect("bounded session lifetime"),
            },
        )
        .await
        .expect("credential update and account proof commit together");
    assert_eq!(completed.session.user, user);
    assert_eq!(
        store
            .resolve_account_session(AccountSessionTokenHash::compute(b"passkey-account-session"))
            .await
            .expect("account session")
            .expect("active account session")
            .user,
        user
    );
    assert_eq!(
        completed.passkey.last_used_at,
        Some(ActivityTimestamp::from_unix_millis(3_000))
    );

    store
        .revoke_passkey(user, passkey.id)
        .await
        .expect("revoke passkey");
    assert_eq!(
        store
            .get_active_passkey_by_credential_id_hash(passkey.credential_id_hash)
            .await
            .expect("revoked credential lookup"),
        None,
        "revoked and unknown credentials share the same lookup result"
    );
}

#[tokio::test]
async fn memory_authentication_rate_limit_is_atomic_and_window_bounded() {
    let store = MemoryStore::default();
    store
        .set_authoritative_time(ActivityTimestamp::from_unix_millis(4_000))
        .expect("fixture clock");
    let command = ConsumeAuthenticationRateLimit {
        scope: AuthenticationRateLimitScope::Email,
        key: AuthenticationRateLimitKey::compute(b"server-keyed-email"),
        policy: AuthenticationRateLimitPolicy::new(2, 60).expect("bounded policy"),
    };
    assert_eq!(
        store
            .consume_authentication_rate_limit(command)
            .await
            .expect("first allowance"),
        AuthenticationRateLimitDecision::Allowed {
            remaining_attempts: 1
        }
    );
    assert_eq!(
        store
            .consume_authentication_rate_limit(command)
            .await
            .expect("second allowance"),
        AuthenticationRateLimitDecision::Allowed {
            remaining_attempts: 0
        }
    );
    assert_eq!(
        store
            .consume_authentication_rate_limit(command)
            .await
            .expect("denied allowance"),
        AuthenticationRateLimitDecision::Denied {
            retry_after_seconds: 60
        }
    );

    store
        .set_authoritative_time(ActivityTimestamp::from_unix_millis(64_000))
        .expect("advance fixture clock");
    assert_eq!(
        store
            .consume_authentication_rate_limit(command)
            .await
            .expect("new window"),
        AuthenticationRateLimitDecision::Allowed {
            remaining_attempts: 1
        }
    );
}

#[tokio::test]
async fn verified_mailbox_completion_releases_only_its_email_quota() {
    let store = MemoryStore::default();
    store
        .set_authoritative_time(ActivityTimestamp::from_unix_millis(5_000))
        .expect("fixture clock");
    let email_key = AuthenticationRateLimitKey::compute(b"verified-mailbox-limit");
    let network_key = AuthenticationRateLimitKey::compute(b"shared-campus-network-limit");
    let policy = AuthenticationRateLimitPolicy::new(1, 900).expect("bounded policy");
    for (scope, key) in [
        (AuthenticationRateLimitScope::Email, email_key),
        (AuthenticationRateLimitScope::Network, network_key),
    ] {
        store
            .consume_authentication_rate_limit(ConsumeAuthenticationRateLimit {
                scope,
                key,
                policy,
            })
            .await
            .expect("first allowance");
    }
    let token_hash = EmailChallengeSecretHash::compute(b"verified-mailbox-token");
    let browser_binding = BrowserBindingHash::compute(b"verified-mailbox-browser");
    store
        .begin_email_authentication(BeginEmailAuthentication {
            id: EmailChallengeId::from_uuid(uuid(120_030)),
            token_hash,
            browser_binding,
            email_rate_limit_key: email_key,
            email: AuthenticationEmail::parse("verified@example.edu").expect("valid email"),
            purpose: EmailAuthenticationPurpose::SignInOrRegister,
            lifetime: EmailChallengeLifetime::from_seconds(600).expect("bounded lifetime"),
        })
        .await
        .expect("challenge should persist");
    store
        .complete_email_authentication(CompleteEmailAuthentication {
            token_hash,
            browser_binding,
            proposed_user: UserId::from_uuid(uuid(120_031)),
            proposed_display_name: "Verified Learner".to_string(),
        })
        .await
        .expect("mailbox proof should complete");
    assert!(matches!(
        store
            .consume_authentication_rate_limit(ConsumeAuthenticationRateLimit {
                scope: AuthenticationRateLimitScope::Email,
                key: email_key,
                policy,
            })
            .await,
        Ok(AuthenticationRateLimitDecision::Allowed { .. })
    ));
    assert!(matches!(
        store
            .consume_authentication_rate_limit(ConsumeAuthenticationRateLimit {
                scope: AuthenticationRateLimitScope::Network,
                key: network_key,
                policy,
            })
            .await,
        Ok(AuthenticationRateLimitDecision::Denied { .. })
    ));
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
        .upsert_course(
            context,
            CourseRecord {
                id: course,
                tenant,
                title: "Roster cross-product".to_string(),
                members: vec![CourseMembership {
                    user: instructor,
                    role: CourseMembershipRole::Instructor,
                }],
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
        .upsert_course(
            context,
            CourseRecord {
                id: course,
                tenant,
                title: "Pending invitation identity".to_string(),
                members: vec![CourseMembership {
                    user: instructor,
                    role: CourseMembershipRole::Instructor,
                }],
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
    store
        .upsert_course(
            context,
            CourseRecord {
                id: course,
                tenant,
                title: "Sysadmin roster authority".to_string(),
                members: vec![CourseMembership {
                    user: instructor,
                    role: CourseMembershipRole::Instructor,
                }],
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

    let invitation = store
        .create_course_invitation(
            context,
            sysadmin_session,
            CreateCourseInvitation {
                course,
                email: AuthenticationEmail::parse("sysadmin-invited@example.edu")
                    .expect("valid email"),
                roster_id: CourseRosterId::parse("900123003").expect("valid roster ID"),
                token_hash: CourseInvitationSecretHash::compute(b"sysadmin-invitation"),
                idempotency_key: RosterIdempotencyKey::parse("sysadmin-invitation")
                    .expect("valid idempotency key"),
                lifetime: CourseInvitationLifetime::from_seconds(86_400).expect("bounded lifetime"),
            },
        )
        .await
        .expect("sysadmin may perform narrow roster support without course membership");
    assert_eq!(invitation.invited_by, sysadmin);
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
    store
        .upsert_course(
            context,
            CourseRecord {
                id: course,
                tenant,
                title: "Bulk roster preview".to_string(),
                members: vec![CourseMembership {
                    user: instructor,
                    role: CourseMembershipRole::Instructor,
                }],
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
    assert_eq!(
        store
            .commit_course_roster_import(context, session, commit)
            .await
            .expect("same commit key is idempotent"),
        committed
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
    assert_eq!(roster.entries.items.len(), 1);
    assert_eq!(roster.policy.revision, committed.roster_revision);
}

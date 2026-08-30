//! Account authentication and account-session conformance behavior.

use super::*;

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
async fn memory_email_change_replaces_the_completing_proof_and_revokes_all_prior_sessions() {
    let store = MemoryStore::default();
    store
        .set_authoritative_time(ActivityTimestamp::from_unix_millis(3_000))
        .expect("fixture clock");
    let user = UserId::from_uuid(uuid(120_020));
    let original_email = AuthenticationEmail::parse("replace-old@example.edu").expect("email");
    let original_token = EmailChallengeSecretHash::compute(b"replace-original-token");
    let original_binding = BrowserBindingHash::compute(b"replace-original-binding");
    store
        .begin_email_authentication(BeginEmailAuthentication {
            id: EmailChallengeId::from_uuid(uuid(120_021)),
            token_hash: original_token,
            browser_binding: original_binding,
            email_rate_limit_key: AuthenticationRateLimitKey::compute(b"replace-original-limit"),
            email: original_email,
            purpose: EmailAuthenticationPurpose::SignInOrRegister,
            lifetime: EmailChallengeLifetime::from_seconds(600).expect("lifetime"),
        })
        .await
        .expect("original account challenge");
    store
        .complete_email_authentication(CompleteEmailAuthentication {
            token_hash: original_token,
            browser_binding: original_binding,
            proposed_user: user,
            proposed_display_name: "Replacement Learner".to_string(),
        })
        .await
        .expect("original account");

    let stale_account = AccountSessionTokenHash::compute(b"replace-stale-account");
    let other_account = AccountSessionTokenHash::compute(b"replace-other-account");
    let stale_tenant = SessionTokenHash::compute(b"replace-stale-tenant");
    let replacement_account = AccountSessionTokenHash::compute(b"replace-fresh-account");
    let account_lifetime = AccountSessionLifetime::from_seconds(900).expect("lifetime");
    store
        .create_account_session(stale_account, user, account_lifetime)
        .await
        .expect("stale account proof");
    store
        .create_account_session(other_account, user, account_lifetime)
        .await
        .expect("second account proof");
    store
        .create_session(
            stale_tenant,
            SessionSubject::new(
                TenantId::from_uuid(uuid(120_022)),
                user,
                "Replacement Learner",
                vec![UserRole::Student],
            )
            .expect("tenant subject"),
            SessionLifetime::from_seconds(3_600).expect("lifetime"),
        )
        .await
        .expect("tenant proof");

    let replacement_token = EmailChallengeSecretHash::compute(b"replace-email-token");
    let replacement_binding = BrowserBindingHash::compute(b"replace-email-binding");
    store
        .begin_email_authentication(BeginEmailAuthentication {
            id: EmailChallengeId::from_uuid(uuid(120_023)),
            token_hash: replacement_token,
            browser_binding: replacement_binding,
            email_rate_limit_key: AuthenticationRateLimitKey::compute(b"replace-email-limit"),
            email: AuthenticationEmail::parse("replace-new@example.edu").expect("email"),
            purpose: EmailAuthenticationPurpose::ChangeEmail { user },
            lifetime: EmailChallengeLifetime::from_seconds(600).expect("lifetime"),
        })
        .await
        .expect("replacement challenge");
    let completed = store
        .complete_email_change_and_revoke_user_sessions(CompleteEmailChangeAndRevokeUserSessions {
            authentication: CompleteEmailAuthentication {
                token_hash: replacement_token,
                browser_binding: replacement_binding,
                proposed_user: user,
                proposed_display_name: "Replacement Learner".to_string(),
            },
            session_token_hash: replacement_account,
            session_lifetime: account_lifetime,
        })
        .await
        .expect("atomic replacement");

    assert_eq!(completed.session.token_hash, replacement_account);
    let next_replica = store.clone();
    assert_eq!(
        next_replica.resolve_account_session(stale_account).await,
        Ok(None)
    );
    assert_eq!(
        next_replica.resolve_account_session(other_account).await,
        Ok(None)
    );
    assert_eq!(next_replica.resolve_session(stale_tenant).await, Ok(None));
    assert_eq!(
        store.resolve_account_session(replacement_account).await,
        Ok(Some(completed.session)),
        "the completing browser receives the only replacement account proof"
    );
}

#[tokio::test]
async fn memory_rejected_email_change_preserves_existing_sessions() {
    let store = MemoryStore::default();
    store
        .set_authoritative_time(ActivityTimestamp::from_unix_millis(4_000))
        .expect("fixture clock");
    let user = UserId::from_uuid(uuid(120_030));
    let other_user = UserId::from_uuid(uuid(120_031));
    for (candidate, email, token, binding) in [
        (
            user,
            "preserve-owner@example.edu",
            b"preserve-owner-token".as_slice(),
            b"preserve-owner-binding".as_slice(),
        ),
        (
            other_user,
            "preserve-conflict@example.edu",
            b"preserve-conflict-token".as_slice(),
            b"preserve-conflict-binding".as_slice(),
        ),
    ] {
        let token_hash = EmailChallengeSecretHash::compute(token);
        let browser_binding = BrowserBindingHash::compute(binding);
        store
            .begin_email_authentication(BeginEmailAuthentication {
                id: EmailChallengeId::from_uuid(uuid(if candidate == user {
                    120_032
                } else {
                    120_033
                })),
                token_hash,
                browser_binding,
                email_rate_limit_key: AuthenticationRateLimitKey::compute(token),
                email: AuthenticationEmail::parse(email).expect("email"),
                purpose: EmailAuthenticationPurpose::SignInOrRegister,
                lifetime: EmailChallengeLifetime::from_seconds(600).expect("lifetime"),
            })
            .await
            .expect("account challenge");
        store
            .complete_email_authentication(CompleteEmailAuthentication {
                token_hash,
                browser_binding,
                proposed_user: candidate,
                proposed_display_name: "Preserved Learner".to_string(),
            })
            .await
            .expect("account");
    }
    let stale_account = AccountSessionTokenHash::compute(b"preserve-account");
    let stale_tenant = SessionTokenHash::compute(b"preserve-tenant");
    store
        .create_account_session(
            stale_account,
            user,
            AccountSessionLifetime::from_seconds(900).expect("lifetime"),
        )
        .await
        .expect("account proof");
    store
        .create_session(
            stale_tenant,
            SessionSubject::new(
                TenantId::from_uuid(uuid(120_034)),
                user,
                "Preserved Learner",
                vec![UserRole::Student],
            )
            .expect("tenant subject"),
            SessionLifetime::from_seconds(3_600).expect("lifetime"),
        )
        .await
        .expect("tenant proof");

    let change_token = EmailChallengeSecretHash::compute(b"preserve-change-token");
    let change_binding = BrowserBindingHash::compute(b"preserve-change-binding");
    store
        .begin_email_authentication(BeginEmailAuthentication {
            id: EmailChallengeId::from_uuid(uuid(120_035)),
            token_hash: change_token,
            browser_binding: change_binding,
            email_rate_limit_key: AuthenticationRateLimitKey::compute(b"preserve-change-limit"),
            email: AuthenticationEmail::parse("preserve-conflict@example.edu").expect("email"),
            purpose: EmailAuthenticationPurpose::ChangeEmail { user },
            lifetime: EmailChallengeLifetime::from_seconds(600).expect("lifetime"),
        })
        .await
        .expect("change challenge");
    let rejected = CompleteEmailChangeAndRevokeUserSessions {
        authentication: CompleteEmailAuthentication {
            token_hash: change_token,
            browser_binding: BrowserBindingHash::compute(b"wrong-browser"),
            proposed_user: user,
            proposed_display_name: "Preserved Learner".to_string(),
        },
        session_token_hash: AccountSessionTokenHash::compute(b"preserve-replacement"),
        session_lifetime: AccountSessionLifetime::from_seconds(900).expect("lifetime"),
    };
    assert_eq!(
        store
            .complete_email_change_and_revoke_user_sessions(rejected)
            .await,
        Err(StoreError::NotFound)
    );
    assert!(
        store
            .resolve_account_session(stale_account)
            .await
            .unwrap()
            .is_some()
            && store.resolve_session(stale_tenant).await.unwrap().is_some(),
        "wrong-browser rejection must not revoke prior proofs"
    );

    let completed = store
        .complete_email_change_and_revoke_user_sessions(CompleteEmailChangeAndRevokeUserSessions {
            authentication: CompleteEmailAuthentication {
                token_hash: change_token,
                browser_binding: change_binding,
                proposed_user: user,
                proposed_display_name: "Preserved Learner".to_string(),
            },
            session_token_hash: AccountSessionTokenHash::compute(b"preserve-conflict-new"),
            session_lifetime: AccountSessionLifetime::from_seconds(900).expect("lifetime"),
        })
        .await;
    assert_eq!(completed, Err(StoreError::Conflict));
    assert!(
        store
            .resolve_account_session(stale_account)
            .await
            .unwrap()
            .is_some()
            && store.resolve_session(stale_tenant).await.unwrap().is_some(),
        "email-conflict rejection must roll back without revoking prior proofs"
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
        let course_instructor = if role == CourseMembershipRole::Instructor {
            user
        } else {
            UserId::from_uuid(uuid(120_036))
        };
        let course_creation_authority =
            sysadmin_course_creation_authority(&store, tenant, course, course_instructor).await;
        store
            .create_course(
                TenantContext::from_authenticated_session(tenant),
                CreateCourseCommand {
                    course: CourseRecord {
                        id: course,
                        tenant,
                        title: title.to_string(),
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
            .expect("course context fixture");
        if role == CourseMembershipRole::Student {
            store
                .upsert_course_member(
                    TenantContext::from_authenticated_session(tenant),
                    course_instructor,
                    UpsertCourseMember {
                        course,
                        user,
                        display_name: "Account learner".to_string(),
                        roster_contact: None,
                    },
                )
                .await
                .expect("student course context membership");
        }
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

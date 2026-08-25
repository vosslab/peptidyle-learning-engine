use super::*;

#[tokio::test]
#[ignore = "requires the disposable PostgreSQL 17 database baseline"]
async fn postgres_sysadmin_candidate_discovery_is_brokered_paged_and_safe() {
    let runtime = load_acceptance_runtime();
    let url = runtime.admin_url().expose();
    let pool = lazy_pool(url).expect("disposable PostgreSQL URL");
    verify_application_schema(&pool)
        .await
        .expect("migrated schema");
    let store = PostgresStore::with_question_id_secret(pool.clone(), [0x56; 32]);
    let tenant = TenantId::from_uuid(id());
    let context = TenantContext::from_authenticated_session(tenant);
    let sysadmin = UserId::from_uuid(id());
    let ordinary = UserId::from_uuid(id());
    let alpha = UserId::from_uuid(id());
    let alpine = UserId::from_uuid(id());
    let candidate_marker = alpha.as_uuid().simple().to_string();
    let candidate_prefix = format!("Candidate {}", &candidate_marker[..12]);
    create_account_named(&store, sysadmin, "Sysadmin").await;
    create_account_named(&store, ordinary, "Ordinary").await;
    create_account_named(&store, alpha, &format!("{candidate_prefix} Alpha")).await;
    create_account_named(&store, alpine, &format!("{candidate_prefix} Alpine")).await;
    let sysadmin_session = session(&store, tenant, sysadmin, vec![UserRole::Sysadmin]).await;
    let ordinary_session = session(&store, tenant, ordinary, vec![UserRole::Student]).await;
    let request = question_model::SysadminInstructorCandidateSearchRequest {
        query: candidate_prefix.clone().try_into().expect("bounded query"),
        after: None,
        size: question_model::TeachingPageSize::try_from(1).expect("bounded page"),
    };
    assert_eq!(
        store
            .search_sysadmin_instructor_candidates(context, ordinary_session, request.clone())
            .await,
        Err(StoreError::Forbidden),
        "the broker requires the persisted Sysadmin role"
    );
    let first = store
        .search_sysadmin_instructor_candidates(context, sysadmin_session, request)
        .await
        .expect("first candidate page");
    assert_eq!(first.candidates.len(), 1);
    assert!(first.next_cursor.is_some(), "one-row page has continuation");
    let first_candidate = &first.candidates[0];
    assert_eq!(
        first_candidate.approval.state,
        question_model::SysadminInstructorApprovalStateView::Unapproved
    );
    assert!(first_candidate.approval.revision.is_none());
    let serialized = serde_json::to_string(&first).expect("safe serializes");
    assert!(!serialized.contains('@'));
    assert!(!serialized.contains("userId"));
    let target = store
        .resolve_account_reference_for_operator(
            context,
            sysadmin_session,
            first_candidate.account.reference,
        )
        .await
        .expect("sysadmin resolves opaque reference")
        .expect("candidate exists");
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
        .expect("normal approval");
    let refreshed = store
        .search_sysadmin_instructor_candidates(
            context,
            sysadmin_session,
            question_model::SysadminInstructorCandidateSearchRequest {
                query: candidate_prefix.try_into().expect("bounded query"),
                after: None,
                size: question_model::TeachingPageSize::try_from(100).expect("bounded page"),
            },
        )
        .await
        .expect("refreshed candidates");
    let approved = refreshed
        .candidates
        .iter()
        .find(|candidate| candidate.account.reference == first_candidate.account.reference)
        .expect("approved candidate remains discoverable");
    assert_eq!(
        approved.approval.state,
        question_model::SysadminInstructorApprovalStateView::Approved
    );
    assert_eq!(
        approved.approval.revision.map(|value| value.value()),
        Some(approval.revision.as_i64() as u64)
    );
    let catalog = sqlx::query(
        "SELECT p.prosecdef, p.proconfig, r.rolname, \
         has_function_privilege('public', p.oid, 'EXECUTE') AS public_execute, \
         has_function_privilege('ple_app', p.oid, 'EXECUTE') AS app_execute \
         FROM pg_proc p JOIN pg_roles r ON r.oid = p.proowner \
         WHERE p.proname = 'ple_sysadmin_instructor_candidate_search'",
    )
    .fetch_one(&pool)
    .await
    .expect("candidate broker catalog");
    assert!(catalog.try_get::<bool, _>("prosecdef").expect("definer"));
    assert_eq!(
        catalog.try_get::<String, _>("rolname").expect("owner"),
        "ple_teaching_authority_broker"
    );
    assert!(
        !catalog
            .try_get::<bool, _>("public_execute")
            .expect("public revoke")
    );
    assert!(
        catalog
            .try_get::<bool, _>("app_execute")
            .expect("app grant")
    );
    assert!(
        catalog
            .try_get::<Option<Vec<String>>, _>("proconfig")
            .expect("fixed search path")
            .unwrap_or_default()
            .iter()
            .any(|value| value == "search_path=pg_catalog, public"
                || value == "search_path=pg_catalog,public")
    );
}

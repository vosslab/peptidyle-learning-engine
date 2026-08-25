use learning_data_access::{
    PageRequest, PageSize, ProblemCollectionReplacementTarget, ProblemCurationStore,
    ReplaceProblemCollectionCommand, ReplaceSavedProblemSearchCommand, SessionTokenHash,
    StoreError,
};
use question_model::{
    CatalogSearchFilter, CatalogSearchQuery, ProblemCollectionSelectionAvailability,
    ProblemCollectionVisibility,
};
use sqlx::{PgPool, Postgres, Transaction};

use crate::fixture::Fixture;

pub(super) async fn favorites_replacement_and_retention_are_atomic(fixture: &Fixture) {
    let (first_capability, second_capability) = tokio::join!(
        ensure_favorites_through_broker(
            &fixture.pool,
            fixture.tenant.as_uuid(),
            fixture.elena_session.to_string(),
        ),
        ensure_favorites_through_broker(
            &fixture.pool,
            fixture.tenant.as_uuid(),
            fixture.elena_session.to_string(),
        ),
    );
    let first_reference = first_capability.expect("first concurrent Favorites capability");
    let second_reference = second_capability.expect("second concurrent Favorites capability");
    assert_eq!(
        first_reference, second_reference,
        "the broker materializes one Favorites collection under concurrency"
    );

    let (first, second) = tokio::join!(
        fixture
            .store
            .get_or_create_favorites(fixture.context, fixture.elena_session),
        fixture
            .store
            .get_or_create_favorites(fixture.context, fixture.elena_session),
    );
    let first = first.expect("first Favorites request");
    let second = second.expect("concurrent Favorites request");
    assert_eq!(
        first, second,
        "Favorites materializes once under concurrent requests"
    );
    let favorites = fixture
        .store
        .replace_problem_collection(
            fixture.context,
            fixture.elena_session,
            ReplaceProblemCollectionCommand {
                target: ProblemCollectionReplacementTarget::Favorites,
                expected_revision: Some(first.revision),
                title: None,
                visibility: None,
                question_ids: fixture.public_questions.clone(),
            },
        )
        .await
        .expect("Favorites complete replacement");
    let no_op = fixture
        .store
        .replace_problem_collection(
            fixture.context,
            fixture.elena_session,
            ReplaceProblemCollectionCommand {
                target: ProblemCollectionReplacementTarget::Favorites,
                expected_revision: Some(favorites.revision),
                title: None,
                visibility: None,
                question_ids: fixture.public_questions.clone(),
            },
        )
        .await
        .expect("Favorites no-op replacement");
    assert_eq!(
        no_op.revision, favorites.revision,
        "no-op preserves strong revision"
    );
    let members = fixture
        .store
        .list_problem_collection_members(
            fixture.context,
            fixture.elena_session,
            favorites.reference,
            PageRequest::first(PageSize::new(100).expect("page")),
        )
        .await
        .expect("Favorites members")
        .expect("Favorites exists");
    assert_eq!(
        members
            .members
            .items
            .iter()
            .map(|member| member.question_id.clone())
            .collect::<Vec<_>>(),
        fixture.public_questions
    );

    let unavailable = question_model::QuestionId::from_canonical_parts("ZZZZZZ", 'Z')
        .expect("well-formed unavailable question ID");
    assert!(matches!(
        fixture
            .store
            .replace_problem_collection(
                fixture.context,
                fixture.elena_session,
                ReplaceProblemCollectionCommand {
                    target: ProblemCollectionReplacementTarget::Favorites,
                    expected_revision: Some(favorites.revision),
                    title: None,
                    visibility: None,
                    question_ids: vec![unavailable],
                },
            )
            .await,
        Err(StoreError::InvalidRecord(_))
    ));
    let unchanged = fixture
        .store
        .list_problem_collection_members(
            fixture.context,
            fixture.elena_session,
            favorites.reference,
            PageRequest::first(PageSize::new(100).expect("page")),
        )
        .await
        .expect("post-rollback members")
        .expect("Favorites still exists");
    assert_eq!(
        unchanged.members.items.len(),
        2,
        "unresolvable whole replacement rolls back every member"
    );
    let too_many = std::iter::repeat_n(fixture.public_questions[0].clone(), 201).collect();
    assert!(matches!(
        fixture
            .store
            .replace_problem_collection(
                fixture.context,
                fixture.elena_session,
                ReplaceProblemCollectionCommand {
                    target: ProblemCollectionReplacementTarget::NewNamed,
                    expected_revision: None,
                    title: Some("Too many".into()),
                    visibility: Some(ProblemCollectionVisibility::Private),
                    question_ids: too_many,
                },
            )
            .await,
        Err(StoreError::InvalidRecord(_))
    ));

    let retained = fixture
        .store
        .replace_problem_collection(
            fixture.context,
            fixture.elena_session,
            ReplaceProblemCollectionCommand {
                target: ProblemCollectionReplacementTarget::NewNamed,
                expected_revision: None,
                title: Some("Institution provenance".into()),
                visibility: Some(ProblemCollectionVisibility::Private),
                question_ids: vec![fixture.institution_question.clone()],
            },
        )
        .await
        .expect("retain exact institution publication");
    sqlx::query(
        "DELETE FROM catalog_tenant_grant WHERE tenant_id=$1 AND problem_id=$2 AND version_id=$3",
    )
    .bind(fixture.tenant.as_uuid())
    .bind(fixture.institution_reference.problem.as_uuid())
    .bind(fixture.institution_reference.version.as_uuid())
    .execute(&fixture.pool)
    .await
    .expect("withdraw current institution grant");
    let after_withdrawal = fixture
        .store
        .list_problem_collection_members(
            fixture.context,
            fixture.elena_session,
            retained.reference,
            PageRequest::first(PageSize::new(100).expect("page")),
        )
        .await
        .expect("retained read survives grant withdrawal")
        .expect("retained collection");
    assert_eq!(after_withdrawal.members.items.len(), 1);
    sqlx::query("UPDATE catalog_search_document SET lifecycle='archived', lifecycle_reason='D2 retained proof' WHERE problem_id=$1 AND version_id=$2")
        .bind(fixture.institution_reference.problem.as_uuid()).bind(fixture.institution_reference.version.as_uuid())
        .execute(&fixture.pool).await.expect("archive the current catalog projection");
    let retained_page = fixture
        .store
        .list_problem_collection_members(
            fixture.context,
            fixture.elena_session,
            retained.reference,
            PageRequest::first(PageSize::new(100).expect("page")),
        )
        .await
        .expect("retained exact member")
        .expect("retained collection");
    assert_eq!(
        retained_page.members.items[0].selection_availability,
        ProblemCollectionSelectionAvailability::Retained
    );
    assert!(
        matches!(
            fixture
                .store
                .replace_problem_collection(
                    fixture.context,
                    fixture.elena_session,
                    ReplaceProblemCollectionCommand {
                        target: ProblemCollectionReplacementTarget::Existing(retained.reference),
                        expected_revision: Some(retained.revision),
                        title: Some(retained.title.clone()),
                        visibility: Some(ProblemCollectionVisibility::Private),
                        question_ids: vec![fixture.institution_question.clone()],
                    },
                )
                .await,
            Err(StoreError::InvalidRecord(_))
        ),
        "new selection re-resolves current catalog authority"
    );
    assert!(
        fixture
            .store
            .delete_problem_collection(
                fixture.context,
                fixture.elena_session,
                retained.reference,
                retained.revision,
            )
            .await
            .expect("named collection deletion")
    );
    assert_eq!(
        fixture
            .store
            .get_problem_collection_summary(
                fixture.context,
                fixture.elena_session,
                retained.reference,
            )
            .await
            .expect("deleted summary"),
        None
    );
}

async fn ensure_favorites_through_broker(
    pool: &PgPool,
    tenant_id: uuid::Uuid,
    session_hash: String,
) -> Result<i32, sqlx::Error> {
    let mut transaction = pool.begin().await?;
    sqlx::query("SET LOCAL ROLE ple_app")
        .execute(&mut *transaction)
        .await?;
    sqlx::query("SELECT set_config('ple.tenant_id', $1, true)")
        .bind(tenant_id.to_string())
        .execute(&mut *transaction)
        .await?;
    sqlx::query("SELECT set_config('ple.session_hash', $1, true)")
        .bind(&session_hash)
        .execute(&mut *transaction)
        .await?;
    let reference = sqlx::query_scalar(
        "SELECT collection_reference FROM public.ple_ensure_problem_favorites_v1($1, $2)",
    )
    .bind(tenant_id)
    .bind(session_hash)
    .fetch_one(&mut *transaction)
    .await?;
    transaction.commit().await?;
    Ok(reference)
}

async fn begin_app_transaction(
    fixture: &Fixture,
    session: SessionTokenHash,
) -> Transaction<'_, Postgres> {
    let mut transaction = fixture
        .pool
        .begin()
        .await
        .expect("direct broker transaction");
    sqlx::query("SET LOCAL ROLE ple_app")
        .execute(&mut *transaction)
        .await
        .expect("application capability role");
    sqlx::query("SELECT set_config('ple.tenant_id',$1,true)")
        .bind(fixture.tenant.to_string())
        .execute(&mut *transaction)
        .await
        .expect("tenant context");
    sqlx::query("SELECT set_config('ple.session_hash',$1,true)")
        .bind(session.to_string())
        .execute(&mut *transaction)
        .await
        .expect("presented session context");
    transaction
}

fn assert_invalid_parameter(error: sqlx::Error) {
    assert_eq!(
        error
            .as_database_error()
            .and_then(|database_error| database_error.code())
            .as_deref(),
        Some("22023")
    );
}

pub(super) async fn saved_searches_are_normalized_revisioned_and_personal(fixture: &Fixture) {
    let filter = CatalogSearchFilter::from_query(CatalogSearchQuery {
        text: Some("  protein   folding ".into()),
        ..CatalogSearchQuery::default()
    })
    .expect("saved filter normalization");
    let saved = fixture
        .store
        .replace_saved_problem_search(
            fixture.context,
            fixture.elena_session,
            ReplaceSavedProblemSearchCommand {
                reference: None,
                expected_revision: None,
                title: "Protein folding".into(),
                filter,
            },
        )
        .await
        .expect("save normalized D1 filter");
    assert_eq!(
        saved.filter.fresh_query().text.as_deref(),
        Some("protein folding")
    );
    assert!(saved.filter.fresh_query().cursor.is_none());
    let persisted_digest: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM saved_problem_search WHERE owner_tenant_id=$1 AND search_reference=$2 \
         AND query_schema_version=1 AND octet_length(normalized_query_sha256)=32",
    ).bind(fixture.tenant.as_uuid()).bind(i32::try_from(saved.reference.number()).expect("reference"))
        .fetch_one(&fixture.pool).await.expect("saved query durable digest");
    assert_eq!(
        persisted_digest, 1,
        "saved D1 filter carries v1 canonical digest evidence"
    );
    let again = fixture
        .store
        .replace_saved_problem_search(
            fixture.context,
            fixture.elena_session,
            ReplaceSavedProblemSearchCommand {
                reference: Some(saved.reference),
                expected_revision: Some(saved.revision),
                title: saved.title.clone(),
                filter: saved.filter.clone(),
            },
        )
        .await
        .expect("saved-search no-op");
    assert_eq!(
        again.revision, saved.revision,
        "saved-search no-op preserves revision"
    );
    assert_eq!(
        fixture
            .store
            .get_saved_problem_search(fixture.context, fixture.ada_session, saved.reference,)
            .await
            .expect("foreign saved-search read"),
        None,
        "personal searches have no foreign existence oracle"
    );
    assert!(
        fixture
            .store
            .delete_saved_problem_search(
                fixture.context,
                fixture.elena_session,
                saved.reference,
                saved.revision,
            )
            .await
            .expect("saved-search deletion")
    );
    assert_eq!(
        fixture
            .store
            .list_saved_problem_searches(
                fixture.context,
                fixture.elena_session,
                PageRequest::first(PageSize::new(100).expect("page")),
            )
            .await
            .expect("saved-search list")
            .items
            .len(),
        0
    );
}

pub(super) async fn aggregate_limits_title_conflicts_and_broker_input_validation(
    fixture: &Fixture,
) {
    let saved_rows_before: i64 =
        sqlx::query_scalar("SELECT count(*) FROM saved_problem_search WHERE owner_tenant_id=$1")
            .bind(fixture.tenant.as_uuid())
            .fetch_one(&fixture.pool)
            .await
            .expect("saved-search count before Unicode-whitespace rejection");
    assert!(matches!(
        fixture
            .store
            .replace_saved_problem_search(
                fixture.context,
                fixture.elena_session,
                ReplaceSavedProblemSearchCommand {
                    reference: None,
                    expected_revision: None,
                    title: "\u{a0}D2 Unicode whitespace\u{a0}".into(),
                    filter: CatalogSearchFilter::from_query(CatalogSearchQuery::default())
                        .expect("Unicode-whitespace title filter"),
                },
            )
            .await,
        Err(StoreError::InvalidRecord(_))
    ));
    let saved_rows_after: i64 =
        sqlx::query_scalar("SELECT count(*) FROM saved_problem_search WHERE owner_tenant_id=$1")
            .bind(fixture.tenant.as_uuid())
            .fetch_one(&fixture.pool)
            .await
            .expect("saved-search count after Unicode-whitespace rejection");
    assert_eq!(
        saved_rows_after, saved_rows_before,
        "Unicode-whitespace title rejection persists no saved search"
    );

    let full = fixture
        .store
        .replace_problem_collection(
            fixture.context,
            fixture.elena_session,
            ReplaceProblemCollectionCommand {
                target: ProblemCollectionReplacementTarget::NewNamed,
                expected_revision: None,
                title: Some("D2 valid two hundred".into()),
                visibility: Some(ProblemCollectionVisibility::Private),
                question_ids: fixture.bulk_public_questions.clone(),
            },
        )
        .await
        .expect("atomic 200-member replacement");
    let full_members = fixture
        .store
        .list_problem_collection_members(
            fixture.context,
            fixture.elena_session,
            full.reference,
            PageRequest::first(PageSize::new(100).expect("page")),
        )
        .await
        .expect("first bounded 200-member page")
        .expect("200-member collection exists");
    assert_eq!(full_members.members.items.len(), 100);
    assert!(
        full_members.members.next_cursor.is_some(),
        "200 members page through the bounded contract"
    );
    let updated = fixture
        .store
        .replace_problem_collection(
            fixture.context,
            fixture.elena_session,
            ReplaceProblemCollectionCommand {
                target: ProblemCollectionReplacementTarget::Existing(full.reference),
                expected_revision: Some(full.revision),
                title: Some("D2 valid two hundred updated".into()),
                visibility: Some(ProblemCollectionVisibility::Private),
                question_ids: fixture.bulk_public_questions.clone(),
            },
        )
        .await
        .expect("current strong revision replaces full collection");
    assert!(matches!(
        fixture
            .store
            .replace_problem_collection(
                fixture.context,
                fixture.elena_session,
                ReplaceProblemCollectionCommand {
                    target: ProblemCollectionReplacementTarget::Existing(full.reference),
                    expected_revision: Some(full.revision),
                    title: Some("stale replacement".into()),
                    visibility: Some(ProblemCollectionVisibility::Private),
                    question_ids: Vec::new(),
                },
            )
            .await,
        Err(StoreError::Conflict)
    ));
    assert!(matches!(
        fixture
            .store
            .delete_problem_collection(
                fixture.context,
                fixture.elena_session,
                full.reference,
                full.revision,
            )
            .await,
        Err(StoreError::Conflict)
    ));
    assert!(
        fixture
            .store
            .delete_problem_collection(
                fixture.context,
                fixture.elena_session,
                full.reference,
                updated.revision,
            )
            .await
            .expect("current revision deletes named collection")
    );

    let collision = fixture
        .store
        .replace_problem_collection(
            fixture.context,
            fixture.ada_session,
            ReplaceProblemCollectionCommand {
                target: ProblemCollectionReplacementTarget::NewNamed,
                expected_revision: None,
                title: Some("D2 Case Collision".into()),
                visibility: Some(ProblemCollectionVisibility::Private),
                question_ids: Vec::new(),
            },
        )
        .await
        .expect("first case-insensitive collection title");
    assert!(matches!(
        fixture
            .store
            .replace_problem_collection(
                fixture.context,
                fixture.ada_session,
                ReplaceProblemCollectionCommand {
                    target: ProblemCollectionReplacementTarget::NewNamed,
                    expected_revision: None,
                    title: Some("d2 case collision".into()),
                    visibility: Some(ProblemCollectionVisibility::Private),
                    question_ids: Vec::new(),
                },
            )
            .await,
        Err(StoreError::AlreadyExists)
    ));
    let named_count: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM problem_collection WHERE owner_tenant_id=$1 AND owner_user_id=$2 AND kind='named'",
    ).bind(fixture.tenant.as_uuid()).bind(fixture.ada.as_uuid()).fetch_one(&fixture.pool).await.expect("Ada named count");
    for index in named_count..100 {
        fixture
            .store
            .replace_problem_collection(
                fixture.context,
                fixture.ada_session,
                ReplaceProblemCollectionCommand {
                    target: ProblemCollectionReplacementTarget::NewNamed,
                    expected_revision: None,
                    title: Some(format!("D2 named cap {index}")),
                    visibility: Some(ProblemCollectionVisibility::Private),
                    question_ids: Vec::new(),
                },
            )
            .await
            .expect("serialized named collection below cap");
    }
    assert!(matches!(
        fixture
            .store
            .replace_problem_collection(
                fixture.context,
                fixture.ada_session,
                ReplaceProblemCollectionCommand {
                    target: ProblemCollectionReplacementTarget::NewNamed,
                    expected_revision: None,
                    title: Some("D2 named cap overflow".into()),
                    visibility: Some(ProblemCollectionVisibility::Private),
                    question_ids: Vec::new(),
                },
            )
            .await,
        Err(StoreError::InvalidRecord(_))
    ));
    let saved_filter =
        CatalogSearchFilter::from_query(CatalogSearchQuery::default()).expect("saved cap filter");
    let first_saved = fixture
        .store
        .replace_saved_problem_search(
            fixture.context,
            fixture.ada_session,
            ReplaceSavedProblemSearchCommand {
                reference: None,
                expected_revision: None,
                title: "D2 Saved Collision".into(),
                filter: saved_filter.clone(),
            },
        )
        .await
        .expect("first saved title");
    assert!(matches!(
        fixture
            .store
            .replace_saved_problem_search(
                fixture.context,
                fixture.ada_session,
                ReplaceSavedProblemSearchCommand {
                    reference: None,
                    expected_revision: None,
                    title: "d2 saved collision".into(),
                    filter: saved_filter.clone()
                },
            )
            .await,
        Err(StoreError::AlreadyExists)
    ));
    for index in 1..100 {
        fixture
            .store
            .replace_saved_problem_search(
                fixture.context,
                fixture.ada_session,
                ReplaceSavedProblemSearchCommand {
                    reference: None,
                    expected_revision: None,
                    title: format!("D2 saved cap {index}"),
                    filter: saved_filter.clone(),
                },
            )
            .await
            .expect("serialized saved search below cap");
    }
    assert!(matches!(
        fixture
            .store
            .replace_saved_problem_search(
                fixture.context,
                fixture.ada_session,
                ReplaceSavedProblemSearchCommand {
                    reference: None,
                    expected_revision: None,
                    title: "D2 saved cap overflow".into(),
                    filter: saved_filter
                },
            )
            .await,
        Err(StoreError::InvalidRecord(_))
    ));
    assert!(first_saved.reference.number() > 0);

    let mut tx = fixture
        .pool
        .begin()
        .await
        .expect("direct broker validation transaction");
    sqlx::query("SET LOCAL ROLE ple_app")
        .execute(&mut *tx)
        .await
        .expect("application capability role");
    sqlx::query("SELECT set_config('ple.tenant_id',$1,true)")
        .bind(fixture.tenant.to_string())
        .execute(&mut *tx)
        .await
        .expect("tenant context");
    let malformed_qid = sqlx::query("SELECT * FROM public.ple_replace_problem_collection_v1($1,$2,NULL,0,'D2 malformed','private',ARRAY['bad']::text[])")
        .bind(fixture.tenant.as_uuid()).bind(fixture.elena_session.to_string()).fetch_all(&mut *tx).await.expect_err("broker rejects malformed Question ID");
    assert_eq!(
        malformed_qid
            .as_database_error()
            .and_then(|error| error.code())
            .as_deref(),
        Some("22023")
    );
    tx.rollback().await.expect("malformed identifier rollback");
    let mut tx = fixture
        .pool
        .begin()
        .await
        .expect("unknown query key transaction");
    sqlx::query("SET LOCAL ROLE ple_app")
        .execute(&mut *tx)
        .await
        .expect("application capability role");
    sqlx::query("SELECT set_config('ple.tenant_id',$1,true)")
        .bind(fixture.tenant.to_string())
        .execute(&mut *tx)
        .await
        .expect("tenant context");
    let unknown_key = sqlx::query("SELECT * FROM public.ple_replace_saved_problem_search_v1($1,$2,NULL,0,'D2 unknown key','{\"unknown\":true}'::jsonb)")
        .bind(fixture.tenant.as_uuid()).bind(fixture.elena_session.to_string()).fetch_all(&mut *tx).await.expect_err("broker rejects unknown saved-filter key");
    assert_eq!(
        unknown_key
            .as_database_error()
            .and_then(|error| error.code())
            .as_deref(),
        Some("22023")
    );
    tx.rollback().await.expect("unknown saved-filter rollback");

    let canonical_filter = serde_json::json!({
        "text": null,
        "bylines": ["ada lovelace", "grace hopper"],
        "backends": ["qti", "h5p"],
        "tags": ["alpha", "zeta"],
        "responseFamilies": ["numeric", "multipleChoice"],
        "taxonomy": [
            {"scheme": "course", "code": "alpha"},
            {"scheme": "course", "code": "zeta"}
        ],
        "capabilities": ["serverGrading", "partialCredit", "hints"],
        "licenses": ["ccBySa", "ccByNc", "cc0"],
        "publicationScopes": ["institution", "public"],
        "evidence": "any",
        "usedInMyCourses": "any",
        "authorship": "any"
    });
    let mut canonical_transaction = begin_app_transaction(fixture, fixture.elena_session).await;
    let canonical: (i32, sqlx::types::Json<serde_json::Value>, i64) = sqlx::query_as(
        "SELECT search_reference,normalized_query,revision \
         FROM public.ple_replace_saved_problem_search_v1( \
         $1,$2,NULL,0,'Canonical replacement',$3)",
    )
    .bind(fixture.tenant.as_uuid())
    .bind(fixture.elena_session.to_string())
    .bind(sqlx::types::Json(canonical_filter.clone()))
    .fetch_one(&mut *canonical_transaction)
    .await
    .expect("direct application broker accepts the Rust-canonical filter");
    canonical_transaction
        .commit()
        .await
        .expect("canonical saved-filter commit");
    let saved_reference = canonical.0;
    assert_eq!(canonical.1.0, canonical_filter);
    assert_eq!(canonical.2, 1);
    let canonical_revision = canonical.2;

    let mut noncanonical_bylines = canonical_filter.clone();
    noncanonical_bylines["bylines"] = serde_json::json!(["grace hopper", "ada lovelace"]);
    let mut noncanonical_backends = canonical_filter.clone();
    noncanonical_backends["backends"] = serde_json::json!(["h5p", "qti"]);
    let mut noncanonical_tags = canonical_filter.clone();
    noncanonical_tags["tags"] = serde_json::json!(["zeta", "alpha"]);
    let mut noncanonical_responses = canonical_filter.clone();
    noncanonical_responses["responseFamilies"] = serde_json::json!(["multipleChoice", "numeric"]);
    let mut noncanonical_taxonomy = canonical_filter.clone();
    noncanonical_taxonomy["taxonomy"] = serde_json::json!([
        {"scheme": "course", "code": "zeta"},
        {"scheme": "course", "code": "alpha"}
    ]);
    let mut noncanonical_capabilities = canonical_filter.clone();
    noncanonical_capabilities["capabilities"] =
        serde_json::json!(["hints", "partialCredit", "serverGrading"]);
    let mut noncanonical_licenses = canonical_filter.clone();
    noncanonical_licenses["licenses"] = serde_json::json!(["cc0", "ccByNc", "ccBySa"]);
    let mut noncanonical_publication_scopes = canonical_filter.clone();
    noncanonical_publication_scopes["publicationScopes"] =
        serde_json::json!(["public", "institution"]);
    let mut invalid_type = canonical_filter.clone();
    invalid_type["bylines"] = serde_json::json!("not-an-array");
    let mut invalid_enum = canonical_filter.clone();
    invalid_enum["backends"] = serde_json::json!(["ambientBackend"]);
    let mut oversized = canonical_filter.clone();
    oversized["bylines"] = serde_json::json!(
        (0..17)
            .map(|index| format!("bounded byline {index}"))
            .collect::<Vec<_>>()
    );
    let mut invalid_taxonomy = canonical_filter;
    invalid_taxonomy["taxonomy"] = serde_json::json!([{"scheme": "", "code": "term"}]);
    let before: (String, sqlx::types::Json<serde_json::Value>, i64, Vec<u8>) = sqlx::query_as(
        "SELECT title,normalized_query,revision,normalized_query_sha256 FROM saved_problem_search \
         WHERE owner_tenant_id=$1 AND search_reference=$2",
    )
    .bind(fixture.tenant.as_uuid())
    .bind(saved_reference)
    .fetch_one(&fixture.pool)
    .await
    .expect("retained saved-search state before hostile broker calls");
    assert_eq!(before.1.0, canonical.1.0);
    for (label, invalid_filter) in [
        ("noncanonical byline order", noncanonical_bylines),
        ("noncanonical backend order", noncanonical_backends),
        ("noncanonical tag order", noncanonical_tags),
        ("noncanonical response order", noncanonical_responses),
        ("noncanonical taxonomy order", noncanonical_taxonomy),
        ("noncanonical capability order", noncanonical_capabilities),
        ("noncanonical license order", noncanonical_licenses),
        (
            "noncanonical publication scope order",
            noncanonical_publication_scopes,
        ),
        ("invalid field type", invalid_type),
        ("invalid closed enum", invalid_enum),
        ("oversized array", oversized),
        ("invalid taxonomy value", invalid_taxonomy),
    ] {
        let mut transaction = begin_app_transaction(fixture, fixture.elena_session).await;
        let error = sqlx::query(
            "SELECT * FROM public.ple_replace_saved_problem_search_v1( \
             $1,$2,$3,$4,'Hostile replacement',$5)",
        )
        .bind(fixture.tenant.as_uuid())
        .bind(fixture.elena_session.to_string())
        .bind(saved_reference)
        .bind(canonical_revision)
        .bind(sqlx::types::Json(invalid_filter))
        .fetch_all(&mut *transaction)
        .await
        .expect_err(label);
        assert_invalid_parameter(error);
        transaction
            .rollback()
            .await
            .expect("hostile saved-filter rollback");
    }
    let corrupt = serde_json::json!({
        "text": null,
        "bylines": [],
        "backends": [],
        "tags": [],
        "responseFamilies": [],
        "taxonomy": [],
        "capabilities": [],
        "licenses": [],
        "publicationScopes": [],
        "evidence": "availableToAnyone",
        "usedInMyCourses": "any",
        "authorship": "any"
    });
    let corruption = sqlx::query(
        "UPDATE saved_problem_search SET normalized_query=$3 \
         WHERE owner_tenant_id=$1 AND search_reference=$2",
    )
    .bind(fixture.tenant.as_uuid())
    .bind(saved_reference)
    .bind(sqlx::types::Json(corrupt))
    .execute(&fixture.pool)
    .await
    .expect_err("table constraint rejects saved-filter corruption");
    assert_eq!(
        corruption
            .as_database_error()
            .and_then(|database_error| database_error.code())
            .as_deref(),
        Some("23514")
    );

    let mut collection_delete = begin_app_transaction(fixture, fixture.ada_session).await;
    let error =
        sqlx::query("SELECT public.ple_delete_problem_collection_v1($1,$2,$3,NULL::bigint)")
            .bind(fixture.tenant.as_uuid())
            .bind(fixture.ada_session.to_string())
            .bind(i32::try_from(collision.reference.number()).expect("collection reference"))
            .execute(&mut *collection_delete)
            .await
            .expect_err("NULL collection revision is rejected before deletion");
    assert_invalid_parameter(error);
    collection_delete
        .rollback()
        .await
        .expect("NULL collection revision rollback");

    let mut saved_delete = begin_app_transaction(fixture, fixture.elena_session).await;
    let error =
        sqlx::query("SELECT public.ple_delete_saved_problem_search_v1($1,$2,$3,NULL::bigint)")
            .bind(fixture.tenant.as_uuid())
            .bind(fixture.elena_session.to_string())
            .bind(saved_reference)
            .execute(&mut *saved_delete)
            .await
            .expect_err("NULL saved-search revision is rejected before deletion");
    assert_invalid_parameter(error);
    saved_delete
        .rollback()
        .await
        .expect("NULL saved-search revision rollback");

    let after: (String, sqlx::types::Json<serde_json::Value>, i64, Vec<u8>) = sqlx::query_as(
        "SELECT title,normalized_query,revision,normalized_query_sha256 FROM saved_problem_search \
         WHERE owner_tenant_id=$1 AND search_reference=$2",
    )
    .bind(fixture.tenant.as_uuid())
    .bind(saved_reference)
    .fetch_one(&fixture.pool)
    .await
    .expect("saved search survives hostile writes and NULL deletion");
    assert_eq!(
        after, before,
        "rejected saved-search operations preserve exact state"
    );
    assert!(
        fixture
            .store
            .get_problem_collection_summary(
                fixture.context,
                fixture.ada_session,
                collision.reference,
            )
            .await
            .expect("collection lookup after NULL deletion")
            .is_some(),
        "NULL expected revision leaves the collection intact"
    );
}

use super::*;

#[tokio::test]
async fn catalog_and_taxonomy_lists_use_cursors_and_hide_deprecated_versions() {
    let store = Arc::new(MemoryStore::default());
    let tenant = TenantId::from_uuid(id(1));
    let context = TenantContext::from_authenticated_session(tenant);
    let publisher = UserId::from_uuid(id(10));
    let cookie = issued_cookie(&store, vec![UserRole::Publisher], publisher).await;
    let app = router(
        Arc::clone(&store),
        Arc::new(FixtureRegistry {
            capabilities: BackendCapabilities::from_iter([Capability::ServerGrading]),
        }),
        Arc::new(ReviewNotRequired),
    );

    let mut published_references = Vec::new();
    for value in [20_u128, 30_u128] {
        let workspace = WorkspaceId::from_uuid(id(value));
        let version = VersionId::from_uuid(id(value + 1));
        let draft_revision = store
            .upsert_draft(context, publisher, None, draft(tenant, workspace, version))
            .await
            .expect("draft save")
            .revision;
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!("/api/problems/{workspace}/publish"))
                    .header("cookie", &cookie)
                    .header(IF_MATCH, strong_if_match(draft_revision))
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"scope":"public"}"#))
                    .expect("publish request"),
            )
            .await
            .expect("publish response");
        assert_eq!(response.status(), StatusCode::CREATED);
        let response = response_json(response).await;
        published_references.push((
            response["problem"]
                .as_str()
                .expect("published problem ID")
                .to_string(),
            response["version"]
                .as_str()
                .expect("published version ID")
                .to_string(),
        ));
    }

    let first = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/problems?pageSize=1")
                .header("cookie", &cookie)
                .body(Body::empty())
                .expect("list request"),
        )
        .await
        .expect("list response");
    assert_eq!(first.status(), StatusCode::OK);
    let first = response_json(first).await;
    assert_eq!(first["items"].as_array().map(Vec::len), Some(1));
    let cursor = first["nextCursor"]
        .as_str()
        .expect("first page cursor")
        .to_string();
    let second = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!(
                    "/api/problems?pageSize=1&cursor={}",
                    cursor.replace('/', "%2F")
                ))
                .header("cookie", &cookie)
                .body(Body::empty())
                .expect("list request"),
        )
        .await
        .expect("list response");
    let second = response_json(second).await;
    assert_eq!(second["items"].as_array().map(Vec::len), Some(1));
    assert_ne!(first["items"][0], second["items"][0]);
    assert_eq!(second["nextCursor"], serde_json::Value::Null);

    let taxonomy = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/taxonomy?pageSize=1")
                .header("cookie", &cookie)
                .body(Body::empty())
                .expect("taxonomy request"),
        )
        .await
        .expect("taxonomy response");
    assert_eq!(taxonomy.status(), StatusCode::OK);
    let taxonomy = response_json(taxonomy).await;
    assert_eq!(taxonomy["items"].as_array().map(Vec::len), Some(1));
    assert!(taxonomy["nextCursor"].is_string());
    let taxonomy_cursor = taxonomy["nextCursor"]
        .as_str()
        .expect("taxonomy cursor")
        .to_string();
    let taxonomy_second = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!("/api/taxonomy?pageSize=1&cursor={taxonomy_cursor}"))
                .header("cookie", &cookie)
                .body(Body::empty())
                .expect("taxonomy continuation request"),
        )
        .await
        .expect("taxonomy continuation response");
    assert_eq!(taxonomy_second.status(), StatusCode::OK);
    let taxonomy_second = response_json(taxonomy_second).await;
    assert_eq!(taxonomy_second["items"].as_array().map(Vec::len), Some(1));
    assert_ne!(taxonomy["items"][0], taxonomy_second["items"][0]);
    assert_eq!(taxonomy_second["nextCursor"], serde_json::Value::Null);

    for path in ["/api/problems", "/api/taxonomy"] {
        for query in ["pageSize=0", "pageSize=101", "cursor=", "offset=1"] {
            let response = app
                .clone()
                .oneshot(
                    Request::builder()
                        .uri(format!("{path}?{query}"))
                        .header("cookie", &cookie)
                        .body(Body::empty())
                        .expect("invalid pagination request"),
                )
                .await
                .expect("invalid pagination response");
            assert_eq!(
                response.status(),
                StatusCode::BAD_REQUEST,
                "{path}?{query} must be rejected"
            );
        }
    }

    let (problem, version) = &published_references[0];
    let deprecated = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!(
                    "/api/problems/{problem}/versions/{version}/deprecate"
                ))
                .header("cookie", &cookie)
                .header("content-type", "application/json")
                .body(Body::from(r#"{"reason":"Correction available"}"#))
                .expect("deprecate request"),
        )
        .await
        .expect("deprecate response");
    assert_eq!(deprecated.status(), StatusCode::OK);

    let browse = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/problems?pageSize=10")
                .header("cookie", &cookie)
                .body(Body::empty())
                .expect("catalog request"),
        )
        .await
        .expect("catalog response");
    let browse = response_json(browse).await;
    assert_eq!(browse["items"].as_array().map(Vec::len), Some(1));

    let exact = app
        .oneshot(
            Request::builder()
                .uri(format!("/api/problems/{problem}/versions/{version}"))
                .header("cookie", cookie)
                .body(Body::empty())
                .expect("exact request"),
        )
        .await
        .expect("exact response");
    assert_eq!(exact.status(), StatusCode::OK);
}

#[tokio::test]
async fn catalog_search_and_safe_detail_are_authenticated_bounded_and_non_cacheable() {
    let store = Arc::new(MemoryStore::default());
    let tenant = TenantId::from_uuid(id(1));
    let context = TenantContext::from_authenticated_session(tenant);
    let publisher = UserId::from_uuid(id(901));
    let cookie = issued_cookie(&store, vec![UserRole::Publisher], publisher).await;
    let workspace = WorkspaceId::from_uuid(id(902));
    let version = VersionId::from_uuid(id(903));
    let draft_revision = store
        .upsert_draft(context, publisher, None, draft(tenant, workspace, version))
        .await
        .expect("draft save")
        .revision;
    let app = router(
        Arc::clone(&store),
        Arc::new(FixtureRegistry {
            capabilities: BackendCapabilities::from_iter([Capability::ServerGrading]),
        }),
        Arc::new(ReviewNotRequired),
    );
    let published = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/problems/{workspace}/publish"))
                .header("cookie", &cookie)
                .header(IF_MATCH, strong_if_match(draft_revision))
                .header("content-type", "application/json")
                .body(Body::from(r#"{"scope":"public"}"#))
                .expect("publish request"),
        )
        .await
        .expect("publish response");
    let published = response_json(published).await;
    let problem = published["problem"].as_str().expect("problem id");
    let version = published["version"].as_str().expect("version id");

    let search = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/problems/search?text=catalog&pageSize=1")
                .header("cookie", &cookie)
                .body(Body::empty())
                .expect("search request"),
        )
        .await
        .expect("search response");
    assert_eq!(search.status(), StatusCode::OK);
    assert_eq!(search.headers()["cache-control"], "no-store");
    let search = response_json(search).await;
    assert_eq!(search["items"].as_array().map(Vec::len), Some(1));
    assert_eq!(search["facets"]["statistics"]["available"], 0);
    assert_eq!(search["facets"]["statistics"]["unavailable"], 1);

    let display_reference = format!(
        "P-{}-v{}",
        search["items"][0]["publicId"].as_u64().expect("public ID"),
        search["items"][0]["versionNumber"]
            .as_u64()
            .expect("version number"),
    );
    let exact_search = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!("/api/problems/search?text={display_reference}"))
                .header("cookie", &cookie)
                .body(Body::empty())
                .expect("exact display-reference search request"),
        )
        .await
        .expect("exact display-reference search response");
    assert_eq!(exact_search.status(), StatusCode::OK);
    let exact_search = response_json(exact_search).await;
    assert_eq!(exact_search["items"].as_array().map(Vec::len), Some(1));
    assert_eq!(
        exact_search["items"][0]["publicId"],
        search["items"][0]["publicId"]
    );
    assert_eq!(
        exact_search["items"][0]["versionNumber"],
        search["items"][0]["versionNumber"]
    );

    let detail = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!("/api/problems/{problem}/versions/{version}/detail"))
                .header("cookie", &cookie)
                .body(Body::empty())
                .expect("detail request"),
        )
        .await
        .expect("detail response");
    assert_eq!(detail.status(), StatusCode::OK);
    assert_eq!(detail.headers()["cache-control"], "no-store");
    let detail = response_json(detail).await;
    for forbidden in ["source", "response", "grading", "answerKey", "provider"] {
        assert!(detail.get(forbidden).is_none(), "detail leaked {forbidden}");
    }
    assert_eq!(detail["statistics"], "unavailable");

    let hostile = app
        .oneshot(
            Request::builder()
                .uri("/api/problems/search?offset=1")
                .header("cookie", cookie)
                .body(Body::empty())
                .expect("hostile request"),
        )
        .await
        .expect("hostile response");
    assert_eq!(hostile.status(), StatusCode::BAD_REQUEST);
}

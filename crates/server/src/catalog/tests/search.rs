use super::*;
use base64::Engine;
use learning_data_access::CatalogStore;
use objects::Sha256Digest;
use question_model::ProblemVersionRef;

#[tokio::test]
async fn catalog_and_taxonomy_lists_use_cursors_and_retain_lifecycle_labels() {
    let store = Arc::new(MemoryStore::default());
    let tenant = TenantId::from_uuid(id(1));
    let context = TenantContext::from_authenticated_session(tenant);
    let publisher = UserId::from_uuid(id(10));
    let cookie = issued_cookie(&store, vec![UserRole::Instructor], publisher).await;
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
                    .body(Body::from(
                        r#"{"scope":"public","byline":{"names":["PLE fixture"]}}"#,
                    ))
                    .expect("publish request"),
            )
            .await
            .expect("publish response");
        assert_eq!(response.status(), StatusCode::CREATED);
        let response = response_json(response).await;
        published_references.push(
            response["questionId"]
                .as_str()
                .expect("published Question ID")
                .to_string(),
        );
    }

    let first = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/problems?page_size=1")
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
                    "/api/problems?page_size=1&cursor={}",
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
                .uri("/api/taxonomy?page_size=1")
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
                .uri(format!(
                    "/api/taxonomy?page_size=1&cursor={taxonomy_cursor}"
                ))
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
        for query in [
            "page_size=0",
            "page_size=101",
            "pageSize=1",
            "cursor=",
            "offset=1",
        ] {
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

    let archived_question_id = published_references[0]
        .parse()
        .expect("published Question ID parses");
    let archived_record = store
        .resolve_catalog_problem(
            context,
            question_model::ProblemDisplayRef {
                question_id: archived_question_id,
            },
        )
        .await
        .expect("catalog lookup")
        .expect("published question exists");
    let deprecated_question_id = published_references[1]
        .parse()
        .expect("published Question ID parses");
    let deprecated_record = store
        .resolve_catalog_problem(
            context,
            question_model::ProblemDisplayRef {
                question_id: deprecated_question_id,
            },
        )
        .await
        .expect("catalog lookup")
        .expect("published question exists");
    store
        .transition_catalog_problem(
            context,
            publisher,
            ProblemVersionRef {
                problem: archived_record.problem,
                version: archived_record.version,
            },
            learning_data_access::CatalogTransition::Deprecate {
                reason: "A newer question addresses this topic.".to_string(),
            },
        )
        .await
        .expect("deprecate published question");

    store
        .transition_catalog_problem(
            context,
            publisher,
            ProblemVersionRef {
                problem: archived_record.problem,
                version: archived_record.version,
            },
            learning_data_access::CatalogTransition::Archive,
        )
        .await
        .expect("archive deprecated question");

    store
        .transition_catalog_problem(
            context,
            publisher,
            ProblemVersionRef {
                problem: deprecated_record.problem,
                version: deprecated_record.version,
            },
            learning_data_access::CatalogTransition::Deprecate {
                reason: "A newer question addresses this topic.".to_string(),
            },
        )
        .await
        .expect("deprecate published question");

    let browse = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/problems?page_size=10")
                .header("cookie", &cookie)
                .body(Body::empty())
                .expect("catalog request"),
        )
        .await
        .expect("catalog response");
    let browse = response_json(browse).await;
    for (question_id, lifecycle) in [
        (&published_references[0], "archived"),
        (&published_references[1], "deprecated"),
    ] {
        assert!(browse["items"].as_array().is_some_and(|items| {
            items.iter().any(|item| {
                item["questionId"].as_str() == Some(question_id.as_str())
                    && item["lifecycle"]["state"] == lifecycle
            })
        }));

        let detail = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri(format!("/api/problems/by-id/{question_id}/detail"))
                    .header("cookie", &cookie)
                    .body(Body::empty())
                    .expect("retired detail request"),
            )
            .await
            .expect("retired detail response");
        assert_eq!(detail.status(), StatusCode::OK);
        let detail = response_json(detail).await;
        assert_eq!(detail["summary"]["lifecycle"]["state"], lifecycle);
    }
}

#[tokio::test]
async fn catalog_search_and_safe_detail_are_authenticated_bounded_and_non_cacheable() {
    let store = Arc::new(MemoryStore::default());
    let tenant = TenantId::from_uuid(id(1));
    let context = TenantContext::from_authenticated_session(tenant);
    let publisher = UserId::from_uuid(id(901));
    let cookie = issued_approved_instructor_cookie(&store, publisher).await;
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
                .body(Body::from(
                    r#"{"scope":"public","byline":{"names":["PLE fixture"]}}"#,
                ))
                .expect("publish request"),
        )
        .await
        .expect("publish response");
    response_json(published).await;
    let search = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(
                    "/api/problems/search?text=catalog&bylines=PLE%20fixture&backends=native&backends=qti&response_families=numeric&response_families=shortText&licenses=ccBySa&licenses=cc0&used_in_my_courses=any&page_size=1",
                )
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
    assert_eq!(search["facets"]["evidence"]["available"], 0);
    assert_eq!(search["facets"]["evidence"]["unavailable"], 1);
    assert_eq!(search["items"][0]["summary"]["responseFamily"], "numeric");

    let display_reference = search["items"][0]["summary"]["questionId"]
        .as_str()
        .expect("Question ID")
        .to_string();
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
        exact_search["items"][0]["summary"]["questionId"],
        search["items"][0]["summary"]["questionId"]
    );
    assert!(exact_search["items"][0]["summary"].get("problem").is_none());
    assert!(exact_search["items"][0]["summary"].get("version").is_none());

    let retired = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!("/api/problems/by-id/{display_reference}/versions"))
                .header("cookie", &cookie)
                .body(Body::empty())
                .expect("retired catalog route request"),
        )
        .await
        .expect("retired catalog route response");
    assert_eq!(retired.status(), StatusCode::NOT_FOUND);

    let detail = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!("/api/problems/by-id/{display_reference}/detail"))
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
    assert_eq!(detail["prompt"]["kind"], "static");
    let prompt = detail["prompt"]
        .as_object()
        .expect("catalog prompt is one closed projection");
    assert_eq!(prompt.len(), 2);
    assert!(prompt.contains_key("kind"));
    assert!(prompt.contains_key("blocks"));
    for forbidden in [
        "seed",
        "randomization",
        "source",
        "response",
        "grading",
        "answer",
        "answerKey",
    ] {
        assert!(
            prompt.get(forbidden).is_none(),
            "catalog prompt leaked {forbidden}"
        );
    }
    assert_eq!(detail["evidence"]["state"], "insufficientEvidence");
    assert!(detail.get("statistics").is_none());
    assert_eq!(detail["usage"]["summary"]["ownCourseCount"], 0);
    assert_eq!(detail["usage"]["ownCourses"], serde_json::json!([]));

    let duplicate_text = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/problems/search?text=catalog&text=duplicate")
                .header("cookie", &cookie)
                .body(Body::empty())
                .expect("duplicate scalar request"),
        )
        .await
        .expect("duplicate scalar response");
    assert_eq!(duplicate_text.status(), StatusCode::BAD_REQUEST);
    assert_eq!(duplicate_text.headers()["cache-control"], "no-store");

    let unauthenticated_malformed = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/problems/search?publication_scopes=public")
                .body(Body::empty())
                .expect("unauthenticated malformed search request"),
        )
        .await
        .expect("unauthenticated malformed search response");
    assert_eq!(unauthenticated_malformed.status(), StatusCode::UNAUTHORIZED);
    assert_eq!(
        unauthenticated_malformed.headers()["cache-control"],
        "no-store"
    );

    for retired_or_camel_case_key in [
        "publicationScopes=public",
        "publication_scopes=public",
        "responseFamilies=numeric",
        "usedInMyCourses=any",
        "pageSize=1",
    ] {
        let rejected = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri(format!("/api/problems/search?{retired_or_camel_case_key}"))
                    .header("cookie", &cookie)
                    .body(Body::empty())
                    .expect("retired catalog search request"),
            )
            .await
            .expect("retired catalog search response");
        assert_eq!(
            rejected.status(),
            StatusCode::BAD_REQUEST,
            "{retired_or_camel_case_key} must be rejected"
        );
        assert_eq!(rejected.headers()["cache-control"], "no-store");
    }

    let hostile = app
        .oneshot(
            Request::builder()
                .uri("/api/problems/search?offset=1")
                .header("cookie", &cookie)
                .body(Body::empty())
                .expect("hostile request"),
        )
        .await
        .expect("hostile response");
    assert_eq!(hostile.status(), StatusCode::BAD_REQUEST);

    let malformed_cursor = router(
        Arc::clone(&store),
        Arc::new(FixtureRegistry {
            capabilities: BackendCapabilities::from_iter([Capability::ServerGrading]),
        }),
        Arc::new(ReviewNotRequired),
    )
    .oneshot(
        Request::builder()
            .uri("/api/problems/search?text=catalog&cursor=AAAA")
            .header("cookie", cookie)
            .body(Body::empty())
            .expect("malformed cursor request"),
    )
    .await
    .expect("malformed cursor response");
    assert_eq!(malformed_cursor.status(), StatusCode::UNPROCESSABLE_ENTITY);
}

#[tokio::test]
async fn catalog_read_routes_reject_student_access() {
    let store = Arc::new(MemoryStore::default());
    let tenant = TenantId::from_uuid(id(1));
    let context = TenantContext::from_authenticated_session(tenant);
    let instructor = UserId::from_uuid(id(31));
    let student = UserId::from_uuid(id(32));
    let app = router(
        Arc::clone(&store),
        Arc::new(FixtureRegistry {
            capabilities: BackendCapabilities::from_iter([Capability::ServerGrading]),
        }),
        Arc::new(ReviewNotRequired),
    );
    let student_cookie = issued_cookie(&store, vec![UserRole::Student], student).await;
    let instructor_cookie = issued_cookie(&store, vec![UserRole::Instructor], instructor).await;
    let workspace = WorkspaceId::from_uuid(id(33));
    let version = VersionId::from_uuid(id(34));
    let revision = store
        .upsert_draft(context, instructor, None, draft(tenant, workspace, version))
        .await
        .expect("draft save")
        .revision;
    let published = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/problems/{workspace}/publish"))
                .header("cookie", &instructor_cookie)
                .header(IF_MATCH, strong_if_match(revision))
                .header("content-type", "application/json")
                .body(Body::from(
                    r#"{"scope":"public","byline":{"names":["PLE fixture"]}}"#,
                ))
                .expect("publish request"),
        )
        .await
        .expect("publish response");
    let published = response_json(published).await;
    let question_id = published["questionId"].as_str().expect("Question ID");

    let student_blocked_endpoints = vec![
        "/api/problems?page_size=1".to_string(),
        "/api/taxonomy?page_size=1".to_string(),
        "/api/problems/search?text=catalog&page_size=1".to_string(),
        "/api/problems/by-id/not-a-question".to_string(),
        format!("/api/problems/by-id/{question_id}"),
        format!("/api/problems/by-id/{question_id}/detail"),
    ];
    for endpoint in student_blocked_endpoints {
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri(&endpoint)
                    .header("cookie", &student_cookie)
                    .body(Body::empty())
                    .expect("read request"),
            )
            .await
            .expect("read response");
        assert_eq!(
            response.status(),
            StatusCode::FORBIDDEN,
            "{endpoint} must reject student access"
        );
    }
}

#[tokio::test]
async fn catalog_search_rejects_a_cursor_forged_with_an_ordinary_sha256() {
    let store = Arc::new(MemoryStore::default());
    let tenant = TenantId::from_uuid(id(1));
    let context = TenantContext::from_authenticated_session(tenant);
    let publisher = UserId::from_uuid(id(951));
    let cookie = issued_approved_instructor_cookie(&store, publisher).await;
    let app = router(
        Arc::clone(&store),
        Arc::new(FixtureRegistry {
            capabilities: BackendCapabilities::from_iter([Capability::ServerGrading]),
        }),
        Arc::new(ReviewNotRequired),
    );
    for value in [952_u128, 953] {
        let workspace = WorkspaceId::from_uuid(id(value));
        let revision = store
            .upsert_draft(
                context,
                publisher,
                None,
                draft(tenant, workspace, VersionId::from_uuid(id(value + 100))),
            )
            .await
            .expect("save search fixture")
            .revision;
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!("/api/problems/{workspace}/publish"))
                    .header("cookie", &cookie)
                    .header(IF_MATCH, strong_if_match(revision))
                    .header("content-type", "application/json")
                    .body(Body::from(
                        r#"{"scope":"public","byline":{"names":["PLE fixture"]}}"#,
                    ))
                    .expect("publish search fixture"),
            )
            .await
            .expect("publish search fixture response");
        assert_eq!(response.status(), StatusCode::CREATED);
    }
    let first = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/problems/search?text=catalog&page_size=1")
                .header("cookie", &cookie)
                .body(Body::empty())
                .expect("first search request"),
        )
        .await
        .expect("first search response");
    let first = response_json(first).await;
    let cursor = first["nextCursor"].as_str().expect("search continuation");
    let mut forged = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode(cursor)
        .expect("server cursor is base64url");
    let tag_start = forged.len() - Sha256Digest::compute(b"").as_bytes().len();
    forged[tag_start - 1] ^= 1;
    let ordinary_digest = Sha256Digest::compute(&forged[..tag_start]);
    forged[tag_start..].copy_from_slice(ordinary_digest.as_bytes());
    let forged = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(forged);
    let response = app
        .oneshot(
            Request::builder()
                .uri(format!(
                    "/api/problems/search?text=catalog&page_size=1&cursor={forged}"
                ))
                .header("cookie", cookie)
                .body(Body::empty())
                .expect("forged search request"),
        )
        .await
        .expect("forged search response");
    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
}

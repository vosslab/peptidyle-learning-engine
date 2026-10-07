//! Mixed-page transport, projection, and capability-count regressions.

use super::*;
use learning_data_access::LibrarySearchEntry;
use question_model::{
    AccountId, LibrarySearchKind, LibrarySearchResult, PublishedQuestionFilter,
    QuestionPoolEditNumber, QuestionPoolId, QuestionPoolLibrarySummary, QuestionPoolMetadata,
    QuestionPoolMetadataEditNumber,
};

fn owner() -> AccountId {
    AccountId::from_debug_serial(91)
}

fn pool() -> QuestionPoolLibrarySummary {
    QuestionPoolLibrarySummary {
        question_pool_id: QuestionPoolId::from_random_identifier("P100001").expect("Pool"),
        owner_account_id: owner(),
        question_type: QuestionType::MultipleChoice,
        backend: QuestionBackend::Ple,
        license: QuestionLicense::CcBySa4_0,
        question_pool_edit_number: QuestionPoolEditNumber::new(1).expect("edit"),
        question_pool_metadata_edit_number: QuestionPoolMetadataEditNumber::new(1).expect("edit"),
        member_count: std::num::NonZeroU32::new(1).expect("member"),
        bloom: None,
        metadata: QuestionPoolMetadata {
            title: "Pool before Question".to_owned(),
            description: "Pool-owned description".to_owned(),
            discipline_uuid: uuid::Uuid::from_u128(1),
            discipline_name: "Biology".to_owned(),
            discipline_is_retired: false,
            subject_uuid: uuid::Uuid::from_u128(2),
            topic_uuid: None,
            subtopic_uuid: None,
            tags: vec!["pool-only".to_owned()],
            bloom_cognitive_process: None,
            bloom_knowledge_dimension: None,
        },
    }
}

fn store(page: Vec<LibrarySearchEntry>) -> PageOnlyLibrary {
    PageOnlyLibrary {
        continuation: QuestionLibrarySearchCursorPosition::TitleAscending {
            title: "next".to_owned(),
            public_id: pool()
                .question_pool_id
                .as_str()
                .parse()
                .expect("Library ID"),
        },
        page,
        catalog: Vec::new(),
        list_calls: AtomicUsize::new(0),
        search_calls: AtomicUsize::new(0),
        statistics_ids: Mutex::new(Vec::new()),
        statistics_calls: AtomicUsize::new(0),
    }
}

#[tokio::test]
async fn pool_only_page_never_reads_question_source_or_statistics() {
    let expected = pool();
    let store = store(vec![LibrarySearchEntry::Pool {
        summary: Box::new(expected.clone()),
        created_at: Timestamp::from_unix_millis(1),
    }]);
    let objects = RecordingObjects {
        inner: MemoryObjectStore::default(),
        reads: Mutex::new(Vec::new()),
    };
    let response = search_library_objects(
        &store,
        &objects,
        SessionTokenHash::compute(b"reader"),
        true,
        LibraryObjectSearchRequest {
            kind: LibrarySearchKind::Pools,
            ..LibraryObjectSearchRequest::default()
        },
    )
    .await;
    assert_eq!(response.status(), StatusCode::OK);
    let bytes = to_bytes(response.into_body(), 65_536).await.expect("body");
    let page: LibraryObjectSearchPage = serde_json::from_slice(&bytes).expect("mixed page");
    assert_eq!(
        page.items,
        vec![LibrarySearchResult::Pool {
            pool: Box::new(expected)
        }]
    );
    assert!(objects.reads.lock().expect("reads").is_empty());
    assert_eq!(store.statistics_calls.load(Ordering::SeqCst), 0);
    assert_eq!(store.list_calls.load(Ordering::SeqCst), 0);
}

#[tokio::test]
async fn mixed_projection_keeps_store_order_and_question_owner() {
    let objects = RecordingObjects {
        inner: MemoryObjectStore::default(),
        reads: Mutex::new(Vec::new()),
    };
    let (question, address) = stored_library_entry(&objects, "QQ00001").await;
    let id = question
        .published_question_revision_tuple
        .published_question_id
        .clone();
    let store = store(vec![
        LibrarySearchEntry::Pool {
            summary: Box::new(pool()),
            created_at: Timestamp::from_unix_millis(1),
        },
        LibrarySearchEntry::Question {
            entry: Box::new(question),
            owner_account_id: owner(),
        },
    ]);
    let response = search_library_objects(
        &store,
        &objects,
        SessionTokenHash::compute(b"reader"),
        true,
        LibraryObjectSearchRequest {
            kind: LibrarySearchKind::Both,
            ..LibraryObjectSearchRequest::default()
        },
    )
    .await;
    assert_eq!(response.status(), StatusCode::OK);
    let bytes = to_bytes(response.into_body(), 65_536).await.expect("body");
    let page: LibraryObjectSearchPage = serde_json::from_slice(&bytes).expect("mixed page");
    assert!(matches!(page.items[0], LibrarySearchResult::Pool { .. }));
    let LibrarySearchResult::Question {
        question,
        owner_account_id,
    } = &page.items[1]
    else {
        panic!("Question second");
    };
    assert_eq!(question.summary.question_id, id);
    assert_eq!(owner_account_id, &owner());
    assert_eq!(*objects.reads.lock().expect("reads"), vec![address]);
    assert_eq!(*store.statistics_ids.lock().expect("ids"), vec![id]);
}

#[test]
fn mixed_query_transport_is_closed_and_defaults_to_questions_all() {
    let parse = |suffix: &str| {
        let uri: Uri = format!("/api/library-objects/search{suffix}")
            .parse()
            .expect("URI");
        Query::<LibraryObjectSearchQuery>::try_from_uri(&uri)
            .ok()
            .and_then(|query| LibraryObjectSearchRequest::try_from(query.0).ok())
    };
    let default = parse("").expect("defaults");
    assert_eq!(default.kind, LibrarySearchKind::Questions);
    assert_eq!(default.questions, PublishedQuestionFilter::All);
    let query = parse(&format!(
        "?kind=both&questions=inNoPool&owner_account_id={}&text=type%3Amultiple+choice",
        owner()
    ))
    .expect("mixed filters");
    assert_eq!(query.kind, LibrarySearchKind::Both);
    assert_eq!(query.questions, PublishedQuestionFilter::InNoPool);
    assert_eq!(query.owner_account_id, Some(owner()));
    assert_eq!(query.text.as_deref(), Some("type:multiple choice"));
    for invalid in [
        "?kind=other",
        "?questions=none",
        "?membership=all",
        "?owner_account_id=invalid",
    ] {
        assert!(parse(invalid).is_none(), "{invalid}");
    }
}

#[test]
fn capability_facets_count_matching_questions_and_pools_by_their_common_backend() {
    let mut counts = empty_search_facets();
    counts.backends = vec![question_model::QuestionSearchBackendFacet {
        backend: QuestionBackend::Ple,
        count: 7,
    }];
    let facets = facets::from_store(counts);
    assert_eq!(facets.backends[0].count, 7);
    assert!(facets.capabilities.iter().any(|facet| facet.capability
        == question_model::Capability::ClientRendering
        && facet.count == 7));
    assert!(facets.capabilities.iter().any(|facet| facet.capability
        == question_model::Capability::ServerGrading
        && facet.count == 7));
}

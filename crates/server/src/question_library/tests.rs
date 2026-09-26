use async_trait::async_trait;
use axum::body::to_bytes;
use axum::http::{HeaderValue, StatusCode, Uri};
use learning_data_access::{
    QuestionLibrarySearchCursorPosition, QuestionLibrarySearchFacets, QuestionLibrarySearchPage,
};
use objects::{
    ObjectAddress, ObjectRecord, ObjectStore, ObjectStoreError, PutObject, SignedUrl, StoredObject,
    memory::MemoryObjectStore,
};
use question_model::{
    BloomCognitiveProcess, BloomKnowledgeDimension, ObjectId, PublishedQuestionRevisionTuple,
    PublishedQuestionSharedMetadata, QuestionAuthor, QuestionAuthorDisplayName, QuestionAuthorship,
    QuestionAvailability, QuestionFormat, QuestionLicense, QuestionRevisionNumber,
    QuestionSearchPage, QuestionSearchSort, QuestionStatistics, QuestionType, SourceObjectChecksum,
    Tag, Timestamp,
};
use std::sync::Mutex;
use std::sync::atomic::{AtomicUsize, Ordering};

use super::*;

struct LookupCountingStore(AtomicUsize);

#[async_trait]
impl QuestionLibraryStore for LookupCountingStore {
    async fn search_published_question_library_entries(
        &self,
        _: SessionTokenHash,
        _: learning_data_access::QuestionLibrarySearchRequest,
    ) -> Result<learning_data_access::QuestionLibrarySearchPage, StoreError> {
        Err(StoreError::Unavailable(
            "not used by this contract".to_string(),
        ))
    }

    async fn list_published_question_library_entries(
        &self,
        _: SessionTokenHash,
    ) -> Result<Vec<PublishedQuestionLibraryEntry>, StoreError> {
        Err(StoreError::Unavailable(
            "not used by this contract".to_string(),
        ))
    }

    async fn load_published_question_library_entry(
        &self,
        _: SessionTokenHash,
        _: &PublishedQuestionId,
    ) -> Result<PublishedQuestionLibraryEntry, StoreError> {
        self.0.fetch_add(1, Ordering::SeqCst);
        Err(StoreError::Unavailable("lookup must not occur".to_string()))
    }

    async fn load_published_question_revision_library_entry(
        &self,
        _: SessionTokenHash,
        _: &PublishedQuestionRevisionTuple,
    ) -> Result<PublishedQuestionLibraryEntry, StoreError> {
        Err(StoreError::Unavailable(
            "not used by this contract".to_string(),
        ))
    }

    async fn load_current_published_question_shared_metadata(
        &self,
        _: SessionTokenHash,
        _: &[PublishedQuestionId],
    ) -> Result<Vec<question_model::PublishedQuestionSharedMetadata>, StoreError> {
        Err(StoreError::Unavailable(
            "not used by this contract".to_string(),
        ))
    }

    async fn archive_published_question(
        &self,
        _: SessionTokenHash,
        _: &PublishedQuestionId,
        _: question_model::QuestionAvailabilityEditNumber,
        _: &str,
    ) -> Result<learning_data_access::PublishedQuestionAvailability, StoreError> {
        Err(StoreError::Unavailable(
            "not used by this contract".to_string(),
        ))
    }

    async fn restore_published_question(
        &self,
        _: SessionTokenHash,
        _: &PublishedQuestionId,
        _: question_model::QuestionAvailabilityEditNumber,
    ) -> Result<learning_data_access::PublishedQuestionAvailability, StoreError> {
        Err(StoreError::Unavailable(
            "not used by this contract".to_string(),
        ))
    }

    async fn correct_question_revision_bloom(
        &self,
        _: SessionTokenHash,
        _: &PublishedQuestionRevisionTuple,
        _: question_model::BloomClassificationEditNumber,
        _: BloomCognitiveProcess,
        _: BloomKnowledgeDimension,
    ) -> Result<question_model::BloomClassificationView, StoreError> {
        Err(StoreError::Unavailable(
            "not used by this contract".to_string(),
        ))
    }
}

#[tokio::test]
async fn exact_question_routes_reject_a_wrong_checksum_character_before_lookup() {
    let store = LookupCountingStore(AtomicUsize::new(0));

    assert_eq!(
        verified_question_id("0000-4000")
            .expect("documented checksum vector")
            .to_string(),
        "0000-4000"
    );
    assert_eq!(
        load_verified_question_library_entry(
            &store,
            SessionTokenHash::compute(b"instructor session"),
            "0000-N00N",
        )
        .await
        .expect("invalid checksum is concealed before any Store failure"),
        None
    );
    assert_eq!(store.0.load(Ordering::SeqCst), 0);
}

#[test]
fn availability_transitions_require_one_canonical_strong_edit_number() {
    let mut headers = HeaderMap::new();
    headers.insert(IF_MATCH, HeaderValue::from_static("\"7\""));
    assert_eq!(
        expected_availability_edit_number(&headers)
            .expect("canonical availability edit number")
            .value(),
        7
    );
    headers.insert(IF_MATCH, HeaderValue::from_static("\"07\""));
    assert!(expected_availability_edit_number(&headers).is_err());
    headers.insert(IF_MATCH, HeaderValue::from_static("W/\"7\""));
    assert!(expected_availability_edit_number(&headers).is_err());
}

#[test]
fn question_search_query_accepts_repeated_filter_values() {
    let uri: Uri = concat!(
        "/api/questions/search?backends=ple&backends=webwork",
        "&author_names=Ada&author_names=Grace",
        "&tags=protein&tags=structure",
        "&question_types=multipleChoice&question_types=fillInBlank",
        "&capabilities=hints&capabilities=serverGrading",
        "&question_licenses=CC-BY-4.0&question_licenses=CC0-1.0",
        "&bloom_cognitive_process=Analyze",
        "&bloom_knowledge_dimension=Conceptual%20Knowledge"
    )
    .parse()
    .expect("test URI parses");

    let query = Query::<QuestionSearchQuery>::try_from_uri(&uri)
        .expect("repeated filters decode")
        .0;
    let request = QuestionSearchRequest::try_from(query).expect("valid query request");

    assert_eq!(
        request.backends,
        vec![QuestionBackend::Ple, QuestionBackend::Webwork]
    );
    assert_eq!(request.author_names, vec!["ada", "grace"]);
    assert_eq!(request.tags, vec!["protein", "structure"]);
    assert_eq!(
        request.bloom_cognitive_process,
        Some(BloomCognitiveProcess::Analyze)
    );
    assert_eq!(
        request.bloom_knowledge_dimension,
        Some(BloomKnowledgeDimension::ConceptualKnowledge)
    );
    assert_eq!(request.page_size, Some(DEFAULT_PAGE_SIZE));
}

#[test]
fn question_search_query_accepts_single_filter_values_and_defaults() {
    let uri: Uri = concat!(
        "/api/questions/search?backends=ple&author_names=Ada&tags=protein",
        "&question_types=multipleChoice&capabilities=hints",
        "&question_licenses=CC-BY-4.0&sort=publishedNewest"
    )
    .parse()
    .expect("test URI parses");

    let query = Query::<QuestionSearchQuery>::try_from_uri(&uri)
        .expect("single filters decode")
        .0;
    let request = QuestionSearchRequest::try_from(query).expect("valid query request");

    assert_eq!(request.backends, vec![QuestionBackend::Ple]);
    assert_eq!(request.author_names, vec!["ada"]);
    assert_eq!(request.tags, vec!["protein"]);
    assert_eq!(request.sort, QuestionSearchSort::PublishedNewest);

    let default_uri: Uri = "/api/questions/search".parse().expect("test URI parses");
    let default_query = Query::<QuestionSearchQuery>::try_from_uri(&default_uri)
        .expect("omitted filters decode")
        .0;
    let default_request =
        QuestionSearchRequest::try_from(default_query).expect("valid default query request");
    assert!(default_request.backends.is_empty());
    assert!(default_request.author_names.is_empty());
    assert_eq!(default_request.sort, QuestionSearchSort::TitleAscending);
    assert_eq!(default_request.page_size, Some(DEFAULT_PAGE_SIZE));
}

#[test]
fn question_search_page_size_accepts_250_and_rejects_outside_discovery_bounds() {
    let accepted_uri: Uri = "/api/questions/search?page_size=250"
        .parse()
        .expect("test URI parses");
    let accepted = QuestionSearchRequest::try_from(
        Query::<QuestionSearchQuery>::try_from_uri(&accepted_uri)
            .expect("transport")
            .0,
    )
    .expect("250 is within the discovery boundary");
    assert_eq!(accepted.page_size, Some(250));

    for rejected_size in [0, 251] {
        let rejected_uri: Uri = format!("/api/questions/search?page_size={rejected_size}")
            .parse()
            .expect("test URI parses");
        assert!(
            QuestionSearchRequest::try_from(
                Query::<QuestionSearchQuery>::try_from_uri(&rejected_uri)
                    .expect("transport")
                    .0,
            )
            .is_err()
        );
    }
}

#[test]
fn question_search_query_rejects_scalar_parameter_pollution_and_invalid_fields() {
    for query in [
        "text=one&text=two",
        "page_size=10&page_size=20",
        "backends=unknown",
        "unexpected=value",
        "discipline_uuid=bad",
        "cross_discipline=yes",
        "cross_discipline=false&cross_discipline=true",
        "sort=unknown",
        "sort=titleAscending&sort=publishedNewest",
        "bloom_cognitive_process=analyze",
        "bloom_cognitive_process=",
        "bloom_cognitive_process=Analyze&bloom_cognitive_process=Create",
        "bloom_knowledge_dimension=Conceptual",
        "bloom_knowledge_dimension=",
        "bloom_knowledge_dimension=Factual%20Knowledge&bloom_knowledge_dimension=Procedural%20Knowledge",
        "bloomCognitiveProcess=Analyze",
    ] {
        let uri: Uri = format!("/api/questions/search?{query}")
            .parse()
            .expect("test URI parses");
        let rejected = match Query::<QuestionSearchQuery>::try_from_uri(&uri) {
            Err(_) => true,
            Ok(Query(transport)) => QuestionSearchRequest::try_from(transport).is_err(),
        };
        assert!(rejected, "query must reject: {query}");
    }
}

#[test]
fn hierarchy_http_transport_preserves_the_tuple_and_rejects_incomplete_chains() {
    let uri: Uri = "/api/questions/search?discipline_uuid=00000000-0000-0000-0000-000000000001&subject_uuid=00000000-0000-0000-0000-000000000002&topic_uuid=00000000-0000-0000-0000-000000000003&subtopic_uuid=00000000-0000-0000-0000-000000000004&cross_discipline=true&tags=review".parse().expect("URI");
    let request = QuestionSearchRequest::try_from(
        Query::<QuestionSearchQuery>::try_from_uri(&uri)
            .expect("transport")
            .0,
    )
    .expect("chain");
    assert_eq!(request.discipline_uuid, Some(uuid::Uuid::from_u128(1)));
    assert_eq!(request.subject_uuid, Some(uuid::Uuid::from_u128(2)));
    assert_eq!(request.topic_uuid, Some(uuid::Uuid::from_u128(3)));
    assert_eq!(request.subtopic_uuid, Some(uuid::Uuid::from_u128(4)));
    assert!(request.cross_discipline);
    assert_eq!(request.tags, vec!["review"]);
    for suffix in [
        "subject_uuid=00000000-0000-0000-0000-000000000002",
        "cross_discipline=true",
    ] {
        let uri: Uri = format!("/api/questions/search?{suffix}")
            .parse()
            .expect("URI");
        assert!(
            QuestionSearchRequest::try_from(
                Query::<QuestionSearchQuery>::try_from_uri(&uri)
                    .expect("transport")
                    .0
            )
            .is_err()
        );
    }
}

const PAGE_SOURCE: &str = r#"{
  "format": "pleQuestionJson",
  "questionTitle": "Favorite color",
  "questionDescription": "Instructor-facing color-choice example.",
  "prompt": "What is my favorite color?",
  "response": {
    "kind": "singleChoice",
    "choices": [
      {"id": "blue", "text": "Blue", "feedback": "Blue is a calm choice."},
      {"id": "red", "text": "Red", "feedback": "Red is not my favorite."}
    ],
    "correctChoice": "blue"
  },
  "feedback": {"correct": "Exactly right.", "incorrect": "Try thinking of a cool color."},
  "tags": ["example"],
  "questionLicense": "CC-BY-SA-4.0",
  "questionCitation": null,
  "language": "en-US"
}"#;

struct RecordingObjects {
    inner: MemoryObjectStore,
    reads: Mutex<Vec<ObjectAddress>>,
}

#[async_trait]
impl ObjectStore for RecordingObjects {
    async fn put(&self, request: PutObject) -> Result<ObjectRecord, ObjectStoreError> {
        self.inner.put(request).await
    }

    async fn get(&self, address: &ObjectAddress) -> Result<StoredObject, ObjectStoreError> {
        self.reads
            .lock()
            .expect("object read record")
            .push(address.clone());
        self.inner.get(address).await
    }

    async fn delete(&self, address: &ObjectAddress) -> Result<(), ObjectStoreError> {
        self.inner.delete(address).await
    }

    async fn signed_url(
        &self,
        address: &ObjectAddress,
        now: Timestamp,
    ) -> Result<SignedUrl, ObjectStoreError> {
        self.inner.signed_url(address, now).await
    }
}

struct PageOnlyLibrary {
    page: PublishedQuestionLibraryEntry,
    catalog: Vec<PublishedQuestionLibraryEntry>,
    list_calls: AtomicUsize,
    search_calls: AtomicUsize,
    statistics_ids: Mutex<Vec<PublishedQuestionId>>,
    continuation: QuestionLibrarySearchCursorPosition,
}

#[async_trait]
impl QuestionLibraryStore for PageOnlyLibrary {
    async fn search_published_question_library_entries(
        &self,
        _: SessionTokenHash,
        request: learning_data_access::QuestionLibrarySearchRequest,
    ) -> Result<QuestionLibrarySearchPage, StoreError> {
        self.search_calls.fetch_add(1, Ordering::SeqCst);
        assert_eq!(request.page_size, DEFAULT_PAGE_SIZE);
        assert!(request.after.is_none());
        Ok(QuestionLibrarySearchPage {
            items: vec![self.page.clone()],
            next_position: Some(self.continuation.clone()),
            facets: empty_search_facets(),
        })
    }

    async fn list_published_question_library_entries(
        &self,
        _: SessionTokenHash,
    ) -> Result<Vec<PublishedQuestionLibraryEntry>, StoreError> {
        self.list_calls.fetch_add(1, Ordering::SeqCst);
        Ok(self.catalog.clone())
    }

    async fn load_published_question_library_entry(
        &self,
        _: SessionTokenHash,
        _: &PublishedQuestionId,
    ) -> Result<PublishedQuestionLibraryEntry, StoreError> {
        Err(StoreError::Unavailable(
            "detail lookup is not search".into(),
        ))
    }

    async fn load_published_question_revision_library_entry(
        &self,
        _: SessionTokenHash,
        _: &PublishedQuestionRevisionTuple,
    ) -> Result<PublishedQuestionLibraryEntry, StoreError> {
        Err(StoreError::Unavailable(
            "revision lookup is not search".into(),
        ))
    }

    async fn load_current_published_question_shared_metadata(
        &self,
        _: SessionTokenHash,
        _: &[PublishedQuestionId],
    ) -> Result<Vec<question_model::PublishedQuestionSharedMetadata>, StoreError> {
        Err(StoreError::Unavailable(
            "shared metadata is not search".into(),
        ))
    }

    async fn archive_published_question(
        &self,
        _: SessionTokenHash,
        _: &PublishedQuestionId,
        _: question_model::QuestionAvailabilityEditNumber,
        _: &str,
    ) -> Result<learning_data_access::PublishedQuestionAvailability, StoreError> {
        Err(StoreError::Unavailable("archive is not search".into()))
    }

    async fn restore_published_question(
        &self,
        _: SessionTokenHash,
        _: &PublishedQuestionId,
        _: question_model::QuestionAvailabilityEditNumber,
    ) -> Result<learning_data_access::PublishedQuestionAvailability, StoreError> {
        Err(StoreError::Unavailable("restore is not search".into()))
    }

    async fn correct_question_revision_bloom(
        &self,
        _: SessionTokenHash,
        _: &PublishedQuestionRevisionTuple,
        _: question_model::BloomClassificationEditNumber,
        _: BloomCognitiveProcess,
        _: BloomKnowledgeDimension,
    ) -> Result<question_model::BloomClassificationView, StoreError> {
        Err(StoreError::Unavailable(
            "bloom correction is not search".into(),
        ))
    }
}

#[async_trait]
impl QuestionLibraryPageStatistics for PageOnlyLibrary {
    async fn page_statistics(
        &self,
        _: SessionTokenHash,
        is_instructor: bool,
        question_ids: &[PublishedQuestionId],
    ) -> Result<std::collections::BTreeMap<PublishedQuestionId, QuestionStatistics>, Response> {
        assert!(is_instructor);
        *self.statistics_ids.lock().expect("statistics ids") = question_ids.to_vec();
        Ok(std::collections::BTreeMap::new())
    }
}

fn empty_search_facets() -> QuestionLibrarySearchFacets {
    QuestionLibrarySearchFacets {
        author_names: Vec::new(),
        author_names_truncated: false,
        backends: Vec::new(),
        tags: Vec::new(),
        tags_truncated: false,
        subjects: Vec::new(),
        subjects_truncated: false,
        topics: Vec::new(),
        topics_truncated: false,
        question_types: Vec::new(),
        question_licenses: Vec::new(),
        used_in_my_courses: question_model::QuestionSearchCourseUseFacet { used: 0 },
        bloom_cognitive_processes: Vec::new(),
        bloom_knowledge_dimensions: Vec::new(),
    }
}

fn question_id(value: &str) -> PublishedQuestionId {
    PublishedQuestionId::from_random_identifier(value).expect("canonical Question ID")
}

async fn stored_library_entry(
    objects: &RecordingObjects,
    identifier: &str,
) -> (PublishedQuestionLibraryEntry, ObjectAddress) {
    let published_question_id = question_id(identifier);
    let published_question_revision_tuple = PublishedQuestionRevisionTuple {
        published_question_id: published_question_id.clone(),
        revision_number: QuestionRevisionNumber::new(1).expect("revision"),
    };
    let object_id = ObjectId::generate();
    let address = ObjectAddress::QuestionSource {
        published_question_revision_tuple: published_question_revision_tuple.clone(),
        object_id,
    };
    let record = objects
        .put(PutObject {
            address: address.clone(),
            bytes: PAGE_SOURCE.as_bytes().to_vec(),
            media_type: adapter_ple::question_json::PLE_QUESTION_JSON_MEDIA_TYPE.to_string(),
            created_at: Timestamp::from_unix_millis(1_700_000_000_000),
        })
        .await
        .expect("source object");
    let entry = PublishedQuestionLibraryEntry {
        published_question_revision_tuple,
        backend: QuestionBackend::Ple,
        question_format: QuestionFormat::PleQuestionJson,
        question_type: QuestionType::MultipleChoice,
        published_at: Timestamp::from_unix_millis(1_700_000_000_000),
        bloom: None,
        question_title: "Favorite color".to_string(),
        question_description: "Instructor-facing color-choice example.".to_string(),
        shared_metadata: PublishedQuestionSharedMetadata {
            question_id: published_question_id,
            metadata_edit_number: 1,
            tags: vec![Tag::new("example")],
            discipline_uuid: uuid::Uuid::nil(),
            subject_uuid: uuid::Uuid::nil(),
            topic_uuid: None,
            subtopic_uuid: None,
        },
        subject_name: "Colors".to_string(),
        topic_name: None,
        discipline_name: "Biology".to_string(),
        discipline_is_retired: false,
        subtopic_name: None,
        used_in_current_account_courses: false,
        authorship: QuestionAuthorship::new(vec![QuestionAuthor {
            display_name: QuestionAuthorDisplayName::new("Ada".to_string()).expect("author"),
        }])
        .expect("authorship"),
        authored_by_current_account: false,
        viewer_may_archive: false,
        question_license: QuestionLicense::CcBySa4_0,
        availability: QuestionAvailability::Available,
        availability_edit_number: question_model::QuestionAvailabilityEditNumber::INITIAL,
        source_object_id: object_id,
        source_object_checksum: SourceObjectChecksum::parse(record.sha256.to_string())
            .expect("checksum"),
        source_media_type: adapter_ple::question_json::PLE_QUESTION_JSON_MEDIA_TYPE.to_string(),
        webwork_pg_path: None,
    };
    (entry, address)
}

#[tokio::test]
async fn question_search_resolves_native_source_only_for_the_returned_page() {
    let objects = RecordingObjects {
        inner: MemoryObjectStore::default(),
        reads: Mutex::new(Vec::new()),
    };
    let (page_entry, page_address) = stored_library_entry(&objects, "0000000").await;
    let (off_page_entry, off_page_address) = stored_library_entry(&objects, "0000001").await;
    objects.reads.lock().expect("reads").clear();
    let page_id = page_entry
        .published_question_revision_tuple
        .published_question_id
        .clone();
    let store = PageOnlyLibrary {
        continuation: QuestionLibrarySearchCursorPosition::TitleAscending {
            title: page_entry.question_title.clone(),
            question_id: page_id.clone(),
        },
        page: page_entry,
        catalog: vec![off_page_entry],
        list_calls: AtomicUsize::new(0),
        search_calls: AtomicUsize::new(0),
        statistics_ids: Mutex::new(Vec::new()),
    };
    let query = QuestionSearchRequest::try_from(
        Query::<QuestionSearchQuery>::try_from_uri(
            &"/api/questions/search".parse::<Uri>().expect("URI"),
        )
        .expect("query")
        .0,
    )
    .expect("normalized search");
    let session = SessionTokenHash::compute(b"instructor session");

    let rejected = search_question_library(
        &store,
        &objects,
        session,
        true,
        QuestionSearchRequest {
            cursor: Some("not-a-cursor".to_string()),
            ..query.clone()
        },
    )
    .await;
    assert_eq!(rejected.status(), StatusCode::BAD_REQUEST);
    assert_eq!(store.search_calls.load(Ordering::SeqCst), 0);
    assert_eq!(store.list_calls.load(Ordering::SeqCst), 0);
    assert!(objects.reads.lock().expect("reads").is_empty());

    let response = search_question_library(&store, &objects, session, true, query.clone()).await;
    assert_eq!(response.status(), StatusCode::OK);
    let bytes = to_bytes(response.into_body(), 65_536)
        .await
        .expect("page body");
    let page: QuestionSearchPage = serde_json::from_slice(&bytes).expect("existing page shape");
    assert_eq!(page.items.len(), 1);
    assert_eq!(page.items[0].summary.question_id, page_id);
    assert_eq!(page.items[0].evidence, QuestionStatistics::Unavailable);
    let decoded = paging::decode_position(&QuestionSearchRequest {
        cursor: page.next_cursor.clone(),
        ..query
    })
    .expect("continuation stays bound to this query");
    assert_eq!(decoded, Some(store.continuation.clone()));
    assert_eq!(store.search_calls.load(Ordering::SeqCst), 1);
    assert_eq!(
        store.list_calls.load(Ordering::SeqCst),
        0,
        "search must not load the catalog"
    );
    assert_eq!(
        store
            .statistics_ids
            .lock()
            .expect("statistics ids")
            .as_slice(),
        std::slice::from_ref(&page_id)
    );
    let reads = objects.reads.lock().expect("reads").clone();
    assert_eq!(reads, vec![page_address]);
    assert!(!reads.contains(&off_page_address));
}

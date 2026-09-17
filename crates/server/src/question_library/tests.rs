use async_trait::async_trait;
use axum::http::{HeaderValue, Uri};
use question_model::QuestionSearchSort;
use std::sync::atomic::{AtomicUsize, Ordering};

use super::*;

struct LookupCountingStore(AtomicUsize);

#[async_trait]
impl QuestionLibraryStore for LookupCountingStore {
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
        _: &QuestionId,
    ) -> Result<PublishedQuestionLibraryEntry, StoreError> {
        self.0.fetch_add(1, Ordering::SeqCst);
        Err(StoreError::Unavailable("lookup must not occur".to_string()))
    }

    async fn load_published_question_revision_library_entry(
        &self,
        _: SessionTokenHash,
        _: &QuestionRevisionReference,
    ) -> Result<PublishedQuestionLibraryEntry, StoreError> {
        Err(StoreError::Unavailable(
            "not used by this contract".to_string(),
        ))
    }

    async fn load_current_published_question_shared_metadata(
        &self,
        _: SessionTokenHash,
        _: &[QuestionId],
    ) -> Result<Vec<question_model::PublishedQuestionSharedMetadata>, StoreError> {
        Err(StoreError::Unavailable(
            "not used by this contract".to_string(),
        ))
    }

    async fn archive_published_question(
        &self,
        _: SessionTokenHash,
        _: &QuestionId,
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
        _: &QuestionId,
        _: question_model::QuestionAvailabilityEditNumber,
    ) -> Result<learning_data_access::PublishedQuestionAvailability, StoreError> {
        Err(StoreError::Unavailable(
            "not used by this contract".to_string(),
        ))
    }
}

#[tokio::test]
async fn exact_question_routes_reject_a_wrong_checksum_character_before_lookup() {
    let issuer = RandomQuestionIdIssuer::new();
    let store = LookupCountingStore(AtomicUsize::new(0));

    assert_eq!(
        verified_question_id(&issuer, "0000-4000")
            .expect("documented checksum vector")
            .to_string(),
        "0000-4000"
    );
    assert_eq!(
        load_verified_question_library_entry(
            &store,
            &issuer,
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
        "&question_licenses=CC-BY-4.0&question_licenses=CC0-1.0"
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
    ] {
        let uri: Uri = format!("/api/questions/search?{query}")
            .parse()
            .expect("test URI parses");
        assert!(
            Query::<QuestionSearchQuery>::try_from_uri(&uri).is_err(),
            "query must reject: {query}"
        );
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

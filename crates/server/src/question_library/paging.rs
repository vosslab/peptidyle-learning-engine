//! Stateless, query-bound Question Library continuation tokens.

use std::cmp::Ordering;

use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use question_model::{
    MAX_QUESTION_SEARCH_CURSOR_ENCODED_BYTES, QuestionId, QuestionSearchFilter,
    QuestionSearchRequest, QuestionSearchSort,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use super::{DEFAULT_PAGE_SIZE, ResolvedQuestionLibraryEntry};

const CURSOR_VERSION: u8 = 2;
// A Question Title permits 512 Unicode scalars. JSON escaping uses up to six
// bytes per scalar, with fixed cursor fields remaining within this bound.
const MAX_CURSOR_BYTES: usize = 4_096;

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Cursor {
    version: u8,
    query_digest: [u8; 32],
    position: CursorPosition,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "sort", rename_all = "camelCase")]
enum CursorPosition {
    TitleAscending {
        title: String,
        question_id: QuestionId,
    },
    PublishedNewest {
        published_at_millis: i64,
        question_id: QuestionId,
    },
}

/// Selects one stable, visibly ordered page from an already authorized,
/// answer-free query snapshot. The cursor is only a query binding and position;
/// ordinary session authorization remains required for every request.
pub(super) fn page<'entry>(
    matching: &mut Vec<&'entry ResolvedQuestionLibraryEntry>,
    query: &QuestionSearchRequest,
) -> Result<(Vec<&'entry ResolvedQuestionLibraryEntry>, Option<String>), ()> {
    matching.sort_by(|left, right| compare_entries(left, right, query.sort));
    let cursor = query
        .cursor
        .as_deref()
        .map(|value| decode_cursor(value, query))
        .transpose()?;
    let page_size = usize::from(query.page_size.unwrap_or(DEFAULT_PAGE_SIZE));
    let after_cursor = matching.iter().copied().filter(|entry| {
        cursor
            .as_ref()
            .is_none_or(|cursor| compare_entry_to_cursor(entry, cursor, query.sort).is_gt())
    });
    let page = after_cursor.take(page_size + 1).collect::<Vec<_>>();
    let has_next_page = page.len() > page_size;
    let items = page.into_iter().take(page_size).collect::<Vec<_>>();
    let next_cursor = has_next_page
        .then(|| items.last().map(|entry| encode_cursor(entry, query)))
        .flatten();
    Ok((items, next_cursor))
}

fn compare_entries(
    left: &ResolvedQuestionLibraryEntry,
    right: &ResolvedQuestionLibraryEntry,
    sort: QuestionSearchSort,
) -> Ordering {
    match sort {
        QuestionSearchSort::TitleAscending => left
            .summary
            .metadata
            .question_title
            .cmp(&right.summary.metadata.question_title)
            .then_with(|| left.summary.question_id.cmp(&right.summary.question_id)),
        QuestionSearchSort::PublishedNewest => right
            .summary
            .published_at
            .cmp(&left.summary.published_at)
            .then_with(|| left.summary.question_id.cmp(&right.summary.question_id)),
    }
}

fn compare_entry_to_cursor(
    entry: &ResolvedQuestionLibraryEntry,
    cursor: &Cursor,
    sort: QuestionSearchSort,
) -> Ordering {
    match (sort, &cursor.position) {
        (
            QuestionSearchSort::TitleAscending,
            CursorPosition::TitleAscending { title, question_id },
        ) => entry
            .summary
            .metadata
            .question_title
            .cmp(title)
            .then_with(|| entry.summary.question_id.cmp(question_id)),
        (
            QuestionSearchSort::PublishedNewest,
            CursorPosition::PublishedNewest {
                published_at_millis,
                question_id,
            },
        ) => published_at_millis
            .cmp(&entry.summary.published_at.as_unix_millis())
            .then_with(|| entry.summary.question_id.cmp(question_id)),
        _ => Ordering::Equal,
    }
}

fn encode_cursor(entry: &ResolvedQuestionLibraryEntry, query: &QuestionSearchRequest) -> String {
    let position = match query.sort {
        QuestionSearchSort::TitleAscending => CursorPosition::TitleAscending {
            title: entry.summary.metadata.question_title.clone(),
            question_id: entry.summary.question_id.clone(),
        },
        QuestionSearchSort::PublishedNewest => CursorPosition::PublishedNewest {
            published_at_millis: entry.summary.published_at.as_unix_millis(),
            question_id: entry.summary.question_id.clone(),
        },
    };
    let cursor = Cursor {
        version: CURSOR_VERSION,
        query_digest: query_digest(query),
        position,
    };
    let bytes = serde_json::to_vec(&cursor).expect("Question Library cursor serializes");
    URL_SAFE_NO_PAD.encode(bytes)
}

fn decode_cursor(value: &str, query: &QuestionSearchRequest) -> Result<Cursor, ()> {
    // ASVS V2.2: bound untrusted transport before decoding it into an allocation.
    if value.len() > MAX_QUESTION_SEARCH_CURSOR_ENCODED_BYTES {
        return Err(());
    }
    let bytes = URL_SAFE_NO_PAD.decode(value).map_err(|_| ())?;
    if bytes.len() > MAX_CURSOR_BYTES {
        return Err(());
    }
    let cursor = serde_json::from_slice::<Cursor>(&bytes).map_err(|_| ())?;
    let valid_position = match (query.sort, &cursor.position) {
        (QuestionSearchSort::TitleAscending, CursorPosition::TitleAscending { title, .. }) => {
            !title.is_empty()
        }
        (QuestionSearchSort::PublishedNewest, CursorPosition::PublishedNewest { .. }) => true,
        _ => false,
    };
    if cursor.version != CURSOR_VERSION
        || cursor.query_digest != query_digest(query)
        || !valid_position
        || URL_SAFE_NO_PAD.encode(&bytes) != value
    {
        return Err(());
    }
    Ok(cursor)
}

fn query_digest(query: &QuestionSearchRequest) -> [u8; 32] {
    let filter = QuestionSearchFilter::from_query(query.clone())
        .expect("normalized Question Library query remains a valid filter");
    Sha256::digest(serde_json::to_vec(&filter).expect("Question Library filter serializes")).into()
}

#[cfg(test)]
mod tests {
    use super::*;
    use question_model::{
        QuestionAuthor, QuestionAuthorDisplayName, QuestionAuthorship, QuestionAvailability,
        QuestionBackend, QuestionBackendCapabilities, QuestionLicense, QuestionMetadata,
        QuestionRevisionNumber, QuestionRevisionReference, QuestionSummary, QuestionType,
        Timestamp, validate_question_title,
    };

    fn test_question_id(identifier: &str) -> QuestionId {
        QuestionId::from_random_identifier(identifier).expect("canonical question ID")
    }

    fn entry(title: &str, identifier: &str) -> ResolvedQuestionLibraryEntry {
        let question_id = test_question_id(identifier);
        ResolvedQuestionLibraryEntry {
            summary: QuestionSummary {
                question_id: question_id.clone(),
                latest_question_revision: QuestionRevisionReference {
                    question_id,
                    revision_number: QuestionRevisionNumber::new(1).expect("positive revision"),
                },
                backend: QuestionBackend::Ple,
                question_format: question_model::QuestionFormat::PleQuestionJson,
                question_type: QuestionType::MultipleChoice,
                capabilities: QuestionBackendCapabilities::none(),
                metadata: QuestionMetadata {
                    question_title: title.to_string(),
                    question_description: "Question Library paging test".to_string(),
                    tags: Vec::new(),
                    question_license: Some(QuestionLicense::Cc0_1_0),
                    question_citation: None,
                    language: "en".to_string(),
                },
                authorship: QuestionAuthorship::new(vec![QuestionAuthor {
                    display_name: QuestionAuthorDisplayName::new("Test Author".to_string())
                        .expect("valid author"),
                }])
                .expect("valid authorship"),
                availability: QuestionAvailability::Available,
                published_at: Timestamp::from_unix_millis(0),
                bloom: question_model::BloomClassificationView {
                    cognitive_process: question_model::BloomCognitiveProcess::Understand,
                    knowledge_dimension:
                        question_model::BloomKnowledgeDimension::ConceptualKnowledge,
                    classification_edit_number:
                        question_model::BloomClassificationEditNumber::INITIAL,
                },
            },
            prompt: Vec::new(),
            response_preview: None,
            authored_by_current_account: false,
            used_in_current_account_courses: false,
            subject: None,
            topic: None,
            discipline: None,
            discipline_is_retired: false,
            subtopic: None,
            classification: question_model::PublishedQuestionSharedMetadata {
                question_id: test_question_id("0000085"),
                metadata_edit_number: 1,
                tags: Vec::new(),
                discipline_uuid: uuid::Uuid::from_u128(1),
                subject_uuid: uuid::Uuid::from_u128(2),
                topic_uuid: None,
                subtopic_uuid: None,
            },
        }
    }

    fn published_entry(
        title: &str,
        question_id: &str,
        published_at_millis: i64,
    ) -> ResolvedQuestionLibraryEntry {
        let mut entry = entry(title, question_id);
        entry.summary.published_at = Timestamp::from_unix_millis(published_at_millis);
        entry
    }

    #[test]
    fn continuation_returns_the_remaining_title_and_id_ordered_results() {
        let entries = [
            entry("Beta", "0000085"),
            entry("Alpha", "0000002"),
            entry("Beta", "0000024"),
        ];
        let first_query = QuestionSearchRequest {
            page_size: Some(2),
            ..QuestionSearchRequest::default()
        }
        .normalized()
        .expect("query normalizes");
        let (first, cursor) =
            page(&mut entries.iter().collect(), &first_query).expect("first page succeeds");
        assert_eq!(
            first
                .iter()
                .map(|entry| entry.summary.metadata.question_title.as_str())
                .collect::<Vec<_>>(),
            vec!["Alpha", "Beta"]
        );
        let second_query = QuestionSearchRequest {
            cursor,
            ..first_query
        };
        let (second, next_cursor) =
            page(&mut entries.iter().collect(), &second_query).expect("continuation succeeds");
        assert_eq!(second.len(), 1);
        assert_eq!(second[0].summary.question_id, test_question_id("0000085"));
        assert!(next_cursor.is_none());
    }

    #[test]
    fn continuation_accepts_a_maximum_unicode_question_title() {
        let maximum_title = "\u{1F9EC}".repeat(512);
        validate_question_title(&maximum_title).expect("maximum title is valid");
        let entries = [entry(&maximum_title, "0000085"), entry("Zeta", "0000002")];
        let first_query = QuestionSearchRequest {
            page_size: Some(1),
            ..QuestionSearchRequest::default()
        }
        .normalized()
        .expect("query normalizes");
        let (first, cursor) =
            page(&mut entries.iter().collect(), &first_query).expect("first page succeeds");
        assert_eq!(first[0].summary.question_id, test_question_id("0000002"));
        assert!(
            cursor
                .as_ref()
                .is_some_and(|value| value.len() <= MAX_QUESTION_SEARCH_CURSOR_ENCODED_BYTES)
        );
        let second_query = QuestionSearchRequest {
            cursor,
            ..first_query
        };
        let (second, next_cursor) =
            page(&mut entries.iter().collect(), &second_query).expect("continuation succeeds");
        assert_eq!(second[0].summary.metadata.question_title, maximum_title);
        assert!(next_cursor.is_none());
    }

    #[test]
    fn cursor_rejects_a_different_normalized_query() {
        let original = QuestionSearchRequest {
            text: Some("genes".to_string()),
            ..QuestionSearchRequest::default()
        }
        .normalized()
        .expect("query normalizes");
        let changed = QuestionSearchRequest {
            text: Some("proteins".to_string()),
            ..QuestionSearchRequest::default()
        }
        .normalized()
        .expect("query normalizes");
        let value = encode_cursor(&entry("Gene question", "0000085"), &original);

        assert!(decode_cursor(&value, &changed).is_err());
    }

    #[test]
    fn cursor_binds_both_bloom_filters_independently() {
        let original = QuestionSearchRequest {
            bloom_cognitive_process: Some(question_model::BloomCognitiveProcess::Analyze),
            bloom_knowledge_dimension: Some(
                question_model::BloomKnowledgeDimension::ConceptualKnowledge,
            ),
            ..QuestionSearchRequest::default()
        };
        let value = encode_cursor(&entry("Gene question", "0000085"), &original);
        for changed in [
            QuestionSearchRequest {
                bloom_cognitive_process: Some(question_model::BloomCognitiveProcess::Evaluate),
                ..original.clone()
            },
            QuestionSearchRequest {
                bloom_knowledge_dimension: Some(
                    question_model::BloomKnowledgeDimension::ProceduralKnowledge,
                ),
                ..original.clone()
            },
        ] {
            assert!(decode_cursor(&value, &changed).is_err());
        }
    }

    #[test]
    fn bloom_query_facets_count_the_whole_match_beyond_the_first_page() {
        let mut first = entry("Alpha", "0000085");
        first.summary.bloom.cognitive_process = question_model::BloomCognitiveProcess::Analyze;
        let mut second = entry("Beta", "0000002");
        second.summary.bloom.cognitive_process = question_model::BloomCognitiveProcess::Analyze;
        let mut other_process = entry("Gamma", "0000024");
        other_process.summary.bloom.cognitive_process =
            question_model::BloomCognitiveProcess::Evaluate;
        let mut other_knowledge = entry("Delta", "0000042");
        other_knowledge.summary.bloom.cognitive_process =
            question_model::BloomCognitiveProcess::Analyze;
        other_knowledge.summary.bloom.knowledge_dimension =
            question_model::BloomKnowledgeDimension::ProceduralKnowledge;
        let entries = [first, second, other_process, other_knowledge];
        let query = QuestionSearchRequest {
            bloom_cognitive_process: Some(question_model::BloomCognitiveProcess::Analyze),
            bloom_knowledge_dimension: Some(
                question_model::BloomKnowledgeDimension::ConceptualKnowledge,
            ),
            page_size: Some(1),
            ..QuestionSearchRequest::default()
        };
        let text_query = super::super::search_query::QuestionTextQuery::parse(None);
        let mut matching = entries
            .iter()
            .filter(|entry| super::super::matches_query(entry, &query, &text_query))
            .collect::<Vec<_>>();
        let facets = super::super::facets::facets(&matching);
        let (items, next_cursor) = page(&mut matching, &query).expect("first page succeeds");

        assert_eq!(matching.len(), 2);
        assert_eq!(items.len(), 1);
        assert!(next_cursor.is_some());
        assert_eq!(
            facets
                .bloom_cognitive_processes
                .iter()
                .find(|facet| {
                    facet.cognitive_process == question_model::BloomCognitiveProcess::Analyze
                })
                .map(|facet| facet.count),
            Some(2)
        );
        assert_eq!(
            facets
                .bloom_knowledge_dimensions
                .iter()
                .find(|facet| {
                    facet.knowledge_dimension
                        == question_model::BloomKnowledgeDimension::ConceptualKnowledge
                })
                .map(|facet| facet.count),
            Some(2)
        );
    }

    #[test]
    fn newest_publication_order_pages_equal_timestamps_by_question_id() {
        let entries = [
            published_entry("Old", "0000085", 100),
            published_entry("Same timestamp B", "0000024", 200),
            published_entry("Same timestamp A", "0000002", 200),
        ];
        let first_query = QuestionSearchRequest {
            sort: QuestionSearchSort::PublishedNewest,
            page_size: Some(2),
            ..QuestionSearchRequest::default()
        }
        .normalized()
        .expect("query normalizes");
        let (first, cursor) =
            page(&mut entries.iter().collect(), &first_query).expect("first page succeeds");
        assert_eq!(
            first
                .iter()
                .map(|entry| entry.summary.question_id.to_string())
                .collect::<Vec<_>>(),
            vec![
                test_question_id("0000002").to_string(),
                test_question_id("0000024").to_string(),
            ]
        );

        let second_query = QuestionSearchRequest {
            cursor,
            ..first_query
        };
        let (second, next_cursor) =
            page(&mut entries.iter().collect(), &second_query).expect("continuation succeeds");
        assert_eq!(second[0].summary.question_id, test_question_id("0000085"));
        assert!(next_cursor.is_none());
    }

    #[test]
    fn continuation_rejects_a_changed_visible_sort() {
        let original = QuestionSearchRequest::default()
            .normalized()
            .expect("default query normalizes");
        let value = encode_cursor(&entry("Gene", "0000085"), &original);
        let changed = QuestionSearchRequest {
            sort: QuestionSearchSort::PublishedNewest,
            ..original
        };

        assert!(decode_cursor(&value, &changed).is_err());
    }

    #[test]
    fn continuation_binds_each_classification_identity_and_cross_mode() {
        let original = QuestionSearchRequest {
            discipline_uuid: Some(uuid::Uuid::from_u128(1)),
            subject_uuid: Some(uuid::Uuid::from_u128(2)),
            topic_uuid: Some(uuid::Uuid::from_u128(3)),
            subtopic_uuid: Some(uuid::Uuid::from_u128(4)),
            ..QuestionSearchRequest::default()
        }
        .normalized()
        .expect("valid full hierarchy");
        let value = encode_cursor(&entry("Gene", "0000085"), &original);
        assert!(decode_cursor(&value, &original).is_ok());
        for field in 0..5 {
            let mut changed = original.clone();
            match field {
                0 => changed.discipline_uuid = Some(uuid::Uuid::from_u128(10)),
                1 => changed.subject_uuid = Some(uuid::Uuid::from_u128(20)),
                2 => changed.topic_uuid = Some(uuid::Uuid::from_u128(30)),
                3 => changed.subtopic_uuid = Some(uuid::Uuid::from_u128(40)),
                _ => changed.cross_discipline = true,
            }
            assert!(decode_cursor(&value, &changed).is_err());
        }
    }

    #[test]
    fn classification_fields_keep_phrases_exclusions_exact_id_and_cross_intersection() {
        use crate::question_library::{matches_query, search_query::QuestionTextQuery};
        let mut row = entry("Gene", "0000085");
        row.discipline = Some("Biology".into());
        row.subtopic = Some("X-linked recessive crosses".into());
        let first_question_id = test_question_id("0000085").to_string();
        let second_question_id = test_question_id("0000002").to_string();
        let query = QuestionSearchRequest {
            discipline_uuid: Some(uuid::Uuid::from_u128(10)),
            subject_uuid: Some(row.classification.subject_uuid),
            cross_discipline: true,
            ..QuestionSearchRequest::default()
        };
        for (text, expected) in [
            (
                "discipline:biology subtopic:\"x-linked recessive crosses\"",
                true,
            ),
            ("discipline:chemistry", false),
            ("-subtopic:\"x-linked recessive crosses\"", false),
            ("subtopic:\"recessive x-linked\"", false),
            (&first_question_id, true),
            (&second_question_id, false),
        ] {
            assert_eq!(
                matches_query(&&row, &query, &QuestionTextQuery::parse(Some(text))),
                expected,
                "{text}"
            );
        }
    }
}

//! Stateless, query-bound Question Library continuation tokens.

use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use question_model::{
    MAX_QUESTION_SEARCH_CURSOR_ENCODED_BYTES, QuestionId, QuestionSearchFilter,
    QuestionSearchRequest,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use super::{DEFAULT_PAGE_SIZE, ResolvedQuestionLibraryEntry};

const CURSOR_VERSION: u8 = 1;
// A Question Title permits 512 Unicode scalars. JSON escaping uses up to six
// bytes per scalar, with fixed cursor fields remaining within this bound.
const MAX_CURSOR_BYTES: usize = 4_096;

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Cursor {
    version: u8,
    query_digest: [u8; 32],
    title: String,
    question_id: QuestionId,
}

/// Selects one stable title-and-ID ordered page from an already authorized,
/// answer-free query snapshot. The cursor is only a query binding and position;
/// ordinary session authorization remains required for every request.
pub(super) fn page<'entry>(
    matching: &mut Vec<&'entry ResolvedQuestionLibraryEntry>,
    query: &QuestionSearchRequest,
) -> Result<(Vec<&'entry ResolvedQuestionLibraryEntry>, Option<String>), ()> {
    matching.sort_by(|left, right| entry_key(left).cmp(&entry_key(right)));
    let cursor = query
        .cursor
        .as_deref()
        .map(|value| decode_cursor(value, query))
        .transpose()?;
    let page_size = usize::from(query.page_size.unwrap_or(DEFAULT_PAGE_SIZE));
    let after_cursor = matching.iter().copied().filter(|entry| {
        cursor
            .as_ref()
            .is_none_or(|cursor| entry_key(entry) > cursor_key(cursor))
    });
    let page = after_cursor.take(page_size + 1).collect::<Vec<_>>();
    let has_next_page = page.len() > page_size;
    let items = page.into_iter().take(page_size).collect::<Vec<_>>();
    let next_cursor = has_next_page
        .then(|| items.last().map(|entry| encode_cursor(entry, query)))
        .flatten();
    Ok((items, next_cursor))
}

fn entry_key(entry: &ResolvedQuestionLibraryEntry) -> (&str, &QuestionId) {
    (
        entry.summary.metadata.question_title.as_str(),
        &entry.summary.question_id,
    )
}

fn cursor_key(cursor: &Cursor) -> (&str, &QuestionId) {
    (&cursor.title, &cursor.question_id)
}

fn encode_cursor(entry: &ResolvedQuestionLibraryEntry, query: &QuestionSearchRequest) -> String {
    let cursor = Cursor {
        version: CURSOR_VERSION,
        query_digest: query_digest(query),
        title: entry.summary.metadata.question_title.clone(),
        question_id: entry.summary.question_id.clone(),
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
    if cursor.version != CURSOR_VERSION
        || cursor.query_digest != query_digest(query)
        || cursor.title.is_empty()
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

    fn entry(title: &str, question_id: &str) -> ResolvedQuestionLibraryEntry {
        let question_id: QuestionId = question_id.parse().expect("canonical question ID");
        ResolvedQuestionLibraryEntry {
            summary: QuestionSummary {
                question_id: question_id.clone(),
                latest_question_revision: QuestionRevisionReference {
                    question_id,
                    revision_number: QuestionRevisionNumber::new(1).expect("positive revision"),
                },
                backend: QuestionBackend::Ple,
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
            },
            prompt: Vec::new(),
            authored_by_current_account: false,
        }
    }

    #[test]
    fn continuation_returns_the_remaining_title_and_id_ordered_results() {
        let entries = [
            entry("Beta", "000-000N"),
            entry("Alpha", "000-001P"),
            entry("Beta", "000-002R"),
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
        assert_eq!(second[0].summary.question_id.to_string(), "000-002R");
        assert!(next_cursor.is_none());
    }

    #[test]
    fn continuation_accepts_a_maximum_unicode_question_title() {
        let maximum_title = "🧬".repeat(512);
        validate_question_title(&maximum_title).expect("maximum title is valid");
        let entries = [entry(&maximum_title, "000-000N"), entry("Zeta", "000-001P")];
        let first_query = QuestionSearchRequest {
            page_size: Some(1),
            ..QuestionSearchRequest::default()
        }
        .normalized()
        .expect("query normalizes");
        let (first, cursor) =
            page(&mut entries.iter().collect(), &first_query).expect("first page succeeds");
        assert_eq!(first[0].summary.question_id.to_string(), "000-001P");
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
        let cursor = Cursor {
            version: CURSOR_VERSION,
            query_digest: query_digest(&original),
            title: "Gene question".to_string(),
            question_id: "000-000N".parse().expect("canonical question ID"),
        };
        let value = URL_SAFE_NO_PAD.encode(serde_json::to_vec(&cursor).expect("cursor serializes"));

        assert!(decode_cursor(&value, &changed).is_err());
    }
}

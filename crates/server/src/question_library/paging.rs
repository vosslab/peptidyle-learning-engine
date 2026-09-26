//! Stateless, query-bound Question Library continuation tokens.

use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use learning_data_access::QuestionLibrarySearchCursorPosition;
use question_model::{
    MAX_QUESTION_SEARCH_CURSOR_ENCODED_BYTES, PublishedQuestionId, QuestionSearchFilter,
    QuestionSearchRequest, QuestionSearchSort,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

const CURSOR_VERSION: u8 = 2;
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
        question_id: PublishedQuestionId,
    },
    PublishedNewest {
        published_at_millis: i64,
        question_id: PublishedQuestionId,
    },
}

/// Decodes a cursor before the route requests any source material.
pub(super) fn decode_position(
    query: &QuestionSearchRequest,
) -> Result<Option<QuestionLibrarySearchCursorPosition>, ()> {
    query
        .cursor
        .as_deref()
        .map(|value| decode_cursor(value, query))
        .transpose()
        .map(|cursor| {
            cursor.map(|cursor| match cursor.position {
                CursorPosition::TitleAscending { title, question_id } => {
                    QuestionLibrarySearchCursorPosition::TitleAscending { title, question_id }
                }
                CursorPosition::PublishedNewest {
                    published_at_millis,
                    question_id,
                } => QuestionLibrarySearchCursorPosition::PublishedNewest {
                    published_at_millis,
                    question_id,
                },
            })
        })
}

/// Retains the established opaque cursor envelope while encoding the Store's
/// validated continuation position.
pub(super) fn encode_position(
    position: QuestionLibrarySearchCursorPosition,
    query: &QuestionSearchRequest,
) -> String {
    let position = match position {
        QuestionLibrarySearchCursorPosition::TitleAscending { title, question_id } => {
            CursorPosition::TitleAscending { title, question_id }
        }
        QuestionLibrarySearchCursorPosition::PublishedNewest {
            published_at_millis,
            question_id,
        } => CursorPosition::PublishedNewest {
            published_at_millis,
            question_id,
        },
    };
    let cursor = Cursor {
        version: CURSOR_VERSION,
        query_digest: query_digest(query),
        position,
    };
    URL_SAFE_NO_PAD.encode(serde_json::to_vec(&cursor).expect("Question Library cursor serializes"))
}

fn decode_cursor(value: &str, query: &QuestionSearchRequest) -> Result<Cursor, ()> {
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

    fn question_id(value: &str) -> PublishedQuestionId {
        PublishedQuestionId::from_random_identifier(value).expect("canonical Question ID")
    }

    #[test]
    fn cursor_binds_the_query_and_sort_before_store_search() {
        let query = QuestionSearchRequest::default()
            .normalized()
            .expect("query");
        let cursor = encode_position(
            QuestionLibrarySearchCursorPosition::TitleAscending {
                title: "Gene".to_owned(),
                question_id: question_id("0000085"),
            },
            &query,
        );
        assert!(
            decode_position(&QuestionSearchRequest {
                cursor: Some(cursor.clone()),
                ..query.clone()
            })
            .is_ok()
        );
        assert!(
            decode_position(&QuestionSearchRequest {
                sort: QuestionSearchSort::PublishedNewest,
                cursor: Some(cursor),
                ..query
            })
            .is_err()
        );
    }
}

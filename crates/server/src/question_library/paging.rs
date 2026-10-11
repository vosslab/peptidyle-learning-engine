//! Stateless, query-bound Question Library continuation tokens.

use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use learning_data_access::QuestionLibrarySearchCursorPosition;
use question_model::{
    LibraryObjectId, LibraryObjectSearchFilter, LibraryObjectSearchRequest,
    LibraryObjectSearchSort, MAX_QUESTION_SEARCH_CURSOR_ENCODED_BYTES,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

const CURSOR_VERSION: u8 = 3;
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
        public_id: LibraryObjectId,
    },
    PublishedNewest {
        published_at_millis: i64,
        public_id: LibraryObjectId,
    },
}

/// Decodes a cursor before the route requests any source material.
pub(super) fn decode_position(
    query: &LibraryObjectSearchRequest,
) -> Result<Option<QuestionLibrarySearchCursorPosition>, ()> {
    query
        .cursor
        .as_deref()
        .map(|value| decode_cursor(value, query))
        .transpose()
        .map(|cursor| {
            cursor.map(|cursor| match cursor.position {
                CursorPosition::TitleAscending { title, public_id } => {
                    QuestionLibrarySearchCursorPosition::TitleAscending { title, public_id }
                }
                CursorPosition::PublishedNewest {
                    published_at_millis,
                    public_id,
                } => QuestionLibrarySearchCursorPosition::PublishedNewest {
                    published_at_millis,
                    public_id,
                },
            })
        })
}

/// Retains the established opaque cursor envelope while encoding the Store's
/// validated continuation position.
pub(super) fn encode_position(
    position: QuestionLibrarySearchCursorPosition,
    query: &LibraryObjectSearchRequest,
) -> String {
    let position = match position {
        QuestionLibrarySearchCursorPosition::TitleAscending { title, public_id } => {
            CursorPosition::TitleAscending { title, public_id }
        }
        QuestionLibrarySearchCursorPosition::PublishedNewest {
            published_at_millis,
            public_id,
        } => CursorPosition::PublishedNewest {
            published_at_millis,
            public_id,
        },
    };
    let cursor = Cursor {
        version: CURSOR_VERSION,
        query_digest: query_digest(query),
        position,
    };
    URL_SAFE_NO_PAD.encode(serde_json::to_vec(&cursor).expect("Question Library cursor serializes"))
}

fn decode_cursor(value: &str, query: &LibraryObjectSearchRequest) -> Result<Cursor, ()> {
    if value.len() > MAX_QUESTION_SEARCH_CURSOR_ENCODED_BYTES {
        return Err(());
    }
    let bytes = URL_SAFE_NO_PAD.decode(value).map_err(|_| ())?;
    if bytes.len() > MAX_CURSOR_BYTES {
        return Err(());
    }
    let cursor = serde_json::from_slice::<Cursor>(&bytes).map_err(|_| ())?;
    let valid_position = match (query.sort, &cursor.position) {
        (LibraryObjectSearchSort::TitleAscending, CursorPosition::TitleAscending { title, .. }) => {
            !title.is_empty()
        }
        (LibraryObjectSearchSort::PublishedNewest, CursorPosition::PublishedNewest { .. }) => true,
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

fn query_digest(query: &LibraryObjectSearchRequest) -> [u8; 32] {
    let filter = LibraryObjectSearchFilter::from_query(query.clone())
        .expect("normalized Question Library query remains a valid filter");
    Sha256::digest(serde_json::to_vec(&filter).expect("Question Library filter serializes")).into()
}

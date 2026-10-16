//! Stateless, query-bound Question Library continuation tokens.

use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use learning_data_access::QuestionLibrarySearchCursorPosition;
use question_model::{
    LibraryObjectId, LibraryObjectSearchFilter, LibraryObjectSearchRequest,
    LibraryObjectSearchSort, MAX_QUESTION_SEARCH_CURSOR_ENCODED_BYTES,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

const CURSOR_VERSION: u8 = 4;
const MAX_CURSOR_BYTES: usize = 4_096;

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Cursor {
    version: u8,
    query_digest: [u8; 32],
    position: CursorPosition,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
enum CursorPosition {
    Text {
        sort: LibraryObjectSearchSort,
        key: String,
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
                CursorPosition::Text {
                    sort,
                    key,
                    public_id,
                } => QuestionLibrarySearchCursorPosition::Text {
                    sort: store_sort(sort),
                    key,
                    public_id,
                },
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
        QuestionLibrarySearchCursorPosition::Text {
            sort,
            key,
            public_id,
        } => CursorPosition::Text {
            sort: public_sort(sort),
            key,
            public_id,
        },
        QuestionLibrarySearchCursorPosition::PublishedNewest {
            published_at_millis,
            public_id,
        } => CursorPosition::PublishedNewest {
            published_at_millis,
            public_id,
        },
        QuestionLibrarySearchCursorPosition::StarredDate { .. } => {
            unreachable!("private Starred cursor is outside public Library transport")
        }
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
        (
            sort,
            CursorPosition::Text {
                sort: position_sort,
                key,
                ..
            },
        ) => {
            sort == *position_sort
                && sort != LibraryObjectSearchSort::PublishedNewest
                && !key.is_empty()
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

pub(super) fn store_sort(
    sort: LibraryObjectSearchSort,
) -> learning_data_access::QuestionLibrarySearchSort {
    use learning_data_access::QuestionLibrarySearchSort as Store;
    match sort {
        LibraryObjectSearchSort::TitleAscending => Store::TitleAscending,
        LibraryObjectSearchSort::TitleDescending => Store::TitleDescending,
        LibraryObjectSearchSort::KindAscending => Store::KindAscending,
        LibraryObjectSearchSort::KindDescending => Store::KindDescending,
        LibraryObjectSearchSort::DisciplineAscending => Store::DisciplineAscending,
        LibraryObjectSearchSort::DisciplineDescending => Store::DisciplineDescending,
        LibraryObjectSearchSort::QuestionTypeAscending => Store::QuestionTypeAscending,
        LibraryObjectSearchSort::QuestionTypeDescending => Store::QuestionTypeDescending,
        LibraryObjectSearchSort::PublishedNewest => Store::PublishedNewest,
    }
}

fn public_sort(sort: learning_data_access::QuestionLibrarySearchSort) -> LibraryObjectSearchSort {
    use learning_data_access::QuestionLibrarySearchSort as Store;
    match sort {
        Store::TitleAscending => LibraryObjectSearchSort::TitleAscending,
        Store::TitleDescending => LibraryObjectSearchSort::TitleDescending,
        Store::KindAscending => LibraryObjectSearchSort::KindAscending,
        Store::KindDescending => LibraryObjectSearchSort::KindDescending,
        Store::DisciplineAscending => LibraryObjectSearchSort::DisciplineAscending,
        Store::DisciplineDescending => LibraryObjectSearchSort::DisciplineDescending,
        Store::QuestionTypeAscending => LibraryObjectSearchSort::QuestionTypeAscending,
        Store::QuestionTypeDescending => LibraryObjectSearchSort::QuestionTypeDescending,
        Store::PublishedNewest => LibraryObjectSearchSort::PublishedNewest,
        Store::StarredDateAscending | Store::StarredDateDescending => {
            unreachable!("private Starred sort is outside public Library cursor")
        }
    }
}

fn query_digest(query: &LibraryObjectSearchRequest) -> [u8; 32] {
    let filter = LibraryObjectSearchFilter::from_query(query.clone())
        .expect("normalized Question Library query remains a valid filter");
    Sha256::digest(serde_json::to_vec(&filter).expect("Question Library filter serializes")).into()
}

#[cfg(test)]
mod tests {
    use super::*;
    use question_model::BloomCognitiveProcess;

    #[test]
    fn normalized_multi_value_filters_bind_cursor_digest() {
        let discipline = uuid::Uuid::parse_str("00000000-0000-0000-0000-000000000001").unwrap();
        let second = uuid::Uuid::parse_str("00000000-0000-0000-0000-000000000002").unwrap();
        let first = LibraryObjectSearchRequest {
            discipline_uuids: vec![second, discipline],
            bloom_cognitive_processes: vec![BloomCognitiveProcess::Apply],
            ..LibraryObjectSearchRequest::default()
        };
        let first = first.normalized().unwrap();
        let mut same = first.clone();
        same.discipline_uuids.reverse();
        assert_eq!(query_digest(&first), query_digest(&same));
        same.bloom_cognitive_processes
            .push(BloomCognitiveProcess::Analyze);
        assert_ne!(query_digest(&first), query_digest(&same));
    }
}

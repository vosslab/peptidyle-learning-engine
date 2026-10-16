//! Query-bound continuation for bounded Library facet choices.

use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use question_model::{LibraryFacetField, LibraryObjectSearchFilter, LibraryObjectSearchRequest};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

const VERSION: u8 = 1;
const MAX_BYTES: usize = 1024;

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Cursor {
    version: u8,
    digest: [u8; 32],
    after_key: String,
}

fn digest(
    query: &LibraryObjectSearchRequest,
    field: LibraryFacetField,
    search: Option<&str>,
) -> [u8; 32] {
    let filter = LibraryObjectSearchFilter::from_query(query.clone())
        .expect("normalized Library choice query remains a valid filter");
    let identity = serde_json::to_vec(&(filter, field, search, query.page_size))
        .expect("Library choice cursor identity serializes");
    Sha256::digest(identity).into()
}

pub(super) fn decode(
    query: &LibraryObjectSearchRequest,
    field: LibraryFacetField,
    search: Option<&str>,
) -> Result<Option<String>, ()> {
    let Some(value) = query.cursor.as_deref() else {
        return Ok(None);
    };
    if value.len() > MAX_BYTES * 2 {
        return Err(());
    }
    let bytes = URL_SAFE_NO_PAD.decode(value).map_err(|_| ())?;
    if bytes.len() > MAX_BYTES || URL_SAFE_NO_PAD.encode(&bytes) != value {
        return Err(());
    }
    let cursor: Cursor = serde_json::from_slice(&bytes).map_err(|_| ())?;
    if cursor.version != VERSION
        || cursor.after_key.is_empty()
        || cursor.after_key.len() > 256
        || cursor.digest != digest(query, field, search)
    {
        return Err(());
    }
    Ok(Some(cursor.after_key))
}

pub(super) fn encode(
    query: &LibraryObjectSearchRequest,
    field: LibraryFacetField,
    search: Option<&str>,
    after_key: String,
) -> String {
    URL_SAFE_NO_PAD.encode(
        serde_json::to_vec(&Cursor {
            version: VERSION,
            digest: digest(query, field, search),
            after_key,
        })
        .expect("Library choice cursor serializes"),
    )
}

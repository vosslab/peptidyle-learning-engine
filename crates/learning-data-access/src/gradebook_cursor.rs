//! Opaque native-key cursor for the summary-only gradebook projection.
//!
//! The transport sees one URL-safe token. Storage decodes it before querying
//! so PostgreSQL can compare the assignment/enrollment UUID tuple without a
//! text expression or a sort that defeats the page indexes.

use base64::Engine as _;
use uuid::Uuid;

use crate::{Cursor, StoreError};

const ENCODED_LENGTH: usize = 43;
const RAW_LENGTH: usize = 32;

/// Native UUID tuple used by the gradebook page order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct GradebookCursor {
    pub(crate) assignment: Uuid,
    pub(crate) enrollment: Uuid,
}

impl GradebookCursor {
    /// Creates an opaque transport cursor from the final native tuple of a page.
    pub(crate) fn encode(self) -> Cursor {
        let mut bytes = [0_u8; RAW_LENGTH];
        bytes[..16].copy_from_slice(self.assignment.as_bytes());
        bytes[16..].copy_from_slice(self.enrollment.as_bytes());
        Cursor::from_stable_key(base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(bytes))
    }

    /// Decodes the exact fixed-width cursor accepted by this query path.
    pub(crate) fn decode(cursor: &Cursor) -> Result<Self, StoreError> {
        let token = cursor.as_str();
        if token.len() != ENCODED_LENGTH {
            return Err(invalid_cursor());
        }
        let bytes = base64::engine::general_purpose::URL_SAFE_NO_PAD
            .decode(token)
            .map_err(|_| invalid_cursor())?;
        if bytes.len() != RAW_LENGTH {
            return Err(invalid_cursor());
        }
        let assignment = Uuid::from_slice(&bytes[..16]).map_err(|_| invalid_cursor())?;
        let enrollment = Uuid::from_slice(&bytes[16..]).map_err(|_| invalid_cursor())?;
        Ok(Self {
            assignment,
            enrollment,
        })
    }
}

fn invalid_cursor() -> StoreError {
    StoreError::InvalidRecord("invalid gradebook cursor".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cursor_round_trips_without_exposing_uuid_boundaries() {
        let original = GradebookCursor {
            assignment: Uuid::from_u128(1),
            enrollment: Uuid::from_u128(2),
        };
        let encoded = original.encode();

        assert_eq!(encoded.as_str().len(), ENCODED_LENGTH);
        assert!(!encoded.as_str().contains('/'));
        assert_eq!(GradebookCursor::decode(&encoded), Ok(original));
    }

    #[test]
    fn malformed_cursor_is_rejected_before_a_query_is_built() {
        assert!(matches!(
            Cursor::parse(String::new()),
            Err(crate::PaginationError::EmptyCursor)
        ));
        for token in [
            "not-a-gradebook-cursor",
            "!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!",
        ] {
            let cursor = Cursor::parse(token.to_string()).expect("nonempty generic cursor");
            assert!(matches!(
                GradebookCursor::decode(&cursor),
                Err(StoreError::InvalidRecord(message)) if message == "invalid gradebook cursor"
            ));
        }
    }
}

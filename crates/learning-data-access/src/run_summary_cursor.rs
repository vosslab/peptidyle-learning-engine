//! Canonical, run-bound keyset cursors for bounded run-summary outcomes.

use base64::Engine as _;
use objects::Sha256Digest;
use uuid::Uuid;

use crate::{Cursor, StoreError};

const VERSION: u8 = 1;
const PAYLOAD_LENGTH: usize = 37;
const RAW_LENGTH: usize = PAYLOAD_LENGTH + 32;
const ENCODED_LENGTH: usize = 92;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct RunSummaryCursor {
    pub(crate) assignment_position: u32,
    pub(crate) attempt: Uuid,
}

impl RunSummaryCursor {
    /// Binds the cursor to its exact run before adding an integrity digest. A
    /// continuation copied to another run is rejected before either store
    /// queries outcomes.
    pub(crate) fn encode(self, run: Uuid) -> Cursor {
        let mut payload = [0_u8; PAYLOAD_LENGTH];
        payload[0] = VERSION;
        payload[1..17].copy_from_slice(run.as_bytes());
        payload[17..21].copy_from_slice(&self.assignment_position.to_be_bytes());
        payload[21..].copy_from_slice(self.attempt.as_bytes());
        let mut bytes = [0_u8; RAW_LENGTH];
        bytes[..PAYLOAD_LENGTH].copy_from_slice(&payload);
        bytes[PAYLOAD_LENGTH..].copy_from_slice(Sha256Digest::compute(&payload).as_bytes());
        Cursor::from_stable_key(base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(bytes))
    }

    pub(crate) fn decode(cursor: &Cursor, run: Uuid) -> Result<Self, StoreError> {
        let token = cursor.as_str();
        if token.len() != ENCODED_LENGTH {
            return Err(invalid_cursor());
        }
        let bytes = base64::engine::general_purpose::URL_SAFE_NO_PAD
            .decode(token)
            .map_err(|_| invalid_cursor())?;
        if bytes.len() != RAW_LENGTH
            || bytes[0] != VERSION
            || bytes[1..17] != *run.as_bytes()
            || Sha256Digest::compute(&bytes[..PAYLOAD_LENGTH]).as_bytes()
                != &bytes[PAYLOAD_LENGTH..]
        {
            return Err(invalid_cursor());
        }
        let assignment_position =
            u32::from_be_bytes(bytes[17..21].try_into().map_err(|_| invalid_cursor())?);
        let attempt = Uuid::from_slice(&bytes[21..37]).map_err(|_| invalid_cursor())?;
        let decoded = Self {
            assignment_position,
            attempt,
        };
        if decoded.encode(run).as_str() != token {
            return Err(invalid_cursor());
        }
        Ok(decoded)
    }
}

fn invalid_cursor() -> StoreError {
    StoreError::InvalidRecord("invalid run summary cursor".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    const RUN: Uuid = Uuid::from_u128(2);

    #[test]
    fn cursor_round_trips_the_exact_run_bound_keyset_tuple() {
        let original = RunSummaryCursor {
            assignment_position: 51,
            attempt: Uuid::from_u128(7),
        };
        let token = original.encode(RUN);
        assert_eq!(token.as_str().len(), ENCODED_LENGTH);
        assert_eq!(RunSummaryCursor::decode(&token, RUN), Ok(original));
    }

    #[test]
    fn tampered_or_cross_run_cursor_never_becomes_a_query_key() {
        let original = RunSummaryCursor {
            assignment_position: 51,
            attempt: Uuid::from_u128(7),
        };
        let token = original.encode(RUN);
        let mut tampered = token.as_str().as_bytes().to_vec();
        tampered[10] = if tampered[10] == b'A' { b'B' } else { b'A' };
        let tampered =
            Cursor::parse(String::from_utf8(tampered).expect("ASCII token")).expect("nonempty");
        for candidate in [tampered, token.clone()] {
            assert!(matches!(
                RunSummaryCursor::decode(&candidate, Uuid::from_u128(3)),
                Err(StoreError::InvalidRecord(message)) if message == "invalid run summary cursor"
            ));
        }
        let malformed = Cursor::parse("not-a-run-summary-cursor".to_string()).expect("nonempty");
        assert!(matches!(
            RunSummaryCursor::decode(&malformed, RUN),
            Err(StoreError::InvalidRecord(message)) if message == "invalid run summary cursor"
        ));
    }

    #[test]
    fn fifty_one_keyset_rows_continue_without_duplicates() {
        let keys = (0_u32..51)
            .map(|position| RunSummaryCursor {
                assignment_position: position,
                attempt: Uuid::from_u128(u128::from(position) + 100),
            })
            .collect::<Vec<_>>();
        let first = &keys[..50];
        let after = RunSummaryCursor::decode(&first.last().expect("first page").encode(RUN), RUN)
            .expect("canonical continuation");
        let second = keys
            .iter()
            .copied()
            .filter(|key| *key > after)
            .collect::<Vec<_>>();
        assert_eq!(second, vec![keys[50]]);
    }
}

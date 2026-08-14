//! Server-side Question ID generation and validation.
//!
//! The browser can normalize syntax, but only this server-owned capability has
//! the durable HMAC key that makes the seventh character authoritative.

use hmac::{Hmac, KeyInit, Mac};
use question_model::{QUESTION_ID_ALPHABET, QuestionId};

use crate::StoreError;

const QUESTION_ID_SECRET_BYTES: usize = 32;

/// Durable server-held Question ID issuer and validator.
#[derive(Clone)]
pub struct QuestionIdCodec(Option<[u8; QUESTION_ID_SECRET_BYTES]>);

impl QuestionIdCodec {
    /// Creates the codec from one dedicated 256-bit secret.
    pub fn from_server_secret(secret: [u8; QUESTION_ID_SECRET_BYTES]) -> Self {
        Self(Some(secret))
    }

    /// Fail-closed capability for processes that never issue or resolve a
    /// human Question ID, such as a queue-only worker.
    pub fn unavailable() -> Self {
        Self(None)
    }

    /// Issues one random ID candidate. Persistence still owns collision retry
    /// and the independent 100,000,000-question product cap.
    pub fn issue(&self) -> Result<QuestionId, StoreError> {
        if self.0.is_none() {
            return Err(StoreError::Unavailable(
                "Question ID secret is unavailable".to_string(),
            ));
        }
        let mut randomness = [0_u8; 4];
        getrandom::fill(&mut randomness).map_err(|error| {
            StoreError::Unavailable(format!("Question ID randomness is unavailable: {error}"))
        })?;
        let value = u32::from_be_bytes(randomness) & 0x3fff_ffff;
        let mut identifier = [b'0'; 6];
        for (index, output) in identifier.iter_mut().enumerate() {
            let shift = (5 - index) * 5;
            *output = QUESTION_ID_ALPHABET[((value >> shift) & 0x1f) as usize];
        }
        self.issue_for_identifier(
            std::str::from_utf8(&identifier).expect("Crockford alphabet is ASCII"),
        )
    }

    /// Verifies the server-authenticated validation character.
    pub fn validates(&self, question_id: &QuestionId) -> bool {
        let identifier = question_id.identifier_compact();
        self.validation_character(&identifier) == Some(question_id.validation_character())
    }

    pub(crate) fn issue_for_identifier(&self, identifier: &str) -> Result<QuestionId, StoreError> {
        let validation = self.validation_character(identifier).ok_or_else(|| {
            StoreError::InvalidRecord("Question ID candidate is not canonical".to_string())
        })?;
        QuestionId::from_canonical_parts(identifier, validation)
            .map_err(|message| StoreError::InvalidRecord(message.to_string()))
    }

    /// The first five bits of HMAC-SHA-256 byte zero are the fixed v1 rule.
    fn validation_character(&self, identifier: &str) -> Option<char> {
        if identifier.len() != 6
            || !identifier
                .bytes()
                .all(|character| QUESTION_ID_ALPHABET.contains(&character))
        {
            return None;
        }
        let secret = self.0?;
        let mut mac = Hmac::<sha2::Sha256>::new_from_slice(&secret)
            .expect("HMAC-SHA256 accepts a 32-byte Question ID secret");
        mac.update(identifier.as_bytes());
        let digest = mac.finalize().into_bytes();
        Some(QUESTION_ID_ALPHABET[(digest[0] >> 3) as usize] as char)
    }
}

impl std::fmt::Debug for QuestionIdCodec {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("QuestionIdCodec([redacted])")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const TEST_SECRET: [u8; 32] = [0x42; 32];

    #[test]
    fn stable_vectors_fix_the_hmac_extraction_rule() {
        let codec = QuestionIdCodec::from_server_secret(TEST_SECRET);
        let first = codec
            .issue_for_identifier("7K3M9Q")
            .expect("canonical candidate issues");
        let second = codec
            .issue_for_identifier("000001")
            .expect("canonical candidate issues");
        assert_eq!(first.to_string(), "7K3-M9QP");
        assert_eq!(second.to_string(), "000-001X");
        assert!(codec.validates(&first));
        assert!(codec.validates(&second));
    }

    #[test]
    fn syntax_alone_cannot_make_a_question_id_authoritative() {
        let codec = QuestionIdCodec::from_server_secret(TEST_SECRET);
        let valid = codec
            .issue_for_identifier("ABC123")
            .expect("canonical candidate issues");
        let different_key = QuestionIdCodec::from_server_secret([0x24; 32]);
        assert!(codec.validates(&valid));
        assert!(!different_key.validates(&valid));
        assert_eq!(format!("{codec:?}"), "QuestionIdCodec([redacted])");
    }
}

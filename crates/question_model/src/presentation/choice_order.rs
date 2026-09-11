//! Deterministic native choice ordering for one issued presentation.

use sha2::{Digest, Sha256};

use crate::response::QuestionChoice;

use super::model::QuestionPresentationNonce;

/// Sorts choices by a domain-separated hash rank derived from the durable
/// nonce and each stable authored choice identifier.
///
/// Hash ranks avoid bounded-index modulo bias. The identifier is a deterministic
/// tie-breaker for the astronomically unlikely equal digest case, and means an
/// authored-vector reorder cannot change the permutation for a fixed nonce.
pub(super) fn nonce_randomized_choices(
    choices: &[QuestionChoice],
    nonce: QuestionPresentationNonce,
) -> Vec<QuestionChoice> {
    let mut randomized = choices.to_vec();
    randomized.sort_by_cached_key(|choice| {
        let mut input = b"ple:question-choice-order:v1\0".to_vec();
        input.extend_from_slice(&nonce.as_bytes());
        input.extend_from_slice(choice.id.as_str().as_bytes());
        let rank: [u8; 32] = Sha256::digest(input).into();
        (rank, choice.id.as_str().to_owned())
    });
    randomized
}

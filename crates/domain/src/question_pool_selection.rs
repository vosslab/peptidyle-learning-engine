//! Server-owned selection of current Question Pool members.
//!
//! The caller supplies transient server entropy and the complete saved Question
//! Pool Assessment Entry and its current members. This module records no entropy and reads no storage:
//! persistence owns Reuse Selection lookup, while this function creates the
//! selected Question Pool Item result for Select Again and no-store Question Pool Previews.

use question_model::{QuestionPoolAssessmentEntry, QuestionPoolSelectedItem};
use rand_chacha::ChaCha20Rng;
use rand_chacha::rand_core::{Rng, SeedableRng};

/// Opaque transient entropy supplied by a trusted server operation.
///
/// The selected Question Pool Items, rather than these bytes, become durable Student
/// Work evidence. The browser neither supplies nor receives this value.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct QuestionPoolSelectionEntropy([u8; 32]);

impl QuestionPoolSelectionEntropy {
    /// Wraps 256 bits produced by the server's cryptographically secure source.
    pub const fn from_bytes(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }
}

/// A saved Question Pool cannot produce a requested durable selection.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QuestionPoolSelectionError {
    /// A storage candidate does not belong to the Pool named by the Assessment Entry.
    CandidatePoolMismatch,
    /// Fewer current Pool members exist than the Assessment requires.
    InsufficientAvailableQuestionPoolItems {
        /// Instructor-requested Question Pool Selection Count.
        selection_count: u32,
        /// Current Pool member count at selection time.
        available_question_pool_item_count: usize,
    },
}

impl std::fmt::Display for QuestionPoolSelectionError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::CandidatePoolMismatch => formatter
                .write_str("Question Pool candidate does not belong to the Assessment Entry Pool"),
            Self::InsufficientAvailableQuestionPoolItems {
                selection_count,
                available_question_pool_item_count,
            } => write!(
                formatter,
                "Question Pool requires {selection_count} members but only {available_question_pool_item_count} exist"
            ),
        }
    }
}

impl std::error::Error for QuestionPoolSelectionError {}

/// Selects current Pool members for one new Question Pool Selection.
///
/// Membership is sampled without replacement from canonical tuple order, so
/// storage or request order cannot affect selection. The returned values carry exact Question
/// Revision Tuples and are suitable for a server-held Question Pool
/// Selection record.
pub fn select_question_pool_items(
    question_pool: &QuestionPoolAssessmentEntry,
    candidates: &[QuestionPoolSelectedItem],
    entropy: QuestionPoolSelectionEntropy,
) -> Result<Vec<QuestionPoolSelectedItem>, QuestionPoolSelectionError> {
    if candidates.iter().any(|candidate| {
        candidate.question_pool_id != question_pool.question_pool_id
            || candidate.question_pool_edit_number != question_pool.question_pool_edit_number
    }) {
        return Err(QuestionPoolSelectionError::CandidatePoolMismatch);
    }
    let selection_count = usize::try_from(question_pool.selection_count.get())
        .expect("u32 selection count fits the current supported usize targets");
    if selection_count > candidates.len() {
        return Err(
            QuestionPoolSelectionError::InsufficientAvailableQuestionPoolItems {
                selection_count: question_pool.selection_count.get(),
                available_question_pool_item_count: candidates.len(),
            },
        );
    }

    let mut canonical_candidates = candidates.to_vec();
    canonical_candidates.sort_by(|left, right| {
        left.published_question_revision_tuple
            .cmp(&right.published_question_revision_tuple)
    });
    let mut positions = (0..canonical_candidates.len()).collect::<Vec<_>>();
    let mut random = ChaCha20Rng::from_seed(entropy.0);
    for position in 0..selection_count {
        let remaining = positions.len() - position;
        let selected = position
            + usize::try_from(sample_below(&mut random, remaining as u64))
                .expect("selected Question Pool position fits usize");
        positions.swap(position, selected);
    }
    positions.truncate(selection_count);
    Ok(positions
        .into_iter()
        .map(|position| canonical_candidates[position].clone())
        .collect())
}

/// Samples `0..upper` without modulo bias.
fn sample_below(random: &mut ChaCha20Rng, upper: u64) -> u64 {
    debug_assert!(upper > 0);
    let rejection_threshold = upper.wrapping_neg() % upper;
    loop {
        let random_value = random.next_u64();
        if random_value >= rejection_threshold {
            return random_value % upper;
        }
    }
}

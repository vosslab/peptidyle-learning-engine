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

#[cfg(test)]
mod tests {
    use question_model::{
        AssessmentEntryAvailability, AssessmentEntryId, AssessmentEntryScoringRule,
        AssessmentPointValue, PublishedQuestionId, PublishedQuestionRevisionTuple,
        QuestionAttemptLimit, QuestionAttemptTimeLimit, QuestionPoolEditNumber, QuestionPoolId,
        QuestionRevisionNumber,
    };
    use uuid::Uuid;

    use super::*;

    fn pool_member(number: u32) -> QuestionPoolSelectedItem {
        QuestionPoolSelectedItem {
            question_pool_id: QuestionPoolId::from_random_identifier("7K3M9QP").expect("Pool ID"),
            question_pool_edit_number: QuestionPoolEditNumber::new(1).expect("edit number"),
            published_question_revision_tuple: PublishedQuestionRevisionTuple {
                published_question_id: PublishedQuestionId::from_random_identifier(format!(
                    "7K3M9Q{number}"
                ))
                .expect("valid Question ID"),
                revision_number: QuestionRevisionNumber::new(1).expect("positive version"),
            },
        }
    }

    fn question_pool() -> QuestionPoolAssessmentEntry {
        QuestionPoolAssessmentEntry {
            id: AssessmentEntryId::from_uuid(Uuid::from_u128(1)),
            availability: AssessmentEntryAvailability::Available,
            scoring_rule: AssessmentEntryScoringRule::Normal,
            question_pool_id: QuestionPoolId::from_random_identifier("7K3M9QP").expect("Pool ID"),
            question_pool_edit_number: QuestionPoolEditNumber::new(1).expect("edit number"),
            selection_count: std::num::NonZeroU32::new(2).expect("positive count"),
            points_per_item: AssessmentPointValue::from_whole(1),
            question_attempt_limit: QuestionAttemptLimit { max_attempts: None },
            question_attempt_time_limit: QuestionAttemptTimeLimit::Unlimited,
        }
    }

    #[test]
    fn selection_is_independent_of_candidate_input_order_and_without_replacement() {
        let selection = select_question_pool_items(
            &question_pool(),
            &[pool_member(0), pool_member(1), pool_member(2)],
            QuestionPoolSelectionEntropy::from_bytes([7; 32]),
        )
        .expect("available Question Pool Items satisfy the selection count");
        let reversed_candidates = [pool_member(2), pool_member(1), pool_member(0)];
        let reversed_selection = select_question_pool_items(
            &question_pool(),
            &reversed_candidates,
            QuestionPoolSelectionEntropy::from_bytes([7; 32]),
        )
        .expect("the same unordered tuple set satisfies the selection count");

        assert_eq!(selection, reversed_selection);
        assert_eq!(selection.len(), 2);
        assert_ne!(
            selection[0].published_question_revision_tuple,
            selection[1].published_question_revision_tuple
        );
        assert!(selection.iter().all(|item| {
            item.published_question_revision_tuple
                .published_question_id
                .as_str()
                != "7K3M9Q3"
        }));
    }

    #[test]
    fn selection_varies_with_server_entropy() {
        let candidates = [pool_member(0), pool_member(1), pool_member(2)];
        let selections = (0..16)
            .map(|seed| {
                select_question_pool_items(
                    &question_pool(),
                    &candidates,
                    QuestionPoolSelectionEntropy::from_bytes([seed; 32]),
                )
                .expect("available Question Pool Items satisfy the selection count")
                .into_iter()
                .map(|item| {
                    item.published_question_revision_tuple
                        .published_question_id
                        .as_str()
                        .to_owned()
                })
                .collect::<Vec<_>>()
            })
            .collect::<std::collections::HashSet<_>>();

        assert!(selections.len() > 1);
    }

    #[test]
    fn selection_refuses_a_pool_when_immutable_members_are_too_few() {
        let mut pool = question_pool();
        pool.selection_count = std::num::NonZeroU32::new(4).expect("positive count");

        assert_eq!(
            select_question_pool_items(
                &pool,
                &[pool_member(0), pool_member(1), pool_member(2)],
                QuestionPoolSelectionEntropy::from_bytes([1; 32])
            ),
            Err(
                QuestionPoolSelectionError::InsufficientAvailableQuestionPoolItems {
                    selection_count: 4,
                    available_question_pool_item_count: 3,
                }
            ),
        );
    }
}

//! Server-owned selection of exact immutable Question Pool Revision members.
//!
//! The caller supplies transient server entropy and the complete saved Question
//! Pool Assessment Entry and its immutable Revision members. This module records no entropy and reads no storage:
//! persistence owns Reuse Selection lookup, while this function creates the
//! selected Question Pool Item result for Select Again and no-store Question Pool Previews.

use question_model::{
    QuestionPoolAssessmentEntry, QuestionPoolSelectedItem, QuestionPoolSelectedQuestionOrder,
};
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
    /// A storage candidate does not belong to the exact Pool Revision pinned by the Entry.
    CandidatePoolRevisionMismatch,
    /// Fewer immutable Pool Revision members exist than the Assessment requires.
    InsufficientAvailableQuestionPoolItems {
        /// Instructor-requested Question Pool Selection Count.
        selection_count: u32,
        /// Immutable Pool Revision member count at selection time.
        available_question_pool_item_count: usize,
    },
}

impl std::fmt::Display for QuestionPoolSelectionError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::CandidatePoolRevisionMismatch => formatter.write_str(
                "Question Pool candidate does not belong to the Assessment Entry Pool Revision",
            ),
            Self::InsufficientAvailableQuestionPoolItems {
                selection_count,
                available_question_pool_item_count,
            } => write!(
                formatter,
                "Question Pool requires {selection_count} immutable members but only {available_question_pool_item_count} exist"
            ),
        }
    }
}

impl std::error::Error for QuestionPoolSelectionError {}

/// Selects exact immutable Pool Revision members for one new Question Pool Selection.
///
/// Membership is sampled without replacement. Question Pool Order restores the
/// immutable Pool Revision member order after membership selection; Random Order
/// keeps the sampled order. The returned values carry immutable Question
/// Revision References and are suitable for a server-held Question Pool
/// Selection record.
pub fn select_question_pool_items(
    question_pool: &QuestionPoolAssessmentEntry,
    candidates: &[QuestionPoolSelectedItem],
    entropy: QuestionPoolSelectionEntropy,
) -> Result<Vec<QuestionPoolSelectedItem>, QuestionPoolSelectionError> {
    if candidates.iter().any(|candidate| {
        candidate.pool_revision_member.question_pool_revision
            != question_pool.question_pool_revision
    }) {
        return Err(QuestionPoolSelectionError::CandidatePoolRevisionMismatch);
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

    let mut positions = (0..candidates.len()).collect::<Vec<_>>();
    let mut random = ChaCha20Rng::from_seed(entropy.0);
    for position in 0..selection_count {
        let remaining = positions.len() - position;
        let selected = position
            + usize::try_from(sample_below(&mut random, remaining as u64))
                .expect("selected Question Pool position fits usize");
        positions.swap(position, selected);
    }
    positions.truncate(selection_count);
    if question_pool.selection_rule.selected_question_order
        == QuestionPoolSelectedQuestionOrder::QuestionPoolOrder
    {
        positions.sort_unstable();
    }

    Ok(positions
        .into_iter()
        .map(|position| candidates[position].clone())
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
        AssessmentPointValue, PoolRevisionMemberReference, QuestionAttemptLimit,
        QuestionAttemptTimeLimit, QuestionId, QuestionPoolRevisionNumber,
        QuestionPoolRevisionReference, QuestionPoolSelectionRule, QuestionRevisionNumber,
        QuestionRevisionReference,
    };
    use uuid::Uuid;

    use super::*;

    fn pool_member(number: u32) -> QuestionPoolSelectedItem {
        QuestionPoolSelectedItem {
            pool_revision_member: PoolRevisionMemberReference {
                question_pool_revision: QuestionPoolRevisionReference {
                    question_pool_id: QuestionId::from_random_identifier("7K3M9QP")
                        .expect("Pool ID"),
                    revision_number: QuestionPoolRevisionNumber::new(1).expect("revision"),
                },
                member_position: number,
            },
            reference: QuestionRevisionReference {
                question_id: QuestionId::from_random_identifier(format!("7K3M9Q{number}"))
                    .expect("valid Question ID"),
                revision_number: QuestionRevisionNumber::new(1).expect("positive version"),
            },
        }
    }

    fn question_pool(order: QuestionPoolSelectedQuestionOrder) -> QuestionPoolAssessmentEntry {
        QuestionPoolAssessmentEntry {
            id: AssessmentEntryId::from_uuid(Uuid::from_u128(1)),
            availability: AssessmentEntryAvailability::Available,
            scoring_rule: AssessmentEntryScoringRule::Normal,
            question_pool_revision: QuestionPoolRevisionReference {
                question_pool_id: QuestionId::from_random_identifier("7K3M9QP").expect("Pool ID"),
                revision_number: QuestionPoolRevisionNumber::new(1).expect("revision"),
            },
            selection_count: std::num::NonZeroU32::new(2).expect("positive count"),
            points_per_item: AssessmentPointValue::from_whole(1),
            selection_rule: QuestionPoolSelectionRule {
                selected_question_order: order,
            },
            question_attempt_limit: QuestionAttemptLimit { max_attempts: None },
            question_attempt_time_limit: QuestionAttemptTimeLimit::Unlimited,
        }
    }

    #[test]
    fn question_pool_order_selects_available_items_without_replacement_in_source_order() {
        let selection = select_question_pool_items(
            &question_pool(QuestionPoolSelectedQuestionOrder::QuestionPoolOrder),
            &[pool_member(0), pool_member(1), pool_member(2)],
            QuestionPoolSelectionEntropy::from_bytes([7; 32]),
        )
        .expect("available Question Pool Items satisfy the selection count");

        assert_eq!(selection.len(), 2);
        assert!(
            selection
                .windows(2)
                .all(|pair| pair[0].pool_revision_member != pair[1].pool_revision_member)
        );
        assert!(
            selection
                .iter()
                .all(|item| item.pool_revision_member.member_position != 3)
        );
        assert!(
            selection[0].pool_revision_member.member_position
                < selection[1].pool_revision_member.member_position
        );
    }

    #[test]
    fn selection_refuses_a_pool_when_immutable_members_are_too_few() {
        let mut pool = question_pool(QuestionPoolSelectedQuestionOrder::QuestionPoolOrder);
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

//! Server-owned selection of exact Question Pool Candidates.
//!
//! The caller supplies transient server entropy and the complete saved Question
//! Pool Assignment Entry. This module records no entropy and reads no storage:
//! persistence owns Reuse Selection lookup, while this function creates the
//! candidate result for Select Again and no-store Question Pool Previews.

use question_model::{
    QuestionPoolAssignmentEntry, QuestionPoolCandidateAvailability, QuestionPoolSelectedCandidate,
    QuestionPoolSelectedQuestionOrder,
};
use rand_chacha::ChaCha20Rng;
use rand_chacha::rand_core::{Rng, SeedableRng};

/// Opaque transient entropy supplied by a trusted server operation.
///
/// The selected candidates, rather than these bytes, become durable Student
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
    /// Fewer currently available candidates exist than the Assignment requires.
    InsufficientAvailableCandidates {
        /// Instructor-requested Question Pool Selection Count.
        selection_count: u32,
        /// Available candidate count at selection time.
        available_candidate_count: usize,
    },
}

impl std::fmt::Display for QuestionPoolSelectionError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InsufficientAvailableCandidates {
                selection_count,
                available_candidate_count,
            } => write!(
                formatter,
                "Question Pool requires {selection_count} candidates but only {available_candidate_count} are available"
            ),
        }
    }
}

impl std::error::Error for QuestionPoolSelectionError {}

/// Selects the exact candidates for one new Question Pool Selection.
///
/// Candidate membership is sampled without replacement. Candidate Order
/// restores the saved candidate order after membership selection; Random Order
/// keeps the sampled order. The returned values carry immutable Question
/// Revision References and are suitable for a server-held Question Pool
/// Selection record.
pub fn select_question_pool_candidates(
    question_pool: &QuestionPoolAssignmentEntry,
    entropy: QuestionPoolSelectionEntropy,
) -> Result<Vec<QuestionPoolSelectedCandidate>, QuestionPoolSelectionError> {
    let available = question_pool
        .candidates
        .iter()
        .filter(|candidate| candidate.availability == QuestionPoolCandidateAvailability::Available)
        .collect::<Vec<_>>();
    let selection_count = usize::try_from(question_pool.selection_count)
        .expect("u32 selection count fits the current supported usize targets");
    if selection_count > available.len() {
        return Err(
            QuestionPoolSelectionError::InsufficientAvailableCandidates {
                selection_count: question_pool.selection_count,
                available_candidate_count: available.len(),
            },
        );
    }

    let mut positions = (0..available.len()).collect::<Vec<_>>();
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
        == QuestionPoolSelectedQuestionOrder::CandidateOrder
    {
        positions.sort_unstable();
    }

    Ok(positions
        .into_iter()
        .map(|position| QuestionPoolSelectedCandidate {
            candidate: available[position].id,
            reference: available[position].reference.clone(),
        })
        .collect())
}

/// Samples `0..upper` without modulo bias.
fn sample_below(random: &mut ChaCha20Rng, upper: u64) -> u64 {
    debug_assert!(upper > 0);
    let rejection_threshold = upper.wrapping_neg() % upper;
    loop {
        let candidate = random.next_u64();
        if candidate >= rejection_threshold {
            return candidate % upper;
        }
    }
}

#[cfg(test)]
mod tests {
    use question_model::{
        AssignmentEntryAvailability, AssignmentEntryId, AssignmentEntryScoringRule,
        AssignmentPointValue, QuestionPoolCandidate, QuestionPoolCandidateId,
        QuestionPoolSelectionRule, QuestionVersionNumber, QuestionVersionReference,
    };
    use uuid::Uuid;

    use super::*;

    fn candidate(
        number: u128,
        availability: QuestionPoolCandidateAvailability,
    ) -> QuestionPoolCandidate {
        QuestionPoolCandidate {
            id: QuestionPoolCandidateId::from_uuid(Uuid::from_u128(number)),
            reference: QuestionVersionReference {
                question_id: format!("123-456{number}")
                    .parse()
                    .expect("valid Question ID"),
                version_number: QuestionVersionNumber::new(1).expect("positive version"),
            },
            availability,
        }
    }

    fn question_pool(order: QuestionPoolSelectedQuestionOrder) -> QuestionPoolAssignmentEntry {
        QuestionPoolAssignmentEntry {
            id: AssignmentEntryId::from_uuid(Uuid::from_u128(1)),
            availability: AssignmentEntryAvailability::Available,
            scoring_rule: AssignmentEntryScoringRule::Normal,
            selection_count: 2,
            points_per_item: AssignmentPointValue::from_whole(1),
            selection_rule: QuestionPoolSelectionRule {
                selected_question_order: order,
            },
            candidates: vec![
                candidate(2, QuestionPoolCandidateAvailability::Available),
                candidate(3, QuestionPoolCandidateAvailability::Retired),
                candidate(4, QuestionPoolCandidateAvailability::Available),
                candidate(5, QuestionPoolCandidateAvailability::Available),
            ],
        }
    }

    #[test]
    fn candidate_order_selects_available_candidates_without_replacement_in_source_order() {
        let selection = select_question_pool_candidates(
            &question_pool(QuestionPoolSelectedQuestionOrder::CandidateOrder),
            QuestionPoolSelectionEntropy::from_bytes([7; 32]),
        )
        .expect("available candidates satisfy the selection count");

        assert_eq!(selection.len(), 2);
        assert!(
            selection
                .windows(2)
                .all(|pair| pair[0].candidate != pair[1].candidate)
        );
        assert!(
            selection
                .iter()
                .all(|candidate| candidate.candidate.as_uuid() != Uuid::from_u128(3))
        );
        assert!(selection[0].candidate.as_uuid() < selection[1].candidate.as_uuid());
    }

    #[test]
    fn random_order_is_reproducible_only_from_transient_server_entropy() {
        let pool = question_pool(QuestionPoolSelectedQuestionOrder::RandomOrder);
        let entropy = QuestionPoolSelectionEntropy::from_bytes([9; 32]);

        assert_eq!(
            select_question_pool_candidates(&pool, entropy),
            select_question_pool_candidates(&pool, entropy),
        );
    }

    #[test]
    fn selection_refuses_a_pool_when_retired_candidates_leave_too_few_available() {
        let mut pool = question_pool(QuestionPoolSelectedQuestionOrder::CandidateOrder);
        pool.selection_count = 4;

        assert_eq!(
            select_question_pool_candidates(
                &pool,
                QuestionPoolSelectionEntropy::from_bytes([1; 32])
            ),
            Err(
                QuestionPoolSelectionError::InsufficientAvailableCandidates {
                    selection_count: 4,
                    available_candidate_count: 3,
                }
            ),
        );
    }
}

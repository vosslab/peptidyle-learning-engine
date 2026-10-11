//! Partial-credit scoring for a validated Native JSON ORDER response.
//!
//! The caller maps stable item IDs to their zero-based ranks in the answer
//! key and validates that the submitted ranks are a complete permutation.
//! This helper owns only the score formula.

/// Scores a submitted order represented by each item's rank in the answer key.
///
/// `submitted_key_ranks` must contain every rank from `0..len` exactly once,
/// and must have at least two entries. These are publication/response
/// validation invariants owned by the caller.
pub fn ordering_credit_fraction(answer_key_ranks: &[usize]) -> f64 {
    let item_count = answer_key_ranks.len();
    let pair_count = item_count * (item_count - 1) / 2;

    let correct_positions = answer_key_ranks
        .iter()
        .enumerate()
        .filter(|(position, key_rank)| *position == **key_rank)
        .count();

    let mut correctly_ordered_pairs = 0;
    for first_position in 0..item_count {
        for second_position in first_position + 1..item_count {
            if answer_key_ranks[first_position] < answer_key_ranks[second_position] {
                correctly_ordered_pairs += 1;
            }
        }
    }

    let position_fraction = correct_positions as f64 / item_count as f64;
    let pair_fraction = correctly_ordered_pairs as f64 / pair_count as f64;
    (position_fraction + pair_fraction) / 2.0
}

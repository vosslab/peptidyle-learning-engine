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

#[cfg(test)]
mod tests {
    use super::ordering_credit_fraction;

    fn assert_score(order: &[usize], expected: f64) {
        let score = ordering_credit_fraction(order);
        assert!(
            (score - expected).abs() < 1e-12,
            "{order:?}: {score} != {expected}"
        );
    }

    #[test]
    fn worked_example_and_perfect_order_match_the_specification() {
        assert_score(&[3, 0, 1, 2], 0.25); // DABC against ABCD
        assert_score(&[0, 1, 2, 3], 1.0);
    }

    #[test]
    fn reversals_have_zero_pair_credit_and_only_odd_middle_position_credit() {
        assert_score(&[3, 2, 1, 0], 0.0);
        assert_score(&[2, 1, 0], 1.0 / 6.0);
        assert_score(&[4, 3, 2, 1, 0], 0.1);
        assert_score(&[6, 5, 4, 3, 2, 1, 0], 1.0 / 14.0);
    }

    #[test]
    fn single_swaps_match_the_specified_size_and_distance_examples() {
        let adjacent_expected = [0.5, 2.0 / 3.0, 0.75, 0.8, 5.0 / 6.0];
        for (offset, expected) in adjacent_expected.into_iter().enumerate() {
            let item_count = offset + 3;
            let mut order: Vec<_> = (0..item_count).collect();
            order.swap(2, 1);
            assert_score(&order, expected);
        }

        for (distance, expected) in [
            5.0 / 6.0,
            11.0 / 14.0,
            31.0 / 42.0,
            29.0 / 42.0,
            9.0 / 14.0,
            25.0 / 42.0,
        ]
        .into_iter()
        .enumerate()
        {
            let mut order: Vec<_> = (0..7).collect();
            order.swap(0, distance + 1);
            assert_score(&order, expected);
        }
    }

    #[test]
    fn every_permutation_from_three_through_seven_is_bounded_and_only_perfect_is_full_credit() {
        fn visit(order: &mut [usize], position: usize, full_credit_count: &mut usize) {
            if position == order.len() {
                let score = ordering_credit_fraction(order);
                assert!((0.0..=1.0).contains(&score), "{order:?}: {score}");
                if score == 1.0 {
                    *full_credit_count += 1;
                    assert!(order.iter().enumerate().all(|(index, rank)| index == *rank));
                }
                return;
            }
            for next in position..order.len() {
                order.swap(position, next);
                visit(order, position + 1, full_credit_count);
                order.swap(position, next);
            }
        }

        for item_count in 3..=7 {
            let mut order: Vec<_> = (0..item_count).collect();
            let mut full_credit_count = 0;
            visit(&mut order, 0, &mut full_credit_count);
            assert_eq!(full_credit_count, 1, "item count {item_count}");
        }
    }
}

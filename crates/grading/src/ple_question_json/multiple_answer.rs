//! Native JSON Multiple Answer credit calculation.

/// Returns the stored credit fraction for a validated Multiple Answer response.
///
/// `correct_choice_count` is positive for a published question, and `tp` and
/// `fp` count distinct selected choices. The caller validates those counts
/// against the Question and its Answer Key before calling this helper.
pub(super) fn multiple_answer_credit_fraction(
    tp: usize,
    fp: usize,
    correct_choice_count: usize,
) -> f64 {
    let selected_count = tp + fp;
    if correct_choice_count == 0 || selected_count == 0 {
        return 0.0;
    }

    let base_credit = ((tp as f64 - fp as f64) / correct_choice_count as f64).max(0.0);
    let choice_count_score = selected_count.min(correct_choice_count) as f64
        / selected_count.max(correct_choice_count) as f64;

    base_credit * choice_count_score
}

#[cfg(test)]
mod tests {
    use super::multiple_answer_credit_fraction as credit;

    fn assert_credit(tp: usize, fp: usize, correct: usize, expected: f64) {
        let actual = credit(tp, fp, correct);
        assert!(
            (actual - expected).abs() < 0.000_05,
            "TP={tp}, FP={fp}, C={correct}: expected {expected}, got {actual}"
        );
    }

    #[test]
    fn matches_the_two_correct_of_ten_examples() {
        assert_credit(1, 0, 2, 0.25);
        assert_credit(1, 1, 2, 0.0);
        assert_credit(2, 0, 2, 1.0);
        assert_credit(2, 1, 2, 1.0 / 3.0);
        assert_credit(2, 8, 2, 0.0);
    }

    #[test]
    fn matches_the_five_correct_of_ten_examples() {
        assert_credit(3, 1, 5, 0.32);
        assert_credit(4, 0, 5, 0.64);
        assert_credit(4, 1, 5, 0.60);
        assert_credit(4, 2, 5, 1.0 / 3.0);
        assert_credit(5, 0, 5, 1.0);
        assert_credit(5, 1, 5, 2.0 / 3.0);
        assert_credit(5, 5, 5, 0.0);
    }

    #[test]
    fn matches_the_eight_correct_of_ten_examples() {
        assert_credit(6, 0, 8, 0.5625);
        assert_credit(6, 1, 8, 0.546875);
        assert_credit(6, 2, 8, 0.5);
        assert_credit(7, 0, 8, 0.765625);
        assert_credit(7, 1, 8, 0.75);
        assert_credit(8, 0, 8, 1.0);
        assert_credit(8, 1, 8, 7.0 / 9.0);
        assert_credit(8, 2, 8, 0.6);
    }

    #[test]
    fn empty_selection_and_zero_correct_count_score_zero() {
        assert_credit(0, 0, 5, 0.0);
        assert_credit(0, 0, 0, 0.0);
    }

    #[test]
    fn valid_ten_choice_questions_stay_bounded_and_extra_wrong_choices_do_not_help() {
        for correct in 1..10 {
            let incorrect = 10 - correct;
            for tp in 0..=correct {
                let mut previous = credit(tp, 0, correct);
                for fp in 1..=incorrect {
                    let current = credit(tp, fp, correct);
                    assert!((0.0..=1.0).contains(&current));
                    assert!(
                        current <= previous + f64::EPSILON,
                        "adding an incorrect choice raised credit: C={correct}, TP={tp}, FP={fp}"
                    );
                    previous = current;
                }
            }
        }
    }

    #[test]
    fn score_depends_on_choice_counts_not_the_order_used_to_collect_them() {
        fn counts_in_order(correct: &[bool], selected: &[bool]) -> (usize, usize) {
            correct
                .iter()
                .zip(selected)
                .fold((0, 0), |(tp, fp), (is_correct, is_selected)| {
                    match (*is_correct, *is_selected) {
                        (true, true) => (tp + 1, fp),
                        (false, true) => (tp, fp + 1),
                        _ => (tp, fp),
                    }
                })
        }

        let answer_key = [true, false, true, false, false, true];
        let selected = [true, true, false, false, true, false];
        let (tp, fp) = counts_in_order(&answer_key, &selected);
        let (reversed_tp, reversed_fp) = counts_in_order(
            &answer_key.iter().rev().copied().collect::<Vec<_>>(),
            &selected.iter().rev().copied().collect::<Vec<_>>(),
        );

        assert_eq!((tp, fp), (reversed_tp, reversed_fp));
        assert_eq!(credit(tp, fp, 3), credit(reversed_tp, reversed_fp, 3));
    }
}

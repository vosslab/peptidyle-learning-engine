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

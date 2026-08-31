use question_model::response::ResponseItemReference;

use super::super::*;

#[test]
fn accepted_grades_preserve_exact_correct_and_choice_counts() {
    let mut statistics = QuestionVersionStatistics::empty();
    statistics
        .record(
            QuestionStatisticsObservation::new(
                true,
                [
                    ResponseItemReference::new("a"),
                    ResponseItemReference::new("c"),
                ],
            )
            .expect("valid observation"),
        )
        .expect("first accepted grade records");
    statistics
        .record(
            QuestionStatisticsObservation::new(false, [ResponseItemReference::new("c")])
                .expect("valid observation"),
        )
        .expect("second accepted grade records");

    assert_eq!(statistics.accepted_graded_attempt_count(), 2);
    assert_eq!(statistics.correct_count(), 1);
    assert_eq!(
        statistics.eligible_choice_selection_counts(),
        &[
            (ResponseItemReference::new("a"), 1),
            (ResponseItemReference::new("c"), 2)
        ]
        .into_iter()
        .collect()
    );
}

#[test]
fn snapshot_refuses_counts_that_could_not_come_from_accepted_grades() {
    assert_eq!(
        QuestionVersionStatistics::restore(QuestionVersionStatisticsSnapshot {
            accepted_graded_attempt_count: 2,
            correct_count: 3,
            eligible_choice_selection_counts: Default::default(),
        }),
        Err(StatisticsError::SnapshotInvariant)
    );
    assert_eq!(
        QuestionVersionStatistics::restore(QuestionVersionStatisticsSnapshot {
            accepted_graded_attempt_count: 1,
            correct_count: 1,
            eligible_choice_selection_counts: [(ResponseItemReference::new("a"), 2)]
                .into_iter()
                .collect(),
        }),
        Err(StatisticsError::SnapshotInvariant)
    );
}

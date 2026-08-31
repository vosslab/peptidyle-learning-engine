use std::num::NonZeroU32;

use question_model::{QuestionStatisticsDisclosure, StatisticsDisclosurePolicy};

use super::super::*;

fn observation(
    score: f64,
    attempts: u64,
    duration_seconds: u64,
    rest_score: Option<f64>,
) -> QuestionCohortRollupObservation {
    QuestionCohortRollupObservation::new(score, attempts, duration_seconds, rest_score)
        .expect("test observation should be valid")
}

#[test]
fn hand_computed_fixture_discloses_exact_aggregate_metrics() {
    let mut aggregate = QuestionCohortRollup::empty();
    for (score, attempts, duration) in [
        (0.0, 1, 1),
        (0.25, 2, 5),
        (0.5, 3, 15),
        (0.75, 4, 30),
        (1.0, 5, 60),
    ] {
        aggregate
            .record(observation(score, attempts, duration, Some(score)))
            .expect("observation should merge");
    }

    let QuestionStatisticsDisclosure::Available(view) =
        aggregate.disclose(StatisticsDisclosurePolicy::default())
    else {
        panic!("five observations pass default k");
    };
    assert_eq!(view.cohort_size, 5);
    assert!((view.difficulty_index - 0.5).abs() < f64::EPSILON);
    assert!((view.attempts_mean - 3.0).abs() < f64::EPSILON);
    assert_eq!(view.time_median_seconds_estimate, 15);
    assert_eq!(view.discrimination_index, Some(1.0));
    assert_eq!(aggregate.durations().version(), DURATION_HISTOGRAM_VERSION);
}

#[test]
fn disclosure_omits_unavailable_discrimination_without_losing_general_metrics() {
    let mut aggregate = QuestionCohortRollup::empty();
    for duration in [1, 5, 15, 30, 60] {
        aggregate
            .record(observation(0.8, 1, duration, None))
            .expect("observation should merge");
    }

    let QuestionStatisticsDisclosure::Available(view) =
        aggregate.disclose(StatisticsDisclosurePolicy::default())
    else {
        panic!("aggregate is releasable");
    };
    assert!((view.difficulty_index - 0.8).abs() < f64::EPSILON);
    assert_eq!(view.discrimination_index, None);
}

#[test]
fn default_k_suppresses_four_and_releases_five() {
    let mut aggregate = QuestionCohortRollup::empty();
    for score in [0.0, 0.25, 0.5, 0.75] {
        aggregate
            .record(observation(score, 1, 5, Some(score)))
            .expect("observation should merge");
    }
    assert_eq!(
        aggregate.disclose(StatisticsDisclosurePolicy::default()),
        QuestionStatisticsDisclosure::Suppressed
    );

    aggregate
        .record(observation(1.0, 1, 5, Some(1.0)))
        .expect("fifth observation should merge");
    assert!(matches!(
        aggregate.disclose(StatisticsDisclosurePolicy::default()),
        QuestionStatisticsDisclosure::Available(_)
    ));

    let custom = StatisticsDisclosurePolicy::new(NonZeroU32::new(6).expect("six is nonzero"))
        .expect("higher threshold is valid");
    assert_eq!(
        aggregate.disclose(custom),
        QuestionStatisticsDisclosure::Suppressed
    );
}

#[test]
fn sub_k_scored_cohort_cannot_leak_correlation_through_a_releasable_view() {
    let mut aggregate = QuestionCohortRollup::empty();
    for (score, rest_score) in [
        (0.0, Some(0.0)),
        (1.0, Some(1.0)),
        (0.25, None),
        (0.5, None),
        (0.75, None),
    ] {
        aggregate
            .record(observation(score, 1, 5, rest_score))
            .expect("observation should merge");
    }

    let QuestionStatisticsDisclosure::Available(view) =
        aggregate.disclose(StatisticsDisclosurePolicy::default())
    else {
        panic!("five general observations pass default k");
    };
    assert_eq!(aggregate.scored_cohort_size(), 2);
    assert_eq!(view.discrimination_index, None);
}

#[test]
fn disclosure_never_returns_partial_metrics_for_an_empty_aggregate() {
    assert_eq!(
        QuestionCohortRollup::empty().disclose(StatisticsDisclosurePolicy::default()),
        QuestionStatisticsDisclosure::Suppressed
    );
}

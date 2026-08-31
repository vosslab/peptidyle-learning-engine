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
fn zero_variance_omits_discrimination() {
    let mut aggregate = QuestionCohortRollup::empty();
    for (score, rest) in [(0.5, 0.0), (0.5, 0.25), (0.5, 0.75), (0.5, 1.0), (0.5, 0.5)] {
        aggregate
            .record(observation(score, 1, 5, Some(rest)))
            .expect("constant score should merge");
    }

    assert_eq!(aggregate.discrimination_index(), None);
}

#[test]
fn invalid_scalars_and_attempt_bounds_are_refused() {
    assert_eq!(
        QuestionCohortRollupObservation::new(f64::NAN, 1, 1, None),
        Err(StatisticsError::NonFiniteScalar)
    );
    assert_eq!(
        QuestionCohortRollupObservation::new(1.1, 1, 1, None),
        Err(StatisticsError::ScoreOutOfRange)
    );
    assert_eq!(
        QuestionCohortRollupObservation::new(0.5, 0, 1, None),
        Err(StatisticsError::ZeroAttempts)
    );
    assert_eq!(
        QuestionCohortRollupObservation::new(0.5, 1, 1, Some(f64::INFINITY)),
        Err(StatisticsError::NonFiniteScalar)
    );
}

#[test]
fn duration_bins_are_fixed_and_bounded() {
    let mut histogram = DurationHistogram::empty();
    for duration in [
        0,
        1,
        2,
        5,
        6,
        MAX_DURATION_SECONDS,
        MAX_DURATION_SECONDS + 1,
    ] {
        histogram.record(duration).expect("bounded duration");
    }
    assert_eq!(histogram.bins(), &[2, 2, 1, 0, 0, 0, 0, 0, 0, 2]);
    assert_eq!(histogram.median_seconds_estimate(), Some(5));
}

#[test]
fn snapshot_restore_and_atomic_merge_match_direct_recording() {
    let left_observations = [
        observation(0.0, 1, 1, Some(1.0)),
        observation(0.25, 2, 5, Some(0.75)),
        observation(0.5, 3, 15, Some(0.5)),
    ];
    let right_observations = [
        observation(0.75, 4, 30, Some(0.25)),
        observation(1.0, 5, 60, Some(0.0)),
    ];
    let mut left = QuestionCohortRollup::empty();
    let mut right = QuestionCohortRollup::empty();
    let mut direct = QuestionCohortRollup::empty();
    for value in left_observations {
        left.record(value).expect("left observation merges");
        direct.record(value).expect("direct observation merges");
    }
    for value in right_observations {
        right.record(value).expect("right observation merges");
        direct.record(value).expect("direct observation merges");
    }

    let right_snapshot = right.snapshot();
    let restored = QuestionCohortRollup::restore(&right_snapshot).expect("valid snapshot restores");
    assert_eq!(restored.snapshot(), right_snapshot);
    left.merge_snapshot(&right_snapshot)
        .expect("valid snapshot merges atomically");
    assert_eq!(left.snapshot(), direct.snapshot());
    assert_eq!(left.discrimination_index(), Some(-1.0));
}

#[test]
fn partial_negative_correlation_is_preserved_by_stable_moments() {
    let mut aggregate = QuestionCohortRollup::empty();
    for (score, rest) in [(0.0, 1.0), (0.25, 0.6), (0.5, 0.7), (0.75, 0.1), (1.0, 0.0)] {
        aggregate
            .record(observation(score, 1, 5, Some(rest)))
            .expect("bounded pair merges");
    }
    let correlation = aggregate
        .discrimination_index()
        .expect("nontrivial correlation is defined");
    assert!(correlation < -0.8 && correlation > -1.0);
}

#[test]
fn invalid_snapshots_are_refused_without_mutating_the_target() {
    let valid = QuestionCohortRollup::empty().snapshot();
    let bad_version = QuestionCohortRollupSnapshot {
        durations: DurationHistogramSnapshot {
            version: DURATION_HISTOGRAM_VERSION + 1,
            bins: vec![0; DURATION_HISTOGRAM_UPPER_BOUNDS_SECONDS.len()],
        },
        ..valid.clone()
    };
    assert_eq!(
        QuestionCohortRollup::restore(&bad_version),
        Err(StatisticsError::HistogramVersionMismatch)
    );
    let bad_bins = QuestionCohortRollupSnapshot {
        durations: DurationHistogramSnapshot {
            version: DURATION_HISTOGRAM_VERSION,
            bins: vec![0; DURATION_HISTOGRAM_UPPER_BOUNDS_SECONDS.len() - 1],
        },
        ..valid.clone()
    };
    assert_eq!(
        QuestionCohortRollup::restore(&bad_bins),
        Err(StatisticsError::HistogramBinCountMismatch)
    );
    let bad_terms = QuestionCohortRollupSnapshot {
        cohort_size: 1,
        score_sum: 0.5,
        attempts_sum: 1,
        durations: DurationHistogramSnapshot {
            version: DURATION_HISTOGRAM_VERSION,
            bins: vec![1, 0, 0, 0, 0, 0, 0, 0, 0, 0],
        },
        discrimination: PearsonMomentSnapshot {
            count: 2,
            mean_x: 0.5,
            mean_y: 0.5,
            m2_x: 0.25,
            m2_y: 0.25,
            co_moment: 0.25,
        },
    };
    let mut target = QuestionCohortRollup::empty();
    let before = target.clone();
    assert_eq!(
        target.merge_snapshot(&bad_terms),
        Err(StatisticsError::SnapshotInvariant)
    );
    assert_eq!(target, before);

    let incompatible_paired_score_sum = QuestionCohortRollupSnapshot {
        cohort_size: 5,
        score_sum: 0.0,
        attempts_sum: 5,
        durations: DurationHistogramSnapshot {
            version: DURATION_HISTOGRAM_VERSION,
            bins: vec![5, 0, 0, 0, 0, 0, 0, 0, 0, 0],
        },
        discrimination: PearsonMomentSnapshot {
            count: 5,
            mean_x: 0.5,
            mean_y: 0.5,
            m2_x: 1.25,
            m2_y: 1.25,
            co_moment: 0.0,
        },
    };
    assert_eq!(
        QuestionCohortRollup::restore(&incompatible_paired_score_sum),
        Err(StatisticsError::SnapshotInvariant)
    );

    let impossible_singleton = QuestionCohortRollupSnapshot {
        cohort_size: 1,
        score_sum: 0.5,
        attempts_sum: 1,
        durations: DurationHistogramSnapshot {
            version: DURATION_HISTOGRAM_VERSION,
            bins: vec![1, 0, 0, 0, 0, 0, 0, 0, 0, 0],
        },
        discrimination: PearsonMomentSnapshot {
            count: 1,
            mean_x: 0.5,
            mean_y: 0.5,
            m2_x: 0.1,
            m2_y: 0.1,
            co_moment: 0.1,
        },
    };
    assert_eq!(
        QuestionCohortRollup::restore(&impossible_singleton),
        Err(StatisticsError::SnapshotInvariant)
    );

    let impossible_bounded_variance = QuestionCohortRollupSnapshot {
        cohort_size: 2,
        score_sum: 1.0,
        attempts_sum: 2,
        durations: DurationHistogramSnapshot {
            version: DURATION_HISTOGRAM_VERSION,
            bins: vec![2, 0, 0, 0, 0, 0, 0, 0, 0, 0],
        },
        discrimination: PearsonMomentSnapshot {
            count: 2,
            mean_x: 0.5,
            mean_y: 0.5,
            m2_x: 0.75,
            m2_y: 0.5,
            co_moment: 0.0,
        },
    };
    assert_eq!(
        QuestionCohortRollup::restore(&impossible_bounded_variance),
        Err(StatisticsError::SnapshotInvariant)
    );

    let negative_zero = QuestionCohortRollupSnapshot {
        score_sum: -0.0,
        ..valid
    };
    assert_eq!(
        QuestionCohortRollup::restore(&negative_zero),
        Err(StatisticsError::SnapshotInvariant)
    );
}

//! Pure, retention-safe anonymous question-statistics aggregation.
//!
//! These values are sufficient statistics only: they carry no deployment or
//! Student identity, answer material, feedback, source reference, or raw time
//! series. Store implementations derive one observation per exact published
//! question revision before course-owned Student Work Records are deleted.

use std::fmt;

/// Version of the fixed duration-histogram format.
pub const DURATION_HISTOGRAM_VERSION: u8 = 1;

/// Largest exact duration represented by the initial fixed histogram: one day.
pub const MAX_DURATION_SECONDS: u64 = 86_400;

/// Inclusive upper bounds for the fixed duration histogram's bins.
///
/// The sequence preserves useful resolution for ordinary learning responses
/// without retaining raw timing values. Durations beyond the final bound are
/// saturated into that final bounded bin, so a pathological clock reading
/// cannot make an otherwise valid submission fail.
pub const DURATION_HISTOGRAM_UPPER_BOUNDS_SECONDS: [u64; 10] =
    [1, 5, 15, 30, 60, 120, 300, 900, 3_600, MAX_DURATION_SECONDS];

/// A rejected statistics scalar or aggregate operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StatisticsError {
    /// An eligible choice ID cannot identify a selection count.
    InvalidChoiceIdentifier,
    /// A score or sufficient-statistic value was NaN or infinite.
    NonFiniteScalar,
    /// A normalized score was outside the inclusive unit interval.
    ScoreOutOfRange,
    /// A collapsed observation had no submitted attempts.
    ZeroAttempts,
    /// An integer aggregate counter could not represent another contribution.
    CounterOverflow,
    /// A floating aggregate sum became non-finite.
    AggregateOverflow,
    /// A persisted histogram used an unsupported format version.
    HistogramVersionMismatch,
    /// A persisted histogram used the wrong fixed bin count.
    HistogramBinCountMismatch,
    /// Persisted sufficient terms contradict the bounded-score contract.
    SnapshotInvariant,
}

impl fmt::Display for StatisticsError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidChoiceIdentifier => {
                formatter.write_str("statistics choice identifier must be nonempty")
            }
            Self::NonFiniteScalar => formatter.write_str("statistics scalar must be finite"),
            Self::ScoreOutOfRange => {
                formatter.write_str("normalized statistics score must be between zero and one")
            }
            Self::ZeroAttempts => {
                formatter.write_str("collapsed statistics observation must have an attempt")
            }
            Self::CounterOverflow => formatter.write_str("statistics counter overflow"),
            Self::AggregateOverflow => formatter.write_str("statistics aggregate overflow"),
            Self::HistogramVersionMismatch => {
                formatter.write_str("statistics histogram version is unsupported")
            }
            Self::HistogramBinCountMismatch => {
                formatter.write_str("statistics histogram bin count is invalid")
            }
            Self::SnapshotInvariant => {
                formatter.write_str("statistics snapshot violates bounded-score invariants")
            }
        }
    }
}

impl std::error::Error for StatisticsError {}

/// One identity-free cohort measure for one exact Question Revision.
///
/// Store code derives this from one enrollment's first completed assigned run.
/// Repeated assignment positions of the same version have already been
/// collapsed before constructing this type.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct QuestionCohortRollupObservation {
    normalized_score: f64,
    attempts: u64,
    duration_seconds: u64,
    rest_score: Option<f64>,
}

impl QuestionCohortRollupObservation {
    /// Validates one cohort-rollup observation.
    pub fn new(
        normalized_score: f64,
        attempts: u64,
        duration_seconds: u64,
        rest_score: Option<f64>,
    ) -> Result<Self, StatisticsError> {
        validate_score(normalized_score)?;
        if attempts == 0 {
            return Err(StatisticsError::ZeroAttempts);
        }
        if let Some(rest_score) = rest_score {
            validate_score(rest_score)?;
        }
        Ok(Self {
            normalized_score,
            attempts,
            duration_seconds,
            rest_score,
        })
    }

    /// Returns the collapsed normalized score.
    pub fn normalized_score(self) -> f64 {
        self.normalized_score
    }

    /// Returns the count of submitted attempts represented by this observation.
    pub fn attempts(self) -> u64 {
        self.attempts
    }

    /// Returns the bounded representative response duration.
    pub fn duration_seconds(self) -> u64 {
        self.duration_seconds
    }

    /// Returns the rest-of-run score used for discrimination when available.
    pub fn rest_score(self) -> Option<f64> {
        self.rest_score
    }
}

/// Fixed, mergeable duration histogram with no raw duration series.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DurationHistogram {
    bins: [u64; DURATION_HISTOGRAM_UPPER_BOUNDS_SECONDS.len()],
}

/// Non-wire persistence snapshot of the fixed duration histogram.
///
/// The vector is intentionally validated on restore so a database row with a
/// stale format or malformed bin count cannot enter a live aggregate.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DurationHistogramSnapshot {
    /// Fixed histogram format version.
    pub version: u8,
    /// Bin counts in ascending upper-bound order.
    pub bins: Vec<u64>,
}

impl DurationHistogram {
    /// Creates an empty histogram in the current fixed format.
    pub const fn empty() -> Self {
        Self {
            bins: [0; DURATION_HISTOGRAM_UPPER_BOUNDS_SECONDS.len()],
        }
    }

    /// Returns the fixed storage format version.
    pub const fn version(&self) -> u8 {
        DURATION_HISTOGRAM_VERSION
    }

    /// Returns the count in each fixed bin in ascending upper-bound order.
    pub const fn bins(&self) -> &[u64; DURATION_HISTOGRAM_UPPER_BOUNDS_SECONDS.len()] {
        &self.bins
    }

    /// Adds one duration to its fixed bin, saturating the final bin at one day.
    pub fn record(&mut self, duration_seconds: u64) -> Result<(), StatisticsError> {
        let index = duration_bin_index(duration_seconds)?;
        self.bins[index] = self.bins[index]
            .checked_add(1)
            .ok_or(StatisticsError::CounterOverflow)?;
        Ok(())
    }

    /// Returns the lower median's fixed-bin upper-bound estimate.
    pub fn median_seconds_estimate(&self) -> Option<u64> {
        let total = self
            .bins
            .iter()
            .try_fold(0_u64, |sum, count| sum.checked_add(*count))?;
        if total == 0 {
            return None;
        }
        let target = total.div_ceil(2);
        let mut cumulative = 0_u64;
        for (index, count) in self.bins.iter().enumerate() {
            cumulative = cumulative.checked_add(*count)?;
            if cumulative >= target {
                return Some(DURATION_HISTOGRAM_UPPER_BOUNDS_SECONDS[index]);
            }
        }
        None
    }

    /// Captures the exact server-side persistence representation.
    pub fn snapshot(&self) -> DurationHistogramSnapshot {
        DurationHistogramSnapshot {
            version: self.version(),
            bins: self.bins.to_vec(),
        }
    }

    /// Restores a validated fixed histogram from server persistence.
    pub fn restore(snapshot: &DurationHistogramSnapshot) -> Result<Self, StatisticsError> {
        if snapshot.version != DURATION_HISTOGRAM_VERSION {
            return Err(StatisticsError::HistogramVersionMismatch);
        }
        let bins: [u64; DURATION_HISTOGRAM_UPPER_BOUNDS_SECONDS.len()] = snapshot
            .bins
            .clone()
            .try_into()
            .map_err(|_| StatisticsError::HistogramBinCountMismatch)?;
        Ok(Self { bins })
    }

    fn merge(&self, other: &Self) -> Result<Self, StatisticsError> {
        let mut bins = [0_u64; DURATION_HISTOGRAM_UPPER_BOUNDS_SECONDS.len()];
        for (target, (left, right)) in bins.iter_mut().zip(self.bins.iter().zip(other.bins.iter()))
        {
            *target = left
                .checked_add(*right)
                .ok_or(StatisticsError::CounterOverflow)?;
        }
        Ok(Self { bins })
    }
}

impl Default for DurationHistogram {
    fn default() -> Self {
        Self::empty()
    }
}

/// Non-wire stable moments required to merge a Pearson calculation.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PearsonMomentSnapshot {
    /// Number of paired question/rest scores.
    pub count: u64,
    /// Running mean of normalized question score.
    pub mean_x: f64,
    /// Running mean of rest-of-run score.
    pub mean_y: f64,
    /// Sum of squared deviations for question score.
    pub m2_x: f64,
    /// Sum of squared deviations for rest score.
    pub m2_y: f64,
    /// Sum of paired centered products.
    pub co_moment: f64,
}

/// Stable, mergeable moments for a Pearson discrimination calculation.
///
/// Inputs are bounded to `0.0..=1.0`. The representation uses the parallel
/// Welford/Chan update and merge formulas rather than raw sum-of-squares, so
/// ordinary large bounded cohorts do not lose variance through cancellation.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PearsonSufficientSums {
    count: u64,
    mean_x: f64,
    mean_y: f64,
    m2_x: f64,
    m2_y: f64,
    co_moment: f64,
}

impl PearsonSufficientSums {
    /// Creates zero correlation terms.
    pub const fn empty() -> Self {
        Self {
            count: 0,
            mean_x: 0.0,
            mean_y: 0.0,
            m2_x: 0.0,
            m2_y: 0.0,
            co_moment: 0.0,
        }
    }

    /// Adds one validated question-score/rest-score pair.
    pub fn record(&mut self, x: f64, y: f64) -> Result<(), StatisticsError> {
        validate_score(x)?;
        validate_score(y)?;
        *self = self.merge(&Self::single(x, y))?;
        Ok(())
    }

    /// Returns the sample count represented by these terms.
    pub const fn count(&self) -> u64 {
        self.count
    }

    /// Calculates Pearson's r when the terms have nonzero variance.
    pub fn pearson_r(&self) -> Option<f64> {
        if self.count < 2 {
            return None;
        }
        if !self.co_moment.is_finite()
            || !self.m2_x.is_finite()
            || !self.m2_y.is_finite()
            || self.m2_x <= 0.0
            || self.m2_y <= 0.0
        {
            return None;
        }
        let denominator = (self.m2_x * self.m2_y).sqrt();
        if !denominator.is_finite() || denominator <= 0.0 {
            return None;
        }
        let correlation = self.co_moment / denominator;
        correlation
            .is_finite()
            .then(|| correlation.clamp(-1.0, 1.0))
    }

    /// Captures exact stable sufficient terms for server persistence.
    pub const fn snapshot(&self) -> PearsonMomentSnapshot {
        PearsonMomentSnapshot {
            count: self.count,
            mean_x: self.mean_x,
            mean_y: self.mean_y,
            m2_x: self.m2_x,
            m2_y: self.m2_y,
            co_moment: self.co_moment,
        }
    }

    /// Restores validated stable terms from server persistence.
    pub fn restore(snapshot: PearsonMomentSnapshot) -> Result<Self, StatisticsError> {
        validate_pearson_snapshot(snapshot)?;
        Ok(Self {
            count: snapshot.count,
            mean_x: snapshot.mean_x,
            mean_y: snapshot.mean_y,
            m2_x: snapshot.m2_x,
            m2_y: snapshot.m2_y,
            co_moment: snapshot.co_moment,
        })
    }

    fn single(x: f64, y: f64) -> Self {
        Self {
            count: 1,
            mean_x: x,
            mean_y: y,
            m2_x: 0.0,
            m2_y: 0.0,
            co_moment: 0.0,
        }
    }

    fn merge(&self, other: &Self) -> Result<Self, StatisticsError> {
        if self.count == 0 {
            return Ok(*other);
        }
        if other.count == 0 {
            return Ok(*self);
        }
        let count = self
            .count
            .checked_add(other.count)
            .ok_or(StatisticsError::CounterOverflow)?;
        let left_count = self.count as f64;
        let right_count = other.count as f64;
        let total_count = count as f64;
        let factor = left_count * right_count / total_count;
        let delta_x = other.mean_x - self.mean_x;
        let delta_y = other.mean_y - self.mean_y;
        let merged = Self {
            count,
            mean_x: self.mean_x + delta_x * right_count / total_count,
            mean_y: self.mean_y + delta_y * right_count / total_count,
            m2_x: self.m2_x + other.m2_x + delta_x * delta_x * factor,
            m2_y: self.m2_y + other.m2_y + delta_y * delta_y * factor,
            co_moment: self.co_moment + other.co_moment + delta_x * delta_y * factor,
        };
        validate_pearson_snapshot(merged.snapshot())?;
        Ok(merged)
    }
}

impl Default for PearsonSufficientSums {
    fn default() -> Self {
        Self::empty()
    }
}

/// Non-wire, identity-free persistence snapshot of one Question Cohort Rollup.
#[derive(Debug, Clone, PartialEq)]
pub struct QuestionCohortRollupSnapshot {
    /// Number of first-completed-run cohort contributions.
    pub cohort_size: u64,
    /// Sum of normalized question scores.
    pub score_sum: f64,
    /// Sum of attempts represented by cohort observations.
    pub attempts_sum: u64,
    /// Exact fixed-bin duration representation.
    pub durations: DurationHistogramSnapshot,
    /// Exact stable Pearson sufficient terms.
    pub discrimination: PearsonMomentSnapshot,
}

/// Incremental Question Cohort Rollup for one immutable Question Revision.
#[derive(Debug, Clone, PartialEq)]
pub struct QuestionCohortRollup {
    cohort_size: u64,
    score_sum: f64,
    attempts_sum: u64,
    durations: DurationHistogram,
    discrimination: PearsonSufficientSums,
}

impl QuestionCohortRollup {
    /// Creates an empty cohort rollup in the current format.
    pub const fn empty() -> Self {
        Self {
            cohort_size: 0,
            score_sum: 0.0,
            attempts_sum: 0,
            durations: DurationHistogram::empty(),
            discrimination: PearsonSufficientSums::empty(),
        }
    }

    /// Records one cohort-rollup observation.
    pub fn record(
        &mut self,
        observation: QuestionCohortRollupObservation,
    ) -> Result<(), StatisticsError> {
        let cohort_size = self
            .cohort_size
            .checked_add(1)
            .ok_or(StatisticsError::CounterOverflow)?;
        let score_sum = checked_sum(self.score_sum, observation.normalized_score())?;
        let attempts_sum = self
            .attempts_sum
            .checked_add(observation.attempts())
            .ok_or(StatisticsError::CounterOverflow)?;
        let mut durations = self.durations.clone();
        durations.record(observation.duration_seconds())?;
        let mut discrimination = self.discrimination;
        if let Some(rest_score) = observation.rest_score() {
            discrimination.record(observation.normalized_score(), rest_score)?;
        }
        *self = Self {
            cohort_size,
            score_sum,
            attempts_sum,
            durations,
            discrimination,
        };
        Ok(())
    }

    /// Returns an identity-free snapshot suitable for server-only persistence.
    pub fn snapshot(&self) -> QuestionCohortRollupSnapshot {
        QuestionCohortRollupSnapshot {
            cohort_size: self.cohort_size,
            score_sum: self.score_sum,
            attempts_sum: self.attempts_sum,
            durations: self.durations.snapshot(),
            discrimination: self.discrimination.snapshot(),
        }
    }

    /// Restores a fully validated cohort rollup from server-only persistence.
    pub fn restore(snapshot: &QuestionCohortRollupSnapshot) -> Result<Self, StatisticsError> {
        validate_aggregate_snapshot(snapshot)?;
        Ok(Self {
            cohort_size: snapshot.cohort_size,
            score_sum: snapshot.score_sum,
            attempts_sum: snapshot.attempts_sum,
            durations: DurationHistogram::restore(&snapshot.durations)?,
            discrimination: PearsonSufficientSums::restore(snapshot.discrimination)?,
        })
    }

    /// Atomically merges another validated server snapshot into this cohort rollup.
    pub fn merge_snapshot(
        &mut self,
        snapshot: &QuestionCohortRollupSnapshot,
    ) -> Result<(), StatisticsError> {
        let other = Self::restore(snapshot)?;
        let cohort_size = self
            .cohort_size
            .checked_add(other.cohort_size)
            .ok_or(StatisticsError::CounterOverflow)?;
        let score_sum = checked_sum(self.score_sum, other.score_sum)?;
        let attempts_sum = self
            .attempts_sum
            .checked_add(other.attempts_sum)
            .ok_or(StatisticsError::CounterOverflow)?;
        let durations = self.durations.merge(&other.durations)?;
        let discrimination = self.discrimination.merge(&other.discrimination)?;
        *self = Self {
            cohort_size,
            score_sum,
            attempts_sum,
            durations,
            discrimination,
        };
        Ok(())
    }

    /// Returns the number of independent cohort contributions.
    pub const fn cohort_size(&self) -> u64 {
        self.cohort_size
    }

    /// Returns mean normalized question score, when observations exist.
    pub fn difficulty_index(&self) -> Option<f64> {
        self.mean(self.score_sum)
    }

    /// Returns mean submitted attempts, when observations exist.
    pub fn attempts_mean(&self) -> Option<f64> {
        self.mean(self.attempts_sum as f64)
    }

    /// Returns the fixed-bin median duration estimate, when observations exist.
    pub fn time_median_seconds_estimate(&self) -> Option<u64> {
        self.durations.median_seconds_estimate()
    }

    /// Returns rest-score Pearson discrimination, when calculable.
    pub fn discrimination_index(&self) -> Option<f64> {
        self.discrimination.pearson_r()
    }

    /// Returns the number of observations eligible for discrimination only.
    pub const fn scored_cohort_size(&self) -> u64 {
        self.discrimination.count()
    }

    /// Returns the cohort rollup's fixed duration histogram.
    pub const fn durations(&self) -> &DurationHistogram {
        &self.durations
    }

    fn mean(&self, sum: f64) -> Option<f64> {
        if self.cohort_size == 0 || !sum.is_finite() {
            return None;
        }
        let mean = sum / self.cohort_size as f64;
        mean.is_finite().then_some(mean)
    }
}

impl Default for QuestionCohortRollup {
    fn default() -> Self {
        Self::empty()
    }
}

fn duration_bin_index(duration_seconds: u64) -> Result<usize, StatisticsError> {
    let duration_seconds = duration_seconds.min(MAX_DURATION_SECONDS);
    DURATION_HISTOGRAM_UPPER_BOUNDS_SECONDS
        .iter()
        .position(|upper_bound| duration_seconds <= *upper_bound)
        .ok_or(StatisticsError::CounterOverflow)
}

fn validate_score(value: f64) -> Result<(), StatisticsError> {
    if !value.is_finite() {
        return Err(StatisticsError::NonFiniteScalar);
    }
    if !(0.0..=1.0).contains(&value) || is_negative_zero(value) {
        return Err(StatisticsError::ScoreOutOfRange);
    }
    Ok(())
}

fn is_negative_zero(value: f64) -> bool {
    value == 0.0 && value.is_sign_negative()
}

fn checked_sum(left: f64, right: f64) -> Result<f64, StatisticsError> {
    let sum = left + right;
    sum.is_finite()
        .then_some(sum)
        .ok_or(StatisticsError::AggregateOverflow)
}

fn validate_aggregate_snapshot(
    snapshot: &QuestionCohortRollupSnapshot,
) -> Result<(), StatisticsError> {
    let durations = DurationHistogram::restore(&snapshot.durations)?;
    let duration_count = durations
        .bins()
        .iter()
        .try_fold(0_u64, |sum, count| sum.checked_add(*count))
        .ok_or(StatisticsError::CounterOverflow)?;
    if !snapshot.score_sum.is_finite()
        || snapshot.score_sum < 0.0
        || is_negative_zero(snapshot.score_sum)
        || snapshot.score_sum > snapshot.cohort_size as f64
        || duration_count != snapshot.cohort_size
        || snapshot.attempts_sum < snapshot.cohort_size
        || snapshot.discrimination.count > snapshot.cohort_size
    {
        return Err(StatisticsError::SnapshotInvariant);
    }
    if snapshot.cohort_size == 0
        && (snapshot.score_sum != 0.0
            || snapshot.attempts_sum != 0
            || snapshot.discrimination.count != 0)
    {
        return Err(StatisticsError::SnapshotInvariant);
    }
    validate_pearson_snapshot(snapshot.discrimination)?;

    // The paired x scores are a subset of the cohort's question scores.  The
    // remaining bounded scores can contribute only zero through one each, so
    // this catches cross-field corruption that each scalar's local bounds
    // cannot detect.  Retain a small machine-scaled tolerance for valid
    // persisted floating-point sums.
    let paired_sum = snapshot.discrimination.count as f64 * snapshot.discrimination.mean_x;
    let unpaired_count = snapshot.cohort_size - snapshot.discrimination.count;
    let lower_bound = paired_sum;
    let upper_bound = paired_sum + unpaired_count as f64;
    let tolerance = f64::EPSILON * 256.0 * snapshot.cohort_size.max(1) as f64;
    if snapshot.score_sum + tolerance < lower_bound || snapshot.score_sum > upper_bound + tolerance
    {
        return Err(StatisticsError::SnapshotInvariant);
    }
    Ok(())
}

fn validate_pearson_snapshot(snapshot: PearsonMomentSnapshot) -> Result<(), StatisticsError> {
    let scalars = [
        snapshot.mean_x,
        snapshot.mean_y,
        snapshot.m2_x,
        snapshot.m2_y,
        snapshot.co_moment,
    ];
    if scalars.iter().any(|value| !value.is_finite())
        || !(0.0..=1.0).contains(&snapshot.mean_x)
        || !(0.0..=1.0).contains(&snapshot.mean_y)
        || snapshot.m2_x < 0.0
        || snapshot.m2_y < 0.0
        || scalars.iter().any(|value| is_negative_zero(*value))
    {
        return Err(StatisticsError::SnapshotInvariant);
    }
    if snapshot.count == 0 {
        return (snapshot.mean_x == 0.0
            && snapshot.mean_y == 0.0
            && snapshot.m2_x == 0.0
            && snapshot.m2_y == 0.0
            && snapshot.co_moment == 0.0)
            .then_some(())
            .ok_or(StatisticsError::SnapshotInvariant);
    }
    if snapshot.count == 1
        && (snapshot.m2_x != 0.0 || snapshot.m2_y != 0.0 || snapshot.co_moment != 0.0)
    {
        return Err(StatisticsError::SnapshotInvariant);
    }
    let count = snapshot.count as f64;
    let tolerance = f64::EPSILON * 128.0 * count;
    let maximum_m2_x = count * snapshot.mean_x * (1.0 - snapshot.mean_x);
    let maximum_m2_y = count * snapshot.mean_y * (1.0 - snapshot.mean_y);
    let covariance_bound = (snapshot.m2_x * snapshot.m2_y).sqrt();
    if !covariance_bound.is_finite()
        || snapshot.m2_x > maximum_m2_x + tolerance
        || snapshot.m2_y > maximum_m2_y + tolerance
        || snapshot.co_moment.abs() > covariance_bound + tolerance
    {
        return Err(StatisticsError::SnapshotInvariant);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn failed_merge_does_not_leave_a_partial_aggregate() {
        let mut aggregate = QuestionCohortRollup {
            cohort_size: u64::MAX,
            score_sum: 1.0,
            attempts_sum: 1,
            durations: DurationHistogram::empty(),
            discrimination: PearsonSufficientSums::empty(),
        };
        let before = aggregate.clone();
        let observation = QuestionCohortRollupObservation::new(0.5, 1, 5, Some(0.5))
            .expect("test observation should be valid");

        assert_eq!(
            aggregate.record(observation),
            Err(StatisticsError::CounterOverflow)
        );
        assert_eq!(aggregate, before);
    }
}

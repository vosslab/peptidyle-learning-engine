//! Shared score rounding at the persistence boundary.

const PERSISTED_DECIMAL_PLACES: u32 = 4;

/// Rounds a finite score to the repository's persisted precision.
///
/// [`f64::round`] defines exact midpoint ties away from zero. Callers validate
/// finiteness and domain bounds before this storage-boundary conversion.
pub(crate) fn round_for_persistence(value: f64) -> f64 {
    let scale = 10_f64.powi(PERSISTED_DECIMAL_PLACES as i32);
    let rounded = (value * scale).round() / scale;
    if rounded == 0.0 { 0.0 } else { rounded }
}

#[cfg(test)]
mod tests {
    use super::round_for_persistence;

    #[test]
    fn rounds_to_four_places_with_midpoints_away_from_zero() {
        assert_eq!(round_for_persistence(8.000_000_000_000_6), 8.0);
        assert_eq!(round_for_persistence(0.080_000_000_000_006), 0.08);
        assert_eq!(round_for_persistence(0.001_24), 0.0012);
        assert_eq!(round_for_persistence(-0.001_24), -0.0012);
        assert_eq!(round_for_persistence(0.001_25), 0.0013);
        assert_eq!(round_for_persistence(-0.001_25), -0.0013);
        assert_eq!(round_for_persistence(0.833_35), 0.8334);
        assert_eq!(round_for_persistence(-0.833_35), -0.8334);
        assert!(!round_for_persistence(-0.000_01).is_sign_negative());
    }
}

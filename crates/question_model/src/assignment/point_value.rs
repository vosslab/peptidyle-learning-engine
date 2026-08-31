//! Exact Assignment-owned point values.

use std::str::FromStr;

use serde::{Deserialize, Serialize};

const POINT_SCALE: i64 = 10_000;
const MAX_WHOLE_POINTS: i64 = 1_000_000_000;
const MAX_SCALED_POINTS: i64 = MAX_WHOLE_POINTS * POINT_SCALE + (POINT_SCALE - 1);

/// Exact Assignment-owned point value with four decimal places of precision.
///
/// JSON represents this as a decimal string so JavaScript cannot silently
/// round an instructor-authored value before it reaches PostgreSQL `NUMERIC`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct AssignmentPointValue(i64);

impl AssignmentPointValue {
    /// Zero points, used for remediation without retiring the item.
    pub const ZERO: Self = Self(0);

    /// Rebuilds an exact point value from its fixed four-decimal-place integer.
    pub fn from_scaled(value: i64) -> Option<Self> {
        (0..=MAX_SCALED_POINTS)
            .contains(&value)
            .then_some(Self(value))
    }

    /// Builds an exact whole-number point value.
    pub fn from_whole(value: u32) -> Self {
        Self(i64::from(value) * POINT_SCALE)
    }

    /// Returns the fixed four-decimal-place storage integer.
    pub fn scaled(self) -> i64 {
        self.0
    }

    /// Adds two exact point values when their sum remains representable.
    pub fn checked_add(self, other: Self) -> Option<Self> {
        self.0.checked_add(other.0).and_then(Self::from_scaled)
    }

    /// Multiplies an exact point value by a nonnegative item count.
    pub fn checked_mul_u32(self, multiplier: u32) -> Option<Self> {
        self.0
            .checked_mul(i64::from(multiplier))
            .and_then(Self::from_scaled)
    }
}

impl std::fmt::Display for AssignmentPointValue {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let whole = self.0 / POINT_SCALE;
        let fractional = self.0 % POINT_SCALE;
        if fractional == 0 {
            write!(formatter, "{whole}")
        } else {
            let value = format!("{whole}.{fractional:04}");
            formatter.write_str(value.trim_end_matches('0'))
        }
    }
}

impl FromStr for AssignmentPointValue {
    type Err = &'static str;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        if value.is_empty() || value.starts_with('-') || value.starts_with('+') {
            return Err("points must be a nonnegative decimal value");
        }
        let (whole, fractional) = value.split_once('.').unwrap_or((value, ""));
        if whole.is_empty()
            || whole.len() > 10
            || !whole.bytes().all(|byte| byte.is_ascii_digit())
            || fractional.len() > 4
            || !fractional.bytes().all(|byte| byte.is_ascii_digit())
        {
            return Err("points must have at most four decimal places");
        }
        let whole = whole
            .parse::<i64>()
            .map_err(|_| "points are outside the supported range")?;
        if whole > MAX_WHOLE_POINTS {
            return Err("points are outside the supported range");
        }
        let mut fraction = fractional.to_string();
        while fraction.len() < 4 {
            fraction.push('0');
        }
        let fraction = if fraction.is_empty() {
            0
        } else {
            fraction
                .parse::<i64>()
                .map_err(|_| "points are outside the supported range")?
        };
        whole
            .checked_mul(POINT_SCALE)
            .and_then(|scaled| scaled.checked_add(fraction))
            .and_then(Self::from_scaled)
            .ok_or("points are outside the supported range")
    }
}

impl TryFrom<String> for AssignmentPointValue {
    type Error = &'static str;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        value.parse()
    }
}

impl From<AssignmentPointValue> for String {
    fn from(value: AssignmentPointValue) -> Self {
        value.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::{AssignmentPointValue, MAX_SCALED_POINTS};

    #[test]
    fn point_values_round_trip_without_binary_floating_point() {
        for value in ["0", "1", "2.5", "100.125", "0.0001"] {
            let points: AssignmentPointValue = value.parse().expect("valid exact points");
            let json = serde_json::to_string(&points).expect("points serialize");
            let decoded: AssignmentPointValue =
                serde_json::from_str(&json).expect("points deserialize");
            assert_eq!(decoded, points);
            assert_eq!(decoded.to_string(), value);
        }
        for invalid in ["", "-1", "+1", "1.00001", "NaN", "1000000001"] {
            assert!(
                invalid.parse::<AssignmentPointValue>().is_err(),
                "{invalid}"
            );
        }
    }

    #[test]
    fn point_values_add_and_multiply_exact_scaled_values() {
        let one_and_a_half = AssignmentPointValue::from_scaled(15_000).expect("in range");
        let quarter = AssignmentPointValue::from_scaled(2_500).expect("in range");

        assert_eq!(
            one_and_a_half
                .checked_add(quarter)
                .map(AssignmentPointValue::scaled),
            Some(17_500)
        );
        assert_eq!(
            quarter.checked_mul_u32(6).map(AssignmentPointValue::scaled),
            Some(15_000)
        );
    }

    #[test]
    fn point_values_reject_negative_and_overflowing_scaled_arithmetic() {
        let maximum =
            AssignmentPointValue::from_scaled(MAX_SCALED_POINTS).expect("maximum is valid");
        let smallest = AssignmentPointValue::from_scaled(1).expect("smallest is valid");

        assert!(AssignmentPointValue::from_scaled(-1).is_none());
        assert!(
            AssignmentPointValue::from_scaled(MAX_SCALED_POINTS.checked_add(1).expect("i64 room"))
                .is_none()
        );
        assert!(maximum.checked_add(smallest).is_none());
        assert!(maximum.checked_mul_u32(2).is_none());
    }
}

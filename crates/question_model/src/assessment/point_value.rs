//! Exact Assessment-owned point values.

use std::str::FromStr;

use serde::{Deserialize, Serialize};

const POINT_SCALE: i64 = 10_000;
const MAX_WHOLE_POINTS: i64 = 1_000_000_000;
const MAX_SCALED_POINTS: i64 = MAX_WHOLE_POINTS * POINT_SCALE + (POINT_SCALE - 1);

/// Exact Assessment-owned point value with four decimal places of precision.
///
/// JSON represents this as a decimal string so JavaScript cannot silently
/// round an instructor-authored value before it reaches PostgreSQL `NUMERIC`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct AssessmentPointValue(i64);

impl AssessmentPointValue {
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

impl std::fmt::Display for AssessmentPointValue {
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

impl FromStr for AssessmentPointValue {
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

impl TryFrom<String> for AssessmentPointValue {
    type Error = &'static str;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        value.parse()
    }
}

impl From<AssessmentPointValue> for String {
    fn from(value: AssessmentPointValue) -> Self {
        value.to_string()
    }
}

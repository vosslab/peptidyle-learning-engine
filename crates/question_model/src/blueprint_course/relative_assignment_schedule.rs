//! Curriculum-relative schedule value contracts for Blueprint Courses.

use chrono::NaiveTime;
use serde::{Deserialize, Serialize};

use super::BlueprintCourseValidationError;

/// Exact local wall-clock time used with a signed curriculum-day offset.
///
/// This is intentionally time-only: B2 resolves it against an instructor's
/// selected target term and reports any daylight-saving correction required.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct LocalTimeOfDay(String);

impl LocalTimeOfDay {
    /// Parses the canonical `HH:MM:SS.sss` browser wire value.
    pub fn parse(value: &str) -> Result<Self, LocalTimeOfDayError> {
        let bytes = value.as_bytes();
        let exact_shape = bytes.len() == 12
            && bytes[2] == b':'
            && bytes[5] == b':'
            && bytes[8] == b'.'
            && bytes
                .iter()
                .enumerate()
                .all(|(index, byte)| matches!(index, 2 | 5 | 8) || byte.is_ascii_digit());
        if !exact_shape || NaiveTime::parse_from_str(value, "%H:%M:%S%.3f").is_err() {
            return Err(LocalTimeOfDayError);
        }
        Ok(Self(value.to_owned()))
    }

    /// Returns the canonical browser wire value.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl TryFrom<String> for LocalTimeOfDay {
    type Error = LocalTimeOfDayError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::parse(&value)
    }
}

impl From<LocalTimeOfDay> for String {
    fn from(value: LocalTimeOfDay) -> Self {
        value.0
    }
}

/// A local time was not the exact time-only browser wire form.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LocalTimeOfDayError;

impl std::fmt::Display for LocalTimeOfDayError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("local time must be exact HH:MM:SS.sss")
    }
}

impl std::error::Error for LocalTimeOfDayError {}

/// One curriculum-calendar moment relative to a target term's first day.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct RelativeAssignmentScheduleMoment {
    /// Signed calendar-day offset from the target term's first local day.
    pub day_offset: i32,
    /// Exact local wall-clock time for that calendar day.
    pub local_time: LocalTimeOfDay,
}

/// Optional curriculum-relative availability, due, and close defaults.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct RelativeAssignmentSchedule {
    /// First local moment when students may open a future copied assignment.
    pub available_at: Option<RelativeAssignmentScheduleMoment>,
    /// Ordinary local due moment for a future copied assignment.
    pub due_at: Option<RelativeAssignmentScheduleMoment>,
    /// Local moment after which a future copied assignment is closed.
    pub closes_at: Option<RelativeAssignmentScheduleMoment>,
}

impl RelativeAssignmentSchedule {
    /// Validates the partial schedule's meaningful chronological order.
    pub fn validate(&self) -> Result<(), BlueprintCourseValidationError> {
        if ordered_after(self.available_at.as_ref(), self.due_at.as_ref())
            || ordered_after(self.available_at.as_ref(), self.closes_at.as_ref())
            || ordered_after(self.due_at.as_ref(), self.closes_at.as_ref())
        {
            return Err(BlueprintCourseValidationError::InvalidScheduleOrder);
        }
        Ok(())
    }
}

fn ordered_after(
    earlier: Option<&RelativeAssignmentScheduleMoment>,
    later: Option<&RelativeAssignmentScheduleMoment>,
) -> bool {
    earlier
        .zip(later)
        .is_some_and(|(earlier, later)| earlier > later)
}

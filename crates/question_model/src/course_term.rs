//! Validated inclusive calendar bounds for a teaching course.

use std::error::Error;
use std::fmt::{self, Display, Formatter};
use std::str::FromStr;

use chrono::{Months, NaiveDate};
use serde::{Deserialize, Serialize};

const ACTIVE_LIFETIME_MONTHS: u32 = 6;

/// One exact proleptic-Gregorian calendar date serialized as `YYYY-MM-DD`.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct CourseDate(String);

impl CourseDate {
    /// Parses one exact, four-digit-year calendar date.
    pub fn parse(value: &str) -> Result<Self, CourseDateError> {
        let bytes = value.as_bytes();
        let exact_shape = bytes.len() == 10
            && bytes[4] == b'-'
            && bytes[7] == b'-'
            && bytes
                .iter()
                .enumerate()
                .all(|(index, byte)| index == 4 || index == 7 || byte.is_ascii_digit());
        if !exact_shape {
            return Err(CourseDateError);
        }
        let year = value[0..4].parse::<u16>().map_err(|_| CourseDateError)?;
        if year == 0 || NaiveDate::parse_from_str(value, "%Y-%m-%d").is_err() {
            return Err(CourseDateError);
        }
        Ok(Self(value.to_string()))
    }

    /// Returns the exact canonical wire and database spelling.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Display for CourseDate {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

impl FromStr for CourseDate {
    type Err = CourseDateError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        Self::parse(value)
    }
}

impl TryFrom<String> for CourseDate {
    type Error = CourseDateError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::parse(&value)
    }
}

impl From<CourseDate> for String {
    fn from(value: CourseDate) -> Self {
        value.0
    }
}

/// An input is not one exact, possible `YYYY-MM-DD` calendar date.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CourseDateError;

impl Display for CourseDateError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str("course date must be an exact valid YYYY-MM-DD calendar date")
    }
}

impl Error for CourseDateError {}

/// Inclusive calendar bounds for one course.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", try_from = "CourseTermParts")]
pub struct CourseTerm {
    start_date: CourseDate,
    end_date: CourseDate,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct CourseTermParts {
    start_date: CourseDate,
    end_date: CourseDate,
}

impl TryFrom<CourseTermParts> for CourseTerm {
    type Error = CourseTermError;

    fn try_from(parts: CourseTermParts) -> Result<Self, Self::Error> {
        Self::new(parts.start_date, parts.end_date)
    }
}

impl CourseTerm {
    /// Constructs a term whose inclusive start is not after its inclusive end.
    pub fn new(start_date: CourseDate, end_date: CourseDate) -> Result<Self, CourseTermError> {
        if start_date > end_date {
            return Err(CourseTermError::EndBeforeStart);
        }
        Ok(Self {
            start_date,
            end_date,
        })
    }

    /// Parses both explicit wire values without supplying any fallback.
    pub fn from_parts(start_date: &str, end_date: &str) -> Result<Self, CourseTermError> {
        let start_date = CourseDate::parse(start_date).map_err(|_| CourseTermError::StartDate)?;
        let end_date = CourseDate::parse(end_date).map_err(|_| CourseTermError::EndDate)?;
        Self::new(start_date, end_date)
    }

    /// Inclusive first course-calendar date.
    pub fn start_date(&self) -> &CourseDate {
        &self.start_date
    }

    /// Inclusive final course-calendar date.
    pub fn end_date(&self) -> &CourseDate {
        &self.end_date
    }

    /// Rejects an inclusive end after the UTC calendar date six months after creation.
    ///
    /// The cutoff date stays inside the Active lifetime. A stored term may span more than six
    /// months when its start is earlier than the creation date.
    pub fn ensure_within_active_lifetime(
        &self,
        created_on: &CourseDate,
    ) -> Result<(), CourseTermError> {
        let cutoff = active_lifetime_end(created_on)?;
        if self.end_date > cutoff {
            return Err(CourseTermError::EndAfterActiveLifetime);
        }
        Ok(())
    }
}

fn active_lifetime_end(created_on: &CourseDate) -> Result<CourseDate, CourseTermError> {
    let created = NaiveDate::parse_from_str(created_on.as_str(), "%Y-%m-%d")
        .map_err(|_| CourseTermError::StartDate)?;
    let cutoff = created
        .checked_add_months(Months::new(ACTIVE_LIFETIME_MONTHS))
        .ok_or(CourseTermError::EndAfterActiveLifetime)?;
    CourseDate::parse(&cutoff.format("%Y-%m-%d").to_string()).map_err(|_| CourseTermError::EndDate)
}

/// The concrete invalid component of a proposed course term.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CourseTermError {
    /// Start date is not an exact possible calendar date.
    StartDate,
    /// End date is not an exact possible calendar date.
    EndDate,
    /// Inclusive end precedes inclusive start.
    EndBeforeStart,
    /// Inclusive end is after the UTC calendar date six months after creation.
    EndAfterActiveLifetime,
}

impl Display for CourseTermError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::StartDate => "course term start date is invalid",
            Self::EndDate => "course term end date is invalid",
            Self::EndBeforeStart => "course term end date is before its start date",
            Self::EndAfterActiveLifetime => {
                "course term end date is after its six-month Active lifetime"
            }
        })
    }
}

impl Error for CourseTermError {}

/// Stable public code for the bounded course-term validation response.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum CourseTermFailureCode {
    /// The submitted course term cannot be accepted.
    CourseTermInvalid,
}

/// Input field the Instructor needs to correct.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum CourseTermField {
    /// The required nested term object.
    Term,
    /// Inclusive start date.
    StartDate,
    /// Inclusive end date.
    EndDate,
}

/// Stable reason the submitted field was refused.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum CourseTermFailureReason {
    /// A required field is absent.
    Required,
    /// A date is malformed or impossible.
    InvalidCalendarDate,
    /// Inclusive end precedes inclusive start.
    EndBeforeStart,
}

/// Answer-free, bounded correction contract for course creation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CourseTermValidationFailure {
    /// Stable response discriminator.
    pub error: CourseTermFailureCode,
    /// Field the Instructor needs to correct.
    pub field: CourseTermField,
    /// Machine-readable refusal reason.
    pub reason: CourseTermFailureReason,
    /// Short correction guidance that never echoes submitted text.
    pub message: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exact_calendar_dates_round_trip_without_becoming_instants() {
        for date in ["0001-01-01", "2026-08-24", "2028-02-29", "9999-12-31"] {
            let parsed: CourseDate =
                serde_json::from_str(&format!("\"{date}\"")).expect("valid course date");
            assert_eq!(parsed.as_str(), date);
            assert_eq!(
                serde_json::to_string(&parsed).unwrap(),
                format!("\"{date}\"")
            );
        }
    }

    #[test]
    fn malformed_and_impossible_calendar_dates_are_rejected() {
        for date in [
            "2026-2-03",
            "2026-02-30",
            "0000-01-01",
            "10000-01-01",
            "2026-01-01T00:00:00Z",
        ] {
            assert!(
                CourseDate::parse(date).is_err(),
                "unexpected valid date: {date}"
            );
            assert!(serde_json::from_str::<CourseDate>(&format!("\"{date}\"")).is_err());
        }
    }

    #[test]
    fn course_term_bounds_are_inclusive_and_ordered() {
        CourseTerm::from_parts("2026-08-24", "2026-08-24").expect("one-day inclusive term");
        assert_eq!(
            CourseTerm::from_parts("2026-08-25", "2026-08-24"),
            Err(CourseTermError::EndBeforeStart)
        );
    }

    #[test]
    fn active_lifetime_includes_the_cutoff_date_and_rejects_the_next_day() {
        let created_on = CourseDate::parse("2026-01-31").expect("creation date");
        let through_cutoff =
            CourseTerm::from_parts("2025-08-01", "2026-07-31").expect("term ending on the cutoff");
        through_cutoff
            .ensure_within_active_lifetime(&created_on)
            .expect("the cutoff date remains Active");
        let past_cutoff = CourseTerm::from_parts("2026-01-31", "2026-08-01").expect("ordered term");
        assert_eq!(
            past_cutoff.ensure_within_active_lifetime(&created_on),
            Err(CourseTermError::EndAfterActiveLifetime)
        );

        let month_end = CourseDate::parse("2026-08-31").expect("month-end creation");
        CourseTerm::from_parts("2026-08-31", "2027-02-28")
            .expect("clamped February cutoff")
            .ensure_within_active_lifetime(&month_end)
            .expect("the clamped cutoff date remains Active");
        assert_eq!(
            CourseTerm::from_parts("2026-08-31", "2027-03-01")
                .expect("day after the clamp")
                .ensure_within_active_lifetime(&month_end),
            Err(CourseTermError::EndAfterActiveLifetime)
        );

        let leap_creation = CourseDate::parse("2023-08-31").expect("leap-year creation");
        CourseTerm::from_parts("2023-08-31", "2024-02-29")
            .expect("leap-day cutoff")
            .ensure_within_active_lifetime(&leap_creation)
            .expect("the leap-day cutoff remains Active");
        assert_eq!(
            CourseTerm::from_parts("2023-08-31", "2024-03-01")
                .expect("day after the leap-day cutoff")
                .ensure_within_active_lifetime(&leap_creation),
            Err(CourseTermError::EndAfterActiveLifetime)
        );
    }

    #[test]
    fn stored_terms_longer_than_six_months_remain_valid_course_terms() {
        CourseTerm::from_parts("2026-01-01", "2026-12-31").expect("stored year-long term");
    }

    #[test]
    fn deserialization_cannot_construct_a_reversed_term() {
        let result = serde_json::from_str::<CourseTerm>(
            r#"{"startDate":"2026-08-25","endDate":"2026-08-24"}"#,
        );
        assert!(result.is_err());
        assert!(
            serde_json::from_str::<CourseTerm>(
                r#"{"startDate":"2026-08-24","endDate":"2026-12-18","unexpected":"value"}"#,
            )
            .is_err()
        );
    }
}

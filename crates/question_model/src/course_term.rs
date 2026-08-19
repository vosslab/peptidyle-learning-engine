//! Validated calendar bounds and scheduling zone for a teaching course.
//!
//! These values deliberately carry a calendar date and an IANA zone name, not
//! an instant or UTC offset. Later scheduling packages own local-time
//! resolution and daylight-saving refusal.

use std::error::Error;
use std::fmt::{self, Display, Formatter};
use std::str::FromStr;

use chrono::NaiveDate;
use serde::{Deserialize, Serialize};

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

/// One exact case-sensitive name in the embedded IANA time-zone database.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct IanaTimeZone(String);

impl IanaTimeZone {
    /// Validates exact case-sensitive membership in the IANA database.
    pub fn parse(value: &str) -> Result<Self, IanaTimeZoneError> {
        let parsed = value
            .parse::<chrono_tz::Tz>()
            .map_err(|_| IanaTimeZoneError)?;
        if parsed.name() != value {
            return Err(IanaTimeZoneError);
        }
        Ok(Self(value.to_string()))
    }

    /// Returns the exact stable IANA name.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Display for IanaTimeZone {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

impl FromStr for IanaTimeZone {
    type Err = IanaTimeZoneError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        Self::parse(value)
    }
}

impl TryFrom<String> for IanaTimeZone {
    type Error = IanaTimeZoneError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::parse(&value)
    }
}

impl From<IanaTimeZone> for String {
    fn from(value: IanaTimeZone) -> Self {
        value.0
    }
}

/// An input is not an exact known IANA time-zone name.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IanaTimeZoneError;

impl Display for IanaTimeZoneError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str("course time zone must be an exact known IANA name")
    }
}

impl Error for IanaTimeZoneError {}

/// Inclusive calendar bounds and authoritative scheduling zone for one course.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", try_from = "CourseTermParts")]
pub struct CourseTerm {
    start_date: CourseDate,
    end_date: CourseDate,
    time_zone: IanaTimeZone,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct CourseTermParts {
    start_date: CourseDate,
    end_date: CourseDate,
    time_zone: IanaTimeZone,
}

impl TryFrom<CourseTermParts> for CourseTerm {
    type Error = CourseTermError;

    fn try_from(parts: CourseTermParts) -> Result<Self, Self::Error> {
        Self::new(parts.start_date, parts.end_date, parts.time_zone)
    }
}

impl CourseTerm {
    /// Constructs a term whose inclusive start is not after its inclusive end.
    pub fn new(
        start_date: CourseDate,
        end_date: CourseDate,
        time_zone: IanaTimeZone,
    ) -> Result<Self, CourseTermError> {
        if start_date > end_date {
            return Err(CourseTermError::EndBeforeStart);
        }
        Ok(Self {
            start_date,
            end_date,
            time_zone,
        })
    }

    /// Parses all three explicit wire values without supplying any fallback.
    pub fn from_parts(
        start_date: &str,
        end_date: &str,
        time_zone: &str,
    ) -> Result<Self, CourseTermError> {
        let start_date = CourseDate::parse(start_date).map_err(|_| CourseTermError::StartDate)?;
        let end_date = CourseDate::parse(end_date).map_err(|_| CourseTermError::EndDate)?;
        let time_zone = IanaTimeZone::parse(time_zone).map_err(|_| CourseTermError::TimeZone)?;
        Self::new(start_date, end_date, time_zone)
    }

    /// Inclusive first course-calendar date.
    pub fn start_date(&self) -> &CourseDate {
        &self.start_date
    }

    /// Inclusive final course-calendar date.
    pub fn end_date(&self) -> &CourseDate {
        &self.end_date
    }

    /// Course-owned zone for later local schedule resolution.
    pub fn time_zone(&self) -> &IanaTimeZone {
        &self.time_zone
    }
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
    /// Zone is not an exact known IANA name.
    TimeZone,
}

impl Display for CourseTermError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::StartDate => "course term start date is invalid",
            Self::EndDate => "course term end date is invalid",
            Self::EndBeforeStart => "course term end date is before its start date",
            Self::TimeZone => "course term time zone is not a known IANA name",
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

/// Input field the professor needs to correct.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum CourseTermField {
    /// The required nested term object.
    Term,
    /// Inclusive start date.
    StartDate,
    /// Inclusive end date.
    EndDate,
    /// Authoritative IANA zone.
    TimeZone,
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
    /// The zone is not an exact known IANA name.
    UnknownIanaTimeZone,
}

/// Answer-free, bounded correction contract for course creation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CourseTermValidationFailure {
    /// Stable response discriminator.
    pub error: CourseTermFailureCode,
    /// Field the professor needs to correct.
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
    fn iana_names_require_exact_case_sensitive_database_membership() {
        for zone in chrono_tz::TZ_VARIANTS {
            assert_eq!(
                IanaTimeZone::parse(zone.name()).unwrap().as_str(),
                zone.name()
            );
        }
        for zone in [
            "America/Chicago",
            "Europe/Paris",
            "Pacific/Kiritimati",
            "Etc/GMT+1",
            "UTC",
        ] {
            assert_eq!(IanaTimeZone::parse(zone).unwrap().as_str(), zone);
        }
        for zone in [
            "america/chicago",
            "America/Imaginary",
            "-06:00",
            " Chicago ",
            "America/Chicago\n",
        ] {
            assert!(
                IanaTimeZone::parse(zone).is_err(),
                "unexpected valid zone: {zone}"
            );
        }
    }

    #[test]
    fn course_term_bounds_are_inclusive_and_ordered() {
        CourseTerm::from_parts("2026-08-24", "2026-08-24", "America/Chicago")
            .expect("one-day inclusive term");
        assert_eq!(
            CourseTerm::from_parts("2026-08-25", "2026-08-24", "America/Chicago"),
            Err(CourseTermError::EndBeforeStart)
        );
    }

    #[test]
    fn deserialization_cannot_construct_a_reversed_term() {
        let result = serde_json::from_str::<CourseTerm>(
            r#"{"startDate":"2026-08-25","endDate":"2026-08-24","timeZone":"America/Chicago"}"#,
        );
        assert!(result.is_err());
        assert!(serde_json::from_str::<CourseTerm>(
            r#"{"startDate":"2026-08-24","endDate":"2026-12-18","timeZone":"America/Chicago","offset":"-06:00"}"#,
        )
        .is_err());
    }
}

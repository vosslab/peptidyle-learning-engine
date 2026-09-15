use std::num::NonZeroU32;

use chrono::{DateTime, LocalResult, NaiveDateTime, TimeZone, Utc};
use serde::{Deserialize, Serialize};

use super::{
    AssessmentAuthoredContent, AssessmentInstructions, AssessmentStatus, BaseAssessmentPolicy,
    LateWorkRule, MAX_ASSESSMENT_ATTEMPT_LIMIT, MAX_ASSESSMENT_ATTEMPT_TIME_LIMIT_SECONDS,
};
use crate::{AccountTimeZone, AssessmentActivityRules, CourseTerm, Timestamp};

/// Server-derived Instructor Assessment Availability View at one authoritative instant.
///
/// This is deliberately a closed tagged union: a browser cannot invent a
/// timestamp for a state that has none, and it never compares its own clock.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    tag = "state",
    rename_all = "camelCase",
    rename_all_fields = "snake_case",
    deny_unknown_fields
)]
pub enum InstructorAssessmentAvailabilityView {
    Unreleased,
    Scheduled { available_at: LocalDateAndTime },
    Available,
    Closed { closed_at: Option<LocalDateAndTime> },
    Archived,
}

pub fn derive_instructor_assessment_availability(
    term: &CourseTerm,
    account_time_zone: &AccountTimeZone,
    assessment_status: AssessmentStatus,
    settings: &AssessmentAuthoredContent,
    now: Timestamp,
) -> Result<InstructorAssessmentAvailabilityView, AssessmentAuthoredContentLocalError> {
    use super::AssessmentStatus::{Archived, Closed, Released, Unreleased};
    match assessment_status {
        Unreleased => Ok(InstructorAssessmentAvailabilityView::Unreleased),
        Archived => Ok(InstructorAssessmentAvailabilityView::Archived),
        Closed => Ok(InstructorAssessmentAvailabilityView::Closed { closed_at: None }),
        Released if settings.base_policy.available_at.is_some_and(|at| now < at) => {
            Ok(InstructorAssessmentAvailabilityView::Scheduled {
                available_at: project_optional_local_timestamp_in_account_time_zone(
                    settings.base_policy.available_at,
                    term,
                    account_time_zone,
                    AssessmentAuthoredContentField::AvailableAt,
                )?
                .expect("released scheduled state has an available-at instant"),
            })
        }
        Released => {
            let due_boundary = (settings.base_policy.late_work_rule == LateWorkRule::Reject)
                .then_some(settings.base_policy.due_at)
                .flatten();
            let closed_at = match (settings.base_policy.closes_at, due_boundary) {
                (Some(closes), Some(due)) => Some(closes.min(due)),
                (Some(closes), None) => Some(closes),
                (None, Some(due)) => Some(due),
                (None, None) => None,
            };
            if closed_at.is_some_and(|at| now >= at) {
                return Ok(InstructorAssessmentAvailabilityView::Closed {
                    closed_at: project_optional_local_timestamp_in_account_time_zone(
                        closed_at,
                        term,
                        account_time_zone,
                        AssessmentAuthoredContentField::ClosesAt,
                    )?,
                });
            }
            Ok(InstructorAssessmentAvailabilityView::Available)
        }
    }
}

/// Exact browser `datetime-local` wire value without a time-zone claim.
///
/// This is deliberately a local wall-clock value, not a stored instant. The
/// authenticated server resolves it in its trusted Account zone, using
/// authorized Course calendar bounds, before persisting the resulting
/// [`AssessmentAuthoredContent`]. Its wire form is exactly
/// `YYYY-MM-DDTHH:MM:SS.sss`, which is accepted by HTML `datetime-local`
/// controls with `step="0.001"`. A browser may initialize its form at whole
/// minutes, but this canonical wire value never loses an existing server
/// timestamp's supported millisecond precision.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct LocalDateAndTime(String);

impl LocalDateAndTime {
    /// Parses one exact millisecond-precision local wall-clock string.
    pub fn parse(value: &str) -> Result<Self, LocalDateAndTimeError> {
        let bytes = value.as_bytes();
        let exact_shape = bytes.len() == 23
            && bytes[4] == b'-'
            && bytes[7] == b'-'
            && bytes[10] == b'T'
            && bytes[13] == b':'
            && bytes[16] == b':'
            && bytes[19] == b'.'
            && bytes.iter().enumerate().all(|(index, byte)| {
                matches!(index, 4 | 7 | 10 | 13 | 16 | 19) || byte.is_ascii_digit()
            });
        if !exact_shape || NaiveDateTime::parse_from_str(value, "%Y-%m-%dT%H:%M:%S%.3f").is_err() {
            return Err(LocalDateAndTimeError);
        }
        Ok(Self(value.to_string()))
    }

    /// Returns the exact canonical browser wire form.
    pub fn as_str(&self) -> &str {
        &self.0
    }

    fn naive(&self) -> NaiveDateTime {
        // `parse` is the only constructor, including deserialization.
        NaiveDateTime::parse_from_str(&self.0, "%Y-%m-%dT%H:%M:%S%.3f")
            .expect("validated local date time")
    }

    /// Resolves this zone-free wall-clock input only in the authenticated
    /// Account's exact IANA zone. The caller supplies calendar bounds from
    /// the authorized Course context; no browser-selected zone is accepted.
    pub fn resolve_in_account_time_zone(
        &self,
        course_term: &CourseTerm,
        account_time_zone: &AccountTimeZone,
        field: AssessmentAuthoredContentField,
    ) -> Result<Timestamp, AssessmentAuthoredContentLocalError> {
        resolve_local_timestamp_in_account_time_zone(self, course_term, account_time_zone, field)
    }

    /// Projects an instant through the authenticated Account's exact IANA zone.
    pub fn from_activity_timestamp_in_account_time_zone(
        value: Timestamp,
        course_term: &CourseTerm,
        account_time_zone: &AccountTimeZone,
        field: AssessmentAuthoredContentField,
    ) -> Result<Self, AssessmentAuthoredContentLocalError> {
        project_local_timestamp_in_account_time_zone(value, course_term, account_time_zone, field)
    }
}

impl TryFrom<String> for LocalDateAndTime {
    type Error = LocalDateAndTimeError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::parse(&value)
    }
}

impl From<LocalDateAndTime> for String {
    fn from(value: LocalDateAndTime) -> Self {
        value.0
    }
}

/// A local wall-clock string is not exact `YYYY-MM-DDTHH:MM:SS.sss` calendar time.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LocalDateAndTimeError;

impl std::fmt::Display for LocalDateAndTimeError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("local date time must be exact YYYY-MM-DDTHH:MM:SS.sss")
    }
}

impl std::error::Error for LocalDateAndTimeError {}

/// Browser-facing Instructor Assessment Authored Content Local.
///
/// This is an edit/display boundary only. It contains zone-free local strings;
/// the server supplies its trusted Account zone so a browser never consults
/// its own machine zone.
/// [`AssessmentAuthoredContent`] and its [`BaseAssessmentPolicy`] remain the
/// only stored and effective-policy authority.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct InstructorAssessmentAuthoredContentLocal {
    /// Validated student-facing plain-text instructions.
    pub instructions: AssessmentInstructions,
    /// First local wall-clock time at which students may open the assessment.
    #[serde(rename = "available_at")]
    pub available_at: Option<LocalDateAndTime>,
    /// Ordinary local wall-clock due time.
    #[serde(rename = "due_at")]
    pub due_at: Option<LocalDateAndTime>,
    /// Hard local wall-clock time after which new work is closed.
    #[serde(rename = "closes_at")]
    pub closes_at: Option<LocalDateAndTime>,
    /// Whole Assessment Attempt limit when one applies.
    #[serde(rename = "assessment_attempt_time_limit_seconds")]
    pub assessment_attempt_time_limit_seconds: Option<NonZeroU32>,
    /// Maximum number of Assessment Attempts when one applies.
    #[serde(rename = "attempt_limit")]
    pub attempt_limit: Option<NonZeroU32>,
    /// Treatment of work after the ordinary due instant.
    #[serde(rename = "late_work_rule")]
    pub late_work_rule: LateWorkRule,
}

impl InstructorAssessmentAuthoredContentLocal {
    /// Builds the browser value after validating limits and local ordering.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        instructions: AssessmentInstructions,
        available_at: Option<LocalDateAndTime>,
        due_at: Option<LocalDateAndTime>,
        closes_at: Option<LocalDateAndTime>,
        assessment_attempt_time_limit_seconds: Option<NonZeroU32>,
        attempt_limit: Option<NonZeroU32>,
        late_work_rule: LateWorkRule,
    ) -> Result<Self, AssessmentAuthoredContentLocalError> {
        if assessment_attempt_time_limit_seconds
            .is_some_and(|limit| limit.get() > MAX_ASSESSMENT_ATTEMPT_TIME_LIMIT_SECONDS)
        {
            return Err(AssessmentAuthoredContentLocalError::AssessmentAttemptTimeLimitOutOfRange);
        }
        if attempt_limit.is_some_and(|limit| limit.get() > MAX_ASSESSMENT_ATTEMPT_LIMIT) {
            return Err(AssessmentAuthoredContentLocalError::AttemptLimitOutOfRange);
        }
        validate_local_ordering(&available_at, &due_at, &closes_at)?;
        Ok(Self {
            instructions,
            available_at,
            due_at,
            closes_at,
            assessment_attempt_time_limit_seconds,
            attempt_limit,
            late_work_rule,
        })
    }

    /// Resolves this local instructor input in a trusted Account IANA zone.
    ///
    /// The server calls this before a store mutation. It refuses a calendar
    /// escape, DST gap, DST ambiguity, and invalid ordering.
    pub fn into_absolute(
        self,
        course_term: &CourseTerm,
        account_time_zone: &AccountTimeZone,
        activity_rules: AssessmentActivityRules,
    ) -> Result<AssessmentAuthoredContent, AssessmentAuthoredContentLocalError> {
        self.validate()?;
        let available_at = self
            .available_at
            .as_ref()
            .map(|value| {
                value.resolve_in_account_time_zone(
                    course_term,
                    account_time_zone,
                    AssessmentAuthoredContentField::AvailableAt,
                )
            })
            .transpose()?;
        let due_at = self
            .due_at
            .as_ref()
            .map(|value| {
                value.resolve_in_account_time_zone(
                    course_term,
                    account_time_zone,
                    AssessmentAuthoredContentField::DueAt,
                )
            })
            .transpose()?;
        let closes_at = self
            .closes_at
            .as_ref()
            .map(|value| {
                value.resolve_in_account_time_zone(
                    course_term,
                    account_time_zone,
                    AssessmentAuthoredContentField::ClosesAt,
                )
            })
            .transpose()?;
        validate_absolute_ordering(available_at, due_at, closes_at)?;
        Ok(AssessmentAuthoredContent {
            instructions: self.instructions,
            base_policy: BaseAssessmentPolicy {
                available_at,
                due_at,
                closes_at,
                assessment_attempt_time_limit_seconds: self.assessment_attempt_time_limit_seconds,
                attempt_limit: self.attempt_limit,
                late_work_rule: self.late_work_rule,
            },
            activity_rules,
        })
    }

    fn validate(&self) -> Result<(), AssessmentAuthoredContentLocalError> {
        if self
            .assessment_attempt_time_limit_seconds
            .is_some_and(|limit| limit.get() > MAX_ASSESSMENT_ATTEMPT_TIME_LIMIT_SECONDS)
        {
            return Err(AssessmentAuthoredContentLocalError::AssessmentAttemptTimeLimitOutOfRange);
        }
        if self
            .attempt_limit
            .is_some_and(|limit| limit.get() > MAX_ASSESSMENT_ATTEMPT_LIMIT)
        {
            return Err(AssessmentAuthoredContentLocalError::AttemptLimitOutOfRange);
        }
        validate_local_ordering(&self.available_at, &self.due_at, &self.closes_at)
    }

    /// Projects stored absolute settings into the trusted Account-zone wall-clock form.
    pub fn from_absolute(
        course_term: &CourseTerm,
        account_time_zone: &AccountTimeZone,
        settings: &AssessmentAuthoredContent,
    ) -> Result<Self, AssessmentAuthoredContentLocalError> {
        let available_at = settings
            .base_policy
            .available_at
            .map(|value| {
                LocalDateAndTime::from_activity_timestamp_in_account_time_zone(
                    value,
                    course_term,
                    account_time_zone,
                    AssessmentAuthoredContentField::AvailableAt,
                )
            })
            .transpose()?;
        let due_at = settings
            .base_policy
            .due_at
            .map(|value| {
                LocalDateAndTime::from_activity_timestamp_in_account_time_zone(
                    value,
                    course_term,
                    account_time_zone,
                    AssessmentAuthoredContentField::DueAt,
                )
            })
            .transpose()?;
        let closes_at = settings
            .base_policy
            .closes_at
            .map(|value| {
                LocalDateAndTime::from_activity_timestamp_in_account_time_zone(
                    value,
                    course_term,
                    account_time_zone,
                    AssessmentAuthoredContentField::ClosesAt,
                )
            })
            .transpose()?;
        Self::new(
            settings.instructions.clone(),
            available_at,
            due_at,
            closes_at,
            settings.base_policy.assessment_attempt_time_limit_seconds,
            settings.base_policy.attempt_limit,
            settings.base_policy.late_work_rule,
        )
    }
}

/// Refusal reason while translating an instructor local schedule at the server boundary.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum AssessmentAuthoredContentFailureCode {
    /// The submitted Assessment-authored content cannot be accepted.
    AssessmentAuthoredContentInvalid,
}

/// Browser-safe field that needs a correction in current Assessment content.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum AssessmentAuthoredContentField {
    AssessmentAuthoredContent,
    AvailableAt,
    DueAt,
    ClosesAt,
    Schedule,
    AssessmentAttemptTimeLimitSeconds,
    AttemptLimit,
    Instructions,
}

/// Browser-safe reason an assessment teaching-settings input was refused.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum AssessmentAuthoredContentFailureReason {
    InvalidInput,
    OutsideCourseTerm,
    NonexistentLocalTime,
    AmbiguousLocalTime,
    TimestampOutOfRange,
    ScheduleOutOfOrder,
    AssessmentAttemptTimeLimitOutOfRange,
    AttemptLimitOutOfRange,
    InvalidInstructions,
}

/// Answer-free bounded correction contract for local teaching-settings input.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AssessmentAuthoredContentValidationFailure {
    pub error: AssessmentAuthoredContentFailureCode,
    pub field: AssessmentAuthoredContentField,
    pub reason: AssessmentAuthoredContentFailureReason,
    pub message: String,
}

/// Refusal reason while translating an instructor local schedule at the server boundary.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AssessmentAuthoredContentLocalError {
    /// A local schedule timestamp lies outside the inclusive course calendar.
    OutsideCourseTerm(AssessmentAuthoredContentField),
    /// A local wall-clock time never occurred because DST skipped it.
    NonexistentLocalTime(AssessmentAuthoredContentField),
    /// A local wall-clock time occurred twice and lacks an offset discriminator.
    AmbiguousLocalTime(AssessmentAuthoredContentField),
    /// A timestamp cannot be represented by Chrono's supported range.
    TimestampOutOfRange(AssessmentAuthoredContentField),
    /// Available, due, and closes values are not chronological.
    ScheduleOutOfOrder,
    /// The time limit exceeds PostgreSQL's supported integer range.
    AssessmentAttemptTimeLimitOutOfRange,
    /// The attempt limit exceeds PostgreSQL's supported integer range.
    AttemptLimitOutOfRange,
}

impl AssessmentAuthoredContentLocalError {
    pub fn field(self) -> AssessmentAuthoredContentField {
        match self {
            Self::OutsideCourseTerm(field)
            | Self::NonexistentLocalTime(field)
            | Self::AmbiguousLocalTime(field)
            | Self::TimestampOutOfRange(field) => field,
            Self::ScheduleOutOfOrder => AssessmentAuthoredContentField::Schedule,
            Self::AssessmentAttemptTimeLimitOutOfRange => {
                AssessmentAuthoredContentField::AssessmentAttemptTimeLimitSeconds
            }
            Self::AttemptLimitOutOfRange => AssessmentAuthoredContentField::AttemptLimit,
        }
    }

    pub fn reason(self) -> AssessmentAuthoredContentFailureReason {
        match self {
            Self::OutsideCourseTerm(_) => AssessmentAuthoredContentFailureReason::OutsideCourseTerm,
            Self::NonexistentLocalTime(_) => {
                AssessmentAuthoredContentFailureReason::NonexistentLocalTime
            }
            Self::AmbiguousLocalTime(_) => {
                AssessmentAuthoredContentFailureReason::AmbiguousLocalTime
            }
            Self::TimestampOutOfRange(_) => {
                AssessmentAuthoredContentFailureReason::TimestampOutOfRange
            }
            Self::ScheduleOutOfOrder => AssessmentAuthoredContentFailureReason::ScheduleOutOfOrder,
            Self::AssessmentAttemptTimeLimitOutOfRange => {
                AssessmentAuthoredContentFailureReason::AssessmentAttemptTimeLimitOutOfRange
            }
            Self::AttemptLimitOutOfRange => {
                AssessmentAuthoredContentFailureReason::AttemptLimitOutOfRange
            }
        }
    }
}

impl std::fmt::Display for AssessmentAuthoredContentLocalError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(match self {
            Self::OutsideCourseTerm(_) => "teaching schedule must be inside the course calendar",
            Self::NonexistentLocalTime(_) => {
                "teaching schedule uses a nonexistent daylight-saving local time"
            }
            Self::AmbiguousLocalTime(_) => {
                "teaching schedule uses an ambiguous daylight-saving local time"
            }
            Self::TimestampOutOfRange(_) => {
                "teaching schedule timestamp is outside the supported range"
            }
            Self::ScheduleOutOfOrder => "available, due, and closes times must be chronological",
            Self::AssessmentAttemptTimeLimitOutOfRange => {
                "teaching time limit exceeds the supported range"
            }
            Self::AttemptLimitOutOfRange => "teaching attempt limit exceeds the supported range",
        })
    }
}

impl std::error::Error for AssessmentAuthoredContentLocalError {}

fn validate_local_ordering(
    available_at: &Option<LocalDateAndTime>,
    due_at: &Option<LocalDateAndTime>,
    closes_at: &Option<LocalDateAndTime>,
) -> Result<(), AssessmentAuthoredContentLocalError> {
    if available_at
        .as_ref()
        .zip(due_at.as_ref())
        .is_some_and(|(available, due)| available > due)
        || due_at
            .as_ref()
            .zip(closes_at.as_ref())
            .is_some_and(|(due, closes)| due > closes)
        || available_at
            .as_ref()
            .zip(closes_at.as_ref())
            .is_some_and(|(available, closes)| available > closes)
    {
        return Err(AssessmentAuthoredContentLocalError::ScheduleOutOfOrder);
    }
    Ok(())
}

fn validate_absolute_ordering(
    available_at: Option<Timestamp>,
    due_at: Option<Timestamp>,
    closes_at: Option<Timestamp>,
) -> Result<(), AssessmentAuthoredContentLocalError> {
    if available_at
        .zip(due_at)
        .is_some_and(|(available, due)| available > due)
        || due_at
            .zip(closes_at)
            .is_some_and(|(due, closes)| due > closes)
        || available_at
            .zip(closes_at)
            .is_some_and(|(available, closes)| available > closes)
    {
        return Err(AssessmentAuthoredContentLocalError::ScheduleOutOfOrder);
    }
    Ok(())
}

fn parsed_account_time_zone(account_time_zone: &AccountTimeZone) -> chrono_tz::Tz {
    account_time_zone
        .as_str()
        .parse()
        .expect("AccountTimeZone contains an exact known IANA zone")
}

/// Resolves an authoring wall-clock value in the authenticated Account zone.
/// ASVS 2.2.1 and 8.2.2: the Account zone is validated and authorized before
/// this boundary; callers never accept an Account ID or zone from the browser.
pub fn resolve_local_timestamp_in_account_time_zone(
    value: &LocalDateAndTime,
    course_term: &CourseTerm,
    account_time_zone: &AccountTimeZone,
    field: AssessmentAuthoredContentField,
) -> Result<Timestamp, AssessmentAuthoredContentLocalError> {
    resolve_local_timestamp(
        value,
        course_term,
        parsed_account_time_zone(account_time_zone),
        field,
    )
}

fn resolve_local_timestamp(
    value: &LocalDateAndTime,
    course_term: &CourseTerm,
    time_zone: chrono_tz::Tz,
    field: AssessmentAuthoredContentField,
) -> Result<Timestamp, AssessmentAuthoredContentLocalError> {
    let naive = value.naive();
    let date = naive.date().format("%Y-%m-%d").to_string();
    if date.as_str() < course_term.start_date().as_str()
        || date.as_str() > course_term.end_date().as_str()
    {
        return Err(AssessmentAuthoredContentLocalError::OutsideCourseTerm(
            field,
        ));
    }
    match time_zone.from_local_datetime(&naive) {
        LocalResult::Single(value) => Ok(Timestamp::from_unix_millis(value.timestamp_millis())),
        LocalResult::None => Err(AssessmentAuthoredContentLocalError::NonexistentLocalTime(
            field,
        )),
        LocalResult::Ambiguous(_, _) => Err(
            AssessmentAuthoredContentLocalError::AmbiguousLocalTime(field),
        ),
    }
}

/// Projects an instant into the authenticated Account zone without changing it.
pub fn project_local_timestamp_in_account_time_zone(
    value: Timestamp,
    course_term: &CourseTerm,
    account_time_zone: &AccountTimeZone,
    field: AssessmentAuthoredContentField,
) -> Result<LocalDateAndTime, AssessmentAuthoredContentLocalError> {
    project_local_timestamp(
        value,
        course_term,
        parsed_account_time_zone(account_time_zone),
        field,
    )
}

fn project_optional_local_timestamp_in_account_time_zone(
    value: Option<Timestamp>,
    course_term: &CourseTerm,
    account_time_zone: &AccountTimeZone,
    field: AssessmentAuthoredContentField,
) -> Result<Option<LocalDateAndTime>, AssessmentAuthoredContentLocalError> {
    value
        .map(|value| {
            project_local_timestamp_in_account_time_zone(
                value,
                course_term,
                account_time_zone,
                field,
            )
        })
        .transpose()
}

fn project_local_timestamp(
    value: Timestamp,
    _course_term: &CourseTerm,
    time_zone: chrono_tz::Tz,
    field: AssessmentAuthoredContentField,
) -> Result<LocalDateAndTime, AssessmentAuthoredContentLocalError> {
    let utc = DateTime::<Utc>::from_timestamp_millis(value.as_unix_millis()).ok_or(
        AssessmentAuthoredContentLocalError::TimestampOutOfRange(field),
    )?;
    let local = time_zone.from_utc_datetime(&utc.naive_utc());
    // A stored instant is already unambiguous. Its display may be an ambiguous
    // fall-back local clock value, but formatting must not make it unusable.
    Ok(
        LocalDateAndTime::parse(&local.format("%Y-%m-%dT%H:%M:%S%.3f").to_string())
            .expect("formatted Account-local timestamp is valid"),
    )
}

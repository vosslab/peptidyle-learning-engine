use std::num::NonZeroU32;

use chrono::{DateTime, LocalResult, NaiveDateTime, TimeZone, Utc};
use serde::{Deserialize, Serialize};

use super::{
    AssignmentAuthoredContent, AssignmentDeadlineRule, AssignmentInstructions, AssignmentStatus,
    BaseAssignmentPolicy, LateWorkRule, MAX_ASSIGNMENT_ATTEMPT_LIMIT,
    MAX_ASSIGNMENT_ATTEMPT_TIME_LIMIT_SECONDS,
};
use crate::{AssignmentActivityRules, CourseTerm, CourseTimeZone, Timestamp};

/// Server-derived Instructor Assignment Availability View at one authoritative instant.
///
/// This is deliberately a closed tagged union: a browser cannot invent a
/// timestamp for a state that has none, and it never compares its own clock.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    tag = "state",
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum InstructorAssignmentAvailabilityView {
    Unreleased,
    Scheduled {
        available_at: CourseLocalDateAndTime,
    },
    Available,
    Closed {
        closed_at: Option<CourseLocalDateAndTime>,
    },
    Archived,
}

pub fn derive_instructor_assignment_availability(
    term: &CourseTerm,
    assignment_status: AssignmentStatus,
    settings: &AssignmentAuthoredContent,
    now: Timestamp,
) -> Result<InstructorAssignmentAvailabilityView, AssignmentAuthoredContentLocalError> {
    use super::AssignmentStatus::{Archived, Closed, Released, Unreleased};
    match assignment_status {
        Unreleased => Ok(InstructorAssignmentAvailabilityView::Unreleased),
        Archived => Ok(InstructorAssignmentAvailabilityView::Archived),
        Closed => Ok(InstructorAssignmentAvailabilityView::Closed { closed_at: None }),
        Released if settings.base_policy.available_at.is_some_and(|at| now < at) => {
            Ok(InstructorAssignmentAvailabilityView::Scheduled {
                available_at: project_optional_course_local_timestamp(
                    settings.base_policy.available_at,
                    term,
                    AssignmentAuthoredContentField::AvailableAt,
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
                return Ok(InstructorAssignmentAvailabilityView::Closed {
                    closed_at: project_optional_course_local_timestamp(
                        closed_at,
                        term,
                        AssignmentAuthoredContentField::ClosesAt,
                    )?,
                });
            }
            Ok(InstructorAssignmentAvailabilityView::Available)
        }
    }
}

/// Exact browser `datetime-local` wire value in the course's authoritative zone.
///
/// This is deliberately a local wall-clock value, not a stored instant. The
/// server resolves it with [`CourseTerm`] before persisting the resulting
/// [`AssignmentAuthoredContent`]. Its wire form is exactly
/// `YYYY-MM-DDTHH:MM:SS.sss`, which is accepted by HTML `datetime-local`
/// controls with `step="0.001"`. A browser may initialize its form at whole
/// minutes, but this canonical wire value never loses an existing server
/// timestamp's supported millisecond precision.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct CourseLocalDateAndTime(String);

impl CourseLocalDateAndTime {
    /// Parses one exact millisecond-precision local wall-clock string.
    pub fn parse(value: &str) -> Result<Self, CourseLocalDateAndTimeError> {
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
            return Err(CourseLocalDateAndTimeError);
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
            .expect("validated course-local date time")
    }

    /// Resolves this wall-clock value in the course's authoritative IANA zone.
    ///
    /// The supplied field identifies the exact correction target for DST,
    /// term, and range refusals at the server boundary.
    pub fn resolve_for_course(
        &self,
        course_term: &CourseTerm,
        field: AssignmentAuthoredContentField,
    ) -> Result<Timestamp, AssignmentAuthoredContentLocalError> {
        resolve_course_local_timestamp(self, course_term, field)
    }

    /// Projects one server-resolved instant into this course's local wire form.
    ///
    /// The supplied field identifies the exact correction target if an instant
    /// cannot round-trip through the course calendar and zone.
    pub fn from_activity_timestamp(
        value: Timestamp,
        course_term: &CourseTerm,
        field: AssignmentAuthoredContentField,
    ) -> Result<Self, AssignmentAuthoredContentLocalError> {
        project_course_local_timestamp(value, course_term, field)
    }
}

impl TryFrom<String> for CourseLocalDateAndTime {
    type Error = CourseLocalDateAndTimeError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::parse(&value)
    }
}

impl From<CourseLocalDateAndTime> for String {
    fn from(value: CourseLocalDateAndTime) -> Self {
        value.0
    }
}

/// A local wall-clock string is not exact `YYYY-MM-DDTHH:MM:SS.sss` calendar time.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CourseLocalDateAndTimeError;

impl std::fmt::Display for CourseLocalDateAndTimeError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("course-local date time must be exact YYYY-MM-DDTHH:MM:SS.sss")
    }
}

impl std::error::Error for CourseLocalDateAndTimeError {}

/// Browser-facing Instructor projection of one Assignment-authored content.
///
/// This is an edit/display boundary only. It contains local strings plus the
/// course-owned IANA zone so a browser never consults its own machine zone.
/// [`AssignmentAuthoredContent`] and its [`BaseAssignmentPolicy`] remain the
/// only stored and effective-policy authority.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct InstructorAssignmentAuthoredContentLocal {
    /// Authoritative course IANA zone shown beside local form controls.
    pub time_zone: CourseTimeZone,
    /// Validated student-facing plain-text instructions.
    pub instructions: AssignmentInstructions,
    /// First local course time at which students may open the assignment.
    pub available_at: Option<CourseLocalDateAndTime>,
    /// Ordinary local course due time.
    pub due_at: Option<CourseLocalDateAndTime>,
    /// Hard local course time after which new work is closed.
    pub closes_at: Option<CourseLocalDateAndTime>,
    /// Whole Assignment Attempt limit when one applies.
    pub assignment_attempt_time_limit_seconds: Option<NonZeroU32>,
    /// Maximum number of Assignment Attempts when one applies.
    pub attempt_limit: Option<NonZeroU32>,
    /// Treatment of work after the ordinary due instant.
    pub late_work_rule: LateWorkRule,
    /// Server behavior at an effective assignment deadline.
    pub assignment_deadline_rule: AssignmentDeadlineRule,
}

impl InstructorAssignmentAuthoredContentLocal {
    /// Builds a browser projection after validating limits and local ordering.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        time_zone: CourseTimeZone,
        instructions: AssignmentInstructions,
        available_at: Option<CourseLocalDateAndTime>,
        due_at: Option<CourseLocalDateAndTime>,
        closes_at: Option<CourseLocalDateAndTime>,
        assignment_attempt_time_limit_seconds: Option<NonZeroU32>,
        attempt_limit: Option<NonZeroU32>,
        late_work_rule: LateWorkRule,
        assignment_deadline_rule: AssignmentDeadlineRule,
    ) -> Result<Self, AssignmentAuthoredContentLocalError> {
        if assignment_attempt_time_limit_seconds
            .is_some_and(|limit| limit.get() > MAX_ASSIGNMENT_ATTEMPT_TIME_LIMIT_SECONDS)
        {
            return Err(AssignmentAuthoredContentLocalError::AssignmentAttemptTimeLimitOutOfRange);
        }
        if attempt_limit.is_some_and(|limit| limit.get() > MAX_ASSIGNMENT_ATTEMPT_LIMIT) {
            return Err(AssignmentAuthoredContentLocalError::AttemptLimitOutOfRange);
        }
        validate_local_ordering(&available_at, &due_at, &closes_at)?;
        Ok(Self {
            time_zone,
            instructions,
            available_at,
            due_at,
            closes_at,
            assignment_attempt_time_limit_seconds,
            attempt_limit,
            late_work_rule,
            assignment_deadline_rule,
        })
    }

    /// Resolves this local instructor input against the course-owned IANA zone.
    ///
    /// The server calls this before a store mutation. It refuses a zone mismatch,
    /// course-calendar escape, DST gap, DST ambiguity, and invalid ordering.
    pub fn into_absolute(
        self,
        course_term: &CourseTerm,
        activity_rules: AssignmentActivityRules,
    ) -> Result<AssignmentAuthoredContent, AssignmentAuthoredContentLocalError> {
        self.validate()?;
        if self.time_zone != *course_term.time_zone() {
            return Err(AssignmentAuthoredContentLocalError::CourseTimeZoneMismatch);
        }
        let available_at = resolve_optional_course_local_timestamp(
            self.available_at.as_ref(),
            course_term,
            AssignmentAuthoredContentField::AvailableAt,
        )?;
        let due_at = resolve_optional_course_local_timestamp(
            self.due_at.as_ref(),
            course_term,
            AssignmentAuthoredContentField::DueAt,
        )?;
        let closes_at = resolve_optional_course_local_timestamp(
            self.closes_at.as_ref(),
            course_term,
            AssignmentAuthoredContentField::ClosesAt,
        )?;
        validate_absolute_ordering(available_at, due_at, closes_at)?;
        Ok(AssignmentAuthoredContent {
            instructions: self.instructions,
            base_policy: BaseAssignmentPolicy {
                available_at,
                due_at,
                closes_at,
                assignment_attempt_time_limit_seconds: self.assignment_attempt_time_limit_seconds,
                attempt_limit: self.attempt_limit,
                late_work_rule: self.late_work_rule,
                assignment_deadline_rule: self.assignment_deadline_rule,
            },
            activity_rules,
        })
    }

    fn validate(&self) -> Result<(), AssignmentAuthoredContentLocalError> {
        if self
            .assignment_attempt_time_limit_seconds
            .is_some_and(|limit| limit.get() > MAX_ASSIGNMENT_ATTEMPT_TIME_LIMIT_SECONDS)
        {
            return Err(AssignmentAuthoredContentLocalError::AssignmentAttemptTimeLimitOutOfRange);
        }
        if self
            .attempt_limit
            .is_some_and(|limit| limit.get() > MAX_ASSIGNMENT_ATTEMPT_LIMIT)
        {
            return Err(AssignmentAuthoredContentLocalError::AttemptLimitOutOfRange);
        }
        validate_local_ordering(&self.available_at, &self.due_at, &self.closes_at)
    }

    /// Projects stored absolute settings into exact local course wall-clock values.
    pub fn from_absolute(
        course_term: &CourseTerm,
        settings: &AssignmentAuthoredContent,
    ) -> Result<Self, AssignmentAuthoredContentLocalError> {
        let available_at = project_optional_course_local_timestamp(
            settings.base_policy.available_at,
            course_term,
            AssignmentAuthoredContentField::AvailableAt,
        )?;
        let due_at = project_optional_course_local_timestamp(
            settings.base_policy.due_at,
            course_term,
            AssignmentAuthoredContentField::DueAt,
        )?;
        let closes_at = project_optional_course_local_timestamp(
            settings.base_policy.closes_at,
            course_term,
            AssignmentAuthoredContentField::ClosesAt,
        )?;
        Self::new(
            course_term.time_zone().clone(),
            settings.instructions.clone(),
            available_at,
            due_at,
            closes_at,
            settings.base_policy.assignment_attempt_time_limit_seconds,
            settings.base_policy.attempt_limit,
            settings.base_policy.late_work_rule,
            settings.base_policy.assignment_deadline_rule,
        )
    }
}

/// Refusal reason while translating an instructor local schedule at the server boundary.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum AssignmentAuthoredContentFailureCode {
    /// The submitted Assignment-authored content cannot be accepted.
    AssignmentAuthoredContentInvalid,
}

/// Browser-safe field that needs a correction in an Assignment Revision.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum AssignmentAuthoredContentField {
    AssignmentAuthoredContent,
    TimeZone,
    AvailableAt,
    DueAt,
    ClosesAt,
    Schedule,
    AssignmentAttemptTimeLimitSeconds,
    AttemptLimit,
    Instructions,
}

/// Browser-safe reason an assignment teaching-settings input was refused.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum AssignmentAuthoredContentFailureReason {
    InvalidInput,
    CourseTimeZoneMismatch,
    OutsideCourseTerm,
    NonexistentLocalTime,
    AmbiguousLocalTime,
    TimestampOutOfRange,
    ScheduleOutOfOrder,
    AssignmentAttemptTimeLimitOutOfRange,
    AttemptLimitOutOfRange,
    InvalidInstructions,
}

/// Answer-free bounded correction contract for local teaching-settings input.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AssignmentAuthoredContentValidationFailure {
    pub error: AssignmentAuthoredContentFailureCode,
    pub field: AssignmentAuthoredContentField,
    pub reason: AssignmentAuthoredContentFailureReason,
    pub message: String,
}

/// Refusal reason while translating an instructor local schedule at the server boundary.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AssignmentAuthoredContentLocalError {
    /// The browser repeated a zone other than the course's authoritative IANA zone.
    CourseTimeZoneMismatch,
    /// A local schedule timestamp lies outside the inclusive course calendar.
    OutsideCourseTerm(AssignmentAuthoredContentField),
    /// A local wall-clock time never occurred because DST skipped it.
    NonexistentLocalTime(AssignmentAuthoredContentField),
    /// A local wall-clock time occurred twice and lacks an offset discriminator.
    AmbiguousLocalTime(AssignmentAuthoredContentField),
    /// A timestamp cannot be represented by Chrono's supported range.
    TimestampOutOfRange(AssignmentAuthoredContentField),
    /// Available, due, and closes values are not chronological.
    ScheduleOutOfOrder,
    /// The time limit exceeds PostgreSQL's supported integer range.
    AssignmentAttemptTimeLimitOutOfRange,
    /// The attempt limit exceeds PostgreSQL's supported integer range.
    AttemptLimitOutOfRange,
}

impl AssignmentAuthoredContentLocalError {
    pub fn field(self) -> AssignmentAuthoredContentField {
        match self {
            Self::CourseTimeZoneMismatch => AssignmentAuthoredContentField::TimeZone,
            Self::OutsideCourseTerm(field)
            | Self::NonexistentLocalTime(field)
            | Self::AmbiguousLocalTime(field)
            | Self::TimestampOutOfRange(field) => field,
            Self::ScheduleOutOfOrder => AssignmentAuthoredContentField::Schedule,
            Self::AssignmentAttemptTimeLimitOutOfRange => {
                AssignmentAuthoredContentField::AssignmentAttemptTimeLimitSeconds
            }
            Self::AttemptLimitOutOfRange => AssignmentAuthoredContentField::AttemptLimit,
        }
    }

    pub fn reason(self) -> AssignmentAuthoredContentFailureReason {
        match self {
            Self::CourseTimeZoneMismatch => {
                AssignmentAuthoredContentFailureReason::CourseTimeZoneMismatch
            }
            Self::OutsideCourseTerm(_) => AssignmentAuthoredContentFailureReason::OutsideCourseTerm,
            Self::NonexistentLocalTime(_) => {
                AssignmentAuthoredContentFailureReason::NonexistentLocalTime
            }
            Self::AmbiguousLocalTime(_) => {
                AssignmentAuthoredContentFailureReason::AmbiguousLocalTime
            }
            Self::TimestampOutOfRange(_) => {
                AssignmentAuthoredContentFailureReason::TimestampOutOfRange
            }
            Self::ScheduleOutOfOrder => AssignmentAuthoredContentFailureReason::ScheduleOutOfOrder,
            Self::AssignmentAttemptTimeLimitOutOfRange => {
                AssignmentAuthoredContentFailureReason::AssignmentAttemptTimeLimitOutOfRange
            }
            Self::AttemptLimitOutOfRange => {
                AssignmentAuthoredContentFailureReason::AttemptLimitOutOfRange
            }
        }
    }
}

impl std::fmt::Display for AssignmentAuthoredContentLocalError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(match self {
            Self::CourseTimeZoneMismatch => {
                "Assignment-authored content time zone must match the course time zone"
            }
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
            Self::AssignmentAttemptTimeLimitOutOfRange => {
                "teaching time limit exceeds the supported range"
            }
            Self::AttemptLimitOutOfRange => "teaching attempt limit exceeds the supported range",
        })
    }
}

impl std::error::Error for AssignmentAuthoredContentLocalError {}

fn validate_local_ordering(
    available_at: &Option<CourseLocalDateAndTime>,
    due_at: &Option<CourseLocalDateAndTime>,
    closes_at: &Option<CourseLocalDateAndTime>,
) -> Result<(), AssignmentAuthoredContentLocalError> {
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
        return Err(AssignmentAuthoredContentLocalError::ScheduleOutOfOrder);
    }
    Ok(())
}

fn validate_absolute_ordering(
    available_at: Option<Timestamp>,
    due_at: Option<Timestamp>,
    closes_at: Option<Timestamp>,
) -> Result<(), AssignmentAuthoredContentLocalError> {
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
        return Err(AssignmentAuthoredContentLocalError::ScheduleOutOfOrder);
    }
    Ok(())
}

fn course_time_zone(course_term: &CourseTerm) -> chrono_tz::Tz {
    course_term
        .time_zone()
        .as_str()
        .parse()
        .expect("CourseTerm contains an exact known IANA zone")
}

fn resolve_optional_course_local_timestamp(
    value: Option<&CourseLocalDateAndTime>,
    course_term: &CourseTerm,
    field: AssignmentAuthoredContentField,
) -> Result<Option<Timestamp>, AssignmentAuthoredContentLocalError> {
    value
        .map(|value| resolve_course_local_timestamp(value, course_term, field))
        .transpose()
}

/// Resolves one exact course-local wall-clock value at the server boundary.
///
/// Callers supply the receiving field so every DST, term, and range refusal
/// points at the browser control the instructor must correct. This function
/// never consults a machine-local time zone.
pub fn resolve_course_local_timestamp(
    value: &CourseLocalDateAndTime,
    course_term: &CourseTerm,
    field: AssignmentAuthoredContentField,
) -> Result<Timestamp, AssignmentAuthoredContentLocalError> {
    let naive = value.naive();
    let date = naive.date().format("%Y-%m-%d").to_string();
    if date.as_str() < course_term.start_date().as_str()
        || date.as_str() > course_term.end_date().as_str()
    {
        return Err(AssignmentAuthoredContentLocalError::OutsideCourseTerm(
            field,
        ));
    }
    match course_time_zone(course_term).from_local_datetime(&naive) {
        LocalResult::Single(value) => Ok(Timestamp::from_unix_millis(value.timestamp_millis())),
        LocalResult::None => Err(AssignmentAuthoredContentLocalError::NonexistentLocalTime(
            field,
        )),
        LocalResult::Ambiguous(_, _) => Err(
            AssignmentAuthoredContentLocalError::AmbiguousLocalTime(field),
        ),
    }
}

fn project_optional_course_local_timestamp(
    value: Option<Timestamp>,
    course_term: &CourseTerm,
    field: AssignmentAuthoredContentField,
) -> Result<Option<CourseLocalDateAndTime>, AssignmentAuthoredContentLocalError> {
    value
        .map(|value| project_course_local_timestamp(value, course_term, field))
        .transpose()
}

/// Projects a stored absolute timestamp into an exact course-local wall-clock value.
///
/// The round-trip check refuses an instant that cannot be represented without
/// choosing between two local times. Callers supply the receiving field so a
/// correction remains field-specific.
pub fn project_course_local_timestamp(
    value: Timestamp,
    course_term: &CourseTerm,
    field: AssignmentAuthoredContentField,
) -> Result<CourseLocalDateAndTime, AssignmentAuthoredContentLocalError> {
    let utc = DateTime::<Utc>::from_timestamp_millis(value.as_unix_millis()).ok_or(
        AssignmentAuthoredContentLocalError::TimestampOutOfRange(field),
    )?;
    let local = course_time_zone(course_term).from_utc_datetime(&utc.naive_utc());
    let wall_clock =
        CourseLocalDateAndTime::parse(&local.format("%Y-%m-%dT%H:%M:%S%.3f").to_string())
            .expect("formatted course-local timestamp is valid");
    match course_time_zone(course_term).from_local_datetime(&wall_clock.naive()) {
        LocalResult::Single(round_trip)
            if round_trip.timestamp_millis() == value.as_unix_millis() => {}
        LocalResult::Single(_) | LocalResult::Ambiguous(_, _) => {
            return Err(AssignmentAuthoredContentLocalError::AmbiguousLocalTime(
                field,
            ));
        }
        LocalResult::None => {
            return Err(AssignmentAuthoredContentLocalError::NonexistentLocalTime(
                field,
            ));
        }
    }
    let date = local.format("%Y-%m-%d").to_string();
    if date.as_str() < course_term.start_date().as_str()
        || date.as_str() > course_term.end_date().as_str()
    {
        return Err(AssignmentAuthoredContentLocalError::OutsideCourseTerm(
            field,
        ));
    }
    Ok(wall_clock)
}

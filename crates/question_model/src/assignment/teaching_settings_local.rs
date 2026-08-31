use std::num::NonZeroU32;

use chrono::{DateTime, LocalResult, NaiveDateTime, TimeZone, Utc};
use serde::{Deserialize, Serialize};

use super::{
    AssignmentDeadlineRule, AssignmentInstructions, AssignmentLifecycle,
    AssignmentRevisionDefinition, BaseAssignmentPolicy, LateWorkRule, MAX_ASSIGNMENT_ATTEMPT_LIMIT,
    MAX_ASSIGNMENT_ATTEMPT_TIME_LIMIT_SECONDS,
};
use crate::{ActivityTimestamp, AssignmentActivityRules, CourseTerm, CourseTimeZone};

/// Server-derived current teaching state at one authoritative instant.
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
pub enum InstructorAssignmentCurrentState {
    Draft,
    Scheduled {
        available_at: CourseLocalDateAndTime,
    },
    Open,
    Closed {
        closed_at: Option<CourseLocalDateAndTime>,
    },
    Archived,
}

pub fn derive_instructor_assignment_current_state(
    term: &CourseTerm,
    settings: &AssignmentRevisionDefinition,
    now: ActivityTimestamp,
) -> Result<InstructorAssignmentCurrentState, AssignmentRevisionDefinitionLocalError> {
    use super::AssignmentLifecycle::{Archived, Closed, Draft, Published};
    match settings.lifecycle {
        Draft => Ok(InstructorAssignmentCurrentState::Draft),
        Archived => Ok(InstructorAssignmentCurrentState::Archived),
        Closed => Ok(InstructorAssignmentCurrentState::Closed { closed_at: None }),
        Published if settings.base_policy.available_at.is_some_and(|at| now < at) => {
            Ok(InstructorAssignmentCurrentState::Scheduled {
                available_at: project_optional_course_local_timestamp(
                    settings.base_policy.available_at,
                    term,
                    AssignmentRevisionDefinitionField::AvailableAt,
                )?
                .expect("published scheduled state has an available-at instant"),
            })
        }
        Published => {
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
                return Ok(InstructorAssignmentCurrentState::Closed {
                    closed_at: project_optional_course_local_timestamp(
                        closed_at,
                        term,
                        AssignmentRevisionDefinitionField::ClosesAt,
                    )?,
                });
            }
            Ok(InstructorAssignmentCurrentState::Open)
        }
    }
}

/// Exact browser `datetime-local` wire value in the course's authoritative zone.
///
/// This is deliberately a local wall-clock value, not a stored instant. The
/// server resolves it with [`CourseTerm`] before persisting the resulting
/// [`AssignmentRevisionDefinition`]. Its wire form is exactly
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
        field: AssignmentRevisionDefinitionField,
    ) -> Result<ActivityTimestamp, AssignmentRevisionDefinitionLocalError> {
        resolve_course_local_timestamp(self, course_term, field)
    }

    /// Projects one server-resolved instant into this course's local wire form.
    ///
    /// The supplied field identifies the exact correction target if an instant
    /// cannot round-trip through the course calendar and zone.
    pub fn from_activity_timestamp(
        value: ActivityTimestamp,
        course_term: &CourseTerm,
        field: AssignmentRevisionDefinitionField,
    ) -> Result<Self, AssignmentRevisionDefinitionLocalError> {
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

/// Browser-facing instructor projection of one stored Assignment Revision Definition.
///
/// This is an edit/display boundary only. It contains local strings plus the
/// course-owned IANA zone so a browser never consults its own machine zone.
/// [`AssignmentRevisionDefinition`] and its [`BaseAssignmentPolicy`] remain the
/// only stored and effective-policy authority.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct InstructorAssignmentRevisionDefinitionLocal {
    /// Authoritative course IANA zone shown beside local form controls.
    pub time_zone: CourseTimeZone,
    /// Instructor-controlled assignment lifecycle intent.
    pub lifecycle: AssignmentLifecycle,
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
    /// Maximum number of runs when one applies.
    pub attempt_limit: Option<NonZeroU32>,
    /// Treatment of work after the ordinary due instant.
    pub late_work_rule: LateWorkRule,
    /// Server behavior at an effective assignment deadline.
    pub assignment_deadline_rule: AssignmentDeadlineRule,
}

impl InstructorAssignmentRevisionDefinitionLocal {
    /// Builds a browser projection after validating limits and local ordering.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        time_zone: CourseTimeZone,
        lifecycle: AssignmentLifecycle,
        instructions: AssignmentInstructions,
        available_at: Option<CourseLocalDateAndTime>,
        due_at: Option<CourseLocalDateAndTime>,
        closes_at: Option<CourseLocalDateAndTime>,
        assignment_attempt_time_limit_seconds: Option<NonZeroU32>,
        attempt_limit: Option<NonZeroU32>,
        late_work_rule: LateWorkRule,
        assignment_deadline_rule: AssignmentDeadlineRule,
    ) -> Result<Self, AssignmentRevisionDefinitionLocalError> {
        if assignment_attempt_time_limit_seconds
            .is_some_and(|limit| limit.get() > MAX_ASSIGNMENT_ATTEMPT_TIME_LIMIT_SECONDS)
        {
            return Err(
                AssignmentRevisionDefinitionLocalError::AssignmentAttemptTimeLimitOutOfRange,
            );
        }
        if attempt_limit.is_some_and(|limit| limit.get() > MAX_ASSIGNMENT_ATTEMPT_LIMIT) {
            return Err(AssignmentRevisionDefinitionLocalError::AttemptLimitOutOfRange);
        }
        validate_local_ordering(&available_at, &due_at, &closes_at)?;
        Ok(Self {
            time_zone,
            lifecycle,
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
    ) -> Result<AssignmentRevisionDefinition, AssignmentRevisionDefinitionLocalError> {
        self.validate()?;
        if self.time_zone != *course_term.time_zone() {
            return Err(AssignmentRevisionDefinitionLocalError::CourseTimeZoneMismatch);
        }
        let available_at = resolve_optional_course_local_timestamp(
            self.available_at.as_ref(),
            course_term,
            AssignmentRevisionDefinitionField::AvailableAt,
        )?;
        let due_at = resolve_optional_course_local_timestamp(
            self.due_at.as_ref(),
            course_term,
            AssignmentRevisionDefinitionField::DueAt,
        )?;
        let closes_at = resolve_optional_course_local_timestamp(
            self.closes_at.as_ref(),
            course_term,
            AssignmentRevisionDefinitionField::ClosesAt,
        )?;
        validate_absolute_ordering(available_at, due_at, closes_at)?;
        Ok(AssignmentRevisionDefinition {
            lifecycle: self.lifecycle,
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

    fn validate(&self) -> Result<(), AssignmentRevisionDefinitionLocalError> {
        if self
            .assignment_attempt_time_limit_seconds
            .is_some_and(|limit| limit.get() > MAX_ASSIGNMENT_ATTEMPT_TIME_LIMIT_SECONDS)
        {
            return Err(
                AssignmentRevisionDefinitionLocalError::AssignmentAttemptTimeLimitOutOfRange,
            );
        }
        if self
            .attempt_limit
            .is_some_and(|limit| limit.get() > MAX_ASSIGNMENT_ATTEMPT_LIMIT)
        {
            return Err(AssignmentRevisionDefinitionLocalError::AttemptLimitOutOfRange);
        }
        validate_local_ordering(&self.available_at, &self.due_at, &self.closes_at)
    }

    /// Projects stored absolute settings into exact local course wall-clock values.
    pub fn from_absolute(
        course_term: &CourseTerm,
        settings: &AssignmentRevisionDefinition,
    ) -> Result<Self, AssignmentRevisionDefinitionLocalError> {
        let available_at = project_optional_course_local_timestamp(
            settings.base_policy.available_at,
            course_term,
            AssignmentRevisionDefinitionField::AvailableAt,
        )?;
        let due_at = project_optional_course_local_timestamp(
            settings.base_policy.due_at,
            course_term,
            AssignmentRevisionDefinitionField::DueAt,
        )?;
        let closes_at = project_optional_course_local_timestamp(
            settings.base_policy.closes_at,
            course_term,
            AssignmentRevisionDefinitionField::ClosesAt,
        )?;
        Self::new(
            course_term.time_zone().clone(),
            settings.lifecycle,
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
pub enum AssignmentRevisionDefinitionFailureCode {
    /// The submitted Assignment Revision Definition cannot be accepted.
    AssignmentRevisionDefinitionInvalid,
}

/// Browser-safe field that needs a correction in an Assignment Revision Definition.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum AssignmentRevisionDefinitionField {
    AssignmentRevisionDefinition,
    TimeZone,
    AvailableAt,
    DueAt,
    ClosesAt,
    Schedule,
    AssignmentAttemptTimeLimitSeconds,
    AttemptLimit,
    Lifecycle,
    Instructions,
}

/// Browser-safe reason an assignment teaching-settings input was refused.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum AssignmentRevisionDefinitionFailureReason {
    InvalidInput,
    CourseTimeZoneMismatch,
    OutsideCourseTerm,
    NonexistentLocalTime,
    AmbiguousLocalTime,
    TimestampOutOfRange,
    ScheduleOutOfOrder,
    AssignmentAttemptTimeLimitOutOfRange,
    AttemptLimitOutOfRange,
    IllegalLifecycleTransition,
    InvalidInstructions,
}

/// Answer-free bounded correction contract for local teaching-settings input.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AssignmentRevisionDefinitionValidationFailure {
    pub error: AssignmentRevisionDefinitionFailureCode,
    pub field: AssignmentRevisionDefinitionField,
    pub reason: AssignmentRevisionDefinitionFailureReason,
    pub message: String,
}

/// Refusal reason while translating an instructor local schedule at the server boundary.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AssignmentRevisionDefinitionLocalError {
    /// The browser repeated a zone other than the course's authoritative IANA zone.
    CourseTimeZoneMismatch,
    /// A local schedule timestamp lies outside the inclusive course calendar.
    OutsideCourseTerm(AssignmentRevisionDefinitionField),
    /// A local wall-clock time never occurred because DST skipped it.
    NonexistentLocalTime(AssignmentRevisionDefinitionField),
    /// A local wall-clock time occurred twice and lacks an offset discriminator.
    AmbiguousLocalTime(AssignmentRevisionDefinitionField),
    /// A timestamp cannot be represented by Chrono's supported range.
    TimestampOutOfRange(AssignmentRevisionDefinitionField),
    /// Available, due, and closes values are not chronological.
    ScheduleOutOfOrder,
    /// The time limit exceeds PostgreSQL's supported integer range.
    AssignmentAttemptTimeLimitOutOfRange,
    /// The attempt limit exceeds PostgreSQL's supported integer range.
    AttemptLimitOutOfRange,
}

impl AssignmentRevisionDefinitionLocalError {
    pub fn field(self) -> AssignmentRevisionDefinitionField {
        match self {
            Self::CourseTimeZoneMismatch => AssignmentRevisionDefinitionField::TimeZone,
            Self::OutsideCourseTerm(field)
            | Self::NonexistentLocalTime(field)
            | Self::AmbiguousLocalTime(field)
            | Self::TimestampOutOfRange(field) => field,
            Self::ScheduleOutOfOrder => AssignmentRevisionDefinitionField::Schedule,
            Self::AssignmentAttemptTimeLimitOutOfRange => {
                AssignmentRevisionDefinitionField::AssignmentAttemptTimeLimitSeconds
            }
            Self::AttemptLimitOutOfRange => AssignmentRevisionDefinitionField::AttemptLimit,
        }
    }

    pub fn reason(self) -> AssignmentRevisionDefinitionFailureReason {
        match self {
            Self::CourseTimeZoneMismatch => {
                AssignmentRevisionDefinitionFailureReason::CourseTimeZoneMismatch
            }
            Self::OutsideCourseTerm(_) => {
                AssignmentRevisionDefinitionFailureReason::OutsideCourseTerm
            }
            Self::NonexistentLocalTime(_) => {
                AssignmentRevisionDefinitionFailureReason::NonexistentLocalTime
            }
            Self::AmbiguousLocalTime(_) => {
                AssignmentRevisionDefinitionFailureReason::AmbiguousLocalTime
            }
            Self::TimestampOutOfRange(_) => {
                AssignmentRevisionDefinitionFailureReason::TimestampOutOfRange
            }
            Self::ScheduleOutOfOrder => {
                AssignmentRevisionDefinitionFailureReason::ScheduleOutOfOrder
            }
            Self::AssignmentAttemptTimeLimitOutOfRange => {
                AssignmentRevisionDefinitionFailureReason::AssignmentAttemptTimeLimitOutOfRange
            }
            Self::AttemptLimitOutOfRange => {
                AssignmentRevisionDefinitionFailureReason::AttemptLimitOutOfRange
            }
        }
    }
}

impl std::fmt::Display for AssignmentRevisionDefinitionLocalError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(match self {
            Self::CourseTimeZoneMismatch => {
                "Assignment Revision Definition time zone must match the course time zone"
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

impl std::error::Error for AssignmentRevisionDefinitionLocalError {}

fn validate_local_ordering(
    available_at: &Option<CourseLocalDateAndTime>,
    due_at: &Option<CourseLocalDateAndTime>,
    closes_at: &Option<CourseLocalDateAndTime>,
) -> Result<(), AssignmentRevisionDefinitionLocalError> {
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
        return Err(AssignmentRevisionDefinitionLocalError::ScheduleOutOfOrder);
    }
    Ok(())
}

fn validate_absolute_ordering(
    available_at: Option<ActivityTimestamp>,
    due_at: Option<ActivityTimestamp>,
    closes_at: Option<ActivityTimestamp>,
) -> Result<(), AssignmentRevisionDefinitionLocalError> {
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
        return Err(AssignmentRevisionDefinitionLocalError::ScheduleOutOfOrder);
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
    field: AssignmentRevisionDefinitionField,
) -> Result<Option<ActivityTimestamp>, AssignmentRevisionDefinitionLocalError> {
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
    field: AssignmentRevisionDefinitionField,
) -> Result<ActivityTimestamp, AssignmentRevisionDefinitionLocalError> {
    let naive = value.naive();
    let date = naive.date().format("%Y-%m-%d").to_string();
    if date.as_str() < course_term.start_date().as_str()
        || date.as_str() > course_term.end_date().as_str()
    {
        return Err(AssignmentRevisionDefinitionLocalError::OutsideCourseTerm(
            field,
        ));
    }
    match course_time_zone(course_term).from_local_datetime(&naive) {
        LocalResult::Single(value) => Ok(ActivityTimestamp::from_unix_millis(
            value.timestamp_millis(),
        )),
        LocalResult::None => {
            Err(AssignmentRevisionDefinitionLocalError::NonexistentLocalTime(field))
        }
        LocalResult::Ambiguous(_, _) => Err(
            AssignmentRevisionDefinitionLocalError::AmbiguousLocalTime(field),
        ),
    }
}

fn project_optional_course_local_timestamp(
    value: Option<ActivityTimestamp>,
    course_term: &CourseTerm,
    field: AssignmentRevisionDefinitionField,
) -> Result<Option<CourseLocalDateAndTime>, AssignmentRevisionDefinitionLocalError> {
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
    value: ActivityTimestamp,
    course_term: &CourseTerm,
    field: AssignmentRevisionDefinitionField,
) -> Result<CourseLocalDateAndTime, AssignmentRevisionDefinitionLocalError> {
    let utc = DateTime::<Utc>::from_timestamp_millis(value.as_unix_millis()).ok_or(
        AssignmentRevisionDefinitionLocalError::TimestampOutOfRange(field),
    )?;
    let local = course_time_zone(course_term).from_utc_datetime(&utc.naive_utc());
    let wall_clock =
        CourseLocalDateAndTime::parse(&local.format("%Y-%m-%dT%H:%M:%S%.3f").to_string())
            .expect("formatted course-local timestamp is valid");
    match course_time_zone(course_term).from_local_datetime(&wall_clock.naive()) {
        LocalResult::Single(round_trip)
            if round_trip.timestamp_millis() == value.as_unix_millis() => {}
        LocalResult::Single(_) | LocalResult::Ambiguous(_, _) => {
            return Err(AssignmentRevisionDefinitionLocalError::AmbiguousLocalTime(
                field,
            ));
        }
        LocalResult::None => {
            return Err(AssignmentRevisionDefinitionLocalError::NonexistentLocalTime(field));
        }
    }
    let date = local.format("%Y-%m-%d").to_string();
    if date.as_str() < course_term.start_date().as_str()
        || date.as_str() > course_term.end_date().as_str()
    {
        return Err(AssignmentRevisionDefinitionLocalError::OutsideCourseTerm(
            field,
        ));
    }
    Ok(wall_clock)
}

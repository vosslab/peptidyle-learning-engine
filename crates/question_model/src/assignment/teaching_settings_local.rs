use std::num::NonZeroU32;

use chrono::{DateTime, LocalResult, NaiveDateTime, TimeZone, Utc};
use serde::{Deserialize, Serialize};

use super::{
    AssignmentDeadlineBehavior, AssignmentInstructions, AssignmentLifecycle,
    AssignmentTeachingSettings, BaseAssignmentPolicy, LateSubmissionPolicy,
    MAX_ASSIGNMENT_ATTEMPT_LIMIT, MAX_ASSIGNMENT_TIME_LIMIT_SECONDS,
};
use crate::{ActivityTimestamp, CourseTerm, IanaTimeZone};

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
        available_at: CourseLocalDateTime,
    },
    Open,
    Closed {
        closed_at: Option<CourseLocalDateTime>,
    },
    Archived,
}

pub fn derive_instructor_assignment_current_state(
    term: &CourseTerm,
    settings: &AssignmentTeachingSettings,
    now: ActivityTimestamp,
) -> Result<InstructorAssignmentCurrentState, AssignmentTeachingSettingsLocalError> {
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
                    AssignmentTeachingSettingsField::AvailableAt,
                )?
                .expect("published scheduled state has an available-at instant"),
            })
        }
        Published => {
            let due_boundary = (settings.base_policy.late_submission
                == LateSubmissionPolicy::Reject)
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
                        AssignmentTeachingSettingsField::ClosesAt,
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
/// [`AssignmentTeachingSettings`]. Its wire form is exactly
/// `YYYY-MM-DDTHH:MM:SS.sss`, which is accepted by HTML `datetime-local`
/// controls with `step="0.001"`. A browser may initialize its form at whole
/// minutes, but this canonical wire value never loses an existing server
/// timestamp's supported millisecond precision.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct CourseLocalDateTime(String);

impl CourseLocalDateTime {
    /// Parses one exact millisecond-precision local wall-clock string.
    pub fn parse(value: &str) -> Result<Self, CourseLocalDateTimeError> {
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
            return Err(CourseLocalDateTimeError);
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
        field: AssignmentTeachingSettingsField,
    ) -> Result<ActivityTimestamp, AssignmentTeachingSettingsLocalError> {
        resolve_course_local_timestamp(self, course_term, field)
    }

    /// Projects one server-resolved instant into this course's local wire form.
    ///
    /// The supplied field identifies the exact correction target if an instant
    /// cannot round-trip through the course calendar and zone.
    pub fn from_activity_timestamp(
        value: ActivityTimestamp,
        course_term: &CourseTerm,
        field: AssignmentTeachingSettingsField,
    ) -> Result<Self, AssignmentTeachingSettingsLocalError> {
        project_course_local_timestamp(value, course_term, field)
    }
}

impl TryFrom<String> for CourseLocalDateTime {
    type Error = CourseLocalDateTimeError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::parse(&value)
    }
}

impl From<CourseLocalDateTime> for String {
    fn from(value: CourseLocalDateTime) -> Self {
        value.0
    }
}

/// A local wall-clock string is not exact `YYYY-MM-DDTHH:MM:SS.sss` calendar time.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CourseLocalDateTimeError;

impl std::fmt::Display for CourseLocalDateTimeError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("course-local date time must be exact YYYY-MM-DDTHH:MM:SS.sss")
    }
}

impl std::error::Error for CourseLocalDateTimeError {}

/// Browser-facing instructor projection of stored absolute teaching settings.
///
/// This is an edit/display boundary only. It contains local strings plus the
/// course-owned IANA zone so a browser never consults its own machine zone.
/// [`AssignmentTeachingSettings`] and its [`BaseAssignmentPolicy`] remain the
/// only stored and effective-policy authority.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct InstructorAssignmentTeachingSettingsLocal {
    /// Authoritative course IANA zone shown beside local form controls.
    pub time_zone: IanaTimeZone,
    /// Professor-controlled assignment lifecycle intent.
    pub lifecycle: AssignmentLifecycle,
    /// Validated learner-facing plain-text instructions.
    pub instructions: AssignmentInstructions,
    /// First local course time at which learners may open the assignment.
    pub available_at: Option<CourseLocalDateTime>,
    /// Ordinary local course due time.
    pub due_at: Option<CourseLocalDateTime>,
    /// Hard local course time after which new work is closed.
    pub closes_at: Option<CourseLocalDateTime>,
    /// Whole-run limit when one applies.
    pub time_limit_seconds: Option<NonZeroU32>,
    /// Maximum number of runs when one applies.
    pub attempt_limit: Option<NonZeroU32>,
    /// Treatment of work after the ordinary due instant.
    pub late_submission: LateSubmissionPolicy,
    /// Server behavior at an effective assignment deadline.
    pub deadline_behavior: AssignmentDeadlineBehavior,
}

impl InstructorAssignmentTeachingSettingsLocal {
    /// Builds a browser projection after validating limits and local ordering.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        time_zone: IanaTimeZone,
        lifecycle: AssignmentLifecycle,
        instructions: AssignmentInstructions,
        available_at: Option<CourseLocalDateTime>,
        due_at: Option<CourseLocalDateTime>,
        closes_at: Option<CourseLocalDateTime>,
        time_limit_seconds: Option<NonZeroU32>,
        attempt_limit: Option<NonZeroU32>,
        late_submission: LateSubmissionPolicy,
        deadline_behavior: AssignmentDeadlineBehavior,
    ) -> Result<Self, AssignmentTeachingSettingsLocalError> {
        if time_limit_seconds.is_some_and(|limit| limit.get() > MAX_ASSIGNMENT_TIME_LIMIT_SECONDS) {
            return Err(AssignmentTeachingSettingsLocalError::TimeLimitOutOfRange);
        }
        if attempt_limit.is_some_and(|limit| limit.get() > MAX_ASSIGNMENT_ATTEMPT_LIMIT) {
            return Err(AssignmentTeachingSettingsLocalError::AttemptLimitOutOfRange);
        }
        validate_local_ordering(&available_at, &due_at, &closes_at)?;
        Ok(Self {
            time_zone,
            lifecycle,
            instructions,
            available_at,
            due_at,
            closes_at,
            time_limit_seconds,
            attempt_limit,
            late_submission,
            deadline_behavior,
        })
    }

    /// Resolves this local instructor input against the course-owned IANA zone.
    ///
    /// The server calls this before a store mutation. It refuses a zone mismatch,
    /// course-calendar escape, DST gap, DST ambiguity, and invalid ordering.
    pub fn into_absolute(
        self,
        course_term: &CourseTerm,
    ) -> Result<AssignmentTeachingSettings, AssignmentTeachingSettingsLocalError> {
        self.validate()?;
        if self.time_zone != *course_term.time_zone() {
            return Err(AssignmentTeachingSettingsLocalError::CourseTimeZoneMismatch);
        }
        let available_at = resolve_optional_course_local_timestamp(
            self.available_at.as_ref(),
            course_term,
            AssignmentTeachingSettingsField::AvailableAt,
        )?;
        let due_at = resolve_optional_course_local_timestamp(
            self.due_at.as_ref(),
            course_term,
            AssignmentTeachingSettingsField::DueAt,
        )?;
        let closes_at = resolve_optional_course_local_timestamp(
            self.closes_at.as_ref(),
            course_term,
            AssignmentTeachingSettingsField::ClosesAt,
        )?;
        validate_absolute_ordering(available_at, due_at, closes_at)?;
        Ok(AssignmentTeachingSettings {
            lifecycle: self.lifecycle,
            instructions: self.instructions,
            base_policy: BaseAssignmentPolicy {
                available_at,
                due_at,
                closes_at,
                time_limit_seconds: self.time_limit_seconds,
                attempt_limit: self.attempt_limit,
                late_submission: self.late_submission,
                deadline_behavior: self.deadline_behavior,
            },
        })
    }

    fn validate(&self) -> Result<(), AssignmentTeachingSettingsLocalError> {
        if self
            .time_limit_seconds
            .is_some_and(|limit| limit.get() > MAX_ASSIGNMENT_TIME_LIMIT_SECONDS)
        {
            return Err(AssignmentTeachingSettingsLocalError::TimeLimitOutOfRange);
        }
        if self
            .attempt_limit
            .is_some_and(|limit| limit.get() > MAX_ASSIGNMENT_ATTEMPT_LIMIT)
        {
            return Err(AssignmentTeachingSettingsLocalError::AttemptLimitOutOfRange);
        }
        validate_local_ordering(&self.available_at, &self.due_at, &self.closes_at)
    }

    /// Projects stored absolute settings into exact local course wall-clock values.
    pub fn from_absolute(
        course_term: &CourseTerm,
        settings: &AssignmentTeachingSettings,
    ) -> Result<Self, AssignmentTeachingSettingsLocalError> {
        let available_at = project_optional_course_local_timestamp(
            settings.base_policy.available_at,
            course_term,
            AssignmentTeachingSettingsField::AvailableAt,
        )?;
        let due_at = project_optional_course_local_timestamp(
            settings.base_policy.due_at,
            course_term,
            AssignmentTeachingSettingsField::DueAt,
        )?;
        let closes_at = project_optional_course_local_timestamp(
            settings.base_policy.closes_at,
            course_term,
            AssignmentTeachingSettingsField::ClosesAt,
        )?;
        Self::new(
            course_term.time_zone().clone(),
            settings.lifecycle,
            settings.instructions.clone(),
            available_at,
            due_at,
            closes_at,
            settings.base_policy.time_limit_seconds,
            settings.base_policy.attempt_limit,
            settings.base_policy.late_submission,
            settings.base_policy.deadline_behavior,
        )
    }
}

/// Refusal reason while translating an instructor local schedule at the server boundary.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum AssignmentTeachingSettingsFailureCode {
    /// The submitted teaching settings cannot be accepted.
    AssignmentTeachingSettingsInvalid,
}

/// Browser-safe field that needs a correction in assignment teaching settings.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum AssignmentTeachingSettingsField {
    TeachingSettings,
    TimeZone,
    AvailableAt,
    DueAt,
    ClosesAt,
    Schedule,
    TimeLimitSeconds,
    AttemptLimit,
    Lifecycle,
    Instructions,
}

/// Browser-safe reason an assignment teaching-settings input was refused.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum AssignmentTeachingSettingsFailureReason {
    InvalidInput,
    CourseTimeZoneMismatch,
    OutsideCourseTerm,
    NonexistentLocalTime,
    AmbiguousLocalTime,
    TimestampOutOfRange,
    ScheduleOutOfOrder,
    TimeLimitOutOfRange,
    AttemptLimitOutOfRange,
    IllegalLifecycleTransition,
    InvalidInstructions,
}

/// Answer-free bounded correction contract for local teaching-settings input.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AssignmentTeachingSettingsValidationFailure {
    pub error: AssignmentTeachingSettingsFailureCode,
    pub field: AssignmentTeachingSettingsField,
    pub reason: AssignmentTeachingSettingsFailureReason,
    pub message: String,
}

/// Refusal reason while translating an instructor local schedule at the server boundary.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AssignmentTeachingSettingsLocalError {
    /// The browser repeated a zone other than the course's authoritative IANA zone.
    CourseTimeZoneMismatch,
    /// A local schedule timestamp lies outside the inclusive course calendar.
    OutsideCourseTerm(AssignmentTeachingSettingsField),
    /// A local wall-clock time never occurred because DST skipped it.
    NonexistentLocalTime(AssignmentTeachingSettingsField),
    /// A local wall-clock time occurred twice and lacks an offset discriminator.
    AmbiguousLocalTime(AssignmentTeachingSettingsField),
    /// A timestamp cannot be represented by Chrono's supported range.
    TimestampOutOfRange(AssignmentTeachingSettingsField),
    /// Available, due, and closes values are not chronological.
    ScheduleOutOfOrder,
    /// The time limit exceeds PostgreSQL's supported integer range.
    TimeLimitOutOfRange,
    /// The attempt limit exceeds PostgreSQL's supported integer range.
    AttemptLimitOutOfRange,
}

impl AssignmentTeachingSettingsLocalError {
    pub fn field(self) -> AssignmentTeachingSettingsField {
        match self {
            Self::CourseTimeZoneMismatch => AssignmentTeachingSettingsField::TimeZone,
            Self::OutsideCourseTerm(field)
            | Self::NonexistentLocalTime(field)
            | Self::AmbiguousLocalTime(field)
            | Self::TimestampOutOfRange(field) => field,
            Self::ScheduleOutOfOrder => AssignmentTeachingSettingsField::Schedule,
            Self::TimeLimitOutOfRange => AssignmentTeachingSettingsField::TimeLimitSeconds,
            Self::AttemptLimitOutOfRange => AssignmentTeachingSettingsField::AttemptLimit,
        }
    }

    pub fn reason(self) -> AssignmentTeachingSettingsFailureReason {
        match self {
            Self::CourseTimeZoneMismatch => {
                AssignmentTeachingSettingsFailureReason::CourseTimeZoneMismatch
            }
            Self::OutsideCourseTerm(_) => {
                AssignmentTeachingSettingsFailureReason::OutsideCourseTerm
            }
            Self::NonexistentLocalTime(_) => {
                AssignmentTeachingSettingsFailureReason::NonexistentLocalTime
            }
            Self::AmbiguousLocalTime(_) => {
                AssignmentTeachingSettingsFailureReason::AmbiguousLocalTime
            }
            Self::TimestampOutOfRange(_) => {
                AssignmentTeachingSettingsFailureReason::TimestampOutOfRange
            }
            Self::ScheduleOutOfOrder => AssignmentTeachingSettingsFailureReason::ScheduleOutOfOrder,
            Self::TimeLimitOutOfRange => {
                AssignmentTeachingSettingsFailureReason::TimeLimitOutOfRange
            }
            Self::AttemptLimitOutOfRange => {
                AssignmentTeachingSettingsFailureReason::AttemptLimitOutOfRange
            }
        }
    }
}

impl std::fmt::Display for AssignmentTeachingSettingsLocalError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(match self {
            Self::CourseTimeZoneMismatch => {
                "teaching settings time zone must match the course time zone"
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
            Self::TimeLimitOutOfRange => "teaching time limit exceeds the supported range",
            Self::AttemptLimitOutOfRange => "teaching attempt limit exceeds the supported range",
        })
    }
}

impl std::error::Error for AssignmentTeachingSettingsLocalError {}

fn validate_local_ordering(
    available_at: &Option<CourseLocalDateTime>,
    due_at: &Option<CourseLocalDateTime>,
    closes_at: &Option<CourseLocalDateTime>,
) -> Result<(), AssignmentTeachingSettingsLocalError> {
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
        return Err(AssignmentTeachingSettingsLocalError::ScheduleOutOfOrder);
    }
    Ok(())
}

fn validate_absolute_ordering(
    available_at: Option<ActivityTimestamp>,
    due_at: Option<ActivityTimestamp>,
    closes_at: Option<ActivityTimestamp>,
) -> Result<(), AssignmentTeachingSettingsLocalError> {
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
        return Err(AssignmentTeachingSettingsLocalError::ScheduleOutOfOrder);
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
    value: Option<&CourseLocalDateTime>,
    course_term: &CourseTerm,
    field: AssignmentTeachingSettingsField,
) -> Result<Option<ActivityTimestamp>, AssignmentTeachingSettingsLocalError> {
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
    value: &CourseLocalDateTime,
    course_term: &CourseTerm,
    field: AssignmentTeachingSettingsField,
) -> Result<ActivityTimestamp, AssignmentTeachingSettingsLocalError> {
    let naive = value.naive();
    let date = naive.date().format("%Y-%m-%d").to_string();
    if date.as_str() < course_term.start_date().as_str()
        || date.as_str() > course_term.end_date().as_str()
    {
        return Err(AssignmentTeachingSettingsLocalError::OutsideCourseTerm(
            field,
        ));
    }
    match course_time_zone(course_term).from_local_datetime(&naive) {
        LocalResult::Single(value) => Ok(ActivityTimestamp::from_unix_millis(
            value.timestamp_millis(),
        )),
        LocalResult::None => Err(AssignmentTeachingSettingsLocalError::NonexistentLocalTime(
            field,
        )),
        LocalResult::Ambiguous(_, _) => Err(
            AssignmentTeachingSettingsLocalError::AmbiguousLocalTime(field),
        ),
    }
}

fn project_optional_course_local_timestamp(
    value: Option<ActivityTimestamp>,
    course_term: &CourseTerm,
    field: AssignmentTeachingSettingsField,
) -> Result<Option<CourseLocalDateTime>, AssignmentTeachingSettingsLocalError> {
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
    field: AssignmentTeachingSettingsField,
) -> Result<CourseLocalDateTime, AssignmentTeachingSettingsLocalError> {
    let utc = DateTime::<Utc>::from_timestamp_millis(value.as_unix_millis()).ok_or(
        AssignmentTeachingSettingsLocalError::TimestampOutOfRange(field),
    )?;
    let local = course_time_zone(course_term).from_utc_datetime(&utc.naive_utc());
    let wall_clock = CourseLocalDateTime::parse(&local.format("%Y-%m-%dT%H:%M:%S%.3f").to_string())
        .expect("formatted course-local timestamp is valid");
    match course_time_zone(course_term).from_local_datetime(&wall_clock.naive()) {
        LocalResult::Single(round_trip)
            if round_trip.timestamp_millis() == value.as_unix_millis() => {}
        LocalResult::Single(_) | LocalResult::Ambiguous(_, _) => {
            return Err(AssignmentTeachingSettingsLocalError::AmbiguousLocalTime(
                field,
            ));
        }
        LocalResult::None => {
            return Err(AssignmentTeachingSettingsLocalError::NonexistentLocalTime(
                field,
            ));
        }
    }
    let date = local.format("%Y-%m-%d").to_string();
    if date.as_str() < course_term.start_date().as_str()
        || date.as_str() > course_term.end_date().as_str()
    {
        return Err(AssignmentTeachingSettingsLocalError::OutsideCourseTerm(
            field,
        ));
    }
    Ok(wall_clock)
}

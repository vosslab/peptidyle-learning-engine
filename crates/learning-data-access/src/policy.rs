//! Current course-group accommodations and assignment timing policy resolution.

use crate::{AssignmentRevision, StoreError, validate_assignment_timing};
use question_model::{
    ActivityTimestamp, AssignmentId, AssignmentPolicyExceptionId, AssignmentTimingPolicy,
    CourseGroupId, CourseId, QuestionAttemptId, StudentId, TenantId, UserId,
};
use serde::{Deserialize, Serialize};

/// Server-issued optimistic revision for one current course group.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct CourseGroupRevision(u64);

impl CourseGroupRevision {
    pub(crate) const INITIAL: Self = Self(1);
    const MAX: u64 = i64::MAX as u64;

    /// Returns the positive stored revision number.
    pub fn value(self) -> u64 {
        self.0
    }

    pub(crate) fn next(self) -> Result<Self, StoreError> {
        self.0
            .checked_add(1)
            .filter(|value| *value <= Self::MAX)
            .map(Self)
            .ok_or_else(|| {
                StoreError::Unavailable("course group revision limit reached".to_string())
            })
    }

    #[cfg(feature = "postgres")]
    pub(crate) fn from_stored(value: i64) -> Result<Self, StoreError> {
        let value = u64::try_from(value).map_err(|_| {
            StoreError::Unavailable("stored course group revision is invalid".to_string())
        })?;
        if value == 0 {
            return Err(StoreError::Unavailable(
                "stored course group revision is invalid".to_string(),
            ));
        }
        Ok(Self(value))
    }
}

/// Current course group whose members may share an assignment accommodation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CourseGroupRecord {
    pub id: CourseGroupId,
    pub tenant: TenantId,
    pub course: CourseId,
    pub title: String,
    pub members: Vec<UserId>,
}

/// One course group together with its exact compare-and-swap revision.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoredCourseGroup {
    pub record: CourseGroupRecord,
    pub revision: CourseGroupRevision,
}

/// Instructor-authenticated create or replacement of a course group.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PutCourseGroupCommand {
    pub actor: UserId,
    pub expected_revision: Option<CourseGroupRevision>,
    pub record: CourseGroupRecord,
}

/// One exception target. A student target is assignment-enrollment identity;
/// a group target is current course membership identity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum AssignmentPolicyExceptionTarget {
    Student(StudentId),
    CourseGroup(CourseGroupId),
}

/// Explicit availability endpoint override. `Unrestricted` is distinct from
/// an absent field, which means this exception does not address that endpoint.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum AssignmentExceptionTimestamp {
    Unrestricted,
    At(ActivityTimestamp),
}

/// Explicit attempt/timer override. `Unlimited` is distinct from inheritance.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum AssignmentExceptionLimit {
    Unlimited,
    Value(u32),
}

/// Mutable current accommodation for one student or course group.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AssignmentPolicyException {
    pub id: AssignmentPolicyExceptionId,
    pub target: AssignmentPolicyExceptionTarget,
    pub available_at: Option<AssignmentExceptionTimestamp>,
    pub closes_at: Option<AssignmentExceptionTimestamp>,
    pub time_limit_seconds: Option<AssignmentExceptionLimit>,
    pub attempt_limit: Option<AssignmentExceptionLimit>,
}

/// One stored exception paired with the assignment's shared revision.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoredAssignmentPolicyException {
    pub exception: AssignmentPolicyException,
    pub assignment_revision: AssignmentRevision,
}

/// Effective learner policy and the exceptions that actually expanded it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResolvedAssignmentTiming {
    pub tenant: TenantId,
    pub course: CourseId,
    pub assignment: AssignmentId,
    pub student: StudentId,
    pub policy: AssignmentTimingPolicy,
    pub contributors: Vec<AssignmentPolicyExceptionTarget>,
    pub revision: AssignmentRevision,
}

/// Policy explanation recorded for one issued attempt. Terminal work retains
/// the last policy that governed it instead of being rewritten by later edits.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResolvedAttemptTiming {
    pub attempt: QuestionAttemptId,
    pub policy: AssignmentTimingPolicy,
    pub contributors: Vec<AssignmentPolicyExceptionTarget>,
}

/// Revision-checked replacement for one target's current accommodation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SetAssignmentPolicyExceptionCommand {
    pub actor: UserId,
    pub course: CourseId,
    pub assignment: AssignmentId,
    pub expected_revision: AssignmentRevision,
    pub exception: AssignmentPolicyException,
}

/// Revision-checked removal of one current accommodation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DeleteAssignmentPolicyExceptionCommand {
    pub actor: UserId,
    pub course: CourseId,
    pub assignment: AssignmentId,
    pub expected_revision: AssignmentRevision,
    pub exception: AssignmentPolicyExceptionId,
}

/// Effective policy fields plus only the exception targets that expanded them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ResolvedAssignmentTimingPolicy {
    pub policy: AssignmentTimingPolicy,
    pub contributors: Vec<AssignmentPolicyExceptionTarget>,
}

/// Validates one exception before either backend mutates current state.
pub(crate) fn validate_assignment_policy_exception(
    exception: &AssignmentPolicyException,
) -> Result<(), StoreError> {
    if exception.available_at.is_none()
        && exception.closes_at.is_none()
        && exception.time_limit_seconds.is_none()
        && exception.attempt_limit.is_none()
    {
        return Err(StoreError::InvalidRecord(
            "an assignment policy exception must override at least one field".to_string(),
        ));
    }
    if exception.time_limit_seconds == Some(AssignmentExceptionLimit::Value(0)) {
        return Err(StoreError::InvalidRecord(
            "exception time limit must be greater than zero".to_string(),
        ));
    }
    if exception.time_limit_seconds.is_some_and(|limit| {
		matches!(limit, AssignmentExceptionLimit::Value(value) if value > question_model::MAX_ASSIGNMENT_TIME_LIMIT_SECONDS)
	}) {
		return Err(StoreError::InvalidRecord(format!(
			"exception time limit must not exceed {} seconds",
			question_model::MAX_ASSIGNMENT_TIME_LIMIT_SECONDS
		)));
	}
    if exception.attempt_limit == Some(AssignmentExceptionLimit::Value(0)) {
        return Err(StoreError::InvalidRecord(
            "exception attempt limit must be greater than zero".to_string(),
        ));
    }
    if let (
        Some(AssignmentExceptionTimestamp::At(available_at)),
        Some(AssignmentExceptionTimestamp::At(closes_at)),
    ) = (exception.available_at, exception.closes_at)
        && available_at > closes_at
    {
        return Err(StoreError::InvalidRecord(
            "exception availability and close date must be ordered".to_string(),
        ));
    }
    Ok(())
}

/// Resolves all applicable accommodations against the assignment policy.
/// Every dimension can only become more permissive; an exception can never
/// shorten another learner's access by accident.
pub(crate) fn resolve_assignment_policy(
    base: AssignmentTimingPolicy,
    exceptions: &[AssignmentPolicyException],
) -> Result<ResolvedAssignmentTimingPolicy, StoreError> {
    validate_assignment_timing(base)?;
    let mut policy = base;
    let mut contributors = std::collections::BTreeSet::new();
    for exception in exceptions {
        validate_assignment_policy_exception(exception)?;
        if exception_expands_policy(base, exception) {
            contributors.insert(exception.target);
        }
        if let Some(value) = exception.available_at {
            expand_start_boundary(&mut policy.available_at, value);
        }
        if let Some(value) = exception.closes_at {
            expand_end_boundary(&mut policy.closes_at, value);
        }
        if let Some(value) = exception.time_limit_seconds {
            expand_numeric_limit(&mut policy.time_limit_seconds, value);
        }
        if let Some(value) = exception.attempt_limit {
            expand_numeric_limit(&mut policy.attempt_limit, value);
        }
    }
    validate_assignment_timing(policy)?;
    Ok(ResolvedAssignmentTimingPolicy {
        policy,
        contributors: contributors.into_iter().collect(),
    })
}

fn exception_expands_policy(
    base: AssignmentTimingPolicy,
    exception: &AssignmentPolicyException,
) -> bool {
    let mut available_at = base.available_at;
    let mut closes_at = base.closes_at;
    let mut time_limit_seconds = base.time_limit_seconds;
    let mut attempt_limit = base.attempt_limit;
    exception
        .available_at
        .is_some_and(|value| expand_start_boundary(&mut available_at, value))
        || exception
            .closes_at
            .is_some_and(|value| expand_end_boundary(&mut closes_at, value))
        || exception
            .time_limit_seconds
            .is_some_and(|value| expand_numeric_limit(&mut time_limit_seconds, value))
        || exception
            .attempt_limit
            .is_some_and(|value| expand_numeric_limit(&mut attempt_limit, value))
}

fn expand_start_boundary(
    current: &mut Option<ActivityTimestamp>,
    exception: AssignmentExceptionTimestamp,
) -> bool {
    match exception {
        AssignmentExceptionTimestamp::Unrestricted if current.is_some() => {
            *current = None;
            true
        }
        AssignmentExceptionTimestamp::At(value)
            if current.is_some_and(|existing| value < existing) =>
        {
            *current = Some(value);
            true
        }
        AssignmentExceptionTimestamp::Unrestricted | AssignmentExceptionTimestamp::At(_) => false,
    }
}

fn expand_end_boundary(
    current: &mut Option<ActivityTimestamp>,
    exception: AssignmentExceptionTimestamp,
) -> bool {
    match exception {
        AssignmentExceptionTimestamp::Unrestricted if current.is_some() => {
            *current = None;
            true
        }
        AssignmentExceptionTimestamp::At(value)
            if current.is_some_and(|existing| value > existing) =>
        {
            *current = Some(value);
            true
        }
        AssignmentExceptionTimestamp::Unrestricted | AssignmentExceptionTimestamp::At(_) => false,
    }
}

fn expand_numeric_limit(current: &mut Option<u32>, exception: AssignmentExceptionLimit) -> bool {
    match exception {
        AssignmentExceptionLimit::Unlimited if current.is_some() => {
            *current = None;
            true
        }
        AssignmentExceptionLimit::Value(value)
            if current.is_some_and(|existing| value > existing) =>
        {
            *current = Some(value);
            true
        }
        AssignmentExceptionLimit::Unlimited | AssignmentExceptionLimit::Value(_) => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    fn id(value: u128) -> Uuid {
        Uuid::from_u128(value)
    }

    #[test]
    fn applicable_exceptions_resolve_each_dimension_most_permissively() {
        let student = StudentId::from_uuid(id(1));
        let group = CourseGroupId::from_uuid(id(2));
        let base = AssignmentTimingPolicy {
            available_at: Some(ActivityTimestamp::from_unix_millis(100)),
            closes_at: Some(ActivityTimestamp::from_unix_millis(200)),
            time_limit_seconds: Some(10),
            attempt_limit: Some(1),
            ..AssignmentTimingPolicy::default()
        };
        let group_exception = AssignmentPolicyException {
            id: AssignmentPolicyExceptionId::from_uuid(id(3)),
            target: AssignmentPolicyExceptionTarget::CourseGroup(group),
            available_at: Some(AssignmentExceptionTimestamp::At(
                ActivityTimestamp::from_unix_millis(90),
            )),
            closes_at: Some(AssignmentExceptionTimestamp::At(
                ActivityTimestamp::from_unix_millis(300),
            )),
            time_limit_seconds: Some(AssignmentExceptionLimit::Value(20)),
            attempt_limit: Some(AssignmentExceptionLimit::Value(2)),
        };
        let student_exception = AssignmentPolicyException {
            id: AssignmentPolicyExceptionId::from_uuid(id(4)),
            target: AssignmentPolicyExceptionTarget::Student(student),
            available_at: Some(AssignmentExceptionTimestamp::Unrestricted),
            closes_at: Some(AssignmentExceptionTimestamp::At(
                ActivityTimestamp::from_unix_millis(250),
            )),
            time_limit_seconds: Some(AssignmentExceptionLimit::Unlimited),
            attempt_limit: Some(AssignmentExceptionLimit::Value(1)),
        };
        let resolved = resolve_assignment_policy(base, &[group_exception, student_exception])
            .expect("valid accommodations");
        assert_eq!(resolved.policy.available_at, None);
        assert_eq!(
            resolved.policy.closes_at,
            Some(ActivityTimestamp::from_unix_millis(300))
        );
        assert_eq!(resolved.policy.time_limit_seconds, None);
        assert_eq!(resolved.policy.attempt_limit, Some(2));
        assert_eq!(
            resolved.contributors,
            vec![
                AssignmentPolicyExceptionTarget::Student(student),
                AssignmentPolicyExceptionTarget::CourseGroup(group),
            ]
        );
    }

    #[test]
    fn exception_validation_refuses_empty_zero_and_reversed_overrides() {
        let mut exception = AssignmentPolicyException {
            id: AssignmentPolicyExceptionId::from_uuid(id(10)),
            target: AssignmentPolicyExceptionTarget::Student(StudentId::from_uuid(id(11))),
            available_at: None,
            closes_at: None,
            time_limit_seconds: None,
            attempt_limit: None,
        };
        assert!(validate_assignment_policy_exception(&exception).is_err());
        exception.time_limit_seconds = Some(AssignmentExceptionLimit::Value(0));
        assert!(validate_assignment_policy_exception(&exception).is_err());
        exception.time_limit_seconds = None;
        exception.available_at = Some(AssignmentExceptionTimestamp::At(
            ActivityTimestamp::from_unix_millis(2),
        ));
        exception.closes_at = Some(AssignmentExceptionTimestamp::At(
            ActivityTimestamp::from_unix_millis(1),
        ));
        assert!(validate_assignment_policy_exception(&exception).is_err());
    }
}

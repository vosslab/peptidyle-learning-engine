//! Pure resolution of a Student's current assignment policy.
//!
//! S5 is the sole authority that evaluates membership and mints an
//! [`EntitlementGrant`]. This module consumes that grant: it validates supplied
//! modifier identifiers against the grant's opaque scopes, then resolves the
//! assignment window and limits without reading roster state or a clock.

use std::num::NonZeroU32;

use chrono::{DateTime, Utc};
use question_model::{
    ActivityTimestamp, AssignmentDeadlineBehavior, AssignmentLifecycle, CourseTerm,
    LateSubmissionPolicy, MAX_ASSIGNMENT_ATTEMPT_LIMIT, MAX_ASSIGNMENT_TIME_LIMIT_SECONDS,
    StudentRecordId,
};

/// Compatibility re-export for established policy-resolution callers.
pub use question_model::BaseAssignmentPolicy;

use crate::entitlement::{EntitlementDecision, EntitlementDenial, SyntheticPreviewEntitlementDecision};


#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AssignmentLifecycleGate {
    Open,
    Denied(AssignmentLifecycleDenial),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AssignmentLifecycleDenial {
    NotPublished,
    Retired,
}

/// Maps persisted lifecycle intent to the first effective-policy gate.
pub fn assignment_lifecycle_gate(lifecycle: AssignmentLifecycle) -> AssignmentLifecycleGate {
    match lifecycle {
        AssignmentLifecycle::Published => AssignmentLifecycleGate::Open,
        AssignmentLifecycle::Draft => {
            AssignmentLifecycleGate::Denied(AssignmentLifecycleDenial::NotPublished)
        }
        AssignmentLifecycle::Closed | AssignmentLifecycle::Archived => {
            AssignmentLifecycleGate::Denied(AssignmentLifecycleDenial::Retired)
        }
    }
}

/// Returns whether a Instructor-controlled lifecycle transition is legal.
pub fn is_legal_assignment_lifecycle_transition(
    from: AssignmentLifecycle,
    to: AssignmentLifecycle,
) -> bool {
    from == to
        || matches!(
            (from, to),
            (
                AssignmentLifecycle::Draft,
                AssignmentLifecycle::Published | AssignmentLifecycle::Archived
            ) | (
                AssignmentLifecycle::Published,
                AssignmentLifecycle::Closed | AssignmentLifecycle::Archived
            ) | (
                AssignmentLifecycle::Closed,
                AssignmentLifecycle::Published | AssignmentLifecycle::Archived
            )
        )
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuthorizationGate {
    Authorized,
    Denied(AuthorizationDenial),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuthorizationDenial {
    ActionNotPermitted,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PolicyGate {
    Lifecycle,
    Entitlement,
    Authorization,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GateDenial {
    Lifecycle(AssignmentLifecycleDenial),
    Entitlement(EntitlementDenial),
    Authorization(AuthorizationDenial),
}

/// Origin of one resolved field.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PolicySource {
    Base,
    Accommodation(StudentRecordId),
    HypotheticalAccommodation,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedField<T> {
    pub value: T,
    pub source: PolicySource,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EffectiveAssignmentPolicy {
    pub available_at: ResolvedField<Option<ActivityTimestamp>>,
    pub due_at: ResolvedField<Option<ActivityTimestamp>>,
    pub closes_at: ResolvedField<Option<ActivityTimestamp>>,
    pub time_limit_seconds: ResolvedField<Option<NonZeroU32>>,
    pub attempt_limit: ResolvedField<Option<NonZeroU32>>,
    pub late_submission: ResolvedField<LateSubmissionPolicy>,
    pub deadline_behavior: ResolvedField<AssignmentDeadlineBehavior>,
}

/// A sparse direct-Student accommodation patch. Assignment-owned late and
/// deadline behavior remain Assignment policy.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PolicyPatchSet {
    pub available_at: PolicyPatch<ActivityTimestamp>,
    pub due_at: PolicyPatch<ActivityTimestamp>,
    pub closes_at: PolicyPatch<ActivityTimestamp>,
    pub time_limit_seconds: PolicyPatch<NonZeroU32>,
    pub attempt_limit: PolicyPatch<NonZeroU32>,
}

impl PolicyPatchSet {
    pub const INHERIT: Self = Self {
        available_at: PolicyPatch::Inherit,
        due_at: PolicyPatch::Inherit,
        closes_at: PolicyPatch::Inherit,
        time_limit_seconds: PolicyPatch::Inherit,
        attempt_limit: PolicyPatch::Inherit,
    };
}

/// A sparse patch distinguishes inheritance from removing an optional bound.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PolicyPatch<T> {
    Inherit,
    Set(T),
    Unrestricted,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PolicyModificationMode {
    ExtendOnly,
    Override,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Accommodation {
    pub student_record: StudentRecordId,
    pub mode: PolicyModificationMode,
    pub patch: PolicyPatchSet,
}

/// A preview-only individual policy modifier with no persisted Student key.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HypotheticalAccommodation {
    pub mode: PolicyModificationMode,
    pub patch: PolicyPatchSet,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LateVerdict {
    OnTime,
    AcceptedLate,
    MarkedLate,
    RejectedLate,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StartVerdict {
    MayStart { late: LateVerdict },
    NotYetAvailable,
    Closed,
    AttemptLimitReached,
    DueDateRejectsNewRun,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EffectivePolicyDecision {
    Denied {
        gate: PolicyGate,
        reason: GateDenial,
    },
    Allowed {
        policy: Box<EffectiveAssignmentPolicy>,
        start: StartVerdict,
    },
}

pub struct ResolveEffectivePolicyInput {
    pub lifecycle: AssignmentLifecycleGate,
    pub entitlement: EntitlementDecision,
    pub authorization: AuthorizationGate,
    pub now: ActivityTimestamp,
    pub prior_run_count: u32,
    pub base: BaseAssignmentPolicy,
    pub accommodation: Option<Accommodation>,
}

/// Identity-free S3 input for a synthetic T3 preview subject.
pub struct ResolveSyntheticPreviewPolicyInput {
    pub lifecycle: AssignmentLifecycleGate,
    pub entitlement: SyntheticPreviewEntitlementDecision,
    pub authorization: AuthorizationGate,
    pub now: ActivityTimestamp,
    pub prior_run_count: u32,
    pub base: BaseAssignmentPolicy,
    pub hypothetical_accommodation: Option<HypotheticalAccommodation>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PolicyField {
    AvailableAt,
    DueAt,
    ClosesAt,
    TimeLimitSeconds,
    AttemptLimit,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModifierSource {
    Accommodation(StudentRecordId),
    HypotheticalAccommodation,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EffectivePolicyError {
    BaseTimeLimitOutOfRange,
    BaseAttemptLimitOutOfRange,
    BaseTimestampOutsideCourseTerm(PolicyField),
    BaseTimestampOutOfRange(PolicyField),
    AccommodationStudentRecordMismatch {
        granted: StudentRecordId,
        modifier: StudentRecordId,
    },
    ExtendOnlyViolation {
        field: PolicyField,
        source: ModifierSource,
    },
    ScheduleOffsetOverflow,
    InvalidScheduleOrder,
}

/// Validates one persisted base policy independently of Student authority.
///
/// Store writes call this before opening a mutation. The resolver deliberately
/// keeps its gate-first behavior, so a denied Student request never exposes
/// policy-shape errors or causes modifier reads.
pub fn validate_base_assignment_policy(
    base: BaseAssignmentPolicy,
) -> Result<(), EffectivePolicyError> {
    if base
        .time_limit_seconds
        .is_some_and(|limit| limit.get() > MAX_ASSIGNMENT_TIME_LIMIT_SECONDS)
    {
        return Err(EffectivePolicyError::BaseTimeLimitOutOfRange);
    }
    if base
        .attempt_limit
        .is_some_and(|limit| limit.get() > MAX_ASSIGNMENT_ATTEMPT_LIMIT)
    {
        return Err(EffectivePolicyError::BaseAttemptLimitOutOfRange);
    }
    validate_schedule_values(base.available_at, base.due_at, base.closes_at)
}

/// Validates absolute persisted policy instants against the course-owned term.
///
/// The input is already an absolute server timestamp. This only projects that
/// instant into the course's authoritative zone to check its calendar date; it
/// neither accepts nor resolves Instructor-entered local wall-clock text.
pub fn validate_base_assignment_policy_for_course_term(
    base: BaseAssignmentPolicy,
    term: &CourseTerm,
) -> Result<(), EffectivePolicyError> {
    validate_base_assignment_policy(base)?;
    for (field, value) in [
        (PolicyField::AvailableAt, base.available_at),
        (PolicyField::DueAt, base.due_at),
        (PolicyField::ClosesAt, base.closes_at),
    ] {
        validate_absolute_timestamp_in_course_term(field, value, term)?;
    }
    Ok(())
}

fn validate_absolute_timestamp_in_course_term(
    field: PolicyField,
    value: Option<ActivityTimestamp>,
    term: &CourseTerm,
) -> Result<(), EffectivePolicyError> {
    let Some(value) = value else {
        return Ok(());
    };
    let utc = DateTime::<Utc>::from_timestamp_millis(value.as_unix_millis())
        .ok_or(EffectivePolicyError::BaseTimestampOutOfRange(field))?;
    let zone = term
        .time_zone()
        .as_str()
        .parse::<chrono_tz::Tz>()
        .expect("CourseTerm contains an exact known IANA zone");
    let date = utc.with_timezone(&zone).format("%Y-%m-%d").to_string();
    if date.as_str() < term.start_date().as_str() || date.as_str() > term.end_date().as_str() {
        return Err(EffectivePolicyError::BaseTimestampOutsideCourseTerm(field));
    }
    Ok(())
}

/// Resolves the complete policy after lifecycle, S5 entitlement, and action
/// authorization, in that exact order. A denied gate is returned before any
/// modifier is inspected.
pub fn resolve_effective_policy(
    input: ResolveEffectivePolicyInput,
) -> Result<EffectivePolicyDecision, EffectivePolicyError> {
    if let AssignmentLifecycleGate::Denied(reason) = input.lifecycle {
        return Ok(EffectivePolicyDecision::Denied {
            gate: PolicyGate::Lifecycle,
            reason: GateDenial::Lifecycle(reason),
        });
    }
    let grant = match input.entitlement {
        EntitlementDecision::Granted(grant) => grant,
        EntitlementDecision::Denied(reason) => {
            return Ok(EffectivePolicyDecision::Denied {
                gate: PolicyGate::Entitlement,
                reason: GateDenial::Entitlement(reason),
            });
        }
    };
    if let AuthorizationGate::Denied(reason) = input.authorization {
        return Ok(EffectivePolicyDecision::Denied {
            gate: PolicyGate::Authorization,
            reason: GateDenial::Authorization(reason),
        });
    }

    if let Some(individual) = input.accommodation
        && individual.student_record != grant.student_record()
    {
        return Err(EffectivePolicyError::AccommodationStudentRecordMismatch {
            granted: grant.student_record(),
            modifier: individual.student_record,
        });
    }
    resolve_authorized_policy(
        input.now,
        input.prior_run_count,
        input.base,
        input.accommodation.map(AccommodationPatch::Student),
    )
}

/// Resolves a synthetic preview policy after lifecycle, S5 synthetic
/// entitlement, and action authorization. A hypothetical modifier cannot carry
/// a persisted Student identifier or receipt authority.
pub fn resolve_synthetic_preview_policy(
    input: ResolveSyntheticPreviewPolicyInput,
) -> Result<EffectivePolicyDecision, EffectivePolicyError> {
    if let AssignmentLifecycleGate::Denied(reason) = input.lifecycle {
        return Ok(EffectivePolicyDecision::Denied {
            gate: PolicyGate::Lifecycle,
            reason: GateDenial::Lifecycle(reason),
        });
    }
    match input.entitlement {
        SyntheticPreviewEntitlementDecision::Granted(grant) => grant,
        SyntheticPreviewEntitlementDecision::Denied(reason) => {
            return Ok(EffectivePolicyDecision::Denied {
                gate: PolicyGate::Entitlement,
                reason: GateDenial::Entitlement(reason),
            });
        }
    };
    if let AuthorizationGate::Denied(reason) = input.authorization {
        return Ok(EffectivePolicyDecision::Denied {
            gate: PolicyGate::Authorization,
            reason: GateDenial::Authorization(reason),
        });
    }

    resolve_authorized_policy(
        input.now,
        input.prior_run_count,
        input.base,
        input
            .hypothetical_accommodation
            .map(AccommodationPatch::Hypothetical),
    )
}

fn resolve_authorized_policy(
    now: ActivityTimestamp,
    prior_run_count: u32,
    base: BaseAssignmentPolicy,
    accommodation: Option<AccommodationPatch>,
) -> Result<EffectivePolicyDecision, EffectivePolicyError> {
    let mut policy = base_policy(base);
    if let Some(accommodation) = accommodation {
        apply_accommodation_patch(&mut policy, accommodation)?;
    }
    validate_schedule(&policy)?;
    let start = start_verdict(&policy, now, prior_run_count);
    Ok(EffectivePolicyDecision::Allowed {
        policy: Box::new(policy),
        start,
    })
}

fn base_policy(base: BaseAssignmentPolicy) -> EffectiveAssignmentPolicy {
    EffectiveAssignmentPolicy {
        available_at: resolved(base.available_at),
        due_at: resolved(base.due_at),
        closes_at: resolved(base.closes_at),
        time_limit_seconds: resolved(base.time_limit_seconds),
        attempt_limit: resolved(base.attempt_limit),
        late_submission: resolved(base.late_submission),
        deadline_behavior: resolved(base.deadline_behavior),
    }
}

fn resolved<T>(value: T) -> ResolvedField<T> {
    ResolvedField {
        value,
        source: PolicySource::Base,
    }
}

#[derive(Clone, Copy)]
enum AccommodationPatch {
    Student(Accommodation),
    Hypothetical(HypotheticalAccommodation),
}

impl AccommodationPatch {
    fn mode(self) -> PolicyModificationMode {
        match self {
            Self::Student(value) => value.mode,
            Self::Hypothetical(value) => value.mode,
        }
    }

    fn patch(self) -> PolicyPatchSet {
        match self {
            Self::Student(value) => value.patch,
            Self::Hypothetical(value) => value.patch,
        }
    }

    fn source(self) -> ModifierSource {
        match self {
            Self::Student(value) => ModifierSource::Accommodation(value.student_record),
            Self::Hypothetical(_) => ModifierSource::HypotheticalAccommodation,
        }
    }
}

fn apply_accommodation_patch(
    policy: &mut EffectiveAssignmentPolicy,
    accommodation: AccommodationPatch,
) -> Result<(), EffectivePolicyError> {
    let source = accommodation.source();
    let patch = accommodation.patch();
    let mode = accommodation.mode();
    apply_accommodation_field(
        &mut policy.available_at,
        patch.available_at,
        mode,
        PolicyField::AvailableAt,
        OptionalRule::Earlier,
        source,
    )?;
    apply_accommodation_field(
        &mut policy.due_at,
        patch.due_at,
        mode,
        PolicyField::DueAt,
        OptionalRule::Later,
        source,
    )?;
    apply_accommodation_field(
        &mut policy.closes_at,
        patch.closes_at,
        mode,
        PolicyField::ClosesAt,
        OptionalRule::Later,
        source,
    )?;
    apply_accommodation_field(
        &mut policy.time_limit_seconds,
        patch.time_limit_seconds,
        mode,
        PolicyField::TimeLimitSeconds,
        OptionalRule::Later,
        source,
    )?;
    apply_accommodation_field(
        &mut policy.attempt_limit,
        patch.attempt_limit,
        mode,
        PolicyField::AttemptLimit,
        OptionalRule::Later,
        source,
    )?;
    Ok(())
}

#[derive(Clone, Copy)]
enum OptionalRule {
    Earlier,
    Later,
}

fn apply_accommodation_field<T: Ord + Copy>(
    field: &mut ResolvedField<Option<T>>,
    patch: PolicyPatch<T>,
    mode: PolicyModificationMode,
    policy_field: PolicyField,
    rule: OptionalRule,
    error_source: ModifierSource,
) -> Result<(), EffectivePolicyError> {
    if matches!(patch, PolicyPatch::Inherit) {
        return Ok(());
    }
    let replacement = match patch {
        PolicyPatch::Inherit => unreachable!("inherited accommodation fields return above"),
        PolicyPatch::Set(value) => Some(value),
        PolicyPatch::Unrestricted => None,
    };
    if mode == PolicyModificationMode::ExtendOnly
        && !extends_optional(field.value, replacement, rule)
    {
        return Err(EffectivePolicyError::ExtendOnlyViolation {
            field: policy_field,
            source: error_source,
        });
    }
    field.value = replacement;
    field.source = match error_source {
        ModifierSource::Accommodation(student) => PolicySource::Accommodation(student),
        ModifierSource::HypotheticalAccommodation => PolicySource::HypotheticalAccommodation,
    };
    Ok(())
}

fn extends_optional<T: Ord>(old: Option<T>, new: Option<T>, rule: OptionalRule) -> bool {
    match (old, new) {
        (_, None) => true,
        (None, Some(_)) => false,
        (Some(old), Some(new)) => match rule {
            OptionalRule::Earlier => new <= old,
            OptionalRule::Later => new >= old,
        },
    }
}

fn validate_schedule(policy: &EffectiveAssignmentPolicy) -> Result<(), EffectivePolicyError> {
    validate_schedule_values(
        policy.available_at.value,
        policy.due_at.value,
        policy.closes_at.value,
    )
}

fn validate_schedule_values(
    available: Option<ActivityTimestamp>,
    due: Option<ActivityTimestamp>,
    closes: Option<ActivityTimestamp>,
) -> Result<(), EffectivePolicyError> {
    if available.zip(due).is_some_and(|(a, d)| a > d)
        || available.zip(closes).is_some_and(|(a, c)| a > c)
        || due.zip(closes).is_some_and(|(d, c)| d > c)
    {
        return Err(EffectivePolicyError::InvalidScheduleOrder);
    }
    Ok(())
}

fn start_verdict(
    policy: &EffectiveAssignmentPolicy,
    now: ActivityTimestamp,
    prior_run_count: u32,
) -> StartVerdict {
    if policy.closes_at.value.is_some_and(|closes| now >= closes) {
        return StartVerdict::Closed;
    }
    if policy
        .available_at
        .value
        .is_some_and(|available| now < available)
    {
        return StartVerdict::NotYetAvailable;
    }
    if policy
        .attempt_limit
        .value
        .is_some_and(|limit| prior_run_count >= limit.get())
    {
        return StartVerdict::AttemptLimitReached;
    }
    let late = match policy.due_at.value {
        Some(due) if now > due => match policy.late_submission.value {
            LateSubmissionPolicy::Accept => LateVerdict::AcceptedLate,
            LateSubmissionPolicy::MarkLate => LateVerdict::MarkedLate,
            LateSubmissionPolicy::Reject => LateVerdict::RejectedLate,
        },
        _ => LateVerdict::OnTime,
    };
    match late {
        LateVerdict::RejectedLate => StartVerdict::DueDateRejectsNewRun,
        late => StartVerdict::MayStart { late },
    }
}

#[cfg(test)]
#[path = "effective_assignment_policy/tests.rs"]
mod tests;

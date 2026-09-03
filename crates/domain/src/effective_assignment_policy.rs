//! Pure resolution of a Student's current assignment policy.
//!
//! Active Student Course Membership is the sole authority that evaluates the active-membership prerequisite
//! and mints an [`ActiveStudentCourseMembershipGrant`]. This module consumes that grant: it validates supplied
//! modifier identifiers against the grant's opaque scopes, then resolves the
//! assignment window and limits without reading roster state or a clock.

use std::num::NonZeroU32;

use chrono::{DateTime, Utc};
use question_model::{
    AssignmentDeadlineRule, AssignmentStatus, BaseAssignmentPolicy, CourseTerm, LateWorkRule,
    MAX_ASSIGNMENT_ATTEMPT_LIMIT, MAX_ASSIGNMENT_ATTEMPT_TIME_LIMIT_SECONDS, StudentRecordId,
    Timestamp,
};

use crate::active_student_course_membership::{
    ActiveStudentCourseMembershipDecision, ActiveStudentCourseMembershipDenial,
    HypotheticalStudentViewScenarioAdmissionDecision,
    HypotheticalStudentViewScenarioAdmissionDenial,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AssignmentStatusGate {
    Open,
    Denied(AssignmentStatusDenial),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AssignmentStatusDenial {
    Unreleased,
    Retired,
}

/// Maps stable Assignment Status to the first effective-policy gate.
pub fn assignment_status_gate(status: AssignmentStatus) -> AssignmentStatusGate {
    match status {
        AssignmentStatus::Released => AssignmentStatusGate::Open,
        AssignmentStatus::Unreleased => {
            AssignmentStatusGate::Denied(AssignmentStatusDenial::Unreleased)
        }
        AssignmentStatus::Closed | AssignmentStatus::Archived => {
            AssignmentStatusGate::Denied(AssignmentStatusDenial::Retired)
        }
    }
}

/// Returns whether an Instructor-controlled Assignment Status transition is legal.
pub fn is_legal_assignment_status_transition(from: AssignmentStatus, to: AssignmentStatus) -> bool {
    from == to
        || matches!(
            (from, to),
            (
                AssignmentStatus::Unreleased,
                AssignmentStatus::Released | AssignmentStatus::Archived
            ) | (
                AssignmentStatus::Released,
                AssignmentStatus::Closed | AssignmentStatus::Archived
            ) | (
                AssignmentStatus::Closed,
                AssignmentStatus::Released | AssignmentStatus::Archived
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
    AssignmentStatus,
    ActiveStudentCourseMembership,
    Authorization,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GateDenial {
    AssignmentStatus(AssignmentStatusDenial),
    ActiveStudentCourseMembership(ActiveStudentCourseMembershipDenial),
    Authorization(AuthorizationDenial),
}

/// Assignment policy source for one resolved field.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AssignmentPolicySource {
    Base,
    Accommodation(StudentRecordId),
    HypotheticalStudentViewScenario,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EffectiveAssignmentPolicyValue<T> {
    pub value: T,
    pub source: AssignmentPolicySource,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EffectiveAssignmentPolicy {
    pub available_at: EffectiveAssignmentPolicyValue<Option<Timestamp>>,
    pub due_at: EffectiveAssignmentPolicyValue<Option<Timestamp>>,
    pub closes_at: EffectiveAssignmentPolicyValue<Option<Timestamp>>,
    pub assignment_attempt_time_limit_seconds: EffectiveAssignmentPolicyValue<Option<NonZeroU32>>,
    pub attempt_limit: EffectiveAssignmentPolicyValue<Option<NonZeroU32>>,
    pub late_work_rule: EffectiveAssignmentPolicyValue<LateWorkRule>,
    pub assignment_deadline_rule: EffectiveAssignmentPolicyValue<AssignmentDeadlineRule>,
}

/// A sparse direct-Student accommodation adjustment. Assignment-owned late and
/// deadline behavior remain Assignment policy.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AccommodationAdjustment {
    pub available_at: AccommodationAdjustmentValue<Timestamp>,
    pub due_at: AccommodationAdjustmentValue<Timestamp>,
    pub closes_at: AccommodationAdjustmentValue<Timestamp>,
    pub assignment_attempt_time_limit_seconds: AccommodationAdjustmentValue<NonZeroU32>,
    pub attempt_limit: AccommodationAdjustmentValue<NonZeroU32>,
}

impl AccommodationAdjustment {
    pub const INHERIT: Self = Self {
        available_at: AccommodationAdjustmentValue::Inherit,
        due_at: AccommodationAdjustmentValue::Inherit,
        closes_at: AccommodationAdjustmentValue::Inherit,
        assignment_attempt_time_limit_seconds: AccommodationAdjustmentValue::Inherit,
        attempt_limit: AccommodationAdjustmentValue::Inherit,
    };
}

/// A sparse adjustment distinguishes inheritance from removing an optional bound.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AccommodationAdjustmentValue<T> {
    Inherit,
    Set(T),
    Unrestricted,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AccommodationApplicationRule {
    ExtendOnly,
    Replace,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Accommodation {
    pub student_record: StudentRecordId,
    pub mode: AccommodationApplicationRule,
    pub adjustment: AccommodationAdjustment,
}

/// Identity-free policy modifiers for a Hypothetical Student View Scenario.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HypotheticalStudentViewScenarioModifiers {
    pub mode: AccommodationApplicationRule,
    pub adjustment: AccommodationAdjustment,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StudentLateWorkStatus {
    OnTime,
    AcceptedLate,
    MarkedLate,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AssignmentStartDecision {
    MayStart {
        late_work_status: StudentLateWorkStatus,
    },
    NotYetAvailable,
    Closed,
    AttemptLimitReached,
    LateWorkRefused,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AssignmentAccessDecision {
    Denied {
        gate: PolicyGate,
        reason: GateDenial,
    },
    Allowed {
        policy: Box<EffectiveAssignmentPolicy>,
        start_decision: AssignmentStartDecision,
    },
}

/// Scenario-specific policy gate. This cannot describe access by a Student Record.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HypotheticalStudentViewScenarioPolicyGate {
    AssignmentStatus,
    HypotheticalStudentViewScenarioAdmission,
    Authorization,
}

/// Scenario-specific denial reason. The admission branch carries only course and Assignment scope.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HypotheticalStudentViewScenarioPolicyDenial {
    AssignmentStatus(AssignmentStatusDenial),
    HypotheticalStudentViewScenarioAdmission(HypotheticalStudentViewScenarioAdmissionDenial),
    Authorization(AuthorizationDenial),
}

/// Closed policy result for an identity-free Hypothetical Student View Scenario.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HypotheticalStudentViewScenarioPolicyDecision {
    Denied {
        gate: HypotheticalStudentViewScenarioPolicyGate,
        reason: HypotheticalStudentViewScenarioPolicyDenial,
    },
    Allowed {
        policy: Box<EffectiveAssignmentPolicy>,
        start_decision: AssignmentStartDecision,
    },
}

pub struct ResolveEffectivePolicyInput {
    pub assignment_status: AssignmentStatusGate,
    pub active_student_course_membership: ActiveStudentCourseMembershipDecision,
    pub authorization: AuthorizationGate,
    pub now: Timestamp,
    pub prior_assignment_attempt_count: u32,
    pub base: BaseAssignmentPolicy,
    pub accommodation: Option<Accommodation>,
}

/// Identity-free Hypothetical Student View Scenario policy-resolution input.
pub struct ResolveHypotheticalStudentViewScenarioPolicyInput {
    pub assignment_status: AssignmentStatusGate,
    pub hypothetical_student_view_scenario_admission:
        HypotheticalStudentViewScenarioAdmissionDecision,
    pub authorization: AuthorizationGate,
    pub now: Timestamp,
    pub prior_assignment_attempt_count: u32,
    pub base: BaseAssignmentPolicy,
    pub hypothetical_student_view_scenario_modifiers:
        Option<HypotheticalStudentViewScenarioModifiers>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PolicyField {
    AvailableAt,
    DueAt,
    ClosesAt,
    AssignmentAttemptTimeLimitSeconds,
    AttemptLimit,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModifierSource {
    Accommodation(StudentRecordId),
    HypotheticalStudentViewScenario,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EffectivePolicyError {
    BaseAssignmentAttemptAssignmentAttemptTimeLimitOutOfRange,
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
        .assignment_attempt_time_limit_seconds
        .is_some_and(|limit| limit.get() > MAX_ASSIGNMENT_ATTEMPT_TIME_LIMIT_SECONDS)
    {
        return Err(
            EffectivePolicyError::BaseAssignmentAttemptAssignmentAttemptTimeLimitOutOfRange,
        );
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
    value: Option<Timestamp>,
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

/// Resolves the complete policy after Assignment Status, Active Student Course Membership, and action
/// authorization, in that exact order. A denied gate is returned before any
/// modifier is inspected.
pub fn resolve_effective_policy(
    input: ResolveEffectivePolicyInput,
) -> Result<AssignmentAccessDecision, EffectivePolicyError> {
    if let AssignmentStatusGate::Denied(reason) = input.assignment_status {
        return Ok(AssignmentAccessDecision::Denied {
            gate: PolicyGate::AssignmentStatus,
            reason: GateDenial::AssignmentStatus(reason),
        });
    }
    let grant = match input.active_student_course_membership {
        ActiveStudentCourseMembershipDecision::Granted(grant) => grant,
        ActiveStudentCourseMembershipDecision::Denied(reason) => {
            return Ok(AssignmentAccessDecision::Denied {
                gate: PolicyGate::ActiveStudentCourseMembership,
                reason: GateDenial::ActiveStudentCourseMembership(reason),
            });
        }
    };
    if let AuthorizationGate::Denied(reason) = input.authorization {
        return Ok(AssignmentAccessDecision::Denied {
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
        input.prior_assignment_attempt_count,
        input.base,
        input
            .accommodation
            .map(AccommodationAdjustmentInput::Student),
    )
}

/// Resolves a Hypothetical Student View Scenario policy after Assignment Status,
/// scenario admission, and action authorization. Scenario modifiers cannot carry a persisted
/// Student identifier or receipt authority.
pub fn resolve_hypothetical_student_view_scenario_policy(
    input: ResolveHypotheticalStudentViewScenarioPolicyInput,
) -> Result<HypotheticalStudentViewScenarioPolicyDecision, EffectivePolicyError> {
    if let AssignmentStatusGate::Denied(reason) = input.assignment_status {
        return Ok(HypotheticalStudentViewScenarioPolicyDecision::Denied {
            gate: HypotheticalStudentViewScenarioPolicyGate::AssignmentStatus,
            reason: HypotheticalStudentViewScenarioPolicyDenial::AssignmentStatus(reason),
        });
    }
    match input.hypothetical_student_view_scenario_admission {
        HypotheticalStudentViewScenarioAdmissionDecision::Granted(grant) => grant,
        HypotheticalStudentViewScenarioAdmissionDecision::Denied(reason) => {
            return Ok(HypotheticalStudentViewScenarioPolicyDecision::Denied {
                gate: HypotheticalStudentViewScenarioPolicyGate::HypotheticalStudentViewScenarioAdmission,
                reason: HypotheticalStudentViewScenarioPolicyDenial::HypotheticalStudentViewScenarioAdmission(reason),
            });
        }
    };
    if let AuthorizationGate::Denied(reason) = input.authorization {
        return Ok(HypotheticalStudentViewScenarioPolicyDecision::Denied {
            gate: HypotheticalStudentViewScenarioPolicyGate::Authorization,
            reason: HypotheticalStudentViewScenarioPolicyDenial::Authorization(reason),
        });
    }
    let (policy, start_decision) = resolve_authorized_policy_values(
        input.now,
        input.prior_assignment_attempt_count,
        input.base,
        input
            .hypothetical_student_view_scenario_modifiers
            .map(AccommodationAdjustmentInput::HypotheticalStudentViewScenario),
    )?;
    Ok(HypotheticalStudentViewScenarioPolicyDecision::Allowed {
        policy: Box::new(policy),
        start_decision,
    })
}

fn resolve_authorized_policy(
    now: Timestamp,
    prior_assignment_attempt_count: u32,
    base: BaseAssignmentPolicy,
    accommodation: Option<AccommodationAdjustmentInput>,
) -> Result<AssignmentAccessDecision, EffectivePolicyError> {
    let (policy, start_decision) =
        resolve_authorized_policy_values(now, prior_assignment_attempt_count, base, accommodation)?;
    Ok(AssignmentAccessDecision::Allowed {
        policy: Box::new(policy),
        start_decision,
    })
}

fn resolve_authorized_policy_values(
    now: Timestamp,
    prior_assignment_attempt_count: u32,
    base: BaseAssignmentPolicy,
    accommodation: Option<AccommodationAdjustmentInput>,
) -> Result<(EffectiveAssignmentPolicy, AssignmentStartDecision), EffectivePolicyError> {
    let mut policy = base_policy(base);
    if let Some(accommodation) = accommodation {
        apply_accommodation_adjustment(&mut policy, accommodation)?;
    }
    validate_schedule(&policy)?;
    let start_decision = assignment_start_decision(&policy, now, prior_assignment_attempt_count);
    Ok((policy, start_decision))
}

fn base_policy(base: BaseAssignmentPolicy) -> EffectiveAssignmentPolicy {
    EffectiveAssignmentPolicy {
        available_at: resolved(base.available_at),
        due_at: resolved(base.due_at),
        closes_at: resolved(base.closes_at),
        assignment_attempt_time_limit_seconds: resolved(base.assignment_attempt_time_limit_seconds),
        attempt_limit: resolved(base.attempt_limit),
        late_work_rule: resolved(base.late_work_rule),
        assignment_deadline_rule: resolved(base.assignment_deadline_rule),
    }
}

fn resolved<T>(value: T) -> EffectiveAssignmentPolicyValue<T> {
    EffectiveAssignmentPolicyValue {
        value,
        source: AssignmentPolicySource::Base,
    }
}

#[derive(Clone, Copy)]
enum AccommodationAdjustmentInput {
    Student(Accommodation),
    HypotheticalStudentViewScenario(HypotheticalStudentViewScenarioModifiers),
}

impl AccommodationAdjustmentInput {
    fn mode(self) -> AccommodationApplicationRule {
        match self {
            Self::Student(value) => value.mode,
            Self::HypotheticalStudentViewScenario(value) => value.mode,
        }
    }

    fn adjustment(self) -> AccommodationAdjustment {
        match self {
            Self::Student(value) => value.adjustment,
            Self::HypotheticalStudentViewScenario(value) => value.adjustment,
        }
    }

    fn source(self) -> ModifierSource {
        match self {
            Self::Student(value) => ModifierSource::Accommodation(value.student_record),
            Self::HypotheticalStudentViewScenario(_) => {
                ModifierSource::HypotheticalStudentViewScenario
            }
        }
    }
}

fn apply_accommodation_adjustment(
    policy: &mut EffectiveAssignmentPolicy,
    accommodation: AccommodationAdjustmentInput,
) -> Result<(), EffectivePolicyError> {
    let source = accommodation.source();
    let adjustment = accommodation.adjustment();
    let mode = accommodation.mode();
    apply_accommodation_field(
        &mut policy.available_at,
        adjustment.available_at,
        mode,
        PolicyField::AvailableAt,
        OptionalRule::Earlier,
        source,
    )?;
    apply_accommodation_field(
        &mut policy.due_at,
        adjustment.due_at,
        mode,
        PolicyField::DueAt,
        OptionalRule::Later,
        source,
    )?;
    apply_accommodation_field(
        &mut policy.closes_at,
        adjustment.closes_at,
        mode,
        PolicyField::ClosesAt,
        OptionalRule::Later,
        source,
    )?;
    apply_accommodation_field(
        &mut policy.assignment_attempt_time_limit_seconds,
        adjustment.assignment_attempt_time_limit_seconds,
        mode,
        PolicyField::AssignmentAttemptTimeLimitSeconds,
        OptionalRule::Later,
        source,
    )?;
    apply_accommodation_field(
        &mut policy.attempt_limit,
        adjustment.attempt_limit,
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
    field: &mut EffectiveAssignmentPolicyValue<Option<T>>,
    adjustment: AccommodationAdjustmentValue<T>,
    mode: AccommodationApplicationRule,
    policy_field: PolicyField,
    rule: OptionalRule,
    error_source: ModifierSource,
) -> Result<(), EffectivePolicyError> {
    if matches!(adjustment, AccommodationAdjustmentValue::Inherit) {
        return Ok(());
    }
    let replacement = match adjustment {
        AccommodationAdjustmentValue::Inherit => {
            unreachable!("inherited accommodation fields return above")
        }
        AccommodationAdjustmentValue::Set(value) => Some(value),
        AccommodationAdjustmentValue::Unrestricted => None,
    };
    if mode == AccommodationApplicationRule::ExtendOnly
        && !extends_optional(field.value, replacement, rule)
    {
        return Err(EffectivePolicyError::ExtendOnlyViolation {
            field: policy_field,
            source: error_source,
        });
    }
    field.value = replacement;
    field.source = match error_source {
        ModifierSource::Accommodation(student) => AssignmentPolicySource::Accommodation(student),
        ModifierSource::HypotheticalStudentViewScenario => {
            AssignmentPolicySource::HypotheticalStudentViewScenario
        }
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
    available: Option<Timestamp>,
    due: Option<Timestamp>,
    closes: Option<Timestamp>,
) -> Result<(), EffectivePolicyError> {
    if available.zip(due).is_some_and(|(a, d)| a > d)
        || available.zip(closes).is_some_and(|(a, c)| a > c)
        || due.zip(closes).is_some_and(|(d, c)| d > c)
    {
        return Err(EffectivePolicyError::InvalidScheduleOrder);
    }
    Ok(())
}

fn assignment_start_decision(
    policy: &EffectiveAssignmentPolicy,
    now: Timestamp,
    prior_assignment_attempt_count: u32,
) -> AssignmentStartDecision {
    if policy.closes_at.value.is_some_and(|closes| now >= closes) {
        return AssignmentStartDecision::Closed;
    }
    if policy
        .available_at
        .value
        .is_some_and(|available| now < available)
    {
        return AssignmentStartDecision::NotYetAvailable;
    }
    if policy
        .attempt_limit
        .value
        .is_some_and(|limit| prior_assignment_attempt_count >= limit.get())
    {
        return AssignmentStartDecision::AttemptLimitReached;
    }
    let late_work_status = match policy.due_at.value {
        Some(due) if now > due => match policy.late_work_rule.value {
            LateWorkRule::Accept => StudentLateWorkStatus::AcceptedLate,
            LateWorkRule::MarkLate => StudentLateWorkStatus::MarkedLate,
            LateWorkRule::Reject => return AssignmentStartDecision::LateWorkRefused,
        },
        _ => StudentLateWorkStatus::OnTime,
    };
    AssignmentStartDecision::MayStart { late_work_status }
}

#[cfg(test)]
#[path = "effective_assignment_policy/tests.rs"]
mod tests;

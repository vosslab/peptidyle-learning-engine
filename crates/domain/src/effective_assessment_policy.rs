//! Pure resolution of a Student's current assessment policy.
//!
//! Active Student Course Membership is the sole authority that evaluates the active-membership prerequisite
//! and mints an [`ActiveStudentCourseMembershipGrant`]. This module consumes that grant: it validates supplied
//! modifier identifiers against the grant's opaque scopes, then resolves the
//! assessment window and limits without reading roster state or a clock.

use std::num::NonZeroU32;

pub use question_model::StudentLateWorkStatus;
use question_model::{
    AssessmentStatus, BaseAssessmentPolicy, LateWorkRule, MAX_ASSESSMENT_ATTEMPT_LIMIT,
    MAX_ASSESSMENT_ATTEMPT_TIME_LIMIT_SECONDS, StudentRecordId, Timestamp,
};

use crate::active_student_course_membership::{
    ActiveStudentCourseMembershipDecision, ActiveStudentCourseMembershipDenial,
    HypotheticalStudentViewScenarioAdmissionDecision,
    HypotheticalStudentViewScenarioAdmissionDenial,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AssessmentStatusGate {
    Open,
    Denied(AssessmentStatusDenial),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AssessmentStatusDenial {
    Unreleased,
    Retired,
}

/// Maps stable Assessment Status to the first effective-policy gate.
pub fn assessment_status_gate(status: AssessmentStatus) -> AssessmentStatusGate {
    match status {
        AssessmentStatus::Released => AssessmentStatusGate::Open,
        AssessmentStatus::Unreleased => {
            AssessmentStatusGate::Denied(AssessmentStatusDenial::Unreleased)
        }
        AssessmentStatus::Closed | AssessmentStatus::Archived => {
            AssessmentStatusGate::Denied(AssessmentStatusDenial::Retired)
        }
    }
}

/// Returns whether an Instructor-controlled Assessment Status transition is legal.
pub fn is_legal_assessment_status_transition(from: AssessmentStatus, to: AssessmentStatus) -> bool {
    from == to
        || matches!(
            (from, to),
            (
                AssessmentStatus::Unreleased,
                AssessmentStatus::Released | AssessmentStatus::Archived
            ) | (
                AssessmentStatus::Released,
                AssessmentStatus::Unreleased
                    | AssessmentStatus::Closed
                    | AssessmentStatus::Archived
            ) | (
                AssessmentStatus::Closed,
                AssessmentStatus::Released | AssessmentStatus::Archived
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
    AssessmentStatus,
    ActiveStudentCourseMembership,
    Authorization,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GateDenial {
    AssessmentStatus(AssessmentStatusDenial),
    ActiveStudentCourseMembership(ActiveStudentCourseMembershipDenial),
    Authorization(AuthorizationDenial),
}

/// Assessment policy source for one resolved field.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AssessmentPolicySource {
    Base,
    Accommodation(StudentRecordId),
    HypotheticalStudentViewScenario,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EffectiveAssessmentPolicyValue<T> {
    pub value: T,
    pub source: AssessmentPolicySource,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EffectiveAssessmentPolicy {
    pub available_at: EffectiveAssessmentPolicyValue<Option<Timestamp>>,
    pub due_at: EffectiveAssessmentPolicyValue<Option<Timestamp>>,
    pub closes_at: EffectiveAssessmentPolicyValue<Option<Timestamp>>,
    pub assessment_attempt_time_limit_seconds: EffectiveAssessmentPolicyValue<Option<NonZeroU32>>,
    pub attempt_limit: EffectiveAssessmentPolicyValue<Option<NonZeroU32>>,
    pub late_work_rule: EffectiveAssessmentPolicyValue<LateWorkRule>,
}

/// A sparse direct-Student accommodation adjustment. Assessment-owned late and
/// deadline behavior remain Assessment policy.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AccommodationAdjustment {
    pub available_at: AccommodationAdjustmentValue<Timestamp>,
    pub due_at: AccommodationAdjustmentValue<Timestamp>,
    pub closes_at: AccommodationAdjustmentValue<Timestamp>,
    pub assessment_attempt_time_limit_seconds: AccommodationAdjustmentValue<NonZeroU32>,
    pub attempt_limit: AccommodationAdjustmentValue<NonZeroU32>,
}

impl AccommodationAdjustment {
    pub const INHERIT: Self = Self {
        available_at: AccommodationAdjustmentValue::Inherit,
        due_at: AccommodationAdjustmentValue::Inherit,
        closes_at: AccommodationAdjustmentValue::Inherit,
        assessment_attempt_time_limit_seconds: AccommodationAdjustmentValue::Inherit,
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
pub enum AssessmentStartDecision {
    MayStart {
        student_late_work_status: StudentLateWorkStatus,
    },
    NotYetAvailable,
    Closed,
    AttemptLimitReached,
    LateWorkRefused,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AssessmentAccessDecision {
    Denied {
        gate: PolicyGate,
        reason: GateDenial,
    },
    Allowed {
        policy: Box<EffectiveAssessmentPolicy>,
        start_decision: AssessmentStartDecision,
    },
}

/// Scenario-specific policy gate. This cannot describe access by a Student Record.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HypotheticalStudentViewScenarioPolicyGate {
    AssessmentStatus,
    HypotheticalStudentViewScenarioAdmission,
    Authorization,
}

/// Scenario-specific denial reason. The admission branch carries only course and Assessment scope.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HypotheticalStudentViewScenarioPolicyDenial {
    AssessmentStatus(AssessmentStatusDenial),
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
        policy: Box<EffectiveAssessmentPolicy>,
        start_decision: AssessmentStartDecision,
    },
}

pub struct ResolveEffectivePolicyInput {
    pub assessment_status: AssessmentStatusGate,
    pub active_student_course_membership: ActiveStudentCourseMembershipDecision,
    pub authorization: AuthorizationGate,
    pub now: Timestamp,
    pub prior_assessment_attempt_count: u32,
    pub base: BaseAssessmentPolicy,
    pub accommodation: Option<Accommodation>,
}

/// Identity-free Hypothetical Student View Scenario policy-resolution input.
pub struct ResolveHypotheticalStudentViewScenarioPolicyInput {
    pub assessment_status: AssessmentStatusGate,
    pub hypothetical_student_view_scenario_admission:
        HypotheticalStudentViewScenarioAdmissionDecision,
    pub authorization: AuthorizationGate,
    pub now: Timestamp,
    pub prior_assessment_attempt_count: u32,
    pub base: BaseAssessmentPolicy,
    pub hypothetical_student_view_scenario_modifiers:
        Option<HypotheticalStudentViewScenarioModifiers>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PolicyField {
    AvailableAt,
    DueAt,
    ClosesAt,
    AssessmentAttemptTimeLimitSeconds,
    AttemptLimit,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModifierSource {
    Accommodation(StudentRecordId),
    HypotheticalStudentViewScenario,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EffectivePolicyError {
    BaseAssessmentAttemptAssessmentAttemptTimeLimitOutOfRange,
    BaseAttemptLimitOutOfRange,
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
pub fn validate_base_assessment_policy(
    base: BaseAssessmentPolicy,
) -> Result<(), EffectivePolicyError> {
    if base
        .assessment_attempt_time_limit_seconds
        .is_some_and(|limit| limit.get() > MAX_ASSESSMENT_ATTEMPT_TIME_LIMIT_SECONDS)
    {
        return Err(
            EffectivePolicyError::BaseAssessmentAttemptAssessmentAttemptTimeLimitOutOfRange,
        );
    }
    if base
        .attempt_limit
        .is_some_and(|limit| limit.get() > MAX_ASSESSMENT_ATTEMPT_LIMIT)
    {
        return Err(EffectivePolicyError::BaseAttemptLimitOutOfRange);
    }
    validate_schedule_values(base.available_at, base.due_at, base.closes_at)
}

/// Resolves the complete policy after Assessment Status, Active Student Course Membership, and action
/// authorization, in that exact order. A denied gate is returned before any
/// modifier is inspected.
pub fn resolve_effective_policy(
    input: ResolveEffectivePolicyInput,
) -> Result<AssessmentAccessDecision, EffectivePolicyError> {
    if let AssessmentStatusGate::Denied(reason) = input.assessment_status {
        return Ok(AssessmentAccessDecision::Denied {
            gate: PolicyGate::AssessmentStatus,
            reason: GateDenial::AssessmentStatus(reason),
        });
    }
    let grant = match input.active_student_course_membership {
        ActiveStudentCourseMembershipDecision::Granted(grant) => grant,
        ActiveStudentCourseMembershipDecision::Denied(reason) => {
            return Ok(AssessmentAccessDecision::Denied {
                gate: PolicyGate::ActiveStudentCourseMembership,
                reason: GateDenial::ActiveStudentCourseMembership(reason),
            });
        }
    };
    if let AuthorizationGate::Denied(reason) = input.authorization {
        return Ok(AssessmentAccessDecision::Denied {
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
        input.prior_assessment_attempt_count,
        input.base,
        input
            .accommodation
            .map(AccommodationAdjustmentInput::Student),
    )
}

/// Resolves a Hypothetical Student View Scenario policy after Assessment Status,
/// scenario admission, and action authorization. Scenario modifiers cannot carry a persisted
/// Student identifier or receipt authority.
pub fn resolve_hypothetical_student_view_scenario_policy(
    input: ResolveHypotheticalStudentViewScenarioPolicyInput,
) -> Result<HypotheticalStudentViewScenarioPolicyDecision, EffectivePolicyError> {
    if let AssessmentStatusGate::Denied(reason) = input.assessment_status {
        return Ok(HypotheticalStudentViewScenarioPolicyDecision::Denied {
            gate: HypotheticalStudentViewScenarioPolicyGate::AssessmentStatus,
            reason: HypotheticalStudentViewScenarioPolicyDenial::AssessmentStatus(reason),
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
        input.prior_assessment_attempt_count,
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
    prior_assessment_attempt_count: u32,
    base: BaseAssessmentPolicy,
    accommodation: Option<AccommodationAdjustmentInput>,
) -> Result<AssessmentAccessDecision, EffectivePolicyError> {
    let (policy, start_decision) =
        resolve_authorized_policy_values(now, prior_assessment_attempt_count, base, accommodation)?;
    Ok(AssessmentAccessDecision::Allowed {
        policy: Box::new(policy),
        start_decision,
    })
}

fn resolve_authorized_policy_values(
    now: Timestamp,
    prior_assessment_attempt_count: u32,
    base: BaseAssessmentPolicy,
    accommodation: Option<AccommodationAdjustmentInput>,
) -> Result<(EffectiveAssessmentPolicy, AssessmentStartDecision), EffectivePolicyError> {
    let mut policy = base_policy(base);
    if let Some(accommodation) = accommodation {
        apply_accommodation_adjustment(&mut policy, accommodation)?;
    }
    validate_schedule(&policy)?;
    let start_decision = assessment_start_decision(&policy, now, prior_assessment_attempt_count);
    Ok((policy, start_decision))
}

fn base_policy(base: BaseAssessmentPolicy) -> EffectiveAssessmentPolicy {
    EffectiveAssessmentPolicy {
        available_at: resolved(base.available_at),
        due_at: resolved(base.due_at),
        closes_at: resolved(base.closes_at),
        assessment_attempt_time_limit_seconds: resolved(base.assessment_attempt_time_limit_seconds),
        attempt_limit: resolved(base.attempt_limit),
        late_work_rule: resolved(base.late_work_rule),
    }
}

fn resolved<T>(value: T) -> EffectiveAssessmentPolicyValue<T> {
    EffectiveAssessmentPolicyValue {
        value,
        source: AssessmentPolicySource::Base,
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
    policy: &mut EffectiveAssessmentPolicy,
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
        &mut policy.assessment_attempt_time_limit_seconds,
        adjustment.assessment_attempt_time_limit_seconds,
        mode,
        PolicyField::AssessmentAttemptTimeLimitSeconds,
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
    field: &mut EffectiveAssessmentPolicyValue<Option<T>>,
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
        ModifierSource::Accommodation(student) => AssessmentPolicySource::Accommodation(student),
        ModifierSource::HypotheticalStudentViewScenario => {
            AssessmentPolicySource::HypotheticalStudentViewScenario
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

fn validate_schedule(policy: &EffectiveAssessmentPolicy) -> Result<(), EffectivePolicyError> {
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

fn assessment_start_decision(
    policy: &EffectiveAssessmentPolicy,
    now: Timestamp,
    prior_assessment_attempt_count: u32,
) -> AssessmentStartDecision {
    // ASVS 2.1.2, 2.2.3, 8.1.3: keep the documented close, availability,
    // completed-Attempt-limit, and late-work decision order explicit.
    if policy.closes_at.value.is_some_and(|closes| now >= closes) {
        return AssessmentStartDecision::Closed;
    }
    if policy
        .available_at
        .value
        .is_some_and(|available| now < available)
    {
        return AssessmentStartDecision::NotYetAvailable;
    }
    if policy
        .attempt_limit
        .value
        .is_some_and(|limit| prior_assessment_attempt_count >= limit.get())
    {
        return AssessmentStartDecision::AttemptLimitReached;
    }
    let student_late_work_status = match policy.due_at.value {
        Some(due) if now > due => match policy.late_work_rule.value {
            LateWorkRule::Accept => StudentLateWorkStatus::AcceptedLate,
            LateWorkRule::MarkLate => StudentLateWorkStatus::MarkedLate,
            LateWorkRule::Reject => return AssessmentStartDecision::LateWorkRefused,
        },
        _ => StudentLateWorkStatus::OnTime,
    };
    AssessmentStartDecision::MayStart {
        student_late_work_status,
    }
}

#[cfg(test)]
#[path = "effective_assessment_policy/tests.rs"]
mod tests;

//! Pure resolution of a learner's current assignment policy.
//!
//! S5 is the sole authority that evaluates membership and mints an
//! [`EntitlementGrant`]. This module consumes that grant: it validates supplied
//! modifier identifiers against the grant's opaque scopes, then resolves the
//! assignment window and limits without reading roster state or a clock.

use std::num::NonZeroU32;

use question_model::{
    ActivityTimestamp, AssignmentDeadlineBehavior, CourseGroupId, GroupPurposeCapabilities,
    LateSubmissionPolicy, StudentId,
};

use crate::entitlement::{EntitlementDecision, EntitlementDenial, EntitlementGrant};

const MAX_SCHEDULE_OFFSET_SECONDS: i32 = 31_536_000;

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

/// Origin of one resolved field. Schedule offsets are additive; equally
/// permissive M3 rows keep every winning accommodation source in ID order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PolicySource {
    Base,
    GroupScheduleOffsets(Vec<CourseGroupId>),
    GroupAccommodations(Vec<CourseGroupId>),
    IndividualException(StudentId),
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BaseAssignmentPolicy {
    pub available_at: Option<ActivityTimestamp>,
    pub due_at: Option<ActivityTimestamp>,
    pub closes_at: Option<ActivityTimestamp>,
    pub time_limit_seconds: Option<NonZeroU32>,
    pub attempt_limit: Option<NonZeroU32>,
    pub late_submission: LateSubmissionPolicy,
    pub deadline_behavior: AssignmentDeadlineBehavior,
}

/// A sparse M3/M4 patch. Assignment-owned late and deadline behavior are not
/// writable through group accommodations or individual exceptions.
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

/// Validated representation of the normalized signed, non-zero seconds
/// column. Conversion to milliseconds happens only while applying a window.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ScheduleOffsetSeconds(i32);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScheduleOffsetSecondsError {
    Zero,
    OutOfRange,
}

impl ScheduleOffsetSeconds {
    pub fn try_new(seconds: i32) -> Result<Self, ScheduleOffsetSecondsError> {
        if seconds == 0 {
            return Err(ScheduleOffsetSecondsError::Zero);
        }
        if !(-MAX_SCHEDULE_OFFSET_SECONDS..=MAX_SCHEDULE_OFFSET_SECONDS).contains(&seconds) {
            return Err(ScheduleOffsetSecondsError::OutOfRange);
        }
        Ok(Self(seconds))
    }

    pub fn get(self) -> i32 {
        self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GroupScheduleOffset {
    pub group: CourseGroupId,
    pub offset_seconds: ScheduleOffsetSeconds,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GroupAccommodation {
    pub group: CourseGroupId,
    pub mode: PolicyModificationMode,
    pub patch: PolicyPatchSet,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IndividualPolicyException {
    pub student: StudentId,
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
    pub group_schedule_offsets: Vec<GroupScheduleOffset>,
    pub group_accommodations: Vec<GroupAccommodation>,
    pub individual_exception: Option<IndividualPolicyException>,
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
    Group(CourseGroupId),
    Individual(StudentId),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EffectivePolicyError {
    UnapprovedScheduleScope(CourseGroupId),
    UnapprovedAccommodationScope(CourseGroupId),
    IndividualExceptionStudentMismatch {
        granted: StudentId,
        modifier: StudentId,
    },
    DuplicateScheduleOffset(CourseGroupId),
    MultipleAccommodationOverrides {
        field: PolicyField,
        sources: Vec<CourseGroupId>,
    },
    ExtendOnlyViolation {
        field: PolicyField,
        source: ModifierSource,
    },
    ScheduleOffsetOverflow,
    InvalidScheduleOrder,
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

    validate_modifier_authority(
        &grant,
        &input.group_schedule_offsets,
        &input.group_accommodations,
        input.individual_exception,
    )?;
    let mut policy = base_policy(input.base);
    apply_schedule_offsets(&mut policy, &input.group_schedule_offsets)?;
    apply_accommodations(&mut policy, &input.group_accommodations)?;
    if let Some(individual) = input.individual_exception {
        apply_individual_patch(&mut policy, individual)?;
    }
    validate_schedule(&policy)?;
    let start = start_verdict(&policy, input.now, input.prior_run_count);
    Ok(EffectivePolicyDecision::Allowed {
        policy: Box::new(policy),
        start,
    })
}

fn validate_modifier_authority(
    grant: &EntitlementGrant,
    schedule_offsets: &[GroupScheduleOffset],
    accommodations: &[GroupAccommodation],
    individual_exception: Option<IndividualPolicyException>,
) -> Result<(), EffectivePolicyError> {
    for offset in schedule_offsets {
        if !grant_scope_allows(grant, offset.group, |capabilities| {
            capabilities.schedule_scope
        }) {
            return Err(EffectivePolicyError::UnapprovedScheduleScope(offset.group));
        }
    }
    for accommodation in accommodations {
        if !grant_scope_allows(grant, accommodation.group, |capabilities| {
            capabilities.accommodation_scope
        }) {
            return Err(EffectivePolicyError::UnapprovedAccommodationScope(
                accommodation.group,
            ));
        }
    }
    if let Some(individual) = individual_exception
        && individual.student != grant.student()
    {
        return Err(EffectivePolicyError::IndividualExceptionStudentMismatch {
            granted: grant.student(),
            modifier: individual.student,
        });
    }
    Ok(())
}

fn grant_scope_allows(
    grant: &EntitlementGrant,
    group: CourseGroupId,
    permits: impl Fn(GroupPurposeCapabilities) -> bool,
) -> bool {
    grant
        .applicable_policy_scopes()
        .iter()
        .any(|(scope, purpose)| {
            *scope == group && permits(GroupPurposeCapabilities::for_purpose(*purpose))
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

fn apply_schedule_offsets(
    policy: &mut EffectiveAssignmentPolicy,
    offsets: &[GroupScheduleOffset],
) -> Result<(), EffectivePolicyError> {
    let mut groups = Vec::with_capacity(offsets.len());
    let mut total_seconds = 0_i64;
    for offset in offsets {
        if groups.contains(&offset.group) {
            return Err(EffectivePolicyError::DuplicateScheduleOffset(offset.group));
        }
        groups.push(offset.group);
        total_seconds = total_seconds
            .checked_add(i64::from(offset.offset_seconds.get()))
            .ok_or(EffectivePolicyError::ScheduleOffsetOverflow)?;
    }
    if groups.is_empty() {
        return Ok(());
    }
    let total_milliseconds = total_seconds
        .checked_mul(1_000)
        .ok_or(EffectivePolicyError::ScheduleOffsetOverflow)?;
    groups.sort_unstable();
    let source = PolicySource::GroupScheduleOffsets(groups);
    offset_timestamp(&mut policy.available_at, total_milliseconds, source.clone())?;
    offset_timestamp(&mut policy.due_at, total_milliseconds, source.clone())?;
    offset_timestamp(&mut policy.closes_at, total_milliseconds, source)?;
    Ok(())
}

fn offset_timestamp(
    field: &mut ResolvedField<Option<ActivityTimestamp>>,
    offset_milliseconds: i64,
    source: PolicySource,
) -> Result<(), EffectivePolicyError> {
    let Some(value) = field.value else {
        return Ok(());
    };
    let value = value
        .as_unix_millis()
        .checked_add(offset_milliseconds)
        .ok_or(EffectivePolicyError::ScheduleOffsetOverflow)?;
    field.value = Some(ActivityTimestamp::from_unix_millis(value));
    field.source = source;
    Ok(())
}

fn apply_accommodations(
    policy: &mut EffectiveAssignmentPolicy,
    accommodations: &[GroupAccommodation],
) -> Result<(), EffectivePolicyError> {
    apply_accommodation_field(
        &mut policy.available_at,
        accommodations,
        |patch| patch.available_at,
        PolicyField::AvailableAt,
        OptionalRule::Earlier,
    )?;
    apply_accommodation_field(
        &mut policy.due_at,
        accommodations,
        |patch| patch.due_at,
        PolicyField::DueAt,
        OptionalRule::Later,
    )?;
    apply_accommodation_field(
        &mut policy.closes_at,
        accommodations,
        |patch| patch.closes_at,
        PolicyField::ClosesAt,
        OptionalRule::Later,
    )?;
    apply_accommodation_field(
        &mut policy.time_limit_seconds,
        accommodations,
        |patch| patch.time_limit_seconds,
        PolicyField::TimeLimitSeconds,
        OptionalRule::Later,
    )?;
    apply_accommodation_field(
        &mut policy.attempt_limit,
        accommodations,
        |patch| patch.attempt_limit,
        PolicyField::AttemptLimit,
        OptionalRule::Later,
    )?;
    Ok(())
}

fn apply_accommodation_field<T: Ord + Copy>(
    field: &mut ResolvedField<Option<T>>,
    accommodations: &[GroupAccommodation],
    patch_for: impl Fn(PolicyPatchSet) -> PolicyPatch<T>,
    policy_field: PolicyField,
    rule: OptionalRule,
) -> Result<(), EffectivePolicyError> {
    let mut overrides = Vec::new();
    let mut extensions = Vec::new();
    for accommodation in accommodations {
        let patch = patch_for(accommodation.patch);
        if matches!(patch, PolicyPatch::Inherit) {
            continue;
        }
        match accommodation.mode {
            PolicyModificationMode::Override => overrides.push((accommodation.group, patch)),
            PolicyModificationMode::ExtendOnly => {
                let replacement = patch_value(patch);
                if !extends_optional(field.value, replacement, rule) {
                    return Err(EffectivePolicyError::ExtendOnlyViolation {
                        field: policy_field,
                        source: ModifierSource::Group(accommodation.group),
                    });
                }
                extensions.push((accommodation.group, replacement));
            }
        }
    }

    overrides.sort_unstable_by_key(|(group, _)| *group);
    if overrides.len() > 1 {
        return Err(EffectivePolicyError::MultipleAccommodationOverrides {
            field: policy_field,
            sources: overrides.into_iter().map(|(group, _)| group).collect(),
        });
    }
    if let Some((group, patch)) = overrides.pop() {
        field.value = patch_value(patch);
        field.source = PolicySource::GroupAccommodations(vec![group]);
        return Ok(());
    }
    if extensions.is_empty() {
        return Ok(());
    }

    let winner = select_most_permissive(&extensions, rule);
    let mut sources = extensions
        .into_iter()
        .filter_map(|(group, candidate)| (candidate == winner).then_some(group))
        .collect::<Vec<_>>();
    sources.sort_unstable();
    field.value = winner;
    field.source = PolicySource::GroupAccommodations(sources);
    Ok(())
}

fn patch_value<T>(patch: PolicyPatch<T>) -> Option<T> {
    match patch {
        PolicyPatch::Set(value) => Some(value),
        PolicyPatch::Unrestricted => None,
        PolicyPatch::Inherit => {
            unreachable!("inherit patches are filtered before value extraction")
        }
    }
}

#[derive(Clone, Copy)]
enum OptionalRule {
    Earlier,
    Later,
}

fn select_most_permissive<T: Ord + Copy>(
    candidates: &[(CourseGroupId, Option<T>)],
    rule: OptionalRule,
) -> Option<T> {
    candidates
        .iter()
        .map(|(_, candidate)| *candidate)
        .reduce(|current, candidate| more_permissive(current, candidate, rule))
        .expect("at least one accommodation candidate")
}

fn more_permissive<T: Ord>(
    current: Option<T>,
    candidate: Option<T>,
    rule: OptionalRule,
) -> Option<T> {
    match (current, candidate) {
        (None, _) | (_, None) => None,
        (Some(current), Some(candidate)) => Some(match rule {
            OptionalRule::Earlier => current.min(candidate),
            OptionalRule::Later => current.max(candidate),
        }),
    }
}

fn apply_individual_patch(
    policy: &mut EffectiveAssignmentPolicy,
    individual: IndividualPolicyException,
) -> Result<(), EffectivePolicyError> {
    let source = ModifierSource::Individual(individual.student);
    apply_individual_field(
        &mut policy.available_at,
        individual.patch.available_at,
        individual.mode,
        PolicyField::AvailableAt,
        OptionalRule::Earlier,
        source,
    )?;
    apply_individual_field(
        &mut policy.due_at,
        individual.patch.due_at,
        individual.mode,
        PolicyField::DueAt,
        OptionalRule::Later,
        source,
    )?;
    apply_individual_field(
        &mut policy.closes_at,
        individual.patch.closes_at,
        individual.mode,
        PolicyField::ClosesAt,
        OptionalRule::Later,
        source,
    )?;
    apply_individual_field(
        &mut policy.time_limit_seconds,
        individual.patch.time_limit_seconds,
        individual.mode,
        PolicyField::TimeLimitSeconds,
        OptionalRule::Later,
        source,
    )?;
    apply_individual_field(
        &mut policy.attempt_limit,
        individual.patch.attempt_limit,
        individual.mode,
        PolicyField::AttemptLimit,
        OptionalRule::Later,
        source,
    )?;
    Ok(())
}

fn apply_individual_field<T: Ord + Copy>(
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
    let replacement = patch_value(patch);
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
        ModifierSource::Individual(student) => PolicySource::IndividualException(student),
        ModifierSource::Group(_) => {
            unreachable!("individual patches always carry an individual source")
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
    let available = policy.available_at.value;
    let due = policy.due_at.value;
    let closes = policy.closes_at.value;
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

//! Backend-neutral effective-assignment-policy records and commands.
//!
//! This module deliberately contains no resolver.  `domain` owns resolution;
//! this layer persists its M1--M4 inputs and immutable issued receipts.

use domain::effective_assignment_policy::{
    BaseAssignmentPolicy, EffectiveAssignmentPolicy, GroupAccommodation, GroupScheduleOffset,
    IndividualPolicyException, PolicySource, ResolvedField,
};
use question_model::{
    AssignmentId, AssignmentPolicyExceptionId, CourseGroupId, CourseGroupPurpose,
    CourseGroupPurposePolicy, CourseGroupReference, CourseId, CourseMembershipId,
    MultipleMembershipDisposition, QuestionAttemptId, StudentId, UserId,
};
use serde::{Deserialize, Serialize};

use crate::{AssignmentRevision, StoreError};

/// Server-issued optimistic revision for one current course group.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct CourseGroupRevision(u64);

impl CourseGroupRevision {
    #[cfg(any(test, feature = "test-support"))]
    pub(crate) const INITIAL: Self = Self(1);
    #[cfg(any(test, feature = "test-support"))]
    const MAX: u64 = i64::MAX as u64;

    pub fn value(self) -> u64 {
        self.0
    }

    #[cfg(any(test, feature = "test-support"))]
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
        (value > 0).then_some(Self(value)).ok_or_else(|| {
            StoreError::Unavailable("stored course group revision is invalid".to_string())
        })
    }
}

/// Current course group with one closed purpose and canonical members.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CourseGroupRecord {
    pub id: CourseGroupId,
    pub course: CourseId,
    pub purpose: CourseGroupPurpose,
    pub title: String,
    pub members: Vec<CourseMembershipId>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoredCourseGroup {
    pub record: CourseGroupRecord,
    pub revision: CourseGroupRevision,
}

/// Authorized internal Store projection. HTTP must remap it to a strict
/// browser DTO and never expose its durable identifiers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CourseGroupView {
    pub reference: CourseGroupReference,
    pub group: StoredCourseGroup,
}

/// Server-issued revision for one course-purpose membership policy.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct CourseGroupPurposePolicyRevision(u64);

impl CourseGroupPurposePolicyRevision {
    /// Initial durable revision assigned when a purpose policy is created.
    ///
    /// This is part of the public revision contract used by connected store
    /// clients as well as the in-memory implementation.
    pub const INITIAL: Self = Self(1);
    #[cfg(any(test, feature = "test-support"))]
    const MAX: u64 = i64::MAX as u64;

    pub const fn value(self) -> u64 {
        self.0
    }

    #[cfg(any(test, feature = "test-support"))]
    pub(crate) fn next(self) -> Result<Self, StoreError> {
        self.0
            .checked_add(1)
            .filter(|value| *value <= Self::MAX)
            .map(Self)
            .ok_or_else(|| {
                StoreError::Unavailable("course group purpose policy revision limit reached".into())
            })
    }

    #[cfg(feature = "postgres")]
    #[allow(dead_code)] // consumed by the reserved PostgreSQL T2 management implementation
    pub(crate) fn from_stored(value: i64) -> Result<Self, StoreError> {
        let value = u64::try_from(value).map_err(|_| {
            StoreError::Unavailable("stored course group purpose policy revision is invalid".into())
        })?;
        (value > 0).then_some(Self(value)).ok_or_else(|| {
            StoreError::Unavailable("stored course group purpose policy revision is invalid".into())
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StoredCourseGroupPurposePolicy {
    pub policy: CourseGroupPurposePolicy,
    pub revision: CourseGroupPurposePolicyRevision,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UpdateCourseGroupPurposePolicyCommand {
    /// Opaque authenticated-session capability. The Store resolves its live
    /// subject while it performs the policy CAS; callers cannot nominate an
    /// authority-bearing user identifier.
    pub session: crate::SessionTokenHash,
    pub course: CourseId,
    pub expected_revision: CourseGroupPurposePolicyRevision,
    pub policy: CourseGroupPurposePolicy,
}

/// Informational result of one accepted membership edit. It is deliberately
/// not an authorization or entitlement verdict.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CourseGroupMembershipWarning {
    pub membership: CourseMembershipId,
    pub purpose: CourseGroupPurpose,
    pub membership_count: u32,
    pub disposition: MultipleMembershipDisposition,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PutCourseGroupCommand {
    pub actor: UserId,
    pub expected_revision: Option<CourseGroupRevision>,
    pub record: CourseGroupRecord,
}

/// M1: assignment-owned policy with no audience or membership authority.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StoredBaseAssignmentPolicy {
    pub course: CourseId,
    pub assignment: AssignmentId,
    pub policy: BaseAssignmentPolicy,
    pub revision: AssignmentRevision,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PutAssignmentTeachingSettingsCommand {
    pub actor: UserId,
    pub course: CourseId,
    pub assignment: AssignmentId,
    pub expected_revision: AssignmentRevision,
    pub settings: question_model::AssignmentTeachingSettings,
}

/// M2: additive schedule adjustment, keyed by assignment and group.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PutGroupScheduleOffsetCommand {
    pub actor: UserId,
    pub course: CourseId,
    pub assignment: AssignmentId,
    pub expected_revision: AssignmentRevision,
    pub offset: GroupScheduleOffset,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DeleteGroupScheduleOffsetCommand {
    pub actor: UserId,
    pub course: CourseId,
    pub assignment: AssignmentId,
    pub expected_revision: AssignmentRevision,
    pub group: CourseGroupId,
}

/// M3: group accommodation, keyed by assignment and group.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PutGroupAccommodationCommand {
    pub actor: UserId,
    pub course: CourseId,
    pub assignment: AssignmentId,
    pub expected_revision: AssignmentRevision,
    pub accommodation: GroupAccommodation,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DeleteGroupAccommodationCommand {
    pub actor: UserId,
    pub course: CourseId,
    pub assignment: AssignmentId,
    pub expected_revision: AssignmentRevision,
    pub group: CourseGroupId,
}

/// M4 stable identity.  The explicit ID is an internal record identity only;
/// the resolver authorizes it by its StudentId against an S5 grant.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StoredIndividualPolicyException {
    pub id: AssignmentPolicyExceptionId,
    pub exception: IndividualPolicyException,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PutIndividualPolicyExceptionCommand {
    pub actor: UserId,
    pub course: CourseId,
    pub assignment: AssignmentId,
    pub expected_revision: AssignmentRevision,
    pub exception: StoredIndividualPolicyException,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DeleteIndividualPolicyExceptionCommand {
    pub actor: UserId,
    pub course: CourseId,
    pub assignment: AssignmentId,
    pub expected_revision: AssignmentRevision,
    pub student: StudentId,
}

/// A persistence read is deliberately a receipt-shaped domain decision, not
/// a second timing resolver.  The caller supplies the S5 decision and clock.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EffectivePolicyResolution {
    pub course: CourseId,
    pub assignment: AssignmentId,
    pub decision: domain::effective_assignment_policy::EffectivePolicyDecision,
    pub revision: AssignmentRevision,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolveEffectivePolicyCommand {
    pub assignment: AssignmentId,
    pub entitlement: domain::entitlement::EntitlementDecision,
    pub authorization: domain::effective_assignment_policy::AuthorizationGate,
    pub now: question_model::ActivityTimestamp,
    pub prior_run_count: u32,
}

/// Immutable effective-policy receipt for an issued question attempt.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IssuedEffectivePolicyReceipt {
    pub attempt: QuestionAttemptId,
    pub generation: u64,
    pub policy: EffectiveAssignmentPolicy,
}

/// Normalized receipt field sources; one row per effective policy field and
/// source ordering.  It has no mutable current-policy semantics.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IssuedEffectivePolicyFieldSource {
    pub attempt: QuestionAttemptId,
    pub generation: u64,
    pub field: EffectivePolicyField,
    pub source_order: u32,
    pub source: PolicySource,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum EffectivePolicyField {
    AvailableAt,
    DueAt,
    ClosesAt,
    TimeLimitSeconds,
    AttemptLimit,
    LateSubmission,
    DeadlineBehavior,
}

impl IssuedEffectivePolicyReceipt {
    pub fn field_sources(&self) -> Vec<IssuedEffectivePolicyFieldSource> {
        let mut sources = Vec::new();
        push_sources(
            &mut sources,
            self.attempt,
            self.generation,
            EffectivePolicyField::AvailableAt,
            &self.policy.available_at,
        );
        push_sources(
            &mut sources,
            self.attempt,
            self.generation,
            EffectivePolicyField::DueAt,
            &self.policy.due_at,
        );
        push_sources(
            &mut sources,
            self.attempt,
            self.generation,
            EffectivePolicyField::ClosesAt,
            &self.policy.closes_at,
        );
        push_sources(
            &mut sources,
            self.attempt,
            self.generation,
            EffectivePolicyField::TimeLimitSeconds,
            &self.policy.time_limit_seconds,
        );
        push_sources(
            &mut sources,
            self.attempt,
            self.generation,
            EffectivePolicyField::AttemptLimit,
            &self.policy.attempt_limit,
        );
        push_sources(
            &mut sources,
            self.attempt,
            self.generation,
            EffectivePolicyField::LateSubmission,
            &self.policy.late_submission,
        );
        push_sources(
            &mut sources,
            self.attempt,
            self.generation,
            EffectivePolicyField::DeadlineBehavior,
            &self.policy.deadline_behavior,
        );
        sources
    }
}

fn push_sources<T>(
    out: &mut Vec<IssuedEffectivePolicyFieldSource>,
    attempt: QuestionAttemptId,
    generation: u64,
    field: EffectivePolicyField,
    resolved: &ResolvedField<T>,
) {
    match &resolved.source {
        PolicySource::GroupScheduleOffsets(groups) | PolicySource::GroupAccommodations(groups) => {
            for (source_order, group) in groups.iter().enumerate() {
                out.push(IssuedEffectivePolicyFieldSource {
                    attempt,
                    generation,
                    field,
                    source_order: u32::try_from(source_order)
                        .expect("policy source count fits u32"),
                    source: match &resolved.source {
                        PolicySource::GroupScheduleOffsets(_) => {
                            PolicySource::GroupScheduleOffsets(vec![*group])
                        }
                        PolicySource::GroupAccommodations(_) => {
                            PolicySource::GroupAccommodations(vec![*group])
                        }
                        _ => unreachable!(),
                    },
                });
            }
        }
        source => out.push(IssuedEffectivePolicyFieldSource {
            attempt,
            generation,
            field,
            source_order: 0,
            source: source.clone(),
        }),
    }
}

/// Stored M1--M4 inputs used exactly once by the domain resolver.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct EffectivePolicyInputs {
    pub base: BaseAssignmentPolicy,
    pub schedule_offsets: Vec<GroupScheduleOffset>,
    pub accommodations: Vec<GroupAccommodation>,
    pub individual: Option<IndividualPolicyException>,
}

//! Internal entitlement facts and materialization evidence.
//!
//! These contracts are deliberately not serialized.  The browser asks the
//! server to perform an action; it never receives a reusable authority token
//! or a roster/group explanation.

use crate::{AccountId, ActivityTimestamp, CourseMembershipId, StudentRecordId};
/// Basis that granted current direct Assignment Access.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MaterializationBasis {
    ActiveStudentCourseMembership,
}

/// Event which is allowed to mint an educational receipt.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EntitlementPurpose {
    StartRun,
    GradeBearingAction,
    InstructorIssue,
}

/// Closed non-person authority that can materialize a grade-bearing receipt.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MaterializationRule {
    ImportedGrade,
    AutomatedGrader,
}

/// Who or what justified the immutable receipt creation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MaterializationAuthority {
    Account(AccountId),
    Rule(MaterializationRule),
}

/// Version of the pure evaluator used when a receipt was first materialized.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EvaluatorVersion(pub u16);

impl EvaluatorVersion {
    pub const INITIAL: Self = Self(1);
}

/// Immutable receipt provenance, separate from mutable scoring fields.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EntitlementMaterialization {
    pub student_record: StudentRecordId,
    pub membership: CourseMembershipId,
    pub account: AccountId,
    pub occurred_at: ActivityTimestamp,
    pub purpose: EntitlementPurpose,
    pub authority: MaterializationAuthority,
    pub basis: MaterializationBasis,
    pub evaluator_version: EvaluatorVersion,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MaterializationDisposition {
    Created,
    Existing,
}

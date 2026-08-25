//! Complete pre-issuance assignment-definition replacement.
//!
//! A structural edit is deliberately one revision-checked command.  Fixed
//! items and selection groups share one visible position namespace, so a
//! sequence of narrow add/remove/move calls could expose an incomplete
//! definition to the first learner run.

use question_model::{AssignmentId, BaseAssignmentPolicy, CourseId, UserId};

use super::{AssignmentRecord, AssignmentRevision};

/// Replaces the complete server-resolved assignment definition before any
/// learner run exists.
///
/// The server constructs `definition` only after resolving every selected
/// public Question ID to its immutable publication and minting assignment-owned
/// identities.  Storage authorizes `actor`, verifies all route bindings and
/// `expected_revision`, serializes with first-run issuance, and rejects once
/// immutable run evidence exists.
#[derive(Debug, Clone, PartialEq)]
pub struct ReplaceUnissuedAssignmentDefinitionCommand {
    /// Authenticated direct Instructor evaluated inside the broker.
    pub actor: UserId,
    /// Course that owns both the authority and assignment route.
    pub course: CourseId,
    /// Assignment whose unissued definition is being replaced.
    pub assignment: AssignmentId,
    /// Strong revision observed by the editor before submitting the complete replacement.
    pub expected_revision: AssignmentRevision,
    /// Entire ordered, server-resolved mutable definition.
    pub definition: AssignmentRecord,
    /// Complete server-resolved base policy paired with the definition.
    pub base_policy: BaseAssignmentPolicy,
}

/// Database-authoritative result of a structural definition replacement.
#[derive(Debug, Clone, PartialEq)]
pub enum ReplaceUnissuedAssignmentDefinitionOutcome {
    /// The complete definition replaced one revision before learner work exists.
    Replaced(Box<super::StoredAssignment>),
    /// A request with the current expected revision lost the shared assignment
    /// lock to the first committed learner run. The server presents the
    /// recovery path rather than retrying a now-immutable structural edit;
    /// stale revisions remain ordinary conflicts.
    Issued,
}

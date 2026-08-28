//! Server-owned causes for recalculating one assignment's derived scores.
//!
//! Origins are intentionally compact evidence: they identify the authorized
//! mutation that made a prior derived total stale, while keeping responses,
//! evaluations, and numeric scores in their existing protected owners.

use question_model::{AssignmentRevision, UserId};
use uuid::Uuid;

/// The closed causal vocabulary shared by Memory and PostgreSQL.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ScoringInvalidationOriginKind {
    InstructorRecalculation,
    AssignmentDefinition,
    ManualGrade,
    LearnerSupport,
    AcceptedSubmissionCompletion,
}

/// Stable UUID identity scoped by tenant and origin kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ScoringInvalidationOriginId(Uuid);

impl ScoringInvalidationOriginId {
    pub fn from_uuid(value: Uuid) -> Self {
        Self(value)
    }

    pub fn as_uuid(self) -> Uuid {
        self.0
    }
}

/// Immutable, answer-free evidence that a scoring generation was requested.
///
/// The actor is required for every authorized mutation and intentionally absent
/// for server-owned accepted-submission completion.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ScoringInvalidationOrigin {
    pub kind: ScoringInvalidationOriginKind,
    pub id: ScoringInvalidationOriginId,
    pub actor: Option<UserId>,
}

impl ScoringInvalidationOrigin {
    pub fn instructor_recalculation(id: ScoringInvalidationOriginId, actor: UserId) -> Self {
        Self {
            kind: ScoringInvalidationOriginKind::InstructorRecalculation,
            id,
            actor: Some(actor),
        }
    }

    pub fn assignment_definition(
        assignment: Uuid,
        revision: AssignmentRevision,
        actor: UserId,
    ) -> Self {
        Self {
            kind: ScoringInvalidationOriginKind::AssignmentDefinition,
            id: assignment_definition_scoring_invalidation_origin_id(assignment, revision),
            actor: Some(actor),
        }
    }

    pub fn manual_grade(id: ScoringInvalidationOriginId, actor: UserId) -> Self {
        Self {
            kind: ScoringInvalidationOriginKind::ManualGrade,
            id,
            actor: Some(actor),
        }
    }

    pub fn learner_support(id: ScoringInvalidationOriginId, actor: UserId) -> Self {
        Self {
            kind: ScoringInvalidationOriginKind::LearnerSupport,
            id,
            actor: Some(actor),
        }
    }

    pub fn accepted_submission_completion(id: ScoringInvalidationOriginId) -> Self {
        Self {
            kind: ScoringInvalidationOriginKind::AcceptedSubmissionCompletion,
            id,
            actor: None,
        }
    }
}

/// Derives a collision-resistant stable UUID from the immutable assignment and
/// resulting definition revision. This preserves exact replay without adding a
/// second action token to ordinary revision-checked definition commands.
pub(crate) fn assignment_definition_scoring_invalidation_origin_id(
    assignment: Uuid,
    revision: AssignmentRevision,
) -> ScoringInvalidationOriginId {
    let mut evidence = Vec::with_capacity(24);
    evidence.extend_from_slice(assignment.as_bytes());
    evidence.extend_from_slice(&revision.value().to_be_bytes());
    let digest = objects::Sha256Digest::compute(&evidence);
    let mut bytes = [0_u8; 16];
    bytes.copy_from_slice(&digest.as_bytes()[..16]);
    ScoringInvalidationOriginId(Uuid::from_bytes(bytes))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn assignment_definition_origin_matches_the_sql_uuid_and_bigint_digest_contract() {
        let assignment =
            Uuid::parse_str("00010203-0405-0607-0809-0a0b0c0d0e0f").expect("fixed assignment UUID");
        let revision = AssignmentRevision::new(7).expect("fixed revision");

        assert_eq!(
            assignment_definition_scoring_invalidation_origin_id(assignment, revision).as_uuid(),
            Uuid::parse_str("ea315a3e-5b0b-7765-5c3e-f5f312c80f8f")
                .expect("fixed independently calculated SHA-256 prefix"),
        );
    }
}

//! Operation capability modules for curriculum adoption.

mod assignment_update;
mod course_lifecycle;
mod dispatch;
mod source_adoption;

use super::{MemoryCurriculumAdoptionEvidence, MemoryCurriculumAdoptionOutcome};

/// Exact domain facts produced by one lock-held curriculum-adoption mutation.
///
/// The dispatcher supplies the transaction envelope: it authorizes the session,
/// verifies the canonical intent, handles replay, and persists this result as the
/// immutable completed receipt. Keeping those responsibilities outside the core
/// makes every state transition participate in one rollback boundary.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct AppliedCurriculumAdoption {
    pub(super) outcome: MemoryCurriculumAdoptionOutcome,
    pub(super) evidence: MemoryCurriculumAdoptionEvidence,
}

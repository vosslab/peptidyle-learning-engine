//! Shared authenticated transaction boundary for current Assessment operations.

use super::Pool;

/// PostgreSQL Store for the direct-Instructor Assessment Workspace.
#[derive(Clone)]
pub struct PostgresLiveAssessmentStore {
    pub(super) pool: Pool,
}

impl PostgresLiveAssessmentStore {
    /// Binds the attested API pool to Assessment Workspace procedures.
    pub fn new(pool: Pool) -> Self {
        Self { pool }
    }
}

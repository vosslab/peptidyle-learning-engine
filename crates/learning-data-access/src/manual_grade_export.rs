//! Protected, bounded manual grade-export contract.

use async_trait::async_trait;
use question_model::{AssignmentId, CourseId};
use uuid::Uuid;

use crate::{AuthenticationEmail, CourseRosterId, SessionTokenHash, StoreError, TenantContext};

/// Maximum rows in one synchronous manual export.
pub const MAX_MANUAL_GRADE_EXPORT_ROWS: usize = 500;

/// Opaque audit identity for one instructor-triggered download.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ManualGradeExportId(Uuid);

impl ManualGradeExportId {
    pub fn generate() -> Result<Self, StoreError> {
        let mut bytes = [0_u8; 16];
        getrandom::fill(&mut bytes).map_err(|error| {
            StoreError::Unavailable(format!("manual grade export ID unavailable: {error}"))
        })?;
        Ok(Self(Uuid::from_bytes(bytes)))
    }

    pub fn from_uuid(value: Uuid) -> Self {
        Self(value)
    }

    pub fn as_uuid(self) -> Uuid {
        self.0
    }
}

/// One course-scoped row; global account and learner identifiers are absent.
#[derive(Clone, PartialEq)]
pub struct ManualGradeExportRow {
    pub roster_id: CourseRosterId,
    pub roster_email: AuthenticationEmail,
    pub display_name: String,
    pub current_score: Option<f64>,
}

impl std::fmt::Debug for ManualGradeExportRow {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("ManualGradeExportRow")
            .field("roster_id", &"[protected]")
            .field("roster_email", &"[protected]")
            .field("display_name", &"[protected]")
            .field("current_score", &self.current_score)
            .finish()
    }
}

/// Instructor-authorized request for one course assignment.
#[derive(Debug, Clone, Copy)]
pub struct CreateManualGradeExport {
    pub course: CourseId,
    pub assignment: AssignmentId,
}

/// Ephemeral export data plus its durable PII-free audit identity.
#[derive(Clone, PartialEq)]
pub struct ManualGradeExport {
    pub id: ManualGradeExportId,
    pub course: CourseId,
    pub assignment: AssignmentId,
    pub rows: Vec<ManualGradeExportRow>,
}

impl std::fmt::Debug for ManualGradeExport {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("ManualGradeExport")
            .field("id", &self.id)
            .field("course", &self.course)
            .field("assignment", &self.assignment)
            .field("row_count", &self.rows.len())
            .finish()
    }
}

/// Store boundary for one protected, synchronous manual export.
#[async_trait]
pub trait ManualGradeExportStore: Send + Sync {
    async fn create_manual_grade_export(
        &self,
        context: TenantContext,
        session: SessionTokenHash,
        command: CreateManualGradeExport,
    ) -> Result<ManualGradeExport, StoreError>;
}

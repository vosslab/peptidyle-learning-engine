//! Protected course-grade configuration, totals, and synchronous export contract.

use async_trait::async_trait;
use domain::course_grade::CourseGradeOutcome;
use question_model::{
    AssignmentDeliveryState, AssignmentId, AssignmentScoringMode, CourseGradeMode,
    CourseGradeScheme, CourseId, GradeCategoryId, PointValue,
};
use uuid::Uuid;

use crate::{AuthenticationEmail, CourseRosterId, SessionTokenHash, StoreError, TenantContext};

/// Maximum active students returned by a synchronous course-grade export.
pub const MAX_COURSE_GRADE_EXPORT_ROWS: usize = 500;

/// Strong, positive compare-and-swap revision for one course scheme.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CourseGradeSchemeRevision(u64);

impl CourseGradeSchemeRevision {
    /// Initial revision used by the implicit total-points scheme.
    pub const INITIAL: Self = Self(1);

    /// Returns the storage value.
    pub fn value(self) -> u64 {
        self.0
    }

    /// Rebuilds a positive revision from a storage value.
    pub fn from_u64(value: u64) -> Result<Self, StoreError> {
        if value == 0 {
            return Err(StoreError::InvalidRecord(
                "course grade scheme revision must be positive".to_string(),
            ));
        }
        Ok(Self(value))
    }

    /// Rebuilds a positive revision from PostgreSQL's signed integer domain.
    pub fn from_i64(value: i64) -> Result<Self, StoreError> {
        let value = u64::try_from(value).map_err(|_| {
            StoreError::InvalidRecord("course grade scheme revision must be positive".to_string())
        })?;
        Self::from_u64(value)
    }

    /// Converts this positive revision to PostgreSQL's signed integer domain.
    pub fn to_i64(self) -> Result<i64, StoreError> {
        i64::try_from(self.0).map_err(|_| {
            StoreError::InvalidRecord(
                "course grade scheme revision exceeds storage range".to_string(),
            )
        })
    }

    /// Advances one successful update without permitting a zero revision.
    pub fn next(self) -> Result<Self, StoreError> {
        self.0.checked_add(1).map(Self).ok_or_else(|| {
            StoreError::Unavailable("course grade scheme revision exhausted".to_string())
        })
    }
}

/// Course-owned assignment membership in the active grade scheme.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CourseGradeAssignmentMembership {
    /// Assignment included by this course-grade configuration.
    pub assignment: AssignmentId,
    /// Whether the assignment contributes to course totals.
    pub included: bool,
    /// Required category for included weighted assignments.
    pub category: Option<question_model::GradeCategoryId>,
    /// Canonical zero-based position within the selected category.
    pub position: Option<u32>,
}

/// Instructor read projection for one current course assignment.
///
/// The title is sourced from the current assignment record by storage. It is
/// deliberately absent from [`CourseGradeAssignmentMembership`], so a scheme
/// update cannot grant a client authority to rename an assignment.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CourseGradeAssignmentRecord {
    /// Current assignment identity.
    pub assignment: AssignmentId,
    /// Current server-owned assignment title.
    pub title: String,
    /// Whether the assignment contributes to course totals.
    pub included: bool,
    /// Current category mapping, if any.
    pub category: Option<question_model::GradeCategoryId>,
    /// Current category-local position, if any.
    pub position: Option<u32>,
}

/// Scheme plus its strong revision and explicit assignment memberships.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CourseGradeSchemeRecord {
    /// Course that owns the scheme.
    pub course: CourseId,
    /// Strong revision read by the instructor.
    pub revision: CourseGradeSchemeRevision,
    /// Closed aggregation configuration.
    pub scheme: CourseGradeScheme,
    /// Current assignment display projections and explicit settings.
    pub assignments: Vec<CourseGradeAssignmentRecord>,
}

/// Revision-checked whole-scheme replacement.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UpdateCourseGradeScheme {
    /// Course that owns the update.
    pub course: CourseId,
    /// Revision returned by the preceding read.
    pub expected_revision: CourseGradeSchemeRevision,
    /// Closed replacement scheme.
    pub scheme: CourseGradeScheme,
    /// Exact current assignment settings.
    pub assignments: Vec<CourseGradeAssignmentMembership>,
}

/// One protected course-total row. Debug deliberately omits roster PII.
#[derive(Clone, PartialEq)]
pub struct CourseGradebookTotalRow {
    /// Course-local stable roster identifier.
    pub roster_id: CourseRosterId,
    /// Roster email used only for the instructor's ephemeral export.
    pub roster_email: AuthenticationEmail,
    /// Course roster display name used only for the instructor's ephemeral export.
    pub display_name: String,
    /// Calculated total or its explicit unavailable reason.
    pub outcome: CourseGradeOutcome,
}

/// One atomic scheme snapshot and its bounded instructor-only total rows.
#[derive(Clone, PartialEq)]
pub struct CourseGradebookTotals {
    /// Revision that governed every returned row.
    pub scheme_revision: CourseGradeSchemeRevision,
    /// Aggregation mode that governed every returned row.
    pub mode: question_model::CourseGradeMode,
    /// Rounding rule that governed every returned row.
    pub rounding: question_model::CourseGradeRoundingRule,
    /// Instructor-only rows, ordered by stable roster identifier.
    pub rows: Vec<CourseGradebookTotalRow>,
}

impl std::fmt::Debug for CourseGradebookTotals {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("CourseGradebookTotals")
            .field("scheme_revision", &self.scheme_revision)
            .field("mode", &self.mode)
            .field("rounding", &self.rounding)
            .field("row_count", &self.rows.len())
            .finish()
    }
}

impl std::fmt::Debug for CourseGradebookTotalRow {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("CourseGradebookTotalRow")
            .field("roster_id", &"[protected]")
            .field("roster_email", &"[protected]")
            .field("display_name", &"[protected]")
            .field("outcome", &self.outcome)
            .finish()
    }
}

/// Opaque audit identity for one course-grade export.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CourseGradeExportId(Uuid);

impl CourseGradeExportId {
    /// Mints a durable audit identifier without exposing roster data.
    pub fn generate() -> Result<Self, StoreError> {
        let mut bytes = [0_u8; 16];
        getrandom::fill(&mut bytes).map_err(|error| {
            StoreError::Unavailable(format!("course grade export ID unavailable: {error}"))
        })?;
        Ok(Self(Uuid::from_bytes(bytes)))
    }

    /// Rebuilds an audit identifier read from durable storage.
    pub fn from_uuid(value: Uuid) -> Self {
        Self(value)
    }

    /// Returns the durable UUID representation.
    pub fn as_uuid(self) -> Uuid {
        self.0
    }
}

/// Durable PII-free metadata for one requested export.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CourseGradeExportAudit {
    /// Export audit identity.
    pub id: CourseGradeExportId,
    /// Tenant RLS boundary for this audit record.
    pub tenant: question_model::TenantId,
    /// Course whose grade totals were exported.
    pub course: CourseId,
    /// Actor that requested the export.
    pub requested_by: question_model::UserId,
    /// Scheme revision used for the ephemeral rows.
    pub scheme_revision: CourseGradeSchemeRevision,
    /// Closed aggregation mode used by the emitted rows, including an empty export.
    pub mode: question_model::CourseGradeMode,
    /// Explicit final rounding rule used by the emitted rows, including an empty export.
    pub rounding: question_model::CourseGradeRoundingRule,
    /// Number of returned active-student rows.
    pub row_count: usize,
}

/// Ephemeral course-grade rows plus a durable PII-free audit record.
#[derive(Clone, PartialEq)]
pub struct CourseGradeExport {
    /// Audit metadata; it intentionally carries no row data.
    pub audit: CourseGradeExportAudit,
    /// Instructor-only data to be encoded by the HTTP boundary.
    pub rows: Vec<CourseGradebookTotalRow>,
}

impl std::fmt::Debug for CourseGradeExport {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("CourseGradeExport")
            .field("audit", &self.audit)
            .field("row_count", &self.rows.len())
            .finish()
    }
}

/// Isolated instructor-only course-grade capability.
#[async_trait]
pub trait CourseGradebookStore: Send + Sync {
    /// Reads one course scheme, returning the deterministic implicit default when absent.
    async fn course_grade_scheme(
        &self,
        context: TenantContext,
        session: SessionTokenHash,
        course: CourseId,
    ) -> Result<CourseGradeSchemeRecord, StoreError>;

    /// Atomically validates and saves one revision-checked scheme replacement.
    async fn update_course_grade_scheme(
        &self,
        context: TenantContext,
        session: SessionTokenHash,
        command: UpdateCourseGradeScheme,
    ) -> Result<CourseGradeSchemeRecord, StoreError>;

    /// Returns bounded protected course totals derived only from assignment summaries.
    async fn course_gradebook_totals(
        &self,
        context: TenantContext,
        session: SessionTokenHash,
        course: CourseId,
    ) -> Result<CourseGradebookTotals, StoreError>;

    /// Produces bounded ephemeral rows and records a PII-free audit.
    async fn create_course_grade_export(
        &self,
        context: TenantContext,
        session: SessionTokenHash,
        course: CourseId,
    ) -> Result<CourseGradeExport, StoreError>;
}

/// Validates one whole-scheme command after its backend has locked the exact
/// current course assignment IDs. Both persistence backends use this rule.
pub(crate) fn validate_course_grade_scheme_update(
    command: &UpdateCourseGradeScheme,
    current_assignments: &std::collections::BTreeSet<AssignmentId>,
) -> Result<(), StoreError> {
    command
        .scheme
        .validate()
        .map_err(|error| StoreError::InvalidRecord(error.to_string()))?;
    let supplied: std::collections::BTreeSet<_> = command
        .assignments
        .iter()
        .map(|member| member.assignment)
        .collect();
    if supplied.len() != command.assignments.len() || supplied != *current_assignments {
        return Err(StoreError::InvalidRecord(
            "course grade scheme assignments must exactly match the current course".to_string(),
        ));
    }
    match command.scheme.mode {
        CourseGradeMode::TotalPoints => {
            if command
                .assignments
                .iter()
                .any(|member| member.category.is_some() || member.position.is_some())
            {
                return Err(StoreError::InvalidRecord(
                    "total-points schemes cannot carry category mappings".to_string(),
                ));
            }
        }
        CourseGradeMode::WeightedCategories => {
            let known: std::collections::BTreeSet<_> = command
                .scheme
                .categories
                .iter()
                .map(|category| category.id)
                .collect();
            let mut positions: std::collections::BTreeMap<
                GradeCategoryId,
                std::collections::BTreeSet<u32>,
            > = std::collections::BTreeMap::new();
            let mut included: std::collections::BTreeMap<GradeCategoryId, usize> =
                std::collections::BTreeMap::new();
            for member in &command.assignments {
                if member.category.is_some() != member.position.is_some() {
                    return Err(StoreError::InvalidRecord(
                        "weighted assignment category and position must be paired".to_string(),
                    ));
                }
                if let Some(category) = member.category
                    && !known.contains(&category)
                {
                    return Err(StoreError::InvalidRecord(
                        "course grade assignment references an unknown category".to_string(),
                    ));
                }
                if member.included && member.category.is_none() {
                    return Err(StoreError::InvalidRecord(
                        "included weighted assignment requires a category mapping".to_string(),
                    ));
                }
                if let (Some(category), Some(position)) = (member.category, member.position) {
                    if !positions.entry(category).or_default().insert(position) {
                        return Err(StoreError::InvalidRecord(
                            "weighted assignment positions must be unique per category".to_string(),
                        ));
                    }
                    if member.included {
                        *included.entry(category).or_default() += 1;
                    }
                }
            }
            for category in &command.scheme.categories {
                let positions = positions.get(&category.id).cloned().unwrap_or_default();
                if !positions
                    .iter()
                    .copied()
                    .eq(0..u32::try_from(positions.len()).expect("position count fits"))
                {
                    return Err(StoreError::InvalidRecord(
                        "weighted assignment positions must be canonical per category".to_string(),
                    ));
                }
                if usize::try_from(category.drop_lowest).expect("u32 fits usize")
                    >= included.get(&category.id).copied().unwrap_or_default()
                {
                    return Err(StoreError::InvalidRecord(
                        "weighted category drop-lowest must leave an included assignment"
                            .to_string(),
                    ));
                }
            }
        }
    }
    Ok(())
}

/// Derives exactly the grade-bearing possible points for an assignment.
pub(crate) fn course_grade_assignment_points(
    assignment: &crate::AssignmentRecord,
) -> Result<PointValue, StoreError> {
    let mut total = PointValue::ZERO;
    for item in assignment
        .items
        .iter()
        .filter(|item| item.delivery_state == AssignmentDeliveryState::Active)
    {
        if matches!(
            item.scoring_mode,
            AssignmentScoringMode::Normal | AssignmentScoringMode::FullCredit
        ) {
            total = total.checked_add(item.points_possible).ok_or_else(|| {
                StoreError::InvalidRecord(
                    "assignment course points exceed the supported range".to_string(),
                )
            })?;
        }
    }
    for group in &assignment.selection_groups {
        let points = group
            .points_per_item
            .checked_mul_u32(group.draw_count)
            .ok_or_else(|| {
                StoreError::InvalidRecord(
                    "selection-group course points exceed the supported range".to_string(),
                )
            })?;
        total = total.checked_add(points).ok_or_else(|| {
            StoreError::InvalidRecord(
                "assignment course points exceed the supported range".to_string(),
            )
        })?;
    }
    Ok(total)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scheme_revision_reconstructs_only_positive_storage_values() {
        assert_eq!(
            CourseGradeSchemeRevision::from_u64(1)
                .expect("positive revision")
                .value(),
            1
        );
        assert!(CourseGradeSchemeRevision::from_u64(0).is_err());
        assert!(CourseGradeSchemeRevision::from_i64(0).is_err());
        assert!(CourseGradeSchemeRevision::from_i64(-1).is_err());
        assert!(
            CourseGradeSchemeRevision::from_u64(u64::MAX)
                .expect("value is a valid Rust revision")
                .to_i64()
                .is_err()
        );
    }
}

//! In-memory manual grade-export projection and audit.

use async_trait::async_trait;

use super::course_roster::require_manager;
use super::{MemoryStore, require_course_records_accessible};
use crate::{
    CreateManualGradeExport, MAX_MANUAL_GRADE_EXPORT_ROWS, ManualGradeExport, ManualGradeExportId,
    ManualGradeExportRow, ManualGradeExportStore, SessionTokenHash, StoreError, TenantContext,
};

#[async_trait]
impl ManualGradeExportStore for MemoryStore {
    async fn create_manual_grade_export(
        &self,
        context: TenantContext,
        session: SessionTokenHash,
        command: CreateManualGradeExport,
    ) -> Result<ManualGradeExport, StoreError> {
        let tenant = context.tenant_id();
        let mut state = self.write_state()?;
        require_course_records_accessible(&state, tenant, command.course)?;
        let actor = require_manager(&state, context, session, command.course)?;
        let assignment = state
            .assignments
            .get(&(tenant, command.assignment))
            .filter(|assignment| assignment.course_id == command.course)
            .ok_or(StoreError::NotFound)?;
        let mut rows = state
            .roster_members
            .values()
            .filter(|member| member.tenant == tenant && member.course == command.course)
            .filter_map(|member| {
                let roster_id = member.roster_id.clone()?;
                let roster_email = member.roster_email.clone()?;
                let enrollment = state.enrollments.values().find(|enrollment| {
                    enrollment.tenant == tenant
                        && enrollment.assignment == assignment.id
                        && enrollment.student == member.student
                })?;
                let summary = state.summaries.get(&(tenant, enrollment.id))?;
                Some(ManualGradeExportRow {
                    roster_id,
                    roster_email,
                    display_name: member.display_name.clone(),
                    current_score: summary.current_score,
                })
            })
            .collect::<Vec<_>>();
        if rows.len() > MAX_MANUAL_GRADE_EXPORT_ROWS {
            return Err(StoreError::InvalidRecord(
                "manual grade export exceeds the row limit".to_string(),
            ));
        }
        rows.sort_by(|left, right| left.roster_id.cmp(&right.roster_id));
        let id = ManualGradeExportId::generate()?;
        state.manual_grade_export_audits.insert(
            id,
            (
                tenant,
                command.course,
                command.assignment,
                actor,
                rows.len(),
            ),
        );
        Ok(ManualGradeExport {
            id,
            course: command.course,
            assignment: command.assignment,
            rows,
        })
    }
}

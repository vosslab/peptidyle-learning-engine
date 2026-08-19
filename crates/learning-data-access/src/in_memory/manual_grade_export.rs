//! In-memory manual grade-export projection and audit.

use async_trait::async_trait;

use super::course_roster::require_course_instructor;
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
        let actor = require_course_instructor(&state, context, session, command.course)?;
        let assignment = state
            .assignments
            .get(&(tenant, command.assignment))
            .filter(|assignment| assignment.course_id == command.course)
            .ok_or(StoreError::NotFound)?;
        let mut rows = state
            .course_memberships
            .values()
            .filter(|membership| {
                membership.tenant == tenant
                    && membership.course == command.course
                    && membership.status == crate::CourseMemberStatus::Active
                    && membership.role == question_model::CourseMembershipRole::Student
            })
            .filter_map(|membership| {
                let student = membership.student?;
                let roster_id = membership.roster_id.clone()?;
                let profile =
                    state
                        .roster_profiles
                        .get(&(tenant, command.course, membership.id))?;
                let roster_email = profile.roster_email.clone()?;
                Some((membership, profile, student, roster_id, roster_email))
            })
            .map(|(_membership, profile, student, roster_id, roster_email)| {
                let current_score = match state.enrollments.values().find(|enrollment| {
                    enrollment.tenant == tenant
                        && enrollment.assignment == assignment.id
                        && enrollment.student == student
                }) {
                    Some(enrollment) => {
                        state
                            .summaries
                            .get(&(tenant, enrollment.id))
                            .ok_or_else(|| {
                                StoreError::Unavailable(
                                    "entitlement receipt is missing its summary".to_string(),
                                )
                            })?
                            .current_score
                    }
                    None => None,
                };
                Ok(ManualGradeExportRow {
                    roster_id,
                    roster_email,
                    display_name: profile.display_name.clone(),
                    current_score,
                })
            })
            .collect::<Result<Vec<_>, StoreError>>()?;
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

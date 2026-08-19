//! PostgreSQL manual grade-export projection and PII-free audit.

use async_trait::async_trait;
use sqlx::Row;

use super::course_roster::require_course_instructor;
use super::{PostgresStore, map_sqlx_error, retry_transaction};
use crate::{
    AuthenticationEmail, CourseRosterId, CreateManualGradeExport, MAX_MANUAL_GRADE_EXPORT_ROWS,
    ManualGradeExport, ManualGradeExportId, ManualGradeExportRow, ManualGradeExportStore,
    SessionTokenHash, StoreError, TenantContext,
};

#[async_trait]
impl ManualGradeExportStore for PostgresStore {
    async fn create_manual_grade_export(
        &self,
        context: TenantContext,
        session: SessionTokenHash,
        command: CreateManualGradeExport,
    ) -> Result<ManualGradeExport, StoreError> {
        retry_transaction(|| async move {
            let tenant = context.tenant_id();
            let mut transaction = self.begin_tenant(context).await?;
            let actor =
                require_course_instructor(&mut transaction, session, command.course).await?;
            let assignment_exists: bool = sqlx::query_scalar(
                "SELECT EXISTS(SELECT 1 FROM assignment \
             WHERE tenant_id = $1 AND course_id = $2 AND assignment_id = $3 \
               AND public.ple_course_records_accessible(tenant_id, course_id))",
            )
            .bind(tenant.as_uuid())
            .bind(command.course.as_uuid())
            .bind(command.assignment.as_uuid())
            .fetch_one(&mut *transaction)
            .await
            .map_err(map_sqlx_error)?;
            if !assignment_exists {
                return Err(StoreError::NotFound);
            }
            let rows = sqlx::query(
                "SELECT profile.roster_email_normalized, profile.roster_email_delivery, \
                    member.roster_id, profile.display_name, \
                    enrollment.enrollment_id AS entitlement_enrollment_id, \
                    summary.enrollment_id AS summary_enrollment_id, summary.current_score \
             FROM course_member member \
             JOIN course_roster_profile profile \
               ON profile.tenant_id = member.tenant_id \
              AND profile.course_id = member.course_id \
              AND profile.course_membership_id = member.course_membership_id \
        LEFT JOIN enrollment enrollment \
               ON enrollment.tenant_id = member.tenant_id \
              AND enrollment.student_id = member.student_id \
              AND enrollment.assignment_id = $3 \
        LEFT JOIN student_assignment_summary summary \
               ON summary.tenant_id = enrollment.tenant_id \
              AND summary.enrollment_id = enrollment.enrollment_id \
             WHERE member.tenant_id = $1 AND member.course_id = $2 \
               AND member.role = 'student' AND member.status = 'active' \
               AND profile.roster_email_normalized IS NOT NULL \
               AND profile.roster_email_delivery IS NOT NULL \
               AND member.roster_id IS NOT NULL \
               AND public.ple_course_records_accessible(member.tenant_id, member.course_id) \
             ORDER BY member.roster_id LIMIT $4",
            )
            .bind(tenant.as_uuid())
            .bind(command.course.as_uuid())
            .bind(command.assignment.as_uuid())
            .bind(i64::try_from(MAX_MANUAL_GRADE_EXPORT_ROWS + 1).expect("bounded export limit"))
            .fetch_all(&mut *transaction)
            .await
            .map_err(map_sqlx_error)?;
            if rows.len() > MAX_MANUAL_GRADE_EXPORT_ROWS {
                return Err(StoreError::InvalidRecord(
                    "manual grade export exceeds the row limit".to_string(),
                ));
            }
            let rows = rows
                .iter()
                .map(|row| {
                    let normalized: String = row
                        .try_get("roster_email_normalized")
                        .map_err(map_sqlx_error)?;
                    let delivery: String = row
                        .try_get("roster_email_delivery")
                        .map_err(map_sqlx_error)?;
                    let roster_email = AuthenticationEmail::parse(&delivery).map_err(|_| {
                        StoreError::Unavailable("stored roster email is invalid".to_string())
                    })?;
                    if roster_email.normalized() != normalized {
                        return Err(StoreError::Unavailable(
                            "stored roster email normalization is invalid".to_string(),
                        ));
                    }
                    let current_score = if row
                        .try_get::<Option<uuid::Uuid>, _>("entitlement_enrollment_id")
                        .map_err(map_sqlx_error)?
                        .is_some()
                    {
                        if row
                            .try_get::<Option<uuid::Uuid>, _>("summary_enrollment_id")
                            .map_err(map_sqlx_error)?
                            .is_none()
                        {
                            return Err(StoreError::Unavailable(
                                "entitlement receipt is missing its summary".to_string(),
                            ));
                        }
                        row.try_get("current_score").map_err(map_sqlx_error)?
                    } else {
                        None
                    };
                    Ok(ManualGradeExportRow {
                        roster_id: CourseRosterId::parse(
                            &row.try_get::<String, _>("roster_id")
                                .map_err(map_sqlx_error)?,
                        )
                        .map_err(|_| {
                            StoreError::Unavailable("stored roster ID is invalid".to_string())
                        })?,
                        roster_email,
                        display_name: row.try_get("display_name").map_err(map_sqlx_error)?,
                        current_score,
                    })
                })
                .collect::<Result<Vec<_>, StoreError>>()?;
            let id = ManualGradeExportId::generate()?;
            sqlx::query(
                "INSERT INTO course_grade_export_audit \
             (tenant_id, course_id, assignment_id, export_id, requested_by, row_count) \
             VALUES ($1, $2, $3, $4, $5, $6)",
            )
            .bind(tenant.as_uuid())
            .bind(command.course.as_uuid())
            .bind(command.assignment.as_uuid())
            .bind(id.as_uuid())
            .bind(actor.as_uuid())
            .bind(i32::try_from(rows.len()).expect("bounded export row count"))
            .execute(&mut *transaction)
            .await
            .map_err(map_sqlx_error)?;
            transaction.commit().await.map_err(map_sqlx_error)?;
            Ok(ManualGradeExport {
                id,
                course: command.course,
                assignment: command.assignment,
                rows,
            })
        })
        .await
    }
}

use async_trait::async_trait;

use super::*;

#[async_trait]
impl crate::StatisticsStore for PostgresStore {
    async fn question_statistics_impl(
        &self,
        context: TenantContext,
        reference: ProblemVersionRef,
    ) -> Result<QuestionStatisticsDisclosure, StoreError> {
        let mut transaction = self.begin_tenant(context).await?;
        let row = sqlx::query(
            "SELECT cohort_size, difficulty_index, attempts_mean, time_median_seconds_estimate, \
                    discrimination_index \
             FROM ple_question_statistics_view($1, $2)",
        )
        .bind(reference.problem.as_uuid())
        .bind(reference.version.as_uuid())
        .fetch_optional(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;
        transaction.commit().await.map_err(map_sqlx_error)?;
        question_statistics_disclosure_from_row(row.as_ref())
    }
    async fn list_gradebook_rows_impl(
        &self,
        context: TenantContext,
        course: CourseId,
        page: PageRequest,
    ) -> Result<Page<question_model::GradebookSummaryRow>, StoreError> {
        let cursor = page
            .after
            .as_ref()
            .map(GradebookCursor::decode)
            .transpose()?;
        let limit = i64::from(page.size.get()) + 1;
        let mut transaction = self.begin_tenant(context).await?;
        let course_exists: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM course WHERE tenant_id = $1 AND course_id = $2)",
        )
        .bind(context.tenant_id().as_uuid())
        .bind(course.as_uuid())
        .fetch_one(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;
        if !course_exists {
            return Err(StoreError::NotFound);
        }
        let rows = sqlx::query(GRADEBOOK_SUMMARY_PAGE_SQL)
            .bind(context.tenant_id().as_uuid())
            .bind(course.as_uuid())
            .bind(cursor.map(|value| value.assignment))
            .bind(cursor.map(|value| value.enrollment))
            .bind(limit)
            .fetch_all(&mut *transaction)
            .await
            .map_err(map_sqlx_error)?;
        let mut records = rows
            .iter()
            .map(|row| {
                let assignment_id =
                    AssignmentId::from_uuid(row.try_get("assignment_id").map_err(map_sqlx_error)?);
                let enrollment_id =
                    EnrollmentId::from_uuid(row.try_get("enrollment_id").map_err(map_sqlx_error)?);
                let summary: StudentAssignmentSummary = decode_payload_row(row)?;
                Ok((
                    GradebookCursor {
                        assignment: assignment_id.as_uuid(),
                        enrollment: enrollment_id.as_uuid(),
                    },
                    question_model::GradebookSummaryRow {
                        tenant: context.tenant_id(),
                        course_id: course,
                        enrollment_id,
                        student_id: question_model::StudentId::from_uuid(
                            row.try_get("student_id").map_err(map_sqlx_error)?,
                        ),
                        learner_name: row.try_get("learner_name").map_err(map_sqlx_error)?,
                        assignment_id,
                        assignment_title: row
                            .try_get("assignment_title")
                            .map_err(map_sqlx_error)?,
                        summary,
                    },
                ))
            })
            .collect::<Result<Vec<_>, StoreError>>()?;
        let result = gradebook_page_from_records(&mut records, page.size.get());
        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(result)
    }
}

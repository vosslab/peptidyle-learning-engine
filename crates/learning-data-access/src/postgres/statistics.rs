use async_trait::async_trait;

use super::*;

#[async_trait]
impl crate::StatisticsStore for PostgresStore {
    async fn question_statistics_impl(
        &self,
        reference: ProblemVersionRef,
    ) -> Result<QuestionStatisticsDisclosure, StoreError> {
        let mut transaction = self.begin_app().await?;
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
        actor: ActorContext,
        course: CourseId,
        page: PageRequest,
    ) -> Result<Page<question_model::GradebookSummaryRow>, StoreError> {
        let cursor = page
            .after
            .as_ref()
            .map(GradebookCursor::decode)
            .transpose()?;
        let limit = i64::from(page.size.get()) + 1;
        let mut transaction = self.begin_app().await?;
        let tenants = sqlx::query_scalar("SELECT tenant_id FROM course WHERE course_id = $1")
            .bind(course.as_uuid())
            .fetch_all(&mut *transaction)
            .await
            .map_err(map_sqlx_error)?;
        let [tenant] = tenants.as_slice() else {
            return Err(StoreError::NotFound);
        };
        let role = sqlx::query_scalar::<_, String>(
            "SELECT role FROM course_member \
             WHERE tenant_id = $1 AND course_id = $2 AND user_id = $3 \
               AND status = 'active'",
        )
        .bind(*tenant)
        .bind(course.as_uuid())
        .bind(actor.user_id().as_uuid())
        .fetch_optional(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;
        match role.as_deref() {
            Some("instructor") => {}
            Some("student") => return Err(StoreError::Forbidden),
            Some(_) | None => return Err(StoreError::NotFound),
        }
        let rows = sqlx::query(GRADEBOOK_SUMMARY_PAGE_SQL)
            .bind(*tenant)
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
                let summary = decode_summary_row_named(row, "summary_")?;
                Ok((
                    GradebookCursor {
                        assignment: assignment_id.as_uuid(),
                        enrollment: enrollment_id.as_uuid(),
                    },
                    question_model::GradebookSummaryRow {
                        course_id: course,
                        enrollment_id,
                        student_id: question_model::StudentId::from_uuid(
                            row.try_get("student_id").map_err(map_sqlx_error)?,
                        ),
                        student_name: row.try_get("student_name").map_err(map_sqlx_error)?,
                        assignment_id,
                        assignment_title: row
                            .try_get("assignment_title")
                            .map_err(map_sqlx_error)?,
                        scoring_status: decode_scoring_status(row)?,
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

use async_trait::async_trait;

use super::*;

#[async_trait]
impl crate::ActivityStore for PostgresStore {
    async fn apply_activity_transition_impl(
        &self,
        context: TenantContext,
        transition: ActivityTransition,
    ) -> Result<StudentAssignmentSummary, StoreError> {
        let mut transaction = self.begin_tenant(context).await?;
        let next = match transition {
            ActivityTransition::StartRun { run } => {
                apply_start_run(&mut transaction, context, run).await?
            }
            ActivityTransition::RecordQuestionAttempt { attempt } => {
                apply_question_attempt(&mut transaction, context, *attempt).await?
            }
            ActivityTransition::CompleteRun { run, score, at } => {
                apply_complete_run(&mut transaction, context, run, score, at).await?
            }
        };
        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(next)
    }
    async fn get_run_impl(
        &self,
        context: TenantContext,
        run: RunId,
    ) -> Result<Option<AssignmentRun>, StoreError> {
        let mut transaction = self.begin_tenant(context).await?;
        let row = sqlx::query(
            "SELECT payload, payload_sha256 FROM assignment_run \
             WHERE tenant_id = $1 AND run_id = $2",
        )
        .bind(context.tenant_id().as_uuid())
        .bind(run.as_uuid())
        .fetch_optional(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;
        let record = row.as_ref().map(decode_payload_row).transpose()?;
        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(record)
    }
    async fn list_runs_impl(
        &self,
        context: TenantContext,
        enrollment: EnrollmentId,
        page: PageRequest,
    ) -> Result<Page<AssignmentRun>, StoreError> {
        let cursor = page.after.as_ref().map(|value| value.as_str().to_string());
        let limit = i64::from(page.size.get()) + 1;
        let mut transaction = self.begin_tenant(context).await?;
        let enrollment_exists: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM enrollment \
             WHERE tenant_id = $1 AND enrollment_id = $2)",
        )
        .bind(context.tenant_id().as_uuid())
        .bind(enrollment.as_uuid())
        .fetch_one(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;
        if !enrollment_exists {
            return Err(StoreError::NotFound);
        }
        let rows = sqlx::query(
            "SELECT lpad(run_number::text, 10, '0') || '/' || run_id::text AS stable_key, \
                    payload, payload_sha256 \
             FROM assignment_run \
             WHERE tenant_id = $1 AND enrollment_id = $2 \
               AND ($3::text IS NULL \
                    OR lpad(run_number::text, 10, '0') || '/' || run_id::text > $3) \
             ORDER BY run_number, run_id::text LIMIT $4",
        )
        .bind(context.tenant_id().as_uuid())
        .bind(enrollment.as_uuid())
        .bind(cursor)
        .bind(limit)
        .fetch_all(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;
        let result = page_from_rows(rows, page.size.get())?;
        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(result)
    }
    async fn get_question_attempt_impl(
        &self,
        context: TenantContext,
        attempt: QuestionAttemptId,
    ) -> Result<Option<QuestionAttempt>, StoreError> {
        let mut transaction = self.begin_tenant(context).await?;
        let row = sqlx::query(
            "SELECT COALESCE(si.payload, qa.payload) AS payload, \
                    COALESCE(si.payload_sha256, qa.payload_sha256) AS payload_sha256, \
                    evaluation.payload AS evaluation_payload, \
                    evaluation.payload_sha256 AS evaluation_payload_sha256, \
                    evaluation.grading_status AS evaluation_grading_status, \
                    qa.attempt_status AS current_attempt_status, \
                    floor(extract(epoch FROM qa.submitted_at) * 1000)::bigint \
                        AS current_submitted_at, \
                    floor(extract(epoch FROM timing.effective_deadline) * 1000)::bigint \
                        AS current_deadline_at \
             FROM question_attempt AS qa \
             LEFT JOIN submission_idempotency AS si \
               ON si.tenant_id = qa.tenant_id AND si.attempt_id = qa.attempt_id \
             LEFT JOIN submission_evaluation AS evaluation \
               ON evaluation.tenant_id = qa.tenant_id AND evaluation.attempt_id = qa.attempt_id \
             LEFT JOIN attempt_timing_current AS timing \
               ON timing.tenant_id = qa.tenant_id AND timing.attempt_id = qa.attempt_id \
             WHERE qa.tenant_id = $1 AND qa.attempt_id = $2 \
             ORDER BY qa.occurred_at LIMIT 1",
        )
        .bind(context.tenant_id().as_uuid())
        .bind(attempt.as_uuid())
        .fetch_optional(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;
        let record = row
            .as_ref()
            .map(decode_current_attempt_with_evaluation_row)
            .transpose()?;
        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(record)
    }
    async fn get_summary_impl(
        &self,
        context: TenantContext,
        enrollment: EnrollmentId,
    ) -> Result<Option<StudentAssignmentSummary>, StoreError> {
        let mut transaction = self.begin_tenant(context).await?;
        let row = sqlx::query(
            "SELECT payload, payload_sha256 FROM student_assignment_summary \
             WHERE tenant_id = $1 AND enrollment_id = $2",
        )
        .bind(context.tenant_id().as_uuid())
        .bind(enrollment.as_uuid())
        .fetch_optional(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;
        let record = row.as_ref().map(decode_payload_row).transpose()?;
        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(record)
    }
}

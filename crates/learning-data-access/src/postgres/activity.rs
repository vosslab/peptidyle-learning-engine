use async_trait::async_trait;

use super::*;

async fn learner_enrollment_for_update(
    transaction: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    tenant: TenantId,
    actor: UserId,
    enrollment_id: EnrollmentId,
) -> Result<Option<AssignmentEnrollment>, StoreError> {
    let enrollment = match load_enrollment_for_update(transaction, tenant, enrollment_id).await {
        Ok(value) => value,
        Err(StoreError::NotFound) => return Ok(None),
        Err(error) => return Err(error),
    };
    if enrollment.user != actor {
        return Ok(None);
    }
    let course_accessible: bool = sqlx::query_scalar(
        "SELECT public.ple_course_records_accessible(a.tenant_id, a.course_id) \
         FROM assignment AS a WHERE a.tenant_id = $1 AND a.assignment_id = $2",
    )
    .bind(tenant.as_uuid())
    .bind(enrollment.assignment.as_uuid())
    .fetch_optional(&mut **transaction)
    .await
    .map_err(map_sqlx_error)?
    .unwrap_or(false);
    if !course_accessible {
        return Err(StoreError::NotFound);
    }
    transaction_context::require_active_learner_membership(
        transaction,
        tenant,
        enrollment.assignment,
        actor,
    )
    .await?;
    Ok(Some(enrollment))
}

#[async_trait]
impl crate::ActivityStore for PostgresStore {
    async fn instructor_get_enrollment_impl(
        &self,
        context: TenantContext,
        actor: UserId,
        enrollment: EnrollmentId,
    ) -> Result<Option<AssignmentEnrollment>, StoreError> {
        let mut transaction = self.begin_tenant(context).await?;
        let record =
            match load_enrollment_for_update(&mut transaction, context.tenant_id(), enrollment)
                .await
            {
                Ok(record) => record,
                Err(StoreError::NotFound) => return Ok(None),
                Err(error) => return Err(error),
            };
        let accessible: bool = sqlx::query_scalar(
            "SELECT public.ple_course_records_accessible(a.tenant_id, a.course_id) \
             FROM assignment AS a WHERE a.tenant_id = $1 AND a.assignment_id = $2",
        )
        .bind(context.tenant_id().as_uuid())
        .bind(record.assignment.as_uuid())
        .fetch_optional(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?
        .unwrap_or(false);
        if !accessible {
            return Err(StoreError::NotFound);
        }
        let assignment =
            load_assignment(&mut transaction, context.tenant_id(), record.assignment).await?;
        let instructor = postgres_is_course_instructor(
            &mut transaction,
            context.tenant_id(),
            assignment.course_id,
            actor,
        )
        .await?;
        transaction.commit().await.map_err(map_sqlx_error)?;
        if instructor {
            Ok(Some(record))
        } else {
            Ok(None)
        }
    }
    async fn learner_get_enrollment_impl(
        &self,
        context: TenantContext,
        actor: UserId,
        enrollment: EnrollmentId,
    ) -> Result<Option<AssignmentEnrollment>, StoreError> {
        let mut transaction = self.begin_tenant(context).await?;
        let record =
            learner_enrollment_for_update(&mut transaction, context.tenant_id(), actor, enrollment)
                .await?;
        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(record)
    }
    async fn learner_get_run_impl(
        &self,
        context: TenantContext,
        actor: UserId,
        run: RunId,
    ) -> Result<Option<AssignmentRun>, StoreError> {
        let mut transaction = self.begin_tenant(context).await?;
        let run = load_run_for_update(&mut transaction, context.tenant_id(), run).await;
        let Ok(run) = run else {
            return match run {
                Err(StoreError::NotFound) => Ok(None),
                Err(error) => Err(error),
                Ok(_) => unreachable!(),
            };
        };
        let enrollment =
            load_enrollment_for_update(&mut transaction, context.tenant_id(), run.enrollment)
                .await?;
        if enrollment.user != actor {
            return Ok(None);
        }
        transaction_context::require_active_learner_membership(
            &mut transaction,
            context.tenant_id(),
            enrollment.assignment,
            actor,
        )
        .await?;
        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(Some(run))
    }
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
    async fn learner_list_runs_impl(
        &self,
        context: TenantContext,
        actor: UserId,
        enrollment: EnrollmentId,
        page: PageRequest,
    ) -> Result<Option<Page<AssignmentRun>>, StoreError> {
        let cursor = page.after.as_ref().map(|value| value.as_str().to_string());
        let limit = i64::from(page.size.get()) + 1;
        let mut transaction = self.begin_tenant(context).await?;
        if learner_enrollment_for_update(&mut transaction, context.tenant_id(), actor, enrollment)
            .await?
            .is_none()
        {
            transaction.commit().await.map_err(map_sqlx_error)?;
            return Ok(None);
        }
        let rows = sqlx::query("SELECT lpad(run_number::text, 10, '0') || '/' || run_id::text AS stable_key, payload, payload_sha256 FROM assignment_run WHERE tenant_id = $1 AND enrollment_id = $2 AND ($3::text IS NULL OR lpad(run_number::text, 10, '0') || '/' || run_id::text > $3) ORDER BY run_number, run_id::text LIMIT $4")
            .bind(context.tenant_id().as_uuid()).bind(enrollment.as_uuid()).bind(cursor).bind(limit).fetch_all(&mut *transaction).await.map_err(map_sqlx_error)?;
        let result = page_from_rows(rows, page.size.get())?;
        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(Some(result))
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
    async fn learner_get_question_attempt_impl(
        &self,
        context: TenantContext,
        actor: UserId,
        attempt: QuestionAttemptId,
    ) -> Result<Option<QuestionAttempt>, StoreError> {
        let mut transaction = self.begin_tenant(context).await?;
        let run_id: Option<Uuid> = sqlx::query_scalar("SELECT run_id FROM question_attempt WHERE tenant_id = $1 AND attempt_id = $2 FOR UPDATE").bind(context.tenant_id().as_uuid()).bind(attempt.as_uuid()).fetch_optional(&mut *transaction).await.map_err(map_sqlx_error)?;
        let Some(run_id) = run_id else {
            transaction.commit().await.map_err(map_sqlx_error)?;
            return Ok(None);
        };
        let run = load_run_for_update(
            &mut transaction,
            context.tenant_id(),
            RunId::from_uuid(run_id),
        )
        .await?;
        if learner_enrollment_for_update(
            &mut transaction,
            context.tenant_id(),
            actor,
            run.enrollment,
        )
        .await?
        .is_none()
        {
            transaction.commit().await.map_err(map_sqlx_error)?;
            return Ok(None);
        }
        let row = sqlx::query("SELECT COALESCE(si.payload, qa.payload) AS payload, COALESCE(si.payload_sha256, qa.payload_sha256) AS payload_sha256, evaluation.payload AS evaluation_payload, evaluation.payload_sha256 AS evaluation_payload_sha256, evaluation.grading_status AS evaluation_grading_status, qa.attempt_status AS current_attempt_status, floor(extract(epoch FROM qa.submitted_at) * 1000)::bigint AS current_submitted_at, floor(extract(epoch FROM timing.effective_deadline) * 1000)::bigint AS current_deadline_at FROM question_attempt qa LEFT JOIN submission_idempotency si ON si.tenant_id = qa.tenant_id AND si.attempt_id = qa.attempt_id LEFT JOIN submission_evaluation evaluation ON evaluation.tenant_id = qa.tenant_id AND evaluation.attempt_id = qa.attempt_id LEFT JOIN attempt_timing_current timing ON timing.tenant_id = qa.tenant_id AND timing.attempt_id = qa.attempt_id WHERE qa.tenant_id = $1 AND qa.attempt_id = $2 ORDER BY qa.occurred_at LIMIT 1").bind(context.tenant_id().as_uuid()).bind(attempt.as_uuid()).fetch_optional(&mut *transaction).await.map_err(map_sqlx_error)?;
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
    async fn learner_get_summary_impl(
        &self,
        context: TenantContext,
        actor: UserId,
        enrollment: EnrollmentId,
    ) -> Result<Option<StudentAssignmentSummary>, StoreError> {
        let mut transaction = self.begin_tenant(context).await?;
        if learner_enrollment_for_update(&mut transaction, context.tenant_id(), actor, enrollment)
            .await?
            .is_none()
        {
            transaction.commit().await.map_err(map_sqlx_error)?;
            return Ok(None);
        }
        let row = sqlx::query("SELECT payload, payload_sha256 FROM student_assignment_summary WHERE tenant_id = $1 AND enrollment_id = $2").bind(context.tenant_id().as_uuid()).bind(enrollment.as_uuid()).fetch_optional(&mut *transaction).await.map_err(map_sqlx_error)?;
        let record = row.as_ref().map(decode_payload_row).transpose()?;
        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(record)
    }
}

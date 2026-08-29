use async_trait::async_trait;

use super::*;

/// Authorizes a learner-owned enrollment for a projection. Mutation brokers
/// own serialization; browser reads must not acquire locks on their source
/// membership, audience, enrollment, or run rows.
async fn learner_enrollment_for_read(
    transaction: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    tenant: TenantId,
    actor: UserId,
    enrollment_id: EnrollmentId,
) -> Result<Option<AssignmentEnrollment>, StoreError> {
    let enrollment = match load_postgres_enrollment(transaction, tenant, enrollment_id).await {
        Ok(value) => value,
        Err(StoreError::NotFound) => return Ok(None),
        Err(error) => return Err(error),
    };
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
    let course: CourseId = sqlx::query_scalar(
        "SELECT course_id FROM assignment WHERE tenant_id = $1 AND assignment_id = $2",
    )
    .bind(tenant.as_uuid())
    .bind(enrollment.assignment.as_uuid())
    .fetch_optional(&mut **transaction)
    .await
    .map_err(map_sqlx_error)?
    .map(CourseId::from_uuid)
    .ok_or(StoreError::NotFound)?;
    let decision = super::entitlement::evaluate_current_read_only(
        transaction,
        tenant,
        actor,
        course,
        enrollment.assignment,
    )
    .await?;
    if !matches!(decision, domain::entitlement::EntitlementDecision::Granted(ref grant) if grant.student() == enrollment.student)
    {
        return Ok(None);
    }
    Ok(Some(enrollment))
}

async fn learner_enrollment_for_assignment_read(
    transaction: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    tenant: TenantId,
    actor: UserId,
    assignment: AssignmentId,
) -> Result<Option<AssignmentEnrollment>, StoreError> {
    let course = sqlx::query_scalar::<_, Uuid>(
        "SELECT course_id FROM assignment WHERE tenant_id = $1 AND assignment_id = $2",
    )
    .bind(tenant.as_uuid())
    .bind(assignment.as_uuid())
    .fetch_optional(&mut **transaction)
    .await
    .map_err(map_sqlx_error)?;
    let Some(course) = course.map(CourseId::from_uuid) else {
        return Ok(None);
    };
    let decision = super::entitlement::evaluate_current_read_only(
        transaction,
        tenant,
        actor,
        course,
        assignment,
    )
    .await?;
    let domain::entitlement::EntitlementDecision::Granted(grant) = decision else {
        return Ok(None);
    };
    let enrollment_id = sqlx::query_scalar::<_, Uuid>(
        "SELECT enrollment_id FROM enrollment \
         WHERE tenant_id = $1 AND assignment_id = $2 AND student_id = $3",
    )
    .bind(tenant.as_uuid())
    .bind(assignment.as_uuid())
    .bind(grant.student().as_uuid())
    .fetch_optional(&mut **transaction)
    .await
    .map_err(map_sqlx_error)?;
    let Some(enrollment_id) = enrollment_id else {
        return Ok(None);
    };
    let enrollment =
        load_postgres_enrollment(transaction, tenant, EnrollmentId::from_uuid(enrollment_id))
            .await?;
    let course_accessible: bool = sqlx::query_scalar(
        "SELECT public.ple_course_records_accessible(a.tenant_id, a.course_id) \
         FROM assignment AS a WHERE a.tenant_id = $1 AND a.assignment_id = $2",
    )
    .bind(tenant.as_uuid())
    .bind(assignment.as_uuid())
    .fetch_optional(&mut **transaction)
    .await
    .map_err(map_sqlx_error)?
    .unwrap_or(false);
    if !course_accessible {
        return Err(StoreError::NotFound);
    }
    Ok(Some(enrollment))
}

async fn run_page(
    transaction: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    tenant: TenantId,
    enrollment: EnrollmentId,
    page: &PageRequest,
) -> Result<Page<AssignmentRun>, StoreError> {
    let cursor = page.after.as_ref().map(|value| value.as_str().to_string());
    let limit = i64::from(page.size.get()) + 1;
    let rows = sqlx::query(
        "SELECT lpad(run_number::text, 10, '0') || '/' || run_id::text AS stable_key, \
                payload, payload_sha256 \
         FROM assignment_run \
         WHERE tenant_id = $1 AND enrollment_id = $2 \
           AND ($3::text IS NULL \
                OR lpad(run_number::text, 10, '0') || '/' || run_id::text > $3) \
         ORDER BY run_number, run_id::text LIMIT $4",
    )
    .bind(tenant.as_uuid())
    .bind(enrollment.as_uuid())
    .bind(cursor)
    .bind(limit)
    .fetch_all(&mut **transaction)
    .await
    .map_err(map_sqlx_error)?;
    page_from_rows(rows, page.size.get())
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
            match load_postgres_enrollment(&mut transaction, context.tenant_id(), enrollment).await
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
    async fn student_get_enrollment_impl(
        &self,
        context: TenantContext,
        actor: UserId,
        enrollment: EnrollmentId,
    ) -> Result<Option<AssignmentEnrollment>, StoreError> {
        let mut transaction = self.begin_tenant(context).await?;
        let record =
            learner_enrollment_for_read(&mut transaction, context.tenant_id(), actor, enrollment)
                .await?;
        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(record)
    }
    async fn student_get_enrollment_for_assignment_impl(
        &self,
        context: TenantContext,
        actor: UserId,
        assignment: AssignmentId,
    ) -> Result<Option<AssignmentEnrollment>, StoreError> {
        let mut transaction = self.begin_tenant(context).await?;
        let record = learner_enrollment_for_assignment_read(
            &mut transaction,
            context.tenant_id(),
            actor,
            assignment,
        )
        .await?;
        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(record)
    }
    async fn student_get_run_impl(
        &self,
        context: TenantContext,
        actor: UserId,
        run: RunId,
    ) -> Result<Option<AssignmentRun>, StoreError> {
        let mut transaction = self.begin_tenant(context).await?;
        let run = load_postgres_run(&mut transaction, context.tenant_id(), run).await;
        let Ok(run) = run else {
            return match run {
                Err(StoreError::NotFound) => Ok(None),
                Err(error) => Err(error),
                Ok(_) => unreachable!(),
            };
        };
        if learner_enrollment_for_read(&mut transaction, context.tenant_id(), actor, run.enrollment)
            .await?
            .is_none()
        {
            return Ok(None);
        }
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
        let result = run_page(&mut transaction, context.tenant_id(), enrollment, &page).await?;
        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(result)
    }
    async fn instructor_list_runs_impl(
        &self,
        context: TenantContext,
        actor: UserId,
        enrollment: EnrollmentId,
        page: PageRequest,
    ) -> Result<Option<Page<AssignmentRun>>, StoreError> {
        let mut transaction = self.begin_tenant(context).await?;
        let record =
            match load_postgres_enrollment(&mut transaction, context.tenant_id(), enrollment).await
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
        if matches!(
            super::entitlement::evaluate_current_read_only(
                &mut transaction,
                context.tenant_id(),
                actor,
                assignment.course_id,
                assignment.id,
            )
            .await?,
            domain::entitlement::EntitlementDecision::Granted(ref grant)
                if grant.student() == record.student
        ) {
            transaction.commit().await.map_err(map_sqlx_error)?;
            return Ok(None);
        }
        let instructor = postgres_is_course_instructor(
            &mut transaction,
            context.tenant_id(),
            assignment.course_id,
            actor,
        )
        .await?;
        if !instructor {
            transaction.commit().await.map_err(map_sqlx_error)?;
            return Ok(None);
        }
        let result = run_page(&mut transaction, context.tenant_id(), enrollment, &page).await?;
        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(Some(result))
    }
    async fn student_list_runs_impl(
        &self,
        context: TenantContext,
        actor: UserId,
        enrollment: EnrollmentId,
        page: PageRequest,
    ) -> Result<Option<Page<AssignmentRun>>, StoreError> {
        let mut transaction = self.begin_tenant(context).await?;
        if learner_enrollment_for_read(&mut transaction, context.tenant_id(), actor, enrollment)
            .await?
            .is_none()
        {
            transaction.commit().await.map_err(map_sqlx_error)?;
            return Ok(None);
        }
        let result = run_page(&mut transaction, context.tenant_id(), enrollment, &page).await?;
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
            "SELECT CASE WHEN si.request_contract_version = 2 THEN qa.payload ELSE COALESCE(si.payload, qa.payload) END AS payload, \
                    CASE WHEN si.request_contract_version = 2 THEN qa.payload_sha256 ELSE COALESCE(si.payload_sha256, qa.payload_sha256) END AS payload_sha256, \
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
             LEFT JOIN attempt_effective_policy_current AS current_effect ON current_effect.tenant_id=qa.tenant_id AND current_effect.attempt_id=qa.attempt_id \
             LEFT JOIN attempt_effective_policy_receipt AS timing ON timing.tenant_id=current_effect.tenant_id AND timing.attempt_id=current_effect.attempt_id AND timing.receipt_generation=current_effect.receipt_generation \
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
    async fn student_get_question_attempt_impl(
        &self,
        context: TenantContext,
        actor: UserId,
        attempt: QuestionAttemptId,
    ) -> Result<Option<QuestionAttempt>, StoreError> {
        let mut transaction = self.begin_tenant(context).await?;
        let run_id: Option<Uuid> = sqlx::query_scalar(
            "SELECT run_id FROM question_attempt WHERE tenant_id = $1 AND attempt_id = $2",
        )
        .bind(context.tenant_id().as_uuid())
        .bind(attempt.as_uuid())
        .fetch_optional(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;
        let Some(run_id) = run_id else {
            transaction.commit().await.map_err(map_sqlx_error)?;
            return Ok(None);
        };
        let run = load_postgres_run(
            &mut transaction,
            context.tenant_id(),
            RunId::from_uuid(run_id),
        )
        .await?;
        if learner_enrollment_for_read(&mut transaction, context.tenant_id(), actor, run.enrollment)
            .await?
            .is_none()
        {
            transaction.commit().await.map_err(map_sqlx_error)?;
            return Ok(None);
        }
        let row = sqlx::query("SELECT CASE WHEN si.request_contract_version = 2 THEN qa.payload ELSE COALESCE(si.payload, qa.payload) END AS payload, CASE WHEN si.request_contract_version = 2 THEN qa.payload_sha256 ELSE COALESCE(si.payload_sha256, qa.payload_sha256) END AS payload_sha256, evaluation.payload AS evaluation_payload, evaluation.payload_sha256 AS evaluation_payload_sha256, evaluation.grading_status AS evaluation_grading_status, qa.attempt_status AS current_attempt_status, floor(extract(epoch FROM qa.submitted_at) * 1000)::bigint AS current_submitted_at, floor(extract(epoch FROM timing.effective_deadline) * 1000)::bigint AS current_deadline_at FROM question_attempt qa LEFT JOIN submission_idempotency si ON si.tenant_id = qa.tenant_id AND si.attempt_id = qa.attempt_id LEFT JOIN submission_evaluation evaluation ON evaluation.tenant_id = qa.tenant_id AND evaluation.attempt_id = qa.attempt_id LEFT JOIN attempt_effective_policy_current current_effect ON current_effect.tenant_id=qa.tenant_id AND current_effect.attempt_id=qa.attempt_id LEFT JOIN attempt_effective_policy_receipt timing ON timing.tenant_id=current_effect.tenant_id AND timing.attempt_id=current_effect.attempt_id AND timing.receipt_generation=current_effect.receipt_generation WHERE qa.tenant_id = $1 AND qa.attempt_id = $2 ORDER BY qa.occurred_at LIMIT 1").bind(context.tenant_id().as_uuid()).bind(attempt.as_uuid()).fetch_optional(&mut *transaction).await.map_err(map_sqlx_error)?;
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
            "SELECT tenant_id, enrollment_id, current_score, best_score, latest_score, \
                    completed_run_count, total_question_attempts, \
                    floor(extract(epoch FROM last_activity_at) * 1000)::bigint \
                        AS last_activity_at_millis \
             FROM student_assignment_summary \
             WHERE tenant_id = $1 AND enrollment_id = $2",
        )
        .bind(context.tenant_id().as_uuid())
        .bind(enrollment.as_uuid())
        .fetch_optional(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;
        let record = row.as_ref().map(decode_summary_row).transpose()?;
        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(record)
    }
    async fn student_get_summary_impl(
        &self,
        context: TenantContext,
        actor: UserId,
        enrollment: EnrollmentId,
    ) -> Result<Option<crate::StudentAssignmentSummarySnapshot>, StoreError> {
        let mut transaction = self.begin_tenant(context).await?;
        if learner_enrollment_for_read(&mut transaction, context.tenant_id(), actor, enrollment)
            .await?
            .is_none()
        {
            transaction.commit().await.map_err(map_sqlx_error)?;
            return Ok(None);
        }
        let row = sqlx::query("SELECT summary.tenant_id, summary.enrollment_id, summary.current_score, summary.best_score, summary.latest_score, summary.completed_run_count, summary.total_question_attempts, floor(extract(epoch FROM summary.last_activity_at) * 1000)::bigint AS last_activity_at_millis, assignment.scoring_status FROM student_assignment_summary AS summary JOIN enrollment ON enrollment.tenant_id=summary.tenant_id AND enrollment.enrollment_id=summary.enrollment_id JOIN assignment ON assignment.tenant_id=enrollment.tenant_id AND assignment.assignment_id=enrollment.assignment_id WHERE summary.tenant_id = $1 AND summary.enrollment_id = $2").bind(context.tenant_id().as_uuid()).bind(enrollment.as_uuid()).fetch_optional(&mut *transaction).await.map_err(map_sqlx_error)?;
        let record = row
            .as_ref()
            .map(|value| {
                Ok::<_, StoreError>(crate::StudentAssignmentSummarySnapshot {
                    summary: decode_summary_row(value)?,
                    scoring_status: decode_scoring_status(value)?,
                })
            })
            .transpose()?;
        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(record)
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn activity_projection_capabilities_do_not_request_mutation_locks() {
        // ActivityStore exposes reads plus the explicit transition writer.
        // Transition serialization lives below its brokered write path; the
        // projection module must remain usable by the SELECT-only app role.
        let source = include_str!("activity.rs")
            .split("#[cfg(test)]")
            .next()
            .expect("production activity source precedes its tests");
        assert!(!source.contains("FOR UPDATE"));
        assert!(!source.contains("load_enrollment_for_update"));
        assert!(!source.contains("load_run_for_update"));
        assert!(!source.contains("entitlement::evaluate_current("));
    }
}

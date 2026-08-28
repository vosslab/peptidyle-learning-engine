//! PostgreSQL worker jobs, scoring workers, and assignment exports.

use super::*;

#[cfg(feature = "postgres")]
#[async_trait]
impl JobStore for PostgresStore {
    async fn enqueue_job(
        &self,
        context: TenantContext,
        job: EnqueueJob,
    ) -> Result<JobId, StoreError> {
        ensure_tenant(context, job.tenant)?;
        job.validate()?;
        let id = JobId::generate()?;
        let payload = serde_json::to_value(&job.payload).map_err(|error| {
            StoreError::InvalidRecord(format!("job payload serialization failed: {error}"))
        })?;
        let mut transaction = self.begin_tenant(context).await?;
        sqlx::query(
            "INSERT INTO worker_job (job_id, tenant_id, payload, state, max_attempts) \
             VALUES ($1, $2, $3, 'ready', $4)",
        )
        .bind(id.as_uuid())
        .bind(job.tenant.as_uuid())
        .bind(payload)
        .bind(i32::from(job.max_attempts))
        .execute(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;
        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(id)
    }

    async fn claim_next_job(
        &self,
        filter: &crate::JobClaimFilter,
        lease: JobLeaseDuration,
    ) -> Result<Option<ClaimedJob>, StoreError> {
        let token = JobLeaseToken::generate()?;
        let mut transaction = self.begin_app().await?;
        let row = sqlx::query(
            "SELECT job_id, tenant_id, payload, lease_token, attempt_count \
             FROM ple_claim_worker_job($1, $2, $3)",
        )
        .bind(token.as_uuid())
        .bind(lease.seconds())
        .bind(filter.database_names())
        .fetch_optional(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;
        let claimed = row
            .as_ref()
            .map(|row| decode_claimed_job(row, token))
            .transpose()?;
        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(claimed)
    }

    async fn complete_job(&self, id: JobId, token: JobLeaseToken) -> Result<(), StoreError> {
        let mut transaction = self.begin_app().await?;
        let completed: bool = sqlx::query_scalar("SELECT ple_complete_worker_job($1, $2)")
            .bind(id.as_uuid())
            .bind(token.as_uuid())
            .fetch_one(&mut *transaction)
            .await
            .map_err(map_sqlx_error)?;
        if !completed {
            return Err(StoreError::Conflict);
        }
        transaction.commit().await.map_err(map_sqlx_error)
    }

    async fn fail_job(
        &self,
        id: JobId,
        token: JobLeaseToken,
        failure: JobFailureKind,
    ) -> Result<JobFailureDisposition, StoreError> {
        let mut transaction = self.begin_app().await?;
        let row = sqlx::query("SELECT ple_fail_worker_job($1, $2, $3) AS disposition")
            .bind(id.as_uuid())
            .bind(token.as_uuid())
            .bind(failure.as_db())
            .fetch_one(&mut *transaction)
            .await
            .map_err(map_sqlx_error)?;
        let disposition: Option<String> = row.try_get("disposition").map_err(map_sqlx_error)?;
        let result = match disposition.as_deref() {
            Some("retrying") => JobFailureDisposition::Retrying,
            Some("dead") => JobFailureDisposition::Dead,
            None => return Err(StoreError::Conflict),
            Some(_) => {
                return Err(StoreError::Unavailable(
                    "queue broker returned an unknown failure disposition".to_string(),
                ));
            }
        };
        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(result)
    }

    async fn get_job(
        &self,
        context: TenantContext,
        id: JobId,
    ) -> Result<Option<TenantJobView>, StoreError> {
        let mut transaction = self.begin_tenant(context).await?;
        let row =
            sqlx::query("SELECT payload, state, attempt_count FROM worker_job WHERE job_id = $1")
                .bind(id.as_uuid())
                .fetch_optional(&mut *transaction)
                .await
                .map_err(map_sqlx_error)?;
        let view = row
            .as_ref()
            .map(|row| decode_tenant_job_view(row, id))
            .transpose()?;
        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(view)
    }

    async fn ready_queue_depth(
        &self,
        filter: &crate::JobClaimFilter,
    ) -> Result<QueueDepth, StoreError> {
        let mut transaction = self.begin_app().await?;
        let ready: i64 = sqlx::query_scalar("SELECT ple_ready_worker_queue_depth($1)")
            .bind(filter.database_names())
            .fetch_one(&mut *transaction)
            .await
            .map_err(map_sqlx_error)?;
        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(QueueDepth {
            ready: u64::try_from(ready).map_err(|_| {
                StoreError::Unavailable("queue broker returned a negative depth".to_string())
            })?,
        })
    }
}

#[cfg(feature = "postgres")]
#[async_trait]
impl crate::AttemptAutoSubmitWorkerStore for PostgresStore {
    async fn commit_attempt_auto_submit(
        &self,
        context: TenantContext,
        command: crate::AttemptAutoSubmitWorkerCommand,
    ) -> Result<crate::AttemptAutoSubmitCommitOutcome, StoreError> {
        let tenant = context.tenant_id();
        let expected_payload = serde_json::to_value(JobPayload::AutoSubmitAttempt {
            attempt: command.attempt,
            timing_generation: command.timing_generation,
        })
        .map_err(|error| StoreError::InvalidRecord(error.to_string()))?;
        let mut transaction = self.begin_tenant(context).await?;
        let claim_active: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM worker_job \
             WHERE job_id = $1 AND tenant_id = $2 AND state = 'leased' \
               AND lease_token = $3 AND lease_expires_at > transaction_timestamp() \
               AND payload = $4)",
        )
        .bind(command.job.as_uuid())
        .bind(tenant.as_uuid())
        .bind(command.lease.as_uuid())
        .bind(expected_payload)
        .fetch_one(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;
        if !claim_active {
            transaction.commit().await.map_err(map_sqlx_error)?;
            return Ok(crate::AttemptAutoSubmitCommitOutcome::ClaimNoLongerActive);
        }
        let attempt =
            load_attempt_for_external_update(&mut transaction, tenant, command.attempt).await?;
        let timing_row = sqlx::query(
            "SELECT timing_generation, job_id, effective_grace_seconds, \
                    floor(extract(epoch FROM effective_deadline) * 1000)::bigint \
                        AS effective_deadline_millis, \
                    floor(extract(epoch FROM auto_submit_at) * 1000)::bigint \
                        AS auto_submit_at_millis \
             FROM attempt_effective_policy_current current_effect \
             JOIN attempt_effective_policy_receipt receipt ON receipt.tenant_id=current_effect.tenant_id AND receipt.attempt_id=current_effect.attempt_id AND receipt.receipt_generation=current_effect.receipt_generation \
             WHERE current_effect.tenant_id = $1 AND current_effect.attempt_id = $2 FOR UPDATE OF current_effect",
        )
        .bind(tenant.as_uuid())
        .bind(command.attempt.as_uuid())
        .fetch_optional(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;
        let mapped_job = timing_row
            .as_ref()
            .map(|row| row.try_get::<Option<Uuid>, _>("job_id"))
            .transpose()
            .map_err(map_sqlx_error)?
            .flatten();
        if attempt.status != AttemptStatus::InProgress
            || timing_row.is_none()
            || mapped_job != Some(command.job.as_uuid())
        {
            complete_postgres_claimed_job(&mut transaction, command.job, command.lease).await?;
            transaction.commit().await.map_err(map_sqlx_error)?;
            return Ok(crate::AttemptAutoSubmitCommitOutcome::Superseded);
        }
        let timing_row = timing_row.expect("current mapping has a timing row");
        let generation = u64::try_from(
            timing_row
                .try_get::<i64, _>("timing_generation")
                .map_err(map_sqlx_error)?,
        )
        .map_err(|_| StoreError::Unavailable("stored timing generation is invalid".to_string()))?;
        let auto_submit_at = timing_row
            .try_get::<Option<i64>, _>("auto_submit_at_millis")
            .map_err(map_sqlx_error)?
            .map(ActivityTimestamp::from_unix_millis);
        let Some(auto_submit_at) = auto_submit_at else {
            complete_postgres_claimed_job(&mut transaction, command.job, command.lease).await?;
            sqlx::query(
                "UPDATE attempt_effective_policy_current SET job_id = NULL, \
                    updated_at = transaction_timestamp() \
                 WHERE tenant_id = $1 AND attempt_id = $2",
            )
            .bind(tenant.as_uuid())
            .bind(command.attempt.as_uuid())
            .execute(&mut *transaction)
            .await
            .map_err(map_sqlx_error)?;
            transaction.commit().await.map_err(map_sqlx_error)?;
            return Ok(crate::AttemptAutoSubmitCommitOutcome::Superseded);
        };
        let now = database_timestamp(&mut transaction).await?;
        if now < auto_submit_at {
            let payload = serde_json::to_value(JobPayload::AutoSubmitAttempt {
                attempt: command.attempt,
                timing_generation: generation,
            })
            .map_err(|error| StoreError::InvalidRecord(error.to_string()))?;
            let changed: bool = sqlx::query_scalar(
                "SELECT ple_reschedule_attempt_timing_job($1, $2, $3, $4, \
                    TIMESTAMPTZ 'epoch' + $5::bigint * INTERVAL '1 millisecond')",
            )
            .bind(tenant.as_uuid())
            .bind(command.job.as_uuid())
            .bind(command.lease.as_uuid())
            .bind(payload)
            .bind(auto_submit_at.as_unix_millis())
            .fetch_one(&mut *transaction)
            .await
            .map_err(map_sqlx_error)?;
            if !changed {
                return Err(StoreError::Conflict);
            }
            transaction.commit().await.map_err(map_sqlx_error)?;
            return Ok(crate::AttemptAutoSubmitCommitOutcome::Rescheduled);
        }

        complete_postgres_claimed_job(&mut transaction, command.job, command.lease).await?;
        let updated = sqlx::query(
            "UPDATE question_attempt SET attempt_status = 'auto_submitted', \
                    submitted_at = transaction_timestamp() \
             WHERE tenant_id = $1 AND attempt_id = $2 AND attempt_status = 'in_progress'",
        )
        .bind(tenant.as_uuid())
        .bind(command.attempt.as_uuid())
        .execute(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;
        if updated.rows_affected() != 1 {
            return Err(StoreError::Conflict);
        }
        sqlx::query(
            "UPDATE attempt_effective_policy_current SET job_id = NULL, \
                updated_at = transaction_timestamp() \
             WHERE tenant_id = $1 AND attempt_id = $2",
        )
        .bind(tenant.as_uuid())
        .bind(command.attempt.as_uuid())
        .execute(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;
        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(crate::AttemptAutoSubmitCommitOutcome::AutoSubmitted)
    }
}

#[cfg(feature = "postgres")]
pub(super) async fn complete_postgres_claimed_job(
    transaction: &mut Transaction<'_, Postgres>,
    job: JobId,
    lease: JobLeaseToken,
) -> Result<(), StoreError> {
    let completed: bool = sqlx::query_scalar("SELECT ple_complete_worker_job($1, $2)")
        .bind(job.as_uuid())
        .bind(lease.as_uuid())
        .fetch_one(&mut **transaction)
        .await
        .map_err(map_sqlx_error)?;
    if !completed {
        return Err(StoreError::Conflict);
    }
    Ok(())
}

#[cfg(feature = "postgres")]
#[async_trait]
impl crate::AssignmentScoringWorkerStore for PostgresStore {
    async fn prepare_assignment_scoring(
        &self,
        context: TenantContext,
        command: crate::AssignmentScoringWorkerCommand,
    ) -> Result<crate::AssignmentScoringPreparationOutcome, StoreError> {
        let tenant = context.tenant_id();
        let expected_payload = serde_json::to_value(JobPayload::RecalculateAssignment {
            assignment: command.assignment,
            generation: command.generation,
        })
        .map_err(|error| StoreError::InvalidRecord(error.to_string()))?;
        let mut transaction = self.begin_tenant(context).await?;
        let claim_active: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM worker_job \
             WHERE job_id = $1 AND tenant_id = $2 AND state = 'leased' \
               AND lease_token = $3 AND lease_expires_at > transaction_timestamp() \
               AND payload = $4)",
        )
        .bind(command.job.as_uuid())
        .bind(tenant.as_uuid())
        .bind(command.lease.as_uuid())
        .bind(expected_payload)
        .fetch_one(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;
        if !claim_active {
            return Err(StoreError::Conflict);
        }
        let assignment = load_assignment(&mut transaction, tenant, command.assignment).await?;
        let generation = i64::try_from(command.generation.value()).map_err(|_| {
            StoreError::InvalidRecord("scoring generation is too large".to_string())
        })?;
        let current: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM assignment \
             WHERE tenant_id = $1 AND assignment_id = $2 \
               AND scoring_generation = $3 AND scoring_status = 'recalculating')",
        )
        .bind(tenant.as_uuid())
        .bind(command.assignment.as_uuid())
        .bind(generation)
        .fetch_one(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;
        sqlx::query("DELETE FROM assignment_scoring_staging WHERE tenant_id = $1 AND job_id = $2")
            .bind(tenant.as_uuid())
            .bind(command.job.as_uuid())
            .execute(&mut *transaction)
            .await
            .map_err(map_sqlx_error)?;
        sqlx::query(
            "DELETE FROM assignment_attempt_score_staging \
             WHERE tenant_id = $1 AND job_id = $2",
        )
        .bind(tenant.as_uuid())
        .bind(command.job.as_uuid())
        .execute(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;
        if !current {
            transaction.commit().await.map_err(map_sqlx_error)?;
            return Ok(crate::AssignmentScoringPreparationOutcome::Superseded);
        }
        sqlx::query(
            "DELETE FROM assignment_summary_staging \
             WHERE tenant_id = $1 AND job_id = $2",
        )
        .bind(tenant.as_uuid())
        .bind(command.job.as_uuid())
        .execute(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;
        let attempt_count = sqlx::query(
            "WITH current_definition AS ( \
                 SELECT se.attempt_id, ri.assignment_item_id, a.course_id, \
                        se.credit_fraction, se.grading_status, \
                        COALESCE(ai.points_possible, sg.points_per_item) AS points_possible, \
                        COALESCE(ai.scoring_mode, CASE WHEN sc.delivery_state = 'retired' \
                            THEN 'excluded' ELSE 'normal' END) AS scoring_mode \
                   FROM submission_evaluation se \
                   JOIN question_attempt qa ON qa.tenant_id = se.tenant_id \
                        AND qa.attempt_id = se.attempt_id \
                   JOIN assignment_run ar ON ar.tenant_id = qa.tenant_id \
                        AND ar.run_id = qa.run_id \
                   JOIN enrollment e ON e.tenant_id = ar.tenant_id \
                        AND e.enrollment_id = ar.enrollment_id \
                   JOIN assignment a ON a.tenant_id = e.tenant_id \
                        AND a.assignment_id = e.assignment_id \
                   JOIN assignment_run_item ri ON ri.tenant_id = qa.tenant_id \
                        AND ri.run_id = qa.run_id \
                        AND ri.issued_position = qa.assignment_position \
              LEFT JOIN assignment_item ai ON ai.tenant_id = a.tenant_id \
                        AND ai.assignment_id = a.assignment_id \
                        AND ai.assignment_item_id = ri.assignment_item_id \
              LEFT JOIN assignment_selection_candidate sc ON sc.tenant_id = a.tenant_id \
                        AND sc.assignment_id = a.assignment_id \
                        AND sc.candidate_id = ri.assignment_item_id \
              LEFT JOIN assignment_selection_group sg ON sg.tenant_id = sc.tenant_id \
                        AND sg.assignment_id = sc.assignment_id \
                        AND sg.selection_group_id = sc.selection_group_id \
                  WHERE se.tenant_id = $1 AND a.assignment_id = $2 \
                    AND se.grading_status = 'graded' \
                    AND qa.attempt_status NOT IN ('cleared', 'exempt') \
                    AND (ai.assignment_item_id IS NOT NULL OR sc.candidate_id IS NOT NULL) \
             ) \
             INSERT INTO assignment_attempt_score_staging \
                 (tenant_id, job_id, assignment_id, scoring_generation, attempt_id, \
                  assignment_item_id, earned_points, possible_points, course_id) \
             SELECT $1, $3, $2, $4, attempt_id, assignment_item_id, \
                    CASE \
                      WHEN scoring_mode = 'excluded' THEN 0 \
                      WHEN scoring_mode = 'full_credit' THEN points_possible \
                      ELSE round(credit_fraction * points_possible, 4) \
                    END, \
                    CASE \
                      WHEN scoring_mode IN ('excluded', 'extra_credit') THEN 0 \
                      ELSE points_possible \
                    END, course_id \
               FROM current_definition",
        )
        .bind(tenant.as_uuid())
        .bind(command.assignment.as_uuid())
        .bind(command.job.as_uuid())
        .bind(generation)
        .execute(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?
        .rows_affected();

        let score_rows = sqlx::query(
            "WITH latest AS ( \
                 SELECT DISTINCT ON (qa.run_id, qa.assignment_position) \
                        qa.run_id, staged.earned_points, staged.possible_points \
                   FROM assignment_attempt_score_staging staged \
                   JOIN question_attempt qa ON qa.tenant_id = staged.tenant_id \
                        AND qa.attempt_id = staged.attempt_id \
                   JOIN submission_evaluation se ON se.tenant_id = staged.tenant_id \
                        AND se.attempt_id = staged.attempt_id \
                  WHERE staged.tenant_id = $1 AND staged.job_id = $2 \
                  ORDER BY qa.run_id, qa.assignment_position, se.evaluated_at DESC, qa.attempt_id DESC \
             ) \
             SELECT ar.enrollment_id, ar.run_id, ar.run_number, \
                    floor(extract(epoch FROM ar.completed_at) * 1000)::bigint \
                        AS completed_at_millis, \
                    COALESCE(sum(latest.earned_points), 0)::text AS earned_points, \
                    COALESCE(sum(latest.possible_points), 0)::text AS possible_points \
               FROM assignment_run ar \
               JOIN enrollment e ON e.tenant_id = ar.tenant_id \
                    AND e.enrollment_id = ar.enrollment_id \
               JOIN latest ON latest.run_id = ar.run_id \
              WHERE ar.tenant_id = $1 AND e.assignment_id = $3 \
                AND ar.completed_at IS NOT NULL \
                AND NOT EXISTS ( \
                    SELECT 1 FROM question_attempt pending \
                    JOIN submission_evaluation evaluation \
                      ON evaluation.tenant_id = pending.tenant_id \
                     AND evaluation.attempt_id = pending.attempt_id \
                   WHERE pending.tenant_id = ar.tenant_id \
                     AND pending.run_id = ar.run_id \
                     AND pending.attempt_status NOT IN ('cleared', 'exempt') \
                     AND evaluation.grading_status = 'needs_manual_grading' \
                ) \
              GROUP BY ar.enrollment_id, ar.run_id, ar.run_number, ar.completed_at",
        )
        .bind(tenant.as_uuid())
        .bind(command.job.as_uuid())
        .bind(command.assignment.as_uuid())
        .fetch_all(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;
        let mut completed_by_enrollment: BTreeMap<
            EnrollmentId,
            Vec<domain::scoring::CompletedRunScore>,
        > = BTreeMap::new();
        let mut first_completed_at_by_enrollment = BTreeMap::new();
        for row in score_rows {
            let earned = row
                .try_get::<String, _>("earned_points")
                .map_err(map_sqlx_error)?
                .parse::<f64>()
                .map_err(|_| {
                    StoreError::Unavailable("stored earned points are invalid".to_string())
                })?;
            let possible = row
                .try_get::<String, _>("possible_points")
                .map_err(map_sqlx_error)?
                .parse::<f64>()
                .map_err(|_| {
                    StoreError::Unavailable("stored possible points are invalid".to_string())
                })?;
            let score = if possible > 0.0 {
                earned / possible
            } else {
                earned
            };
            let enrollment =
                EnrollmentId::from_uuid(row.try_get("enrollment_id").map_err(map_sqlx_error)?);
            let completed_at = ActivityTimestamp::from_unix_millis(
                row.try_get("completed_at_millis").map_err(map_sqlx_error)?,
            );
            first_completed_at_by_enrollment
                .entry(enrollment)
                .and_modify(|current: &mut ActivityTimestamp| {
                    *current = (*current).min(completed_at);
                })
                .or_insert(completed_at);
            completed_by_enrollment.entry(enrollment).or_default().push(
                domain::scoring::CompletedRunScore {
                    run: RunId::from_uuid(row.try_get("run_id").map_err(map_sqlx_error)?),
                    run_number: u32::try_from(
                        row.try_get::<i64, _>("run_number")
                            .map_err(map_sqlx_error)?,
                    )
                    .map_err(|_| {
                        StoreError::Unavailable("stored run number is invalid".to_string())
                    })?,
                    score: crate::score_precision::round_for_persistence(score),
                },
            );
        }
        let enrollment_rows = sqlx::query(
            "SELECT e.enrollment_id, e.tenant_id, e.assignment_id, e.user_id, e.student_id, \
                    floor(extract(epoch FROM e.first_completed_at) * 1000)::bigint \
                        AS first_completed_at_millis, \
                    e.current_grade_run_id, e.best_grade_run_id, \
                    sas.tenant_id AS summary_tenant_id, \
                    sas.enrollment_id AS summary_enrollment_id, \
                    sas.current_score AS summary_current_score, \
                    sas.best_score AS summary_best_score, \
                    sas.latest_score AS summary_latest_score, \
                    sas.completed_run_count AS summary_completed_run_count, \
                    sas.total_question_attempts AS summary_total_question_attempts, \
                    floor(extract(epoch FROM sas.last_activity_at) * 1000)::bigint \
                        AS summary_last_activity_at_millis \
               FROM enrollment e \
               JOIN student_assignment_summary sas ON sas.tenant_id = e.tenant_id \
                    AND sas.enrollment_id = e.enrollment_id \
              WHERE e.tenant_id = $1 AND e.assignment_id = $2 \
              ORDER BY e.enrollment_id FOR SHARE OF e, sas",
        )
        .bind(tenant.as_uuid())
        .bind(command.assignment.as_uuid())
        .fetch_all(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;
        let enrollment_count = enrollment_rows.len();
        for row in enrollment_rows {
            let enrollment = decode_postgres_enrollment_row(&row)?;
            let summary = decode_summary_row_named(&row, "summary_")?;
            let completed = completed_by_enrollment
                .remove(&enrollment.id)
                .unwrap_or_default();
            let first_completed_at = first_completed_at_by_enrollment.remove(&enrollment.id);
            let (enrollment, summary) = crate::recalculated_enrollment_projection(
                enrollment,
                summary,
                assignment.policies.grade,
                completed,
                first_completed_at,
            )?;
            sqlx::query(
                "INSERT INTO assignment_summary_staging \
                 (tenant_id, job_id, assignment_id, scoring_generation, enrollment_id, \
                  current_score, best_score, latest_score, completed_run_count, \
                  total_question_attempts, last_activity_at, first_completed_at, \
                  current_grade_run_id, best_grade_run_id) \
                 VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, \
                         to_timestamp($11::double precision / 1000), \
                         to_timestamp($12::double precision / 1000), $13, $14)",
            )
            .bind(tenant.as_uuid())
            .bind(command.job.as_uuid())
            .bind(command.assignment.as_uuid())
            .bind(generation)
            .bind(enrollment.id.as_uuid())
            .bind(summary.current_score)
            .bind(summary.best_score)
            .bind(summary.latest_score)
            .bind(i64::from(summary.completed_run_count))
            .bind(i64::try_from(summary.total_question_attempts).map_err(|_| StoreError::Conflict)?)
            .bind(summary.last_activity_at.map(|value| value.as_unix_millis()))
            .bind(
                enrollment
                    .first_completed_at
                    .map(|value| value.as_unix_millis()),
            )
            .bind(enrollment.current_grade_run.map(|value| value.as_uuid()))
            .bind(enrollment.best_grade_run.map(|value| value.as_uuid()))
            .execute(&mut *transaction)
            .await
            .map_err(map_sqlx_error)?;
        }
        if !completed_by_enrollment.is_empty() {
            return Err(StoreError::Unavailable(
                "completed run has no assignment enrollment".to_string(),
            ));
        }
        if !first_completed_at_by_enrollment.is_empty() {
            return Err(StoreError::Unavailable(
                "completed run timestamp has no assignment enrollment".to_string(),
            ));
        }
        sqlx::query(
            "INSERT INTO assignment_scoring_staging \
             (tenant_id, job_id, assignment_id, scoring_generation, attempt_count, enrollment_count) \
             VALUES ($1, $2, $3, $4, $5, $6)",
        )
        .bind(tenant.as_uuid())
        .bind(command.job.as_uuid())
        .bind(command.assignment.as_uuid())
        .bind(generation)
        .bind(i64::try_from(attempt_count).map_err(|_| StoreError::Conflict)?)
        .bind(i64::try_from(enrollment_count).map_err(|_| StoreError::Conflict)?)
        .execute(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;
        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(crate::AssignmentScoringPreparationOutcome::Prepared)
    }

    async fn commit_assignment_scoring(
        &self,
        context: TenantContext,
        command: crate::AssignmentScoringWorkerCommand,
    ) -> Result<crate::AssignmentScoringCommitOutcome, StoreError> {
        let tenant = context.tenant_id();
        let expected_payload = serde_json::to_value(JobPayload::RecalculateAssignment {
            assignment: command.assignment,
            generation: command.generation,
        })
        .map_err(|error| StoreError::InvalidRecord(error.to_string()))?;
        let generation = i64::try_from(command.generation.value()).map_err(|_| {
            StoreError::InvalidRecord("scoring generation is too large".to_string())
        })?;
        let mut transaction = self.begin_tenant(context).await?;
        let claim_active: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM worker_job \
             WHERE job_id = $1 AND tenant_id = $2 AND state = 'leased' \
               AND lease_token = $3 AND lease_expires_at > transaction_timestamp() \
               AND payload = $4)",
        )
        .bind(command.job.as_uuid())
        .bind(tenant.as_uuid())
        .bind(command.lease.as_uuid())
        .bind(expected_payload)
        .fetch_one(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;
        if !claim_active {
            transaction.rollback().await.map_err(map_sqlx_error)?;
            return Ok(crate::AssignmentScoringCommitOutcome::ClaimNoLongerActive);
        }
        let (current_generation, current_status): (i64, String) = sqlx::query_as(
            "SELECT scoring_generation, scoring_status FROM assignment \
             WHERE tenant_id = $1 AND assignment_id = $2",
        )
        .bind(tenant.as_uuid())
        .bind(command.assignment.as_uuid())
        .fetch_one(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;
        let superseded = current_generation != generation || current_status != "recalculating";
        let current_attempt_count = if superseded {
            0_i64
        } else {
            sqlx::query_scalar(
                "SELECT count(*) \
               FROM submission_evaluation se \
               JOIN question_attempt qa ON qa.tenant_id = se.tenant_id \
                    AND qa.attempt_id = se.attempt_id \
               JOIN assignment_run ar ON ar.tenant_id = qa.tenant_id \
                    AND ar.run_id = qa.run_id \
               JOIN enrollment e ON e.tenant_id = ar.tenant_id \
                    AND e.enrollment_id = ar.enrollment_id \
               JOIN assignment a ON a.tenant_id = e.tenant_id \
                    AND a.assignment_id = e.assignment_id \
               JOIN assignment_run_item ri ON ri.tenant_id = qa.tenant_id \
                    AND ri.run_id = qa.run_id \
                    AND ri.issued_position = qa.assignment_position \
          LEFT JOIN assignment_item ai ON ai.tenant_id = a.tenant_id \
                    AND ai.assignment_id = a.assignment_id \
                    AND ai.assignment_item_id = ri.assignment_item_id \
          LEFT JOIN assignment_selection_candidate sc ON sc.tenant_id = a.tenant_id \
                    AND sc.assignment_id = a.assignment_id \
                    AND sc.candidate_id = ri.assignment_item_id \
              WHERE se.tenant_id = $1 AND a.assignment_id = $2 \
                AND se.grading_status = 'graded' \
                AND qa.attempt_status NOT IN ('cleared', 'exempt') \
                AND (ai.assignment_item_id IS NOT NULL OR sc.candidate_id IS NOT NULL)",
            )
            .bind(tenant.as_uuid())
            .bind(command.assignment.as_uuid())
            .fetch_one(&mut *transaction)
            .await
            .map_err(map_sqlx_error)?
        };
        let prepared: bool = superseded
            || sqlx::query_scalar(
                "SELECT EXISTS(SELECT 1 FROM assignment_scoring_staging \
             WHERE tenant_id = $1 AND job_id = $2 AND assignment_id = $3 \
               AND scoring_generation = $4 \
               AND ($6 OR attempt_count = $5) \
               AND attempt_count = (SELECT count(*) FROM assignment_attempt_score_staging \
                    WHERE tenant_id = $1 AND job_id = $2) \
               AND enrollment_count = (SELECT count(*) FROM assignment_summary_staging \
                    WHERE tenant_id = $1 AND job_id = $2))",
            )
            .bind(tenant.as_uuid())
            .bind(command.job.as_uuid())
            .bind(command.assignment.as_uuid())
            .bind(generation)
            .bind(current_attempt_count)
            .bind(false)
            .fetch_one(&mut *transaction)
            .await
            .map_err(map_sqlx_error)?;
        if !prepared {
            return Err(StoreError::Conflict);
        }
        if !superseded {
            sqlx::query(
                "DELETE FROM attempt_score_current \
                 WHERE tenant_id = $1 AND assignment_id = $2",
            )
            .bind(tenant.as_uuid())
            .bind(command.assignment.as_uuid())
            .execute(&mut *transaction)
            .await
            .map_err(map_sqlx_error)?;
            sqlx::query(
                "INSERT INTO attempt_score_current \
                 (tenant_id, attempt_id, assignment_id, assignment_item_id, scoring_generation, \
                  earned_points, possible_points, course_id) \
                 SELECT tenant_id, attempt_id, assignment_id, assignment_item_id, \
                        scoring_generation, earned_points, possible_points, course_id \
                   FROM assignment_attempt_score_staging \
                  WHERE tenant_id = $1 AND job_id = $2",
            )
            .bind(tenant.as_uuid())
            .bind(command.job.as_uuid())
            .execute(&mut *transaction)
            .await
            .map_err(map_sqlx_error)?;
            sqlx::query(
                "UPDATE student_assignment_summary sas \
                    SET current_score = staged.current_score, \
                        best_score = staged.best_score, latest_score = staged.latest_score, \
                        completed_run_count = staged.completed_run_count, \
                        total_question_attempts = staged.total_question_attempts, \
                        last_activity_at = staged.last_activity_at, \
                        updated_at = transaction_timestamp() \
                   FROM assignment_summary_staging staged \
                  WHERE staged.tenant_id = $1 AND staged.job_id = $2 \
                    AND sas.tenant_id = staged.tenant_id \
                    AND sas.enrollment_id = staged.enrollment_id",
            )
            .bind(tenant.as_uuid())
            .bind(command.job.as_uuid())
            .execute(&mut *transaction)
            .await
            .map_err(map_sqlx_error)?;
            sqlx::query(
                "UPDATE enrollment e SET first_completed_at = staged.first_completed_at, \
                        current_grade_run_id = staged.current_grade_run_id, \
                        best_grade_run_id = staged.best_grade_run_id \
                   FROM assignment_summary_staging staged \
                  WHERE staged.tenant_id = $1 AND staged.job_id = $2 \
                    AND e.tenant_id = staged.tenant_id \
                    AND e.enrollment_id = staged.enrollment_id",
            )
            .bind(tenant.as_uuid())
            .bind(command.job.as_uuid())
            .execute(&mut *transaction)
            .await
            .map_err(map_sqlx_error)?;
            // Item analysis is a separate, retryable projection.  Its later
            // failure cannot undo this already-published scoring generation.
            let published =
                super::assignment_scoring_publication::publish_assignment_scoring_generation(
                    &mut transaction,
                    tenant,
                    command.job,
                    command.lease,
                    command.assignment,
                    command.generation,
                )
                .await?;
            if !published {
                return Err(StoreError::Conflict);
            }
        }
        sqlx::query("DELETE FROM assignment_scoring_staging WHERE tenant_id = $1 AND job_id = $2")
            .bind(tenant.as_uuid())
            .bind(command.job.as_uuid())
            .execute(&mut *transaction)
            .await
            .map_err(map_sqlx_error)?;
        sqlx::query(
            "DELETE FROM assignment_attempt_score_staging WHERE tenant_id = $1 AND job_id = $2",
        )
        .bind(tenant.as_uuid())
        .bind(command.job.as_uuid())
        .execute(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;
        sqlx::query("DELETE FROM assignment_summary_staging WHERE tenant_id = $1 AND job_id = $2")
            .bind(tenant.as_uuid())
            .bind(command.job.as_uuid())
            .execute(&mut *transaction)
            .await
            .map_err(map_sqlx_error)?;
        let completed: bool = sqlx::query_scalar("SELECT ple_complete_worker_job($1, $2)")
            .bind(command.job.as_uuid())
            .bind(command.lease.as_uuid())
            .fetch_one(&mut *transaction)
            .await
            .map_err(map_sqlx_error)?;
        if !completed {
            return Err(StoreError::Conflict);
        }
        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(if superseded {
            crate::AssignmentScoringCommitOutcome::Superseded
        } else {
            crate::AssignmentScoringCommitOutcome::Committed
        })
    }
}

#[cfg(feature = "postgres")]
fn decode_job_payload(value: Value) -> Result<JobPayload, StoreError> {
    serde_json::from_value(value).map_err(|error| {
        StoreError::Unavailable(format!("stored queue payload is invalid: {error}"))
    })
}

#[cfg(feature = "postgres")]
pub(super) fn decode_claimed_job(
    row: &PgRow,
    expected_token: JobLeaseToken,
) -> Result<ClaimedJob, StoreError> {
    let Json(payload): Json<Value> = row.try_get("payload").map_err(map_sqlx_error)?;
    let stored_token: Uuid = row.try_get("lease_token").map_err(map_sqlx_error)?;
    if stored_token != expected_token.as_uuid() {
        return Err(StoreError::Unavailable(
            "queue broker returned a mismatched lease token".to_string(),
        ));
    }
    let attempt_count: i32 = row.try_get("attempt_count").map_err(map_sqlx_error)?;
    Ok(ClaimedJob {
        id: JobId::from_uuid(row.try_get("job_id").map_err(map_sqlx_error)?),
        tenant: TenantId::from_uuid(row.try_get("tenant_id").map_err(map_sqlx_error)?),
        payload: decode_job_payload(payload)?,
        lease_token: JobLeaseToken::from_uuid(stored_token),
        attempt_count: u16::try_from(attempt_count).map_err(|_| {
            StoreError::Unavailable("stored queue attempt count is invalid".to_string())
        })?,
    })
}

#[cfg(feature = "postgres")]
fn decode_tenant_job_view(row: &PgRow, id: JobId) -> Result<TenantJobView, StoreError> {
    let Json(payload): Json<Value> = row.try_get("payload").map_err(map_sqlx_error)?;
    let state: String = row.try_get("state").map_err(map_sqlx_error)?;
    let state = match state.as_str() {
        "ready" => JobState::Ready,
        "leased" => JobState::Leased,
        "completed" => JobState::Completed,
        "dead" => JobState::Dead,
        _ => {
            return Err(StoreError::Unavailable(
                "stored queue state is invalid".to_string(),
            ));
        }
    };
    let attempt_count: i32 = row.try_get("attempt_count").map_err(map_sqlx_error)?;
    Ok(TenantJobView {
        id,
        payload: decode_job_payload(payload)?,
        state,
        attempt_count: u16::try_from(attempt_count).map_err(|_| {
            StoreError::Unavailable("stored queue attempt count is invalid".to_string())
        })?,
    })
}

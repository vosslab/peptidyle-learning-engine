use async_trait::async_trait;

use super::*;

#[async_trait]
impl crate::FeedbackStore for PostgresStore {
    async fn release_attempt_feedback_impl(
        &self,
        context: TenantContext,
        command: ReleaseAttemptFeedbackCommand,
    ) -> Result<FeedbackReleaseRecord, StoreError> {
        let tenant = context.tenant_id();
        let mut transaction = self.begin_tenant(context).await?;
        let attempt =
            load_attempt_for_external_update(&mut transaction, tenant, command.attempt).await?;
        let run = load_run_for_update(&mut transaction, tenant, attempt.run).await?;
        let enrollment =
            load_enrollment_for_update(&mut transaction, tenant, run.enrollment).await?;
        let assignment = load_assignment(&mut transaction, tenant, enrollment.assignment).await?;
        if !postgres_is_course_instructor(
            &mut transaction,
            tenant,
            assignment.course_id,
            command.actor,
        )
        .await?
        {
            return Err(StoreError::NotFound);
        }
        let has_feedback: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM attempt_feedback WHERE tenant_id = $1 AND attempt_id = $2)",
        )
        .bind(tenant.as_uuid())
        .bind(command.attempt.as_uuid())
        .fetch_one(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;
        if !has_feedback {
            return Err(StoreError::NotFound);
        }
        let disclosure = super::submission::load_issued_feedback_disclosure(
            &mut transaction,
            tenant,
            command.attempt,
        )
        .await?;
        if disclosure != question_model::run_policy::FeedbackDisclosure::OnRelease {
            return Err(StoreError::InvalidRecord(
                "feedback release requires an on-release question policy".to_string(),
            ));
        }
        let inserted = sqlx::query(
            "INSERT INTO feedback_release (tenant_id, attempt_id, released_by, released_at) \
             VALUES ($1, $2, $3, transaction_timestamp()) \
             ON CONFLICT (tenant_id, attempt_id) DO NOTHING \
             RETURNING released_by, floor(extract(epoch FROM released_at) * 1000)::bigint AS released_at",
        )
        .bind(tenant.as_uuid())
        .bind(command.attempt.as_uuid())
        .bind(command.actor.as_uuid())
        .fetch_optional(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;
        let row = match inserted {
            Some(row) => row,
            None => sqlx::query(
                "SELECT released_by, floor(extract(epoch FROM released_at) * 1000)::bigint AS released_at \
                 FROM feedback_release WHERE tenant_id = $1 AND attempt_id = $2",
            )
            .bind(tenant.as_uuid())
            .bind(command.attempt.as_uuid())
            .fetch_one(&mut *transaction)
            .await
            .map_err(map_sqlx_error)?,
        };
        let released_by = UserId::from_uuid(row.try_get("released_by").map_err(map_sqlx_error)?);
        if released_by != command.actor {
            return Err(StoreError::Conflict);
        }
        let released_at: i64 = row.try_get("released_at").map_err(map_sqlx_error)?;
        let record = FeedbackReleaseRecord {
            tenant,
            attempt: command.attempt,
            released_by,
            released_at: ActivityTimestamp::from_unix_millis(released_at),
        };
        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(record)
    }
    async fn get_attempt_feedback_release_impl(
        &self,
        context: TenantContext,
        actor: UserId,
        attempt_id: QuestionAttemptId,
    ) -> Result<Option<FeedbackReleaseRecord>, StoreError> {
        let tenant = context.tenant_id();
        let mut transaction = self.begin_tenant(context).await?;
        let attempt =
            load_attempt_for_external_update(&mut transaction, tenant, attempt_id).await?;
        let run = load_run_for_update(&mut transaction, tenant, attempt.run).await?;
        let enrollment =
            load_enrollment_for_update(&mut transaction, tenant, run.enrollment).await?;
        let assignment = load_assignment(&mut transaction, tenant, enrollment.assignment).await?;
        let learner_self = matches!(
            super::entitlement::evaluate_current(
                &mut transaction, tenant, actor, assignment.course_id, assignment.id,
            )
            .await?,
            domain::entitlement::EntitlementDecision::Granted(ref grant)
                if grant.student() == enrollment.student
        );
        if !learner_self
            && !postgres_is_course_instructor(&mut transaction, tenant, assignment.course_id, actor)
                .await?
        {
            return Err(StoreError::NotFound);
        }
        let row = sqlx::query(
            "SELECT released_by, floor(extract(epoch FROM released_at) * 1000)::bigint AS released_at \
             FROM feedback_release WHERE tenant_id = $1 AND attempt_id = $2",
        )
        .bind(tenant.as_uuid())
        .bind(attempt_id.as_uuid())
        .fetch_optional(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;
        let record = row
            .map(|row| {
                Ok::<FeedbackReleaseRecord, StoreError>(FeedbackReleaseRecord {
                    tenant,
                    attempt: attempt_id,
                    released_by: UserId::from_uuid(
                        row.try_get("released_by").map_err(map_sqlx_error)?,
                    ),
                    released_at: ActivityTimestamp::from_unix_millis(
                        row.try_get("released_at").map_err(map_sqlx_error)?,
                    ),
                })
            })
            .transpose()?;
        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(record)
    }
    async fn get_run_summary_page_impl(
        &self,
        context: TenantContext,
        actor: UserId,
        run_id: RunId,
        page: PageRequest,
    ) -> Result<RunSummaryPageInput, StoreError> {
        let tenant = context.tenant_id();
        let after = page
            .after
            .as_ref()
            .map(|cursor| RunSummaryCursor::decode(cursor, tenant.as_uuid(), run_id.as_uuid()))
            .transpose()?;
        let limit = i64::from(page.size.get()) + 1;
        let mut transaction = self.begin_tenant(context).await?;
        let run_row = sqlx::query(
            "SELECT payload, payload_sha256 FROM assignment_run \
             WHERE tenant_id = $1 AND run_id = $2",
        )
        .bind(tenant.as_uuid())
        .bind(run_id.as_uuid())
        .fetch_optional(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?
        .ok_or(StoreError::NotFound)?;
        let run: AssignmentRun = decode_payload_row(&run_row)?;
        let enrollment = load_postgres_enrollment(&mut transaction, tenant, run.enrollment).await?;
        let assignment = load_assignment(&mut transaction, tenant, enrollment.assignment).await?;
        let learner_self = matches!(
            super::entitlement::evaluate_current(
                &mut transaction, tenant, actor, assignment.course_id, assignment.id,
            )
            .await?,
            domain::entitlement::EntitlementDecision::Granted(ref grant)
                if grant.student() == enrollment.student
        );
        if !learner_self
            && !postgres_is_course_instructor(&mut transaction, tenant, assignment.course_id, actor)
                .await?
        {
            return Err(StoreError::NotFound);
        }
        let summary_row = sqlx::query(
            "SELECT tenant_id, enrollment_id, current_score, best_score, latest_score, \
                    completed_run_count, total_question_attempts, \
                    floor(extract(epoch FROM last_activity_at) * 1000)::bigint \
                        AS last_activity_at_millis \
             FROM student_assignment_summary \
             WHERE tenant_id = $1 AND enrollment_id = $2",
        )
        .bind(tenant.as_uuid())
        .bind(enrollment.id.as_uuid())
        .fetch_optional(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?
        .ok_or(StoreError::NotFound)?;
        let summary = decode_summary_row(&summary_row)?;

        // This is deliberately the sole bounded outcome query. It reads each
        // attempt's issuance-persisted disclosure and private feedback/release
        // rows so a later catalog edit cannot rewrite historical feedback.
        let rows = sqlx::query(
            "SELECT COALESCE(si.payload, qa.payload) AS attempt_payload, \
                    COALESCE(si.payload_sha256, qa.payload_sha256) AS attempt_sha256, \
                    evaluation.payload AS evaluation_payload, \
                    evaluation.payload_sha256 AS evaluation_payload_sha256, \
                    evaluation.grading_status AS evaluation_grading_status, \
                    qa.attempt_status AS current_attempt_status, \
                    floor(extract(epoch FROM qa.submitted_at) * 1000)::bigint \
                        AS current_submitted_at, \
                    floor(extract(epoch FROM timing.effective_deadline) * 1000)::bigint \
                        AS current_deadline_at, \
                    qa.issued_feedback_disclosure AS feedback_policy, \
                    af.hint, af.correct_response, af.rationale, af.content_sha256, \
                    fr.released_by, floor(extract(epoch FROM fr.released_at) * 1000)::bigint AS released_at \
             FROM question_attempt AS qa \
             LEFT JOIN submission_idempotency AS si \
               ON si.tenant_id = qa.tenant_id AND si.attempt_id = qa.attempt_id \
             LEFT JOIN submission_evaluation AS evaluation \
               ON evaluation.tenant_id = qa.tenant_id AND evaluation.attempt_id = qa.attempt_id \
             LEFT JOIN attempt_effective_policy_current AS current_effect ON current_effect.tenant_id=qa.tenant_id AND current_effect.attempt_id=qa.attempt_id \
             LEFT JOIN attempt_effective_policy_receipt AS timing ON timing.tenant_id=current_effect.tenant_id AND timing.attempt_id=current_effect.attempt_id AND timing.receipt_generation=current_effect.receipt_generation \
             JOIN assignment_run_item AS ri \
               ON ri.tenant_id = qa.tenant_id AND ri.run_id = qa.run_id \
              AND ri.issued_position = qa.assignment_position \
             LEFT JOIN assignment_item AS ai \
               ON ai.tenant_id = ri.tenant_id AND ai.assignment_id = $6 \
              AND ai.assignment_item_id = ri.assignment_item_id \
             LEFT JOIN assignment_selection_candidate AS sc \
               ON sc.tenant_id = ri.tenant_id AND sc.assignment_id = $6 \
              AND sc.candidate_id = ri.assignment_item_id \
             LEFT JOIN attempt_feedback AS af \
               ON af.tenant_id = qa.tenant_id AND af.attempt_id = qa.attempt_id \
             LEFT JOIN feedback_release AS fr \
               ON fr.tenant_id = qa.tenant_id AND fr.attempt_id = qa.attempt_id \
             WHERE qa.tenant_id = $1 AND qa.run_id = $2 \
               AND ($3::integer IS NULL OR (qa.assignment_position, qa.attempt_id) > ($3, $4::uuid)) \
               AND (NOT $7::boolean OR COALESCE(ai.delivery_state, sc.delivery_state) <> 'retired') \
               AND (NOT $7::boolean OR qa.attempt_status <> 'cleared') \
             ORDER BY qa.assignment_position, qa.attempt_id LIMIT $5",
        )
        .bind(tenant.as_uuid())
        .bind(run.id.as_uuid())
        .bind(after.map(|cursor| i32::try_from(cursor.assignment_position)).transpose().map_err(|_| StoreError::InvalidRecord("run summary cursor position is invalid".to_string()))?)
        .bind(after.map(|cursor| cursor.attempt))
        .bind(limit)
        .bind(assignment.id.as_uuid())
        .bind(learner_self)
        .fetch_all(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;
        let has_more = rows.len() > usize::from(page.size.get());
        let mut outcomes = Vec::with_capacity(rows.len().min(usize::from(page.size.get())));
        for row in rows.into_iter().take(usize::from(page.size.get())) {
            let attempt = decode_current_attempt_with_evaluation_row_named(
                &row,
                "attempt_payload",
                "attempt_sha256",
            )?;
            let feedback = feedback_from_summary_row(&row)?;
            let release = row
                .try_get::<Option<Uuid>, _>("released_by")
                .map_err(map_sqlx_error)?
                .zip(
                    row.try_get::<Option<i64>, _>("released_at")
                        .map_err(map_sqlx_error)?,
                )
                .map(|(released_by, released_at)| FeedbackReleaseRecord {
                    tenant,
                    attempt: attempt.id,
                    released_by: UserId::from_uuid(released_by),
                    released_at: ActivityTimestamp::from_unix_millis(released_at),
                });
            outcomes.push((
                RunSummaryCursor {
                    assignment_position: attempt.assignment_position,
                    attempt: attempt.id.as_uuid(),
                },
                RunSummaryOutcomeInput {
                    attempt: attempt.id,
                    assignment_position: attempt.assignment_position,
                    submitted_at: attempt.timer.submitted_at,
                    response: attempt.response,
                    result: attempt.result,
                    feedback_policy: feedback_policy_from_summary_row(&row)?,
                    feedback,
                    release,
                },
            ));
        }
        let next_cursor = has_more
            .then(|| {
                outcomes
                    .last()
                    .map(|(cursor, _)| cursor.encode(tenant.as_uuid(), run.id.as_uuid()))
            })
            .flatten();
        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(RunSummaryPageInput {
            practice_allowed: continued_practice_allows_run(
                &summary,
                assignment.policies.continued_practice,
            ),
            run,
            assignment,
            summary,
            outcomes: Page {
                items: outcomes.into_iter().map(|(_, item)| item).collect(),
                next_cursor,
            },
        })
    }
}

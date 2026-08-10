use async_trait::async_trait;

use super::*;

#[async_trait]
impl crate::AssignmentPolicyStore for PostgresStore {
    async fn get_assignment_timing_impl(
        &self,
        context: TenantContext,
        assignment: AssignmentId,
    ) -> Result<Option<StoredAssignmentTiming>, StoreError> {
        let mut transaction = self.begin_tenant(context).await?;
        let row = sqlx::query(
            "SELECT assignment_id, course_id, visible, \
                    floor(extract(epoch FROM available_at) * 1000)::bigint AS available_at_millis, \
                    floor(extract(epoch FROM due_at) * 1000)::bigint AS due_at_millis, \
                    floor(extract(epoch FROM closes_at) * 1000)::bigint AS closes_at_millis, \
                    late_submission_policy, time_limit_seconds, auto_submit, attempt_limit, revision \
             FROM assignment WHERE tenant_id = $1 AND assignment_id = $2",
        )
        .bind(context.tenant_id().as_uuid())
        .bind(assignment.as_uuid())
        .fetch_optional(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;
        let result = row
            .as_ref()
            .map(|row| assignment_timing::decode_stored_assignment_timing(row, context.tenant_id()))
            .transpose()?;
        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(result)
    }
    async fn update_assignment_timing_impl(
        &self,
        context: TenantContext,
        command: UpdateAssignmentTimingCommand,
    ) -> Result<StoredAssignmentTiming, StoreError> {
        retry_transaction(|| async move {
        validate_assignment_timing(command.policy)?;
        let tenant = context.tenant_id();
        let mut transaction = self.begin_tenant(context).await?;
        let row = sqlx::query(
            "SELECT assignment_id, course_id, visible, \
                    floor(extract(epoch FROM available_at) * 1000)::bigint AS available_at_millis, \
                    floor(extract(epoch FROM due_at) * 1000)::bigint AS due_at_millis, \
                    floor(extract(epoch FROM closes_at) * 1000)::bigint AS closes_at_millis, \
                    late_submission_policy, time_limit_seconds, auto_submit, attempt_limit, revision \
             FROM assignment WHERE tenant_id = $1 AND assignment_id = $2",
        )
        .bind(tenant.as_uuid())
        .bind(command.assignment.as_uuid())
        .fetch_optional(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?
        .ok_or(StoreError::NotFound)?;
        let current = assignment_timing::decode_stored_assignment_timing(&row, tenant)?;
        if current.course != command.course
            || !postgres_is_course_instructor(
                &mut transaction,
                tenant,
                command.course,
                command.actor,
            )
            .await?
        {
            return Err(StoreError::NotFound);
        }
        if current.policy == command.policy {
            let locked = sqlx::query(
                "SELECT assignment_id, course_id, visible, \
                        floor(extract(epoch FROM available_at) * 1000)::bigint AS available_at_millis, \
                        floor(extract(epoch FROM due_at) * 1000)::bigint AS due_at_millis, \
                        floor(extract(epoch FROM closes_at) * 1000)::bigint AS closes_at_millis, \
                        late_submission_policy, time_limit_seconds, auto_submit, attempt_limit, revision \
                 FROM assignment WHERE tenant_id = $1 AND assignment_id = $2 FOR UPDATE",
            )
            .bind(tenant.as_uuid())
            .bind(command.assignment.as_uuid())
            .fetch_optional(&mut *transaction)
            .await
            .map_err(map_sqlx_error)?
            .ok_or(StoreError::NotFound)?;
            let locked = assignment_timing::decode_stored_assignment_timing(&locked, tenant)?;
            if locked.policy != command.policy {
                return Err(StoreError::Conflict);
            }
            transaction.commit().await.map_err(map_sqlx_error)?;
            return Ok(locked);
        }
        if current.revision != command.expected_revision {
            return Err(StoreError::Conflict);
        }
        assignment_timing::lock_postgres_assignment_policy(&mut transaction, tenant, command.assignment).await?;
        let active_rows =
            assignment_timing::lock_postgres_active_timing_rows(&mut transaction, tenant, command.assignment).await?;
        let locked =
            assignment_timing::load_postgres_assignment_timing(&mut transaction, tenant, command.assignment, true)
                .await?
                .ok_or(StoreError::NotFound)?;
        if locked.policy == command.policy {
            transaction.commit().await.map_err(map_sqlx_error)?;
            return Ok(locked);
        }
        if locked.revision != command.expected_revision || locked.course != command.course {
            return Err(StoreError::Conflict);
        }
        let now = database_timestamp(&mut transaction).await?;
        assignment_timing::apply_postgres_locked_timing_rows(
            &mut transaction,
            tenant,
            command.assignment,
            Some(command.policy),
            now,
            active_rows,
        )
        .await?;
        let revision = locked.revision.next()?;
        let updated = sqlx::query(
            "UPDATE assignment SET visible = $3, \
                    available_at = TIMESTAMPTZ 'epoch' + $4::bigint * INTERVAL '1 millisecond', \
                    due_at = TIMESTAMPTZ 'epoch' + $5::bigint * INTERVAL '1 millisecond', \
                    closes_at = TIMESTAMPTZ 'epoch' + $6::bigint * INTERVAL '1 millisecond', \
                    late_submission_policy = $7, time_limit_seconds = $8, \
                    auto_submit = true, attempt_limit = $9, revision = $10, \
                    updated_at = transaction_timestamp() \
             WHERE tenant_id = $1 AND assignment_id = $2 AND revision = $11",
        )
        .bind(tenant.as_uuid())
        .bind(command.assignment.as_uuid())
        .bind(command.policy.visible)
        .bind(
            command
                .policy
                .available_at
                .map(|value| value.as_unix_millis()),
        )
        .bind(command.policy.due_at.map(|value| value.as_unix_millis()))
        .bind(command.policy.closes_at.map(|value| value.as_unix_millis()))
        .bind(assignment_timing::late_submission_policy_name(command.policy.late_submission))
        .bind(command.policy.time_limit_seconds.map(i64::from))
        .bind(command.policy.attempt_limit.map(i64::from))
        .bind(i64::try_from(revision.value()).map_err(|_| StoreError::Conflict)?)
        .bind(i64::try_from(locked.revision.value()).map_err(|_| StoreError::Conflict)?)
        .execute(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;
        if updated.rows_affected() != 1 {
            return Err(StoreError::Conflict);
        }
        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(StoredAssignmentTiming {
            tenant,
            course: command.course,
            assignment: command.assignment,
            policy: command.policy,
            revision,
        })
        })
        .await
    }
    async fn set_assignment_policy_exception_impl(
        &self,
        context: TenantContext,
        command: SetAssignmentPolicyExceptionCommand,
    ) -> Result<StoredAssignmentPolicyException, StoreError> {
        retry_transaction(|| {
            let command = command.clone();
            async move {
                validate_assignment_policy_exception(&command.exception)?;
                let tenant = context.tenant_id();
                let mut transaction = self.begin_tenant(context).await?;
                if let AssignmentPolicyExceptionTarget::CourseGroup(group) =
                    command.exception.target
                {
                    let course: Option<Uuid> = sqlx::query_scalar(
                        "SELECT course_id FROM course_group WHERE tenant_id = $1 \
                 AND course_group_id = $2 FOR UPDATE",
                    )
                    .bind(tenant.as_uuid())
                    .bind(group.as_uuid())
                    .fetch_optional(&mut *transaction)
                    .await
                    .map_err(map_sqlx_error)?;
                    if course != Some(command.course.as_uuid()) {
                        return Err(StoreError::NotFound);
                    }
                }
                assignment_timing::lock_postgres_assignment_policy(
                    &mut transaction,
                    tenant,
                    command.assignment,
                )
                .await?;
                let current = assignment_timing::load_postgres_assignment_timing(
                    &mut transaction,
                    tenant,
                    command.assignment,
                    false,
                )
                .await?
                .ok_or(StoreError::NotFound)?;
                if current.course != command.course
                    || !postgres_is_course_instructor(
                        &mut transaction,
                        tenant,
                        command.course,
                        command.actor,
                    )
                    .await?
                {
                    return Err(StoreError::NotFound);
                }
                let accessible: bool =
                    sqlx::query_scalar("SELECT public.ple_course_records_accessible($1, $2)")
                        .bind(tenant.as_uuid())
                        .bind(command.course.as_uuid())
                        .fetch_one(&mut *transaction)
                        .await
                        .map_err(map_sqlx_error)?;
                if !accessible {
                    return Err(StoreError::NotFound);
                }
                if let AssignmentPolicyExceptionTarget::Student(student) = command.exception.target
                {
                    let exists: bool = sqlx::query_scalar(
                        "SELECT EXISTS(SELECT 1 FROM enrollment WHERE tenant_id = $1 \
                 AND assignment_id = $2 AND student_id = $3)",
                    )
                    .bind(tenant.as_uuid())
                    .bind(command.assignment.as_uuid())
                    .bind(student.as_uuid())
                    .fetch_one(&mut *transaction)
                    .await
                    .map_err(map_sqlx_error)?;
                    if !exists {
                        return Err(StoreError::NotFound);
                    }
                }
                let rows = assignment_timing::load_postgres_policy_exception_identity_rows(
                    &mut transaction,
                    tenant,
                    command.assignment,
                    command.exception.id,
                    command.exception.target,
                )
                .await?;
                if rows.len() > 1 {
                    return Err(StoreError::Conflict);
                }
                let existing = rows
                    .first()
                    .map(assignment_timing::decode_postgres_policy_exception)
                    .transpose()?;
                if let Some(existing) = &existing {
                    if existing.id != command.exception.id
                        || existing.target != command.exception.target
                    {
                        return Err(StoreError::Conflict);
                    }
                    if existing == &command.exception {
                        transaction.commit().await.map_err(map_sqlx_error)?;
                        return Ok(StoredAssignmentPolicyException {
                            exception: existing.clone(),
                            assignment_revision: current.revision,
                        });
                    }
                }
                if current.revision != command.expected_revision {
                    return Err(StoreError::Conflict);
                }
                let active_rows = assignment_timing::lock_postgres_active_timing_rows(
                    &mut transaction,
                    tenant,
                    command.assignment,
                )
                .await?;
                let locked = assignment_timing::load_postgres_assignment_timing(
                    &mut transaction,
                    tenant,
                    command.assignment,
                    true,
                )
                .await?
                .ok_or(StoreError::NotFound)?;
                if locked.revision != command.expected_revision || locked.course != command.course {
                    return Err(StoreError::Conflict);
                }
                let (available_mode, available_at) =
                    assignment_timing::postgres_exception_timestamp_columns(
                        command.exception.available_at,
                    );
                let (closes_mode, closes_at) =
                    assignment_timing::postgres_exception_timestamp_columns(
                        command.exception.closes_at,
                    );
                let (time_limit_mode, time_limit_seconds) =
                    assignment_timing::postgres_exception_limit_columns(
                        command.exception.time_limit_seconds,
                    );
                let (attempt_limit_mode, attempt_limit) =
                    assignment_timing::postgres_exception_limit_columns(
                        command.exception.attempt_limit,
                    );
                let (student_id, course_group_id) = match command.exception.target {
                    AssignmentPolicyExceptionTarget::Student(student) => {
                        (Some(student.as_uuid()), None)
                    }
                    AssignmentPolicyExceptionTarget::CourseGroup(group) => {
                        (None, Some(group.as_uuid()))
                    }
                };
                if existing.is_some() {
                    let updated = sqlx::query(
                        "UPDATE assignment_policy_exception SET available_mode = $3, \
                 available_at = TIMESTAMPTZ 'epoch' + $4::bigint * INTERVAL '1 millisecond', \
                 closes_mode = $5, \
                 closes_at = TIMESTAMPTZ 'epoch' + $6::bigint * INTERVAL '1 millisecond', \
                 time_limit_mode = $7, time_limit_seconds = $8, \
                 attempt_limit_mode = $9, attempt_limit = $10, revision = revision + 1, \
                 updated_at = transaction_timestamp() \
                 WHERE tenant_id = $1 AND assignment_policy_exception_id = $2",
                    )
                    .bind(tenant.as_uuid())
                    .bind(command.exception.id.as_uuid())
                    .bind(available_mode)
                    .bind(available_at)
                    .bind(closes_mode)
                    .bind(closes_at)
                    .bind(time_limit_mode)
                    .bind(time_limit_seconds)
                    .bind(attempt_limit_mode)
                    .bind(attempt_limit)
                    .execute(&mut *transaction)
                    .await
                    .map_err(map_sqlx_error)?;
                    if updated.rows_affected() != 1 {
                        return Err(StoreError::Conflict);
                    }
                } else {
                    sqlx::query(
                        "INSERT INTO assignment_policy_exception \
                 (tenant_id, assignment_policy_exception_id, course_id, assignment_id, \
                  student_id, course_group_id, available_mode, available_at, closes_mode, \
                  closes_at, time_limit_mode, time_limit_seconds, attempt_limit_mode, \
                  attempt_limit) VALUES ($1, $2, $3, $4, $5, $6, $7, \
                  TIMESTAMPTZ 'epoch' + $8::bigint * INTERVAL '1 millisecond', $9, \
                  TIMESTAMPTZ 'epoch' + $10::bigint * INTERVAL '1 millisecond', \
                  $11, $12, $13, $14)",
                    )
                    .bind(tenant.as_uuid())
                    .bind(command.exception.id.as_uuid())
                    .bind(command.course.as_uuid())
                    .bind(command.assignment.as_uuid())
                    .bind(student_id)
                    .bind(course_group_id)
                    .bind(available_mode)
                    .bind(available_at)
                    .bind(closes_mode)
                    .bind(closes_at)
                    .bind(time_limit_mode)
                    .bind(time_limit_seconds)
                    .bind(attempt_limit_mode)
                    .bind(attempt_limit)
                    .execute(&mut *transaction)
                    .await
                    .map_err(map_sqlx_error)?;
                }
                let revision = locked.revision.next()?;
                assignment_timing::update_postgres_assignment_revision(
                    &mut transaction,
                    tenant,
                    command.assignment,
                    locked.revision,
                    revision,
                )
                .await?;
                let now = database_timestamp(&mut transaction).await?;
                assignment_timing::apply_postgres_locked_timing_rows(
                    &mut transaction,
                    tenant,
                    command.assignment,
                    None,
                    now,
                    active_rows,
                )
                .await?;
                transaction.commit().await.map_err(map_sqlx_error)?;
                Ok(StoredAssignmentPolicyException {
                    exception: command.exception,
                    assignment_revision: revision,
                })
            }
        })
        .await
    }
    async fn delete_assignment_policy_exception_impl(
        &self,
        context: TenantContext,
        command: DeleteAssignmentPolicyExceptionCommand,
    ) -> Result<AssignmentRevision, StoreError> {
        retry_transaction(|| async move {
            let tenant = context.tenant_id();
            let mut transaction = self.begin_tenant(context).await?;
            let initial_row = sqlx::query(
                "SELECT assignment_policy_exception_id, student_id, course_group_id, \
                    available_mode, \
                    floor(extract(epoch FROM available_at) * 1000)::bigint AS available_at_millis, \
                    closes_mode, \
                    floor(extract(epoch FROM closes_at) * 1000)::bigint AS closes_at_millis, \
                    time_limit_mode, time_limit_seconds, attempt_limit_mode, attempt_limit \
             FROM assignment_policy_exception WHERE tenant_id = $1 AND assignment_id = $2 \
               AND assignment_policy_exception_id = $3",
            )
            .bind(tenant.as_uuid())
            .bind(command.assignment.as_uuid())
            .bind(command.exception.as_uuid())
            .fetch_optional(&mut *transaction)
            .await
            .map_err(map_sqlx_error)?
            .ok_or(StoreError::NotFound)?;
            let initial = assignment_timing::decode_postgres_policy_exception(&initial_row)?;
            if let AssignmentPolicyExceptionTarget::CourseGroup(group) = initial.target {
                let course: Option<Uuid> = sqlx::query_scalar(
                    "SELECT course_id FROM course_group WHERE tenant_id = $1 \
                 AND course_group_id = $2 FOR UPDATE",
                )
                .bind(tenant.as_uuid())
                .bind(group.as_uuid())
                .fetch_optional(&mut *transaction)
                .await
                .map_err(map_sqlx_error)?;
                if course != Some(command.course.as_uuid()) {
                    return Err(StoreError::NotFound);
                }
            }
            assignment_timing::lock_postgres_assignment_policy(
                &mut transaction,
                tenant,
                command.assignment,
            )
            .await?;
            let current = assignment_timing::load_postgres_assignment_timing(
                &mut transaction,
                tenant,
                command.assignment,
                false,
            )
            .await?
            .ok_or(StoreError::NotFound)?;
            if current.course != command.course
                || !postgres_is_course_instructor(
                    &mut transaction,
                    tenant,
                    command.course,
                    command.actor,
                )
                .await?
            {
                return Err(StoreError::NotFound);
            }
            let accessible: bool =
                sqlx::query_scalar("SELECT public.ple_course_records_accessible($1, $2)")
                    .bind(tenant.as_uuid())
                    .bind(command.course.as_uuid())
                    .fetch_one(&mut *transaction)
                    .await
                    .map_err(map_sqlx_error)?;
            if !accessible {
                return Err(StoreError::NotFound);
            }
            if current.revision != command.expected_revision {
                return Err(StoreError::Conflict);
            }
            let active_rows = assignment_timing::lock_postgres_active_timing_rows(
                &mut transaction,
                tenant,
                command.assignment,
            )
            .await?;
            let locked = assignment_timing::load_postgres_assignment_timing(
                &mut transaction,
                tenant,
                command.assignment,
                true,
            )
            .await?
            .ok_or(StoreError::NotFound)?;
            if locked.revision != command.expected_revision || locked.course != command.course {
                return Err(StoreError::Conflict);
            }
            let deleted = sqlx::query(
                "DELETE FROM assignment_policy_exception WHERE tenant_id = $1 \
             AND assignment_id = $2 AND assignment_policy_exception_id = $3",
            )
            .bind(tenant.as_uuid())
            .bind(command.assignment.as_uuid())
            .bind(command.exception.as_uuid())
            .execute(&mut *transaction)
            .await
            .map_err(map_sqlx_error)?;
            if deleted.rows_affected() != 1 {
                return Err(StoreError::Conflict);
            }
            let revision = locked.revision.next()?;
            assignment_timing::update_postgres_assignment_revision(
                &mut transaction,
                tenant,
                command.assignment,
                locked.revision,
                revision,
            )
            .await?;
            let now = database_timestamp(&mut transaction).await?;
            assignment_timing::apply_postgres_locked_timing_rows(
                &mut transaction,
                tenant,
                command.assignment,
                None,
                now,
                active_rows,
            )
            .await?;
            transaction.commit().await.map_err(map_sqlx_error)?;
            Ok(revision)
        })
        .await
    }
    async fn get_assignment_policy_exception_impl(
        &self,
        context: TenantContext,
        assignment: AssignmentId,
        exception: AssignmentPolicyExceptionId,
    ) -> Result<Option<StoredAssignmentPolicyException>, StoreError> {
        let tenant = context.tenant_id();
        let mut transaction = self.begin_tenant(context).await?;
        let row = sqlx::query(
            "SELECT assignment_policy_exception_id, student_id, course_group_id, \
                    available_mode, \
                    floor(extract(epoch FROM available_at) * 1000)::bigint AS available_at_millis, \
                    closes_mode, \
                    floor(extract(epoch FROM closes_at) * 1000)::bigint AS closes_at_millis, \
                    time_limit_mode, time_limit_seconds, attempt_limit_mode, attempt_limit \
             FROM assignment_policy_exception WHERE tenant_id = $1 AND assignment_id = $2 \
               AND assignment_policy_exception_id = $3",
        )
        .bind(tenant.as_uuid())
        .bind(assignment.as_uuid())
        .bind(exception.as_uuid())
        .fetch_optional(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;
        let result = if let Some(row) = row {
            let timing = assignment_timing::load_postgres_assignment_timing(
                &mut transaction,
                tenant,
                assignment,
                false,
            )
            .await?
            .ok_or(StoreError::NotFound)?;
            Some(StoredAssignmentPolicyException {
                exception: assignment_timing::decode_postgres_policy_exception(&row)?,
                assignment_revision: timing.revision,
            })
        } else {
            None
        };
        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(result)
    }
    async fn resolve_assignment_timing_impl(
        &self,
        context: TenantContext,
        assignment: AssignmentId,
        student: StudentId,
    ) -> Result<Option<ResolvedAssignmentTiming>, StoreError> {
        let tenant = context.tenant_id();
        let mut transaction = self.begin_tenant(context).await?;
        let enrollment = assignment_timing::load_postgres_enrollment_by_student(
            &mut transaction,
            tenant,
            assignment,
            student,
        )
        .await?;
        let Some(enrollment) = enrollment else {
            transaction.commit().await.map_err(map_sqlx_error)?;
            return Ok(None);
        };
        let timing = assignment_timing::load_postgres_assignment_timing(
            &mut transaction,
            tenant,
            assignment,
            false,
        )
        .await?
        .ok_or(StoreError::NotFound)?;
        let resolved = assignment_timing::load_postgres_resolved_assignment_policy(
            &mut transaction,
            tenant,
            assignment,
            &enrollment,
            Some(timing.policy),
        )
        .await?;
        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(Some(ResolvedAssignmentTiming {
            tenant,
            course: timing.course,
            assignment,
            student,
            policy: resolved.policy,
            contributors: resolved.contributors,
            revision: timing.revision,
        }))
    }
    async fn get_attempt_resolved_timing_impl(
        &self,
        context: TenantContext,
        attempt: QuestionAttemptId,
    ) -> Result<Option<ResolvedAttemptTiming>, StoreError> {
        let mut transaction = self.begin_tenant(context).await?;
        let row = sqlx::query(
            "SELECT resolved_visible, \
                    floor(extract(epoch FROM resolved_available_at) * 1000)::bigint AS available_at_millis, \
                    floor(extract(epoch FROM resolved_due_at) * 1000)::bigint AS due_at_millis, \
                    floor(extract(epoch FROM resolved_closes_at) * 1000)::bigint AS closes_at_millis, \
                    resolved_late_submission_policy, resolved_time_limit_seconds, \
                    resolved_attempt_limit, resolution_sources \
             FROM attempt_timing_current WHERE tenant_id = $1 AND attempt_id = $2",
        )
        .bind(context.tenant_id().as_uuid())
        .bind(attempt.as_uuid())
        .fetch_optional(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;
        let result = row
            .as_ref()
            .map(|row| assignment_timing::decode_postgres_resolved_attempt_timing(row, attempt))
            .transpose()?;
        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(result)
    }
}

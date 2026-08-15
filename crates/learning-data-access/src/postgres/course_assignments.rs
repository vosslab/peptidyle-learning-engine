use async_trait::async_trait;

use super::*;

#[async_trait]
impl crate::CourseAssignmentStore for PostgresStore {
    async fn create_assignment_with_timing_impl(
        &self,
        context: TenantContext,
        assignment: AssignmentRecord,
        assignment_timing: question_model::AssignmentRunTiming,
    ) -> Result<StoredAssignment, StoreError> {
        ensure_tenant(context, assignment.tenant)?;
        validate_assignment(&assignment)?;
        validate_assignment_timing(AssignmentTimingPolicy {
            time_limit_seconds: assignment_timing.time_limit_seconds,
            ..AssignmentTimingPolicy::default()
        })?;
        let (completion_policy, completion_threshold) =
            completion_policy_columns(assignment.policies.completion);
        let (practice_policy, practice_limit) =
            continued_practice_columns(assignment.policies.continued_practice)?;
        let mut transaction = self.begin_tenant(context).await?;
        super::course_roster::lock_course_roster_cross_product(
            &mut transaction,
            assignment.tenant,
            assignment.course_id,
        )
        .await?;
        validate_postgres_assignment_references(&mut transaction, context, &assignment).await?;
        let inserted = sqlx::query(
            "INSERT INTO assignment \
             (tenant_id, assignment_id, course_id, title, completion_policy, \
              completion_threshold, attempt_selection_policy, continued_practice_policy, \
              practice_max_additional_runs, variation_policy, lifecycle, visible, \
              auto_submit, time_limit_seconds, revision) \
             VALUES ($1, $2, $3, $4, $5, $6::numeric, $7, $8, $9, $10, \
                     'published', true, true, $11, 1) \
             ON CONFLICT (tenant_id, assignment_id) DO NOTHING \
             RETURNING revision, scoring_generation, scoring_status",
        )
        .bind(assignment.tenant.as_uuid())
        .bind(assignment.id.as_uuid())
        .bind(assignment.course_id.as_uuid())
        .bind(&assignment.title)
        .bind(completion_policy)
        .bind(completion_threshold)
        .bind(grade_policy_name(assignment.policies.grade))
        .bind(practice_policy)
        .bind(practice_limit)
        .bind(variation_policy_name(assignment.policies.variation))
        .bind(assignment_timing.time_limit_seconds.map(i64::from))
        .fetch_optional(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;
        let Some(row) = inserted else {
            return Err(StoreError::AlreadyExists);
        };
        insert_postgres_assignment_items(&mut transaction, &assignment).await?;
        super::course_roster::reconcile_new_assignment(&mut transaction, &assignment).await?;
        let revision =
            AssignmentRevision::from_stored(row.try_get("revision").map_err(map_sqlx_error)?)?;
        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(StoredAssignment {
            record: assignment,
            revision,
            assignment_timing,
            scoring_generation: decode_scoring_generation(&row)?,
            scoring_status: decode_scoring_status(&row)?,
        })
    }
    async fn replace_assignment_with_timing_impl(
        &self,
        context: TenantContext,
        course: CourseId,
        assignment: AssignmentId,
        expected_revision: AssignmentRevision,
        update: AssignmentEditorUpdate,
    ) -> Result<StoredAssignment, StoreError> {
        validate_assignment_timing(AssignmentTimingPolicy {
            time_limit_seconds: update.assignment_timing.time_limit_seconds,
            ..AssignmentTimingPolicy::default()
        })?;
        let assignment = AssignmentRecord {
            id: assignment,
            tenant: context.tenant_id(),
            course_id: course,
            title: update.assignment.title.clone(),
            items: update.assignment.items.clone(),
            selection_groups: update.assignment.selection_groups.clone(),
            policies: update.assignment.policies,
        };
        validate_assignment(&assignment)?;
        let (completion_policy, completion_threshold) =
            completion_policy_columns(assignment.policies.completion);
        let (practice_policy, practice_limit) =
            continued_practice_columns(assignment.policies.continued_practice)?;
        let mut transaction = self.begin_tenant(context).await?;
        validate_postgres_assignment_references(&mut transaction, context, &assignment).await?;
        // This advisory lock serializes assignment definition, timing, and
        // accommodation edits before their differing row-level work begins.
        // A content-only save therefore need not lock active attempts.
        assignment_timing::lock_postgres_assignment_policy(
            &mut transaction,
            assignment.tenant,
            assignment.id,
        )
        .await?;
        let previous = load_assignment(&mut transaction, assignment.tenant, assignment.id).await?;
        crate::ensure_assignment_update_preserves_references(&previous, &update.assignment)?;
        let locked_timing = assignment_timing::load_postgres_assignment_timing(
            &mut transaction,
            assignment.tenant,
            assignment.id,
            true,
        )
        .await?
        .ok_or(StoreError::NotFound)?;
        if locked_timing.revision != expected_revision {
            return Err(StoreError::Conflict);
        }
        if locked_timing.policy.time_limit_seconds != update.assignment_timing.time_limit_seconds {
            // The assignment advisory lock is held before this active-row
            // lock. Timing/accommodation writers take the same advisory lock,
            // so no editor writer can invert this conditional order.
            let active_rows = assignment_timing::lock_postgres_active_timing_rows(
                &mut transaction,
                assignment.tenant,
                assignment.id,
            )
            .await?;
            let now = database_timestamp(&mut transaction).await?;
            assignment_timing::apply_postgres_locked_timing_rows(
                &mut transaction,
                assignment.tenant,
                assignment.id,
                Some(AssignmentTimingPolicy {
                    time_limit_seconds: update.assignment_timing.time_limit_seconds,
                    ..locked_timing.policy
                }),
                now,
                active_rows,
            )
            .await?;
        }
        let scoring_changed = assignment_scoring_changed(&previous, &assignment);
        let has_scores: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM attempt_score_current \
             WHERE tenant_id = $1 AND assignment_id = $2)",
        )
        .bind(assignment.tenant.as_uuid())
        .bind(assignment.id.as_uuid())
        .fetch_one(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;
        let row = sqlx::query(
            "UPDATE assignment SET title = $4, completion_policy = $5, \
                    completion_threshold = $6::numeric, attempt_selection_policy = $7, \
                    continued_practice_policy = $8, practice_max_additional_runs = $9, \
                    variation_policy = $10, time_limit_seconds = $14, \
                    scoring_generation = scoring_generation + CASE WHEN $11 THEN 1 ELSE 0 END, \
                    scoring_status = CASE WHEN $11 \
                        THEN CASE WHEN $12 THEN 'recalculating' ELSE 'current' END \
                        ELSE scoring_status END, \
                    revision = revision + 1, updated_at = transaction_timestamp() \
             WHERE tenant_id = $1 AND assignment_id = $2 AND course_id = $3 AND revision = $13 \
             RETURNING revision, scoring_generation, scoring_status, time_limit_seconds",
        )
        .bind(assignment.tenant.as_uuid())
        .bind(assignment.id.as_uuid())
        .bind(assignment.course_id.as_uuid())
        .bind(&assignment.title)
        .bind(completion_policy)
        .bind(completion_threshold)
        .bind(grade_policy_name(assignment.policies.grade))
        .bind(practice_policy)
        .bind(practice_limit)
        .bind(variation_policy_name(assignment.policies.variation))
        .bind(scoring_changed)
        .bind(has_scores)
        .bind(i64::try_from(expected_revision.value()).map_err(|_| StoreError::Conflict)?)
        .bind(update.assignment_timing.time_limit_seconds.map(i64::from))
        .fetch_optional(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;
        let Some(row) = row else {
            let exists: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM assignment WHERE tenant_id = $1 AND assignment_id = $2 AND course_id = $3)",
            )
            .bind(assignment.tenant.as_uuid())
            .bind(assignment.id.as_uuid())
            .bind(assignment.course_id.as_uuid())
            .fetch_one(&mut *transaction)
            .await
            .map_err(map_sqlx_error)?;
            return Err(if exists {
                StoreError::Conflict
            } else {
                StoreError::NotFound
            });
        };
        let scoring_generation = decode_scoring_generation(&row)?;
        let scoring_status = decode_scoring_status(&row)?;
        replace_postgres_assignment_items(&mut transaction, &assignment).await?;
        if scoring_status == ScoringStatus::Recalculating {
            let job = JobId::generate()?;
            let payload = serde_json::to_value(JobPayload::RecalculateAssignment {
                assignment: assignment.id,
                generation: scoring_generation,
            })
            .map_err(|error| {
                StoreError::InvalidRecord(format!(
                    "assignment scoring job serialization failed: {error}"
                ))
            })?;
            sqlx::query(
                "INSERT INTO worker_job (job_id, tenant_id, payload, state, max_attempts) \
                 VALUES ($1, $2, $3, 'ready', 10)",
            )
            .bind(job.as_uuid())
            .bind(assignment.tenant.as_uuid())
            .bind(payload)
            .execute(&mut *transaction)
            .await
            .map_err(map_sqlx_error)?;
        }
        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(StoredAssignment {
            record: assignment,
            revision: AssignmentRevision::from_stored(
                row.try_get("revision").map_err(map_sqlx_error)?,
            )?,
            assignment_timing: update.assignment_timing,
            scoring_generation,
            scoring_status,
        })
    }
    async fn replace_assignment_fixed_item_impl(
        &self,
        context: TenantContext,
        command: ReplaceAssignmentFixedItemCommand,
    ) -> Result<StoredAssignment, StoreError> {
        let mut transaction = self.begin_tenant(context).await?;
        let (current, _) = lock_fixed_item_assignment(
            &mut transaction,
            context,
            command.course,
            command.assignment,
            command.expected_revision,
        )
        .await?;
        let replacement = current
            .items
            .iter()
            .find(|item| item.id == command.current_item)
            .cloned()
            .ok_or(StoreError::NotFound)?;
        let mut replacement = replacement;
        replacement.reference = command.replacement;
        let updated = AssignmentRecord {
            items: current
                .items
                .iter()
                .map(|item| {
                    if item.id == command.current_item {
                        replacement.clone()
                    } else {
                        item.clone()
                    }
                })
                .collect(),
            ..current.clone()
        };
        validate_postgres_assignment_references(&mut transaction, context, &updated).await?;
        sqlx::query_scalar::<_, ()>(
            "SELECT ple_replace_assignment_fixed_item($1, $2, $3, $4, $5, $6, $7)",
        )
        .bind(context.tenant_id().as_uuid())
        .bind(command.course.as_uuid())
        .bind(command.assignment.as_uuid())
        .bind(i64::try_from(command.expected_revision.value()).map_err(|_| StoreError::Conflict)?)
        .bind(command.current_item.as_uuid())
        .bind(command.replacement.problem.as_uuid())
        .bind(command.replacement.version.as_uuid())
        .fetch_one(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;
        let stored =
            load_fixed_item_assignment(&mut transaction, context.tenant_id(), command.assignment)
                .await?;
        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(stored)
    }

    async fn add_assignment_fixed_item_impl(
        &self,
        context: TenantContext,
        command: AddAssignmentFixedItemCommand,
    ) -> Result<StoredAssignment, StoreError> {
        let mut transaction = self.begin_tenant(context).await?;
        let (current, _) = lock_fixed_item_assignment(
            &mut transaction,
            context,
            command.course,
            command.assignment,
            command.expected_revision,
        )
        .await?;
        if current.items.iter().any(|item| item.id == command.item.id) {
            return Err(StoreError::InvalidRecord(
                "new fixed item uses a fresh assignment item identity".to_string(),
            ));
        }
        let mut updated = current.clone();
        for existing in &mut updated.items {
            if existing.position >= command.item.position {
                existing.position = existing.position.checked_add(1).ok_or_else(|| {
                    StoreError::InvalidRecord("assignment item position is too large".to_string())
                })?;
            }
        }
        for group in &mut updated.selection_groups {
            if group.position >= command.item.position {
                group.position = group.position.checked_add(1).ok_or_else(|| {
                    StoreError::InvalidRecord("selection group position is too large".to_string())
                })?;
            }
        }
        updated.items.push(command.item.clone());
        validate_assignment(&updated)?;
        validate_postgres_assignment_references(&mut transaction, context, &updated).await?;
        sqlx::query_scalar::<_, ()>(
            "SELECT ple_add_assignment_fixed_item($1, $2, $3, $4, $5, $6, $7, $8, $9::numeric, $10, $11)",
        )
        .bind(context.tenant_id().as_uuid())
        .bind(command.course.as_uuid())
        .bind(command.assignment.as_uuid())
        .bind(i64::try_from(command.expected_revision.value()).map_err(|_| StoreError::Conflict)?)
        .bind(command.item.id.as_uuid())
        .bind(i32::try_from(command.item.position).map_err(|_| StoreError::InvalidRecord("assignment item position is too large".to_string()))?)
        .bind(command.item.reference.problem.as_uuid())
        .bind(command.item.reference.version.as_uuid())
        .bind(command.item.points_possible.to_string())
        .bind(assignment_delivery_state_name(command.item.delivery_state))
        .bind(assignment_scoring_mode_name(command.item.scoring_mode))
        .fetch_one(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;
        let stored =
            load_fixed_item_assignment(&mut transaction, context.tenant_id(), command.assignment)
                .await?;
        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(stored)
    }

    async fn remove_assignment_fixed_item_impl(
        &self,
        context: TenantContext,
        command: RemoveAssignmentFixedItemCommand,
    ) -> Result<StoredAssignment, StoreError> {
        let mut transaction = self.begin_tenant(context).await?;
        let (current, _) = lock_fixed_item_assignment(
            &mut transaction,
            context,
            command.course,
            command.assignment,
            command.expected_revision,
        )
        .await?;
        let removed = current
            .items
            .iter()
            .find(|item| item.id == command.item)
            .cloned()
            .ok_or(StoreError::NotFound)?;
        let mut updated = current.clone();
        updated.items.retain(|item| item.id != command.item);
        for existing in &mut updated.items {
            if existing.position > removed.position {
                existing.position -= 1;
            }
        }
        for group in &mut updated.selection_groups {
            if group.position > removed.position {
                group.position -= 1;
            }
        }
        validate_assignment(&updated)?;
        sqlx::query_scalar::<_, ()>("SELECT ple_remove_assignment_fixed_item($1, $2, $3, $4, $5)")
            .bind(context.tenant_id().as_uuid())
            .bind(command.course.as_uuid())
            .bind(command.assignment.as_uuid())
            .bind(
                i64::try_from(command.expected_revision.value())
                    .map_err(|_| StoreError::Conflict)?,
            )
            .bind(command.item.as_uuid())
            .fetch_one(&mut *transaction)
            .await
            .map_err(map_sqlx_error)?;
        let stored =
            load_fixed_item_assignment(&mut transaction, context.tenant_id(), command.assignment)
                .await?;
        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(stored)
    }
    async fn delete_and_regrade_assignment_item_impl(
        &self,
        context: TenantContext,
        command: DeleteAndRegradeAssignmentItemCommand,
    ) -> Result<StoredAssignment, StoreError> {
        let stored = self
            .get_assignment_for_edit(context, command.assignment)
            .await?
            .ok_or(StoreError::NotFound)?;
        if stored.record.course_id != command.course {
            return Err(StoreError::NotFound);
        }
        if stored.revision != command.expected_revision {
            return Err(StoreError::Conflict);
        }
        let Some(update) = delete_and_regrade_update(&stored, command.item)? else {
            return Ok(stored);
        };
        self.replace_assignment_with_timing(
            context,
            command.course,
            command.assignment,
            command.expected_revision,
            AssignmentEditorUpdate {
                assignment: update,
                assignment_timing: stored.assignment_timing,
            },
        )
        .await
    }
    async fn get_assignment_for_edit_impl(
        &self,
        context: TenantContext,
        assignment: AssignmentId,
    ) -> Result<Option<StoredAssignment>, StoreError> {
        let mut transaction = self.begin_tenant(context).await?;
        let row = sqlx::query(
            "SELECT assignment_id, course_id, title, completion_policy, \
                    completion_threshold::text AS completion_threshold, \
                    attempt_selection_policy, continued_practice_policy, \
                    practice_max_additional_runs, variation_policy, revision, \
                    scoring_generation, scoring_status, time_limit_seconds \
             FROM assignment \
             WHERE tenant_id = $1 AND assignment_id = $2 FOR SHARE",
        )
        .bind(context.tenant_id().as_uuid())
        .bind(assignment.as_uuid())
        .fetch_optional(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;
        let result = match row.as_ref() {
            Some(row) => Some(StoredAssignment {
                record: load_assignment_relations(
                    &mut transaction,
                    decode_assignment_header(row, context.tenant_id())?,
                )
                .await?,
                revision: AssignmentRevision::from_stored(
                    row.try_get("revision").map_err(map_sqlx_error)?,
                )?,
                assignment_timing: question_model::AssignmentRunTiming {
                    time_limit_seconds: assignment_timing::decode_postgres_assignment_time_limit(
                        row.try_get::<Option<i32>, _>("time_limit_seconds")
                            .map_err(map_sqlx_error)?,
                    )?,
                },
                scoring_generation: decode_scoring_generation(row)?,
                scoring_status: decode_scoring_status(row)?,
            }),
            None => None,
        };
        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(result)
    }
    async fn get_assignment_impl(
        &self,
        context: TenantContext,
        assignment: AssignmentId,
    ) -> Result<Option<AssignmentRecord>, StoreError> {
        let mut transaction = self.begin_tenant(context).await?;
        let row = sqlx::query(
            "SELECT assignment_id, course_id, title, completion_policy, \
                    completion_threshold::text AS completion_threshold, \
                    attempt_selection_policy, continued_practice_policy, \
                    practice_max_additional_runs, variation_policy \
             FROM assignment \
             WHERE tenant_id = $1 AND assignment_id = $2",
        )
        .bind(context.tenant_id().as_uuid())
        .bind(assignment.as_uuid())
        .fetch_optional(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;
        let record = match row.as_ref() {
            Some(row) => Some(
                load_assignment_relations(
                    &mut transaction,
                    decode_assignment_header(row, context.tenant_id())?,
                )
                .await?,
            ),
            None => None,
        };
        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(record)
    }
    async fn list_assignments_impl(
        &self,
        context: TenantContext,
        course: CourseId,
        page: PageRequest,
    ) -> Result<Page<AssignmentRecord>, StoreError> {
        let cursor = page.after.as_ref().map(|value| value.as_str().to_string());
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
        let rows = sqlx::query(
            "SELECT assignment_id::text AS stable_key, assignment_id, course_id, title, \
                    completion_policy, completion_threshold::text AS completion_threshold, \
                    attempt_selection_policy, continued_practice_policy, \
                    practice_max_additional_runs, variation_policy \
             FROM assignment \
             WHERE tenant_id = $1 AND course_id = $2 \
               AND ($3::text IS NULL OR assignment_id::text > $3) \
             ORDER BY assignment_id::text LIMIT $4",
        )
        .bind(context.tenant_id().as_uuid())
        .bind(course.as_uuid())
        .bind(cursor)
        .bind(limit)
        .fetch_all(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;
        let mut records = Vec::with_capacity(rows.len());
        for row in &rows {
            let key: String = row.try_get("stable_key").map_err(map_sqlx_error)?;
            let header = decode_assignment_header(row, context.tenant_id())?;
            records.push((
                key,
                load_assignment_relations(&mut transaction, header).await?,
            ));
        }
        let result = page_from_keyed_records(&mut records, page.size.get())?;
        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(result)
    }
    async fn create_enrollment_impl(
        &self,
        context: TenantContext,
        enrollment: AssignmentEnrollment,
    ) -> Result<(), StoreError> {
        ensure_tenant(context, enrollment.tenant)?;
        let summary = StudentAssignmentSummary::empty(enrollment.tenant, enrollment.id);
        let (enrollment_payload, enrollment_checksum) = encode_payload(&enrollment)?;
        let (summary_payload, summary_checksum) = encode_payload(&summary)?;
        let mut transaction = self.begin_tenant(context).await?;
        let eligible_assignment: bool = sqlx::query_scalar(
            "SELECT EXISTS( \
                 SELECT 1 FROM assignment AS a \
                 JOIN course_member AS cm \
                   ON cm.tenant_id = a.tenant_id AND cm.course_id = a.course_id \
                 WHERE a.tenant_id = $1 AND a.assignment_id = $2 \
                   AND cm.user_id = $3 AND cm.role = 'student' \
             )",
        )
        .bind(enrollment.tenant.as_uuid())
        .bind(enrollment.assignment.as_uuid())
        .bind(enrollment.user.as_uuid())
        .fetch_one(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;
        if !eligible_assignment {
            return Err(StoreError::InvalidRecord(
                "enrollment user must be a student member of the assignment course".to_string(),
            ));
        }
        sqlx::query(
            "INSERT INTO enrollment \
             (tenant_id, enrollment_id, assignment_id, user_id, student_id, payload, payload_sha256) \
             VALUES ($1, $2, $3, $4, $5, $6, $7)",
        )
        .bind(enrollment.tenant.as_uuid())
        .bind(enrollment.id.as_uuid())
        .bind(enrollment.assignment.as_uuid())
        .bind(enrollment.user.as_uuid())
        .bind(enrollment.student.as_uuid())
        .bind(enrollment_payload)
        .bind(enrollment_checksum)
        .execute(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;
        sqlx::query(
            "INSERT INTO student_assignment_summary \
             (tenant_id, enrollment_id, payload, payload_sha256) VALUES ($1, $2, $3, $4)",
        )
        .bind(enrollment.tenant.as_uuid())
        .bind(enrollment.id.as_uuid())
        .bind(summary_payload)
        .bind(summary_checksum)
        .execute(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;
        transaction.commit().await.map_err(map_sqlx_error)
    }
    async fn get_enrollment_impl(
        &self,
        context: TenantContext,
        enrollment: EnrollmentId,
    ) -> Result<Option<AssignmentEnrollment>, StoreError> {
        let mut transaction = self.begin_tenant(context).await?;
        let row = sqlx::query(
            "SELECT payload, payload_sha256 FROM enrollment \
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

#[cfg(feature = "postgres")]
async fn lock_fixed_item_assignment(
    transaction: &mut Transaction<'_, Postgres>,
    context: TenantContext,
    course: CourseId,
    assignment: AssignmentId,
    expected_revision: AssignmentRevision,
) -> Result<(AssignmentRecord, StoredAssignmentTiming), StoreError> {
    assignment_timing::lock_postgres_assignment_policy(
        transaction,
        context.tenant_id(),
        assignment,
    )
    .await?;
    let current = load_assignment(transaction, context.tenant_id(), assignment).await?;
    if current.course_id != course {
        return Err(StoreError::NotFound);
    }
    let timing = assignment_timing::load_postgres_assignment_timing(
        transaction,
        context.tenant_id(),
        assignment,
        true,
    )
    .await?
    .ok_or(StoreError::NotFound)?;
    if timing.revision != expected_revision {
        return Err(StoreError::Conflict);
    }
    Ok((current, timing))
}

#[cfg(feature = "postgres")]
async fn load_fixed_item_assignment(
    transaction: &mut Transaction<'_, Postgres>,
    tenant: TenantId,
    assignment: AssignmentId,
) -> Result<StoredAssignment, StoreError> {
    let row = sqlx::query(
        "SELECT assignment_id, course_id, title, completion_policy, \
                completion_threshold::text AS completion_threshold, \
                attempt_selection_policy, continued_practice_policy, \
                practice_max_additional_runs, variation_policy, revision, \
                scoring_generation, scoring_status, time_limit_seconds \
         FROM assignment WHERE tenant_id = $1 AND assignment_id = $2 FOR SHARE",
    )
    .bind(tenant.as_uuid())
    .bind(assignment.as_uuid())
    .fetch_optional(&mut **transaction)
    .await
    .map_err(map_sqlx_error)?
    .ok_or(StoreError::NotFound)?;
    Ok(StoredAssignment {
        record: load_assignment_relations(transaction, decode_assignment_header(&row, tenant)?)
            .await?,
        revision: AssignmentRevision::from_stored(
            row.try_get("revision").map_err(map_sqlx_error)?,
        )?,
        assignment_timing: question_model::AssignmentRunTiming {
            time_limit_seconds: assignment_timing::decode_postgres_assignment_time_limit(
                row.try_get::<Option<i32>, _>("time_limit_seconds")
                    .map_err(map_sqlx_error)?,
            )?,
        },
        scoring_generation: decode_scoring_generation(&row)?,
        scoring_status: decode_scoring_status(&row)?,
    })
}

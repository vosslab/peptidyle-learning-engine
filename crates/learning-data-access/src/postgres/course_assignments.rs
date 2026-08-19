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
        validate_editor_time_limit(assignment_timing.time_limit_seconds)?;
        let (completion_policy, completion_threshold) =
            completion_policy_columns(assignment.policies.completion);
        let (practice_policy, practice_limit) =
            continued_practice_columns(assignment.policies.continued_practice)?;
        let mut transaction = self.begin_tenant(context).await?;
        validate_postgres_assignment_references(&mut transaction, context, &assignment).await?;
        let inserted = sqlx::query(
            "INSERT INTO assignment \
             (tenant_id, assignment_id, course_id, title, completion_policy, \
              completion_threshold, attempt_selection_policy, continued_practice_policy, \
              practice_max_additional_runs, variation_policy, lifecycle, audience_kind, revision) \
             VALUES ($1, $2, $3, $4, $5, $6::numeric, $7, $8, $9, $10, \
                     'published', $11, 1) \
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
        .bind(assignment_audience_kind(&assignment.audience))
        .fetch_optional(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;
        let Some(row) = inserted else {
            return Err(StoreError::AlreadyExists);
        };
        insert_postgres_assignment_items(&mut transaction, &assignment).await?;
        replace_postgres_assignment_audience(&mut transaction, &assignment).await?;
        insert_editor_base_policy(
            &mut transaction,
            assignment.tenant,
            assignment.id,
            assignment.course_id,
            assignment_timing.time_limit_seconds,
        )
        .await?;
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
        validate_editor_time_limit(update.assignment_timing.time_limit_seconds)?;
        let assignment = AssignmentRecord {
            id: assignment,
            tenant: context.tenant_id(),
            course_id: course,
            title: update.assignment.title.clone(),
            audience: update.assignment.audience.clone(),
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
                    variation_policy = $10, audience_kind = $14, \
                    scoring_generation = scoring_generation + CASE WHEN $11 THEN 1 ELSE 0 END, \
                    scoring_status = CASE WHEN $11 \
                        THEN CASE WHEN $12 THEN 'recalculating' ELSE 'current' END \
                        ELSE scoring_status END, \
                    revision = revision + 1, updated_at = transaction_timestamp() \
             WHERE tenant_id = $1 AND assignment_id = $2 AND course_id = $3 AND revision = $13 \
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
        .bind(scoring_changed)
        .bind(has_scores)
        .bind(i64::try_from(expected_revision.value()).map_err(|_| StoreError::Conflict)?)
        .bind(assignment_audience_kind(&assignment.audience))
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
        replace_postgres_assignment_audience(&mut transaction, &assignment).await?;
        update_editor_base_time_limit(
            &mut transaction,
            assignment.tenant,
            assignment.id,
            update.assignment_timing.time_limit_seconds,
        )
        .await?;
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
                    practice_max_additional_runs, variation_policy, audience_kind, revision, \
                    scoring_generation, scoring_status \
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
                    time_limit_seconds: load_editor_time_limit(
                        &mut transaction,
                        context.tenant_id(),
                        assignment,
                    )
                    .await?,
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
                    practice_max_additional_runs, variation_policy, audience_kind \
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
                    practice_max_additional_runs, variation_policy, audience_kind \
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
    async fn get_enrollment_impl(
        &self,
        context: TenantContext,
        enrollment: EnrollmentId,
    ) -> Result<Option<AssignmentEnrollment>, StoreError> {
        let mut transaction = self.begin_tenant(context).await?;
        let enrollment_id = sqlx::query_scalar::<_, Uuid>(
            "SELECT enrollment_id FROM enrollment WHERE tenant_id = $1 AND enrollment_id = $2",
        )
        .bind(context.tenant_id().as_uuid())
        .bind(enrollment.as_uuid())
        .fetch_optional(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;
        let record = match enrollment_id {
            Some(id) => Some(
                load_postgres_enrollment(
                    &mut transaction,
                    context.tenant_id(),
                    EnrollmentId::from_uuid(id),
                )
                .await?,
            ),
            None => None,
        };
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
) -> Result<(AssignmentRecord, ()), StoreError> {
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
    let revision: i64 = sqlx::query_scalar(
        "SELECT revision FROM assignment WHERE tenant_id=$1 AND assignment_id=$2 FOR UPDATE",
    )
    .bind(context.tenant_id().as_uuid())
    .bind(assignment.as_uuid())
    .fetch_optional(&mut **transaction)
    .await
    .map_err(map_sqlx_error)?
    .ok_or(StoreError::NotFound)?;
    if AssignmentRevision::from_stored(revision)? != expected_revision {
        return Err(StoreError::Conflict);
    }
    Ok((current, ()))
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
                practice_max_additional_runs, variation_policy, audience_kind, revision, \
                scoring_generation, scoring_status \
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
            time_limit_seconds: load_editor_time_limit(transaction, tenant, assignment).await?,
        },
        scoring_generation: decode_scoring_generation(&row)?,
        scoring_status: decode_scoring_status(&row)?,
    })
}

fn validate_editor_time_limit(value: Option<u32>) -> Result<(), StoreError> {
    if value == Some(0)
        || value.is_some_and(|seconds| seconds > question_model::MAX_ASSIGNMENT_TIME_LIMIT_SECONDS)
    {
        return Err(StoreError::InvalidRecord(
            "assignment time limit is invalid".to_string(),
        ));
    }
    Ok(())
}

async fn insert_editor_base_policy(
    transaction: &mut Transaction<'_, Postgres>,
    tenant: TenantId,
    assignment: AssignmentId,
    course: CourseId,
    limit: Option<u32>,
) -> Result<(), StoreError> {
    sqlx::query("INSERT INTO assignment_effective_policy_base (tenant_id, assignment_id, course_id, late_submission_policy, deadline_behavior, time_limit_seconds) VALUES ($1,$2,$3,'accept','auto_submit',$4)")
        .bind(tenant.as_uuid()).bind(assignment.as_uuid()).bind(course.as_uuid()).bind(limit.map(i32::try_from).transpose().map_err(|_| StoreError::InvalidRecord("assignment time limit is invalid".to_string()))?)
        .execute(&mut **transaction).await.map_err(map_sqlx_error)?;
    Ok(())
}

async fn update_editor_base_time_limit(
    transaction: &mut Transaction<'_, Postgres>,
    tenant: TenantId,
    assignment: AssignmentId,
    limit: Option<u32>,
) -> Result<(), StoreError> {
    let changed = sqlx::query("UPDATE assignment_effective_policy_base SET time_limit_seconds=$3, updated_at=transaction_timestamp() WHERE tenant_id=$1 AND assignment_id=$2")
        .bind(tenant.as_uuid()).bind(assignment.as_uuid()).bind(limit.map(i32::try_from).transpose().map_err(|_| StoreError::InvalidRecord("assignment time limit is invalid".to_string()))?)
        .execute(&mut **transaction).await.map_err(map_sqlx_error)?;
    if changed.rows_affected() != 1 {
        return Err(StoreError::NotFound);
    }
    Ok(())
}

async fn load_editor_time_limit(
    transaction: &mut Transaction<'_, Postgres>,
    tenant: TenantId,
    assignment: AssignmentId,
) -> Result<Option<u32>, StoreError> {
    let value: Option<i32> = sqlx::query_scalar("SELECT time_limit_seconds FROM assignment_effective_policy_base WHERE tenant_id=$1 AND assignment_id=$2")
        .bind(tenant.as_uuid()).bind(assignment.as_uuid()).fetch_optional(&mut **transaction).await.map_err(map_sqlx_error)?.ok_or(StoreError::NotFound)?;
    value
        .map(|v| {
            u32::try_from(v).map_err(|_| {
                StoreError::Unavailable("stored assignment time limit is invalid".to_string())
            })
        })
        .transpose()
}

use async_trait::async_trait;

use super::*;

#[async_trait]
impl crate::CourseAssignmentStore for PostgresStore {
    async fn create_assignment_impl(
        &self,
        context: TenantContext,
        assignment: AssignmentRecord,
    ) -> Result<StoredAssignment, StoreError> {
        ensure_tenant(context, assignment.tenant)?;
        validate_assignment(&assignment)?;
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
              auto_submit, revision) \
             VALUES ($1, $2, $3, $4, $5, $6::numeric, $7, $8, $9, $10, \
                     'published', true, true, 1) \
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
            scoring_generation: decode_scoring_generation(&row)?,
            scoring_status: decode_scoring_status(&row)?,
        })
    }
    async fn replace_assignment_impl(
        &self,
        context: TenantContext,
        course: CourseId,
        assignment: AssignmentId,
        expected_revision: AssignmentRevision,
        update: AssignmentUpdate,
    ) -> Result<StoredAssignment, StoreError> {
        let assignment = AssignmentRecord {
            id: assignment,
            tenant: context.tenant_id(),
            course_id: course,
            title: update.title,
            items: update.items,
            selection_groups: update.selection_groups,
            policies: update.policies,
        };
        validate_assignment(&assignment)?;
        let (completion_policy, completion_threshold) =
            completion_policy_columns(assignment.policies.completion);
        let (practice_policy, practice_limit) =
            continued_practice_columns(assignment.policies.continued_practice)?;
        let mut transaction = self.begin_tenant(context).await?;
        validate_postgres_assignment_references(&mut transaction, context, &assignment).await?;
        let previous = load_assignment(&mut transaction, assignment.tenant, assignment.id).await?;
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
                    variation_policy = $10, \
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
            scoring_generation,
            scoring_status,
        })
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
        self.replace_assignment(
            context,
            command.course,
            command.assignment,
            command.expected_revision,
            update,
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
                    scoring_generation, scoring_status \
             FROM assignment \
             WHERE tenant_id = $1 AND assignment_id = $2",
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

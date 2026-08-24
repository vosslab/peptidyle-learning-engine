use async_trait::async_trait;

use super::*;
use crate::{CreateAssignmentCommand, ReplaceAssignmentCommand};

#[async_trait]
impl crate::CourseAssignmentStore for PostgresStore {
    async fn create_assignment_impl(
        &self,
        context: TenantContext,
        command: CreateAssignmentCommand,
    ) -> Result<StoredAssignment, StoreError> {
        let CreateAssignmentCommand {
            actor,
            assignment,
            base_policy,
        } = command;
        ensure_tenant(context, assignment.tenant)?;
        if assignment.lifecycle != question_model::AssignmentLifecycle::Draft {
            return Err(StoreError::InvalidRecord(
                "new assignments must begin in the draft lifecycle".to_string(),
            ));
        }
        validate_assignment(&assignment)?;
        let mut transaction = self.begin_tenant(context).await?;
        let creation_witness = assignment_definition_capability::prepare_creation(
            &mut transaction,
            context,
            actor,
            &assignment,
        )
        .await?;
        validate_postgres_assignment_references(&mut transaction, context, &assignment).await?;
        domain::effective_assignment_policy::validate_base_assignment_policy_for_course_term(
            base_policy,
            creation_witness.course_term(),
        )
        .map_err(|error| {
            StoreError::InvalidRecord(format!("invalid assignment base policy: {error:?}"))
        })?;
        let returned = assignment_definition_capability::create(
            &mut transaction,
            context,
            actor,
            &assignment,
            base_policy,
        )
        .await?;
        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(returned)
    }
    async fn replace_assignment_impl(
        &self,
        context: TenantContext,
        command: ReplaceAssignmentCommand,
    ) -> Result<StoredAssignment, StoreError> {
        let ReplaceAssignmentCommand {
            actor,
            course,
            assignment,
            expected_revision,
            update,
        } = command;
        let mut transaction = self.begin_tenant(context).await?;
        let witness = prepare_assignment_rehearsal_verification(
            &mut transaction,
            context,
            actor,
            course,
            assignment,
            expected_revision,
        )
        .await?;
        let previous = load_assignment(&mut transaction, context.tenant_id(), assignment).await?;
        if previous.course_id != course {
            return Err(StoreError::NotFound);
        }
        let assignment = AssignmentRecord {
            id: assignment,
            tenant: context.tenant_id(),
            course_id: course,
            title: update.title.clone(),
            lifecycle: previous.lifecycle,
            instructions: previous.instructions.clone(),
            audience: update.audience.clone(),
            items: update.items.clone(),
            selection_groups: update.selection_groups.clone(),
            policies: update.policies,
            disclosure_policy: update.disclosure_policy,
        };
        validate_assignment(&assignment)?;
        validate_postgres_assignment_references(&mut transaction, context, &assignment).await?;
        crate::ensure_assignment_update_preserves_references(&previous, &update)?;
        let returned = assignment_definition_capability::replace(
            &mut transaction,
            context,
            actor,
            &previous,
            &assignment,
            expected_revision,
            witness.database_count()?,
        )
        .await?;
        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(returned)
    }
    async fn replace_assignment_fixed_item_impl(
        &self,
        context: TenantContext,
        command: ReplaceAssignmentFixedItemCommand,
    ) -> Result<StoredAssignment, StoreError> {
        let mut transaction = self.begin_tenant(context).await?;
        let witness = prepare_assignment_rehearsal_verification(
            &mut transaction,
            context,
            command.actor,
            command.course,
            command.assignment,
            command.expected_revision,
        )
        .await?;
        let current =
            load_assignment(&mut transaction, context.tenant_id(), command.assignment).await?;
        if current.course_id != command.course {
            return Err(StoreError::NotFound);
        }
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
            "SELECT ple_replace_assignment_fixed_item($1, $2, $3, $4, $5, $6, $7, $8, $9)",
        )
        .bind(context.tenant_id().as_uuid())
        .bind(command.actor.as_uuid())
        .bind(command.course.as_uuid())
        .bind(command.assignment.as_uuid())
        .bind(i64::try_from(command.expected_revision.value()).map_err(|_| StoreError::Conflict)?)
        .bind(command.current_item.as_uuid())
        .bind(command.replacement.problem.as_uuid())
        .bind(command.replacement.version.as_uuid())
        .bind(witness.database_count()?)
        .fetch_one(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;
        let stored =
            load_fixed_item_assignment(&mut transaction, context.tenant_id(), command.assignment)
                .await?;
        super::course_policy::reresolve_post_mutation_active_attempts(
            &mut transaction,
            context,
            command.actor,
            command.course,
            command.assignment,
            stored.revision,
        )
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
        let witness = prepare_assignment_rehearsal_verification(
            &mut transaction,
            context,
            command.actor,
            command.course,
            command.assignment,
            command.expected_revision,
        )
        .await?;
        let current =
            load_assignment(&mut transaction, context.tenant_id(), command.assignment).await?;
        if current.course_id != command.course {
            return Err(StoreError::NotFound);
        }
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
            "SELECT ple_add_assignment_fixed_item($1, $2, $3, $4, $5, $6, $7, $8, $9, $10::numeric, $11, $12, $13)",
        )
        .bind(context.tenant_id().as_uuid())
        .bind(command.actor.as_uuid())
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
        .bind(witness.database_count()?)
        .fetch_one(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;
        let stored =
            load_fixed_item_assignment(&mut transaction, context.tenant_id(), command.assignment)
                .await?;
        super::course_policy::reresolve_post_mutation_active_attempts(
            &mut transaction,
            context,
            command.actor,
            command.course,
            command.assignment,
            stored.revision,
        )
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
        let witness = prepare_assignment_rehearsal_verification(
            &mut transaction,
            context,
            command.actor,
            command.course,
            command.assignment,
            command.expected_revision,
        )
        .await?;
        let current =
            load_assignment(&mut transaction, context.tenant_id(), command.assignment).await?;
        if current.course_id != command.course {
            return Err(StoreError::NotFound);
        }
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
        sqlx::query_scalar::<_, ()>(
            "SELECT ple_remove_assignment_fixed_item($1, $2, $3, $4, $5, $6, $7)",
        )
        .bind(context.tenant_id().as_uuid())
        .bind(command.actor.as_uuid())
        .bind(command.course.as_uuid())
        .bind(command.assignment.as_uuid())
        .bind(i64::try_from(command.expected_revision.value()).map_err(|_| StoreError::Conflict)?)
        .bind(command.item.as_uuid())
        .bind(witness.database_count()?)
        .fetch_one(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;
        let stored =
            load_fixed_item_assignment(&mut transaction, context.tenant_id(), command.assignment)
                .await?;
        super::course_policy::reresolve_post_mutation_active_attempts(
            &mut transaction,
            context,
            command.actor,
            command.course,
            command.assignment,
            stored.revision,
        )
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
        self.replace_assignment(
            context,
            ReplaceAssignmentCommand {
                actor: command.actor,
                course: command.course,
                assignment: command.assignment,
                expected_revision: command.expected_revision,
                update,
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
            "SELECT assignment_id, course_id, title, lifecycle, instructions, completion_policy, \
                    completion_threshold::text AS completion_threshold, \
                    attempt_selection_policy, continued_practice_policy, \
                    practice_max_additional_runs, variation_policy, audience_kind, \
                    score_disclosure, per_item_correctness_disclosure, feedback_text_disclosure, \
                    solution_disclosure, class_statistics_disclosure, revision, \
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
                base_policy: load_base_policy(&mut transaction, context.tenant_id(), assignment)
                    .await?,
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
            "SELECT assignment_id, course_id, title, lifecycle, instructions, completion_policy, \
                    completion_threshold::text AS completion_threshold, \
                    attempt_selection_policy, continued_practice_policy, \
                    practice_max_additional_runs, variation_policy, audience_kind, \
                    score_disclosure, per_item_correctness_disclosure, feedback_text_disclosure, \
                    solution_disclosure, class_statistics_disclosure \
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
                    lifecycle, instructions, \
                    completion_policy, completion_threshold::text AS completion_threshold, \
                    attempt_selection_policy, continued_practice_policy, \
                    practice_max_additional_runs, variation_policy, audience_kind, \
                    score_disclosure, per_item_correctness_disclosure, feedback_text_disclosure, \
                    solution_disclosure, class_statistics_disclosure \
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
async fn prepare_assignment_rehearsal_verification(
    transaction: &mut Transaction<'_, Postgres>,
    context: TenantContext,
    actor: UserId,
    course: CourseId,
    assignment: AssignmentId,
    expected_revision: AssignmentRevision,
) -> Result<super::rehearsal::LockedRehearsalSourceWitness, StoreError> {
    let expected = i64::try_from(expected_revision.value()).map_err(|_| StoreError::Conflict)?;
    let row = sqlx::query(
        "SELECT assignment_revision, locked_rehearsal_count, locked_rehearsal_run_ids \
         FROM ple_prepare_assignment_rehearsal_verification($1,$2,$3,$4,$5)",
    )
    .bind(context.tenant_id().as_uuid())
    .bind(actor.as_uuid())
    .bind(course.as_uuid())
    .bind(assignment.as_uuid())
    .bind(expected)
    .fetch_one(&mut **transaction)
    .await
    .map_err(map_sqlx_error)?;
    let prepared = super::rehearsal::AssignmentRehearsalPrepareWitness::decode(&row)?;
    if prepared.revision() != expected {
        return Err(StoreError::Conflict);
    }
    prepared
        .verify(
            transaction,
            context.tenant_id(),
            super::rehearsal::RehearsalSourceSelector::Assignment { course, assignment },
        )
        .await
}

#[cfg(feature = "postgres")]
async fn load_fixed_item_assignment(
    transaction: &mut Transaction<'_, Postgres>,
    tenant: TenantId,
    assignment: AssignmentId,
) -> Result<StoredAssignment, StoreError> {
    let row = sqlx::query(
        "SELECT assignment_id, course_id, title, lifecycle, instructions, completion_policy, \
                completion_threshold::text AS completion_threshold, \
                attempt_selection_policy, continued_practice_policy, \
                practice_max_additional_runs, variation_policy, audience_kind, \
                score_disclosure, per_item_correctness_disclosure, feedback_text_disclosure, \
                solution_disclosure, class_statistics_disclosure, revision, \
                scoring_generation, scoring_status \
         FROM assignment WHERE tenant_id = $1 AND assignment_id = $2",
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
        base_policy: load_base_policy(transaction, tenant, assignment).await?,
        scoring_generation: decode_scoring_generation(&row)?,
        scoring_status: decode_scoring_status(&row)?,
    })
}

async fn load_base_policy(
    transaction: &mut Transaction<'_, Postgres>,
    tenant: TenantId,
    assignment: AssignmentId,
) -> Result<question_model::BaseAssignmentPolicy, StoreError> {
    super::course_policy::load_base_policy(transaction, tenant, assignment).await
}

pub(super) async fn load_assignment_scoring_status(
    transaction: &mut Transaction<'_, Postgres>,
    tenant: TenantId,
    assignment: AssignmentId,
) -> Result<ScoringStatus, StoreError> {
    let row = sqlx::query(
        "SELECT scoring_status FROM assignment WHERE tenant_id=$1 AND assignment_id=$2",
    )
    .bind(tenant.as_uuid())
    .bind(assignment.as_uuid())
    .fetch_optional(&mut **transaction)
    .await
    .map_err(map_sqlx_error)?
    .ok_or(StoreError::NotFound)?;
    decode_scoring_status(&row)
}

#[cfg(test)]
mod tests {
    #[test]
    fn assignment_edit_and_post_capability_reload_do_not_request_app_share_locks() {
        // 1812 intentionally revokes `assignment` UPDATE from `ple_app`.
        // PostgreSQL requires that privilege for `FOR SHARE`; these reads are
        // either ordinary edit reads or occur while the broker lock survives.
        let source = include_str!("course_assignments.rs");
        let edit_fetch = source
            .split("async fn get_assignment_for_edit_impl")
            .nth(1)
            .and_then(|section| section.split("async fn get_assignment_impl").next())
            .expect("edit fetch remains a discrete store method");
        let post_capability_reload = source
            .split("async fn load_fixed_item_assignment")
            .nth(1)
            .and_then(|section| section.split("async fn load_base_policy").next())
            .expect("post-capability reload remains a discrete helper");
        assert!(!edit_fetch.contains("FOR SHARE"));
        assert!(!post_capability_reload.contains("FOR SHARE"));
    }
}

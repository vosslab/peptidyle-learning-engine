use async_trait::async_trait;

use super::*;

fn course_reference_from_row(row: &PgRow) -> Result<question_model::CourseReference, StoreError> {
    let value: i32 = row.try_get("public_id").map_err(map_sqlx_error)?;
    question_model::CourseReference::new(value as u64)
        .ok_or_else(|| StoreError::Unavailable("stored course route number is invalid".to_string()))
}

fn assignment_reference_from_row(
    row: &PgRow,
) -> Result<question_model::AssignmentReference, StoreError> {
    let value: i32 = row.try_get("public_id").map_err(map_sqlx_error)?;
    question_model::AssignmentReference::new(value as u64).ok_or_else(|| {
        StoreError::Unavailable("stored assignment route number is invalid".to_string())
    })
}

fn run_reference_from_row(row: &PgRow) -> Result<question_model::RunReference, StoreError> {
    let value: i32 = row.try_get("public_id").map_err(map_sqlx_error)?;
    question_model::RunReference::new(value as u64)
        .ok_or_else(|| StoreError::Unavailable("stored run route number is invalid".to_string()))
}

fn workspace_reference_from_row(
    row: &PgRow,
) -> Result<question_model::WorkspaceReference, StoreError> {
    let value: i32 = row.try_get("public_id").map_err(map_sqlx_error)?;
    question_model::WorkspaceReference::new(value as u64).ok_or_else(|| {
        StoreError::Unavailable("stored workspace route number is invalid".to_string())
    })
}

async fn actor_can_navigate_run(
    transaction: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    tenant: TenantId,
    actor: UserId,
    course: CourseId,
    assignment: AssignmentId,
    student: question_model::StudentId,
) -> Result<bool, StoreError> {
    if matches!(
        super::entitlement::evaluate_current(transaction, tenant, actor, course, assignment).await?,
        domain::entitlement::EntitlementDecision::Granted(ref grant) if grant.student() == student
    ) {
        return Ok(true);
    }
    postgres_is_course_instructor(transaction, tenant, course, actor).await
}

#[async_trait]
impl crate::NavigationReferenceStore for PostgresStore {
    async fn course_reference(
        &self,
        context: TenantContext,
        actor: UserId,
        course: CourseId,
    ) -> Result<Option<question_model::CourseReference>, StoreError> {
        let mut transaction = self.begin_tenant(context).await?;
        let row = sqlx::query(
            "SELECT c.public_id FROM course AS c JOIN course_member AS cm \
             ON cm.tenant_id = c.tenant_id AND cm.course_id = c.course_id \
             WHERE c.tenant_id = $1 AND c.course_id = $2 AND cm.user_id = $3 \
               AND cm.status = 'active'",
        )
        .bind(context.tenant_id().as_uuid())
        .bind(course.as_uuid())
        .bind(actor.as_uuid())
        .fetch_optional(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;
        let result = row.as_ref().map(course_reference_from_row).transpose()?;
        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(result)
    }

    async fn resolve_course_reference(
        &self,
        context: TenantContext,
        actor: UserId,
        reference: question_model::CourseReference,
    ) -> Result<Option<CourseId>, StoreError> {
        let mut transaction = self.begin_tenant(context).await?;
        let course: Option<Uuid> = sqlx::query_scalar(
            "SELECT c.course_id FROM course AS c JOIN course_member AS cm \
             ON cm.tenant_id = c.tenant_id AND cm.course_id = c.course_id \
             WHERE c.tenant_id = $1 AND c.public_id = $2 AND cm.user_id = $3 \
               AND cm.status = 'active'",
        )
        .bind(context.tenant_id().as_uuid())
        .bind(i64::from(reference.number()))
        .bind(actor.as_uuid())
        .fetch_optional(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;
        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(course.map(CourseId::from_uuid))
    }

    async fn assignment_reference(
        &self,
        context: TenantContext,
        actor: UserId,
        assignment: AssignmentId,
    ) -> Result<Option<question_model::AssignmentReference>, StoreError> {
        let mut transaction = self.begin_tenant(context).await?;
        let row = sqlx::query(
            "SELECT a.public_id FROM assignment AS a JOIN course_member AS cm \
             ON cm.tenant_id = a.tenant_id AND cm.course_id = a.course_id \
             WHERE a.tenant_id = $1 AND a.assignment_id = $2 AND cm.user_id = $3 \
               AND cm.status = 'active'",
        )
        .bind(context.tenant_id().as_uuid())
        .bind(assignment.as_uuid())
        .bind(actor.as_uuid())
        .fetch_optional(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;
        let result = row
            .as_ref()
            .map(assignment_reference_from_row)
            .transpose()?;
        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(result)
    }

    async fn resolve_assignment_reference(
        &self,
        context: TenantContext,
        actor: UserId,
        reference: question_model::AssignmentReference,
    ) -> Result<Option<crate::AssignmentRouteIdentity>, StoreError> {
        let mut transaction = self.begin_tenant(context).await?;
        let row = sqlx::query(
            "SELECT a.course_id, a.assignment_id FROM assignment AS a JOIN course_member AS cm \
             ON cm.tenant_id = a.tenant_id AND cm.course_id = a.course_id \
             WHERE a.tenant_id = $1 AND a.public_id = $2 AND cm.user_id = $3 \
               AND cm.status = 'active'",
        )
        .bind(context.tenant_id().as_uuid())
        .bind(i64::from(reference.number()))
        .bind(actor.as_uuid())
        .fetch_optional(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;
        let result = row
            .as_ref()
            .map(|row| {
                Ok::<crate::AssignmentRouteIdentity, StoreError>(crate::AssignmentRouteIdentity {
                    course: CourseId::from_uuid(row.try_get("course_id").map_err(map_sqlx_error)?),
                    assignment: AssignmentId::from_uuid(
                        row.try_get("assignment_id").map_err(map_sqlx_error)?,
                    ),
                })
            })
            .transpose()?;
        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(result)
    }

    async fn run_reference(
        &self,
        context: TenantContext,
        actor: UserId,
        run: RunId,
    ) -> Result<Option<question_model::RunReference>, StoreError> {
        let mut transaction = self.begin_tenant(context).await?;
        let row = sqlx::query(
            "SELECT r.public_id, assignment.course_id, assignment.assignment_id, e.student_id \
             FROM assignment_run AS r JOIN enrollment AS e \
             ON e.tenant_id = r.tenant_id AND e.enrollment_id = r.enrollment_id \
             JOIN assignment AS assignment ON assignment.tenant_id = r.tenant_id AND assignment.assignment_id = e.assignment_id \
             WHERE r.tenant_id = $1 AND r.run_id = $2",
        )
        .bind(context.tenant_id().as_uuid())
        .bind(run.as_uuid())
        .fetch_optional(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;
        let result = match row.as_ref() {
            Some(row)
                if actor_can_navigate_run(
                    &mut transaction,
                    context.tenant_id(),
                    actor,
                    CourseId::from_uuid(row.try_get("course_id").map_err(map_sqlx_error)?),
                    AssignmentId::from_uuid(row.try_get("assignment_id").map_err(map_sqlx_error)?),
                    question_model::StudentId::from_uuid(
                        row.try_get("student_id").map_err(map_sqlx_error)?,
                    ),
                )
                .await? =>
            {
                Some(run_reference_from_row(row)?)
            }
            _ => None,
        };
        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(result)
    }

    async fn resolve_run_reference(
        &self,
        context: TenantContext,
        actor: UserId,
        reference: question_model::RunReference,
    ) -> Result<Option<crate::RunRouteIdentity>, StoreError> {
        let mut transaction = self.begin_tenant(context).await?;
        let row = sqlx::query(
            "SELECT assignment.course_id, assignment.assignment_id, e.enrollment_id, e.student_id, r.run_id \
             FROM assignment_run AS r JOIN enrollment AS e \
             ON e.tenant_id = r.tenant_id AND e.enrollment_id = r.enrollment_id \
             JOIN assignment AS assignment ON assignment.tenant_id = r.tenant_id AND assignment.assignment_id = e.assignment_id \
             WHERE r.tenant_id = $1 AND r.public_id = $2",
        )
        .bind(context.tenant_id().as_uuid())
        .bind(i64::from(reference.number()))
        .fetch_optional(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;
        let result = match row {
            Some(row)
                if actor_can_navigate_run(
                    &mut transaction,
                    context.tenant_id(),
                    actor,
                    CourseId::from_uuid(row.try_get("course_id").map_err(map_sqlx_error)?),
                    AssignmentId::from_uuid(row.try_get("assignment_id").map_err(map_sqlx_error)?),
                    question_model::StudentId::from_uuid(
                        row.try_get("student_id").map_err(map_sqlx_error)?,
                    ),
                )
                .await? =>
            {
                Some({
                    Ok::<crate::RunRouteIdentity, StoreError>(crate::RunRouteIdentity {
                        course: CourseId::from_uuid(
                            row.try_get("course_id").map_err(map_sqlx_error)?,
                        ),
                        assignment: AssignmentId::from_uuid(
                            row.try_get("assignment_id").map_err(map_sqlx_error)?,
                        ),
                        enrollment: question_model::EnrollmentId::from_uuid(
                            row.try_get("enrollment_id").map_err(map_sqlx_error)?,
                        ),
                        run: RunId::from_uuid(row.try_get("run_id").map_err(map_sqlx_error)?),
                    })?
                })
            }
            _ => None,
        };
        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(result)
    }

    async fn workspace_reference(
        &self,
        context: TenantContext,
        actor: UserId,
        workspace: WorkspaceId,
    ) -> Result<Option<question_model::WorkspaceReference>, StoreError> {
        let mut transaction = self.begin_tenant(context).await?;
        let row = sqlx::query(
            "SELECT d.public_id FROM workspace_draft AS d JOIN workspace_draft_access AS a \
             ON a.tenant_id = d.tenant_id AND a.workspace_id = d.workspace_id \
             WHERE d.tenant_id = $1 AND d.workspace_id = $2 AND a.user_id = $3",
        )
        .bind(context.tenant_id().as_uuid())
        .bind(workspace.as_uuid())
        .bind(actor.as_uuid())
        .fetch_optional(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;
        let result = row.as_ref().map(workspace_reference_from_row).transpose()?;
        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(result)
    }

    async fn resolve_workspace_reference(
        &self,
        context: TenantContext,
        actor: UserId,
        reference: question_model::WorkspaceReference,
    ) -> Result<Option<WorkspaceId>, StoreError> {
        let mut transaction = self.begin_tenant(context).await?;
        let workspace: Option<Uuid> = sqlx::query_scalar(
            "SELECT d.workspace_id FROM workspace_draft AS d JOIN workspace_draft_access AS a \
             ON a.tenant_id = d.tenant_id AND a.workspace_id = d.workspace_id \
             WHERE d.tenant_id = $1 AND d.public_id = $2 AND a.user_id = $3",
        )
        .bind(context.tenant_id().as_uuid())
        .bind(i64::from(reference.number()))
        .bind(actor.as_uuid())
        .fetch_optional(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;
        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(workspace.map(WorkspaceId::from_uuid))
    }
}

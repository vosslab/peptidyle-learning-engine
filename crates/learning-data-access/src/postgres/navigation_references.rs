use async_trait::async_trait;

use super::*;

fn course_public_id_from_row(row: &PgRow) -> Result<question_model::CoursePublicId, StoreError> {
    let value: i32 = row.try_get("public_id").map_err(map_sqlx_error)?;
    question_model::CoursePublicId::new(value as u64)
        .ok_or_else(|| StoreError::Unavailable("stored course route number is invalid".to_string()))
}

fn assignment_public_id_from_row(
    row: &PgRow,
) -> Result<question_model::AssignmentPublicId, StoreError> {
    let value: i32 = row.try_get("public_id").map_err(map_sqlx_error)?;
    question_model::AssignmentPublicId::new(value as u64).ok_or_else(|| {
        StoreError::Unavailable("stored assignment route number is invalid".to_string())
    })
}

fn run_public_id_from_row(row: &PgRow) -> Result<question_model::RunPublicId, StoreError> {
    let value: i32 = row.try_get("public_id").map_err(map_sqlx_error)?;
    question_model::RunPublicId::new(value as u64)
        .ok_or_else(|| StoreError::Unavailable("stored run route number is invalid".to_string()))
}

fn workspace_public_id_from_row(
    row: &PgRow,
) -> Result<question_model::WorkspacePublicId, StoreError> {
    let value: i32 = row.try_get("public_id").map_err(map_sqlx_error)?;
    question_model::WorkspacePublicId::new(value as u64).ok_or_else(|| {
        StoreError::Unavailable("stored workspace route number is invalid".to_string())
    })
}

#[async_trait]
impl crate::NavigationReferenceStore for PostgresStore {
    async fn course_public_id(
        &self,
        context: TenantContext,
        actor: UserId,
        course: CourseId,
    ) -> Result<Option<question_model::CoursePublicId>, StoreError> {
        let mut transaction = self.begin_tenant(context).await?;
        let row = sqlx::query(
            "SELECT c.public_id FROM course AS c JOIN course_member AS cm \
             ON cm.tenant_id = c.tenant_id AND cm.course_id = c.course_id \
             WHERE c.tenant_id = $1 AND c.course_id = $2 AND cm.user_id = $3",
        )
        .bind(context.tenant_id().as_uuid())
        .bind(course.as_uuid())
        .bind(actor.as_uuid())
        .fetch_optional(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;
        let result = row.as_ref().map(course_public_id_from_row).transpose()?;
        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(result)
    }

    async fn resolve_course_public_id(
        &self,
        context: TenantContext,
        actor: UserId,
        public_id: question_model::CoursePublicId,
    ) -> Result<Option<CourseId>, StoreError> {
        let mut transaction = self.begin_tenant(context).await?;
        let course: Option<Uuid> = sqlx::query_scalar(
            "SELECT c.course_id FROM course AS c JOIN course_member AS cm \
             ON cm.tenant_id = c.tenant_id AND cm.course_id = c.course_id \
             WHERE c.tenant_id = $1 AND c.public_id = $2 AND cm.user_id = $3",
        )
        .bind(context.tenant_id().as_uuid())
        .bind(i64::from(public_id.value()))
        .bind(actor.as_uuid())
        .fetch_optional(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;
        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(course.map(CourseId::from_uuid))
    }

    async fn assignment_public_id(
        &self,
        context: TenantContext,
        actor: UserId,
        assignment: AssignmentId,
    ) -> Result<Option<question_model::AssignmentPublicId>, StoreError> {
        let mut transaction = self.begin_tenant(context).await?;
        let row = sqlx::query(
            "SELECT a.public_id FROM assignment AS a JOIN course_member AS cm \
             ON cm.tenant_id = a.tenant_id AND cm.course_id = a.course_id \
             WHERE a.tenant_id = $1 AND a.assignment_id = $2 AND cm.user_id = $3",
        )
        .bind(context.tenant_id().as_uuid())
        .bind(assignment.as_uuid())
        .bind(actor.as_uuid())
        .fetch_optional(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;
        let result = row
            .as_ref()
            .map(assignment_public_id_from_row)
            .transpose()?;
        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(result)
    }

    async fn resolve_assignment_public_id(
        &self,
        context: TenantContext,
        actor: UserId,
        public_id: question_model::AssignmentPublicId,
    ) -> Result<Option<crate::AssignmentRouteIdentity>, StoreError> {
        let mut transaction = self.begin_tenant(context).await?;
        let row = sqlx::query(
            "SELECT a.course_id, a.assignment_id FROM assignment AS a JOIN course_member AS cm \
             ON cm.tenant_id = a.tenant_id AND cm.course_id = a.course_id \
             WHERE a.tenant_id = $1 AND a.public_id = $2 AND cm.user_id = $3",
        )
        .bind(context.tenant_id().as_uuid())
        .bind(i64::from(public_id.value()))
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

    async fn run_public_id(
        &self,
        context: TenantContext,
        actor: UserId,
        run: RunId,
    ) -> Result<Option<question_model::RunPublicId>, StoreError> {
        let mut transaction = self.begin_tenant(context).await?;
        let row = sqlx::query(
            "SELECT r.public_id FROM assignment_run AS r JOIN enrollment AS e \
             ON e.tenant_id = r.tenant_id AND e.enrollment_id = r.enrollment_id \
             WHERE r.tenant_id = $1 AND r.run_id = $2 AND e.user_id = $3",
        )
        .bind(context.tenant_id().as_uuid())
        .bind(run.as_uuid())
        .bind(actor.as_uuid())
        .fetch_optional(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;
        let result = row.as_ref().map(run_public_id_from_row).transpose()?;
        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(result)
    }

    async fn resolve_run_public_id(
        &self,
        context: TenantContext,
        actor: UserId,
        public_id: question_model::RunPublicId,
    ) -> Result<Option<RunId>, StoreError> {
        let mut transaction = self.begin_tenant(context).await?;
        let run: Option<Uuid> = sqlx::query_scalar(
            "SELECT r.run_id FROM assignment_run AS r JOIN enrollment AS e \
             ON e.tenant_id = r.tenant_id AND e.enrollment_id = r.enrollment_id \
             WHERE r.tenant_id = $1 AND r.public_id = $2 AND e.user_id = $3",
        )
        .bind(context.tenant_id().as_uuid())
        .bind(i64::from(public_id.value()))
        .bind(actor.as_uuid())
        .fetch_optional(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;
        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(run.map(RunId::from_uuid))
    }

    async fn workspace_public_id(
        &self,
        context: TenantContext,
        actor: UserId,
        workspace: WorkspaceId,
    ) -> Result<Option<question_model::WorkspacePublicId>, StoreError> {
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
        let result = row.as_ref().map(workspace_public_id_from_row).transpose()?;
        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(result)
    }

    async fn resolve_workspace_public_id(
        &self,
        context: TenantContext,
        actor: UserId,
        public_id: question_model::WorkspacePublicId,
    ) -> Result<Option<WorkspaceId>, StoreError> {
        let mut transaction = self.begin_tenant(context).await?;
        let workspace: Option<Uuid> = sqlx::query_scalar(
            "SELECT d.workspace_id FROM workspace_draft AS d JOIN workspace_draft_access AS a \
             ON a.tenant_id = d.tenant_id AND a.workspace_id = d.workspace_id \
             WHERE d.tenant_id = $1 AND d.public_id = $2 AND a.user_id = $3",
        )
        .bind(context.tenant_id().as_uuid())
        .bind(i64::from(public_id.value()))
        .bind(actor.as_uuid())
        .fetch_optional(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;
        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(workspace.map(WorkspaceId::from_uuid))
    }
}

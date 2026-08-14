use async_trait::async_trait;

use super::*;

#[async_trait]
impl crate::AuthoringStore for PostgresStore {
    async fn upsert_draft_impl(
        &self,
        context: TenantContext,
        actor: UserId,
        expected_revision: Option<WorkspaceDraftRevision>,
        draft: DraftRecord,
    ) -> Result<WorkspaceDraft, StoreError> {
        ensure_tenant(context, draft.tenant)?;
        validate_draft(&draft)?;
        let (payload, checksum) = encode_payload(&draft)?;
        let mut transaction = self.begin_tenant(context).await?;
        let current: Option<i64> = sqlx::query_scalar(
            "SELECT revision FROM workspace_draft \
             WHERE tenant_id = $1 AND workspace_id = $2 FOR UPDATE",
        )
        .bind(draft.tenant.as_uuid())
        .bind(draft.question.workspace.as_uuid())
        .fetch_optional(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;
        let revision = match current {
            Some(value) => {
                let current = WorkspaceDraftRevision::from_stored(value)?;
                let role: Option<String> = sqlx::query_scalar(
                    "SELECT role FROM workspace_draft_access \
                     WHERE tenant_id = $1 AND workspace_id = $2 AND user_id = $3",
                )
                .bind(draft.tenant.as_uuid())
                .bind(draft.question.workspace.as_uuid())
                .bind(actor.as_uuid())
                .fetch_optional(&mut *transaction)
                .await
                .map_err(map_sqlx_error)?;
                if !matches!(role.as_deref(), Some("owner" | "collaborator")) {
                    return Err(StoreError::Forbidden);
                }
                if expected_revision != Some(current) {
                    return Err(StoreError::Conflict);
                }
                let next = current.next()?;
                sqlx::query(
                    "UPDATE workspace_draft SET payload = $3, payload_sha256 = $4, \
                     revision = $5, updated_at = transaction_timestamp() \
                     WHERE tenant_id = $1 AND workspace_id = $2",
                )
                .bind(draft.tenant.as_uuid())
                .bind(draft.question.workspace.as_uuid())
                .bind(payload)
                .bind(checksum)
                .bind(i64::try_from(next.value()).map_err(|_| {
                    StoreError::Unavailable("workspace draft revision limit reached".to_string())
                })?)
                .execute(&mut *transaction)
                .await
                .map_err(map_sqlx_error)?;
                next
            }
            None => {
                if expected_revision.is_some() {
                    return Err(StoreError::Conflict);
                }
                sqlx::query(
                    "INSERT INTO workspace_draft \
                     (tenant_id, workspace_id, payload, payload_sha256, revision) \
                     VALUES ($1, $2, $3, $4, $5)",
                )
                .bind(draft.tenant.as_uuid())
                .bind(draft.question.workspace.as_uuid())
                .bind(payload)
                .bind(checksum)
                .bind(
                    i64::try_from(WorkspaceDraftRevision::INITIAL.value()).map_err(|_| {
                        StoreError::Unavailable(
                            "workspace draft revision limit reached".to_string(),
                        )
                    })?,
                )
                .execute(&mut *transaction)
                .await
                .map_err(map_sqlx_error)?;
                sqlx::query(
                    "INSERT INTO workspace_draft_access \
                     (tenant_id, workspace_id, user_id, role) VALUES ($1, $2, $3, 'owner')",
                )
                .bind(draft.tenant.as_uuid())
                .bind(draft.question.workspace.as_uuid())
                .bind(actor.as_uuid())
                .execute(&mut *transaction)
                .await
                .map_err(map_sqlx_error)?;
                WorkspaceDraftRevision::INITIAL
            }
        };
        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(WorkspaceDraft {
            record: draft,
            revision,
        })
    }
    async fn get_draft_impl(
        &self,
        context: TenantContext,
        actor: UserId,
        workspace: WorkspaceId,
    ) -> Result<Option<WorkspaceDraft>, StoreError> {
        let mut transaction = self.begin_tenant(context).await?;
        let row = sqlx::query(
            "SELECT d.payload, d.payload_sha256, d.revision FROM workspace_draft AS d \
             JOIN workspace_draft_access AS a \
               ON a.tenant_id = d.tenant_id AND a.workspace_id = d.workspace_id \
             WHERE d.tenant_id = $1 AND d.workspace_id = $2 AND a.user_id = $3",
        )
        .bind(context.tenant_id().as_uuid())
        .bind(workspace.as_uuid())
        .bind(actor.as_uuid())
        .fetch_optional(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;
        let record = row
            .as_ref()
            .map(|row| {
                let record = decode_payload_row(row)?;
                let revision = WorkspaceDraftRevision::from_stored(
                    row.try_get("revision").map_err(map_sqlx_error)?,
                )?;
                Ok::<WorkspaceDraft, StoreError>(WorkspaceDraft { record, revision })
            })
            .transpose()?;
        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(record)
    }
    async fn list_drafts_impl(
        &self,
        context: TenantContext,
        actor: UserId,
        page: PageRequest,
    ) -> Result<Page<WorkspaceDraftSummary>, StoreError> {
        let after = page
            .after
            .as_ref()
            .map(|cursor| decode_workspace_draft_cursor(cursor.as_str(), context.tenant_id()))
            .transpose()?;
        let limit = i64::from(page.size.get()) + 1;
        let mut transaction = self.begin_tenant(context).await?;
        let rows = sqlx::query(
            "SELECT d.workspace_id, d.public_id, d.payload, d.payload_sha256 FROM workspace_draft AS d \
             JOIN workspace_draft_access AS a \
               ON a.tenant_id = d.tenant_id AND a.workspace_id = d.workspace_id \
             WHERE d.tenant_id = $1 AND a.user_id = $2 \
               AND ($3::uuid IS NULL OR d.workspace_id > $3) \
             ORDER BY d.workspace_id LIMIT $4",
        )
        .bind(context.tenant_id().as_uuid())
        .bind(actor.as_uuid())
        .bind(after.map(|workspace| workspace.as_uuid()))
        .bind(limit)
        .fetch_all(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;
        let mut drafts = rows
            .iter()
            .map(|row| {
                let workspace: Uuid = row.try_get("workspace_id").map_err(map_sqlx_error)?;
                let public_number: i32 = row.try_get("public_id").map_err(map_sqlx_error)?;
                let public_id = question_model::WorkspacePublicId::new(public_number as u64)
                    .ok_or_else(|| {
                        StoreError::Unavailable(
                            "stored workspace route number is invalid".to_string(),
                        )
                    })?;
                let draft: DraftRecord = decode_payload_row(row)?;
                if draft.tenant != context.tenant_id()
                    || draft.question.workspace.as_uuid() != workspace
                {
                    return Err(StoreError::Unavailable(
                        "stored workspace draft identity does not match its row".to_string(),
                    ));
                }
                Ok((
                    WorkspaceId::from_uuid(workspace),
                    draft.question.workspace_summary(public_id),
                ))
            })
            .collect::<Result<Vec<_>, StoreError>>()?;
        let has_more = drafts.len() > usize::from(page.size.get());
        if has_more {
            drafts.pop();
        }
        let next_cursor = if has_more {
            drafts.last().map(|(workspace, _)| {
                Cursor::from_stable_key(encode_workspace_draft_cursor(
                    context.tenant_id(),
                    *workspace,
                ))
            })
        } else {
            None
        };
        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(Page {
            items: drafts.into_iter().map(|(_, summary)| summary).collect(),
            next_cursor,
        })
    }
    async fn delete_draft_impl(
        &self,
        context: TenantContext,
        actor: UserId,
        workspace: WorkspaceId,
        expected_revision: WorkspaceDraftRevision,
    ) -> Result<bool, StoreError> {
        let mut transaction = self.begin_tenant(context).await?;
        let expected_revision_value = i64::try_from(expected_revision.value()).map_err(|_| {
            StoreError::Unavailable("workspace draft revision limit reached".to_string())
        })?;
        let authorized: bool =
            sqlx::query_scalar("SELECT ple_delete_draft_qti_jobs($1, $2, $3, $4)")
                .bind(context.tenant_id().as_uuid())
                .bind(workspace.as_uuid())
                .bind(actor.as_uuid())
                .bind(expected_revision_value)
                .fetch_one(&mut *transaction)
                .await
                .map_err(map_sqlx_error)?;
        let deleted = if authorized {
            sqlx::query_scalar::<_, Uuid>(
                "DELETE FROM workspace_draft AS d USING workspace_draft_access AS a \
                 WHERE d.tenant_id = $1 AND d.workspace_id = $2 AND d.revision = $4 \
                   AND a.tenant_id = d.tenant_id AND a.workspace_id = d.workspace_id \
                   AND a.user_id = $3 AND a.role = 'owner' \
                 RETURNING d.workspace_id",
            )
            .bind(context.tenant_id().as_uuid())
            .bind(workspace.as_uuid())
            .bind(actor.as_uuid())
            .bind(expected_revision_value)
            .fetch_optional(&mut *transaction)
            .await
            .map_err(map_sqlx_error)?
        } else {
            None
        };
        if deleted.is_some() {
            transaction.commit().await.map_err(map_sqlx_error)?;
            return Ok(true);
        }

        // The capability and delete predicates above are the authoritative
        // atomic decision. This follow-up only classifies its safe
        // non-mutating failure while preserving absent/foreign non-enumeration.
        let row = sqlx::query(
            "SELECT d.revision, a.role FROM workspace_draft AS d \
             LEFT JOIN workspace_draft_access AS a \
               ON a.tenant_id = d.tenant_id AND a.workspace_id = d.workspace_id AND a.user_id = $3 \
             WHERE d.tenant_id = $1 AND d.workspace_id = $2 \
             FOR UPDATE OF d",
        )
        .bind(context.tenant_id().as_uuid())
        .bind(workspace.as_uuid())
        .bind(actor.as_uuid())
        .fetch_optional(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;
        let Some(row) = row else {
            transaction.commit().await.map_err(map_sqlx_error)?;
            return Ok(false);
        };
        let role: Option<String> = row.try_get("role").map_err(map_sqlx_error)?;
        if role.as_deref() != Some("owner") {
            return Err(StoreError::Forbidden);
        }
        let current =
            WorkspaceDraftRevision::from_stored(row.try_get("revision").map_err(map_sqlx_error)?)?;
        if current != expected_revision {
            return Err(StoreError::Conflict);
        }
        Err(StoreError::Conflict)
    }
    async fn grant_draft_collaborator_impl(
        &self,
        context: TenantContext,
        actor: UserId,
        workspace: WorkspaceId,
        collaborator: UserId,
    ) -> Result<(), StoreError> {
        let mut transaction = self.begin_tenant(context).await?;
        let role: Option<String> = sqlx::query_scalar(
            "SELECT role FROM workspace_draft_access \
             WHERE tenant_id = $1 AND workspace_id = $2 AND user_id = $3",
        )
        .bind(context.tenant_id().as_uuid())
        .bind(workspace.as_uuid())
        .bind(actor.as_uuid())
        .fetch_optional(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;
        if role.as_deref() != Some("owner") {
            let exists: bool = sqlx::query_scalar(
                "SELECT EXISTS(SELECT 1 FROM workspace_draft \
                 WHERE tenant_id = $1 AND workspace_id = $2)",
            )
            .bind(context.tenant_id().as_uuid())
            .bind(workspace.as_uuid())
            .fetch_one(&mut *transaction)
            .await
            .map_err(map_sqlx_error)?;
            return Err(if exists {
                StoreError::Forbidden
            } else {
                StoreError::NotFound
            });
        }
        if collaborator != actor {
            sqlx::query(
                "INSERT INTO workspace_draft_access \
                 (tenant_id, workspace_id, user_id, role) VALUES ($1, $2, $3, 'collaborator') \
                 ON CONFLICT (tenant_id, workspace_id, user_id) DO NOTHING",
            )
            .bind(context.tenant_id().as_uuid())
            .bind(workspace.as_uuid())
            .bind(collaborator.as_uuid())
            .execute(&mut *transaction)
            .await
            .map_err(map_sqlx_error)?;
        }
        transaction.commit().await.map_err(map_sqlx_error)
    }
    async fn get_published_problem_impl(
        &self,
        problem: question_model::ProblemId,
        version: VersionId,
    ) -> Result<Option<PublishedProblemRecord>, StoreError> {
        let mut transaction = self.begin_app().await?;
        let row = sqlx::query(
            "SELECT pv.problem_id, p.public_id, p.question_id, pv.version_id, pv.version_number, \
                    pvp.payload, pvp.payload_sha256, pv.lifecycle, pv.lifecycle_reason \
             FROM problem_version AS pv \
             JOIN problem AS p USING (problem_id) \
             JOIN problem_version_payload AS pvp USING (problem_id, version_id) \
             WHERE pv.problem_id = $1 AND pv.version_id = $2 \
               AND pv.publication_scope = 'public'",
        )
        .bind(problem.as_uuid())
        .bind(version.as_uuid())
        .fetch_optional(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;
        let record = row.as_ref().map(decode_catalog_payload_row).transpose()?;
        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(record)
    }
    async fn list_published_problems_impl(
        &self,
        page: PageRequest,
    ) -> Result<Page<PublishedProblemRecord>, StoreError> {
        let cursor = page.after.as_ref().map(|value| value.as_str().to_string());
        let limit = i64::from(page.size.get()) + 1;
        let mut transaction = self.begin_app().await?;
        let rows = sqlx::query(
            "SELECT pv.problem_id::text || '/' || pv.version_id::text AS stable_key, \
                    payload, payload_sha256 \
             FROM problem_version AS pv \
             JOIN problem_version_payload AS pvp \
               USING (problem_id, version_id) \
             WHERE pv.publication_scope = 'public' \
               AND pv.lifecycle = 'published' \
               AND ($1::text IS NULL \
                    OR pv.problem_id::text || '/' || pv.version_id::text > $1) \
             ORDER BY pv.problem_id::text, pv.version_id::text \
             LIMIT $2",
        )
        .bind(cursor)
        .bind(limit)
        .fetch_all(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;
        let result = page_from_rows(rows, page.size.get())?;
        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(result)
    }
}

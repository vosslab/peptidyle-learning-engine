use async_trait::async_trait;

use super::*;

#[async_trait]
impl crate::AuthoringStore for PostgresStore {
    async fn upsert_draft_impl(
        &self,
        actor: ActorContext,
        expected_revision: Option<WorkspaceDraftRevision>,
        draft: DraftRecord,
    ) -> Result<WorkspaceDraft, StoreError> {
        validate_draft(&draft)?;
        let title = draft.question.metadata.title.clone();
        let definition = serde_json::to_value(&draft)
            .map_err(|error| StoreError::InvalidRecord(error.to_string()))?;
        let workspace = draft.question.workspace;
        let user = actor.user_id();
        let mut transaction = self.begin_actor(actor).await?;
        let current: Option<i64> = sqlx::query_scalar(
            "SELECT revision::bigint FROM ple_private.workspace_draft_question \
             WHERE workspace_id = $1 FOR UPDATE",
        )
        .bind(workspace.as_uuid())
        .fetch_optional(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;
        let revision = match current {
            Some(value) => {
                let current = WorkspaceDraftRevision::from_stored(value)?;
                if expected_revision != Some(current) {
                    return Err(StoreError::Conflict);
                }
                let next = current.next()?;
                sqlx::query(
                    "UPDATE ple_private.workspace_draft_question \
                     SET title = $2, definition = $3, revision = $4, \
                         updated_at = transaction_timestamp() \
                     WHERE workspace_id = $1",
                )
                .bind(workspace.as_uuid())
                .bind(title)
                .bind(definition)
                .bind(i32::try_from(next.value()).map_err(|_| {
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
                let (can_access, owns_workspace): (bool, bool) = sqlx::query_as(
                    "SELECT ple_api.current_actor_can_access_workspace($1), \
                            ple_api.current_actor_owns_workspace($1)",
                )
                .bind(workspace.as_uuid())
                .fetch_one(&mut *transaction)
                .await
                .map_err(map_sqlx_error)?;
                if can_access && !owns_workspace {
                    return Err(StoreError::Forbidden);
                }
                if !can_access {
                    sqlx::query(
                        "INSERT INTO ple_private.authoring_workspace \
                         (workspace_id, owner_user_id, created_at) \
                         VALUES ($1, $2, transaction_timestamp())",
                    )
                    .bind(workspace.as_uuid())
                    .bind(user.as_uuid())
                    .execute(&mut *transaction)
                    .await
                    .map_err(map_sqlx_error)?;
                }
                sqlx::query(
                    "INSERT INTO ple_private.workspace_draft_question \
                     (draft_id, workspace_id, revision, title, definition, created_at, updated_at) \
                     VALUES ($1, $2, 1, $3, $4, transaction_timestamp(), transaction_timestamp())",
                )
                .bind(workspace.as_uuid())
                .bind(workspace.as_uuid())
                .bind(title)
                .bind(definition)
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
        actor: ActorContext,
        workspace: WorkspaceId,
    ) -> Result<Option<WorkspaceDraft>, StoreError> {
        let mut transaction = self.begin_actor(actor).await?;
        let row = sqlx::query(
            "SELECT definition, revision::bigint AS revision \
             FROM ple_private.workspace_draft_question \
             WHERE workspace_id = $1",
        )
        .bind(workspace.as_uuid())
        .fetch_optional(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;
        let record = row
            .as_ref()
            .map(|row| {
                let definition: serde_json::Value = row.try_get("definition").map_err(map_sqlx_error)?;
                let record = serde_json::from_value(definition)
                    .map_err(|error| StoreError::Unavailable(error.to_string()))?;
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
        actor: ActorContext,
        page: PageRequest,
    ) -> Result<Page<WorkspaceDraftSummary>, StoreError> {
        let user = actor.user_id();
        let after = page
            .after
            .as_ref()
            .map(|cursor| decode_workspace_draft_cursor(cursor.as_str(), user))
            .transpose()?;
        let limit = i64::from(page.size.get()) + 1;
        let mut transaction = self.begin_actor(actor).await?;
        let rows = sqlx::query(
            "SELECT d.workspace_id, workspace.reference_number, d.definition \
             FROM ple_private.workspace_draft_question AS d \
             JOIN ple_private.authoring_workspace AS workspace USING (workspace_id) \
             WHERE ($1::uuid IS NULL OR d.workspace_id > $1) \
             ORDER BY d.workspace_id LIMIT $2",
        )
        .bind(after.map(|workspace| workspace.as_uuid()))
        .bind(limit)
        .fetch_all(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;
        let mut drafts = rows
            .iter()
            .map(|row| {
                let workspace: Uuid = row.try_get("workspace_id").map_err(map_sqlx_error)?;
                let reference_number: i64 = row.try_get("reference_number").map_err(map_sqlx_error)?;
                let reference = question_model::WorkspaceReference::new(reference_number as u64)
                    .ok_or_else(|| {
                        StoreError::Unavailable(
                            "stored workspace route number is invalid".to_string(),
                        )
                    })?;
                let definition: serde_json::Value = row.try_get("definition").map_err(map_sqlx_error)?;
                let draft: DraftRecord = serde_json::from_value(definition)
                    .map_err(|error| StoreError::Unavailable(error.to_string()))?;
                if draft.question.workspace.as_uuid() != workspace {
                    return Err(StoreError::Unavailable(
                        "stored workspace draft identity does not match its row".to_string(),
                    ));
                }
                Ok((
                    WorkspaceId::from_uuid(workspace),
                    draft.question.workspace_summary(reference),
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
                    user,
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
        actor: ActorContext,
        workspace: WorkspaceId,
        expected_revision: WorkspaceDraftRevision,
    ) -> Result<bool, StoreError> {
        let mut transaction = self.begin_actor(actor).await?;
        let expected_revision_value = i64::try_from(expected_revision.value()).map_err(|_| {
            StoreError::Unavailable("workspace draft revision limit reached".to_string())
        })?;
        let deleted = sqlx::query_scalar::<_, Uuid>(
            "DELETE FROM ple_private.workspace_draft_question AS draft \
             WHERE draft.workspace_id = $1 AND draft.revision = $2 \
               AND ple_api.current_actor_owns_workspace(draft.workspace_id) \
             RETURNING draft.workspace_id",
        )
        .bind(workspace.as_uuid())
        .bind(expected_revision_value)
        .fetch_optional(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;
        if deleted.is_some() {
            sqlx::query(
                "DELETE FROM ple_private.worker_job \
                 WHERE target_kind = 'qti_import' AND workspace_id = $1",
            )
            .bind(workspace.as_uuid())
            .execute(&mut *transaction)
            .await
            .map_err(map_sqlx_error)?;
            sqlx::query(
                "DELETE FROM ple_private.workspace_qti_import \
                 WHERE workspace_id = $1",
            )
            .bind(workspace.as_uuid())
            .execute(&mut *transaction)
            .await
            .map_err(map_sqlx_error)?;
            sqlx::query(
                "DELETE FROM ple_private.workspace_flat_question_source \
                 WHERE workspace_id = $1",
            )
            .bind(workspace.as_uuid())
            .execute(&mut *transaction)
            .await
            .map_err(map_sqlx_error)?;
            sqlx::query(
                "DELETE FROM ple_private.workspace_flat_question_grading \
                 WHERE workspace_id = $1",
            )
            .bind(workspace.as_uuid())
            .execute(&mut *transaction)
            .await
            .map_err(map_sqlx_error)?;
            transaction.commit().await.map_err(map_sqlx_error)?;
            return Ok(true);
        }

        let row = sqlx::query(
            "SELECT draft.revision::bigint AS revision, \
                    ple_api.current_actor_owns_workspace(draft.workspace_id) AS owns_workspace \
             FROM ple_private.workspace_draft_question AS draft \
             WHERE draft.workspace_id = $1 FOR UPDATE",
        )
        .bind(workspace.as_uuid())
        .fetch_optional(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;
        let Some(row) = row else {
            transaction.commit().await.map_err(map_sqlx_error)?;
            return Ok(false);
        };
        let owns_workspace: bool = row.try_get("owns_workspace").map_err(map_sqlx_error)?;
        if !owns_workspace {
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
        actor: ActorContext,
        workspace: WorkspaceId,
        collaborator: UserId,
    ) -> Result<(), StoreError> {
        let actor_user = actor.user_id();
        let mut transaction = self.begin_actor(actor).await?;
        let (can_access, owns_workspace): (bool, bool) = sqlx::query_as(
            "SELECT ple_api.current_actor_can_access_workspace($1), \
                    ple_api.current_actor_owns_workspace($1)",
        )
        .bind(workspace.as_uuid())
        .fetch_one(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;
        if !owns_workspace {
            return Err(if can_access {
                StoreError::Forbidden
            } else {
                StoreError::NotFound
            });
        }
        if collaborator != actor_user {
            sqlx::query(
                "INSERT INTO ple_private.authoring_workspace_collaborator \
                 (workspace_id, user_id, granted_by_user_id, granted_at) \
                 VALUES ($1, $2, $3, transaction_timestamp()) \
                 ON CONFLICT (workspace_id, user_id) DO NOTHING",
            )
            .bind(workspace.as_uuid())
            .bind(collaborator.as_uuid())
            .bind(actor_user.as_uuid())
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
            "SELECT pv.problem_id, p.question_id, pv.version_id, \
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

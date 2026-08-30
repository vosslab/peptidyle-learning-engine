use async_trait::async_trait;

use super::*;

#[async_trait]
impl crate::AuthoringStore for MemoryStore {
    async fn upsert_draft_impl(
        &self,
        actor: ActorContext,
        expected_revision: Option<WorkspaceDraftRevision>,
        draft: DraftRecord,
    ) -> Result<WorkspaceDraft, StoreError> {
        let actor = actor.user_id();
        validate_draft(&draft)?;
        let mut state = self.write_state()?;
        let key = draft.question.workspace;
        if state.drafts.contains_key(&key) {
            let role = state
                .draft_access
                .get(&(draft.question.workspace, actor));
            if !matches!(
                role,
                Some(WorkspaceDraftRole::Owner | WorkspaceDraftRole::Collaborator)
            ) {
                return Err(StoreError::Forbidden);
            }
            let current = state
                .draft_revisions
                .get(&key)
                .copied()
                .ok_or(StoreError::Forbidden)?;
            if expected_revision != Some(current) {
                return Err(StoreError::Conflict);
            }
            let revision = current.next()?;
            state.drafts.insert(key, draft.clone());
            state.draft_revisions.insert(key, revision);
            state.flat_question_sources.remove(&key);
            state.workspace_flat_question_grading.remove(&key);
            return Ok(WorkspaceDraft {
                record: draft,
                revision,
            });
        }
        if expected_revision.is_some() {
            return Err(StoreError::Conflict);
        }
        if state.workspace_references.contains_key(&key)
            && state.draft_access.get(&(key, actor)) != Some(&WorkspaceDraftRole::Owner)
        {
            return Err(StoreError::Forbidden);
        }
        let revision = WorkspaceDraftRevision::INITIAL;
        super::navigation_references::ensure_workspace_reference(
            &mut state,
            draft.question.workspace,
        )?;
        state.drafts.insert(key, draft.clone());
        state.draft_revisions.insert(key, revision);
        state
            .draft_access
            .entry((draft.question.workspace, actor))
            .or_insert(WorkspaceDraftRole::Owner);
        state.flat_question_sources.remove(&key);
        state.workspace_flat_question_grading.remove(&key);
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
        let actor = actor.user_id();
        let state = self.read_state()?;
        let key = workspace;
        if !state.draft_access.contains_key(&(workspace, actor)) {
            return Ok(None);
        }
        let Some(record) = state.drafts.get(&key).cloned() else {
            return Ok(None);
        };
        let revision = state
            .draft_revisions
            .get(&key)
            .copied()
            .ok_or(StoreError::Unavailable(
                "workspace draft is missing its revision".to_string(),
            ))?;
        Ok(Some(WorkspaceDraft { record, revision }))
    }
    async fn list_drafts_impl(
        &self,
        actor: ActorContext,
        page: PageRequest,
    ) -> Result<Page<question_model::WorkspaceDraftSummary>, StoreError> {
        let actor = actor.user_id();
        let after = page
            .after
            .as_ref()
            .map(|cursor| {
                crate::decode_workspace_draft_cursor(cursor.as_str(), actor)
            })
            .transpose()?;
        let state = self.read_state()?;
        let mut drafts: Vec<_> = state
            .drafts
            .iter()
            .filter(|(workspace, _)| {
                state.draft_access.contains_key(&(*workspace, actor))
            })
            .filter_map(|(workspace, draft)| {
                let reference = state
                    .workspace_references
                    .get(workspace)
                    .copied()?;
                Some((*workspace, draft.question.workspace_summary(reference)))
            })
            .collect();
        drafts.sort_by_key(|(workspace, _)| workspace.as_uuid());
        let mut selected: Vec<_> = drafts
            .into_iter()
            .filter(|(workspace, _)| {
                after.is_none_or(|cursor| workspace.as_uuid() > cursor.as_uuid())
            })
            .take(usize::from(page.size.get()) + 1)
            .collect();
        let has_more = selected.len() > usize::from(page.size.get());
        if has_more {
            selected.pop();
        }
        let next_cursor = if has_more {
            selected.last().map(|(workspace, _)| {
                Cursor::from_stable_key(crate::encode_workspace_draft_cursor(
                    actor,
                    *workspace,
                ))
            })
        } else {
            None
        };
        Ok(Page {
            items: selected.into_iter().map(|(_, summary)| summary).collect(),
            next_cursor,
        })
    }
    async fn delete_draft_impl(
        &self,
        actor: ActorContext,
        workspace: WorkspaceId,
        expected_revision: WorkspaceDraftRevision,
    ) -> Result<bool, StoreError> {
        let actor = actor.user_id();
        let mut state = self.write_state()?;
        let key = workspace;
        if !state.drafts.contains_key(&key) {
            return Ok(false);
        }
        if !state.draft_access.contains_key(&(workspace, actor)) {
            return Ok(false);
        }
        if state
            .draft_access
            .get(&(workspace, actor))
            != Some(&WorkspaceDraftRole::Owner)
        {
            return Err(StoreError::Forbidden);
        }
        let current_revision = state.draft_revisions.get(&key).copied().ok_or_else(|| {
            StoreError::Unavailable("workspace draft is missing its revision".to_string())
        })?;
        if current_revision != expected_revision {
            return Err(StoreError::Conflict);
        }
        let prepared_imports = state
            .prepared_qti_imports
            .keys()
            .filter(|(workspace, _)| *workspace == key)
            .copied()
            .collect::<BTreeSet<_>>();
        state
            .prepared_qti_imports
            .retain(|(workspace, _), _| *workspace != key);
        state
            .prepared_qti_grading
            .retain(|(workspace, import, _), _| {
                !prepared_imports.contains(&(*workspace, *import))
            });
        state
            .qti_profile_import_evidence
            .remove_prepared_imports(&prepared_imports);
        state.drafts.remove(&key);
        state.draft_revisions.remove(&key);
        state.flat_question_sources.remove(&key);
        state.workspace_flat_question_grading.remove(&key);
        state.workspace_flat_import_origins.remove(&key);
        state.jobs.retain(|_, job| {
            !matches!(
                    job.payload,
                    JobPayload::QtiImport { workspace, .. } if workspace == key
                )
        });
        Ok(true)
    }
    async fn grant_draft_collaborator_impl(
        &self,
        actor: ActorContext,
        workspace: WorkspaceId,
        collaborator: UserId,
    ) -> Result<(), StoreError> {
        let actor = actor.user_id();
        let mut state = self.write_state()?;
        let key = workspace;
        if !state.workspace_references.contains_key(&key) {
            return Err(StoreError::NotFound);
        }
        if state
            .draft_access
            .get(&(workspace, actor))
            != Some(&WorkspaceDraftRole::Owner)
        {
            return Err(StoreError::Forbidden);
        }
        if collaborator != actor {
            state.draft_access.insert(
                (workspace, collaborator),
                WorkspaceDraftRole::Collaborator,
            );
        }
        Ok(())
    }
    async fn get_published_problem_impl(
        &self,
        problem: ProblemId,
        version: VersionId,
    ) -> Result<Option<PublishedProblemRecord>, StoreError> {
        let state = self.read_state()?;
        Ok(state
            .published
            .get(&(problem, version))
            .filter(|record| record.scope == PublicationScope::Public)
            .cloned())
    }
    async fn list_published_problems_impl(
        &self,
        page: PageRequest,
    ) -> Result<Page<PublishedProblemRecord>, StoreError> {
        let state = self.read_state()?;
        let records = state
            .published
            .iter()
            .filter(|(_, record)| {
                record.scope == PublicationScope::Public && record.lifecycle.is_discoverable()
            })
            .map(|((problem, version), record)| (format!("{problem}/{version}"), record.clone()))
            .collect();
        Ok(page_records(records, &page))
    }
}

//! In-memory flat-question store implementations.

use async_trait::async_trait;
use question_model::DraftQuestionSource::Native;
use question_model::{UserId, WorkspaceId};

use super::{MemoryFlatQuestionGraderStore, MemoryStore, catalog_record_visible};
use crate::{
    FlatQuestionGradingPayload, FlatQuestionGradingStore, FlatQuestionStore, StoreError,
    TenantContext, UpsertFlatQuestionCommand, WorkspaceDraftRevision, WorkspaceDraftRole,
    WorkspaceFlatQuestionSource, ensure_tenant,
};

#[async_trait]
impl FlatQuestionStore for MemoryStore {
    async fn upsert_flat_question(
        &self,
        context: TenantContext,
        actor: UserId,
        command: UpsertFlatQuestionCommand,
    ) -> Result<WorkspaceFlatQuestionSource, StoreError> {
        ensure_tenant(context, command.draft.tenant)?;
        crate::flat_question::validate_upsert_flat_question_command(&command)?;
        let key = (command.draft.tenant, command.draft.question.workspace);
        let mut state = self.write_state()?;
        let source_family = match &command.draft.question.source {
            Native { family } => family.clone(),
            _ => {
                return Err(StoreError::InvalidRecord(
                    "flat-question sources require native draft source".to_string(),
                ));
            }
        };
        let is_new = !state.drafts.contains_key(&key);
        let revision = if !is_new {
            let role = state.draft_access.get(&(
                context.tenant_id(),
                command.draft.question.workspace,
                actor,
            ));
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
            if command.expected_revision != Some(current) {
                return Err(StoreError::Conflict);
            }
            current.next()?
        } else {
            if command.expected_revision.is_some() {
                return Err(StoreError::Conflict);
            }
            WorkspaceDraftRevision::INITIAL
        };
        let source = WorkspaceFlatQuestionSource::new(
            command.draft.tenant,
            command.draft.question.workspace,
            revision,
            source_family,
            command.source,
            command.canonical_source_sha256,
            command.public_binding_sha256,
        )?;
        // Draft, source, and grading maps change only after every authorization,
        // revision, and binding check succeeds while this write lock is held.
        state.drafts.insert(key, command.draft.clone());
        state.draft_revisions.insert(key, revision);
        if is_new {
            state.draft_access.insert(
                (context.tenant_id(), key.1, actor),
                WorkspaceDraftRole::Owner,
            );
        }
        state.flat_question_sources.insert(key, source.clone());
        state
            .workspace_flat_question_grading
            .insert(key, command.grading);
        Ok(source)
    }

    async fn flat_question_source(
        &self,
        context: TenantContext,
        actor: crate::UserId,
        workspace: WorkspaceId,
    ) -> Result<Option<WorkspaceFlatQuestionSource>, StoreError> {
        let state = self.read_state()?;
        let key = (context.tenant_id(), workspace);
        if !state
            .draft_access
            .contains_key(&(context.tenant_id(), workspace, actor))
        {
            return Ok(None);
        }
        Ok(state.flat_question_sources.get(&key).cloned())
    }
}

#[async_trait]
impl FlatQuestionGradingStore for MemoryFlatQuestionGraderStore {
    async fn flat_question_published_grading(
        &self,
        context: TenantContext,
        reference: question_model::ProblemVersionRef,
    ) -> Result<Option<FlatQuestionGradingPayload>, StoreError> {
        let state = self
            .state
            .read()
            .map_err(|error| StoreError::Unavailable(error.to_string()))?;
        let Some(published) = state
            .published
            .get(&(reference.problem, reference.version))
            .filter(|record| catalog_record_visible(&state, context.tenant_id(), record))
        else {
            return Ok(None);
        };
        Ok(state
            .published_flat_question_grading
            .get(&(published.problem, published.version))
            .cloned())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::flat_question::FLAT_QUESTION_MEDIA_TYPE;
    use crate::{CatalogStore, PublishDraftCommand, PublishedSourceArtifact, Store};
    use crate::{DraftRecord, TenantContext, UserId};
    use objects::ObjectCategory;
    use question_model::capability::Capability;
    use question_model::identity::ObjectId;
    use question_model::{
        BackendCapabilities, ProblemId, ProblemVersionRef, PublicationScope, QuestionBackend,
        QuestionSource, ResponseDefinition, TenantId, VersionId, WorkspaceId,
    };
    use uuid::Uuid;

    const FIXTURE: &str = r#"{"format":"pleFlatQuestion","version":2,"title":"Favorite color","prompt":"What is my favorite color?","response":{"kind":"singleChoice","choices":[{"id":"blue","text":"Blue"},{"id":"red","text":"Red"}],"correctChoice":"blue"},"points":1.0,"attemptPolicy":{"maxAttempts":null,"feedback":"immediateFull"},"timingPolicy":{"kind":"untimed"},"license":{"kind":"cc0"},"language":"en-US"}"#;

    fn tenant() -> TenantId {
        TenantId::from_uuid(Uuid::nil())
    }

    fn workspace() -> WorkspaceId {
        WorkspaceId::from_uuid(Uuid::nil())
    }

    fn actor() -> UserId {
        UserId::from_uuid(Uuid::from_u128(42))
    }

    fn draft() -> DraftRecord {
        let question =
            adapter_native::flat_question::FlatQuestionDocument::parse(FIXTURE.as_bytes())
                .expect("fixture should parse")
                .compile(workspace())
                .expect("fixture should compile")
                .into_parts()
                .0;
        DraftRecord {
            tenant: tenant(),
            question,
            derived_from: None,
        }
    }

    fn source(seed: u128) -> objects::ObjectRecord {
        let object = ObjectId::from_uuid(Uuid::from_u128(seed));
        objects::ObjectRecord {
            id: object,
            bucket: objects::Bucket::PrivateContent,
            key: objects::ObjectKey::WorkspaceQuestionSource {
                tenant: tenant(),
                workspace: workspace(),
                object,
            },
            sha256: objects::Sha256Digest::from_bytes([seed as u8; 32]),
            size_bytes: 1,
            media_type: FLAT_QUESTION_MEDIA_TYPE.to_string(),
            category: ObjectCategory::Source,
            version: None,
            license: "test".to_string(),
            provenance: "test".to_string(),
            created_at: question_model::ActivityTimestamp::from_unix_millis(0),
        }
    }

    fn grading_for_draft(draft: &DraftRecord) -> FlatQuestionGradingPayload {
        let source = FIXTURE.replace("Favorite color", &draft.question.metadata.title);
        let private = adapter_native::flat_question::FlatQuestionDocument::parse(source.as_bytes())
            .expect("fixture should parse")
            .compile(workspace())
            .expect("fixture should compile")
            .into_parts()
            .1;
        FlatQuestionGradingPayload::from_private(&private)
            .expect("compiled private fixture should persist")
    }

    fn command(
        expected_revision: Option<WorkspaceDraftRevision>,
        draft: DraftRecord,
        seed: u128,
    ) -> UpsertFlatQuestionCommand {
        let source = source(seed);
        let grading = grading_for_draft(&draft);
        UpsertFlatQuestionCommand {
            expected_revision,
            draft,
            canonical_source_sha256: source.sha256.to_string(),
            source,
            public_binding_sha256: grading.public_binding_sha256().to_string(),
            grading,
        }
    }

    fn revised_draft(title: &str) -> DraftRecord {
        let mut revised = draft();
        revised.question.metadata.title = title.to_string();
        revised
    }

    fn publication_command(
        draft: DraftRecord,
        staged: WorkspaceFlatQuestionSource,
    ) -> PublishDraftCommand {
        let publication = ProblemVersionRef {
            problem: ProblemId::from_uuid(Uuid::from_u128(50)),
            version: VersionId::from_uuid(Uuid::from_u128(51)),
        };
        let object = ObjectId::from_uuid(Uuid::from_u128(52));
        let key = objects::ObjectKey::ProblemSource {
            problem: publication.problem,
            version: publication.version,
            object,
        };
        let artifact = PublishedSourceArtifact {
            reference: publication,
            backend: QuestionBackend::Native,
            object: objects::ObjectRecord {
                id: object,
                bucket: key.bucket(),
                key,
                sha256: staged.source_record.sha256,
                size_bytes: staged.source_record.size_bytes,
                media_type: staged.source_record.media_type.clone(),
                category: ObjectCategory::Source,
                version: Some(publication.version),
                license: "test".to_string(),
                provenance: "published test".to_string(),
                created_at: question_model::ActivityTimestamp::from_unix_millis(1),
            },
        };
        let published_question = draft.question.clone();
        PublishDraftCommand {
            expected_draft: draft,
            expected_revision: staged.workspace_revision,
            publication,
            published_source: QuestionSource::Native {
                family: "flat_single_choice_v2".to_string(),
            },
            source_artifact: Some(artifact),
            qti_promotion: None,
            flat_question_promotion: Some(crate::FlatQuestionPublicationPromotion {
                source: staged,
                import_origin: None,
                published_question,
                assets: Vec::new(),
            }),
            publisher: actor(),
            scope: PublicationScope::Public,
            byline: question_model::PublicByline::new(vec![
                question_model::PublicAuthorName::new("Flat question test author".to_string())
                    .expect("valid test byline"),
            ])
            .expect("valid test byline"),
            capabilities: BackendCapabilities::from_iter([Capability::ServerGrading]),
        }
    }

    #[tokio::test]
    async fn flat_question_first_save_atomically_creates_draft_and_source() {
        let store = MemoryStore::default();
        let context = TenantContext::from_authenticated_session(tenant());

        let saved = store
            .upsert_flat_question(context, actor(), command(None, draft(), 1))
            .await
            .expect("first atomic save should succeed");

        assert_eq!(saved.workspace_revision.value(), 1);
        let stored_draft = store
            .get_draft(context, actor(), workspace())
            .await
            .expect("draft lookup should succeed")
            .expect("atomic save should create a draft");
        assert_eq!(stored_draft.revision, saved.workspace_revision);
        assert_eq!(stored_draft.record, draft());
        assert_eq!(
            store
                .flat_question_source(context, actor(), workspace())
                .await
                .expect("source lookup should succeed"),
            Some(saved)
        );
    }

    #[tokio::test]
    async fn flat_question_collaborator_save_advances_draft_and_source_together() {
        let store = MemoryStore::default();
        let context = TenantContext::from_authenticated_session(tenant());
        let first = store
            .upsert_flat_question(context, actor(), command(None, draft(), 1))
            .await
            .expect("first atomic save should succeed");
        let collaborator = UserId::from_uuid(Uuid::from_u128(43));
        store
            .grant_draft_collaborator(context, actor(), workspace(), collaborator)
            .await
            .expect("owner should grant collaborator access");

        let revised = revised_draft("revised title");
        let saved = store
            .upsert_flat_question(
                context,
                collaborator,
                command(Some(first.workspace_revision), revised.clone(), 2),
            )
            .await
            .expect("collaborator atomic update should succeed");

        assert_eq!(saved.workspace_revision.value(), 2);
        assert_eq!(
            store
                .get_draft(context, collaborator, workspace())
                .await
                .expect("draft lookup should succeed"),
            Some(crate::WorkspaceDraft {
                record: revised,
                revision: saved.workspace_revision,
            })
        );
        assert_eq!(
            store
                .flat_question_source(context, collaborator, workspace())
                .await
                .expect("source lookup should succeed"),
            Some(saved)
        );
    }

    #[tokio::test]
    async fn flat_question_edit_atomically_replaces_current_grading() {
        let store = MemoryStore::default();
        let context = TenantContext::from_authenticated_session(tenant());
        let first_command = command(None, draft(), 1);
        let first_grading = first_command.grading.clone();
        let first = store
            .upsert_flat_question(context, actor(), first_command)
            .await
            .expect("first atomic save should succeed");
        assert_eq!(
            store
                .read_state()
                .expect("memory state should be readable")
                .workspace_flat_question_grading
                .get(&(tenant(), workspace())),
            Some(&first_grading),
            "successful save stores its private grading payload"
        );
        let revised = revised_draft("revised title");
        let revised_command = command(Some(first.workspace_revision), revised, 2);
        let revised_grading = revised_command.grading.clone();

        store
            .upsert_flat_question(context, actor(), revised_command)
            .await
            .expect("second atomic save should replace grading");

        let state = store.read_state().expect("memory state should be readable");
        let stored = state
            .workspace_flat_question_grading
            .get(&(tenant(), workspace()))
            .expect("successful save stores current grading");
        assert_eq!(stored, &revised_grading);
        assert_ne!(stored, &first_grading);
    }

    #[tokio::test]
    async fn flat_question_publish_requires_bound_stored_grading_and_copies_exact_value() {
        let store = MemoryStore::default();
        let context = TenantContext::from_authenticated_session(tenant());
        let save = command(None, draft(), 1);
        let exact_grading = save.grading.clone();
        let staged = store
            .upsert_flat_question(context, actor(), save)
            .await
            .expect("flat question should stage");
        let publish = publication_command(draft(), staged.clone());
        let key = (tenant(), workspace());

        store
            .write_state()
            .expect("memory state should be writable")
            .workspace_flat_question_grading
            .remove(&key);
        assert_eq!(
            store.publish_draft(context, actor(), publish.clone()).await,
            Err(StoreError::Conflict),
            "publication must refuse absent current grading"
        );
        assert!(
            store
                .get_draft(context, actor(), workspace())
                .await
                .expect("draft lookup should succeed")
                .is_some(),
            "an absent-grading refusal must not consume staging"
        );

        let divergent = grading_for_draft(&revised_draft("divergent title"));
        store
            .write_state()
            .expect("memory state should be writable")
            .workspace_flat_question_grading
            .insert(key, divergent);
        assert!(
            store
                .publish_draft(context, actor(), publish.clone())
                .await
                .is_err(),
            "publication must refuse grading not bound to the locked draft and source"
        );
        assert!(
            store
                .get_catalog_problem(context, publish.publication)
                .await
                .expect("catalog lookup should succeed")
                .is_none(),
            "a divergent-grading refusal must happen before publication mutation"
        );

        store
            .write_state()
            .expect("memory state should be writable")
            .workspace_flat_question_grading
            .insert(key, exact_grading.clone());
        let published = store
            .publish_draft(context, actor(), publish)
            .await
            .expect("exact current grading should publish");
        assert_eq!(
            store
                .read_state()
                .expect("memory state should be readable")
                .published_flat_question_grading
                .get(&(published.problem, published.version)),
            Some(&exact_grading),
            "publication copies exactly the stored private grading value"
        );
    }

    #[tokio::test]
    async fn flat_question_stale_save_leaves_draft_and_source_unchanged() {
        let store = MemoryStore::default();
        let context = TenantContext::from_authenticated_session(tenant());
        let first = store
            .upsert_flat_question(context, actor(), command(None, draft(), 1))
            .await
            .expect("first atomic save should succeed");
        let before_draft = store
            .get_draft(context, actor(), workspace())
            .await
            .expect("draft lookup should succeed");

        let result = store
            .upsert_flat_question(
                context,
                actor(),
                command(None, revised_draft("stale edit"), 2),
            )
            .await;

        assert!(matches!(result, Err(StoreError::Conflict)));
        assert_eq!(
            store
                .get_draft(context, actor(), workspace())
                .await
                .expect("draft lookup should succeed"),
            before_draft
        );
        assert_eq!(
            store
                .flat_question_source(context, actor(), workspace())
                .await
                .expect("source lookup should succeed"),
            Some(first)
        );
    }

    #[tokio::test]
    async fn flat_question_forbidden_save_leaves_draft_and_source_unchanged() {
        let store = MemoryStore::default();
        let context = TenantContext::from_authenticated_session(tenant());
        let first = store
            .upsert_flat_question(context, actor(), command(None, draft(), 1))
            .await
            .expect("first atomic save should succeed");
        let before_draft = store
            .get_draft(context, actor(), workspace())
            .await
            .expect("draft lookup should succeed");
        let stranger = UserId::from_uuid(Uuid::from_u128(44));

        let result = store
            .upsert_flat_question(
                context,
                stranger,
                command(
                    Some(first.workspace_revision),
                    revised_draft("forbidden edit"),
                    2,
                ),
            )
            .await;

        assert!(matches!(result, Err(StoreError::Forbidden)));
        assert_eq!(
            store
                .get_draft(context, actor(), workspace())
                .await
                .expect("draft lookup should succeed"),
            before_draft
        );
        assert_eq!(
            store
                .flat_question_source(context, actor(), workspace())
                .await
                .expect("source lookup should succeed"),
            Some(first)
        );
    }

    #[tokio::test]
    async fn flat_question_source_is_hidden_for_unauthorized_actor() {
        let store = MemoryStore::default();
        let context = TenantContext::from_authenticated_session(tenant());
        assert!(
            store
                .upsert_flat_question(context, actor(), command(None, draft(), 1))
                .await
                .is_ok()
        );
        assert!(
            store
                .flat_question_source(context, UserId::from_uuid(Uuid::from_u128(2)), workspace())
                .await
                .expect("query should succeed")
                .is_none()
        );
    }

    #[tokio::test]
    async fn flat_question_rejects_external_tool_draft_before_mutation() {
        let store = MemoryStore::default();
        let context = TenantContext::from_authenticated_session(tenant());
        let mut invalid = draft();
        invalid.question.response = ResponseDefinition::ExternalTool {};

        let result = store
            .upsert_flat_question(context, actor(), command(None, invalid, 1))
            .await;

        assert!(matches!(result, Err(StoreError::InvalidRecord(_))));
        assert!(
            store
                .get_draft(context, actor(), workspace())
                .await
                .expect("lookup should succeed")
                .is_none()
        );
        assert!(
            store
                .flat_question_source(context, actor(), workspace())
                .await
                .expect("lookup should succeed")
                .is_none()
        );
    }
}

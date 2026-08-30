//! In-memory atomic QTI-profile conversion and private lineage persistence.

use std::collections::{BTreeMap, BTreeSet};

use async_trait::async_trait;
use question_model::{ProblemId, UserId, VersionId, WorkspaceId, WorkspaceImportId};

use super::{MemoryStore, State};
use crate::{
    ActorContext, FlatImportProvenanceStore, PublishedFlatImportOrigin, QtiImportItemStatus,
    QtiProfileFlatConversionCommand, QtiProfileImportEvidence, StoreError, WorkspaceDraftRevision,
    WorkspaceDraftRole, WorkspaceFlatImportOrigin, WorkspaceFlatQuestionSource,
};

type ProfileEvidenceKey = (WorkspaceId, WorkspaceImportId, String);

#[derive(Clone, Default)]
pub(super) struct QtiProfileImportEvidences(BTreeMap<ProfileEvidenceKey, QtiProfileImportEvidence>);

impl std::fmt::Debug for QtiProfileImportEvidences {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("QtiProfileImportEvidences")
            .field("record_count", &self.0.len())
            .finish()
    }
}

impl QtiProfileImportEvidences {
    fn get(&self, key: &ProfileEvidenceKey) -> Option<&QtiProfileImportEvidence> {
        self.0.get(key)
    }

    fn values_for_import(
        &self,
        import: crate::QtiImportRef,
    ) -> impl Iterator<Item = &QtiProfileImportEvidence> {
        self.0.iter().filter_map(move |(key, evidence)| {
            (key.0 == import.workspace && key.1 == import.import).then_some(evidence)
        })
    }

    fn insert(&mut self, key: ProfileEvidenceKey, evidence: QtiProfileImportEvidence) {
        self.0.insert(key, evidence);
    }

    /// Drops evidence that belongs to private QTI preparations being discarded
    /// with their workspace draft. Committed evidence remains durable.
    pub(super) fn remove_prepared_imports(
        &mut self,
        imports: &BTreeSet<(WorkspaceId, WorkspaceImportId)>,
    ) {
        self.0
            .retain(|(workspace, import, _), _| !imports.contains(&(*workspace, *import)));
    }

    /// A recognized registry is not ready to expose until its private item
    /// evidence is a complete, exact projection of every accepted result.
    /// Generic registries deliberately retain their pre-profile behavior.
    pub(super) fn completes_recognized_registry(
        &self,
        registry: &crate::QtiImportRegistry,
    ) -> bool {
        let Some(summary) = &registry.profile_summary else {
            return true;
        };

        let mut accepted = BTreeMap::new();
        for result in &registry.item_results {
            if result.status != QtiImportItemStatus::Accepted {
                continue;
            }
            let (Some(item_id), Some(normalized_sha256)) =
                (result.item_id.as_deref(), result.normalized_sha256)
            else {
                return false;
            };
            // Profile evidence has one identifier, so the accepted source and
            // immutable import-item identities must be the same closed value.
            if result.source_identifier != item_id
                || accepted.insert(item_id, normalized_sha256).is_some()
            {
                return false;
            }
        }

        let evidence = self
            .values_for_import(registry.reference)
            .collect::<Vec<_>>();
        if evidence.len() != accepted.len() {
            return false;
        }
        evidence.into_iter().all(|record| {
            let parts = record.persistence_parts();
            parts.import == registry.reference
                && parts.profile == summary.profile()
                && parts.digests.profile_report_sha256 == summary.profile_report_sha256()
                && accepted
                    .get(parts.source_item_identifier)
                    .is_some_and(|normalized| *normalized == parts.digests.normalized_item_sha256)
        })
    }
}

#[derive(Clone, Default)]
pub(super) struct WorkspaceFlatImportOrigins(BTreeMap<WorkspaceId, WorkspaceFlatImportOrigin>);

impl std::fmt::Debug for WorkspaceFlatImportOrigins {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("WorkspaceFlatImportOrigins")
            .field("record_count", &self.0.len())
            .finish()
    }
}

impl WorkspaceFlatImportOrigins {
    pub(super) fn get(&self, key: &WorkspaceId) -> Option<&WorkspaceFlatImportOrigin> {
        self.0.get(key)
    }

    pub(super) fn insert(&mut self, key: WorkspaceId, origin: WorkspaceFlatImportOrigin) {
        self.0.insert(key, origin);
    }

    pub(super) fn remove(&mut self, key: &WorkspaceId) -> Option<WorkspaceFlatImportOrigin> {
        self.0.remove(key)
    }
}

#[derive(Clone, Default)]
pub(super) struct PublishedFlatImportOrigins(
    BTreeMap<(ProblemId, VersionId), PublishedFlatImportOrigin>,
);

impl std::fmt::Debug for PublishedFlatImportOrigins {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("PublishedFlatImportOrigins")
            .field("record_count", &self.0.len())
            .finish()
    }
}

impl PublishedFlatImportOrigins {
    pub(super) fn contains_key(&self, key: &(ProblemId, VersionId)) -> bool {
        self.0.contains_key(key)
    }

    pub(super) fn insert(
        &mut self,
        key: (ProblemId, VersionId),
        origin: PublishedFlatImportOrigin,
    ) -> Option<PublishedFlatImportOrigin> {
        self.0.insert(key, origin)
    }
}

#[async_trait]
impl FlatImportProvenanceStore for MemoryStore {
    async fn stage_qti_profile_import_evidence(
        &self,
        _context: ActorContext,
        evidence: QtiProfileImportEvidence,
    ) -> Result<(), StoreError> {
        let parts = evidence.persistence_parts();
        let import_key = (parts.import.workspace, parts.import.import);
        let evidence_key = (
            parts.import.workspace,
            parts.import.import,
            parts.source_item_identifier.to_string(),
        );
        let mut state = self.write_state()?;
        let registry = state
            .prepared_qti_imports
            .get(&import_key)
            .ok_or(StoreError::Conflict)?;
        validate_accepted_profile_item(registry, parts.source_item_identifier, parts.digests)?;
        let Some(summary) = &registry.profile_summary else {
            return Err(StoreError::Conflict);
        };
        if summary.profile() != parts.profile
            || summary.profile_report_sha256() != parts.digests.profile_report_sha256
        {
            return Err(StoreError::Conflict);
        }

        if let Some(existing) = state.qti_profile_import_evidence.get(&evidence_key) {
            return if existing == &evidence {
                Ok(())
            } else {
                Err(StoreError::Conflict)
            };
        }
        let import_profile_is_consistent = state
            .qti_profile_import_evidence
            .values_for_import(parts.import)
            .all(|existing| {
                let existing = existing.persistence_parts();
                existing.profile == parts.profile
                    && existing.digests.profile_report_sha256 == parts.digests.profile_report_sha256
            });
        if !import_profile_is_consistent {
            return Err(StoreError::Conflict);
        }
        state
            .qti_profile_import_evidence
            .insert(evidence_key, evidence);
        Ok(())
    }

    async fn convert_qti_profile_item_to_flat(
        &self,
        context: ActorContext,
        actor: UserId,
        command: QtiProfileFlatConversionCommand,
    ) -> Result<WorkspaceFlatQuestionSource, StoreError> {
        let command = QtiProfileFlatConversionCommand::new(
            command.expected_revision,
            command.draft,
            command.source,
            command.canonical_source_sha256,
            command.public_binding_sha256,
            command.grading,
            command.origin,
        )?;
        if command.origin.acknowledged_by() != actor {
            return Err(StoreError::InvalidRecord(
                "flat-import acknowledgement actor must match the authenticated actor".to_string(),
            ));
        }

        let key = command.draft.question.workspace;
        let mut state = self.write_state()?;
        let is_new = !state.drafts.contains_key(&key);
        let revision = conversion_revision(&state, context, actor, &command, key, is_new)?;
        validate_committed_profile_item(&state, &command)?;

        let source_family = match &command.draft.question.source {
            question_model::DraftQuestionSource::Native { family } => family.clone(),
            _ => {
                return Err(StoreError::InvalidRecord(
                    "flat-import conversion requires a native draft source".to_string(),
                ));
            }
        };
        let source = WorkspaceFlatQuestionSource::new(
            command.draft.question.workspace,
            revision,
            source_family,
            command.source,
            command.canonical_source_sha256,
            command.public_binding_sha256,
        )?;

        // Install the replacement origin directly while the old value remains
        // protected by this write lock; no intermediate unpinned state exists.
        state
            .workspace_flat_import_origins
            .insert(key, command.origin);
        state.drafts.insert(key, command.draft);
        state.draft_revisions.insert(key, revision);
        if is_new {
            state
                .draft_access
                .insert((key, actor), WorkspaceDraftRole::Owner);
        }
        state.flat_question_sources.insert(key, source.clone());
        state
            .workspace_flat_question_grading
            .insert(key, command.grading);
        Ok(source)
    }

    async fn workspace_flat_import_origin(
        &self,
        _context: ActorContext,
        actor: UserId,
        workspace: WorkspaceId,
    ) -> Result<Option<WorkspaceFlatImportOrigin>, StoreError> {
        let state = self.read_state()?;
        let key = workspace;
        if !state.draft_access.contains_key(&(workspace, actor)) {
            return Ok(None);
        }
        Ok(state.workspace_flat_import_origins.get(&key).cloned())
    }
}

fn conversion_revision(
    state: &State,
    _context: ActorContext,
    actor: UserId,
    command: &QtiProfileFlatConversionCommand,
    key: WorkspaceId,
    is_new: bool,
) -> Result<WorkspaceDraftRevision, StoreError> {
    if is_new {
        if command.expected_revision.is_some() {
            return Err(StoreError::Conflict);
        }
        return Ok(WorkspaceDraftRevision::INITIAL);
    }
    if !matches!(
        state
            .draft_access
            .get(&(command.draft.question.workspace, actor)),
        Some(WorkspaceDraftRole::Owner | WorkspaceDraftRole::Collaborator)
    ) {
        return Err(StoreError::Forbidden);
    }
    let current = state.draft_revisions.get(&key).copied().ok_or_else(|| {
        StoreError::Unavailable("workspace draft is missing its revision".to_string())
    })?;
    if command.expected_revision != Some(current) {
        return Err(StoreError::Conflict);
    }
    current.next()
}

fn validate_committed_profile_item(
    state: &State,
    command: &QtiProfileFlatConversionCommand,
) -> Result<(), StoreError> {
    let reference = command.origin.import();
    let registry = state
        .qti_imports
        .get(&(reference.workspace, reference.import))
        .ok_or(StoreError::Conflict)?;
    let source_item_identifier = command.origin.source_item_identifier();
    if registry.reference != reference || registry.source != *command.origin.source_archive() {
        return Err(StoreError::Conflict);
    }
    let summary = registry
        .profile_summary
        .as_ref()
        .ok_or(StoreError::Conflict)?;
    if summary.profile() != command.origin.profile()
        || summary.profile_report_sha256() != command.origin.digests().profile_report_sha256
    {
        return Err(StoreError::Conflict);
    }
    let evidence_key = (
        reference.workspace,
        reference.import,
        source_item_identifier.to_string(),
    );
    validate_accepted_profile_item(registry, source_item_identifier, command.origin.digests())?;
    let evidence = state
        .qti_profile_import_evidence
        .get(&evidence_key)
        .ok_or(StoreError::Conflict)?
        .persistence_parts();
    if evidence.import != reference
        || evidence.source_item_identifier != source_item_identifier
        || evidence.profile != command.origin.profile()
        || evidence.digests != command.origin.digests()
    {
        return Err(StoreError::Conflict);
    }
    Ok(())
}

fn validate_accepted_profile_item(
    registry: &crate::QtiImportRegistry,
    source_item_identifier: &str,
    digests: crate::FlatImportIntegrityDigests,
) -> Result<(), StoreError> {
    let exact_item = registry
        .items
        .iter()
        .any(|item| item.item_id == source_item_identifier);
    let exact_accepted_result = registry.item_results.iter().any(|result| {
        result.source_identifier == source_item_identifier
            && result.item_id.as_deref() == Some(source_item_identifier)
            && result.normalized_sha256 == Some(digests.normalized_item_sha256)
            && result.status == QtiImportItemStatus::Accepted
    });
    if !exact_item || !exact_accepted_result {
        return Err(StoreError::Conflict);
    }
    Ok(())
}

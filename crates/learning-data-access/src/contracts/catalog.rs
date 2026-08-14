use super::*;

/// Encodes a catalog keyset continuation without exposing UUID text in URLs.
/// The fixed binary layout binds the continuation to the normalized-query
/// digest; callers still verify it before using it as a SQL cursor.
pub(crate) fn encode_catalog_search_cursor(
    fingerprint: &str,
    problem: Uuid,
    version: Uuid,
) -> String {
    debug_assert_eq!(fingerprint.len(), 64);
    let mut bytes = Vec::with_capacity(129);
    bytes.push(1);
    bytes.extend_from_slice(fingerprint.as_bytes());
    bytes.extend_from_slice(problem.as_bytes());
    bytes.extend_from_slice(version.as_bytes());
    let integrity = objects::Sha256Digest::compute(&bytes);
    bytes.extend_from_slice(integrity.as_bytes());
    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(bytes)
}

/// Decodes a canonical bounded catalog continuation and rejects a different
/// normalized query before a storage key can be used.
pub(crate) fn decode_catalog_search_cursor(
    cursor: &str,
    fingerprint: &str,
) -> Result<(Uuid, Uuid), StoreError> {
    if cursor.len() > 200 {
        return Err(StoreError::InvalidRecord(
            "catalog cursor is malformed".to_string(),
        ));
    }
    let bytes = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode(cursor)
        .map_err(|_| StoreError::InvalidRecord("catalog cursor is malformed".to_string()))?;
    if bytes.len() != 129
        || bytes[0] != 1
        || bytes[1..65] != *fingerprint.as_bytes()
        || objects::Sha256Digest::compute(&bytes[..97]).as_bytes() != &bytes[97..129]
    {
        return Err(StoreError::InvalidRecord(
            "catalog cursor does not belong to this normalized query".to_string(),
        ));
    }
    let problem = Uuid::from_slice(&bytes[65..81])
        .map_err(|_| StoreError::InvalidRecord("catalog cursor is malformed".to_string()))?;
    let version = Uuid::from_slice(&bytes[81..97])
        .map_err(|_| StoreError::InvalidRecord("catalog cursor is malformed".to_string()))?;
    if encode_catalog_search_cursor(fingerprint, problem, version) != cursor {
        return Err(StoreError::InvalidRecord(
            "catalog cursor is malformed".to_string(),
        ));
    }
    Ok((problem, version))
}

/// Encodes a tenant-bound opaque continuation for workspace-draft listing.
///
/// The stable workspace UUID never appears directly in an API cursor. Binding
/// it to the tenant prevents a continuation issued to one tenant from being
/// replayed against another tenant's private workspace list.
pub(crate) fn encode_workspace_draft_cursor(tenant: TenantId, workspace: WorkspaceId) -> String {
    let mut bytes = Vec::with_capacity(65);
    bytes.push(1);
    bytes.extend_from_slice(tenant.as_uuid().as_bytes());
    bytes.extend_from_slice(workspace.as_uuid().as_bytes());
    let integrity = Sha256Digest::compute(&bytes);
    bytes.extend_from_slice(integrity.as_bytes());
    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(bytes)
}

/// Decodes a workspace-draft continuation only for the tenant that received it.
pub(crate) fn decode_workspace_draft_cursor(
    cursor: &str,
    tenant: TenantId,
) -> Result<WorkspaceId, StoreError> {
    if cursor.len() > 128 {
        return Err(StoreError::InvalidRecord(
            "workspace cursor is malformed".to_string(),
        ));
    }
    let bytes = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode(cursor)
        .map_err(|_| StoreError::InvalidRecord("workspace cursor is malformed".to_string()))?;
    let tenant_id = tenant.as_uuid();
    let tenant_bytes = tenant_id.as_bytes();
    if bytes.len() != 65
        || bytes[0] != 1
        || bytes[1..17] != *tenant_bytes
        || Sha256Digest::compute(&bytes[..33]).as_bytes() != &bytes[33..65]
    {
        return Err(StoreError::InvalidRecord(
            "workspace cursor does not belong to this tenant".to_string(),
        ));
    }
    let workspace = Uuid::from_slice(&bytes[17..33])
        .map_err(|_| StoreError::InvalidRecord("workspace cursor is malformed".to_string()))?;
    let workspace = WorkspaceId::from_uuid(workspace);
    if encode_workspace_draft_cursor(tenant, workspace) != cursor {
        return Err(StoreError::InvalidRecord(
            "workspace cursor is malformed".to_string(),
        ));
    }
    Ok(workspace)
}

/// Shared immutable problem/version reference used by catalog lineage.
pub use question_model::ProblemVersionRef;

/// Server-only immutable source-object binding for one published version.
///
/// This is deliberately separate from browser catalog payloads.  Backends use
/// it to resolve the exact bytes that were prepared before publication; an
/// adapter must never reconstruct an object key from a browser value.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PublishedSourceArtifact {
    /// Published version that owns the immutable source bytes.
    pub reference: ProblemVersionRef,
    /// Backend whose source preparation produced this object.
    pub backend: QuestionBackend,
    /// Verified content-bucket object record.
    pub object: ObjectRecord,
}

/// Tenant-owned editable question draft.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DraftRecord {
    /// Direct RLS boundary.
    pub tenant: TenantId,
    /// Editable content with no published identifiers.
    pub question: DraftQuestionDefinition,
    /// Earlier version in the same owned linear chain, for a new revision.
    pub revises: Option<ProblemVersionRef>,
    /// Source version when creating a new attributed fork.
    pub derived_from: Option<ProblemVersionRef>,
}

/// Persisted authority for one authenticated person in a private workspace.
///
/// An owner is established atomically with the first draft write. A
/// collaborator can inspect and revise the workspace, but cannot transfer
/// access, delete it, or publish it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum WorkspaceDraftRole {
    Owner,
    Collaborator,
}

/// Server-issued optimistic-concurrency value for an editable workspace.
///
/// The value is stored as a positive PostgreSQL `bigint`; callers obtain it
/// only from a successful read or write and must echo it on an update.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct WorkspaceDraftRevision(u64);

impl WorkspaceDraftRevision {
    pub(crate) const INITIAL: Self = Self(1);
    const MAX: u64 = i64::MAX as u64;

    /// Returns the value for browser-safe request/response serialization.
    pub fn value(self) -> u64 {
        self.0
    }

    pub(crate) fn next(self) -> Result<Self, StoreError> {
        let next = self.0.checked_add(1).filter(|value| *value <= Self::MAX);
        next.map(Self).ok_or_else(|| {
            StoreError::Unavailable("workspace draft revision limit reached".to_string())
        })
    }

    #[cfg(feature = "postgres")]
    pub(crate) fn from_stored(value: i64) -> Result<Self, StoreError> {
        let value = u64::try_from(value).map_err(|_| {
            StoreError::Unavailable("stored workspace draft revision is invalid".to_string())
        })?;
        if value == 0 {
            return Err(StoreError::Unavailable(
                "stored workspace draft revision is invalid".to_string(),
            ));
        }
        Ok(Self(value))
    }
}

/// Server-issued optimistic-concurrency value for one editable assignment.
///
/// Assignment definitions are tenant-owned course artifacts.  Their selected
/// published versions stay immutable, while the ordered selection and policies
/// change only through this compare-and-swap token.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct AssignmentRevision(u64);

impl AssignmentRevision {
    pub(crate) const INITIAL: Self = Self(1);
    const MAX: u64 = i64::MAX as u64;

    pub fn value(self) -> u64 {
        self.0
    }

    pub(crate) fn next(self) -> Result<Self, StoreError> {
        self.0
            .checked_add(1)
            .filter(|value| *value <= Self::MAX)
            .map(Self)
            .ok_or_else(|| StoreError::Unavailable("assignment revision limit reached".to_string()))
    }

    #[cfg(feature = "postgres")]
    pub(crate) fn from_stored(value: i64) -> Result<Self, StoreError> {
        let value = u64::try_from(value).map_err(|_| {
            StoreError::Unavailable("stored assignment revision is invalid".to_string())
        })?;
        if value == 0 {
            return Err(StoreError::Unavailable(
                "stored assignment revision is invalid".to_string(),
            ));
        }
        Ok(Self(value))
    }
}

/// Editable draft plus its server-managed revision token.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceDraft {
    pub record: DraftRecord,
    pub revision: WorkspaceDraftRevision,
}

/// Shared immutable published content.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PublishedProblemRecord {
    /// Stable published problem.
    pub problem: ProblemId,
    /// One stable, non-sequential human-facing identity for this question.
    pub question_id: question_model::QuestionId,
    /// Copyable human-facing identity of the stable problem.
    ///
    /// Retained only as a hidden pre-production storage key while the schema
    /// converges on [`Self::question_id`]. It is never projected to a browser.
    pub public_id: question_model::ProblemPublicId,
    /// Exact immutable version.
    pub version: VersionId,
    /// One-based human-facing version within the stable problem.
    pub version_number: question_model::ProblemVersionNumber,
    /// Browser-safe definition whose IDs match this record.
    pub question: QuestionDefinition,
    /// Capabilities declared by the owning adapter at publication time.
    pub capabilities: BackendCapabilities,
    /// Institution-only or cross-tenant catalog visibility.
    pub scope: PublicationScope,
    /// Discoverability and new-assignment state.
    pub lifecycle: CatalogLifecycle,
    /// Ordered, nonempty owners of this problem's linear version chain.
    pub authors: Vec<UserId>,
    /// Earlier version in the same problem chain.
    pub previous_version: Option<VersionId>,
    /// Original source when this problem is a fork.
    pub derived_from: Option<ProblemVersionRef>,
    /// Backend-authoritative time at which this version became immutable.
    pub published_at: ActivityTimestamp,
}

impl PublishedProblemRecord {
    /// Builds the hot browse projection without loading another representation.
    pub fn summary(&self) -> CatalogProblemSummary {
        CatalogProblemSummary {
            problem: self.problem,
            question_id: self.question_id.clone(),
            version: self.version,
            backend: QuestionBackend::from(&self.question.source),
            capabilities: self.capabilities.clone(),
            metadata: self.question.metadata.clone(),
            scope: self.scope,
            lifecycle: self.lifecycle.clone(),
            authors: self.authors.clone(),
            previous_version: self.previous_version,
            derived_from: self.derived_from,
            published_at: self.published_at,
        }
    }
}

/// Atomic publication of the exact draft that passed API validation.
#[derive(Debug, Clone, PartialEq)]
pub struct PublishDraftCommand {
    /// Exact draft value validated before entering storage.
    pub expected_draft: DraftRecord,
    /// Exact saved workspace revision the author reviewed before publication.
    ///
    /// Storage compares this under the same transaction lock that consumes the
    /// draft, so equivalent content saved through another tab cannot make an
    /// old review valid again.
    pub expected_revision: WorkspaceDraftRevision,
    /// Complete durable identity minted after the draft is validated.
    pub publication: ProblemVersionRef,
    /// Server-prepared immutable source. iMathAS reaches this field only after
    /// the source snapshot and supported integration profile are verified.
    pub published_source: question_model::QuestionSource,
    /// Server-prepared immutable original or snapshot for source-backed
    /// backends. Native questions intentionally have no source artifact.
    pub source_artifact: Option<PublishedSourceArtifact>,
    /// Present only for a server-prepared QTI publication.
    pub qti_promotion: Option<QtiPublicationPromotion>,
    /// Present only for a server-prepared flat-question publication.
    pub flat_question_promotion: Option<FlatQuestionPublicationPromotion>,
    /// Authenticated author performing the transition.
    pub publisher: UserId,
    /// Institution-only or public target.
    pub scope: PublicationScope,
    /// Trusted capabilities resolved from the server adapter registry.
    pub capabilities: BackendCapabilities,
}

/// Allowed post-publication lifecycle changes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CatalogTransition {
    /// Hide a version from browsing and new assignments.
    Deprecate {
        /// Required author explanation.
        reason: String,
    },
    /// Move an already deprecated version to historical status.
    Archive,
}

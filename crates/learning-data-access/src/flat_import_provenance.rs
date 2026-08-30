//! Durable lineage for flat questions converted from recognized QTI profiles.
//!
//! Vendor parsing remains in `adapter_qti`, flat construction remains in
//! `adapter_native`, and the server translates both into these storage-owned
//! types.  No adapter type crosses this persistence boundary.
//!
//! Every implementation uses one lock order: workspace draft `FOR UPDATE`,
//! committed QTI import `FOR KEY SHARE`, current flat-import origin, current
//! flat source, then immutable publication rows. Object copies happen before
//! the transaction. Import cleanup may lock an import and check origin pins,
//! but must never acquire the workspace draft afterward; this prevents the
//! cleanup/import versus conversion/draft lock cycle.

use async_trait::async_trait;
use objects::{ObjectRecord, Sha256Digest};
use question_model::{ActivityTimestamp, ProblemVersionRef, UserId, WorkspaceId};

use crate::{
    ActorContext, DraftRecord, FlatQuestionGradingPayload, QtiImportRef, StoreError,
    WorkspaceDraftRevision, WorkspaceFlatQuestionSource,
};

/// Exact media type retained for original QTI profile archives.
pub const QTI_PROFILE_ARCHIVE_MEDIA_TYPE: &str = "application/zip";
/// The hostile-input archive cap is retained at the persistence boundary.
pub const MAX_QTI_PROFILE_ARCHIVE_BYTES: u64 = 32 * 1024 * 1024;
/// Ordered maps may contain 100 individually bounded vendor identifiers.
pub const MAX_FLAT_IMPORT_CHOICE_MAP_BYTES: usize = 2 * 1024 * 1024;

const MAX_SOURCE_ITEM_IDENTIFIER_CHARS: usize = 1_024;
const MAX_CONVERSION_VERSION_BYTES: usize = 128;

mod validation;
use validation::{
    validate_conversion_inputs, validate_published_archive, validate_source_item_identifier,
    validate_workspace_archive,
};

/// Closed vendor-profile tuples that may create durable flat provenance.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PersistedFlatImportProfile {
    CanvasQti12V1,
    BlackboardQti21V1,
}

impl PersistedFlatImportProfile {
    pub const fn profile_id(self) -> &'static str {
        match self {
            Self::CanvasQti12V1 => "canvas-qti-1.2-static-single-choice/v1",
            Self::BlackboardQti21V1 => "blackboard-qti-2.1-static-single-choice-pool/v1",
        }
    }

    pub const fn profile_version(self) -> &'static str {
        "v1"
    }

    pub const fn mapping_version(self) -> &'static str {
        "v1"
    }

    #[allow(dead_code)] // Used by the PostgreSQL decoder in WP-QTI-8.
    pub(crate) fn from_stored(
        profile_id: &str,
        profile_version: &str,
        mapping_version: &str,
    ) -> Result<Self, StoreError> {
        match (profile_id, profile_version, mapping_version) {
            ("canvas-qti-1.2-static-single-choice/v1", "v1", "v1") => Ok(Self::CanvasQti12V1),
            ("blackboard-qti-2.1-static-single-choice-pool/v1", "v1", "v1") => {
                Ok(Self::BlackboardQti21V1)
            }
            _ => Err(StoreError::InvalidRecord(
                "stored flat-import profile contract is unsupported".to_string(),
            )),
        }
    }
}

/// Version of the server-owned composition from a QTI mapping to flat v2.
///
/// The server owns the meaning of this value. Persistence only enforces a
/// bounded, migration-friendly identifier and stores it without interpretation.
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct FlatImportConversionVersion(String);

impl FlatImportConversionVersion {
    pub fn new(value: impl Into<String>) -> Result<Self, StoreError> {
        let value = value.into();
        if value.is_empty()
            || value.len() > MAX_CONVERSION_VERSION_BYTES
            || !value.bytes().all(|byte| {
                byte.is_ascii_lowercase()
                    || byte.is_ascii_digit()
                    || matches!(byte, b'-' | b'_' | b'/')
            })
        {
            return Err(StoreError::InvalidRecord(
                "flat-import conversion version is invalid".to_string(),
            ));
        }
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Debug for FlatImportConversionVersion {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_tuple("FlatImportConversionVersion")
            .field(&self.0)
            .finish()
    }
}

/// Opaque canonical ordered vendor-to-PLE map retained as private provenance.
///
/// ```compile_fail
/// use learning_data_access::FlatImportChoiceMapPayload;
/// fn needs_serialize<T: serde::Serialize>() {}
/// needs_serialize::<FlatImportChoiceMapPayload>();
/// ```
#[derive(Clone, PartialEq, Eq)]
pub struct FlatImportChoiceMapPayload(Vec<u8>);

impl FlatImportChoiceMapPayload {
    /// Wraps bytes emitted by the adapter-owned canonical provenance encoder.
    pub fn from_canonical_bytes(bytes: Vec<u8>) -> Result<Self, StoreError> {
        if bytes.is_empty() || bytes.len() > MAX_FLAT_IMPORT_CHOICE_MAP_BYTES {
            return Err(StoreError::InvalidRecord(
                "flat-import choice map exceeds its private payload limit".to_string(),
            ));
        }
        Ok(Self(bytes))
    }

    pub fn sha256(&self) -> Sha256Digest {
        Sha256Digest::compute(&self.0)
    }

    pub fn size_bytes(&self) -> usize {
        self.0.len()
    }

    #[allow(dead_code)] // Used by protected payload persistence in WP-QTI-8.
    pub(crate) fn bytes(&self) -> &[u8] {
        &self.0
    }
}

impl std::fmt::Debug for FlatImportChoiceMapPayload {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("FlatImportChoiceMapPayload([redacted])")
    }
}

/// Immutable checksums that bind a committed profile result to one conversion.
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct FlatImportIntegrityDigests {
    pub normalized_item_sha256: Sha256Digest,
    pub profile_report_sha256: Sha256Digest,
    pub public_mapping_sha256: Sha256Digest,
    pub private_mapping_sha256: Sha256Digest,
    pub mapping_sha256: Sha256Digest,
    pub warning_sha256: Sha256Digest,
    pub choice_map_sha256: Sha256Digest,
}

impl std::fmt::Debug for FlatImportIntegrityDigests {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("FlatImportIntegrityDigests([redacted])")
    }
}

/// Closed profile evidence staged while one QTI import remains prepared.
///
/// The source item identifier is also the persisted import item identifier;
/// the closed profile disposition requires those identities to be equal. This
/// type deliberately implements neither `Debug` nor serialization so the
/// evidence cannot drift into browser or diagnostic projections.
///
/// ```compile_fail
/// use learning_data_access::QtiProfileImportEvidence;
/// fn needs_debug<T: std::fmt::Debug>() {}
/// needs_debug::<QtiProfileImportEvidence>();
/// ```
///
/// ```compile_fail
/// use learning_data_access::QtiProfileImportEvidence;
/// fn needs_serialize<T: serde::Serialize>() {}
/// needs_serialize::<QtiProfileImportEvidence>();
/// ```
#[derive(Clone, PartialEq, Eq)]
pub struct QtiProfileImportEvidence {
    import: QtiImportRef,
    source_item_identifier: String,
    profile: PersistedFlatImportProfile,
    digests: FlatImportIntegrityDigests,
}

/// Crate-private persistence view for the prepared-import staging capability.
///
/// Backend owners receive typed profile and digest values without introducing
/// a public raw-field or serialization boundary.
#[allow(dead_code)] // Consumed by the Memory/PostgreSQL implementations in WP-QTI-8.
pub(crate) struct QtiProfileImportEvidencePersistenceParts<'a> {
    pub(crate) import: QtiImportRef,
    pub(crate) source_item_identifier: &'a str,
    pub(crate) profile: PersistedFlatImportProfile,
    pub(crate) digests: FlatImportIntegrityDigests,
}

impl QtiProfileImportEvidence {
    /// Creates evidence for one accepted item in a prepared QTI import.
    pub fn new(
        import: QtiImportRef,
        source_item_identifier: String,
        profile: PersistedFlatImportProfile,
        digests: FlatImportIntegrityDigests,
    ) -> Result<Self, StoreError> {
        validate_source_item_identifier(&source_item_identifier)?;
        Ok(Self {
            import,
            source_item_identifier,
            profile,
            digests,
        })
    }

    #[allow(dead_code)] // Consumed by the backend staging implementations in WP-QTI-8.
    pub(crate) fn persistence_parts(&self) -> QtiProfileImportEvidencePersistenceParts<'_> {
        QtiProfileImportEvidencePersistenceParts {
            import: self.import,
            source_item_identifier: &self.source_item_identifier,
            profile: self.profile,
            digests: self.digests,
        }
    }
}

#[derive(Clone, PartialEq, Eq)]
struct FlatImportOriginEvidence {
    source_item_identifier: String,
    profile: PersistedFlatImportProfile,
    conversion_version: FlatImportConversionVersion,
    source_archive_sha256: Sha256Digest,
    digests: FlatImportIntegrityDigests,
    mapped_canonical_source_sha256: Sha256Digest,
    acknowledged_by: UserId,
    acknowledged_at: ActivityTimestamp,
    choice_map: FlatImportChoiceMapPayload,
}

/// Crate-private database view of the private evidence shared by current and
/// immutable origins. Sibling Memory/PostgreSQL modules can persist it without
/// broadening the public API or making raw vendor identifiers serializable.
#[allow(dead_code)] // Consumed by the Memory/PostgreSQL implementations in WP-QTI-8.
pub(crate) struct FlatImportOriginPersistenceParts<'a> {
    pub(crate) source_item_identifier: &'a str,
    pub(crate) profile: PersistedFlatImportProfile,
    pub(crate) conversion_version: &'a str,
    pub(crate) source_archive_sha256: Sha256Digest,
    pub(crate) digests: FlatImportIntegrityDigests,
    pub(crate) mapped_canonical_source_sha256: Sha256Digest,
    pub(crate) acknowledged_by: UserId,
    pub(crate) acknowledged_at: ActivityTimestamp,
    pub(crate) choice_map: &'a FlatImportChoiceMapPayload,
}

impl FlatImportOriginEvidence {
    #[allow(dead_code)] // Consumed by the backend persistence views in WP-QTI-8.
    fn persistence_parts(&self) -> FlatImportOriginPersistenceParts<'_> {
        FlatImportOriginPersistenceParts {
            source_item_identifier: &self.source_item_identifier,
            profile: self.profile,
            conversion_version: self.conversion_version.as_str(),
            source_archive_sha256: self.source_archive_sha256,
            digests: self.digests,
            mapped_canonical_source_sha256: self.mapped_canonical_source_sha256,
            acknowledged_by: self.acknowledged_by,
            acknowledged_at: self.acknowledged_at,
            choice_map: &self.choice_map,
        }
    }
}

/// Current private origin for one editable flat-question workspace.
///
/// This type deliberately implements neither `Debug` nor serialization. It
/// contains a raw vendor item identifier and the private ordered choice map.
#[derive(Clone, PartialEq, Eq)]
pub struct WorkspaceFlatImportOrigin {
    import: QtiImportRef,
    source_archive: ObjectRecord,
    evidence: FlatImportOriginEvidence,
}

impl WorkspaceFlatImportOrigin {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        import: QtiImportRef,
        source_item_identifier: String,
        profile: PersistedFlatImportProfile,
        conversion_version: FlatImportConversionVersion,
        source_archive: ObjectRecord,
        digests: FlatImportIntegrityDigests,
        mapped_canonical_source_sha256: Sha256Digest,
        acknowledged_by: UserId,
        acknowledged_at: ActivityTimestamp,
        choice_map: FlatImportChoiceMapPayload,
    ) -> Result<Self, StoreError> {
        validate_source_item_identifier(&source_item_identifier)?;
        validate_workspace_archive(&source_archive, import)?;
        if choice_map.sha256() != digests.choice_map_sha256 {
            return Err(StoreError::InvalidRecord(
                "flat-import choice map checksum does not match its provenance".to_string(),
            ));
        }
        Ok(Self {
            import,
            source_archive: source_archive.clone(),
            evidence: FlatImportOriginEvidence {
                source_item_identifier,
                profile,
                conversion_version,
                source_archive_sha256: source_archive.sha256,
                digests,
                mapped_canonical_source_sha256,
                acknowledged_by,
                acknowledged_at,
                choice_map,
            },
        })
    }

    pub fn import(&self) -> QtiImportRef {
        self.import
    }

    pub fn source_archive(&self) -> &ObjectRecord {
        &self.source_archive
    }

    pub fn source_item_identifier(&self) -> &str {
        &self.evidence.source_item_identifier
    }

    pub fn profile(&self) -> PersistedFlatImportProfile {
        self.evidence.profile
    }

    pub fn conversion_version(&self) -> &FlatImportConversionVersion {
        &self.evidence.conversion_version
    }

    pub fn digests(&self) -> FlatImportIntegrityDigests {
        self.evidence.digests
    }

    pub fn mapped_canonical_source_sha256(&self) -> Sha256Digest {
        self.evidence.mapped_canonical_source_sha256
    }

    pub fn acknowledged_by(&self) -> UserId {
        self.evidence.acknowledged_by
    }

    pub fn acknowledged_at(&self) -> ActivityTimestamp {
        self.evidence.acknowledged_at
    }

    pub fn identity(&self) -> WorkspaceFlatImportOriginIdentity {
        WorkspaceFlatImportOriginIdentity {
            import: self.import,
            source_archive_id: self.source_archive.id,
            evidence: FlatImportOriginIdentityEvidence::from(&self.evidence),
        }
    }

    #[allow(dead_code)] // Consumed by the Memory/PostgreSQL implementations in WP-QTI-8.
    pub(crate) fn persistence_parts(&self) -> FlatImportOriginPersistenceParts<'_> {
        self.evidence.persistence_parts()
    }
}

#[derive(Clone, PartialEq, Eq)]
struct FlatImportOriginIdentityEvidence {
    source_item_identifier: String,
    profile: PersistedFlatImportProfile,
    conversion_version: FlatImportConversionVersion,
    source_archive_sha256: Sha256Digest,
    digests: FlatImportIntegrityDigests,
    mapped_canonical_source_sha256: Sha256Digest,
    acknowledged_by: UserId,
    acknowledged_at: ActivityTimestamp,
}

impl From<&FlatImportOriginEvidence> for FlatImportOriginIdentityEvidence {
    fn from(evidence: &FlatImportOriginEvidence) -> Self {
        Self {
            source_item_identifier: evidence.source_item_identifier.clone(),
            profile: evidence.profile,
            conversion_version: evidence.conversion_version.clone(),
            source_archive_sha256: evidence.source_archive_sha256,
            digests: evidence.digests,
            mapped_canonical_source_sha256: evidence.mapped_canonical_source_sha256,
            acknowledged_by: evidence.acknowledged_by,
            acknowledged_at: evidence.acknowledged_at,
        }
    }
}

/// Exact current-origin selector used under the publication draft lock.
#[derive(Clone, PartialEq, Eq)]
pub struct WorkspaceFlatImportOriginIdentity {
    import: QtiImportRef,
    source_archive_id: question_model::ObjectId,
    evidence: FlatImportOriginIdentityEvidence,
}

/// Server-prepared selector and typed archive candidate for publication.
#[derive(Clone, PartialEq, Eq)]
pub struct FlatImportPublicationPromotion {
    expected_current_origin: WorkspaceFlatImportOriginIdentity,
    published_archive: ObjectRecord,
}

impl FlatImportPublicationPromotion {
    pub fn new(
        current: &WorkspaceFlatImportOrigin,
        reference: ProblemVersionRef,
        published_archive: ObjectRecord,
    ) -> Result<Self, StoreError> {
        validate_published_archive(current, reference, &published_archive)?;
        Ok(Self {
            expected_current_origin: current.identity(),
            published_archive,
        })
    }

    pub fn expected_current_origin(&self) -> &WorkspaceFlatImportOriginIdentity {
        &self.expected_current_origin
    }

    pub fn published_archive(&self) -> &ObjectRecord {
        &self.published_archive
    }
}

impl std::fmt::Debug for FlatImportPublicationPromotion {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("FlatImportPublicationPromotion([redacted])")
    }
}

/// Immutable published lineage copied from the locked current origin.
#[derive(Clone, PartialEq, Eq)]
pub struct PublishedFlatImportOrigin {
    reference: ProblemVersionRef,
    import: question_model::WorkspaceImportId,
    published_archive: ObjectRecord,
    evidence: FlatImportOriginEvidence,
}

impl PublishedFlatImportOrigin {
    #[allow(dead_code)] // Used by Memory/PostgreSQL publication in WP-QTI-8.
    pub(crate) fn from_current(
        current: &WorkspaceFlatImportOrigin,
        reference: ProblemVersionRef,
        published_archive: ObjectRecord,
    ) -> Result<Self, StoreError> {
        validate_published_archive(current, reference, &published_archive)?;
        Ok(Self {
            reference,
            import: current.import.import,
            published_archive,
            evidence: current.evidence.clone(),
        })
    }

    pub fn reference(&self) -> ProblemVersionRef {
        self.reference
    }

    pub fn published_archive(&self) -> &ObjectRecord {
        &self.published_archive
    }

    #[allow(dead_code)] // Consumed by the PostgreSQL decoder in WP-QTI-8.
    pub(crate) fn import(&self) -> question_model::WorkspaceImportId {
        self.import
    }

    #[allow(dead_code)] // Consumed by the Memory/PostgreSQL implementations in WP-QTI-8.
    pub(crate) fn persistence_parts(&self) -> FlatImportOriginPersistenceParts<'_> {
        self.evidence.persistence_parts()
    }
}

/// One all-or-nothing profile conversion into the existing flat-question path.
#[derive(Clone)]
pub struct QtiProfileFlatConversionCommand {
    pub expected_revision: Option<WorkspaceDraftRevision>,
    pub draft: DraftRecord,
    pub source: ObjectRecord,
    pub canonical_source_sha256: String,
    pub public_binding_sha256: String,
    pub grading: FlatQuestionGradingPayload,
    pub origin: WorkspaceFlatImportOrigin,
}

impl QtiProfileFlatConversionCommand {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        expected_revision: Option<WorkspaceDraftRevision>,
        draft: DraftRecord,
        source: ObjectRecord,
        canonical_source_sha256: String,
        public_binding_sha256: String,
        grading: FlatQuestionGradingPayload,
        origin: WorkspaceFlatImportOrigin,
    ) -> Result<Self, StoreError> {
        validate_conversion_inputs(
            &draft,
            &source,
            &canonical_source_sha256,
            &public_binding_sha256,
            &grading,
            &origin,
        )?;
        Ok(Self {
            expected_revision,
            draft,
            source,
            canonical_source_sha256,
            public_binding_sha256,
            grading,
            origin,
        })
    }
}

impl std::fmt::Debug for QtiProfileFlatConversionCommand {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("QtiProfileFlatConversionCommand")
            .field("expected_revision", &self.expected_revision)
            .field("workspace", &self.draft.question.workspace)
            .field("source", &"[redacted]")
            .field("origin", &"[redacted]")
            .finish()
    }
}

/// Atomic persistence and private read boundary for converted flat provenance.
#[async_trait]
pub trait FlatImportProvenanceStore: Send + Sync {
    /// Stages closed evidence only while its QTI import remains prepared.
    ///
    /// The accepted result's exact normalized digest must match this evidence.
    /// An exact replay is idempotent; a divergent replay or any refusal leaves
    /// existing evidence unchanged. Conversion later revalidates this staged
    /// evidence only after the import has committed.
    async fn stage_qti_profile_import_evidence(
        &self,
        context: ActorContext,
        evidence: QtiProfileImportEvidence,
    ) -> Result<(), StoreError>;

    /// Under the module's fixed lock order, revalidates the committed import's
    /// exact archive, selected item and accepted result, plus the staged
    /// profile/version/mapping contract and every integrity digest. A
    /// successful compare-and-swap writes the draft, canonical flat source,
    /// private grading material, and current origin in one transaction; every
    /// refusal leaves all four unchanged. Installing the new origin pin
    /// precedes releasing any replaced import pin.
    async fn convert_qti_profile_item_to_flat(
        &self,
        context: ActorContext,
        actor: UserId,
        command: QtiProfileFlatConversionCommand,
    ) -> Result<WorkspaceFlatQuestionSource, StoreError>;

    /// Returns the private current origin only to an authorized workspace
    /// actor. Foreign, inaccessible, and absent origins are non-enumerating.
    async fn workspace_flat_import_origin(
        &self,
        context: ActorContext,
        actor: UserId,
        workspace: WorkspaceId,
    ) -> Result<Option<WorkspaceFlatImportOrigin>, StoreError>;
}

#[cfg(test)]
#[path = "flat_import_provenance/tests.rs"]
mod tests;

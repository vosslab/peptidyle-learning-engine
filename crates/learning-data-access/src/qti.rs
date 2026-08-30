//! Private QTI import registry and isolated grading capability.

use async_trait::async_trait;
use objects::{ObjectRecord, Sha256Digest};
use question_model::{AssetId, ObjectId, ProblemVersionRef, WorkspaceId, WorkspaceImportId};
use serde::{Deserialize, Deserializer, Serialize, Serializer, ser::SerializeStruct};

use crate::{
    ActorContext, AssetDeliveryRecord, JobId, JobLeaseToken, PersistedFlatImportProfile, StoreError,
};

/// Private QTI staging evidence selected before any published identity exists.
///
/// The server copies the referenced bytes to candidate published object keys
/// first. Store promotion validates this exact committed staging import and
/// atomically records its catalog asset bindings and grader-owned material.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QtiPublicationPromotion {
    pub staging: QtiImportRef,
    pub assets: Vec<AssetDeliveryRecord>,
}

/// Workspace/import address for a private, immutable QTI staging record.
///
/// This is deliberately not a browser DTO and contains no published identity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QtiImportRef {
    pub workspace: WorkspaceId,
    pub import: WorkspaceImportId,
}

/// Browser-safe identity and integrity record for one item in a QTI package.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QtiImportItem {
    pub item_id: String,
    /// Digest of the canonical, server-sanitized item representation.
    pub model_sha256: Sha256Digest,
    /// Logical assets the item references. The registry verifies each exists.
    pub assets: Vec<AssetId>,
}

/// A supported package feature retained for author-facing diagnostics.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QtiUnsupportedFeature {
    pub code: String,
    pub location: String,
    pub detail: String,
}

/// Durable disposition for one source item in an import batch.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum QtiImportItemStatus {
    Accepted,
    Rejected,
}

/// Bounded answer-free result for one source item, including rejected items.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QtiImportItemResult {
    pub source_identifier: String,
    /// Optional bounded source title safe for an instructor-facing report.
    pub title: Option<String>,
    pub item_id: Option<String>,
    pub normalized_sha256: Option<Sha256Digest>,
    pub status: QtiImportItemStatus,
    /// Refusal diagnostics that explain why an item cannot be converted.
    #[serde(default)]
    pub diagnostics: Vec<QtiUnsupportedFeature>,
    /// Explicit PLE authoring defaults applied during profile mapping.
    #[serde(default)]
    pub defaults: Vec<QtiUnsupportedFeature>,
    pub warnings: Vec<QtiUnsupportedFeature>,
}

/// Closed persisted identity for one recognized QTI profile report.
///
/// This is private registry evidence, not a browser report DTO. Profile labels,
/// report revisions, review tokens, and private item mappings intentionally do
/// not cross this boundary.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QtiImportProfileSummary {
    profile: PersistedFlatImportProfile,
    profile_report_sha256: Sha256Digest,
    defaults: Vec<QtiUnsupportedFeature>,
}

impl QtiImportProfileSummary {
    /// Creates a summary from the storage-owned closed profile contract and
    /// the adapter-owned canonical safe-report digest.
    pub fn new(
        profile: PersistedFlatImportProfile,
        profile_report_sha256: Sha256Digest,
        defaults: Vec<QtiUnsupportedFeature>,
    ) -> Result<Self, StoreError> {
        let summary = Self {
            profile,
            profile_report_sha256,
            defaults,
        };
        crate::publication_validation::validate_qti_import_profile_summary(&summary)?;
        Ok(summary)
    }

    pub const fn profile(&self) -> PersistedFlatImportProfile {
        self.profile
    }

    pub const fn profile_id(&self) -> &'static str {
        self.profile.profile_id()
    }

    pub const fn profile_version(&self) -> &'static str {
        self.profile.profile_version()
    }

    pub const fn mapping_version(&self) -> &'static str {
        self.profile.mapping_version()
    }

    pub const fn profile_report_sha256(&self) -> Sha256Digest {
        self.profile_report_sha256
    }

    pub fn defaults(&self) -> &[QtiUnsupportedFeature] {
        &self.defaults
    }

    pub(crate) fn validate(&self) -> Result<(), StoreError> {
        PersistedFlatImportProfile::from_stored(
            self.profile_id(),
            self.profile_version(),
            self.mapping_version(),
        )
        .map(|_| ())
    }
}

impl Serialize for QtiImportProfileSummary {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut state = serializer.serialize_struct("QtiImportProfileSummary", 5)?;
        state.serialize_field("profileId", self.profile_id())?;
        state.serialize_field("profileVersion", self.profile_version())?;
        state.serialize_field("mappingVersion", self.mapping_version())?;
        state.serialize_field("profileReportSha256", &self.profile_report_sha256)?;
        state.serialize_field("defaults", &self.defaults)?;
        state.end()
    }
}

impl<'de> Deserialize<'de> for QtiImportProfileSummary {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase", deny_unknown_fields)]
        struct StoredSummary {
            profile_id: String,
            profile_version: String,
            mapping_version: String,
            profile_report_sha256: Sha256Digest,
            #[serde(default)]
            defaults: Vec<QtiUnsupportedFeature>,
        }

        let stored = StoredSummary::deserialize(deserializer)?;
        let profile = PersistedFlatImportProfile::from_stored(
            &stored.profile_id,
            &stored.profile_version,
            &stored.mapping_version,
        )
        .map_err(serde::de::Error::custom)?;
        Self::new(profile, stored.profile_report_sha256, stored.defaults)
            .map_err(serde::de::Error::custom)
    }
}

/// Complete safe metadata for a private QTI import.
///
/// Grading choices and archive bytes are intentionally absent. This type is
/// persistence-only; it is never included in question-model serialization.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QtiImportRegistry {
    pub reference: QtiImportRef,
    pub source: ObjectRecord,
    pub source_format: String,
    pub source_identifier: Option<String>,
    pub importer: String,
    pub parse_schema: String,
    pub adapter_version: String,
    /// Present only for an exact recognized profile. Legacy and generic
    /// imports omit this field and continue to decode as `None`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub profile_summary: Option<QtiImportProfileSummary>,
    pub items: Vec<QtiImportItem>,
    pub item_results: Vec<QtiImportItemResult>,
    pub assets: Vec<ObjectRecord>,
    pub unsupported_features: Vec<QtiUnsupportedFeature>,
}

/// Opaque answer-bearing material stored only in the grader table.
///
/// It has no serialization or display implementation and redacts itself when
/// diagnostics inspect a registration. The import writer can persist it; only
/// [`QtiGradingStore`] can read it back.
#[derive(Clone, PartialEq, Eq)]
pub struct QtiImportGradingPayload(Vec<u8>);

impl QtiImportGradingPayload {
    pub fn new(bytes: Vec<u8>) -> Result<Self, StoreError> {
        if bytes.is_empty() || bytes.len() > 256 * 1024 {
            return Err(StoreError::InvalidRecord(
                "QTI grading payload must contain 1 to 262144 bytes".to_string(),
            ));
        }
        Ok(Self(bytes))
    }

    /// Returns an integrity digest without disclosing answer-bearing bytes.
    pub fn sha256(&self) -> Sha256Digest {
        Sha256Digest::compute(&self.0)
    }

    /// Decodes the one correct choice stored by the bounded QTI importer.
    ///
    /// The bytes remain private to the dedicated grader capability. This
    /// method is intentionally the narrowest server-side handoff: callers can
    /// construct an ordinary grading key, but cannot inspect, serialize, or
    /// log the archived grading payload itself.
    pub fn server_correct_choice(&self) -> Result<question_model::response::ChoiceId, StoreError> {
        serde_json::from_slice(&self.0).map_err(|_| {
            StoreError::InvalidRecord("stored QTI grading payload is invalid".to_string())
        })
    }

    pub(crate) fn bytes(&self) -> &[u8] {
        &self.0
    }
}

impl std::fmt::Debug for QtiImportGradingPayload {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("QtiImportGradingPayload([redacted])")
    }
}

/// One item plus its answer-bearing, server-only grading binding.
#[derive(Clone, PartialEq, Eq)]
pub struct QtiImportItemRegistration {
    pub item: QtiImportItem,
    pub grading: QtiImportGradingPayload,
}

impl std::fmt::Debug for QtiImportItemRegistration {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("QtiImportItemRegistration")
            .field("item", &self.item)
            .field("grading", &"[redacted]")
            .finish()
    }
}

/// All metadata supplied after object bytes have already been written and verified.
#[derive(Clone)]
pub struct CreateQtiImportCommand {
    pub registry: QtiImportRegistry,
    pub item_bindings: Vec<QtiImportItemRegistration>,
}

/// Private worker preparation bound to one active QTI-import lease.
#[derive(Clone)]
pub struct PrepareClaimedQtiImport {
    pub job: JobId,
    pub lease: JobLeaseToken,
    pub command: CreateQtiImportCommand,
}

/// Exact private worker claim allowed to expose a prepared QTI import.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CommitPreparedQtiImport {
    pub job: JobId,
    pub lease: JobLeaseToken,
    pub reference: QtiImportRef,
    pub source_object: ObjectId,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommitPreparedQtiImportOutcome {
    Committed,
    ClaimNoLongerActive,
}

impl std::fmt::Debug for CreateQtiImportCommand {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("CreateQtiImportCommand")
            .field("registry", &self.registry)
            .field("item_binding_count", &self.item_bindings.len())
            .finish()
    }
}

/// Private QTI staging registry. Object bytes must already exist when this is
/// called; this transaction records either the entire import or none of it.
#[async_trait]
pub trait QtiImportStore: Send + Sync {
    /// Persists fully validated but invisible worker preparation.
    async fn prepare_qti_import(
        &self,
        context: ActorContext,
        command: CreateQtiImportCommand,
    ) -> Result<(), StoreError>;

    /// Persists invisible worker preparation only while the exact lease still
    /// names the command's immutable workspace, import, and source object.
    async fn prepare_claimed_qti_import(
        &self,
        command: PrepareClaimedQtiImport,
    ) -> Result<(), StoreError>;

    /// Makes a prepared import visible and completes the exact active lease.
    async fn commit_prepared_qti_import(
        &self,
        command: CommitPreparedQtiImport,
    ) -> Result<CommitPreparedQtiImportOutcome, StoreError>;

    /// Resolves safe staging metadata for an actor currently bound to the workspace.
    async fn get_qti_import(
        &self,
        context: ActorContext,
        workspace: WorkspaceId,
        import: WorkspaceImportId,
    ) -> Result<Option<QtiImportRegistry>, StoreError>;
}

/// Deliberately narrow answer-bearing QTI capability.
///
/// Browser, catalog, draft, object-delivery, and normal import-registry code
/// never require this trait. Implementations use the database grader role for
/// its read path.
#[async_trait]
pub trait QtiGradingStore: Send + Sync {
    /// Reads one committed published QTI binding only while trusted server
    /// issue preparation is copying it into an attempt-local contract.
    ///
    /// First-grade code must use `IssuedQtiGradingContractV1` and therefore
    /// has no store method it can call for catalog recovery.
    async fn qti_publication_grading(
        &self,
        reference: ProblemVersionRef,
        item_id: &str,
    ) -> Result<Option<QtiImportGradingPayload>, StoreError>;

    async fn qti_import_grading(
        &self,
        workspace: WorkspaceId,
        import: WorkspaceImportId,
        item_id: &str,
    ) -> Result<Option<QtiImportGradingPayload>, StoreError>;
}

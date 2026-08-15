//! Flat-question staging types and flat-question-only persistence traits.

use async_trait::async_trait;
use base64::Engine as _;
use objects::{ObjectCategory, ObjectKey, ObjectRecord, Sha256Digest};
use question_model::{
    DraftQuestionDefinition, DraftQuestionSource, ProblemVersionRef, QuestionDefinition, TenantId,
    WorkspaceId,
};
use serde::{Deserialize, Serialize};

use crate::FlatImportPublicationPromotion;
use crate::{AssetDeliveryRecord, DraftRecord, StoreError, TenantContext, WorkspaceDraftRevision};

/// Canonical source media type for workspace and published flat-question objects.
pub const FLAT_QUESTION_MEDIA_TYPE: &str = "application/vnd.peptidyle.flat-question+json";
/// Maximum accepted byte size for a flat-question payload record.
pub const MAX_FLAT_QUESTION_PAYLOAD_BYTES: usize = 256 * 1024;

/// Answer-bearing grading payload for flat-question publication.
///
/// This payload remains private to grader paths and should not appear in browser
/// payloads.
#[derive(Clone, PartialEq)]
pub struct FlatQuestionGradingPayload {
    bytes: Vec<u8>,
    public_binding_sha256: String,
}

impl Serialize for FlatQuestionGradingPayload {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&base64::engine::general_purpose::STANDARD.encode(&self.bytes))
    }
}

impl<'de> Deserialize<'de> for FlatQuestionGradingPayload {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let encoded = String::deserialize(deserializer)?;
        let bytes = base64::engine::general_purpose::STANDARD
            .decode(encoded.as_bytes())
            .map_err(serde::de::Error::custom)?;
        Self::from_canonical_bytes(bytes).map_err(serde::de::Error::custom)
    }
}

impl FlatQuestionGradingPayload {
    /// Creates durable grading material from the server-only validated type.
    ///
    /// This is deliberately the only public constructor: callers cannot inject
    /// an answer-like JSON object that happens to contain `publicSha256`.
    pub fn from_private(
        private: &grading::flat_question::FlatQuestionPrivate,
    ) -> Result<Self, StoreError> {
        let bytes = private.canonical_bytes().map_err(flat_grading_error)?;
        Self::from_canonical_bytes(bytes)
    }

    /// Decodes the authoritative private model before the runtime uses it.
    pub fn decode_private(
        &self,
    ) -> Result<grading::flat_question::FlatQuestionPrivate, StoreError> {
        grading::flat_question::FlatQuestionPrivate::from_canonical_bytes(&self.bytes)
            .map_err(flat_grading_error)
    }

    /// Reconstructs trusted persisted canonical bytes after PostgreSQL verifies
    /// its opaque base64 envelope. This crate intentionally does not expose a
    /// raw-byte constructor to application code.
    pub(crate) fn from_canonical_bytes(bytes: Vec<u8>) -> Result<Self, StoreError> {
        if bytes.is_empty() || bytes.len() > MAX_FLAT_QUESTION_PAYLOAD_BYTES {
            return Err(StoreError::InvalidRecord(
                "flat-question grading payload must contain 1 to 262144 bytes".to_string(),
            ));
        }
        let private = grading::flat_question::FlatQuestionPrivate::from_canonical_bytes(&bytes)
            .map_err(flat_grading_error)?;
        let public_binding_sha256 = private.public_binding_sha256().to_string();
        validate_sha256_lower_hex(&public_binding_sha256)?;
        Ok(Self {
            bytes,
            public_binding_sha256,
        })
    }

    /// SHA-256 of the private grading payload for binding checks and storage
    /// integrity bookkeeping.
    pub fn sha256(&self) -> Sha256Digest {
        Sha256Digest::compute(&self.bytes)
    }

    /// Raw byte view for internal grader-store envelope code only.
    #[cfg(feature = "postgres")]
    pub(crate) fn bytes(&self) -> &[u8] {
        &self.bytes
    }

    /// Public payload checksum carried by the private grading material.
    pub fn public_binding_sha256(&self) -> &str {
        &self.public_binding_sha256
    }

    /// Verifies private grading material against one exact immutable published
    /// definition without evaluating a learner response.
    pub fn validate_for_question(&self, question: &QuestionDefinition) -> Result<(), StoreError> {
        self.decode_private()?
            .validate_for_question(question)
            .map_err(flat_grading_error)
    }

    /// Rebinds trusted staged grading material to a server-prepared published
    /// definition without exposing its answer-bearing bytes.
    pub fn rebind_to_draft(&self, draft: &DraftQuestionDefinition) -> Result<Self, StoreError> {
        let rebound = self
            .decode_private()?
            .rebind_to_draft(draft)
            .map_err(flat_grading_error)?;
        Self::from_private(&rebound)
    }
}

/// Immutable server-only flat-question grading authority frozen at issuance.
///
/// The definition is answer-free, while `grading` is intentionally private.
/// Keeping them together lets first submit verify the exact publication-era
/// public binding without loading a later catalog revision or grader record.
#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub struct IssuedFlatGradingContract {
    question: QuestionDefinition,
    grading: FlatQuestionGradingPayload,
}

impl IssuedFlatGradingContract {
    /// Creates a contract only when the private material binds to this exact
    /// immutable published definition.
    pub fn new(
        question: QuestionDefinition,
        grading: FlatQuestionGradingPayload,
    ) -> Result<Self, StoreError> {
        grading.validate_for_question(&question)?;
        Ok(Self { question, grading })
    }

    /// The exact answer-free published definition retained for private grade.
    pub fn question(&self) -> &QuestionDefinition {
        &self.question
    }

    /// The private grader material. This is available only inside trusted
    /// server/store capabilities and never serializes into a learner DTO.
    pub fn grading(&self) -> &FlatQuestionGradingPayload {
        &self.grading
    }

    pub(crate) fn validate(&self) -> Result<(), StoreError> {
        self.grading.validate_for_question(&self.question)
    }
}

impl std::fmt::Debug for IssuedFlatGradingContract {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("IssuedFlatGradingContract")
            .field("question", &self.question)
            .field("grading", &"[REDACTED]")
            .finish()
    }
}

fn flat_grading_error(error: grading::flat_question::FlatQuestionError) -> StoreError {
    StoreError::InvalidRecord(format!(
        "flat-question grading material is invalid: {error}"
    ))
}

impl std::fmt::Debug for FlatQuestionGradingPayload {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("FlatQuestionGradingPayload([redacted])")
    }
}

/// Answer-free workspace-level binding to one flat-question source object.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceFlatQuestionSource {
    /// Source tenant.
    pub tenant: TenantId,
    /// Author workspace that owns the staged source.
    pub workspace: WorkspaceId,
    /// Draft revision this binding currently describes.
    pub workspace_revision: WorkspaceDraftRevision,
    /// Exact native family string from the staged draft.
    pub source_family: String,
    /// Verified private workspace source object.
    pub source_record: ObjectRecord,
    /// Canonical source SHA-256 in lowercase hexadecimal.
    pub canonical_source_sha256: String,
    /// Public binding SHA-256 in lowercase hexadecimal.
    pub public_binding_sha256: String,
}

impl WorkspaceFlatQuestionSource {
    pub fn new(
        tenant: TenantId,
        workspace: WorkspaceId,
        workspace_revision: WorkspaceDraftRevision,
        source_family: String,
        source_record: ObjectRecord,
        canonical_source_sha256: String,
        public_binding_sha256: String,
    ) -> Result<Self, StoreError> {
        validate_sha256_lower_hex(&canonical_source_sha256)?;
        validate_sha256_lower_hex(&public_binding_sha256)?;
        validate_workspace_flat_source_record(&tenant, &workspace, &source_record)?;
        if source_record.sha256.to_string() != canonical_source_sha256 {
            return Err(StoreError::InvalidRecord(
                "flat-question canonical checksum must match the source object".to_string(),
            ));
        }
        if !grading::flat_question::is_flat_question_family(&source_family) {
            return Err(StoreError::InvalidRecord(
                "flat-question source family is unsupported".to_string(),
            ));
        }
        Ok(Self {
            tenant,
            workspace,
            workspace_revision,
            source_family,
            source_record,
            canonical_source_sha256,
            public_binding_sha256,
        })
    }
}

fn validate_sha256_lower_hex(value: &str) -> Result<(), StoreError> {
    if value.len() != 64
        || !value
            .as_bytes()
            .iter()
            .all(|byte| matches!(byte, b'0'..=b'9' | b'a'..=b'f'))
    {
        return Err(StoreError::InvalidRecord(
            "flat-question SHA-256 digest must be 64 lowercase hexadecimal characters".to_string(),
        ));
    }
    Ok(())
}

pub(crate) fn validate_workspace_flat_source_record(
    tenant: &TenantId,
    workspace: &WorkspaceId,
    record: &ObjectRecord,
) -> Result<(), StoreError> {
    let ObjectKey::WorkspaceQuestionSource {
        tenant: source_tenant,
        workspace: source_workspace,
        object,
    } = &record.key
    else {
        return Err(StoreError::InvalidRecord(
            "flat-question source must use the workspace question source key".to_string(),
        ));
    };
    if source_tenant != tenant || source_workspace != workspace || record.id != *object {
        return Err(StoreError::InvalidRecord(
            "flat-question source key must match the workspace source record identity".to_string(),
        ));
    }
    if record.bucket != objects::Bucket::PrivateContent
        || record.key.bucket() != objects::Bucket::PrivateContent
    {
        return Err(StoreError::InvalidRecord(
            "flat-question source must be stored in the private-content bucket".to_string(),
        ));
    }
    if record.category != ObjectCategory::Source || record.key.category() != ObjectCategory::Source
    {
        return Err(StoreError::InvalidRecord(
            "flat-question source must be a source object".to_string(),
        ));
    }
    if record.version.is_some() {
        return Err(StoreError::InvalidRecord(
            "flat-question source must not have a public version".to_string(),
        ));
    }
    if record.size_bytes == 0 || record.size_bytes as usize > MAX_FLAT_QUESTION_PAYLOAD_BYTES {
        return Err(StoreError::InvalidRecord(
            "flat-question source must be at most 262144 bytes".to_string(),
        ));
    }
    if record.media_type != FLAT_QUESTION_MEDIA_TYPE {
        return Err(StoreError::InvalidRecord(
            "flat-question source media type is invalid".to_string(),
        ));
    }
    if record.license.trim().is_empty() || record.provenance.trim().is_empty() {
        return Err(StoreError::InvalidRecord(
            "flat-question source metadata is incomplete".to_string(),
        ));
    }
    Ok(())
}

/// Source binding prepared by the server during publication.
///
/// The caller selects the exact locked source and optional import origin, but
/// cannot supply grading material. Storage promotes only the current private
/// grading value staged by the successful save or conversion.
#[derive(Clone, PartialEq)]
pub struct FlatQuestionPublicationPromotion {
    /// Exact workspace source metadata used to validate publication.
    pub source: WorkspaceFlatQuestionSource,
    /// Present only when the locked current workspace origin must be copied.
    /// A manually authored flat question keeps this `None`; storage rejects an
    /// omitted selector when a trusted current origin exists.
    pub import_origin: Option<FlatImportPublicationPromotion>,
    /// Browser-safe definition after publication-only asset identities have
    /// been assigned. This differs from `source` only when a HOTSPOT surface
    /// is retargeted from its private workspace asset to a fresh immutable
    /// catalog asset for this exact version.
    pub published_question: DraftQuestionDefinition,
    /// Immutable image deliveries prepared by the protected server route.
    /// Empty unless the exact staged source uses a HOTSPOT surface.
    pub assets: Vec<AssetDeliveryRecord>,
}

impl std::fmt::Debug for FlatQuestionPublicationPromotion {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("FlatQuestionPublicationPromotion")
            .field("source", &self.source)
            .field("import_origin", &self.import_origin)
            .field("published_question", &self.published_question)
            .field("assets", &self.assets)
            .finish()
    }
}

/// Atomic draft-and-source write input for one workspace flat question.
///
/// A successful write replaces or creates the editable draft, its private
/// source binding, and its private grading payload under one
/// optimistic-concurrency revision. Implementations must not leave any subset
/// persisted without the others.
///
/// If the workspace has a [`crate::WorkspaceFlatImportOrigin`], this ordinary
/// editor command must preserve it byte-for-byte. It cannot clear, replace, or
/// edit imported lineage; only the dedicated provenance conversion command may
/// atomically install or replace that current origin.
#[derive(Clone)]
pub struct UpsertFlatQuestionCommand {
    /// Revision expected before this binding may replace prior metadata.
    pub expected_revision: Option<WorkspaceDraftRevision>,
    /// Exact draft to bind with the staged flat source.
    pub draft: DraftRecord,
    /// Workspace source object for this binding.
    pub source: ObjectRecord,
    /// Canonical source checksum bound to the public model.
    pub canonical_source_sha256: String,
    /// Public binding checksum for model and grader payload coupling.
    pub public_binding_sha256: String,
    /// Private grading payload produced by the native compiler for this exact
    /// draft and public binding.
    pub grading: FlatQuestionGradingPayload,
}

impl std::fmt::Debug for UpsertFlatQuestionCommand {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let source_family = match &self.draft.question.source {
            DraftQuestionSource::Native { family } => family.as_str(),
            _ => "<non-native>",
        };
        formatter
            .debug_struct("UpsertFlatQuestionCommand")
            .field("expected_revision", &self.expected_revision)
            .field("tenant", &self.draft.tenant)
            .field("workspace", &self.draft.question.workspace)
            .field("source", &"[redacted]")
            .field("source_family", &source_family)
            .field("canonical_source_sha256", &"[redacted]")
            .field("public_binding_sha256", &"[redacted]")
            .field("grading", &"[redacted]")
            .finish()
    }
}

/// Validates the complete ordinary flat-question staging value before a
/// backend mutates draft, source, or private grading state.
pub(crate) fn validate_upsert_flat_question_command(
    command: &UpsertFlatQuestionCommand,
) -> Result<(), StoreError> {
    crate::validate_draft(&command.draft)?;
    grading::flat_question::validate_for_draft(&command.draft.question).map_err(|error| {
        StoreError::InvalidRecord(format!("flat-question draft is invalid: {error}"))
    })?;
    let private = command.grading.decode_private()?;
    private
        .validate_for_draft(&command.draft.question)
        .map_err(|error| {
            StoreError::InvalidRecord(format!(
                "flat-question grading material does not match the staged draft: {error}"
            ))
        })?;
    if command.grading.public_binding_sha256() != command.public_binding_sha256 {
        return Err(StoreError::InvalidRecord(
            "flat-question grading material must match the staged public binding".to_string(),
        ));
    }
    Ok(())
}

/// Flat-question grader-only read path for published versions.
#[async_trait]
pub trait FlatQuestionGradingStore: Send + Sync {
    /// Returns private flat-question grading material for visible published
    /// problems. Returns `None` when the tenant lacks catalog access.
    async fn flat_question_published_grading(
        &self,
        context: TenantContext,
        reference: ProblemVersionRef,
    ) -> Result<Option<FlatQuestionGradingPayload>, StoreError>;
}

/// Workspace persistence for safe flat-question staging metadata.
#[async_trait]
pub trait FlatQuestionStore: Send + Sync {
    /// Atomically creates or replaces one workspace draft, matching
    /// flat-question staging metadata, and current private grading payload.
    /// The returned binding carries the newly assigned draft revision; callers
    /// must use that revision for the next write. Existing flat-import
    /// provenance is preserved unchanged.
    async fn upsert_flat_question(
        &self,
        context: TenantContext,
        actor: crate::UserId,
        command: UpsertFlatQuestionCommand,
    ) -> Result<WorkspaceFlatQuestionSource, StoreError>;

    /// Returns the current binding for authorized workspace actors, if present.
    async fn flat_question_source(
        &self,
        context: TenantContext,
        actor: crate::UserId,
        workspace: WorkspaceId,
    ) -> Result<Option<WorkspaceFlatQuestionSource>, StoreError>;
}

#[cfg(test)]
mod tests {
    use super::{FlatQuestionGradingPayload, WorkspaceFlatQuestionSource};
    use crate::DraftRecord;
    use crate::WorkspaceDraftRevision;
    use objects::{ObjectCategory, ObjectKey, ObjectRecord};
    use question_model::generation::RandomizationDefinition;
    use question_model::identity::ObjectId;
    use question_model::run_policy::{AttemptPolicy, FeedbackDisclosure, TimingPolicy};
    use question_model::taxonomy::License;
    use question_model::{
        DraftQuestionDefinition, DraftQuestionSource, GradingDefinition, QuestionMetadata,
        ResponseDefinition, TenantId, WorkspaceId, WorkspaceImportId,
    };
    use uuid::Uuid;

    use crate::StoreError;

    const FIXTURE: &str = r#"{"format":"pleFlatQuestion","version":2,"title":"Favorite color","prompt":"What is my favorite color?","response":{"kind":"singleChoice","choices":[{"id":"blue","text":"Blue"},{"id":"red","text":"Red"}],"correctChoice":"blue"},"points":1.0,"attemptPolicy":{"maxAttempts":null,"feedback":"immediateFull"},"timingPolicy":{"kind":"untimed"},"license":{"kind":"cc0"},"language":"en-US"}"#;

    fn private() -> grading::flat_question::FlatQuestionPrivate {
        adapter_native::flat_question::FlatQuestionDocument::parse(FIXTURE.as_bytes())
            .expect("fixture should parse")
            .compile(workspace())
            .expect("fixture should compile")
            .into_parts()
            .1
    }

    fn tenant() -> TenantId {
        TenantId::from_uuid(Uuid::nil())
    }

    fn workspace() -> WorkspaceId {
        WorkspaceId::from_uuid(Uuid::nil())
    }

    fn question() -> DraftQuestionDefinition {
        DraftQuestionDefinition {
            workspace: workspace(),
            prompt: vec![],
            source: DraftQuestionSource::Native {
                family: "flat_single_choice_v2".to_string(),
            },
            response: ResponseDefinition::ExternalTool {},
            grading: GradingDefinition::AllOrNothing { points: 1.0 },
            attempt_policy: AttemptPolicy {
                max_attempts: None,
                feedback: FeedbackDisclosure::ImmediateFull,
            },
            metadata: QuestionMetadata {
                title: "x".to_string(),
                tags: vec![],
                taxonomy: vec![],
                license: License::Other {
                    spdx: "CC0-1.0".to_string(),
                },
                language: "en-US".to_string(),
            },
            timing_policy: TimingPolicy::Untimed,
            randomization: RandomizationDefinition::Static,
        }
    }

    #[test]
    fn grading_payload_has_no_public_raw_constructor() {
        let fabricated = br#"{"publicSha256":"0000000000000000000000000000000000000000000000000000000000000000","answerKey":"secret"}"#;
        let err = FlatQuestionGradingPayload::from_canonical_bytes(fabricated.to_vec())
            .expect_err("fabricated answer-like JSON must not be accepted");
        assert!(matches!(err, StoreError::InvalidRecord(_)));
    }

    #[test]
    fn grading_payload_debug_is_redacted() {
        let payload = FlatQuestionGradingPayload::from_private(&private())
            .expect("compiled private fixture should persist");
        let text = format!("{:?}", payload);
        assert!(text.contains("[redacted]"));
    }

    #[test]
    fn grading_payload_decodes_canonical_private_model() {
        let payload = FlatQuestionGradingPayload::from_private(&private())
            .expect("compiled private fixture should persist");
        assert_eq!(
            payload
                .decode_private()
                .expect("payload should decode")
                .public_binding_sha256(),
            payload.public_binding_sha256()
        );
    }

    #[test]
    fn flat_question_source_rejects_wrong_key() {
        let key = ObjectKey::WorkspaceSource {
            tenant: tenant(),
            workspace: workspace(),
            import: WorkspaceImportId::from_uuid(Uuid::nil()),
            object: ObjectId::from_uuid(Uuid::nil()),
        };
        let record = ObjectRecord {
            id: key.object_id(),
            bucket: objects::Bucket::PrivateContent,
            key,
            sha256: objects::Sha256Digest::from_bytes([0u8; 32]),
            size_bytes: 1,
            media_type: super::FLAT_QUESTION_MEDIA_TYPE.to_string(),
            category: ObjectCategory::Source,
            version: None,
            license: "".to_string(),
            provenance: "".to_string(),
            created_at: question_model::ActivityTimestamp::from_unix_millis(0),
        };
        let err = WorkspaceFlatQuestionSource::new(
            tenant(),
            workspace(),
            WorkspaceDraftRevision::INITIAL,
            "flat_single_choice_v2".to_string(),
            record,
            "a".repeat(64),
            "b".repeat(64),
        )
        .expect_err("wrong source key must fail");
        assert!(matches!(err, StoreError::InvalidRecord(_)));
    }

    #[test]
    fn flat_question_source_rejects_bad_media_type() {
        let key = ObjectKey::WorkspaceQuestionSource {
            tenant: tenant(),
            workspace: workspace(),
            object: ObjectId::from_uuid(Uuid::nil()),
        };
        let record = ObjectRecord {
            id: key.object_id(),
            bucket: objects::Bucket::PrivateContent,
            key,
            sha256: objects::Sha256Digest::from_bytes([0u8; 32]),
            size_bytes: 1,
            media_type: "application/unknown".to_string(),
            category: ObjectCategory::Source,
            version: None,
            license: "".to_string(),
            provenance: "".to_string(),
            created_at: question_model::ActivityTimestamp::from_unix_millis(0),
        };
        assert!(
            WorkspaceFlatQuestionSource::new(
                tenant(),
                workspace(),
                WorkspaceDraftRevision::INITIAL,
                "flat_single_choice_v2".to_string(),
                record,
                "a".repeat(64),
                "b".repeat(64),
            )
            .is_err()
        );
    }

    #[test]
    fn upsert_command_debug_redacts_source_hashes() {
        let source = ObjectRecord {
            id: ObjectId::from_uuid(Uuid::nil()),
            bucket: objects::Bucket::PrivateContent,
            key: ObjectKey::WorkspaceQuestionSource {
                tenant: tenant(),
                workspace: workspace(),
                object: ObjectId::from_uuid(Uuid::nil()),
            },
            sha256: objects::Sha256Digest::from_bytes([0u8; 32]),
            size_bytes: 1,
            media_type: super::FLAT_QUESTION_MEDIA_TYPE.to_string(),
            category: ObjectCategory::Source,
            version: None,
            license: "cc-by".to_string(),
            provenance: "local-tests".to_string(),
            created_at: question_model::ActivityTimestamp::from_unix_millis(0),
        };
        let source_sha256 = source.sha256.to_string();
        let draft = DraftRecord {
            tenant: tenant(),
            question: question(),
            derived_from: None,
        };
        let command = crate::UpsertFlatQuestionCommand {
            expected_revision: None,
            draft,
            source,
            canonical_source_sha256: source_sha256.clone(),
            public_binding_sha256: source_sha256,
            grading: FlatQuestionGradingPayload::from_private(&private())
                .expect("compiled private fixture should persist"),
        };
        assert!(format!("{:?}", command).contains("[redacted]"));
    }
}

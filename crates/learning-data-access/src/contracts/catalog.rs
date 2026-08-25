use super::*;
use chacha20poly1305::{
    XChaCha20Poly1305, XNonce,
    aead::{AeadInOut, KeyInit},
};

const CATALOG_CURSOR_AEAD_KEY_DOMAIN: &[u8] = b"peptidyle/catalog-search-cursor/aead-key/v1";
const CATALOG_CURSOR_NONCE_KEY_DOMAIN: &[u8] = b"peptidyle/catalog-search-cursor/nonce-key/v1";
const CATALOG_CURSOR_ASSOCIATED_DATA_DOMAIN: &[u8] =
    b"peptidyle/catalog-search-cursor/associated-data/v1\0";
const CATALOG_CURSOR_WIRE_VERSION: u8 = 3;
const CATALOG_CURSOR_RANKING_VERSION: u8 = 4;
const CATALOG_CURSOR_PLAINTEXT_LENGTH: usize = 106;
const CATALOG_CURSOR_NONCE_LENGTH: usize = 24;
const CATALOG_CURSOR_TAG_LENGTH: usize = 16;
const CATALOG_CURSOR_WIRE_LENGTH: usize =
    1 + CATALOG_CURSOR_NONCE_LENGTH + CATALOG_CURSOR_PLAINTEXT_LENGTH + CATALOG_CURSOR_TAG_LENGTH;
const CATALOG_CURSOR_ENCODED_LENGTH: usize = (CATALOG_CURSOR_WIRE_LENGTH * 4).div_ceil(3);
#[cfg(feature = "postgres")]
const SEALED_CURSOR_NONCE_LENGTH: usize = 24;
#[cfg(feature = "postgres")]
const SEALED_CURSOR_TAG_LENGTH: usize = 16;

/// Encodes a small server-owned keyset value with a domain-separated AEAD
/// capability.  Callers bind tenant, session, list kind, and aggregate revision
/// in `associated_data`; the browser never receives those values or the keyset.
#[cfg(feature = "postgres")]
pub(crate) fn encode_sealed_cursor_u32(
    codec: &CatalogCursorCodec,
    domain: &[u8],
    associated_data: &[u8],
    value: u32,
) -> Result<String, StoreError> {
    let keys = codec.keys()?;
    let key = derive_cursor_key(&keys.aead, domain);
    let nonce_key = derive_cursor_key(&keys.nonce, domain);
    let plaintext = value.to_be_bytes();
    let mut mac = hmac::Hmac::<sha2::Sha256>::new_from_slice(&nonce_key)
        .expect("HMAC-SHA256 accepts a 32-byte key");
    mac.update(associated_data);
    mac.update(&plaintext);
    let digest = mac.finalize().into_bytes();
    let nonce_bytes: [u8; SEALED_CURSOR_NONCE_LENGTH] = digest[..SEALED_CURSOR_NONCE_LENGTH]
        .try_into()
        .expect("fixed digest prefix");
    let cipher = XChaCha20Poly1305::new(&chacha20poly1305::Key::from(key));
    let nonce = XNonce::from(nonce_bytes);
    let mut ciphertext = plaintext;
    let tag = cipher
        .encrypt_inout_detached(&nonce, associated_data, (&mut ciphertext[..]).into())
        .map_err(|_| StoreError::Unavailable("sealed cursor encryption failed".into()))?;
    let mut wire =
        Vec::with_capacity(1 + SEALED_CURSOR_NONCE_LENGTH + 4 + SEALED_CURSOR_TAG_LENGTH);
    wire.push(1);
    wire.extend_from_slice(&nonce_bytes);
    wire.extend_from_slice(&ciphertext);
    wire.extend_from_slice(&tag);
    Ok(base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(wire))
}

/// Decodes the exact canonical sealed cursor emitted by [`encode_sealed_cursor_u32`].
#[cfg(feature = "postgres")]
pub(crate) fn decode_sealed_cursor_u32(
    codec: &CatalogCursorCodec,
    domain: &[u8],
    associated_data: &[u8],
    cursor: &str,
) -> Result<u32, StoreError> {
    let bytes = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode(cursor)
        .map_err(|_| StoreError::InvalidRecord("sealed cursor is malformed".into()))?;
    if bytes.len() != 1 + SEALED_CURSOR_NONCE_LENGTH + 4 + SEALED_CURSOR_TAG_LENGTH || bytes[0] != 1
    {
        return Err(StoreError::InvalidRecord(
            "sealed cursor is malformed".into(),
        ));
    }
    let keys = codec.keys()?;
    let key = derive_cursor_key(&keys.aead, domain);
    let nonce_key = derive_cursor_key(&keys.nonce, domain);
    let nonce_bytes: [u8; SEALED_CURSOR_NONCE_LENGTH] = bytes[1..1 + SEALED_CURSOR_NONCE_LENGTH]
        .try_into()
        .expect("fixed wire");
    let mut plaintext: [u8; 4] = bytes
        [1 + SEALED_CURSOR_NONCE_LENGTH..1 + SEALED_CURSOR_NONCE_LENGTH + 4]
        .try_into()
        .expect("fixed wire");
    let tag: [u8; SEALED_CURSOR_TAG_LENGTH] = bytes[1 + SEALED_CURSOR_NONCE_LENGTH + 4..]
        .try_into()
        .expect("fixed wire");
    let cipher = XChaCha20Poly1305::new(&chacha20poly1305::Key::from(key));
    cipher
        .decrypt_inout_detached(
            &XNonce::from(nonce_bytes),
            associated_data,
            (&mut plaintext[..]).into(),
            &chacha20poly1305::Tag::from(tag),
        )
        .map_err(|_| StoreError::InvalidRecord("sealed cursor is malformed".into()))?;
    let value = u32::from_be_bytes(plaintext);
    if value == 0 || encode_sealed_cursor_u32(codec, domain, associated_data, value)? != cursor {
        return Err(StoreError::InvalidRecord(
            "sealed cursor is malformed".into(),
        ));
    }
    let mut mac = hmac::Hmac::<sha2::Sha256>::new_from_slice(&nonce_key)
        .expect("HMAC-SHA256 accepts a 32-byte key");
    mac.update(associated_data);
    mac.update(&plaintext);
    if mac.finalize().into_bytes()[..SEALED_CURSOR_NONCE_LENGTH] != nonce_bytes {
        return Err(StoreError::InvalidRecord(
            "sealed cursor is malformed".into(),
        ));
    }
    Ok(value)
}

#[derive(Clone)]
struct CatalogCursorKeys {
    aead: [u8; 32],
    nonce: [u8; 32],
}

/// Server-held authenticator for opaque catalog continuations.
///
/// Separate domain-derived AEAD and deterministic-nonce keys preserve
/// stateless continuation while keeping internal cursor values out of the
/// browser-visible wire representation.
#[derive(Clone)]
pub(crate) struct CatalogCursorCodec(Option<CatalogCursorKeys>);

impl CatalogCursorCodec {
    pub(crate) fn from_server_secret(secret: [u8; 32]) -> Self {
        Self(Some(CatalogCursorKeys {
            aead: derive_cursor_key(&secret, CATALOG_CURSOR_AEAD_KEY_DOMAIN),
            nonce: derive_cursor_key(&secret, CATALOG_CURSOR_NONCE_KEY_DOMAIN),
        }))
    }

    #[cfg(feature = "postgres")]
    pub(crate) fn unavailable() -> Self {
        Self(None)
    }

    fn keys(&self) -> Result<&CatalogCursorKeys, StoreError> {
        self.0.as_ref().ok_or_else(|| {
            StoreError::Unavailable("catalog cursor secret is unavailable".to_string())
        })
    }

    fn nonce(&self, plaintext: &[u8], associated_data: &[u8]) -> Result<[u8; 24], StoreError> {
        let keys = self.keys()?;
        // ASVS 11.3.4, 11.4.1: derive one 192-bit nonce per exact plaintext
        // and query binding from the separately derived HMAC-SHA-256 key.
        let mut mac = hmac::Hmac::<sha2::Sha256>::new_from_slice(&keys.nonce)
            .expect("HMAC-SHA256 accepts a 32-byte derived nonce key");
        mac.update(associated_data);
        mac.update(plaintext);
        let digest = mac.finalize().into_bytes();
        let mut nonce = [0_u8; CATALOG_CURSOR_NONCE_LENGTH];
        nonce.copy_from_slice(&digest[..CATALOG_CURSOR_NONCE_LENGTH]);
        Ok(nonce)
    }
}

fn derive_cursor_key(server_secret: &[u8; 32], domain: &[u8]) -> [u8; 32] {
    let mut mac = hmac::Hmac::<sha2::Sha256>::new_from_slice(server_secret)
        .expect("HMAC-SHA256 accepts a 32-byte server secret");
    mac.update(domain);
    mac.finalize().into_bytes().into()
}

impl std::fmt::Debug for CatalogCursorCodec {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("CatalogCursorCodec([redacted])")
    }
}

/// Complete deterministic key for one catalog-search continuation.
///
/// The key evolves with the ranking contract while keeping cursor encoding,
/// decoding, and every backend's keyset predicate synchronized.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct CatalogSearchCursorKey {
    pub(crate) snapshot_boundary: u64,
    pub(crate) full_text_rank: i64,
    pub(crate) similarity: i64,
    pub(crate) quality: i64,
    /// Opaque actor-usage snapshot identity, encoded as its 32-byte digest.
    /// It pins actor-sensitive usage facets and course rows across pages.
    pub(crate) actor_usage_snapshot: [u8; 32],
    /// Authenticated database-authoritative snapshot expiry in epoch milliseconds.
    pub(crate) actor_usage_snapshot_expires_at_millis: u64,
    pub(crate) problem: Uuid,
    pub(crate) version: Uuid,
}

/// Encodes a catalog keyset continuation without exposing UUID text in URLs.
/// The fixed-size authenticated-encryption wire binds the continuation to the
/// normalized-query fingerprint without exposing either it or the keyset.
// ASVS 9.1.1, 9.2.3, 11.2.1, 11.3.3: authenticate, query-bind, and encrypt.
pub(crate) fn encode_catalog_search_cursor(
    codec: &CatalogCursorCodec,
    fingerprint: &str,
    key: CatalogSearchCursorKey,
) -> Result<String, StoreError> {
    debug_assert_eq!(fingerprint.len(), 64);
    let plaintext = catalog_cursor_plaintext(key);
    let associated_data = catalog_cursor_associated_data(fingerprint);
    let keys = codec.keys()?;
    let nonce = codec.nonce(&plaintext, &associated_data)?;
    let aead_key = chacha20poly1305::Key::from(keys.aead);
    let cipher = XChaCha20Poly1305::new(&aead_key);
    let mut ciphertext = plaintext;
    let nonce = XNonce::from(nonce);
    let tag = cipher
        .encrypt_inout_detached(&nonce, &associated_data, (&mut ciphertext[..]).into())
        .map_err(|_| StoreError::Unavailable("catalog cursor encryption failed".to_string()))?;
    let mut wire = Vec::with_capacity(CATALOG_CURSOR_WIRE_LENGTH);
    wire.push(CATALOG_CURSOR_WIRE_VERSION);
    wire.extend_from_slice(&nonce);
    wire.extend_from_slice(&ciphertext);
    wire.extend_from_slice(&tag);
    Ok(base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(wire))
}

/// Decodes a canonical bounded catalog continuation and rejects a different
/// normalized query before a storage key can be used.
pub(crate) fn decode_catalog_search_cursor(
    codec: &CatalogCursorCodec,
    cursor: &str,
    fingerprint: &str,
) -> Result<CatalogSearchCursorKey, StoreError> {
    if cursor.len() != CATALOG_CURSOR_ENCODED_LENGTH {
        return Err(StoreError::InvalidRecord(
            "catalog cursor is malformed".to_string(),
        ));
    }
    let bytes = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode(cursor)
        .map_err(|_| StoreError::InvalidRecord("catalog cursor is malformed".to_string()))?;
    if bytes.len() != CATALOG_CURSOR_WIRE_LENGTH || bytes[0] != CATALOG_CURSOR_WIRE_VERSION {
        return Err(StoreError::InvalidRecord(
            "catalog cursor is malformed".to_string(),
        ));
    }
    let associated_data = catalog_cursor_associated_data(fingerprint);
    let nonce = &bytes[1..1 + CATALOG_CURSOR_NONCE_LENGTH];
    let ciphertext = &bytes
        [1 + CATALOG_CURSOR_NONCE_LENGTH..CATALOG_CURSOR_WIRE_LENGTH - CATALOG_CURSOR_TAG_LENGTH];
    let tag = &bytes[CATALOG_CURSOR_WIRE_LENGTH - CATALOG_CURSOR_TAG_LENGTH..];
    let aead_key = chacha20poly1305::Key::from(codec.keys()?.aead);
    let cipher = XChaCha20Poly1305::new(&aead_key);
    let mut plaintext: [u8; CATALOG_CURSOR_PLAINTEXT_LENGTH] = ciphertext
        .try_into()
        .expect("fixed-size wire carries fixed-size ciphertext");
    let nonce_bytes: [u8; CATALOG_CURSOR_NONCE_LENGTH] = nonce
        .try_into()
        .expect("fixed-size wire carries a 192-bit nonce");
    let nonce = XNonce::from(nonce_bytes);
    let tag: [u8; CATALOG_CURSOR_TAG_LENGTH] = tag
        .try_into()
        .expect("fixed-size wire carries a 128-bit tag");
    let tag = chacha20poly1305::Tag::from(tag);
    cipher
        .decrypt_inout_detached(&nonce, &associated_data, (&mut plaintext[..]).into(), &tag)
        .map_err(|_| StoreError::InvalidRecord("catalog cursor is malformed".to_string()))?;
    let expected_nonce = codec.nonce(&plaintext, &associated_data)?;
    if nonce.as_slice() != expected_nonce || plaintext[..2] != [2, CATALOG_CURSOR_RANKING_VERSION] {
        return Err(StoreError::InvalidRecord(
            "catalog cursor does not belong to this normalized query".to_string(),
        ));
    }
    let key = catalog_cursor_key_from_plaintext(&plaintext)?;
    if encode_catalog_search_cursor(codec, fingerprint, key)? != cursor {
        return Err(StoreError::InvalidRecord(
            "catalog cursor is malformed".to_string(),
        ));
    }
    Ok(key)
}

fn catalog_cursor_plaintext(key: CatalogSearchCursorKey) -> [u8; CATALOG_CURSOR_PLAINTEXT_LENGTH] {
    let mut bytes = [0_u8; CATALOG_CURSOR_PLAINTEXT_LENGTH];
    bytes[..2].copy_from_slice(&[2, CATALOG_CURSOR_RANKING_VERSION]);
    bytes[2..10].copy_from_slice(&key.snapshot_boundary.to_be_bytes());
    bytes[10..18].copy_from_slice(&key.full_text_rank.to_be_bytes());
    bytes[18..26].copy_from_slice(&key.similarity.to_be_bytes());
    bytes[26..34].copy_from_slice(&key.quality.to_be_bytes());
    bytes[34..66].copy_from_slice(&key.actor_usage_snapshot);
    bytes[66..74].copy_from_slice(&key.actor_usage_snapshot_expires_at_millis.to_be_bytes());
    bytes[74..90].copy_from_slice(key.problem.as_bytes());
    bytes[90..106].copy_from_slice(key.version.as_bytes());
    bytes
}

fn catalog_cursor_associated_data(fingerprint: &str) -> Vec<u8> {
    let mut associated_data =
        Vec::with_capacity(CATALOG_CURSOR_ASSOCIATED_DATA_DOMAIN.len() + fingerprint.len());
    associated_data.extend_from_slice(CATALOG_CURSOR_ASSOCIATED_DATA_DOMAIN);
    associated_data.extend_from_slice(fingerprint.as_bytes());
    associated_data
}

fn catalog_cursor_key_from_plaintext(
    plaintext: &[u8; CATALOG_CURSOR_PLAINTEXT_LENGTH],
) -> Result<CatalogSearchCursorKey, StoreError> {
    let problem = Uuid::from_slice(&plaintext[74..90])
        .map_err(|_| StoreError::InvalidRecord("catalog cursor is malformed".to_string()))?;
    let version = Uuid::from_slice(&plaintext[90..106])
        .map_err(|_| StoreError::InvalidRecord("catalog cursor is malformed".to_string()))?;
    Ok(CatalogSearchCursorKey {
        snapshot_boundary: u64::from_be_bytes(plaintext[2..10].try_into().expect("cursor size")),
        full_text_rank: i64::from_be_bytes(plaintext[10..18].try_into().expect("cursor size")),
        similarity: i64::from_be_bytes(plaintext[18..26].try_into().expect("cursor size")),
        quality: i64::from_be_bytes(plaintext[26..34].try_into().expect("cursor size")),
        actor_usage_snapshot: plaintext[34..66].try_into().expect("cursor size"),
        actor_usage_snapshot_expires_at_millis: u64::from_be_bytes(
            plaintext[66..74].try_into().expect("cursor size"),
        ),
        problem,
        version,
    })
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

/// Shared exact immutable-publication reference used for trusted delivery,
/// replay, grading, audit, and optional non-operative provenance.
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
    /// Optional immutable source evidence for an attributed derivative.
    ///
    /// This relation records provenance only. Publishing this draft creates a
    /// distinct Question ID and fresh exact evidence; it does not select,
    /// replace, or advance the source publication.
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
    /// Opaque problem evidence for trusted storage, replay, grading, and audit.
    pub problem: ProblemId,
    /// One stable, non-sequential human-facing identity for this question.
    pub question_id: question_model::QuestionId,
    /// Exact immutable version.
    pub version: VersionId,
    /// Browser-safe definition whose IDs match this record.
    pub question: QuestionDefinition,
    /// Capabilities declared by the owning adapter at publication time.
    pub capabilities: BackendCapabilities,
    /// Institution-only or cross-tenant catalog visibility.
    pub scope: PublicationScope,
    /// Discoverability and new-assignment state.
    pub lifecycle: CatalogLifecycle,
    /// Ordered private account authority for lifecycle checks; never a catalog projection.
    pub author_ids: Vec<UserId>,
    /// Immutable reviewed browser-safe attribution snapshot.
    pub byline: question_model::PublicByline,
    /// Optional immutable source evidence for a derived publication.
    ///
    /// Provenance does not define a successor, current version, redirect, or
    /// authority over another publication.
    pub derived_from: Option<ProblemVersionRef>,
    /// Backend-authoritative time at which this version became immutable.
    pub published_at: ActivityTimestamp,
}

impl PublishedProblemRecord {
    /// Builds the hot browse projection without loading another representation.
    pub fn summary(&self) -> CatalogProblemSummary {
        CatalogProblemSummary {
            question_id: self.question_id.clone(),
            backend: QuestionBackend::from(&self.question.source),
            response_family: question_model::CatalogResponseFamily::from(&self.question.response),
            capabilities: self.capabilities.clone(),
            metadata: self.question.metadata.clone(),
            byline: self.byline.clone(),
            scope: self.scope,
            lifecycle: self.lifecycle.clone(),
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
    /// Explicit reviewed immutable publication attribution. No account default exists.
    pub byline: question_model::PublicByline,
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

#[cfg(test)]
mod tests {
    use super::*;

    const FINGERPRINT: &str = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";
    const OTHER_FINGERPRINT: &str =
        "fedcba9876543210fedcba9876543210fedcba9876543210fedcba9876543210";

    fn key() -> CatalogSearchCursorKey {
        CatalogSearchCursorKey {
            snapshot_boundary: 987_654,
            full_text_rank: -123,
            similarity: 456,
            quality: 789,
            actor_usage_snapshot: [0x7c; 32],
            actor_usage_snapshot_expires_at_millis: 1_234_567_890,
            problem: Uuid::from_u128(0x1122_3344_5566_7788_99aa_bbcc_ddee_ff00),
            version: Uuid::from_u128(0x0102_0304_0506_0708_090a_0b0c_0d0e_0f10),
        }
    }

    #[test]
    fn catalog_cursor_round_trips_with_a_stable_stateless_continuation() {
        let codec = CatalogCursorCodec::from_server_secret([0x42; 32]);
        let cursor = encode_catalog_search_cursor(&codec, FINGERPRINT, key()).expect("cursor");
        assert_eq!(
            decode_catalog_search_cursor(&codec, &cursor, FINGERPRINT),
            Ok(key())
        );
        assert_eq!(
            encode_catalog_search_cursor(&codec, FINGERPRINT, key()).expect("cursor"),
            cursor
        );
    }

    #[test]
    fn catalog_cursor_rejects_tampering_and_a_different_query() {
        let codec = CatalogCursorCodec::from_server_secret([0x42; 32]);
        let cursor = encode_catalog_search_cursor(&codec, FINGERPRINT, key()).expect("cursor");
        let mut wire = base64::engine::general_purpose::URL_SAFE_NO_PAD
            .decode(&cursor)
            .expect("cursor decodes");
        wire[CATALOG_CURSOR_NONCE_LENGTH + 3] ^= 1;
        let tampered = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(wire);
        assert!(decode_catalog_search_cursor(&codec, &tampered, FINGERPRINT).is_err());
        assert!(decode_catalog_search_cursor(&codec, &cursor, OTHER_FINGERPRINT).is_err());
    }

    #[test]
    fn catalog_cursor_wire_keeps_fingerprints_and_internal_uuids_confidential() {
        let codec = CatalogCursorCodec::from_server_secret([0x42; 32]);
        let cursor = encode_catalog_search_cursor(&codec, FINGERPRINT, key()).expect("cursor");
        let wire = base64::engine::general_purpose::URL_SAFE_NO_PAD
            .decode(cursor)
            .expect("cursor wire decodes");
        assert_eq!(wire.len(), CATALOG_CURSOR_WIRE_LENGTH);
        for secret in [
            FINGERPRINT.as_bytes(),
            key().problem.as_bytes(),
            key().version.as_bytes(),
        ] {
            assert!(!wire.windows(secret.len()).any(|window| window == secret));
        }
    }

    #[test]
    fn catalog_cursor_fails_closed_without_its_server_secret() {
        let configured = CatalogCursorCodec::from_server_secret([0x42; 32]);
        let cursor = encode_catalog_search_cursor(&configured, FINGERPRINT, key()).expect("cursor");
        let unavailable = CatalogCursorCodec(None);
        assert!(matches!(
            decode_catalog_search_cursor(&unavailable, &cursor, FINGERPRINT),
            Err(StoreError::Unavailable(message)) if message == "catalog cursor secret is unavailable"
        ));
        assert!(matches!(
            encode_catalog_search_cursor(&unavailable, FINGERPRINT, key()),
            Err(StoreError::Unavailable(message)) if message == "catalog cursor secret is unavailable"
        ));
    }
}

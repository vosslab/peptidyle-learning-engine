//! Object-store contract and backends (WP-C4, MOD-OBJ).
//!
//! Callers provide [`ObjectKey`] values built from typed IDs, never physical
//! key strings. Implementations compute SHA-256 on write and verify it on read.
//! No AWS SDK type appears in this contract.

use async_trait::async_trait;
use question_model::{ActivityTimestamp, ObjectId, VersionId};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// Typed bucket and immutable key construction.
pub mod bucket;
/// In-memory backend used by tests and the M1 conformance suite.
pub mod memory;
/// MinIO client wiring used by the development containers.
pub mod minio;
/// AWS S3 backend implemented in M2.
pub mod s3;

pub use crate::bucket::{Bucket, ObjectKey};

/// Semantic role of an object.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ObjectCategory {
    /// Original imported or authored source.
    Source,
    /// Image, audio, or other referenced content asset.
    Asset,
    /// Regenerable rendered output.
    Render,
    /// Student-specific exported artifact.
    Export,
    /// Short-lived processing data.
    Temporary,
}

/// SHA-256 bytes recorded with every object.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Sha256Digest([u8; 32]);

impl Sha256Digest {
    /// Computes the digest for object bytes.
    pub fn compute(bytes: &[u8]) -> Self {
        Self(Sha256::digest(bytes).into())
    }

    /// Returns the fixed 32-byte digest.
    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

impl std::fmt::Display for Sha256Digest {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for byte in self.0 {
            write!(formatter, "{byte:02x}")?;
        }
        Ok(())
    }
}

/// Immutable database metadata corresponding to stored bytes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ObjectRecord {
    /// Durable object identity.
    pub id: ObjectId,
    /// Physical bucket selected by the semantic key.
    pub bucket: Bucket,
    /// Semantic key from which the physical path is derived.
    pub key: ObjectKey,
    /// Checksum computed from the stored bytes.
    pub sha256: Sha256Digest,
    /// Stored byte count.
    pub size_bytes: u64,
    /// Media type verified by the owning import or render path.
    pub media_type: String,
    /// Semantic storage role.
    pub category: ObjectCategory,
    /// Published version associated with content, when one exists.
    pub version: Option<VersionId>,
    /// License or educational-record handling label.
    pub license: String,
    /// Human-readable source or derivation record.
    pub provenance: String,
    /// Server-supplied creation timestamp.
    pub created_at: ActivityTimestamp,
}

/// Bytes and metadata supplied to `put`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PutObject {
    /// Semantic destination built from stable IDs.
    pub key: ObjectKey,
    /// Bytes stored before the authoritative database record is written.
    pub bytes: Vec<u8>,
    /// Verified media type.
    pub media_type: String,
    /// License or educational-record handling label.
    pub license: String,
    /// Human-readable source or derivation record.
    pub provenance: String,
    /// Server-supplied creation timestamp.
    pub created_at: ActivityTimestamp,
}

/// Stored bytes returned only after checksum verification.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoredObject {
    /// Authoritative object metadata.
    pub record: ObjectRecord,
    /// Verified object bytes.
    pub bytes: Vec<u8>,
}

/// Short-lived authorized URL.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SignedUrl {
    /// Backend URL. Callers must treat it as opaque.
    pub url: String,
    /// Server-supplied expiration time.
    pub expires_at: ActivityTimestamp,
}

/// Portable object-store failure with no AWS type in its variants.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ObjectStoreError {
    /// The requested semantic key has no record.
    NotFound,
    /// Immutable key already exists.
    AlreadyExists,
    /// Stored bytes no longer match their authoritative checksum.
    ChecksumMismatch,
    /// The bucket is never eligible for signed delivery.
    NotSignable,
    /// A size or expiration calculation overflowed.
    NumericOverflow,
    /// Backend state is temporarily unavailable.
    Unavailable(String),
}

impl std::fmt::Display for ObjectStoreError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotFound => write!(formatter, "object not found"),
            Self::AlreadyExists => write!(formatter, "immutable object already exists"),
            Self::ChecksumMismatch => write!(formatter, "stored object checksum mismatch"),
            Self::NotSignable => write!(formatter, "object bucket is not signable"),
            Self::NumericOverflow => write!(formatter, "object metadata calculation overflow"),
            Self::Unavailable(message) => write!(formatter, "object store unavailable: {message}"),
        }
    }
}

impl std::error::Error for ObjectStoreError {}

/// Backend-neutral object storage.
#[async_trait]
pub trait ObjectStore: Send + Sync {
    /// Writes immutable bytes and returns their computed metadata.
    async fn put(&self, request: PutObject) -> Result<ObjectRecord, ObjectStoreError>;

    /// Reads bytes and refuses them when their checksum does not verify.
    async fn get(&self, key: &ObjectKey) -> Result<StoredObject, ObjectStoreError>;

    /// Deletes one exact semantic key.
    async fn delete(&self, key: &ObjectKey) -> Result<(), ObjectStoreError>;

    /// Produces the bucket-policy lifetime from a server-supplied current time.
    async fn signed_url(
        &self,
        key: &ObjectKey,
        now: ActivityTimestamp,
    ) -> Result<SignedUrl, ObjectStoreError>;
}

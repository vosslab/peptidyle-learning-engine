//! Object-store contract and backends.
//!
//! Callers provide [`ObjectAddress`] values built from typed IDs, never physical
//! path strings. Implementations compute SHA-256 on write and verify it on read.
//! No AWS SDK type appears in this contract.

use async_trait::async_trait;
use question_model::{ObjectId, PublishedQuestionRevisionTuple, Timestamp};
use serde::{Deserialize, Deserializer, Serialize, Serializer, de};
use sha2::{Digest, Sha256};

/// Typed Object Storage Area and immutable Object Address construction.
pub mod bucket;
/// Shared hostile-input validation for still instructional raster images.
pub mod image_validation;
/// In-memory backend used by tests and the shared conformance suite.
pub mod memory;
/// MinIO client wiring used by the development containers.
pub mod minio;
/// Trusted immutable Question Source object resolution.
pub mod question_source;
/// S3-compatible object-store backend.
pub mod s3;

pub use crate::bucket::{
    ObjectAddress, ObjectDataClass, ObjectStorageArea, course_banner_rendition_object_id,
    course_banner_source_object_id, course_banner_upload_object_id,
    published_import_archive_object_id, workspace_qti_archive_object_id,
};
pub use crate::question_source::{QuestionSourceResolutionError, ResolvedQuestionSource};

/// SHA-256 bytes recorded with every object.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Sha256Checksum([u8; 32]);

impl Sha256Checksum {
    /// Computes the checksum for object bytes.
    pub fn compute(bytes: &[u8]) -> Self {
        Self(Sha256::digest(bytes).into())
    }

    /// Rebuilds a checksum from its already-verified fixed-width representation.
    ///
    /// Storage adapters use this when decoding a `bytea` checksum; hashing the
    /// checksum bytes again would silently change the authenticated value.
    pub fn from_bytes(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }

    /// Returns the fixed 32-byte checksum.
    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

impl std::fmt::Display for Sha256Checksum {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for byte in self.0 {
            write!(formatter, "{byte:02x}")?;
        }
        Ok(())
    }
}

impl Serialize for Sha256Checksum {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.collect_str(self)
    }
}

impl<'de> Deserialize<'de> for Sha256Checksum {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct Sha256ChecksumVisitor;

        impl de::Visitor<'_> for Sha256ChecksumVisitor {
            type Value = Sha256Checksum;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("a lowercase 64-character hexadecimal SHA-256 checksum")
            }

            fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                let encoded = value.as_bytes();
                if encoded.len() != 64
                    || !encoded
                        .iter()
                        .all(|byte| byte.is_ascii_digit() || matches!(byte, b'a'..=b'f'))
                {
                    return Err(E::invalid_value(de::Unexpected::Str(value), &self));
                }

                let mut bytes = [0_u8; 32];
                for (decoded, pair) in bytes.iter_mut().zip(encoded.as_chunks::<2>().0) {
                    *decoded = (hex_nibble(pair[0]) << 4) | hex_nibble(pair[1]);
                }
                Ok(Sha256Checksum::from_bytes(bytes))
            }
        }

        deserializer.deserialize_str(Sha256ChecksumVisitor)
    }
}

fn hex_nibble(byte: u8) -> u8 {
    match byte {
        b'0'..=b'9' => byte - b'0',
        b'a'..=b'f' => byte - b'a' + 10,
        _ => unreachable!("Sha256Checksum validates lowercase hexadecimal before decoding"),
    }
}

/// Immutable database metadata corresponding to stored bytes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ObjectRecord {
    /// Durable object identity.
    pub id: ObjectId,
    /// Object Storage Area selected by the semantic Object Address.
    pub storage_area: ObjectStorageArea,
    /// Required data class inherited from the semantic Object Address.
    pub data_class: ObjectDataClass,
    /// Semantic Object Address from which the physical path is derived.
    pub address: ObjectAddress,
    /// Checksum computed from the stored bytes.
    pub sha256: Sha256Checksum,
    /// Stored byte count.
    pub size_bytes: u64,
    /// Media type verified by the owning import or render path.
    pub media_type: String,
    /// Exact Question Revision associated with content, when one exists.
    pub published_question_revision_tuple: Option<PublishedQuestionRevisionTuple>,
    /// Server-supplied creation timestamp.
    pub created_at: Timestamp,
}

/// Bytes and metadata supplied to `put`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PutObject {
    /// Semantic destination built from stable IDs.
    pub address: ObjectAddress,
    /// Bytes stored before the authoritative database record is written.
    pub bytes: Vec<u8>,
    /// Verified media type.
    pub media_type: String,
    /// Server-supplied creation timestamp.
    pub created_at: Timestamp,
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
    pub expires_at: Timestamp,
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
    /// The Object Storage Area is never eligible for signed delivery.
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
            Self::NotSignable => write!(formatter, "object storage area is not signable"),
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
    async fn get(&self, address: &ObjectAddress) -> Result<StoredObject, ObjectStoreError>;

    /// Deletes one exact semantic Object Address.
    async fn delete(&self, address: &ObjectAddress) -> Result<(), ObjectStoreError>;

    /// Produces the Object Storage Area policy lifetime from server-supplied current time.
    async fn signed_url(
        &self,
        address: &ObjectAddress,
        now: Timestamp,
    ) -> Result<SignedUrl, ObjectStoreError>;
}

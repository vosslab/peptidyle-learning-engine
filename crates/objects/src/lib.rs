//! Object-store contract and backends.
//!
//! Callers provide [`ObjectAddress`] values built from typed IDs, never physical
//! path strings. Implementations compute SHA-256 on write and verify it on read.
//! No AWS SDK type appears in this contract.

use async_trait::async_trait;
use question_model::classification::License;
use question_model::{ActivityTimestamp, ObjectId, QuestionRevisionReference};
use serde::{Deserialize, Deserializer, Serialize, Serializer, de};
use sha2::{Digest, Sha256};

/// Production AWS client wiring using container workload identity only.
pub mod aws;
/// Typed Object Storage Area and immutable Object Address construction.
pub mod bucket;
/// Shared hostile-input validation for still instructional raster images.
pub mod image_validation;
/// In-memory backend used by tests and the shared conformance suite.
pub mod memory;
/// MinIO client wiring used by the development containers.
pub mod minio;
/// Production AWS S3 backend.
pub mod s3;

pub use crate::bucket::{
    ObjectAddress, ObjectDataClass, ObjectStorageArea, course_banner_candidate_object_id,
    course_banner_object_id, published_import_archive_object_id, workspace_qti_archive_object_id,
};

/// SHA-256 bytes recorded with every object.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Sha256Digest([u8; 32]);

impl Sha256Digest {
    /// Computes the digest for object bytes.
    pub fn compute(bytes: &[u8]) -> Self {
        Self(Sha256::digest(bytes).into())
    }

    /// Rebuilds a digest from its already-verified fixed-width representation.
    ///
    /// Storage adapters use this when decoding a `bytea` checksum; hashing the
    /// checksum bytes again would silently change the authenticated value.
    pub fn from_bytes(bytes: [u8; 32]) -> Self {
        Self(bytes)
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

impl Serialize for Sha256Digest {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.collect_str(self)
    }
}

impl<'de> Deserialize<'de> for Sha256Digest {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct Sha256DigestVisitor;

        impl de::Visitor<'_> for Sha256DigestVisitor {
            type Value = Sha256Digest;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("a lowercase 64-character hexadecimal SHA-256 digest")
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
                Ok(Sha256Digest::from_bytes(bytes))
            }
        }

        deserializer.deserialize_str(Sha256DigestVisitor)
    }
}

fn hex_nibble(byte: u8) -> u8 {
    match byte {
        b'0'..=b'9' => byte - b'0',
        b'a'..=b'f' => byte - b'a' + 10,
        _ => unreachable!("Sha256Digest validates lowercase hexadecimal before decoding"),
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
    pub sha256: Sha256Digest,
    /// Stored byte count.
    pub size_bytes: u64,
    /// Media type verified by the owning import or render path.
    pub media_type: String,
    /// Exact Question Revision associated with content, when one exists.
    pub question_revision: Option<QuestionRevisionReference>,
    /// Optional content-reuse terms. Data sensitivity belongs to `data_class`.
    pub license: Option<License>,
    /// Human-readable source or derivation record.
    pub provenance: String,
    /// Server-supplied creation timestamp.
    pub created_at: ActivityTimestamp,
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
    /// Optional content-reuse terms. Data sensitivity is derived from `address`.
    pub license: Option<License>,
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
        now: ActivityTimestamp,
    ) -> Result<SignedUrl, ObjectStoreError>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use question_model::{QuestionId, QuestionRevisionNumber, QuestionRevisionReference};
    use uuid::Uuid;

    const DIGEST_BYTES: [u8; 32] = [
        0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0a, 0x0b, 0x0c, 0x0d, 0x0e,
        0x0f, 0x10, 0x11, 0x12, 0x13, 0x14, 0x15, 0x16, 0x17, 0x18, 0x19, 0x1a, 0x1b, 0x1c, 0x1d,
        0x1e, 0x1f,
    ];
    const DIGEST_HEX: &str = "000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f";
    const LEGACY_ARRAY_JSON: &str =
        "[0,1,2,3,4,5,6,7,8,9,10,11,12,13,14,15,16,17,18,19,20,21,22,23,24,25,26,27,28,29,30,31]";

    #[test]
    fn sha256_digest_json_is_canonical_lowercase_hex_and_round_trips() {
        let digest = Sha256Digest::from_bytes(DIGEST_BYTES);
        let encoded = serde_json::to_string(&digest).expect("digest should serialize");

        assert_eq!(encoded, format!("\"{DIGEST_HEX}\""));
        assert_eq!(
            serde_json::from_str::<Sha256Digest>(&encoded).expect("digest should deserialize"),
            digest
        );
    }

    #[test]
    fn sha256_digest_json_rejects_noncanonical_values_and_arrays() {
        let uppercase = format!("\"{}\"", DIGEST_HEX.to_ascii_uppercase());
        let wrong_length = format!("\"{}\"", &DIGEST_HEX[..63]);
        let non_hex = format!("\"{}g\"", &DIGEST_HEX[..63]);

        for invalid in [
            uppercase.as_str(),
            wrong_length.as_str(),
            non_hex.as_str(),
            LEGACY_ARRAY_JSON,
        ] {
            assert!(
                serde_json::from_str::<Sha256Digest>(invalid).is_err(),
                "digest JSON should reject {invalid}"
            );
        }
    }

    #[test]
    fn object_record_json_shape_uses_canonical_hex_digest() {
        let question_revision = QuestionRevisionReference {
            question_id: QuestionId::from_canonical_parts("ABCDEF", 'G')
                .expect("canonical Question ID"),
            revision_number: QuestionRevisionNumber::new(2)
                .expect("positive Question Revision Number"),
        };
        let object = ObjectId::from_uuid(Uuid::from_u128(3));
        let record = ObjectRecord {
            id: object,
            storage_area: ObjectStorageArea::PrivateContent,
            data_class: ObjectDataClass::QuestionSource,
            address: ObjectAddress::QuestionSource {
                question_revision: question_revision.clone(),
                object,
            },
            sha256: Sha256Digest::from_bytes(DIGEST_BYTES),
            size_bytes: 123,
            media_type: "application/zip".to_string(),
            question_revision: Some(question_revision),
            license: None,
            provenance: "fixture".to_string(),
            created_at: ActivityTimestamp::from_unix_millis(1_000),
        };
        let encoded = serde_json::to_string(&record).expect("object record should serialize");

        assert_eq!(
            encoded,
            concat!(
                "{\"id\":\"00000000-0000-0000-0000-000000000003\",",
                "\"storageArea\":\"private-content\",",
                "\"dataClass\":\"question-source\",",
                "\"address\":{\"kind\":\"questionSource\",",
                "\"questionRevision\":{\"questionId\":\"ABC-DEFG\",\"revisionNumber\":2},",
                "\"object\":\"00000000-0000-0000-0000-000000000003\"},",
                "\"sha256\":\"000102030405060708090a0b0c0d0e0f",
                "101112131415161718191a1b1c1d1e1f\",",
                "\"sizeBytes\":123,",
                "\"mediaType\":\"application/zip\",",
                "\"questionRevision\":{\"questionId\":\"ABC-DEFG\",\"revisionNumber\":2},",
                "\"license\":null,",
                "\"provenance\":\"fixture\",",
                "\"createdAt\":1000}"
            )
        );
        assert_eq!(
            serde_json::from_str::<ObjectRecord>(&encoded)
                .expect("canonical object record should deserialize"),
            record
        );
    }
}

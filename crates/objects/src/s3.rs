//! AWS S3 backend (MOD-OBJ).
//!
//! The same implementation serves AWS and MinIO. It stays behind the `s3`
//! feature so the memory backend does not pull the AWS SDK. No AWS type leaks
//! through the [`crate::ObjectStore`] trait.

#[cfg(feature = "s3")]
use std::collections::HashMap;
#[cfg(feature = "s3")]
use std::time::{Duration, SystemTime};

#[cfg(feature = "s3")]
use async_trait::async_trait;
#[cfg(feature = "s3")]
use aws_sdk_s3::Client;
#[cfg(feature = "s3")]
use aws_sdk_s3::presigning::PresigningConfig;
#[cfg(feature = "s3")]
use aws_sdk_s3::primitives::ByteStream;
#[cfg(feature = "s3")]
use base64::Engine;
#[cfg(feature = "s3")]
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
#[cfg(feature = "s3")]
use question_model::ActivityTimestamp;

#[cfg(feature = "s3")]
use crate::{
    Bucket, ObjectKey, ObjectRecord, ObjectStore, ObjectStoreError, PutObject, Sha256Digest,
    SignedUrl, StoredObject,
};

#[cfg(feature = "s3")]
const RECORD_METADATA_KEY: &str = "ple-record-v1";

/// Physical names for the three policy-specific buckets.
#[cfg(feature = "s3")]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BucketNames {
    /// Shared immutable content bucket.
    pub content: String,
    /// Tenant-owned educational-record bucket.
    pub student_records: String,
    /// Never-served processing bucket.
    pub temp_processing: String,
}

#[cfg(feature = "s3")]
impl Default for BucketNames {
    fn default() -> Self {
        Self {
            content: Bucket::Content.as_str().to_string(),
            student_records: Bucket::StudentRecords.as_str().to_string(),
            temp_processing: Bucket::TempProcessing.as_str().to_string(),
        }
    }
}

#[cfg(feature = "s3")]
impl BucketNames {
    fn name(&self, bucket: Bucket) -> &str {
        match bucket {
            Bucket::Content => &self.content,
            Bucket::StudentRecords => &self.student_records,
            Bucket::TempProcessing => &self.temp_processing,
        }
    }
}

/// Replica-safe object store backed by AWS S3 or an S3-compatible endpoint.
#[cfg(feature = "s3")]
#[derive(Clone)]
pub struct S3ObjectStore {
    client: Client,
    buckets: BucketNames,
}

#[cfg(feature = "s3")]
impl S3ObjectStore {
    /// Builds a store from a crate-owned SDK client and explicit bucket names.
    pub fn new(client: Client, buckets: BucketNames) -> Self {
        Self { client, buckets }
    }

    async fn head_record(&self, key: &ObjectKey) -> Result<ObjectRecord, ObjectStoreError> {
        let bucket = self.buckets.name(key.bucket());
        let output = self
            .client
            .head_object()
            .bucket(bucket)
            .key(key.path())
            .send()
            .await
            .map_err(|error| {
                if error
                    .as_service_error()
                    .is_some_and(|service_error| service_error.is_not_found())
                    || error
                        .raw_response()
                        .is_some_and(|response| response.status().as_u16() == 404)
                {
                    ObjectStoreError::NotFound
                } else {
                    ObjectStoreError::Unavailable(error.to_string())
                }
            })?;
        decode_record(
            key,
            output.metadata(),
            output.content_length(),
            output.content_type(),
        )
    }
}

#[cfg(feature = "s3")]
#[async_trait]
impl ObjectStore for S3ObjectStore {
    async fn put(&self, request: PutObject) -> Result<ObjectRecord, ObjectStoreError> {
        let size_bytes =
            u64::try_from(request.bytes.len()).map_err(|_| ObjectStoreError::NumericOverflow)?;
        let content_length =
            i64::try_from(size_bytes).map_err(|_| ObjectStoreError::NumericOverflow)?;
        let record = ObjectRecord {
            id: request.key.object_id(),
            bucket: request.key.bucket(),
            key: request.key.clone(),
            sha256: Sha256Digest::compute(&request.bytes),
            size_bytes,
            media_type: request.media_type,
            category: request.key.category(),
            version: request.key.version_id(),
            license: request.license,
            provenance: request.provenance,
            created_at: request.created_at,
        };
        let encoded_record = encode_record(&record)?;
        let bucket = self.buckets.name(record.bucket);

        self.client
            .put_object()
            .bucket(bucket)
            .key(record.key.path())
            .body(ByteStream::from(request.bytes))
            .content_length(content_length)
            .content_type(record.media_type.clone())
            .metadata(RECORD_METADATA_KEY, encoded_record)
            .if_none_match("*")
            .send()
            .await
            .map_err(|error| {
                if error
                    .raw_response()
                    .is_some_and(|response| response.status().as_u16() == 412)
                {
                    ObjectStoreError::AlreadyExists
                } else {
                    ObjectStoreError::Unavailable(error.to_string())
                }
            })?;
        Ok(record)
    }

    async fn get(&self, key: &ObjectKey) -> Result<StoredObject, ObjectStoreError> {
        let bucket = self.buckets.name(key.bucket());
        let output = self
            .client
            .get_object()
            .bucket(bucket)
            .key(key.path())
            .send()
            .await
            .map_err(|error| {
                if error
                    .as_service_error()
                    .is_some_and(|service_error| service_error.is_no_such_key())
                    || error
                        .raw_response()
                        .is_some_and(|response| response.status().as_u16() == 404)
                {
                    ObjectStoreError::NotFound
                } else {
                    ObjectStoreError::Unavailable(error.to_string())
                }
            })?;
        let record = decode_record(
            key,
            output.metadata(),
            output.content_length(),
            output.content_type(),
        )?;
        let bytes = output
            .body
            .collect()
            .await
            .map_err(|error| ObjectStoreError::Unavailable(error.to_string()))?
            .into_bytes()
            .to_vec();
        verify_bytes(record, bytes)
    }

    async fn delete(&self, key: &ObjectKey) -> Result<(), ObjectStoreError> {
        self.head_record(key).await?;
        self.client
            .delete_object()
            .bucket(self.buckets.name(key.bucket()))
            .key(key.path())
            .send()
            .await
            .map_err(|error| ObjectStoreError::Unavailable(error.to_string()))?;
        Ok(())
    }

    async fn signed_url(
        &self,
        key: &ObjectKey,
        now: ActivityTimestamp,
    ) -> Result<SignedUrl, ObjectStoreError> {
        if !key.may_issue_signed_url() {
            return Err(ObjectStoreError::NotSignable);
        }
        self.head_record(key).await?;
        let lifetime = signed_url_lifetime(key.bucket())?;
        let lifetime_millis =
            i64::try_from(lifetime.as_millis()).map_err(|_| ObjectStoreError::NumericOverflow)?;
        let expires_millis = now
            .as_unix_millis()
            .checked_add(lifetime_millis)
            .ok_or(ObjectStoreError::NumericOverflow)?;
        let config = PresigningConfig::builder()
            .start_time(system_time(now)?)
            .expires_in(lifetime)
            .build()
            .map_err(|error| ObjectStoreError::Unavailable(error.to_string()))?;
        let request = self
            .client
            .get_object()
            .bucket(self.buckets.name(key.bucket()))
            .key(key.path())
            .presigned(config)
            .await
            .map_err(|error| ObjectStoreError::Unavailable(error.to_string()))?;
        Ok(SignedUrl {
            url: request.uri().to_string(),
            expires_at: ActivityTimestamp::from_unix_millis(expires_millis),
        })
    }
}

#[cfg(feature = "s3")]
fn encode_record(record: &ObjectRecord) -> Result<String, ObjectStoreError> {
    let json = serde_json::to_vec(record)
        .map_err(|error| ObjectStoreError::Unavailable(error.to_string()))?;
    Ok(URL_SAFE_NO_PAD.encode(json))
}

#[cfg(feature = "s3")]
fn decode_record(
    key: &ObjectKey,
    metadata: Option<&HashMap<String, String>>,
    content_length: Option<i64>,
    content_type: Option<&str>,
) -> Result<ObjectRecord, ObjectStoreError> {
    let encoded = metadata
        .and_then(|values| values.get(RECORD_METADATA_KEY))
        .ok_or_else(|| unavailable_metadata("missing object record"))?;
    let json = URL_SAFE_NO_PAD
        .decode(encoded)
        .map_err(|error| unavailable_metadata(&error.to_string()))?;
    let record: ObjectRecord =
        serde_json::from_slice(&json).map_err(|error| unavailable_metadata(&error.to_string()))?;
    if record.key != *key
        || record.id != key.object_id()
        || record.bucket != key.bucket()
        || record.category != key.category()
        || record.version != key.version_id()
    {
        return Err(unavailable_metadata("semantic key does not match record"));
    }
    let response_size = content_length
        .ok_or_else(|| unavailable_metadata("missing content length"))
        .and_then(|value| {
            u64::try_from(value).map_err(|_| unavailable_metadata("negative content length"))
        })?;
    if response_size != record.size_bytes {
        return Err(unavailable_metadata("content length does not match record"));
    }
    if content_type != Some(record.media_type.as_str()) {
        return Err(unavailable_metadata("content type does not match record"));
    }
    Ok(record)
}

#[cfg(feature = "s3")]
fn verify_bytes(record: ObjectRecord, bytes: Vec<u8>) -> Result<StoredObject, ObjectStoreError> {
    let size_bytes = u64::try_from(bytes.len()).map_err(|_| ObjectStoreError::NumericOverflow)?;
    if size_bytes != record.size_bytes || Sha256Digest::compute(&bytes) != record.sha256 {
        return Err(ObjectStoreError::ChecksumMismatch);
    }
    Ok(StoredObject { record, bytes })
}

#[cfg(feature = "s3")]
fn signed_url_lifetime(bucket: Bucket) -> Result<Duration, ObjectStoreError> {
    match bucket {
        Bucket::Content => Ok(Duration::from_secs(60 * 60)),
        Bucket::StudentRecords => Ok(Duration::from_secs(5 * 60)),
        Bucket::TempProcessing => Err(ObjectStoreError::NotSignable),
    }
}

#[cfg(feature = "s3")]
fn system_time(timestamp: ActivityTimestamp) -> Result<SystemTime, ObjectStoreError> {
    let millis = timestamp.as_unix_millis();
    let offset = Duration::from_millis(millis.unsigned_abs());
    if millis.is_negative() {
        SystemTime::UNIX_EPOCH
            .checked_sub(offset)
            .ok_or(ObjectStoreError::NumericOverflow)
    } else {
        SystemTime::UNIX_EPOCH
            .checked_add(offset)
            .ok_or(ObjectStoreError::NumericOverflow)
    }
}

#[cfg(feature = "s3")]
fn unavailable_metadata(message: &str) -> ObjectStoreError {
    ObjectStoreError::Unavailable(format!("invalid object metadata: {message}"))
}

#[cfg(all(test, feature = "s3"))]
mod tests {
    use super::*;
    use question_model::{ObjectId, ProblemId, VersionId};
    use uuid::Uuid;

    fn record() -> ObjectRecord {
        let key = ObjectKey::ProblemSource {
            problem: ProblemId::from_uuid(Uuid::from_u128(1)),
            version: VersionId::from_uuid(Uuid::from_u128(2)),
            object: ObjectId::from_uuid(Uuid::from_u128(3)),
        };
        ObjectRecord {
            id: key.object_id(),
            bucket: key.bucket(),
            key,
            sha256: Sha256Digest::compute(b"source"),
            size_bytes: 6,
            media_type: "application/zip".to_string(),
            category: crate::ObjectCategory::Source,
            version: Some(VersionId::from_uuid(Uuid::from_u128(2))),
            license: "CC-BY-SA-4.0".to_string(),
            provenance: "faculty source with an accented name: Jos\u{e9}".to_string(),
            created_at: ActivityTimestamp::from_unix_millis(1_000),
        }
    }

    #[test]
    fn encoded_metadata_round_trips_the_authoritative_record() {
        let expected = record();
        let metadata = HashMap::from([(
            RECORD_METADATA_KEY.to_string(),
            encode_record(&expected).expect("record should encode"),
        )]);
        let actual = decode_record(
            &expected.key,
            Some(&metadata),
            Some(6),
            Some("application/zip"),
        )
        .expect("record should decode");

        assert_eq!(actual, expected);
    }

    #[test]
    fn metadata_for_another_semantic_key_is_rejected() {
        let stored = record();
        let requested = ObjectKey::Temporary {
            object: ObjectId::from_uuid(Uuid::from_u128(4)),
        };
        let metadata = HashMap::from([(
            RECORD_METADATA_KEY.to_string(),
            encode_record(&stored).expect("record should encode"),
        )]);

        assert!(matches!(
            decode_record(
                &requested,
                Some(&metadata),
                Some(6),
                Some("application/zip")
            ),
            Err(ObjectStoreError::Unavailable(_))
        ));
    }

    #[test]
    fn downloaded_bytes_must_match_the_record() {
        assert_eq!(
            verify_bytes(record(), b"changed".to_vec()),
            Err(ObjectStoreError::ChecksumMismatch)
        );
    }
}

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
use aws_sdk_s3::types::ServerSideEncryption;
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
/// S3 tag required on the only CDN-readable object kind.
///
/// This is deliberately a tag, rather than metadata: the production bucket
/// policy can require it at write time and use it to protect the immutable
/// publication boundary independently of the application process.
#[cfg(feature = "s3")]
const IMMUTABLE_PUBLICATION_TAG_KEY: &str = "ple-published-immutable";
#[cfg(feature = "s3")]
const IMMUTABLE_PUBLICATION_TAG_VALUE: &str = "true";
#[cfg(feature = "s3")]
const IMMUTABLE_PUBLICATION_TAG_QUERY: &str = "ple-published-immutable=true";

/// Physical names for the four policy-specific buckets.
#[cfg(feature = "s3")]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BucketNames {
    /// CDN-readable immutable student-public assets only.
    pub public_assets: String,
    /// Private authoring, provenance, render, and course-content bytes.
    pub private_content: String,
    /// Course-owned educational-record bucket.
    pub student_records: String,
    /// Never-served processing bucket.
    pub temp_processing: String,
}

/// Customer-managed KMS key ARNs for policy-separated production buckets.
#[cfg(feature = "s3")]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KmsKeyNames {
    pub public_assets: String,
    pub private_content: String,
    pub student_records: String,
    pub temp_processing: String,
}

#[cfg(feature = "s3")]
impl KmsKeyNames {
    /// Accepts only four distinct KMS key ARNs.
    pub fn new(
        public_assets: String,
        private_content: String,
        student_records: String,
        temp_processing: String,
    ) -> Result<Self, String> {
        let keys = [
            &public_assets,
            &private_content,
            &student_records,
            &temp_processing,
        ];
        if keys
            .iter()
            .any(|key| !key.starts_with("arn:aws:kms:") || key.chars().any(char::is_whitespace))
            || keys
                .iter()
                .enumerate()
                .any(|(index, key)| keys.iter().skip(index + 1).any(|other| key == other))
        {
            return Err("production object buckets require four distinct KMS key ARNs".into());
        }
        Ok(Self {
            public_assets,
            private_content,
            student_records,
            temp_processing,
        })
    }

    fn name(&self, bucket: Bucket) -> &str {
        match bucket {
            Bucket::PublicAssets => &self.public_assets,
            Bucket::PrivateContent => &self.private_content,
            Bucket::StudentRecords => &self.student_records,
            Bucket::TempProcessing => &self.temp_processing,
        }
    }
}

#[cfg(feature = "s3")]
impl Default for BucketNames {
    fn default() -> Self {
        Self {
            public_assets: Bucket::PublicAssets.as_str().to_string(),
            private_content: Bucket::PrivateContent.as_str().to_string(),
            student_records: Bucket::StudentRecords.as_str().to_string(),
            temp_processing: Bucket::TempProcessing.as_str().to_string(),
        }
    }
}

#[cfg(feature = "s3")]
impl BucketNames {
    fn name(&self, bucket: Bucket) -> &str {
        match bucket {
            Bucket::PublicAssets => &self.public_assets,
            Bucket::PrivateContent => &self.private_content,
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
    kms_keys: Option<KmsKeyNames>,
}

#[cfg(feature = "s3")]
impl S3ObjectStore {
    /// Builds a store from a crate-owned SDK client and explicit bucket names.
    pub fn new(client: Client, buckets: BucketNames) -> Self {
        Self {
            client,
            buckets,
            kms_keys: None,
        }
    }

    /// Builds a production store that requests and verifies SSE-KMS for every
    /// object using the key assigned to its policy bucket.
    pub fn new_kms_encrypted(client: Client, buckets: BucketNames, kms_keys: KmsKeyNames) -> Self {
        Self {
            client,
            buckets,
            kms_keys: Some(kms_keys),
        }
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
        let record = decode_record(
            key,
            output.metadata(),
            output.content_length(),
            output.content_type(),
        )?;
        self.verify_encryption(
            key.bucket(),
            output.server_side_encryption(),
            output.ssekms_key_id(),
        )?;
        self.verify_immutable_publication_tag(key).await?;
        Ok(record)
    }

    /// Validates the bucket-policy marker before a public asset is accepted,
    /// delivered, or presigned. `HeadObject` does not return object tags, so
    /// the marker must be read through the separate S3 tagging API.
    async fn verify_immutable_publication_tag(
        &self,
        key: &ObjectKey,
    ) -> Result<(), ObjectStoreError> {
        if !requires_immutable_publication_tag(key) {
            return Ok(());
        }
        let tags = self
            .client
            .get_object_tagging()
            .bucket(self.buckets.name(key.bucket()))
            .key(key.path())
            .send()
            .await
            .map_err(|error| {
                ObjectStoreError::Unavailable(format!(
                    "could not verify immutable-publication object tag: {error}"
                ))
            })?;
        validate_immutable_publication_tags(
            tags.tag_set().iter().map(|tag| (tag.key(), tag.value())),
        )
    }

    fn verify_encryption(
        &self,
        bucket: Bucket,
        encryption: Option<&ServerSideEncryption>,
        kms_key_id: Option<&str>,
    ) -> Result<(), ObjectStoreError> {
        let Some(keys) = &self.kms_keys else {
            return Ok(());
        };
        if encryption != Some(&ServerSideEncryption::AwsKms)
            || kms_key_id != Some(keys.name(bucket))
        {
            return Err(ObjectStoreError::Unavailable(
                "object does not satisfy the bucket encryption contract".into(),
            ));
        }
        Ok(())
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
            question_version: request.key.question_version().cloned(),
            license: request.license,
            provenance: request.provenance,
            created_at: request.created_at,
        };
        let encoded_record = encode_record(&record)?;
        let bucket = self.buckets.name(record.bucket);

        let mut put = self
            .client
            .put_object()
            .bucket(bucket)
            .key(record.key.path())
            .body(ByteStream::from(request.bytes))
            .content_length(content_length)
            .content_type(record.media_type.clone())
            .metadata(RECORD_METADATA_KEY, encoded_record)
            .if_none_match("*");
        if let Some(keys) = &self.kms_keys {
            put = put
                .server_side_encryption(ServerSideEncryption::AwsKms)
                .ssekms_key_id(keys.name(record.bucket));
        }
        if let Some(tagging) = immutable_publication_tagging(&record.key) {
            put = put.tagging(tagging);
        }
        put.send().await.map_err(|error| {
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
        self.verify_encryption(
            key.bucket(),
            output.server_side_encryption(),
            output.ssekms_key_id(),
        )?;
        self.verify_immutable_publication_tag(key).await?;
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
        || record.question_version != key.question_version().cloned()
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
        Bucket::PublicAssets | Bucket::PrivateContent => Ok(Duration::from_secs(60 * 60)),
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

/// `QuestionAsset` is the one semantic key admitted to the public-assets
/// bucket. Every other typed key must remain untagged so a private object
/// cannot accidentally acquire the public immutable-publication capability.
#[cfg(feature = "s3")]
fn requires_immutable_publication_tag(key: &ObjectKey) -> bool {
    matches!(key, ObjectKey::QuestionAsset { .. })
}

#[cfg(feature = "s3")]
fn immutable_publication_tagging(key: &ObjectKey) -> Option<&'static str> {
    requires_immutable_publication_tag(key).then_some(IMMUTABLE_PUBLICATION_TAG_QUERY)
}

/// The publication marker is an exact singleton set. Extra tags are refused:
/// accepting them would silently broaden the deployment policy vocabulary at
/// a security boundary without a reviewed contract change.
#[cfg(feature = "s3")]
fn validate_immutable_publication_tags<'a>(
    mut tags: impl Iterator<Item = (&'a str, &'a str)>,
) -> Result<(), ObjectStoreError> {
    match (tags.next(), tags.next()) {
        (Some((IMMUTABLE_PUBLICATION_TAG_KEY, IMMUTABLE_PUBLICATION_TAG_VALUE)), None) => Ok(()),
        _ => Err(ObjectStoreError::Unavailable(
            "public asset does not satisfy the immutable-publication tag contract".into(),
        )),
    }
}

#[cfg(all(test, feature = "s3"))]
mod tests {
    use super::*;
    use question_model::{
        AssetId, ObjectId, QuestionId, QuestionVersionNumber, QuestionVersionReference,
    };
    use uuid::Uuid;

    fn question_version() -> QuestionVersionReference {
        QuestionVersionReference {
            question_id: QuestionId::from_canonical_parts("ABCDEF", 'G').expect("Question ID"),
            version_number: QuestionVersionNumber::new(2).expect("positive version"),
        }
    }

    fn record() -> ObjectRecord {
        let key = ObjectKey::QuestionSource {
            question_version: question_version(),
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
            question_version: Some(question_version()),
            license: "CC-BY-SA-4.0".to_string(),
            provenance: "faculty source with an accented name: Jos\u{e9}".to_string(),
            created_at: ActivityTimestamp::from_unix_millis(1_000),
        }
    }

    fn public_asset_key() -> ObjectKey {
        ObjectKey::QuestionAsset {
            question_version: question_version(),
            asset: AssetId::from_uuid(Uuid::from_u128(3)),
            object: ObjectId::from_uuid(Uuid::from_u128(4)),
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

    #[test]
    fn only_public_problem_assets_receive_the_immutable_publication_tag() {
        let private_source = record().key;
        let public_asset = public_asset_key();

        assert_eq!(
            immutable_publication_tagging(&public_asset),
            Some(IMMUTABLE_PUBLICATION_TAG_QUERY)
        );
        assert!(requires_immutable_publication_tag(&public_asset));
        assert_eq!(immutable_publication_tagging(&private_source), None);
        assert!(!requires_immutable_publication_tag(&private_source));
    }

    #[test]
    fn public_immutable_publication_tag_is_an_exact_singleton() {
        assert!(
            validate_immutable_publication_tags(
                [(
                    IMMUTABLE_PUBLICATION_TAG_KEY,
                    IMMUTABLE_PUBLICATION_TAG_VALUE
                )]
                .into_iter()
            )
            .is_ok()
        );

        for invalid in [
            vec![],
            vec![(IMMUTABLE_PUBLICATION_TAG_KEY, "false")],
            vec![("other", IMMUTABLE_PUBLICATION_TAG_VALUE)],
            vec![
                (
                    IMMUTABLE_PUBLICATION_TAG_KEY,
                    IMMUTABLE_PUBLICATION_TAG_VALUE,
                ),
                ("review-state", "approved"),
            ],
        ] {
            assert!(validate_immutable_publication_tags(invalid.into_iter()).is_err());
        }
    }

    #[test]
    fn production_kms_keys_must_be_distinct_arns() {
        let public_assets = "arn:aws:kms:us-east-1:111122223333:key/public-assets".to_string();
        let private_content = "arn:aws:kms:us-east-1:111122223333:key/private-content".to_string();
        let records = "arn:aws:kms:us-east-1:111122223333:key/records".to_string();
        let temporary = "arn:aws:kms:us-east-1:111122223333:key/temporary".to_string();
        assert!(
            KmsKeyNames::new(
                public_assets.clone(),
                private_content.clone(),
                records.clone(),
                temporary.clone()
            )
            .is_ok()
        );
        assert!(
            KmsKeyNames::new(
                public_assets.clone(),
                public_assets,
                records.clone(),
                temporary.clone()
            )
            .is_err()
        );
        assert!(
            KmsKeyNames::new(
                "alias/public-assets".into(),
                private_content,
                records.clone(),
                temporary
            )
            .is_err()
        );
    }
}

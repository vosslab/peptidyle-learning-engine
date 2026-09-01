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
use question_model::Timestamp;

#[cfg(feature = "s3")]
use crate::{
    ObjectAddress, ObjectRecord, ObjectStorageArea, ObjectStore, ObjectStoreError, PutObject,
    Sha256Checksum, SignedUrl, StoredObject,
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
    /// Private authoring, import evidence, render, and course-content bytes.
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

    fn name(&self, storage_area: ObjectStorageArea) -> &str {
        match storage_area {
            ObjectStorageArea::PublicAssets => &self.public_assets,
            ObjectStorageArea::PrivateContent => &self.private_content,
            ObjectStorageArea::StudentRecords => &self.student_records,
            ObjectStorageArea::TempProcessing => &self.temp_processing,
        }
    }
}

#[cfg(feature = "s3")]
impl Default for BucketNames {
    fn default() -> Self {
        Self {
            public_assets: ObjectStorageArea::PublicAssets.as_str().to_string(),
            private_content: ObjectStorageArea::PrivateContent.as_str().to_string(),
            student_records: ObjectStorageArea::StudentRecords.as_str().to_string(),
            temp_processing: ObjectStorageArea::TempProcessing.as_str().to_string(),
        }
    }
}

#[cfg(feature = "s3")]
impl BucketNames {
    fn name(&self, storage_area: ObjectStorageArea) -> &str {
        match storage_area {
            ObjectStorageArea::PublicAssets => &self.public_assets,
            ObjectStorageArea::PrivateContent => &self.private_content,
            ObjectStorageArea::StudentRecords => &self.student_records,
            ObjectStorageArea::TempProcessing => &self.temp_processing,
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

    async fn head_record(&self, key: &ObjectAddress) -> Result<ObjectRecord, ObjectStoreError> {
        let bucket = self.buckets.name(key.storage_area());
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
            key.storage_area(),
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
        key: &ObjectAddress,
    ) -> Result<(), ObjectStoreError> {
        if !requires_immutable_publication_tag(key) {
            return Ok(());
        }
        let tags = self
            .client
            .get_object_tagging()
            .bucket(self.buckets.name(key.storage_area()))
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
        storage_area: ObjectStorageArea,
        encryption: Option<&ServerSideEncryption>,
        kms_key_id: Option<&str>,
    ) -> Result<(), ObjectStoreError> {
        let Some(keys) = &self.kms_keys else {
            return Ok(());
        };
        if encryption != Some(&ServerSideEncryption::AwsKms)
            || kms_key_id != Some(keys.name(storage_area))
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
            id: request.address.object_id(),
            storage_area: request.address.storage_area(),
            data_class: request.address.data_class(),
            address: request.address.clone(),
            sha256: Sha256Checksum::compute(&request.bytes),
            size_bytes,
            media_type: request.media_type,
            question_revision: request.address.question_revision().cloned(),
            created_at: request.created_at,
        };
        let encoded_record = encode_record(&record)?;
        let bucket = self.buckets.name(record.storage_area);

        let mut put = self
            .client
            .put_object()
            .bucket(bucket)
            .key(record.address.path())
            .body(ByteStream::from(request.bytes))
            .content_length(content_length)
            .content_type(record.media_type.clone())
            .metadata(RECORD_METADATA_KEY, encoded_record)
            .if_none_match("*");
        if let Some(keys) = &self.kms_keys {
            put = put
                .server_side_encryption(ServerSideEncryption::AwsKms)
                .ssekms_key_id(keys.name(record.storage_area));
        }
        if let Some(tagging) = immutable_publication_tagging(&record.address) {
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

    async fn get(&self, key: &ObjectAddress) -> Result<StoredObject, ObjectStoreError> {
        let bucket = self.buckets.name(key.storage_area());
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
            key.storage_area(),
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

    async fn delete(&self, key: &ObjectAddress) -> Result<(), ObjectStoreError> {
        self.head_record(key).await?;
        self.client
            .delete_object()
            .bucket(self.buckets.name(key.storage_area()))
            .key(key.path())
            .send()
            .await
            .map_err(|error| ObjectStoreError::Unavailable(error.to_string()))?;
        Ok(())
    }

    async fn signed_url(
        &self,
        key: &ObjectAddress,
        now: Timestamp,
    ) -> Result<SignedUrl, ObjectStoreError> {
        if !key.may_issue_signed_url() {
            return Err(ObjectStoreError::NotSignable);
        }
        self.head_record(key).await?;
        let lifetime = signed_url_lifetime(key.storage_area())?;
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
            .bucket(self.buckets.name(key.storage_area()))
            .key(key.path())
            .presigned(config)
            .await
            .map_err(|error| ObjectStoreError::Unavailable(error.to_string()))?;
        Ok(SignedUrl {
            url: request.uri().to_string(),
            expires_at: Timestamp::from_unix_millis(expires_millis),
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
    key: &ObjectAddress,
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
    if record.address != *key
        || record.id != key.object_id()
        || record.storage_area != key.storage_area()
        || record.data_class != key.data_class()
        || record.question_revision != key.question_revision().cloned()
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
    if size_bytes != record.size_bytes || Sha256Checksum::compute(&bytes) != record.sha256 {
        return Err(ObjectStoreError::ChecksumMismatch);
    }
    Ok(StoredObject { record, bytes })
}

#[cfg(feature = "s3")]
fn signed_url_lifetime(storage_area: ObjectStorageArea) -> Result<Duration, ObjectStoreError> {
    match storage_area {
        ObjectStorageArea::PublicAssets | ObjectStorageArea::PrivateContent => {
            Ok(Duration::from_secs(60 * 60))
        }
        ObjectStorageArea::StudentRecords => Ok(Duration::from_secs(5 * 60)),
        ObjectStorageArea::TempProcessing => Err(ObjectStoreError::NotSignable),
    }
}

#[cfg(feature = "s3")]
fn system_time(timestamp: Timestamp) -> Result<SystemTime, ObjectStoreError> {
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
fn requires_immutable_publication_tag(key: &ObjectAddress) -> bool {
    matches!(key, ObjectAddress::QuestionAsset { .. })
}

#[cfg(feature = "s3")]
fn immutable_publication_tagging(key: &ObjectAddress) -> Option<&'static str> {
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
    use crate::ObjectDataClass;
    use question_model::{
        ObjectId, QuestionAssetId, QuestionId, QuestionRevisionNumber, QuestionRevisionReference,
    };
    use uuid::Uuid;

    fn question_revision() -> QuestionRevisionReference {
        QuestionRevisionReference {
            question_id: QuestionId::from_canonical_parts("ABCDEF", 'G').expect("Question ID"),
            revision_number: QuestionRevisionNumber::new(2).expect("positive version"),
        }
    }

    fn record() -> ObjectRecord {
        let key = ObjectAddress::QuestionSource {
            question_revision: question_revision(),
            object: ObjectId::from_uuid(Uuid::from_u128(3)),
        };
        ObjectRecord {
            id: key.object_id(),
            storage_area: key.storage_area(),
            data_class: key.data_class(),
            address: key,
            sha256: Sha256Checksum::compute(b"source"),
            size_bytes: 6,
            media_type: "application/zip".to_string(),
            question_revision: Some(question_revision()),
            created_at: Timestamp::from_unix_millis(1_000),
        }
    }

    fn public_asset_key() -> ObjectAddress {
        ObjectAddress::QuestionAsset {
            question_revision: question_revision(),
            asset: QuestionAssetId::from_uuid(Uuid::from_u128(3)),
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
            &expected.address,
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
        let requested = ObjectAddress::Temporary {
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
    fn metadata_with_an_address_mismatched_data_class_is_rejected() {
        let key = record().address;
        let mut stored = record();
        stored.data_class = ObjectDataClass::TemporaryProcessing;
        let metadata = HashMap::from([(
            RECORD_METADATA_KEY.to_string(),
            encode_record(&stored).expect("record should encode"),
        )]);

        assert!(matches!(
            decode_record(&key, Some(&metadata), Some(6), Some("application/zip")),
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
        let private_source = record().address;
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

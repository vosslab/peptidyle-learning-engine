//! Dedicated publisher for immutable public Question Image Asset renditions.
//!
//! It owns no listener, session, Account, generic Job, or caller-selected
//! object key. The registry claim fixes the private source and final public
//! destination before this process sees either object identity.

use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{Result, anyhow};
use learning_data_access::{ClaimedQuestionImagePublication, PublicAssetPublicationStore};
use objects::{
    ObjectAddress, ObjectDataClass, ObjectRecord, ObjectStorageArea, ObjectStore, ObjectStoreError,
    PutObject, Sha256Checksum, image_validation::verify_still_image,
};
use question_model::Timestamp;
use uuid::Uuid;

// The publisher's existing Compose stop grace is 30 seconds. Use that same
// budget for a complete claim/read/write/activation operation, well inside
// the database's 300-second Job lease. Shutdown cancels pending I/O immediately;
// timeout cancellation leaves any claimed Job to the same lease recovery.
pub(crate) const PUBLICATION_OPERATION_BOUND: std::time::Duration =
    std::time::Duration::from_secs(30);

/// Claims and publishes at most one registry-backed Question Image Asset Job.
///
/// PostgreSQL owns the existing Job lease, attempt limit, and recovery policy.
pub async fn publish_one<S, O>(store: &S, objects: &O) -> Result<bool>
where
    S: PublicAssetPublicationStore,
    O: ObjectStore,
{
    let lease_token = Uuid::now_v7();
    let Some(publication) = store
        .claim_question_image_publication(lease_token)
        .await
        .map_err(|_| anyhow!("could not claim a Question Image Asset Publication Job"))?
    else {
        return Ok(false);
    };
    publish_claim(objects, &publication).await?;
    store
        .activate_question_image_publication(publication.job_id, lease_token)
        .await
        .map_err(|_| anyhow!("could not activate a Question Image Asset Publication"))?;
    tracing::info!(event = "public_asset_publication_completed");
    Ok(true)
}

/// Poll one registry-backed Job at a time until the process is stopped.
///
/// The normal runtime uses the same fixed-address publication operation as
/// the installation publisher. No generic queue or retry state is introduced.
pub async fn run_until_shutdown<S, O>(store: S, objects: O) -> Result<()>
where
    S: PublicAssetPublicationStore,
    O: ObjectStore,
{
    run_until_stopped(&store, &objects, shutdown_signal()).await
}

async fn run_until_stopped<S, O>(
    store: &S,
    objects: &O,
    stop: impl std::future::Future<Output = ()>,
) -> Result<()>
where
    S: PublicAssetPublicationStore,
    O: ObjectStore,
{
    tokio::pin!(stop);
    tracing::info!(event = "public_asset_publisher_started");
    loop {
        tokio::select! {
            biased;
            () = &mut stop => {
                tracing::info!(event = "public_asset_publisher_shutdown_requested");
                return Ok(());
            }
            () = tokio::time::sleep(std::time::Duration::from_secs(1)) => {}
        }
        // ASVS 16.2.5: publish_one returns redacted operation errors, not
        // database URLs, object credentials, or private source content.
        tokio::select! {
            biased;
            () = &mut stop => {
                tracing::info!(event = "public_asset_publisher_shutdown_requested");
                return Ok(());
            }
            result = tokio::time::timeout(PUBLICATION_OPERATION_BOUND, publish_one(store, objects)) => {
                match result {
                    Ok(Ok(_)) => {}
                    Ok(Err(error)) => {
                        tracing::warn!(event = "public_asset_publication_failed", error = %error);
                    }
                    Err(_) => {
                        tracing::warn!(event = "public_asset_publication_timed_out");
                    }
                }
            }
        }
    }
}

async fn shutdown_signal() {
    #[cfg(unix)]
    {
        let mut terminate =
            tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
                .expect("SIGTERM signal handler installs");
        tokio::select! {
            _ = tokio::signal::ctrl_c() => {},
            _ = terminate.recv() => {},
        }
    }
    #[cfg(not(unix))]
    if let Err(error) = tokio::signal::ctrl_c().await {
        tracing::error!(event = "public_asset_publisher_shutdown_signal_unavailable", error = %error);
    }
}

async fn publish_claim<O: ObjectStore>(
    objects: &O,
    publication: &ClaimedQuestionImagePublication,
) -> Result<()> {
    let source_address = ObjectAddress::RestrictedQuestionImage {
        published_question_revision_tuple: publication.published_question_revision_tuple.clone(),
        question_image_asset_id: publication.question_image_asset_id,
        object_id: publication.source_object_id,
    };
    let source = objects
        .get(&source_address)
        .await
        .map_err(|_| anyhow!("could not read the claimed restricted Question Image Asset"))?;
    require_exact_source(&source.record, &source_address, publication)?;
    let verified = verify_still_image(&source.bytes).map_err(|_| {
        anyhow!("claimed restricted Question Image Asset is not a valid still image")
    })?;
    if verified.media_type.canonical_media_type() != publication.verified_media_type
        || verified.width != publication.intrinsic_width
        || verified.height != publication.intrinsic_height
        || Sha256Checksum::compute(&source.bytes) != publication.public_checksum
        || u64::try_from(source.bytes.len()).ok() != Some(publication.public_byte_length)
    {
        return Err(anyhow!(
            "claimed Question Image Asset does not match its fixed public rendition"
        ));
    }

    let public_address = ObjectAddress::QuestionImage {
        published_question_revision_tuple: publication.published_question_revision_tuple.clone(),
        question_image_asset_id: publication.question_image_asset_id,
        object_id: publication.public_object_id,
    };
    let expected = ExpectedPublicRecord {
        address: &public_address,
        checksum: publication.public_checksum,
        byte_length: publication.public_byte_length,
        media_type: &publication.verified_media_type,
    };
    match objects
        .put(PutObject {
            address: public_address.clone(),
            bytes: source.bytes,
            media_type: publication.verified_media_type.clone(),
            created_at: now(),
        })
        .await
    {
        Ok(record) => require_exact_public_record(&record, expected),
        Err(ObjectStoreError::AlreadyExists) => {
            let existing = objects.get(&public_address).await.map_err(|_| {
                anyhow!("could not verify the existing public Question Image Asset")
            })?;
            require_exact_public_record(&existing.record, expected)
        }
        Err(_) => Err(anyhow!(
            "could not write the immutable public Question Image Asset"
        )),
    }
}

fn require_exact_source(
    record: &ObjectRecord,
    address: &ObjectAddress,
    publication: &ClaimedQuestionImagePublication,
) -> Result<()> {
    if record.address != *address
        || record.id != publication.source_object_id
        || record.storage_area != ObjectStorageArea::PrivateContent
        || record.data_class != ObjectDataClass::QuestionImage
        || record.sha256 != publication.source_checksum
        || record.media_type != publication.verified_media_type
    {
        return Err(anyhow!(
            "claimed restricted Question Image Asset record is not exact"
        ));
    }
    Ok(())
}

#[derive(Clone, Copy)]
struct ExpectedPublicRecord<'a> {
    address: &'a ObjectAddress,
    checksum: Sha256Checksum,
    byte_length: u64,
    media_type: &'a str,
}

fn require_exact_public_record(
    record: &ObjectRecord,
    expected: ExpectedPublicRecord<'_>,
) -> Result<()> {
    if record.address != *expected.address
        || record.id != expected.address.object_id()
        || record.storage_area != ObjectStorageArea::PublicAssets
        || record.data_class != ObjectDataClass::QuestionImage
        || record.sha256 != expected.checksum
        || record.size_bytes != expected.byte_length
        || record.media_type != expected.media_type
    {
        return Err(anyhow!("public Question Image Asset record is not exact"));
    }
    Ok(())
}

fn now() -> Timestamp {
    let milliseconds = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .unwrap_or_default();
    Timestamp::from_unix_millis(i64::try_from(milliseconds).unwrap_or(i64::MAX))
}

//! Explicit finite maintenance; Library reads and process startup never invoke it.

use std::{future::Future, time::Duration};

use adapter_webwork::{ResolvedWebworkQuestionSource, WebworkQuestionSourceBinding};
use learning_data_access::{
    PreparedQuestionThumbnailPublication, QuestionThumbnailBackfillEvidence,
    QuestionThumbnailBackfillRegistrationError, QuestionThumbnailBackfillStore,
};
use objects::{ObjectAddress, ObjectStore, PreparedQuestionThumbnail, PutObject, Sha256Checksum};
use question_model::{ObjectId, PublishedQuestionRevisionTuple, SourceObjectChecksum, Timestamp};

use crate::question_publication::PreparedPublicationThumbnail;

/// Safe exact-Revision result, suitable for durable maintenance stdout.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QuestionThumbnailBackfillOutcome {
    Created,
    AlreadyComplete,
    Failed {
        stage: &'static str,
        reason: &'static str,
    },
}

/// One finite run's outcome. A nonzero failed count requires a nonzero CLI exit.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct QuestionThumbnailBackfillTotals {
    pub created: usize,
    pub already_complete: usize,
    pub failed: usize,
}

/// Reports and continues each independent Revision failure in snapshot order.
/// The capture dependency receives only the already resolved exact source;
/// production composition supplies the existing seed-zero renderer/client.
pub async fn backfill_question_thumbnails<S, O, F, Fut>(
    store: &S,
    objects: &O,
    mut capture: F,
    mut progress: impl FnMut(&PublishedQuestionRevisionTuple, QuestionThumbnailBackfillOutcome),
) -> Result<QuestionThumbnailBackfillTotals, &'static str>
where
    S: QuestionThumbnailBackfillStore,
    O: ObjectStore,
    F: FnMut(ResolvedWebworkQuestionSource, Sha256Checksum) -> Fut,
    Fut: Future<Output = Result<PreparedPublicationThumbnail, ()>>,
{
    let revisions = tokio::time::timeout(Duration::from_secs(10), store.snapshot())
        .await
        .map_err(|_| "snapshot timed out")?
        .map_err(|_| "snapshot unavailable")?;
    let mut totals = QuestionThumbnailBackfillTotals::default();
    for item in revisions {
        let result = match item.evidence {
            Ok(evidence) => {
                process_revision(store, objects, &mut capture, &item.revision, evidence).await
            }
            Err(_) => Err(failed("source", "registered evidence missing or invalid")),
        };
        let outcome = result.unwrap_or_else(|outcome| outcome);
        match outcome {
            QuestionThumbnailBackfillOutcome::Created => totals.created += 1,
            QuestionThumbnailBackfillOutcome::AlreadyComplete => totals.already_complete += 1,
            QuestionThumbnailBackfillOutcome::Failed { .. } => totals.failed += 1,
        }
        // ASVS 16.5.1: only typed identity and fixed diagnostic literals
        // leave this boundary; source, HTML, URLs and transport errors stay private.
        progress(&item.revision, outcome);
    }
    Ok(totals)
}

async fn process_revision<S, O, F, Fut>(
    store: &S,
    objects: &O,
    capture: &mut F,
    revision: &PublishedQuestionRevisionTuple,
    evidence: QuestionThumbnailBackfillEvidence,
) -> Result<QuestionThumbnailBackfillOutcome, QuestionThumbnailBackfillOutcome>
where
    S: QuestionThumbnailBackfillStore,
    O: ObjectStore,
    F: FnMut(ResolvedWebworkQuestionSource, Sha256Checksum) -> Fut,
    Fut: Future<Output = Result<PreparedPublicationThumbnail, ()>>,
{
    let binding =
        WebworkQuestionSourceBinding::new(revision.clone(), evidence.webwork_pg_path.clone())
            .map_err(|_| failed("source", "registered PG binding invalid"))?;
    verify_source(objects, revision, &evidence).await?;
    if evidence.thumbnail.is_some() {
        verify_thumbnail(objects, &evidence).await?;
        return Ok(QuestionThumbnailBackfillOutcome::AlreadyComplete);
    }
    let source = tokio::time::timeout(
        Duration::from_secs(10),
        ResolvedWebworkQuestionSource::resolve(
            objects,
            binding,
            evidence.source_record.id,
            SourceObjectChecksum::parse(evidence.source_record.sha256.to_string())
                .map_err(|_| failed("source", "source checksum invalid"))?,
        ),
    )
    .await
    .map_err(|_| failed("source", "source resolution timed out"))?
    .map_err(|_| failed("source", "exact source resolution failed"))?;
    let captured = tokio::time::timeout(
        Duration::from_secs(45),
        capture(source, evidence.source_record.sha256),
    )
    .await
    .map_err(|_| failed("capture", "seed-zero capture timed out"))?
    .map_err(|_| failed("capture", "seed-zero capture failed"))?;
    if captured.source_checksum != evidence.source_record.sha256 {
        return Err(failed("capture", "captured source checksum differs"));
    }
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .ok()
        .and_then(|value| i64::try_from(value.as_millis()).ok())
        .ok_or_else(|| failed("prepare", "system clock unavailable"))?;
    let address = ObjectAddress::QuestionThumbnail {
        published_question_revision_tuple: revision.clone(),
        object_id: ObjectId::generate(),
    };
    // Track before put, including a write whose transport response is lost.
    let mut targets = Vec::new();
    let put = tokio::time::timeout(
        Duration::from_secs(10),
        crate::question_publication_objects::put_candidate(
            objects,
            &mut targets,
            PutObject {
                address,
                bytes: captured.thumbnail.bytes().to_vec(),
                media_type: "image/webp".into(),
                created_at: Timestamp::from_unix_millis(now),
            },
        ),
    )
    .await;
    let record = match put {
        Ok(Ok(record)) => record,
        _ => return Err(cleanup(objects, &targets, failed("put", "candidate write failed")).await),
    };
    let prepared = PreparedQuestionThumbnailPublication {
        object_record: record,
        source_checksum: captured.source_checksum,
        intrinsic_width: captured.thumbnail.width(),
        intrinsic_height: captured.thumbnail.height(),
    };
    if prepared.validate(&evidence.source_record).is_err()
        || targets.first() != Some(&prepared.object_record.address)
        || prepared.object_record.created_at != Timestamp::from_unix_millis(now)
        || prepared.object_record.sha256 != Sha256Checksum::compute(captured.thumbnail.bytes())
        || prepared.object_record.size_bytes != captured.thumbnail.bytes().len() as u64
    {
        return Err(cleanup(
            objects,
            &targets,
            failed("prepare", "candidate evidence differs"),
        )
        .await);
    }
    match tokio::time::timeout(
        Duration::from_secs(25),
        store.register(revision, &evidence.source_record, &prepared),
    )
    .await
    {
        Ok(Ok(true)) => Ok(QuestionThumbnailBackfillOutcome::Created),
        Ok(Ok(false)) => {
            // The SQL function confirmed it did not insert this candidate.
            let cleanup_result = cleanup(
                objects,
                &targets,
                QuestionThumbnailBackfillOutcome::AlreadyComplete,
            )
            .await;
            if matches!(
                cleanup_result,
                QuestionThumbnailBackfillOutcome::Failed { .. }
            ) {
                return Err(cleanup_result);
            }
            let winner =
                tokio::time::timeout(Duration::from_secs(10), store.load_revision(revision))
                    .await
                    .map_err(|_| failed("winner", "concurrent artifact read timed out"))?
                    .map_err(|_| failed("winner", "concurrent artifact evidence unavailable"))?;
            verify_source(objects, revision, &winner).await?;
            verify_thumbnail(objects, &winner).await?;
            Ok(QuestionThumbnailBackfillOutcome::AlreadyComplete)
        }
        Ok(Err(QuestionThumbnailBackfillRegistrationError::NotCommitted(_))) => Err(cleanup(
            objects,
            &targets,
            failed("register", "registration rolled back"),
        )
        .await),
        // Cancellation, transport loss, and COMMIT uncertainty retain the
        // candidate. A rerun consults the immutable registry before capture.
        _ => Err(failed("register", "outcome unknown; candidate retained")),
    }
}

async fn verify_source<O: ObjectStore>(
    objects: &O,
    revision: &PublishedQuestionRevisionTuple,
    evidence: &QuestionThumbnailBackfillEvidence,
) -> Result<(), QuestionThumbnailBackfillOutcome> {
    let record = &evidence.source_record;
    if record.address
        != (ObjectAddress::QuestionSource {
            published_question_revision_tuple: revision.clone(),
            object_id: record.id,
        })
        || record.published_question_revision_tuple.as_ref() != Some(revision)
        || record.storage_area != objects::ObjectStorageArea::PrivateContent
        || record.data_class != objects::ObjectDataClass::QuestionSource
        || record.media_type != "text/x-wework-pg"
    {
        return Err(failed("source", "exact source binding differs"));
    }
    let stored = tokio::time::timeout(Duration::from_secs(10), objects.get(&record.address))
        .await
        .map_err(|_| failed("source", "source read timed out"))?
        .map_err(|_| failed("source", "source bytes missing or corrupt"))?;
    if stored.record != *record
        || Sha256Checksum::compute(&stored.bytes) != record.sha256
        || stored.bytes.len() as u64 != record.size_bytes
    {
        return Err(failed("source", "stored source evidence differs"));
    }
    Ok(())
}

async fn verify_thumbnail<O: ObjectStore>(
    objects: &O,
    evidence: &QuestionThumbnailBackfillEvidence,
) -> Result<(), QuestionThumbnailBackfillOutcome> {
    let artifact = evidence
        .thumbnail
        .as_ref()
        .ok_or_else(|| failed("artifact", "registered artifact missing"))?;
    PreparedQuestionThumbnailPublication {
        object_record: artifact.object_record.clone(),
        source_checksum: evidence.source_record.sha256,
        intrinsic_width: artifact.intrinsic_width,
        intrinsic_height: artifact.intrinsic_height,
    }
    .validate(&evidence.source_record)
    .map_err(|_| failed("artifact", "registered artifact metadata invalid"))?;
    let stored = tokio::time::timeout(
        Duration::from_secs(10),
        objects.get(&artifact.object_record.address),
    )
    .await
    .map_err(|_| failed("artifact", "artifact read timed out"))?
    .map_err(|_| failed("artifact", "registered bytes missing or corrupt"))?;
    if stored.record != artifact.object_record
        || Sha256Checksum::compute(&stored.bytes) != artifact.object_record.sha256
        || stored.bytes.len() as u64 != artifact.object_record.size_bytes
        || PreparedQuestionThumbnail::new(stored.bytes).is_err()
    {
        return Err(failed("artifact", "stored artifact evidence differs"));
    }
    Ok(())
}

async fn cleanup<O: ObjectStore>(
    objects: &O,
    targets: &[ObjectAddress],
    outcome: QuestionThumbnailBackfillOutcome,
) -> QuestionThumbnailBackfillOutcome {
    match crate::question_publication_images::cleanup_targets(objects, targets).await {
        Ok(()) => outcome,
        Err(_) => failed("cleanup", "owned candidate cleanup failed"),
    }
}

const fn failed(stage: &'static str, reason: &'static str) -> QuestionThumbnailBackfillOutcome {
    QuestionThumbnailBackfillOutcome::Failed { stage, reason }
}

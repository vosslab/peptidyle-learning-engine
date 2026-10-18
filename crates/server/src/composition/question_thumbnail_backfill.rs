//! Explicit maintenance composition over existing migration/storage/render boundaries.

use anyhow::{Result, anyhow};
use learning_data_access::postgres::{PostgresQuestionThumbnailBackfillStore, lazy_pool};
use question_model::{PublishedQuestionRevisionTuple, generation::QuestionSeed};

use super::{environment, object_storage};
use crate::question_thumbnail_backfill::{
    QuestionThumbnailBackfillOutcome, QuestionThumbnailBackfillTotals, backfill_question_thumbnails,
};

/// Runs only when the maintainer explicitly selects `question-thumbnails backfill`.
pub async fn backfill_question_thumbnails_from_env(
    progress: impl FnMut(&PublishedQuestionRevisionTuple, QuestionThumbnailBackfillOutcome),
) -> Result<QuestionThumbnailBackfillTotals> {
    let database_url = environment::required_env("PLE_MIGRATION_DATABASE_URL")?;
    let pool = lazy_pool(&database_url)
        .map_err(|_| anyhow!("migration database configuration invalid"))?;
    let store = PostgresQuestionThumbnailBackfillStore::new(pool);
    let objects = object_storage::object_store_from_env(
        object_storage::ObjectStoragePrincipal::ApiAndWorker,
    )?;
    let webwork = environment::webwork_adapter_from_env()?;
    let capture = environment::question_thumbnail_capture_from_env()?;
    backfill_question_thumbnails(
        &store,
        &objects,
        |source, checksum| {
            let webwork = webwork.clone();
            let capture = capture.clone();
            async move {
                let seed = QuestionSeed::new(0);
                let document = webwork
                    .preview_document(seed, &source)
                    .await
                    .map_err(|_| ())?;
                let document = String::from_utf8(document).map_err(|_| ())?;
                capture.capture(document, checksum).await.map_err(|_| ())
            }
        },
        progress,
    )
    .await
    .map_err(anyhow::Error::msg)
}

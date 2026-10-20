//! Best-effort JEV classification after publication, using a short in-memory queue.

mod client;
mod evidence;

use std::sync::{Arc, OnceLock};

use adapter_webwork::{HttpWebworkRenderer, WebworkAdapter};
use learning_data_access::postgres::PostgresBloomStore;
use objects::s3::S3ObjectStore;
use question_model::{PublishedQuestionRevisionTuple, QuestionPoolId};
use tokio::{sync::mpsc, task::JoinHandle};

use client::JevClient;

struct Target {
    kind: &'static str,
    id: String,
    revision: i32,
}
static QUEUE: OnceLock<mpsc::Sender<Target>> = OnceLock::new();

/// Keeps the API-owned task alive, and cancels it when the API shuts down.
pub struct BloomTask(JoinHandle<()>);
impl Drop for BloomTask {
    fn drop(&mut self) {
        self.0.abort();
    }
}

/// Called after a successful database publication. Queue capacity never delays publication.
pub(crate) fn question_published(revision: &PublishedQuestionRevisionTuple) {
    if let Ok(number) = i32::try_from(revision.revision_number.get()) {
        enqueue(Target {
            kind: "question",
            id: revision.published_question_id.to_string(),
            revision: number,
        });
    }
}

/// Creation and membership saves submit the Pool identity; member modes are read afterward.
pub(crate) fn queue_pool_bloom_calculation(id: &QuestionPoolId) {
    enqueue(Target {
        kind: "pool",
        id: id.to_string(),
        revision: 0,
    });
}

fn enqueue(target: Target) {
    if let Some(queue) = QUEUE.get()
        && queue.try_send(target).is_err()
    {
        tracing::warn!(event = "bloom_queue_unavailable");
    }
}

/// Missing credentials disable only classification. The API and publication remain available.
pub(crate) fn start(
    store: PostgresBloomStore,
    objects: S3ObjectStore,
    webwork: Arc<WebworkAdapter<HttpWebworkRenderer>>,
) -> Option<BloomTask> {
    let key = std::env::var("PLE_JEV_API_KEY")
        .ok()
        .filter(|s| !s.trim().is_empty())
        .or_else(|| {
            std::env::var("PLE_JEV_API_KEY_FILE")
                .ok()
                .and_then(|path| std::fs::read_to_string(path).ok())
        });
    let client = key.filter(|s| !s.trim().is_empty()).and_then(|key| {
        JevClient::new(&key)
            .map_err(|error| {
                tracing::warn!(event = "bloom_unconfigured", reason = error.reason);
            })
            .ok()
    });
    if client.is_none() {
        tracing::info!(event = "bloom_unconfigured");
    }
    let (sender, mut receiver) = mpsc::channel::<Target>(64);
    if QUEUE.set(sender).is_err() {
        return None;
    }
    Some(BloomTask(tokio::spawn(async move {
        while let Some(target) = receiver.recv().await {
            let candidate = match store
                .candidate(target.kind, &target.id, target.revision)
                .await
            {
                Ok(Some(candidate)) => candidate,
                Ok(None) => continue,
                Err(_) => {
                    tracing::warn!(event = "bloom_read_failed", id = target.id);
                    continue;
                }
            };
            if target.kind == "pool" {
                if store
                    .save(
                        &candidate,
                        candidate.cognitive.as_deref(),
                        candidate.knowledge.as_deref(),
                    )
                    .await
                    .is_err()
                {
                    tracing::warn!(event = "bloom_pool_save_failed", id = target.id);
                }
                continue;
            }
            let Some(client) = &client else {
                continue;
            };
            let evidence = match evidence::prepare(&candidate, &objects, &webwork).await {
                Ok(evidence) => evidence,
                Err(error) => {
                    tracing::warn!(
                        event = "bloom_evidence_failed",
                        id = target.id,
                        reason = error.reason
                    );
                    continue;
                }
            };
            match client.classify(evidence).await {
                Ok((cognitive, knowledge)) => match store
                    .save(&candidate, Some(&cognitive), Some(&knowledge))
                    .await
                {
                    Ok(saved) => tracing::info!(
                        event = "bloom_classification_finished",
                        id = target.id,
                        saved
                    ),
                    Err(_) => tracing::warn!(event = "bloom_save_failed", id = target.id),
                },
                Err(error) => {
                    tracing::warn!(
                        event = "bloom_classification_failed",
                        id = target.id,
                        reason = error.reason
                    );
                    // Respect service throttling before processing the next queued request.
                    if error.reason == "jev_rate_limit" {
                        tokio::time::sleep(error.retry_after).await;
                    }
                }
            }
        }
    })))
}

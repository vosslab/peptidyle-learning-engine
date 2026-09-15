//! Bounded private Watch-notification materialization for the worker only.

use async_trait::async_trait;

use crate::StoreError;

#[async_trait]
pub trait QuestionWatchNotificationStore: Send + Sync {
    /// Materializes at most `limit` in-app private Watch notifications.
    async fn materialize_question_watch_notifications(&self, limit: u16)
    -> Result<u32, StoreError>;
}

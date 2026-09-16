//! Read-only global classification selector capability.

use async_trait::async_trait;
use uuid::Uuid;

use crate::{SessionTokenHash, StoreError};

/// One vocabulary identity and its display name, in SQL-defined order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContentClassificationItem {
    pub uuid: Uuid,
    pub name: String,
}

/// Authorized immediate-child reads; SQL owns actor and association policy.
#[async_trait]
pub trait ContentClassificationStore: Send + Sync {
    async fn list_disciplines(
        &self,
        token: SessionTokenHash,
    ) -> Result<Vec<ContentClassificationItem>, StoreError>;
    async fn list_subjects(
        &self,
        token: SessionTokenHash,
        discipline_uuid: Uuid,
    ) -> Result<Vec<ContentClassificationItem>, StoreError>;
    async fn list_topics(
        &self,
        token: SessionTokenHash,
        subject_uuid: Uuid,
    ) -> Result<Vec<ContentClassificationItem>, StoreError>;
    async fn list_subtopics(
        &self,
        token: SessionTokenHash,
        topic_uuid: Uuid,
    ) -> Result<Vec<ContentClassificationItem>, StoreError>;
}

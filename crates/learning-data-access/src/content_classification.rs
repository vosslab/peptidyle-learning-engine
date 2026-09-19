//! Global classification selector and Sysadmin Discipline-lifecycle capability.

use async_trait::async_trait;
use uuid::Uuid;

use crate::{SessionTokenHash, StoreError};

/// One vocabulary identity, display name, and reversible lifecycle state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContentClassificationItem {
    pub uuid: Uuid,
    pub name: String,
}

/// One Discipline identity and its reversible lifecycle state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContentDiscipline {
    pub uuid: Uuid,
    pub name: String,
    pub is_retired: bool,
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

/// All-status Discipline discovery for filters and existing-ID resolution.
#[async_trait]
pub trait ContentDisciplineDiscoveryStore: Send + Sync {
    /// Lists active and retired Disciplines for discovery and exact IDs.
    async fn list_disciplines_including_retired(
        &self,
        token: SessionTokenHash,
    ) -> Result<Vec<ContentDiscipline>, StoreError>;
}

/// Sysadmin Discipline lifecycle, separate from the all-status discovery projection.
#[async_trait]
pub trait ContentDisciplineAdministrationStore: ContentDisciplineDiscoveryStore {
    /// Creates a stable active Discipline; PostgreSQL derives Sysadmin authority from the session.
    async fn create_discipline(
        &self,
        token: SessionTokenHash,
        name: String,
    ) -> Result<ContentDiscipline, StoreError>;
    /// Renames one stable Discipline without changing its UUID.
    async fn rename_discipline(
        &self,
        token: SessionTokenHash,
        discipline_uuid: Uuid,
        name: String,
    ) -> Result<ContentDiscipline, StoreError>;
    /// Retires one Discipline without deleting existing references.
    async fn retire_discipline(
        &self,
        token: SessionTokenHash,
        discipline_uuid: Uuid,
    ) -> Result<ContentDiscipline, StoreError>;
    /// Restores one previously retired Discipline.
    async fn restore_discipline(
        &self,
        token: SessionTokenHash,
        discipline_uuid: Uuid,
    ) -> Result<ContentDiscipline, StoreError>;
}

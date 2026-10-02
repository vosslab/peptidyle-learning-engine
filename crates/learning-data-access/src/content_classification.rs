//! Global classification selector and Sysadmin Discipline-lifecycle capability.

use async_trait::async_trait;
use uuid::Uuid;

use crate::{SessionTokenHash, StoreError};
use question_model::AccountId;

/// One vocabulary identity, display name, and reversible lifecycle state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContentClassificationItem {
    pub uuid: Uuid,
    pub name: String,
}

/// Result of asking to create a Subject inside one Discipline.
///
/// An existing global name is offered for explicit acceptance and is not
/// associated by the create call.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContentSubjectCreation {
    pub uuid: Uuid,
    pub name: String,
    pub needs_acceptance: bool,
}

/// One Discipline identity and its reversible lifecycle state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContentDiscipline {
    pub uuid: Uuid,
    pub name: String,
    pub is_retired: bool,
}

/// One open request for a Discipline the vocabulary does not yet offer.
///
/// The requester is an Account ID. This record does not include an email or a display name.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContentDisciplineRequest {
    pub uuid: Uuid,
    pub requested_name: String,
    pub requested_by_account_id: AccountId,
}

/// Authorized immediate-child reads and Instructor vocabulary creation.
/// SQL owns actor and association policy.
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
    /// Creates a Subject in the Discipline, or offers an existing global name.
    async fn create_subject(
        &self,
        token: SessionTokenHash,
        name: String,
        discipline_uuid: Uuid,
    ) -> Result<ContentSubjectCreation, StoreError>;
    /// Associates an offered Subject with the selected Discipline.
    async fn accept_subject_discipline(
        &self,
        token: SessionTokenHash,
        subject_uuid: Uuid,
        discipline_uuid: Uuid,
    ) -> Result<ContentClassificationItem, StoreError>;
    async fn create_topic(
        &self,
        token: SessionTokenHash,
        name: String,
        subject_uuid: Uuid,
    ) -> Result<ContentClassificationItem, StoreError>;
    async fn create_subtopic(
        &self,
        token: SessionTokenHash,
        name: String,
        topic_uuid: Uuid,
    ) -> Result<ContentClassificationItem, StoreError>;
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

/// Course Discipline requests. Creating the Discipline stays on the Sysadmin lifecycle.
#[async_trait]
pub trait ContentDisciplineRequestStore: Send + Sync {
    /// Records a bounded name request for the signed-in Instructor or Sysadmin.
    async fn request_discipline(
        &self,
        token: SessionTokenHash,
        name: String,
    ) -> Result<ContentDisciplineRequest, StoreError>;
    /// Lists open requests for a Sysadmin. Each row carries the requester Account ID only.
    async fn list_open_discipline_requests(
        &self,
        token: SessionTokenHash,
    ) -> Result<Vec<ContentDisciplineRequest>, StoreError>;
    /// Marks one open request resolved. It does not create a Discipline.
    async fn resolve_discipline_request(
        &self,
        token: SessionTokenHash,
        request_uuid: Uuid,
    ) -> Result<(), StoreError>;
    /// Creates the requested Discipline and resolves the request in one database transaction.
    async fn fulfill_discipline_request(
        &self,
        token: SessionTokenHash,
        request_uuid: Uuid,
    ) -> Result<ContentDiscipline, StoreError>;
}

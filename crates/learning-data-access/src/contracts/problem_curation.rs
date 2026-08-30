//! Personal Favorites, named collections, and saved catalog-search persistence.

use async_trait::async_trait;
use question_model::{
    CatalogSearchFilter, ProblemCollectionMemberView, ProblemCollectionReference,
    ProblemCollectionRevision, ProblemCollectionSummaryView, ProblemCollectionVisibility,
    QuestionId, SavedProblemSearchReference, SavedProblemSearchRevision, SavedProblemSearchView,
};

use super::{ActorContext, Page, PageRequest, SessionTokenHash, StoreError};

/// Complete destination chosen by one revision-checked collection replacement.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProblemCollectionReplacementTarget {
    /// Idempotently creates and then replaces the actor's fixed Favorites collection.
    Favorites,
    /// Creates one new named collection.
    NewNamed,
    /// Replaces one existing collection with the same immutable kind.
    Existing(ProblemCollectionReference),
}

/// One complete replacement of a personal collection's mutable state.
///
/// Public Question IDs are resolved under the destination authority inside the
/// Store and become private exact [`question_model::ProblemVersionRef`] values
/// only after that resolution succeeds for every submitted member.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReplaceProblemCollectionCommand {
    pub target: ProblemCollectionReplacementTarget,
    pub expected_revision: Option<ProblemCollectionRevision>,
    pub title: Option<String>,
    pub visibility: Option<ProblemCollectionVisibility>,
    pub question_ids: Vec<QuestionId>,
}

/// One personal saved-search creation or complete replacement.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReplaceSavedProblemSearchCommand {
    pub reference: Option<SavedProblemSearchReference>,
    pub expected_revision: Option<SavedProblemSearchRevision>,
    pub title: String,
    pub filter: CatalogSearchFilter,
}

/// One authorized collection summary paired with its bounded member page.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProblemCollectionMembersPage {
    pub collection: ProblemCollectionSummaryView,
    pub members: Page<ProblemCollectionMemberView>,
}

/// Closed route capability resolved from one active problem-curation session.
///
/// The browser never selects this value. HTTP handlers choose the capability
/// required by their route, while the Store derives actor, roles, and
/// current Instructor approval from the authenticated session.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProblemCurationCapability {
    /// Catalog access plus institution-collection reads.
    CatalogInstitutionRead,
    /// Personal Favorites, named-collection, and saved-search authority.
    PersonalMutation,
}

/// Session-derived capability for problem curation.
#[async_trait]
pub trait ProblemCurationStore: Send + Sync {
    /// Resolves route authority without interpreting operation-specific input.
    async fn preflight_problem_curation(
        &self,
        context: ActorContext,
        session: SessionTokenHash,
        capability: ProblemCurationCapability,
    ) -> Result<(), StoreError>;

    /// Idempotently materializes the active approved Instructor's fixed private Favorites collection.
    async fn get_or_create_favorites(
        &self,
        context: ActorContext,
        session: SessionTokenHash,
    ) -> Result<ProblemCollectionSummaryView, StoreError>;

    /// Lists collections visible to the active curation principal.
    async fn list_problem_collections(
        &self,
        context: ActorContext,
        session: SessionTokenHash,
        page: PageRequest,
    ) -> Result<Page<ProblemCollectionSummaryView>, StoreError>;

    /// Reads one visible collection without exposing an existence oracle.
    async fn get_problem_collection_summary(
        &self,
        context: ActorContext,
        session: SessionTokenHash,
        reference: ProblemCollectionReference,
    ) -> Result<Option<ProblemCollectionSummaryView>, StoreError>;

    /// Reads one bounded page of safe members for an already authorized collection.
    async fn list_problem_collection_members(
        &self,
        context: ActorContext,
        session: SessionTokenHash,
        reference: ProblemCollectionReference,
        page: PageRequest,
    ) -> Result<Option<ProblemCollectionMembersPage>, StoreError>;

    /// Atomically creates or replaces one complete collection state.
    async fn replace_problem_collection(
        &self,
        context: ActorContext,
        session: SessionTokenHash,
        command: ReplaceProblemCollectionCommand,
    ) -> Result<ProblemCollectionSummaryView, StoreError>;

    /// Deletes an owned named collection after a strong revision check.
    async fn delete_problem_collection(
        &self,
        context: ActorContext,
        session: SessionTokenHash,
        reference: ProblemCollectionReference,
        expected_revision: ProblemCollectionRevision,
    ) -> Result<bool, StoreError>;

    /// Lists personal saved D1 filter meanings for the active Instructor.
    async fn list_saved_problem_searches(
        &self,
        context: ActorContext,
        session: SessionTokenHash,
        page: PageRequest,
    ) -> Result<Page<SavedProblemSearchView>, StoreError>;

    /// Reads one personal saved search for its owner without exposing foreign existence.
    async fn get_saved_problem_search(
        &self,
        context: ActorContext,
        session: SessionTokenHash,
        reference: SavedProblemSearchReference,
    ) -> Result<Option<SavedProblemSearchView>, StoreError>;

    /// Creates or replaces one personal saved D1 filter meaning.
    async fn replace_saved_problem_search(
        &self,
        context: ActorContext,
        session: SessionTokenHash,
        command: ReplaceSavedProblemSearchCommand,
    ) -> Result<SavedProblemSearchView, StoreError>;

    /// Deletes one owned saved search after a strong revision check.
    async fn delete_saved_problem_search(
        &self,
        context: ActorContext,
        session: SessionTokenHash,
        reference: SavedProblemSearchReference,
        expected_revision: SavedProblemSearchRevision,
    ) -> Result<bool, StoreError>;
}

//! Persistence boundary for reusable personal Blueprints and shared Alpha curricula.

use async_trait::async_trait;
use question_model::{
    AlphaCourseDefinitionInput, AlphaCourseReference, AlphaCourseRevision, AlphaCourseSummaryView,
    AlphaCourseView, BlueprintDefinitionInput, BlueprintReference, BlueprintRevision,
    BlueprintSummaryView, BlueprintView,
};

use super::{Page, PageRequest, SessionTokenHash, StoreError, TenantContext};

/// Route authority selected by a server handler for reusable curriculum work.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReusableCurriculumCapability {
    /// Read or create the active approved Instructor's personal Blueprints.
    BlueprintPersonal,
    /// Read shared Alpha curricula as an approved Instructor.
    AlphaRead,
    /// Create or replace an Alpha curriculum as its approved-Instructor creator.
    AlphaCreatorWrite,
}

/// Atomically create or replace one complete private Blueprint.
#[derive(Debug, Clone, PartialEq)]
pub struct ReplaceBlueprintCommand {
    /// `None` creates a new aggregate; a reference replaces that aggregate.
    pub reference: Option<BlueprintReference>,
    /// Required for replacement and absent for creation.
    pub expected_revision: Option<BlueprintRevision>,
    /// The aggregate's sole reusable assignment definition.
    pub definition: BlueprintDefinitionInput,
}

/// Atomically create or replace one complete public Alpha curriculum tree.
#[derive(Debug, Clone, PartialEq)]
pub struct ReplaceAlphaCourseCommand {
    /// `None` creates a new public curriculum; a reference replaces it.
    pub reference: Option<AlphaCourseReference>,
    /// Required for replacement and absent for creation.
    pub expected_revision: Option<AlphaCourseRevision>,
    /// Ordered modules and reusable definitions owned by the aggregate.
    pub definition: AlphaCourseDefinitionInput,
}

/// Reusable-curriculum persistence separate from the general teaching Store.
#[async_trait]
pub trait ReusableCurriculumStore: Send + Sync {
    /// Resolves active-session instructor approval before a route decodes its body.
    async fn preflight_reusable_curriculum(
        &self,
        context: TenantContext,
        session: SessionTokenHash,
        capability: ReusableCurriculumCapability,
    ) -> Result<(), StoreError>;

    async fn list_blueprints(
        &self,
        context: TenantContext,
        session: SessionTokenHash,
        page: PageRequest,
    ) -> Result<Page<BlueprintSummaryView>, StoreError>;
    async fn get_blueprint(
        &self,
        context: TenantContext,
        session: SessionTokenHash,
        reference: BlueprintReference,
    ) -> Result<Option<BlueprintView>, StoreError>;
    async fn replace_blueprint(
        &self,
        context: TenantContext,
        session: SessionTokenHash,
        command: ReplaceBlueprintCommand,
    ) -> Result<BlueprintView, StoreError>;
    async fn delete_blueprint(
        &self,
        context: TenantContext,
        session: SessionTokenHash,
        reference: BlueprintReference,
        expected_revision: BlueprintRevision,
    ) -> Result<bool, StoreError>;

    async fn list_alpha_courses(
        &self,
        context: TenantContext,
        session: SessionTokenHash,
        page: PageRequest,
    ) -> Result<Page<AlphaCourseSummaryView>, StoreError>;
    async fn get_alpha_course(
        &self,
        context: TenantContext,
        session: SessionTokenHash,
        reference: AlphaCourseReference,
    ) -> Result<Option<AlphaCourseView>, StoreError>;
    async fn replace_alpha_course(
        &self,
        context: TenantContext,
        session: SessionTokenHash,
        command: ReplaceAlphaCourseCommand,
    ) -> Result<AlphaCourseView, StoreError>;
}

//! Persistence boundary for revisioned, reusable BlueprintCourse trees.

use async_trait::async_trait;
use question_model::{
    BlueprintCourseSummaryView, BlueprintCourseView, BlueprintReference, BlueprintRevision,
    CreateBlueprintCourseDefinitionInput, ReplaceBlueprintCourseDefinitionInput,
};

use super::{ActorContext, Page, PageRequest, SessionTokenHash, StoreError};

/// Route authority selected by a server handler for reusable curriculum work.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReusableCurriculumCapability {
    /// Read a published BlueprintCourse as an approved Instructor.
    BlueprintCourseRead,
    /// Create or replace an owned draft BlueprintCourse.
    BlueprintCourseWrite,
}

/// Creates one complete reusable BlueprintCourse tree.
#[derive(Debug, Clone, PartialEq)]
pub struct CreateBlueprintCourseCommand {
    /// Ordered modules and reusable definitions. Child identities are server-owned.
    pub definition: CreateBlueprintCourseDefinitionInput,
}

/// Replaces one complete reusable BlueprintCourse tree under optimistic revision control.
#[derive(Debug, Clone, PartialEq)]
pub struct ReplaceBlueprintCourseCommand {
    /// Stable aggregate locator selected from the current answer-free view.
    pub reference: BlueprintReference,
    /// Exact head revision observed by the editor.
    pub expected_revision: BlueprintRevision,
    /// Ordered replacement tree with explicit retained/new child handles.
    pub definition: ReplaceBlueprintCourseDefinitionInput,
}

/// Reusable-curriculum persistence separate from the general teaching Store.
#[async_trait]
pub trait ReusableCurriculumStore: Send + Sync {
    /// Resolves active-session instructor approval before a route decodes its body.
    async fn preflight_reusable_curriculum(
        &self,
        context: ActorContext,
        session: SessionTokenHash,
        capability: ReusableCurriculumCapability,
    ) -> Result<(), StoreError>;

    /// Lists draft courses owned by the actor plus published courses readable globally.
    async fn list_blueprint_courses(
        &self,
        context: ActorContext,
        session: SessionTokenHash,
        page: PageRequest,
    ) -> Result<Page<BlueprintCourseSummaryView>, StoreError>;
    /// Gets one draft-owner or published-approved-Instructor answer-free course view.
    async fn get_blueprint_course(
        &self,
        context: ActorContext,
        session: SessionTokenHash,
        reference: BlueprintReference,
    ) -> Result<Option<BlueprintCourseView>, StoreError>;
    /// Creates a complete BlueprintCourse tree and allocates every child identity server-side.
    async fn create_blueprint_course(
        &self,
        context: ActorContext,
        session: SessionTokenHash,
        command: CreateBlueprintCourseCommand,
    ) -> Result<BlueprintCourseView, StoreError>;
    /// Atomically replaces the complete course tree under optimistic revision control.
    async fn replace_blueprint_course(
        &self,
        context: ActorContext,
        session: SessionTokenHash,
        command: ReplaceBlueprintCourseCommand,
    ) -> Result<BlueprintCourseView, StoreError>;
}

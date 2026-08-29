//! Persistence boundary for revisioned, reusable BlueprintCourse trees.

use async_trait::async_trait;
use question_model::{
    BlueprintCourseDefinitionInput, BlueprintCourseSummaryView, BlueprintCourseView,
    BlueprintReference, BlueprintRevision,
};

use super::{Page, PageRequest, SessionTokenHash, StoreError, TenantContext};

/// Route authority selected by a server handler for reusable curriculum work.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReusableCurriculumCapability {
    /// Read a published BlueprintCourse as an approved Instructor.
    BlueprintCourseRead,
    /// Create or replace an owned draft BlueprintCourse.
    BlueprintCourseWrite,
}

/// Atomically create or replace one complete reusable BlueprintCourse tree.
#[derive(Debug, Clone, PartialEq)]
pub struct ReplaceBlueprintCourseCommand {
    /// `None` creates an aggregate; a reference replaces that complete tree.
    pub reference: Option<BlueprintReference>,
    /// Required for replacement and absent for creation.
    pub expected_revision: Option<BlueprintRevision>,
    /// Ordered modules and reusable definitions owned by the aggregate.
    pub definition: BlueprintCourseDefinitionInput,
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

    /// Lists draft courses owned by the actor plus published courses readable globally.
    async fn list_blueprint_courses(
        &self,
        context: TenantContext,
        session: SessionTokenHash,
        page: PageRequest,
    ) -> Result<Page<BlueprintCourseSummaryView>, StoreError>;
    /// Gets one draft-owner or published-approved-Instructor answer-free course view.
    async fn get_blueprint_course(
        &self,
        context: TenantContext,
        session: SessionTokenHash,
        reference: BlueprintReference,
    ) -> Result<Option<BlueprintCourseView>, StoreError>;
    /// Atomically replaces the complete course tree under optimistic revision control.
    async fn replace_blueprint_course(
        &self,
        context: TenantContext,
        session: SessionTokenHash,
        command: ReplaceBlueprintCourseCommand,
    ) -> Result<BlueprintCourseView, StoreError>;
}

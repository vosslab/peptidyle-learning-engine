//! Authorized resolution of human-facing navigation references.

use async_trait::async_trait;
use question_model::{
    AssignmentId, AssignmentPublicId, CourseId, CoursePublicId, RunId, RunPublicId, UserId,
    WorkspaceId, WorkspacePublicId,
};

use crate::{StoreError, TenantContext};

/// Internal assignment route resolved from one public locator.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AssignmentRouteIdentity {
    pub course: CourseId,
    pub assignment: AssignmentId,
}

/// Persistence capability for public route locators.
///
/// Every method takes the authenticated actor. A result is absent unless that actor may navigate to
/// the record under the current tenant; the returned UUID remains an internal transport detail.
#[async_trait]
pub trait NavigationReferenceStore: Send + Sync {
    async fn course_public_id(
        &self,
        context: TenantContext,
        actor: UserId,
        course: CourseId,
    ) -> Result<Option<CoursePublicId>, StoreError>;

    async fn resolve_course_public_id(
        &self,
        context: TenantContext,
        actor: UserId,
        public_id: CoursePublicId,
    ) -> Result<Option<CourseId>, StoreError>;

    async fn assignment_public_id(
        &self,
        context: TenantContext,
        actor: UserId,
        assignment: AssignmentId,
    ) -> Result<Option<AssignmentPublicId>, StoreError>;

    async fn resolve_assignment_public_id(
        &self,
        context: TenantContext,
        actor: UserId,
        public_id: AssignmentPublicId,
    ) -> Result<Option<AssignmentRouteIdentity>, StoreError>;

    async fn run_public_id(
        &self,
        context: TenantContext,
        actor: UserId,
        run: RunId,
    ) -> Result<Option<RunPublicId>, StoreError>;

    async fn resolve_run_public_id(
        &self,
        context: TenantContext,
        actor: UserId,
        public_id: RunPublicId,
    ) -> Result<Option<RunId>, StoreError>;

    async fn workspace_public_id(
        &self,
        context: TenantContext,
        actor: UserId,
        workspace: WorkspaceId,
    ) -> Result<Option<WorkspacePublicId>, StoreError>;

    async fn resolve_workspace_public_id(
        &self,
        context: TenantContext,
        actor: UserId,
        public_id: WorkspacePublicId,
    ) -> Result<Option<WorkspaceId>, StoreError>;
}

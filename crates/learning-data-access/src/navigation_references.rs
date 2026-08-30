//! Authorized resolution of human-facing navigation references.

use async_trait::async_trait;
use question_model::{
    AssignmentId, AssignmentReference, CourseId, CourseReference, EnrollmentId, RunId,
    RunReference, UserId, WorkspaceId, WorkspaceReference,
};

use crate::{ActorContext, StoreError};

/// Internal assignment route resolved from one public locator.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AssignmentRouteIdentity {
    pub course: CourseId,
    pub assignment: AssignmentId,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RunRouteIdentity {
    pub course: CourseId,
    pub assignment: AssignmentId,
    pub enrollment: EnrollmentId,
    pub run: RunId,
}

/// Persistence capability for public route locators.
///
/// Every method takes the authenticated actor. A result is absent unless that actor may navigate to
/// the record through their exact course or workspace relationship; the returned UUID remains an internal transport detail.
#[async_trait]
pub trait NavigationReferenceStore: Send + Sync {
    async fn course_reference(
        &self,
        context: ActorContext,
        actor: UserId,
        course: CourseId,
    ) -> Result<Option<CourseReference>, StoreError>;

    async fn resolve_course_reference(
        &self,
        context: ActorContext,
        actor: UserId,
        reference: CourseReference,
    ) -> Result<Option<CourseId>, StoreError>;

    async fn assignment_reference(
        &self,
        context: ActorContext,
        actor: UserId,
        assignment: AssignmentId,
    ) -> Result<Option<AssignmentReference>, StoreError>;

    async fn resolve_assignment_reference(
        &self,
        context: ActorContext,
        actor: UserId,
        reference: AssignmentReference,
    ) -> Result<Option<AssignmentRouteIdentity>, StoreError>;

    async fn run_reference(
        &self,
        context: ActorContext,
        actor: UserId,
        run: RunId,
    ) -> Result<Option<RunReference>, StoreError>;

    async fn resolve_run_reference(
        &self,
        context: ActorContext,
        actor: UserId,
        reference: RunReference,
    ) -> Result<Option<RunRouteIdentity>, StoreError>;

    async fn workspace_reference(
        &self,
        context: ActorContext,
        actor: UserId,
        workspace: WorkspaceId,
    ) -> Result<Option<WorkspaceReference>, StoreError>;

    async fn resolve_workspace_reference(
        &self,
        context: ActorContext,
        actor: UserId,
        reference: WorkspaceReference,
    ) -> Result<Option<WorkspaceId>, StoreError>;
}

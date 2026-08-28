//! Focused persistence boundary for the non-mutating WP-INST-T3 preview plane.

use crate::{PageRequest, StoreError, TenantContext};
use async_trait::async_trait;
use objects::Sha256Digest;
use question_model::{
    AssignmentReference, CourseId, DerivedPreviewSubjectRequest, InstructorPreviewSchedulePage,
    PreviewAccommodationComparison, PreviewEvaluation, SyntheticPreviewSubjectRequest,
    TeachingOperationRevision, TenantId, UserId,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreviewPlaneResult {
    pub evaluation: PreviewEvaluation,
    pub accommodation: Option<PreviewAccommodationComparison>,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreviewSubjectAudit {
    /// Private persistence provenance. This is deliberately not part of the
    /// T3 browser contract; it fences an audit that happens to contain
    /// colliding IDs in another tenant.
    pub tenant: TenantId,
    pub actor: UserId,
    pub course: CourseId,
    pub assignment: AssignmentReference,
    pub target_membership: question_model::CourseMembershipId,
    pub action: &'static str,
    pub schema_version: u16,
    pub payload_sha256: Sha256Digest,
}
#[async_trait]
pub trait PreviewPlaneStore: Send + Sync {
    async fn list_instructor_preview_schedule(
        &self,
        context: TenantContext,
        actor: UserId,
        course: CourseId,
        assignment: AssignmentReference,
        revision: TeachingOperationRevision,
        page: PageRequest,
    ) -> Result<InstructorPreviewSchedulePage, StoreError>;
    async fn construct_synthetic_preview(
        &self,
        context: TenantContext,
        actor: UserId,
        course: CourseId,
        request: SyntheticPreviewSubjectRequest,
    ) -> Result<PreviewPlaneResult, StoreError>;
    async fn construct_derived_preview(
        &self,
        context: TenantContext,
        actor: UserId,
        course: CourseId,
        request: DerivedPreviewSubjectRequest,
    ) -> Result<PreviewPlaneResult, StoreError>;
}

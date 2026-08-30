//! Persistence boundary for explicit B2 curriculum adoption.

use async_trait::async_trait;
use question_model::{
    CourseInstanceBlueprintInspectionView, CourseReference, CurriculumAdoptionApplyIntent,
    CurriculumAdoptionCompleted, CurriculumAdoptionPreview, CurriculumAdoptionPreviewRequest,
    ReconcileCourseInstanceAdoptionCompleted, ReconcileCourseInstanceAdoptionIntent,
};

use super::{SessionTokenHash, StoreError, TenantContext};

/// Explicit adoption persistence, separate from reusable-source and learner-work Stores.
///
/// The server supplies a validated session token; references are locators only.
/// Implementations re-resolve tenant, actor, source, and destination authority at
/// every boundary. Browser inputs are closed and bounded (ASVS 1.5.2, 2.2.1,
/// 2.2.2); the Store re-resolves them under its atomic write boundary (ASVS 2.3.3).
#[async_trait]
pub trait CurriculumAdoptionStore: Send + Sync {
    /// Resolves a current approved Instructor before a route decodes protected input.
    async fn preflight_curriculum_adoption(
        &self,
        context: TenantContext,
        session: SessionTokenHash,
    ) -> Result<(), StoreError>;

    /// Resolves one browser intent into an answer-free advisory preview.
    ///
    /// Preview reserves no authority. A later apply re-authorizes and re-reads every mutable fact.
    async fn preview_curriculum_adoption(
        &self,
        context: TenantContext,
        session: SessionTokenHash,
        request: CurriculumAdoptionPreviewRequest,
    ) -> Result<CurriculumAdoptionPreview, StoreError>;

    /// Atomically resolves, records, consumes, persists, and idempotently replays one apply intent.
    ///
    /// Implementations keep non-Serde records and immutable receipt evidence inside this operation.
    async fn apply_curriculum_adoption(
        &self,
        context: TenantContext,
        session: SessionTokenHash,
        intent: CurriculumAdoptionApplyIntent,
    ) -> Result<CurriculumAdoptionCompleted, StoreError>;

    /// Loads one bounded, answer-free course import projection.
    ///
    /// Missing baseline, envelope, or receipt evidence is an integrity failure;
    /// implementations never reconstruct authoritative evidence from mutable rows.
    async fn inspect_course_instance_blueprint_adoption(
        &self,
        context: TenantContext,
        session: SessionTokenHash,
        course: CourseReference,
    ) -> Result<Option<CourseInstanceBlueprintInspectionView>, StoreError>;

    /// Rebuilds only B2-owned derived/current-index projections from one completed receipt.
    ///
    /// Implementations require matching immutable receipt, baseline, and envelope evidence.
    /// Missing evidence returns an integrity refusal that keeps the capability unavailable for
    /// operator recovery; no authoritative course, assignment, schedule, learner, grade, source,
    /// baseline, envelope, or receipt is changed.
    async fn reconcile_course_instance_adoption(
        &self,
        context: TenantContext,
        session: SessionTokenHash,
        intent: ReconcileCourseInstanceAdoptionIntent,
    ) -> Result<ReconcileCourseInstanceAdoptionCompleted, StoreError>;
}

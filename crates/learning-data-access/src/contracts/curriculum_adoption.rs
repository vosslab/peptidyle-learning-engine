//! Persistence boundary for explicit B2 curriculum adoption.

use async_trait::async_trait;
use question_model::{
    AlphaInstantiationCommand, AlphaInstantiationCompleted, AlphaInstantiationPreviewRequest,
    AlphaInstantiationPreviewView, AssignmentFastForwardCommand, AssignmentFastForwardCompleted,
    AssignmentFastForwardPreviewRequest, AssignmentFastForwardPreviewView,
    BlueprintInstantiationCommand, BlueprintInstantiationCompleted,
    BlueprintInstantiationPreviewRequest, BlueprintInstantiationPreviewView, CourseReference,
    CourseRolloverCommand, CourseRolloverCompleted, CourseRolloverPreviewRequest,
    CourseRolloverPreviewView, CourseTermShiftCommand, CourseTermShiftCompleted,
    CourseTermShiftPreviewOutcome, CourseTermShiftPreviewRequest,
    CreateSourceDerivedAssignmentCommand, CurriculumAdoptionReconciliationResult,
    CurriculumCourseImportView, ForkAlphaCommand, ForkAlphaCompleted, ForkAlphaPreviewRequest,
    ForkAlphaPreviewView, ReconcileCurriculumAdoptionCommand, SourceDerivedAssignmentCompleted,
    SourceDerivedAssignmentPreviewRequest, SourceDerivedAssignmentPreviewView,
};

use super::{SessionTokenHash, StoreError, TenantContext};

/// Explicit adoption persistence, separate from reusable-source and learner-work Stores.
///
/// The server supplies a validated session token; references are locators only.
/// Implementations re-resolve tenant, actor, source, and destination authority at
/// every boundary. Inputs are closed and bounded (ASVS 1.5.2, 2.2.1, 2.2.2),
/// preview observations bind apply (ASVS 2.3.1), and writes commit atomically
/// (ASVS 2.3.3).
#[async_trait]
pub trait CurriculumAdoptionStore: Send + Sync {
    /// Resolves a current approved Instructor before a route decodes protected input.
    async fn preflight_curriculum_adoption(
        &self,
        context: TenantContext,
        session: SessionTokenHash,
    ) -> Result<(), StoreError>;

    async fn preview_fork_alpha(
        &self,
        context: TenantContext,
        session: SessionTokenHash,
        request: ForkAlphaPreviewRequest,
    ) -> Result<ForkAlphaPreviewView, StoreError>;
    async fn apply_fork_alpha(
        &self,
        context: TenantContext,
        session: SessionTokenHash,
        command: ForkAlphaCommand,
    ) -> Result<ForkAlphaCompleted, StoreError>;

    async fn preview_blueprint_instantiation(
        &self,
        context: TenantContext,
        session: SessionTokenHash,
        request: BlueprintInstantiationPreviewRequest,
    ) -> Result<BlueprintInstantiationPreviewView, StoreError>;
    async fn apply_blueprint_instantiation(
        &self,
        context: TenantContext,
        session: SessionTokenHash,
        command: BlueprintInstantiationCommand,
    ) -> Result<BlueprintInstantiationCompleted, StoreError>;

    async fn preview_alpha_instantiation(
        &self,
        context: TenantContext,
        session: SessionTokenHash,
        request: AlphaInstantiationPreviewRequest,
    ) -> Result<AlphaInstantiationPreviewView, StoreError>;
    async fn apply_alpha_instantiation(
        &self,
        context: TenantContext,
        session: SessionTokenHash,
        command: AlphaInstantiationCommand,
    ) -> Result<AlphaInstantiationCompleted, StoreError>;

    async fn preview_course_rollover(
        &self,
        context: TenantContext,
        session: SessionTokenHash,
        request: CourseRolloverPreviewRequest,
    ) -> Result<CourseRolloverPreviewView, StoreError>;
    async fn apply_course_rollover(
        &self,
        context: TenantContext,
        session: SessionTokenHash,
        command: CourseRolloverCommand,
    ) -> Result<CourseRolloverCompleted, StoreError>;

    async fn preview_course_term_shift(
        &self,
        context: TenantContext,
        session: SessionTokenHash,
        request: CourseTermShiftPreviewRequest,
    ) -> Result<CourseTermShiftPreviewOutcome, StoreError>;
    async fn apply_course_term_shift(
        &self,
        context: TenantContext,
        session: SessionTokenHash,
        command: CourseTermShiftCommand,
    ) -> Result<CourseTermShiftCompleted, StoreError>;

    async fn preview_assignment_fast_forward(
        &self,
        context: TenantContext,
        session: SessionTokenHash,
        request: AssignmentFastForwardPreviewRequest,
    ) -> Result<AssignmentFastForwardPreviewView, StoreError>;
    async fn apply_assignment_fast_forward(
        &self,
        context: TenantContext,
        session: SessionTokenHash,
        command: AssignmentFastForwardCommand,
    ) -> Result<AssignmentFastForwardCompleted, StoreError>;

    async fn preview_source_derived_assignment(
        &self,
        context: TenantContext,
        session: SessionTokenHash,
        request: SourceDerivedAssignmentPreviewRequest,
    ) -> Result<SourceDerivedAssignmentPreviewView, StoreError>;
    async fn create_source_derived_assignment(
        &self,
        context: TenantContext,
        session: SessionTokenHash,
        command: CreateSourceDerivedAssignmentCommand,
    ) -> Result<SourceDerivedAssignmentCompleted, StoreError>;

    /// Loads one bounded, answer-free course import projection.
    ///
    /// Missing baseline, envelope, or receipt evidence is an integrity failure;
    /// implementations never reconstruct authoritative evidence from mutable rows.
    async fn inspect_curriculum_imports(
        &self,
        context: TenantContext,
        session: SessionTokenHash,
        course: CourseReference,
    ) -> Result<Option<CurriculumCourseImportView>, StoreError>;

    /// Rebuilds only B2-owned derived/current-index projections from one completed receipt.
    ///
    /// Implementations require matching immutable receipt, baseline, and envelope evidence.
    /// Missing evidence returns an integrity refusal that keeps the capability unavailable for
    /// operator recovery; no authoritative course, assignment, schedule, learner, grade, source,
    /// baseline, envelope, or receipt is changed.
    async fn reconcile_curriculum_adoption(
        &self,
        context: TenantContext,
        session: SessionTokenHash,
        command: ReconcileCurriculumAdoptionCommand,
    ) -> Result<CurriculumAdoptionReconciliationResult, StoreError>;
}

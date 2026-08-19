//! Backend-neutral course-local item-analysis contract.

use async_trait::async_trait;
use domain::item_analysis::CourseItemAnalysisReport;
use question_model::{AssignmentId, CourseId, LearnerClassStatistics, ScoringGeneration, UserId};

use crate::{JobId, JobLeaseToken, SessionTokenHash, StoreError, TenantContext};

/// Lease- and scoring-generation-fenced analysis rebuild command.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CourseItemAnalysisWorkerCommand {
    pub job: JobId,
    pub lease: JobLeaseToken,
    pub assignment: AssignmentId,
    pub generation: ScoringGeneration,
}

/// Result of atomically publishing one current item-analysis generation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CourseItemAnalysisCommitOutcome {
    Committed,
    Superseded,
    ClaimNoLongerActive,
}

/// Instructor-only read boundary for a tenant-owned, current analysis report.
#[async_trait]
pub trait CourseItemAnalysisStore: Send + Sync {
    /// Returns `None` when the assignment is absent or the authenticated session
    /// lacks direct-instructor authority for that exact course.
    async fn course_item_analysis(
        &self,
        context: TenantContext,
        session: SessionTokenHash,
        course: CourseId,
        assignment: AssignmentId,
    ) -> Result<Option<CourseItemAnalysisReport>, StoreError>;

    /// Returns only the k-anonymity-gated aggregate a currently entitled
    /// learner may be shown. It never returns report, roster, or response
    /// identity, and an absent or stale report is indistinguishable from other
    /// insufficient evidence.
    async fn learner_class_statistics(
        &self,
        context: TenantContext,
        learner: UserId,
        course: CourseId,
        assignment: AssignmentId,
    ) -> Result<LearnerClassStatistics, StoreError>;
}

/// Private staging and atomic-publication boundary for a course item-analysis rebuild.
#[async_trait]
pub trait CourseItemAnalysisWorkerStore: Send + Sync {
    /// Rebuilds private aggregate staging rows without changing current analysis.
    async fn prepare_course_item_analysis(
        &self,
        context: TenantContext,
        command: CourseItemAnalysisWorkerCommand,
    ) -> Result<(), StoreError>;

    /// Conditionally replaces the current analysis and completes the exact queue lease.
    async fn commit_course_item_analysis(
        &self,
        context: TenantContext,
        command: CourseItemAnalysisWorkerCommand,
    ) -> Result<CourseItemAnalysisCommitOutcome, StoreError>;
}

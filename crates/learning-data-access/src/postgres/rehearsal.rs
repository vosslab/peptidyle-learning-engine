//! PostgreSQL implementation of the isolated rehearsal aggregate.
//!
//! This module is intentionally only an orchestration layer.  The sibling
//! modules keep private persistence decoding and SQL capability calls away
//! from route-shaped values.  ASVS 1.5.1, 2.2.1, and 2.3.3 apply: authorize
//! and lock live facts before decoding protected JSON, then verify the locked
//! aggregate before a transition is persisted.

mod auth;
mod claims;
mod completion;
mod execution;
mod frozen;
mod hydration;
mod integrity;
#[cfg(feature = "test-support")]
mod lifecycle;
mod material;
mod operations;
mod route_mutations;
mod rows;
mod source_aggregate;
mod start;

use async_trait::async_trait;
use domain::DispatchedClaimHandle;
use question_model::{AssignmentId, CourseId, CourseMembershipId, RehearsalRunReceipt, TenantId};
use sqlx::{Postgres, Row, Transaction, postgres::PgRow};

use super::*;

/// A closed source scope matching the broker-owned source-fence capability.
/// Every variant carries its course to prevent accidental cross-course scope.
pub(super) enum RehearsalSourceSelector {
    Course {
        course: CourseId,
    },
    Assignment {
        course: CourseId,
        assignment: AssignmentId,
    },
    DirectInstructorMembership {
        course: CourseId,
        membership: CourseMembershipId,
    },
}

/// Opaque proof that matching active rehearsal aggregates were verified.
pub(super) struct LockedRehearsalSourceWitness {
    count: u64,
}

impl LockedRehearsalSourceWitness {
    pub(super) fn database_count(&self) -> Result<i64, StoreError> {
        i64::try_from(self.count).map_err(|_| {
            StoreError::InvalidRecord("locked rehearsal count exceeds database range".into())
        })
    }
}

pub(super) async fn verify_prelocked_source_aggregates(
    tx: &mut Transaction<'_, Postgres>,
    tenant: TenantId,
    selector: RehearsalSourceSelector,
    locked_rehearsal_count: i64,
    locked_rehearsal_run_ids: Vec<sqlx::types::Uuid>,
) -> Result<LockedRehearsalSourceWitness, StoreError> {
    source_aggregate::verify_prelocked_source_aggregates(
        tx,
        tenant,
        selector,
        locked_rehearsal_count,
        locked_rehearsal_run_ids,
    )
    .await
}

/// Typed, non-public witnesses returned by the three broker prepare paths.
/// The UUID list remains Store-internal and is consumed only by canonical
/// aggregate verification before the paired live mutation capability.
pub(super) struct AssignmentRehearsalPrepareWitness {
    revision: i64,
    locked_rehearsal_count: i64,
    locked_rehearsal_run_ids: Vec<sqlx::types::Uuid>,
}

pub(super) struct DirectInstructorRehearsalPrepareWitness {
    roster_revision: i64,
    locked_rehearsal_count: i64,
    locked_rehearsal_run_ids: Vec<sqlx::types::Uuid>,
}

pub(super) struct RetentionRehearsalPrepareWitness {
    generation: i64,
    locked_rehearsal_count: i64,
    locked_rehearsal_run_ids: Vec<sqlx::types::Uuid>,
}

macro_rules! prepare_witness {
    ($name:ident, $scalar:ident, $column:literal) => {
        impl $name {
            pub(super) fn decode(row: &PgRow) -> Result<Self, StoreError> {
                Ok(Self {
                    $scalar: row.try_get($column).map_err(map_sqlx_error)?,
                    locked_rehearsal_count: row
                        .try_get("locked_rehearsal_count")
                        .map_err(map_sqlx_error)?,
                    locked_rehearsal_run_ids: row
                        .try_get("locked_rehearsal_run_ids")
                        .map_err(map_sqlx_error)?,
                })
            }

            pub(super) fn $scalar(&self) -> i64 {
                self.$scalar
            }

            pub(super) async fn verify(
                self,
                tx: &mut Transaction<'_, Postgres>,
                tenant: TenantId,
                selector: RehearsalSourceSelector,
            ) -> Result<LockedRehearsalSourceWitness, StoreError> {
                verify_prelocked_source_aggregates(
                    tx,
                    tenant,
                    selector,
                    self.locked_rehearsal_count,
                    self.locked_rehearsal_run_ids,
                )
                .await
            }
        }
    };
}

prepare_witness!(
    AssignmentRehearsalPrepareWitness,
    revision,
    "assignment_revision"
);
prepare_witness!(
    DirectInstructorRehearsalPrepareWitness,
    roster_revision,
    "roster_revision"
);
prepare_witness!(
    RetentionRehearsalPrepareWitness,
    generation,
    "retention_generation"
);

#[async_trait]
impl crate::contracts::RehearsalInternalStore for PostgresStore {
    async fn start_rehearsal_from_route(
        &self,
        context: TenantContext,
        command: crate::StartRehearsalRouteCommand,
    ) -> Result<crate::StartRehearsalRouteResult, StoreError> {
        start::start_from_route(self, context, command).await
    }

    async fn read_rehearsal_from_route(
        &self,
        context: TenantContext,
        command: crate::ReadRehearsalRouteCommand,
    ) -> Result<RehearsalRunReceipt, StoreError> {
        hydration::read_from_route(self, context, command).await
    }

    async fn read_rehearsal(
        &self,
        context: TenantContext,
        locator: crate::RehearsalLocator,
    ) -> Result<RehearsalRunReceipt, StoreError> {
        hydration::read(self, context, locator).await
    }

    #[cfg(feature = "test-support")]
    async fn claim_rehearsal_submission(
        &self,
        context: TenantContext,
        command: crate::ClaimRehearsalSubmissionCommand,
    ) -> Result<crate::RehearsalSubmissionClaimResult, StoreError> {
        claims::claim(self, context, command).await
    }

    #[cfg(feature = "test-support")]
    async fn complete_rehearsal_submission(
        &self,
        context: TenantContext,
        command: crate::CompleteRehearsalSubmissionCommand,
    ) -> Result<crate::RehearsalSubmissionReceipt, StoreError> {
        completion::complete(self, context, command).await
    }

    async fn mark_rehearsal_submission_dispatched(
        &self,
        context: TenantContext,
        command: crate::MarkRehearsalSubmissionDispatchedCommand,
    ) -> Result<DispatchedClaimHandle, StoreError> {
        claims::mark_dispatched(self, context, command).await
    }

    #[cfg(feature = "test-support")]
    async fn discard_rehearsal(
        &self,
        context: TenantContext,
        locator: crate::RehearsalLocator,
    ) -> Result<RehearsalRunReceipt, StoreError> {
        lifecycle::terminalize(self, context, locator, "discardedByInstructor").await
    }

    #[cfg(feature = "test-support")]
    async fn complete_rehearsal(
        &self,
        context: TenantContext,
        locator: crate::RehearsalLocator,
    ) -> Result<RehearsalRunReceipt, StoreError> {
        lifecycle::terminalize(self, context, locator, "completed").await
    }
}

#[async_trait]
impl crate::RehearsalDeliveryMaterialStore for PostgresStore {
    async fn verify_rehearsal_delivery_material_from_route(
        &self,
        context: TenantContext,
        command: crate::VerifyRehearsalDeliveryMaterialRouteCommand,
    ) -> Result<(), StoreError> {
        material::verify_from_route(self, context, command).await
    }
}

#[async_trait]
impl crate::RehearsalPreDispatchCompensationStore for PostgresStore {
    async fn abandon_rehearsal_submission_before_dispatch(
        &self,
        context: TenantContext,
        command: crate::AbandonRehearsalSubmissionBeforeDispatchCommand,
    ) -> Result<(), StoreError> {
        claims::abandon(self, context, command).await
    }
}

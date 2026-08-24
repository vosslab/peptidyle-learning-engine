//! Immutable rehearsal-material verification for route-scoped delivery.
//!
//! Verification proves frozen source/private siblings are structurally usable.
//! It returns no candidate list or selection capability: the delivery Store
//! chooses the ordinal inside its atomic claim.

use async_trait::async_trait;

use crate::{RehearsalRouteIdentity, StoreError, TenantContext};

/// Public route values accepted by immutable rehearsal-material verification.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VerifyRehearsalDeliveryMaterialRouteCommand {
    pub route: RehearsalRouteIdentity,
}

/// Separate server-only capability for immutable rehearsal source validation.
///
/// This preserves a bounded verification seam for early availability feedback
/// while keeping all progress selection in the atomic Store claim.
#[async_trait]
pub trait RehearsalDeliveryMaterialStore: Send + Sync {
    async fn verify_rehearsal_delivery_material_from_route(
        &self,
        context: TenantContext,
        command: VerifyRehearsalDeliveryMaterialRouteCommand,
    ) -> Result<(), StoreError>;
}

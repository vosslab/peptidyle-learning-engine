//! Explicit tenant context for every educational-record operation (WP-C4).

use question_model::TenantId;

use crate::SessionTokenHash;

/// Tenant identity derived from an authenticated server-side session.
///
/// There is no `Default` implementation. Every tenant-owned store operation
/// requires a context value, so omitting tenancy cannot compile.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TenantContext {
    tenant: TenantId,
    owner_correction_session: Option<SessionTokenHash>,
}

impl TenantContext {
    /// Creates context at the authenticated session boundary.
    ///
    /// Request parameters, headers, and JSON bodies must never call this. The
    /// server auth module is the sole production construction site.
    pub fn from_authenticated_session(tenant: TenantId) -> Self {
        Self {
            tenant,
            owner_correction_session: None,
        }
    }

    /// Tenant supplied to storage and PostgreSQL RLS session state.
    pub fn tenant_id(&self) -> TenantId {
        self.tenant
    }

    /// Attaches the server-resolved session capability for the exceptional
    /// original-owner correction transaction. This capability is opaque and
    /// does not alter the tenant boundary.
    pub(crate) fn with_owner_correction_session(self, session: SessionTokenHash) -> Self {
        Self {
            tenant: self.tenant,
            owner_correction_session: Some(session),
        }
    }

    pub(crate) fn owner_correction_session(self) -> Option<SessionTokenHash> {
        self.owner_correction_session
    }
}

//! Explicit tenant context for every educational-record operation (WP-C4).

use question_model::TenantId;

/// Tenant identity derived from an authenticated server-side session.
///
/// There is no `Default` implementation. Every tenant-owned store operation
/// requires a context value, so omitting tenancy cannot compile.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TenantContext {
    tenant: TenantId,
}

impl TenantContext {
    /// Creates context at the authenticated session boundary.
    ///
    /// Request parameters, headers, and JSON bodies must never call this. The
    /// server auth module is the sole production construction site.
    pub fn from_authenticated_session(tenant: TenantId) -> Self {
        Self { tenant }
    }

    /// Tenant supplied to storage and PostgreSQL RLS session state.
    pub fn tenant_id(&self) -> TenantId {
        self.tenant
    }
}

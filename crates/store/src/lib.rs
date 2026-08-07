//! MOD-STO, MOD-SCHEMA: the `Store` trait and PostgreSQL persistence.
//!
//! Every tenant-owned table carries a tenant ID and declares
//! `FORCE ROW LEVEL SECURITY`. The tenant context comes from the authenticated
//! session, never from a client-supplied parameter, and the application role
//! cannot bypass RLS.

/// In-memory backend used by tests and the M1 conformance suite.
pub mod memory;
/// Cursor-based paging. The trait exposes no `OFFSET` parameter anywhere.
pub mod pagination;
/// PostgreSQL backend and migrations.
pub mod postgres;
/// Row-level security policies and the single tenant-context entry point.
pub mod rls;

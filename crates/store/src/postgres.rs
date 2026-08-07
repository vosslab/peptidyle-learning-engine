//! PostgreSQL backend, migrations, and connection handling (MOD-STO, MOD-SCHEMA).
//!
//! Implemented behind the `postgres` feature so `MemoryStore` builds without a
//! database. The `Store` implementation itself lands in M2; what exists today
//! is the connection path WP-F4 health-checks the container against.
//!
//! Tenancy rules that must survive every later change:
//!
//! - Every tenant-owned table declares `FORCE ROW LEVEL SECURITY`.
//! - The application connects as a non-superuser role that cannot bypass RLS.
//! - The tenant context is a session variable set from the authenticated
//!   session, never a client-supplied parameter. See [`crate::rls`].
//!
//! A test in `tests/e2e/` sets a foreign tenant context and asserts zero rows.
//! If that test ever needs relaxing, the schema is wrong, not the test.

#[cfg(feature = "postgres")]
use sqlx::postgres::{PgPool, PgPoolOptions};

/// The connection pool type, re-exported so callers do not need `sqlx`.
///
/// This crate owns the database driver (see the boundary table in the
/// implementation plan). A server or worker that had to name `sqlx` types
/// directly would be depending on the driver, which is the coupling this alias
/// prevents.
#[cfg(feature = "postgres")]
pub type Pool = PgPool;

/// Builds a lazy connection pool.
///
/// Lazy on purpose: the API server must start and report an honest `degraded`
/// health status when PostgreSQL is not up yet, rather than refusing to boot.
/// An orchestrator can only restart what it can reach.
///
/// # Errors
///
/// Returns an error when `database_url` is not a valid connection string. It
/// does not error on an unreachable database; that surfaces at [`ping`].
#[cfg(feature = "postgres")]
pub fn lazy_pool(database_url: &str) -> Result<PgPool, sqlx::Error> {
    // Small ceiling: this pool serves one replica, and a large pool per replica
    // multiplies into connection exhaustion once replicas scale out.
    let pool = PgPoolOptions::new()
        .max_connections(8)
        .connect_lazy(database_url)?;
    Ok(pool)
}

/// Runs a real query against PostgreSQL.
///
/// This is the health probe. It issues `SELECT 1` rather than inspecting pool
/// state, because a pool can look healthy while the server behind it refuses
/// queries -- which is exactly the failure a health check exists to catch.
///
/// # Errors
///
/// Returns an error when the database is unreachable or rejects the query.
#[cfg(feature = "postgres")]
pub async fn ping(pool: &PgPool) -> Result<(), sqlx::Error> {
    sqlx::query("SELECT 1").execute(pool).await?;
    Ok(())
}

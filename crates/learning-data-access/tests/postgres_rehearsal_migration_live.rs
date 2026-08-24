#![cfg(feature = "postgres")]

//! Disposable PostgreSQL capability oracle for the WP-PROF-T4 rehearsal migration.
//!
//! This target tests the migrated SQL authority boundary directly.  The later
//! `PostgresStore` conformance suite owns canonical domain hashes and decoding.

#[path = "postgres_rehearsal_migration_live/bounds.rs"]
mod bounds;
#[path = "postgres_rehearsal_migration_live/fixture.rs"]
mod fixture;
#[path = "postgres_rehearsal_migration_live/lifecycle.rs"]
mod lifecycle;
#[path = "postgres_rehearsal_migration_live/retention.rs"]
mod retention;
#[path = "postgres_rehearsal_migration_live/security.rs"]
mod security;

#![cfg(feature = "postgres")]

//! Disposable PostgreSQL capability oracle for the WP-PROF-T4 rehearsal migration.
//!
//! This target tests the migrated SQL authority boundary directly.  The later
//! `PostgresStore` conformance suite owns canonical domain hashes and decoding.

// The retired direct-SQL start capability used an empty synthetic assignment.
// T4 evidence now creates ordinary course/catalog/assignment state through
// `PostgresStore` and exercises route start before any SQL fault probe.
#[path = "postgres_rehearsal_store_live.rs"]
mod canonical_store;
#[path = "postgres_rehearsal_migration_live/post_start.rs"]
mod post_start;
#[path = "postgres_rehearsal_migration_live/progression.rs"]
mod progression;
#[path = "postgres_rehearsal_migration_live/security.rs"]
mod security;
#[path = "postgres_rehearsal_migration_live/submission_recovery.rs"]
mod submission_recovery;
#[path = "postgres_rehearsal_migration_live/timing.rs"]
mod timing;

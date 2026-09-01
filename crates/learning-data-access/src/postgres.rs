//! PostgreSQL connection and migration administration for the clean baseline.
//!
//! Feature adapters return only after their exact clean-schema contracts exist.

#[cfg(feature = "postgres")]
mod assignment_attempt;
#[cfg(feature = "postgres")]
mod connection;
#[cfg(feature = "postgres")]
mod migrations;
#[cfg(feature = "postgres")]
mod object_record;
#[cfg(feature = "postgres")]
mod question_source;
#[cfg(feature = "postgres")]
mod sessions;

#[cfg(feature = "postgres")]
pub use assignment_attempt::PostgresAssignmentAttemptStore;
#[cfg(feature = "postgres")]
pub use connection::{ProductionLoginProfile, lazy_pool, local_development_pool, production_pool};
#[cfg(feature = "postgres")]
pub use migrations::{
    MigrationCheck, MigrationCheckEntry, MigrationCheckResult, SchemaCompatibilityError,
    apply_migrations, migration_check, migration_principal, migration_status_from_directory,
    verify_application_schema,
};
#[cfg(feature = "postgres")]
pub use object_record::PostgresWorkspaceQuestionSourceObjectRecordStore;
#[cfg(feature = "postgres")]
pub use question_source::PostgresDraftQuestionSourceStore;
#[cfg(feature = "postgres")]
pub use sessions::PostgresSessionStore;

#[cfg(feature = "postgres")]
pub type Pool = sqlx::postgres::PgPool;

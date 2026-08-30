//! PostgreSQL connection and migration administration for the clean baseline.
//!
//! Feature adapters return only after their exact clean-schema contracts exist.

#[cfg(feature = "postgres")]
mod connection;
#[cfg(feature = "postgres")]
mod migrations;
#[cfg(feature = "postgres")]
mod sessions;

#[cfg(feature = "postgres")]
pub use connection::{
    AcceptedSubmissionFastPathPool, AcceptedSubmissionRecoveryPool, BaseCourseInstallerPool,
    ProductionLoginProfile, accepted_submission_fast_path_pool, accepted_submission_recovery_pool,
    base_course_accepted_submission_fast_path_pool, base_course_application_pool,
    base_course_installer_pool, lazy_pool, local_accepted_submission_fast_path_pool,
    local_accepted_submission_recovery_pool, local_base_course_accepted_submission_fast_path_pool,
    local_base_course_application_pool, local_base_course_installer_pool, local_development_pool,
    production_pool,
};
#[cfg(feature = "postgres")]
pub use migrations::{
    MigrationDisposition, MigrationStatus, MigrationStatusEntry, SchemaCompatibilityError,
    apply_migrations, migration_principal, migration_status, migration_status_from_directory,
    verify_application_schema, verify_invitation_delivery_worker_schema,
    verify_public_asset_publisher_schema,
};
#[cfg(feature = "postgres")]
pub use sessions::PostgresSessionStore;

#[cfg(feature = "postgres")]
pub type Pool = sqlx::postgres::PgPool;

//! Embedded SQLx migration status, administration, and application checks.
//!
//! This module owns the immutable migration epoch and the two deliberately
//! distinct verification paths: privileged ledger inspection for project tools, and
//! the restricted `ple_app` compatibility projection used at application startup.

use std::collections::BTreeMap;
use std::fmt;
use std::path::Path;

use sqlx::Row;
use sqlx::postgres::{PgAdvisoryLock, PgAdvisoryLockKey, PgPool};

use super::connection::is_connection_error;

static MIGRATOR: sqlx::migrate::Migrator = sqlx::migrate!("../../schemas/migrations");

// ASCII `PLE_SCHM` in PostgreSQL's signed 64-bit advisory-lock keyspace. This
// stable project key serializes the complete repository-owned schema epoch,
// including the catalog-derived capability reconciliation after SQLx DDL.
const SCHEMA_EPOCH_LOCK_KEY: i64 = 0x504c_455f_5343_484d;

/// Read-only state of one embedded migration relative to a database.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MigrationCheckResult {
    /// The exact embedded checksum is recorded as successful.
    Applied,
    /// The migration is known to the application but absent from the ledger.
    Pending,
    /// The recorded checksum differs from the immutable embedded migration.
    Changed,
    /// SQLx recorded a failed, partially applied migration.
    Incomplete,
}

/// Status of one migration in the initial database epoch.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MigrationCheckEntry {
    version: i64,
    description: String,
    result: MigrationCheckResult,
}

impl MigrationCheckEntry {
    /// Returns the ordered SQLx migration version.
    pub fn version(&self) -> i64 {
        self.version
    }

    /// Returns the filename-derived migration description.
    pub fn description(&self) -> &str {
        &self.description
    }

    /// Returns the database disposition for this migration.
    pub fn result(&self) -> MigrationCheckResult {
        self.result
    }
}

/// Read-only comparison of the embedded epoch with the SQLx ledger.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MigrationCheck {
    ledger_present: bool,
    entries: Vec<MigrationCheckEntry>,
    unexpected_applied_versions: Vec<i64>,
}

impl MigrationCheck {
    /// Returns whether SQLx has created its authoritative ledger.
    pub fn ledger_present(&self) -> bool {
        self.ledger_present
    }

    /// Returns every known migration in version order.
    pub fn entries(&self) -> &[MigrationCheckEntry] {
        &self.entries
    }

    /// Returns applied versions absent from the embedded immutable epoch.
    pub fn unexpected_applied_versions(&self) -> &[i64] {
        &self.unexpected_applied_versions
    }

    /// Returns true only for an exact, successful, complete epoch.
    pub fn is_compatible(&self) -> bool {
        self.ledger_present
            && self.unexpected_applied_versions.is_empty()
            && self
                .entries
                .iter()
                .all(|entry| entry.result == MigrationCheckResult::Applied)
    }

    fn incompatibility_reason(&self) -> String {
        if !self.ledger_present {
            return "the SQLx migration ledger is absent".to_string();
        }
        if let Some(version) = self.unexpected_applied_versions.first() {
            return format!("applied migration {version} is absent from the embedded epoch");
        }
        if let Some(entry) = self
            .entries
            .iter()
            .find(|entry| entry.result != MigrationCheckResult::Applied)
        {
            let state = match entry.result {
                MigrationCheckResult::Applied => "applied",
                MigrationCheckResult::Pending => "pending",
                MigrationCheckResult::Changed => "changed",
                MigrationCheckResult::Incomplete => "incomplete",
            };
            return format!("migration {} is {state}", entry.version);
        }
        "the database migration state is incompatible".to_string()
    }
}

/// Startup migration verification failure with credential-safe diagnostics.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SchemaCompatibilityError {
    /// PostgreSQL could not be reached, so the stateless API may start degraded.
    Unavailable,
    /// PostgreSQL was reachable but its schema was not the exact embedded epoch.
    Incompatible(String),
}

impl fmt::Display for SchemaCompatibilityError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Unavailable => formatter.write_str("database schema state is unavailable"),
            Self::Incompatible(reason) => {
                write!(formatter, "database schema is incompatible: {reason}")
            }
        }
    }
}

impl std::error::Error for SchemaCompatibilityError {}

#[derive(Clone, Debug)]
struct AppliedMigrationState {
    version: i64,
    success: bool,
    checksum: Vec<u8>,
}

fn evaluate_migration_check(
    migrator: &sqlx::migrate::Migrator,
    ledger_present: bool,
    applied: Vec<AppliedMigrationState>,
) -> MigrationCheck {
    let mut applied_by_version = applied
        .into_iter()
        .map(|migration| (migration.version, migration))
        .collect::<BTreeMap<_, _>>();
    let entries = migrator
        .iter()
        .filter(|migration| !migration.migration_type.is_down_migration())
        .map(|migration| {
            let disposition = match applied_by_version.remove(&migration.version) {
                None => MigrationCheckResult::Pending,
                Some(applied) if !applied.success => MigrationCheckResult::Incomplete,
                Some(applied) if applied.checksum.as_slice() != migration.checksum.as_ref() => {
                    MigrationCheckResult::Changed
                }
                Some(_) => MigrationCheckResult::Applied,
            };
            MigrationCheckEntry {
                version: migration.version,
                description: migration.description.to_string(),
                result: disposition,
            }
        })
        .collect();
    MigrationCheck {
        ledger_present,
        entries,
        unexpected_applied_versions: applied_by_version.into_keys().collect(),
    }
}

fn undefined_relation(error: &sqlx::Error) -> bool {
    matches!(
        error,
        sqlx::Error::Database(database_error)
            if database_error.code().as_deref() == Some("42P01")
    )
}

async fn read_migration_rows(
    pool: &PgPool,
) -> Result<(bool, Vec<AppliedMigrationState>), sqlx::Error> {
    let rows = match sqlx::query(
        "SELECT version, success, checksum FROM public._sqlx_migrations ORDER BY version",
    )
    .fetch_all(pool)
    .await
    {
        Ok(rows) => rows,
        Err(error) if undefined_relation(&error) => return Ok((false, Vec::new())),
        Err(error) => return Err(error),
    };
    let applied = rows
        .into_iter()
        .map(|row| {
            Ok(AppliedMigrationState {
                version: row.try_get("version")?,
                success: row.try_get("success")?,
                checksum: row.try_get("checksum")?,
            })
        })
        .collect::<Result<Vec<_>, sqlx::Error>>()?;
    Ok((true, applied))
}

/// Reports known, pending, dirty, modified, and unexpected migrations without mutation.
///
/// A database with no SQLx ledger is reported as a clean pending epoch so the
/// migration command can explain what it will apply.
///
/// # Errors
///
/// Returns a database error when PostgreSQL is unreachable or the ledger cannot be
/// read safely.
pub async fn migration_check(pool: &PgPool) -> Result<MigrationCheck, sqlx::Error> {
    let (ledger_present, applied) = read_migration_rows(pool).await?;
    Ok(evaluate_migration_check(&MIGRATOR, ledger_present, applied))
}

/// Compares the SQLx ledger with a caller-supplied migration directory.
///
/// This is intentionally read-only. The administrative E2E gate uses it with
/// a disposable copied directory to prove that a changed migration checksum is
/// reported without editing the tracked schema epoch.
///
/// # Errors
///
/// Returns an error when the directory is not a valid SQLx migration source or
/// the database ledger cannot be read safely.
pub async fn migration_status_from_directory(
    pool: &PgPool,
    directory: &Path,
) -> Result<MigrationCheck, sqlx::Error> {
    let migrator = sqlx::migrate::Migrator::new(directory).await?;
    let (ledger_present, applied) = read_migration_rows(pool).await?;
    Ok(evaluate_migration_check(&migrator, ledger_present, applied))
}

/// Verifies the exact application-visible schema epoch through a read-only transaction.
///
/// This deliberately queries the narrow `ple_api.ple_migration_state`
/// projection as `ple_app`; application startup never creates the SQLx ledger
/// or applies DDL.
///
/// # Errors
///
/// Returns [`SchemaCompatibilityError::Unavailable`] when PostgreSQL cannot be
/// reached. A reachable database with a missing projection, rejected app role,
/// unknown version, dirty row, pending migration, or checksum mismatch returns
/// [`SchemaCompatibilityError::Incompatible`].
pub async fn verify_application_schema(pool: &PgPool) -> Result<(), SchemaCompatibilityError> {
    verify_schema_as(pool, SchemaVerificationProfile::Application).await
}

#[derive(Clone, Copy)]
enum SchemaVerificationProfile {
    Application,
}

impl SchemaVerificationProfile {
    const fn role_sql(self) -> &'static str {
        match self {
            Self::Application => "SET LOCAL ROLE ple_app",
        }
    }

    const fn migration_state_sql(self) -> &'static str {
        match self {
            Self::Application => {
                "SELECT version, success, checksum FROM ple_api.ple_migration_state ORDER BY version"
            }
        }
    }
}

async fn verify_schema_as(
    pool: &PgPool,
    profile: SchemaVerificationProfile,
) -> Result<(), SchemaCompatibilityError> {
    let mut transaction = pool
        .begin()
        .await
        .map_err(|_| SchemaCompatibilityError::Unavailable)?;
    sqlx::query("SET TRANSACTION READ ONLY")
        .execute(&mut *transaction)
        .await
        .map_err(|_| SchemaCompatibilityError::Unavailable)?;
    acquire_schema_epoch_shared_lock(&mut transaction).await?;
    sqlx::query(profile.role_sql())
        .execute(&mut *transaction)
        .await
        .map_err(|error| verify_step_error(&error, "the application principal is unavailable"))?;
    let rows = sqlx::query(profile.migration_state_sql())
        .fetch_all(&mut *transaction)
        .await
        .map_err(|error| {
            verify_step_error(&error, "the migration-state projection is unavailable")
        })?;
    let applied = rows
        .into_iter()
        .map(|row| {
            let version = row.try_get("version").map_err(|_| {
                SchemaCompatibilityError::Incompatible(
                    "the migration-state projection has an invalid version".to_string(),
                )
            })?;
            let success = row.try_get("success").map_err(|_| {
                SchemaCompatibilityError::Incompatible(
                    "the migration-state projection has an invalid state".to_string(),
                )
            })?;
            let checksum = row.try_get("checksum").map_err(|_| {
                SchemaCompatibilityError::Incompatible(
                    "the migration-state projection has an invalid checksum".to_string(),
                )
            })?;
            Ok(AppliedMigrationState {
                version,
                success,
                checksum,
            })
        })
        .collect::<Result<Vec<_>, SchemaCompatibilityError>>()?;
    let status = evaluate_migration_check(&MIGRATOR, true, applied);
    if !status.is_compatible() {
        return Err(SchemaCompatibilityError::Incompatible(
            status.incompatibility_reason(),
        ));
    }
    transaction
        .commit()
        .await
        .map_err(|_| SchemaCompatibilityError::Unavailable)?;
    Ok(())
}

fn verify_step_error(error: &sqlx::Error, incompatible: &str) -> SchemaCompatibilityError {
    if is_connection_error(error) {
        SchemaCompatibilityError::Unavailable
    } else {
        SchemaCompatibilityError::Incompatible(incompatible.to_string())
    }
}

async fn acquire_schema_epoch_shared_lock(
    transaction: &mut sqlx::Transaction<'_, sqlx::Postgres>,
) -> Result<(), SchemaCompatibilityError> {
    sqlx::query("SELECT pg_advisory_xact_lock_shared($1)")
        .bind(SCHEMA_EPOCH_LOCK_KEY)
        .execute(&mut **transaction)
        .await
        .map_err(|error| verify_step_error(&error, "the schema epoch lock is unavailable"))?;
    Ok(())
}

/// Applies every embedded, checksummed schema migration in version order.
///
/// # Errors
///
/// Returns a database or migration-integrity failure.
pub async fn apply_migrations(pool: &PgPool) -> Result<(), sqlx::Error> {
    let lock = PgAdvisoryLock::with_key(PgAdvisoryLockKey::BigInt(SCHEMA_EPOCH_LOCK_KEY));
    let connection = pool.acquire().await?;
    let mut guard = lock.acquire(connection).await?;
    let application_result = async {
        MIGRATOR.run(&mut *guard).await?;
        Ok::<(), sqlx::Error>(())
    }
    .await;
    let release_result = guard.release_now().await.map(|_| ());
    match (application_result, release_result) {
        (Err(error), _) => Err(error),
        (Ok(()), Err(error)) => Err(error),
        (Ok(()), Ok(())) => Ok(()),
    }
}

/// Returns the PostgreSQL principal used by the migration connection.
///
/// The caller may use this only for a role-policy decision; it must not log a
/// connection URL or credential material.
pub async fn migration_principal(pool: &PgPool) -> Result<String, sqlx::Error> {
    sqlx::query_scalar("SELECT current_user")
        .fetch_one(pool)
        .await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn application_schema_profile_reads_the_baseline_api_projection() {
        let profile = SchemaVerificationProfile::Application;
        assert_eq!(profile.role_sql(), "SET LOCAL ROLE ple_app");
        assert_eq!(
            profile.migration_state_sql(),
            "SELECT version, success, checksum FROM ple_api.ple_migration_state ORDER BY version"
        );
    }

    fn exact_applied_epoch() -> Vec<AppliedMigrationState> {
        MIGRATOR
            .iter()
            .filter(|migration| !migration.migration_type.is_down_migration())
            .map(|migration| AppliedMigrationState {
                version: migration.version,
                success: true,
                checksum: migration.checksum.to_vec(),
            })
            .collect()
    }

    #[test]
    fn exact_successful_epoch_is_compatible() {
        let status = evaluate_migration_check(&MIGRATOR, true, exact_applied_epoch());
        assert!(status.is_compatible());
        assert!(
            status
                .entries()
                .iter()
                .all(|entry| entry.result() == MigrationCheckResult::Applied)
        );
    }

    #[test]
    fn absent_known_migration_is_pending() {
        let mut applied = exact_applied_epoch();
        let missing = applied.remove(0).version;
        let status = evaluate_migration_check(&MIGRATOR, true, applied);
        assert!(!status.is_compatible());
        assert!(status.entries().iter().any(|entry| {
            entry.version() == missing && entry.result() == MigrationCheckResult::Pending
        }));
    }

    #[test]
    fn checksum_change_is_modified() {
        let mut applied = exact_applied_epoch();
        let modified = applied
            .first_mut()
            .expect("embedded database epoch has a first migration");
        modified.checksum[0] ^= 0xff;
        let version = modified.version;
        let status = evaluate_migration_check(&MIGRATOR, true, applied);
        assert!(status.entries().iter().any(|entry| {
            entry.version() == version && entry.result() == MigrationCheckResult::Changed
        }));
    }

    #[test]
    fn failed_and_unknown_versions_are_incompatible() {
        let mut applied = exact_applied_epoch();
        applied
            .first_mut()
            .expect("embedded database epoch has a first migration")
            .success = false;
        applied.push(AppliedMigrationState {
            version: i64::MAX,
            success: true,
            checksum: vec![0; 48],
        });
        let status = evaluate_migration_check(&MIGRATOR, true, applied);
        assert!(!status.is_compatible());
        assert_eq!(status.unexpected_applied_versions(), &[i64::MAX]);
        assert!(
            status
                .entries()
                .iter()
                .any(|entry| entry.result() == MigrationCheckResult::Incomplete)
        );
    }
}

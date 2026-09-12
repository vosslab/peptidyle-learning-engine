//! Small, read-mostly PostgreSQL schema lifecycle checks.
//!
//! The base installer owns DDL. This module serializes the one database
//! lifecycle coordinator, checks the installed base plus SQLx forward ledger,
//! and exposes the restricted application check.

use std::collections::BTreeMap;
use std::fmt;

use sqlx::Row;
use sqlx::pool::PoolConnection;
use sqlx::postgres::{
    PgAdvisoryLock, PgAdvisoryLockGuard, PgAdvisoryLockKey, PgConnection, PgPool, Postgres,
};

use super::connection::is_connection_error;

static MIGRATOR: sqlx::migrate::Migrator = sqlx::migrate!();

/// ASCII `PLE_SCHM` in PostgreSQL's signed advisory-lock keyspace.
const SCHEMA_LIFECYCLE_LOCK_KEY: i64 = 0x504c_455f_5343_484d;
/// Release identity expected from the immutable base's restricted projection.
///
/// The first approved production cutover changes this value together with the
/// frozen base source; before then every disposable base uses this development
/// identity.
pub const BASE_RELEASE_IDENTITY: &str = "pre-production";
const PRE_PRODUCTION_BASE_RELEASE_IDENTITY: &str = "pre-production";
const FORWARD_LEDGER: &str = "ple_migration._sqlx_migrations";

/// Internal state of one fixed embedded forward migration relative to a database.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ForwardMigrationStatus {
    /// The exact embedded checksum is recorded as successful.
    Applied,
    /// The migration is known to the executable but absent from the ledger.
    Pending,
    /// The recorded checksum differs from the immutable embedded migration.
    Changed,
    /// SQLx recorded a failed, partially applied migration.
    Incomplete,
}

/// Internal comparison of fixed embedded forward migrations with the restricted
/// schema-state projection.
#[derive(Clone, Debug, Eq, PartialEq)]
struct ForwardMigrationCheck {
    entries: Vec<ForwardMigrationStatus>,
    unexpected_applied_versions: Vec<i64>,
}

impl ForwardMigrationCheck {
    fn is_compatible(&self) -> bool {
        self.unexpected_applied_versions.is_empty()
            && self
                .entries
                .iter()
                .all(|status| *status == ForwardMigrationStatus::Applied)
    }

    fn incompatibility_reason(&self) -> &'static str {
        if !self.unexpected_applied_versions.is_empty() {
            return "the forward migration ledger has an unknown version";
        }
        if self.entries.contains(&ForwardMigrationStatus::Changed) {
            return "a forward migration checksum differs";
        }
        if self.entries.contains(&ForwardMigrationStatus::Incomplete) {
            return "a forward migration is incomplete";
        }
        "a required forward migration is pending"
    }
}

/// Startup compatibility failure with credential-safe diagnostics.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SchemaCompatibilityError {
    /// PostgreSQL could not be reached.
    Unavailable,
    /// PostgreSQL was reachable but does not provide the exact expected state.
    Incompatible(&'static str),
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

#[derive(Clone, Debug, Eq, PartialEq)]
struct AppliedMigrationState {
    version: i64,
    success: bool,
    checksum: Vec<u8>,
}

#[derive(Debug)]
struct SchemaStateRow {
    base_release: String,
    version: Option<i64>,
    success: Option<bool>,
    checksum: Option<Vec<u8>>,
}

fn evaluate_forward_migration_check(
    migrator: &sqlx::migrate::Migrator,
    applied: Vec<AppliedMigrationState>,
) -> ForwardMigrationCheck {
    let mut applied_by_version = applied
        .into_iter()
        .map(|migration| (migration.version, migration))
        .collect::<BTreeMap<_, _>>();
    let entries = migrator
        .iter()
        .filter(|migration| !migration.migration_type.is_down_migration())
        .map(
            |migration| match applied_by_version.remove(&migration.version) {
                None => ForwardMigrationStatus::Pending,
                Some(applied) if !applied.success => ForwardMigrationStatus::Incomplete,
                Some(applied) if applied.checksum.as_slice() != migration.checksum.as_ref() => {
                    ForwardMigrationStatus::Changed
                }
                Some(_) => ForwardMigrationStatus::Applied,
            },
        )
        .collect();
    ForwardMigrationCheck {
        entries,
        unexpected_applied_versions: applied_by_version.into_keys().collect(),
    }
}

async fn read_schema_state(
    connection: &mut PgConnection,
) -> Result<Vec<SchemaStateRow>, sqlx::Error> {
    sqlx::query(
        "SELECT base_release, version, success, checksum \
           FROM ple_api.ple_schema_state ORDER BY version NULLS FIRST",
    )
    .fetch_all(&mut *connection)
    .await?
    .into_iter()
    .map(|row| {
        Ok(SchemaStateRow {
            base_release: row.try_get("base_release")?,
            version: row.try_get("version")?,
            success: row.try_get("success")?,
            checksum: row.try_get("checksum")?,
        })
    })
    .collect()
}

fn schema_state_applied(rows: &[SchemaStateRow]) -> Option<Vec<AppliedMigrationState>> {
    if rows.is_empty()
        || rows
            .iter()
            .any(|row| row.base_release != BASE_RELEASE_IDENTITY)
    {
        return None;
    }
    let null_rows = rows.iter().filter(|row| row.version.is_none()).count();
    if null_rows > 0 {
        if rows.len() != 1 || rows[0].success.is_some() || rows[0].checksum.is_some() {
            return None;
        }
        return Some(Vec::new());
    }
    rows.iter()
        .map(
            |row| match (row.version, row.success, row.checksum.clone()) {
                (Some(version), Some(success), Some(checksum)) => Some(AppliedMigrationState {
                    version,
                    success,
                    checksum,
                }),
                _ => None,
            },
        )
        .collect()
}

fn validate_base_projection(
    rows: &[SchemaStateRow],
) -> Result<Vec<AppliedMigrationState>, SchemaCompatibilityError> {
    schema_state_applied(rows).ok_or(SchemaCompatibilityError::Incompatible(
        "the schema-state projection is malformed",
    ))
}

fn base_is_pre_production(rows: &[SchemaStateRow]) -> Result<bool, SchemaCompatibilityError> {
    validate_base_projection(rows)?;
    Ok(release_is_pre_production(&rows[0].base_release))
}

fn release_is_pre_production(release: &str) -> bool {
    release == PRE_PRODUCTION_BASE_RELEASE_IDENTITY
}

fn verify_schema_state(rows: &[SchemaStateRow]) -> Result<(), SchemaCompatibilityError> {
    let applied = validate_base_projection(rows)?;
    let status = evaluate_forward_migration_check(&MIGRATOR, applied);
    if status.is_compatible() {
        Ok(())
    } else {
        Err(SchemaCompatibilityError::Incompatible(
            status.incompatibility_reason(),
        ))
    }
}

fn forward_migrator() -> sqlx::migrate::Migrator {
    let migrations = MIGRATOR.iter().cloned().collect();
    let mut migrator = sqlx::migrate::Migrator::with_migrations(migrations);
    migrator.dangerous_set_table_name(FORWARD_LEDGER);
    migrator.set_locking(false);
    migrator
}

/// One exclusive session lock held across base installation, forward migration,
/// and privileged verification.
pub struct SchemaLifecycleGuard {
    connection: PgAdvisoryLockGuard<PoolConnection<Postgres>>,
}

impl SchemaLifecycleGuard {
    /// Acquires the existing stable `PLE_SCHM` exclusive session advisory lock.
    ///
    /// Hold this value while an external `psql` base install runs; the lock stays
    /// live on this connection even though `psql` uses a separate connection.
    pub async fn acquire(pool: &PgPool) -> Result<Self, sqlx::Error> {
        let lock = PgAdvisoryLock::with_key(PgAdvisoryLockKey::BigInt(SCHEMA_LIFECYCLE_LOCK_KEY));
        let connection = pool.acquire().await?;
        Ok(Self {
            connection: lock.acquire(connection).await?,
        })
    }

    /// Returns whether the base's application-visible state projection exists.
    ///
    /// The base installer is the source of truth for an uninstalled dedicated
    /// database. A present projection is validated before SQLx is permitted to
    /// write its qualified forward ledger.
    pub async fn base_is_installed(&mut self) -> Result<bool, sqlx::Error> {
        sqlx::query_scalar("SELECT to_regclass('ple_api.ple_schema_state') IS NOT NULL")
            .fetch_one(&mut *self.connection)
            .await
    }

    /// Checks the base release identity and projection shape before forward writes.
    pub async fn verify_base_projection(&mut self) -> Result<(), SchemaCompatibilityError> {
        let rows = read_schema_state(&mut self.connection)
            .await
            .map_err(|error| {
                verify_step_error(&error, "the schema-state projection is unavailable")
            })?;
        validate_base_projection(&rows).map(|_| ())
    }

    /// Returns whether this database still uses the editable pre-production base.
    pub async fn base_is_pre_production(&mut self) -> Result<bool, SchemaCompatibilityError> {
        let rows = read_schema_state(&mut self.connection)
            .await
            .map_err(|error| {
                verify_step_error(&error, "the schema-state projection is unavailable")
            })?;
        base_is_pre_production(&rows)
    }

    /// Applies only known forward migrations while this guard supplies mutual exclusion.
    ///
    /// The base installer creates the exact qualified ledger. SQLx's own lock is
    /// disabled because this session already holds `PLE_SCHM`.
    pub async fn apply_forward_migrations(&mut self) -> Result<(), SchemaCompatibilityError> {
        if self.base_is_pre_production().await? {
            return Err(SchemaCompatibilityError::Incompatible(
                "forward migrations are unavailable while the base release is pre-production",
            ));
        }
        forward_migrator()
            .run_direct(None, &mut *self.connection, false)
            .await
            .map_err(|error| {
                verify_step_error(
                    &error.into(),
                    "recognized forward migrations could not be applied",
                )
            })
    }

    /// Returns the connected administration principal without exposing credentials.
    pub async fn migration_principal(&mut self) -> Result<String, sqlx::Error> {
        sqlx::query_scalar("SELECT current_user")
            .fetch_one(&mut *self.connection)
            .await
    }

    /// Verifies base identity, projection, and forward ledger on this locked connection.
    pub async fn verify_privileged(&mut self) -> Result<(), SchemaCompatibilityError> {
        let rows = read_schema_state(&mut self.connection)
            .await
            .map_err(|error| {
                verify_step_error(&error, "the schema-state projection is unavailable")
            })?;
        verify_schema_state(&rows)
    }
}

/// Acquires the exclusive session guard used by the one administrative coordinator.
pub async fn acquire_schema_lifecycle(pool: &PgPool) -> Result<SchemaLifecycleGuard, sqlx::Error> {
    SchemaLifecycleGuard::acquire(pool).await
}

/// Verifies application-visible schema state through the restricted projection.
/// Application startup has no DDL, ledger-write, or repair path.
pub async fn verify_application_schema(pool: &PgPool) -> Result<(), SchemaCompatibilityError> {
    let mut transaction = pool
        .begin()
        .await
        .map_err(|_| SchemaCompatibilityError::Unavailable)?;
    sqlx::query("SET TRANSACTION READ ONLY")
        .execute(&mut *transaction)
        .await
        .map_err(|_| SchemaCompatibilityError::Unavailable)?;
    // ASVS 8.3.1: application startup verifies only its restricted projection.
    sqlx::query("SELECT pg_advisory_xact_lock_shared($1)")
        .bind(SCHEMA_LIFECYCLE_LOCK_KEY)
        .execute(&mut *transaction)
        .await
        .map_err(|error| verify_step_error(&error, "the schema lifecycle lock is unavailable"))?;
    sqlx::query("SET LOCAL ROLE ple_app")
        .execute(&mut *transaction)
        .await
        .map_err(|error| verify_step_error(&error, "the application principal is unavailable"))?;
    let rows = sqlx::query(
        "SELECT base_release, version, success, checksum \
           FROM ple_api.ple_schema_state ORDER BY version NULLS FIRST",
    )
    .fetch_all(&mut *transaction)
    .await
    .map_err(|error| verify_step_error(&error, "the schema-state projection is unavailable"))?
    .into_iter()
    .map(|row| {
        Ok(SchemaStateRow {
            base_release: row.try_get("base_release").map_err(|_| {
                SchemaCompatibilityError::Incompatible("the schema-state projection is malformed")
            })?,
            version: row.try_get("version").map_err(|_| {
                SchemaCompatibilityError::Incompatible("the schema-state projection is malformed")
            })?,
            success: row.try_get("success").map_err(|_| {
                SchemaCompatibilityError::Incompatible("the schema-state projection is malformed")
            })?,
            checksum: row.try_get("checksum").map_err(|_| {
                SchemaCompatibilityError::Incompatible("the schema-state projection is malformed")
            })?,
        })
    })
    .collect::<Result<Vec<_>, SchemaCompatibilityError>>()?;
    verify_schema_state(&rows)?;
    transaction
        .commit()
        .await
        .map_err(|_| SchemaCompatibilityError::Unavailable)
}

fn verify_step_error(error: &sqlx::Error, incompatible: &'static str) -> SchemaCompatibilityError {
    if is_connection_error(error) {
        SchemaCompatibilityError::Unavailable
    } else {
        SchemaCompatibilityError::Incompatible(incompatible)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::borrow::Cow;

    fn exact_applied_epoch(migrator: &sqlx::migrate::Migrator) -> Vec<AppliedMigrationState> {
        migrator
            .iter()
            .filter(|migration| !migration.migration_type.is_down_migration())
            .map(|migration| AppliedMigrationState {
                version: migration.version,
                success: true,
                checksum: migration.checksum.to_vec(),
            })
            .collect()
    }

    fn test_migrator() -> sqlx::migrate::Migrator {
        sqlx::migrate::Migrator::with_migrations(vec![sqlx::migrate::Migration::new(
            1,
            Cow::Borrowed("test forward"),
            sqlx::migrate::MigrationType::Simple,
            sqlx::SqlStr::from_static("SELECT 1"),
            false,
        )])
    }

    #[test]
    fn exact_successful_forward_epoch_is_compatible() {
        let migrator = test_migrator();
        let status = evaluate_forward_migration_check(&migrator, exact_applied_epoch(&migrator));
        assert!(status.is_compatible());
    }

    #[test]
    fn pending_forward_migration_is_not_compatible() {
        let migrator = test_migrator();
        let status = evaluate_forward_migration_check(&migrator, Vec::new());
        assert!(!status.is_compatible());
    }

    #[test]
    fn changed_incomplete_and_unknown_versions_are_invalid() {
        let migrator = test_migrator();
        let mut applied = exact_applied_epoch(&migrator);
        let changed = applied.first_mut().expect("embedded migration exists");
        changed.checksum[0] ^= 0xff;
        applied.push(AppliedMigrationState {
            version: i64::MAX,
            success: false,
            checksum: vec![0; 32],
        });
        let status = evaluate_forward_migration_check(&migrator, applied);
        assert_eq!(
            status.incompatibility_reason(),
            "the forward migration ledger has an unknown version"
        );
    }

    #[test]
    fn schema_state_allows_one_null_forward_row_for_zero_forward_files() {
        let rows = vec![SchemaStateRow {
            base_release: BASE_RELEASE_IDENTITY.to_string(),
            version: None,
            success: None,
            checksum: None,
        }];
        assert!(schema_state_applied(&rows).unwrap().is_empty());
    }

    #[test]
    fn schema_state_rejects_mixed_or_wrong_base_rows() {
        let mixed = vec![
            SchemaStateRow {
                base_release: BASE_RELEASE_IDENTITY.to_string(),
                version: None,
                success: None,
                checksum: None,
            },
            SchemaStateRow {
                base_release: BASE_RELEASE_IDENTITY.to_string(),
                version: Some(1),
                success: Some(true),
                checksum: Some(vec![0]),
            },
        ];
        assert!(schema_state_applied(&mixed).is_none());
        assert!(
            schema_state_applied(&[SchemaStateRow {
                base_release: "wrong".to_string(),
                version: None,
                success: None,
                checksum: None,
            }])
            .is_none()
        );
    }

    #[test]
    fn pre_production_base_disables_forward_migrations() {
        let rows = vec![SchemaStateRow {
            base_release: BASE_RELEASE_IDENTITY.to_string(),
            version: None,
            success: None,
            checksum: None,
        }];
        assert!(base_is_pre_production(&rows).unwrap());
    }

    #[test]
    fn frozen_base_enables_forward_migrations() {
        assert!(!release_is_pre_production("production-baseline"));
    }
}

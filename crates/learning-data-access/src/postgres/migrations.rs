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

mod base_course_freshness;
use base_course_freshness::{
    RECONCILIATION_SQL as BASE_COURSE_FRESHNESS_RECONCILIATION_SQL,
    is_compatible as base_course_freshness_is_compatible,
};

static MIGRATOR: sqlx::migrate::Migrator = sqlx::migrate!("../../schemas/migrations");

// ASCII `PLE_SCHM` in PostgreSQL's signed 64-bit advisory-lock keyspace. This
// stable project key serializes the complete repository-owned schema epoch,
// including the catalog-derived capability reconciliation after SQLx DDL.
const SCHEMA_EPOCH_LOCK_KEY: i64 = 0x504c_455f_5343_484d;

/// Read-only state of one embedded migration relative to a database.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MigrationDisposition {
    /// The exact embedded checksum is recorded as successful.
    Applied,
    /// The migration is known to the application but absent from the ledger.
    Pending,
    /// The recorded checksum differs from the immutable embedded migration.
    Modified,
    /// SQLx recorded a failed, partially applied migration.
    Dirty,
}

/// Status of one migration in the initial database epoch.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MigrationStatusEntry {
    version: i64,
    description: String,
    disposition: MigrationDisposition,
}

impl MigrationStatusEntry {
    /// Returns the ordered SQLx migration version.
    pub fn version(&self) -> i64 {
        self.version
    }

    /// Returns the filename-derived migration description.
    pub fn description(&self) -> &str {
        &self.description
    }

    /// Returns the database disposition for this migration.
    pub fn disposition(&self) -> MigrationDisposition {
        self.disposition
    }
}

/// Read-only comparison of the embedded epoch with the SQLx ledger.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MigrationStatus {
    ledger_present: bool,
    entries: Vec<MigrationStatusEntry>,
    unexpected_applied_versions: Vec<i64>,
}

impl MigrationStatus {
    /// Returns whether SQLx has created its authoritative ledger.
    pub fn ledger_present(&self) -> bool {
        self.ledger_present
    }

    /// Returns every known migration in version order.
    pub fn entries(&self) -> &[MigrationStatusEntry] {
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
                .all(|entry| entry.disposition == MigrationDisposition::Applied)
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
            .find(|entry| entry.disposition != MigrationDisposition::Applied)
        {
            let state = match entry.disposition {
                MigrationDisposition::Applied => "applied",
                MigrationDisposition::Pending => "pending",
                MigrationDisposition::Modified => "modified",
                MigrationDisposition::Dirty => "dirty",
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

fn evaluate_migration_status(
    migrator: &sqlx::migrate::Migrator,
    ledger_present: bool,
    applied: Vec<AppliedMigrationState>,
) -> MigrationStatus {
    let mut applied_by_version = applied
        .into_iter()
        .map(|migration| (migration.version, migration))
        .collect::<BTreeMap<_, _>>();
    let entries = migrator
        .iter()
        .filter(|migration| !migration.migration_type.is_down_migration())
        .map(|migration| {
            let disposition = match applied_by_version.remove(&migration.version) {
                None => MigrationDisposition::Pending,
                Some(applied) if !applied.success => MigrationDisposition::Dirty,
                Some(applied) if applied.checksum.as_slice() != migration.checksum.as_ref() => {
                    MigrationDisposition::Modified
                }
                Some(_) => MigrationDisposition::Applied,
            };
            MigrationStatusEntry {
                version: migration.version,
                description: migration.description.to_string(),
                disposition,
            }
        })
        .collect();
    MigrationStatus {
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
pub async fn migration_status(pool: &PgPool) -> Result<MigrationStatus, sqlx::Error> {
    let (ledger_present, applied) = read_migration_rows(pool).await?;
    Ok(evaluate_migration_status(
        &MIGRATOR,
        ledger_present,
        applied,
    ))
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
) -> Result<MigrationStatus, sqlx::Error> {
    let migrator = sqlx::migrate::Migrator::new(directory).await?;
    let (ledger_present, applied) = read_migration_rows(pool).await?;
    Ok(evaluate_migration_status(
        &migrator,
        ledger_present,
        applied,
    ))
}

/// Verifies the exact application-visible schema epoch through a read-only transaction.
///
/// This deliberately queries the narrow `ple_migration_state` projection as
/// `ple_app`; application startup never creates the SQLx ledger or applies DDL.
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

/// Verifies the current public relation catalog has the exact sealed Base Course freshness graph.
///
/// This administrative verifier is read-only. It is distinct from application startup because it
/// inspects capability metadata unavailable to the restricted application principal.
///
/// # Errors
///
/// Returns [`SchemaCompatibilityError::Unavailable`] when PostgreSQL cannot be reached and
/// [`SchemaCompatibilityError::Incompatible`] when the capability graph has drifted.
pub async fn verify_base_course_freshness_capability(
    pool: &PgPool,
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
    let compatible = base_course_freshness_is_compatible(&mut *transaction)
        .await
        .map_err(|error| {
            verify_step_error(
                &error,
                "the Base Course freshness capability is unavailable",
            )
        })?;
    if !compatible {
        return Err(SchemaCompatibilityError::Incompatible(
            "the Base Course freshness capability is incompatible".to_string(),
        ));
    }
    transaction
        .commit()
        .await
        .map_err(|_| SchemaCompatibilityError::Unavailable)?;
    Ok(())
}

/// Verifies the exact embedded schema through the publisher's metadata-only
/// capability. The publisher cannot assume `ple_app` merely to run a startup
/// check.
pub async fn verify_public_asset_publisher_schema(
    pool: &PgPool,
) -> Result<(), SchemaCompatibilityError> {
    verify_schema_as(pool, SchemaVerificationProfile::PublicAssetPublisher).await
}

/// Verifies the exact embedded schema through the invitation-delivery
/// worker's function-only capability. It cannot read the migration projection
/// or application tables directly.
pub async fn verify_invitation_delivery_worker_schema(
    pool: &PgPool,
) -> Result<(), SchemaCompatibilityError> {
    verify_schema_as(pool, SchemaVerificationProfile::InvitationDeliveryWorker).await
}

#[derive(Clone, Copy)]
enum SchemaVerificationProfile {
    Application,
    PublicAssetPublisher,
    InvitationDeliveryWorker,
}

impl SchemaVerificationProfile {
    const fn role_sql(self) -> &'static str {
        match self {
            Self::Application => "SET LOCAL ROLE ple_app",
            Self::PublicAssetPublisher => "SET LOCAL ROLE ple_public_asset_publisher",
            Self::InvitationDeliveryWorker => "SET LOCAL ROLE ple_invitation_delivery_worker",
        }
    }

    const fn migration_state_sql(self) -> &'static str {
        match self {
            Self::Application => {
                "SELECT version, success, checksum FROM public.ple_migration_state ORDER BY version"
            }
            Self::PublicAssetPublisher => {
                "SELECT version, success, checksum \
                 FROM public.ple_public_asset_publisher_migration_state() ORDER BY version"
            }
            Self::InvitationDeliveryWorker => {
                "SELECT version, success, checksum \
                 FROM public.ple_invitation_delivery_worker_migration_state() ORDER BY version"
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
    let status = evaluate_migration_status(&MIGRATOR, true, applied);
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
        if !base_course_freshness_is_compatible(&mut *guard).await? {
            sqlx::raw_sql(BASE_COURSE_FRESHNESS_RECONCILIATION_SQL)
                .execute(&mut *guard)
                .await?;
            if !base_course_freshness_is_compatible(&mut *guard).await? {
                return Err(sqlx::Error::Protocol(
                    "Base Course freshness reconciliation did not restore the required catalog graph"
                        .to_string(),
                ));
            }
        }
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
    fn invitation_delivery_worker_schema_profile_is_function_only() {
        let profile = SchemaVerificationProfile::InvitationDeliveryWorker;
        assert_eq!(
            profile.role_sql(),
            "SET LOCAL ROLE ple_invitation_delivery_worker"
        );
        assert_eq!(
            profile.migration_state_sql(),
            "SELECT version, success, checksum \
                 FROM public.ple_invitation_delivery_worker_migration_state() ORDER BY version"
        );
    }

    #[test]
    fn base_course_freshness_registration_is_catalog_derived_and_repeated() {
        let migration = MIGRATOR
            .iter()
            .find(|migration| migration.version == 2026081835)
            .expect("Base Course freshness registration migration is embedded")
            .sql
            .as_ref();
        assert!(
            migration.contains("FROM pg_catalog.pg_class AS table_row")
                && migration.contains("table_row.relkind IN ('r', 'p')")
                && migration.contains("GRANT SELECT, MAINTAIN ON TABLE")
                && migration.contains("CREATE POLICY ple_base_course_freshness_select")
                && migration.contains("NOLOGIN NOSUPERUSER NOCREATEDB NOCREATEROLE NOINHERIT NOREPLICATION NOBYPASSRLS")
                && migration.contains("REVOKE ALL PRIVILEGES ON ALL SEQUENCES IN SCHEMA public"),
            "freshness registration derives its complete relation graph from the live catalog"
        );
        assert!(
            BASE_COURSE_FRESHNESS_RECONCILIATION_SQL.contains("DROP POLICY %I ON %I.%I")
                && base_course_freshness::VERIFICATION_SQL.contains("expected_relation_privileges")
                && base_course_freshness::VERIFICATION_SQL.contains("expected_policies"),
            "reconciliation and administrative verification share one exact catalog graph"
        );
    }

    #[test]
    fn invitation_delivery_worker_migration_projection_is_execute_only() {
        let sql = MIGRATOR
            .iter()
            .find(|migration| migration.version == 2026081502)
            .expect("invitation delivery migration is embedded")
            .sql
            .as_ref();
        assert!(
            sql.contains("CREATE OR REPLACE FUNCTION public.ple_invitation_delivery_worker_migration_state()")
                && sql.contains("LANGUAGE sql SECURITY DEFINER")
                && sql.contains("SELECT version, success, checksum FROM public.ple_migration_state ORDER BY version")
                && sql.contains("ALTER FUNCTION public.ple_invitation_delivery_worker_migration_state()\n    OWNER TO ple_invitation_delivery_broker;")
                && sql.contains("REVOKE ALL ON FUNCTION public.ple_invitation_delivery_worker_migration_state() FROM PUBLIC, ple_app;")
                && sql.contains("GRANT SELECT ON public.ple_migration_state TO ple_invitation_delivery_broker;")
                && sql.contains("GRANT EXECUTE ON FUNCTION public.ple_invitation_delivery_worker_migration_state()\n    TO ple_invitation_delivery_worker;")
                && !sql.contains("GRANT SELECT ON public.ple_migration_state TO ple_invitation_delivery_worker;"),
            "worker compatibility checks must use the broker-owned function only"
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
        let status = evaluate_migration_status(&MIGRATOR, true, exact_applied_epoch());
        assert!(status.is_compatible());
        assert!(
            status
                .entries()
                .iter()
                .all(|entry| entry.disposition() == MigrationDisposition::Applied)
        );
    }

    #[test]
    fn absent_known_migration_is_pending() {
        let mut applied = exact_applied_epoch();
        let missing = applied.remove(0).version;
        let status = evaluate_migration_status(&MIGRATOR, true, applied);
        assert!(!status.is_compatible());
        assert!(status.entries().iter().any(|entry| {
            entry.version() == missing && entry.disposition() == MigrationDisposition::Pending
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
        let status = evaluate_migration_status(&MIGRATOR, true, applied);
        assert!(status.entries().iter().any(|entry| {
            entry.version() == version && entry.disposition() == MigrationDisposition::Modified
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
        let status = evaluate_migration_status(&MIGRATOR, true, applied);
        assert!(!status.is_compatible());
        assert_eq!(status.unexpected_applied_versions(), &[i64::MAX]);
        assert!(
            status
                .entries()
                .iter()
                .any(|entry| entry.disposition() == MigrationDisposition::Dirty)
        );
    }

    #[test]
    fn draft_qti_cleanup_locks_authority_before_narrow_job_deletion() {
        let sql = MIGRATOR
            .iter()
            .find(|migration| migration.version == 2026080805)
            .expect("operations migration is embedded")
            .sql
            .as_ref();
        let start = sql
            .find("CREATE FUNCTION public.ple_delete_draft_qti_jobs(")
            .expect("draft QTI cleanup capability is present");
        let end = sql[start..]
            .find("CREATE FUNCTION public.ple_commit_export_job(")
            .map(|offset| start + offset)
            .expect("cleanup capability has a bounded definition");
        let capability = &sql[start..end];

        assert!(
            capability
                .find("FOR UPDATE OF draft")
                .is_some_and(|lock| capability
                    .find("DELETE FROM public.worker_job AS job")
                    .is_some_and(|delete| lock < delete))
                && capability.contains("p_tenant <> public.ple_current_tenant()")
                && capability.contains("draft.revision = p_expected_revision")
                && capability.contains("access.user_id = p_actor")
                && capability.contains("access.role = 'owner'"),
            "tenant, owner, and revision authority must lock before job deletion"
        );
        assert!(
            capability.contains("job.tenant_id = p_tenant")
                && capability.contains("job.payload ->> 'kind' = 'qtiImport'")
                && capability.contains("job.payload ->> 'workspace' = p_workspace::text"),
            "cleanup must delete only the exact tenant/workspace QTI job family"
        );
        let staged_deletes = [
            "DELETE FROM public.workspace_qti_profile_item_evidence AS evidence",
            "DELETE FROM public.workspace_qti_profile_import_evidence AS evidence",
            "DELETE FROM public.workspace_qti_import_grading AS grading",
            "DELETE FROM public.workspace_qti_import_asset AS asset",
            "DELETE FROM public.workspace_qti_import_result AS result",
            "DELETE FROM public.workspace_qti_import_unsupported AS unsupported",
            "DELETE FROM public.workspace_qti_import_item AS item",
            "DELETE FROM public.workspace_qti_import AS import_row",
            "DELETE FROM public.worker_job AS job",
        ];
        assert!(
            staged_deletes.windows(2).all(|pair| {
                capability
                    .find(pair[0])
                    .zip(capability.find(pair[1]))
                    .is_some_and(|(before, after)| before < after)
            }) && capability.contains("import_row.state = 'prepared'"),
            "draft cleanup must remove the prepared QTI graph in FK order before its jobs"
        );
    }

    #[test]
    fn draft_qti_cleanup_is_a_least_privilege_execute_only_capability() {
        let sql = MIGRATOR
            .iter()
            .find(|migration| migration.version == 2026080805)
            .expect("operations migration is embedded")
            .sql
            .as_ref();

        assert!(
            sql.contains("LANGUAGE plpgsql SECURITY DEFINER\n    SET search_path TO 'pg_catalog', 'public'")
                && sql.contains(
                "ALTER FUNCTION public.ple_delete_draft_qti_jobs(uuid, uuid, uuid, bigint)\n    OWNER TO ple_qti_provenance_broker;"
            ) && sql.contains(
                "REVOKE ALL ON FUNCTION public.ple_delete_draft_qti_jobs(\n    p_tenant uuid,\n    p_workspace uuid,\n    p_actor uuid,\n    p_expected_revision bigint\n) FROM PUBLIC;"
            ) && sql.contains(
                "GRANT EXECUTE ON FUNCTION public.ple_delete_draft_qti_jobs(\n    p_tenant uuid,\n    p_workspace uuid,\n    p_actor uuid,\n    p_expected_revision bigint\n) TO ple_app;"
            ),
            "only the application capability may invoke the NOLOGIN provenance broker"
        );
        assert!(
            sql.contains(
                "GRANT SELECT,DELETE ON TABLE public.worker_job TO ple_qti_provenance_broker;"
            ) && sql.contains(
                "CREATE POLICY worker_job_qti_provenance_select ON public.worker_job FOR SELECT\n    TO ple_qti_provenance_broker\n    USING ((tenant_id = public.ple_current_tenant()) AND (payload ->> 'kind' = 'qtiImport'));"
            ) && sql.contains(
                "CREATE POLICY worker_job_qti_provenance_delete ON public.worker_job FOR DELETE\n    TO ple_qti_provenance_broker\n    USING ((tenant_id = public.ple_current_tenant()) AND (payload ->> 'kind' = 'qtiImport'));"
            ) && !sql.contains("GRANT SELECT,INSERT,DELETE ON TABLE public.worker_job TO ple_app;"),
            "ple_app must not receive direct worker-job deletion"
        );
        for table in [
            "workspace_qti_import",
            "workspace_qti_import_asset",
            "workspace_qti_import_grading",
            "workspace_qti_import_item",
            "workspace_qti_import_result",
            "workspace_qti_profile_import_evidence",
            "workspace_qti_profile_item_evidence",
            "workspace_qti_import_unsupported",
        ] {
            assert!(
                sql.contains(&format!(
                    "GRANT DELETE ON TABLE public.{table} TO ple_qti_provenance_broker;"
                )) && sql.contains(&format!("CREATE POLICY {table}_prepared_delete")),
                "prepared QTI cleanup must have a narrow broker-only delete path for {table}"
            );
        }
    }

    #[test]
    fn qti_profile_evidence_must_be_complete_before_recognized_import_commit() {
        let sql = MIGRATOR
            .iter()
            .find(|migration| migration.version == 2026080805)
            .expect("operations migration is embedded")
            .sql
            .as_ref();
        let start = sql
            .find("CREATE FUNCTION public.ple_commit_prepared_qti_import(")
            .expect("QTI commit capability is present");
        let end = sql[start..]
            .find("CREATE FUNCTION public.ple_complete_worker_job(")
            .map(|offset| start + offset)
            .expect("QTI commit capability has a bounded definition");
        let capability = &sql[start..end];

        assert!(
            capability.contains("NOT (registry.payload ? 'profileSummary')")
                && capability.contains("workspace_qti_profile_import_evidence AS profile")
                && capability.contains("profile.profile_report_sha256")
                && capability.contains("workspace_qti_profile_item_evidence AS evidence")
                && capability.contains("result.status = 'accepted'::text")
                && capability.contains("result.normalized_sha256 =")
                && capability.contains("result.payload ->> 'itemId' <> result.source_identifier")
                && capability.contains("evidence.source_item_identifier")
                && capability.contains("result.payload ->> 'itemId' = evidence.item_id")
                && capability.contains("NOT EXISTS (\n                                SELECT 1\n                                  FROM public.workspace_qti_import_result AS result")
                && capability.contains("NOT EXISTS (\n                                SELECT 1\n                                  FROM public.workspace_qti_profile_import_evidence AS profile"),
            "recognized profile imports require exact evidence for accepted items but no synthetic evidence when all items are rejected"
        );
    }

    #[test]
    fn qti_profile_evidence_stage_matches_the_prepared_registry_summary() {
        let sql = MIGRATOR
            .iter()
            .find(|migration| migration.version == 2026080805)
            .expect("operations migration is embedded")
            .sql
            .as_ref();
        let start = sql
            .find("CREATE FUNCTION public.ple_stage_qti_profile_evidence(")
            .expect("QTI profile evidence staging capability is present");
        let end = sql[start..]
            .find("CREATE FUNCTION public.ple_read_committed_qti_profile_evidence(")
            .map(|offset| start + offset)
            .expect("QTI profile evidence capability has a bounded definition");
        let capability = &sql[start..end];

        assert!(
            !capability.contains("NOT (registry.payload ? 'profileSummary')")
                && capability
                    .contains("jsonb_typeof(registry.payload -> 'profileSummary') = 'object'")
                && capability
                    .contains("registry.payload #>> '{profileSummary,profileId}' = p_profile_id")
                && capability.contains(
                    "registry.payload #>> '{profileSummary,profileVersion}' = p_profile_version"
                )
                && capability.contains(
                    "registry.payload #>> '{profileSummary,mappingVersion}' = p_mapping_version"
                )
                && capability.contains("'{profileSummary,profileReportSha256}'"),
            "recognized prepared registries reject evidence for another profile or safe report"
        );
    }

    #[test]
    fn qti_profile_conversion_capabilities_repeat_the_committed_summary_binding() {
        let sql = MIGRATOR
            .iter()
            .find(|migration| migration.version == 2026080805)
            .expect("operations migration is embedded")
            .sql
            .as_ref();
        let reader_start = sql
            .find("CREATE FUNCTION public.ple_read_committed_qti_profile_evidence(")
            .expect("committed profile reader is present");
        let reader_end = sql[reader_start..]
            .find("CREATE FUNCTION public.ple_read_workspace_flat_import_origin(")
            .map(|offset| reader_start + offset)
            .expect("committed profile reader has a bounded definition");
        let reader = &sql[reader_start..reader_end];
        let replace_start = sql
            .find("CREATE FUNCTION public.ple_replace_workspace_flat_import_origin(")
            .expect("origin replacement capability is present");
        let replace_end = sql[replace_start..]
            .find("CREATE FUNCTION public.ple_promote_flat_import_origin(")
            .map(|offset| replace_start + offset)
            .expect("origin replacement capability has a bounded definition");
        let replace = &sql[replace_start..replace_end];

        for capability in [reader, replace] {
            assert!(
                capability
                    .contains("jsonb_typeof(registry.payload -> 'profileSummary') = 'object'")
                    && capability.contains("'{profileSummary,profileId}'")
                    && capability.contains("'{profileSummary,profileVersion}'")
                    && capability.contains("'{profileSummary,mappingVersion}'")
                    && capability.contains("'{profileSummary,profileReportSha256}'"),
                "conversion capabilities must bind private evidence to the committed summary"
            );
        }
    }

    #[test]
    fn curriculum_adoption_immutable_evidence_binds_the_complete_qmodel_envelope() {
        let sql = MIGRATOR
            .iter()
            .find(|migration| migration.version == 2026081838)
            .expect("curriculum-adoption foundation migration is embedded")
            .sql
            .as_ref();

        for table in [
            "curriculum_assignment_adoption_evidence",
            "curriculum_whole_course_adoption",
            "curriculum_alpha_fork_lineage",
        ] {
            let start = sql
                .find(&format!("CREATE TABLE public.{table} ("))
                .expect("immutable evidence relation is present");
            let definition = &sql[start..]
                .split_once("\n);\n")
                .expect("immutable evidence relation has a bounded definition")
                .0;

            assert!(
                definition.contains("semantic_payload jsonb NOT NULL")
                    && definition.contains("semantic_canonical_version smallint NOT NULL")
                    && definition.contains("semantic_canonical_bytes bytea NOT NULL")
                    && definition.contains("semantic_sha256 bytea NOT NULL")
                    && definition.contains("CHECK (jsonb_typeof(semantic_payload) = 'object')")
                    && definition.contains(
                        "CHECK (octet_length(semantic_payload::text) BETWEEN 2 AND 524288)"
                    )
                    && definition.contains("CHECK (semantic_canonical_version BETWEEN 1 AND 255)")
                    && definition.contains(
                        "CHECK (octet_length(semantic_canonical_bytes) BETWEEN 1 AND 524288)"
                    )
                    && definition.contains(
                        "CHECK (semantic_sha256 = digest(semantic_canonical_bytes, 'sha256'))"
                    ),
                "{table} must retain its reconstruction DTO separately from the exact, bounded, versioned qmodel semantic envelope"
            );
        }
    }
}

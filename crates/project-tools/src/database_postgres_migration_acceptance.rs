//! Closed runtime loading for the non-runtime PostgreSQL Migration epoch.

use anyhow::{Context, Result, bail};
use learning_data_access::postgres::{MigrationCheck, Pool};
use sqlx::migrate::Migrator;
use sqlx::{Acquire, AssertSqlSafe, Row};
use std::path::{Path, PathBuf};
use std::time::Instant;

// The pre-production schema is a single fresh baseline.  `schemas/migrations`
// is therefore both the authoring source and the ledger comparison source.
const POSTGRES_MIGRATION_ACCEPTANCE_MIGRATIONS_RELATIVE_PATH: &str = "schemas/migrations";
const POSTGRES_MIGRATION_ACCEPTANCE_MIGRATOR_ROLE: &str = "ple_migrator";
const FIRST_POSTGRES_MIGRATION_ACCEPTANCE_MIGRATION_VERSION: i64 = 2026082901;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum PostgresMigrationAcceptanceAction {
    Status,
    Migrate,
    Verify,
}

impl PostgresMigrationAcceptanceAction {
    pub(super) fn parse(value: &str) -> Option<Self> {
        match value {
            "migration-acceptance-status" => Some(Self::Status),
            "migration-acceptance-migrate" => Some(Self::Migrate),
            "migration-acceptance-verify" => Some(Self::Verify),
            _ => None,
        }
    }
}

pub(super) async fn run(action: PostgresMigrationAcceptanceAction, pool: &Pool) -> Result<()> {
    require_postgres_migration_acceptance_migration_principal(pool).await?;
    let directory = postgres_migration_acceptance_migrations_directory()?;
    let migrator = load_postgres_migration_acceptance_migrator(&directory).await?;
    match action {
        PostgresMigrationAcceptanceAction::Status => {
            let status = postgres_migration_acceptance_status(pool, &directory).await?;
            super::print_status(&status);
            Ok(())
        }
        PostgresMigrationAcceptanceAction::Migrate => migrate(pool, &directory, &migrator).await,
        PostgresMigrationAcceptanceAction::Verify => verify(pool, &directory).await,
    }
}

async fn migrate(pool: &Pool, directory: &Path, migrator: &Migrator) -> Result<()> {
    let mut connection = pool
        .acquire()
        .await
        .context("acquiring the PostgreSQL Migration connection")?;
    // SQLx records its own migration ledger in the bootstrap-owned `public`
    // schema. Keep that lookup explicit while the migrations safely switch to
    // the application schema owners.
    sqlx::query("SET search_path TO pg_catalog, public")
        .execute(&mut *connection)
        .await
        .context("pinning the SQLx migration ledger search path")?;
    apply_postgres_migration_acceptance_migrations(&mut connection, migrator)
        .await
        .context("applying the canonical repository-owned PostgreSQL Migrations")?;

    let status = postgres_migration_acceptance_status(pool, directory).await?;
    require_compatible(&status)?;
    println!("database migration-acceptance-migrate: complete and compatible");
    Ok(())
}

async fn apply_postgres_migration_acceptance_migrations(
    connection: &mut sqlx::pool::PoolConnection<sqlx::Postgres>,
    migrator: &Migrator,
) -> Result<()> {
    // SQLx sends an entire migration file as one raw SQL batch. PostgreSQL
    // analyzes policy expressions before an in-file SET LOCAL ROLE establishes
    // the owning schema role. PostgreSQL Migrations deliberately establish ownership
    // in the same transaction as the DDL, so execute complete top-level SQL
    // statements in order while retaining SQLx's ledger shape and checksum.
    // ASVS 8.2.1, 8.2.2: each policy is compiled by its owning schema role.
    let ledger_exists = sqlx::query_scalar::<_, Option<String>>(
        "SELECT to_regclass('public._sqlx_migrations')::text",
    )
    .fetch_one(&mut **connection)
    .await
    .context("checking the PostgreSQL Migration ledger")?
    .is_some();
    if !ledger_exists {
        sqlx::query(
            "CREATE TABLE public._sqlx_migrations (\
                version BIGINT PRIMARY KEY, description TEXT NOT NULL, \
                installed_on TIMESTAMPTZ NOT NULL DEFAULT now(), success BOOLEAN NOT NULL, \
                checksum BYTEA NOT NULL, execution_time BIGINT NOT NULL)",
        )
        .execute(&mut **connection)
        .await
        .context("creating the PostgreSQL Migration ledger")?;
    }

    let rows = sqlx::query(
        "SELECT version, success, checksum FROM public._sqlx_migrations ORDER BY version",
    )
    .fetch_all(&mut **connection)
    .await
    .context("reading the PostgreSQL Migration ledger")?;
    let applied = rows
        .into_iter()
        .map(|row| {
            Ok((
                row.try_get::<i64, _>("version")?,
                (
                    row.try_get::<bool, _>("success")?,
                    row.try_get::<Vec<u8>, _>("checksum")?,
                ),
            ))
        })
        .collect::<Result<std::collections::BTreeMap<_, _>, sqlx::Error>>()?;

    for migration in migrator
        .iter()
        .filter(|migration| !migration.migration_type.is_down_migration())
    {
        if let Some((success, checksum)) = applied.get(&migration.version) {
            if !success {
                bail!(
                    "PostgreSQL Migration {} is recorded as dirty",
                    migration.version
                );
            }
            if checksum.as_slice() != migration.checksum.as_ref() {
                bail!(
                    "PostgreSQL Migration {} checksum does not match its ledger entry",
                    migration.version
                );
            }
            continue;
        }
        if migration.no_tx {
            bail!(
                "PostgreSQL Migration {} must use transactional DDL",
                migration.version
            );
        }

        let started = Instant::now();
        let mut transaction = connection
            .begin()
            .await
            .with_context(|| format!("starting PostgreSQL Migration {}", migration.version))?;
        for (statement_index, statement) in top_level_sql_statements(migration.sql.as_str())?
            .into_iter()
            .enumerate()
        {
            // The splitter receives only the repository-owned migration text
            // loaded from the canonical directory above; no caller input
            // reaches this execution path.
            sqlx::raw_sql(AssertSqlSafe(statement.to_owned()))
                .execute(&mut *transaction)
                .await
                .with_context(|| {
                    let first_line = statement
                        .lines()
                        .find(|line| !line.trim().is_empty() && !line.trim().starts_with("--"))
                        .map(str::trim)
                        .unwrap_or("empty SQL statement");
                    format!(
                        "executing PostgreSQL Migration {} statement {} ({first_line})",
                        migration.version,
                        statement_index + 1
                    )
                })?;
        }
        sqlx::query(
            "INSERT INTO public._sqlx_migrations \
             (version, description, success, checksum, execution_time) VALUES ($1, $2, TRUE, $3, -1)",
        )
        .bind(migration.version)
        .bind(migration.description.as_ref())
        .bind(migration.checksum.as_ref())
        .execute(&mut *transaction)
        .await
        .with_context(|| format!("recording PostgreSQL Migration {}", migration.version))?;
        transaction
            .commit()
            .await
            .with_context(|| format!("committing PostgreSQL Migration {}", migration.version))?;
        sqlx::query("UPDATE public._sqlx_migrations SET execution_time = $1 WHERE version = $2")
            .bind(i64::try_from(started.elapsed().as_nanos()).unwrap_or(i64::MAX))
            .bind(migration.version)
            .execute(&mut **connection)
            .await
            .with_context(|| format!("timing PostgreSQL Migration {}", migration.version))?;
    }
    Ok(())
}

fn top_level_sql_statements(sql: &str) -> Result<Vec<&str>> {
    let bytes = sql.as_bytes();
    let mut statements = Vec::new();
    let mut start = 0;
    let mut index = 0;
    let mut quote: Option<u8> = None;
    let mut dollar_quote: Option<&str> = None;
    let mut block_comment_depth = 0_usize;
    while index < bytes.len() {
        if let Some(tag) = dollar_quote {
            if sql[index..].starts_with(tag) {
                index += tag.len();
                dollar_quote = None;
            } else {
                index += 1;
            }
            continue;
        }
        if block_comment_depth > 0 {
            if bytes[index..].starts_with(b"/*") {
                block_comment_depth += 1;
                index += 2;
            } else if bytes[index..].starts_with(b"*/") {
                block_comment_depth -= 1;
                index += 2;
            } else {
                index += 1;
            }
            continue;
        }
        if let Some(delimiter) = quote {
            if bytes[index] == delimiter {
                if index + 1 < bytes.len() && bytes[index + 1] == delimiter {
                    index += 2;
                } else {
                    quote = None;
                    index += 1;
                }
            } else {
                index += 1;
            }
            continue;
        }
        if bytes[index..].starts_with(b"--") {
            index += 2;
            while index < bytes.len() && bytes[index] != b'\n' {
                index += 1;
            }
        } else if bytes[index..].starts_with(b"/*") {
            block_comment_depth = 1;
            index += 2;
        } else if matches!(bytes[index], b'\'' | b'\"') {
            quote = Some(bytes[index]);
            index += 1;
        } else if bytes[index] == b'$' {
            let end = sql[index + 1..].find('$').map(|offset| index + offset + 1);
            if let Some(end) = end {
                let tag = &sql[index..=end];
                if tag[1..tag.len() - 1]
                    .bytes()
                    .all(|byte| byte == b'_' || byte.is_ascii_alphanumeric())
                {
                    dollar_quote = Some(tag);
                    index = end + 1;
                } else {
                    index += 1;
                }
            } else {
                index += 1;
            }
        } else if bytes[index] == b';' {
            let statement = sql[start..=index].trim();
            if !statement.is_empty() {
                statements.push(statement);
            }
            start = index + 1;
            index += 1;
        } else {
            index += 1;
        }
    }
    if quote.is_some() || dollar_quote.is_some() || block_comment_depth != 0 {
        bail!("PostgreSQL Migration contains an unterminated SQL delimiter");
    }
    let trailing = sql[start..].trim();
    if !trailing.is_empty() {
        statements.push(trailing);
    }
    Ok(statements)
}

async fn load_postgres_migration_acceptance_migrator(directory: &Path) -> Result<Migrator> {
    let migrator = Migrator::new(directory)
        .await
        .context("loading the canonical repository-owned PostgreSQL Migrations")?;
    let first_up_migration = migrator
        .iter()
        .find(|migration| !migration.migration_type.is_down_migration())
        .context("the canonical PostgreSQL Migration epoch is empty")?;
    if first_up_migration.version != FIRST_POSTGRES_MIGRATION_ACCEPTANCE_MIGRATION_VERSION {
        bail!(
            "the canonical PostgreSQL Migration epoch must begin at {FIRST_POSTGRES_MIGRATION_ACCEPTANCE_MIGRATION_VERSION}"
        );
    }
    Ok(migrator)
}

async fn require_postgres_migration_acceptance_migration_principal(pool: &Pool) -> Result<()> {
    let role = learning_data_access::postgres::migration_principal(pool)
        .await
        .context("checking the connected PostgreSQL Migration Acceptance Runtime role")?;
    if !postgres_migration_acceptance_migration_principal_is_expected(&role) {
        bail!(
            "PostgreSQL Migration Acceptance Runtime commands require PostgreSQL role {POSTGRES_MIGRATION_ACCEPTANCE_MIGRATOR_ROLE}; connected as {role}"
        );
    }
    Ok(())
}

fn postgres_migration_acceptance_migration_principal_is_expected(role: &str) -> bool {
    role == POSTGRES_MIGRATION_ACCEPTANCE_MIGRATOR_ROLE
}

async fn verify(pool: &Pool, directory: &Path) -> Result<()> {
    let status = postgres_migration_acceptance_status(pool, directory).await?;
    require_compatible(&status)?;
    println!("database migration-acceptance-verify: compatible");
    Ok(())
}

async fn postgres_migration_acceptance_status(
    pool: &Pool,
    directory: &Path,
) -> Result<MigrationCheck> {
    learning_data_access::postgres::migration_status_from_directory(pool, directory)
        .await
        .context("comparing the SQLx ledger with the canonical PostgreSQL Migrations")
}

fn require_compatible(status: &MigrationCheck) -> Result<()> {
    if !status.is_compatible() {
        super::print_status(status);
        bail!("PostgreSQL Migration ledger is not an exact successful checksum match");
    }
    Ok(())
}

fn postgres_migration_acceptance_migrations_directory() -> Result<PathBuf> {
    let repository_root = repository_root_from_manifest_dir(Path::new(env!("CARGO_MANIFEST_DIR")))?
        .canonicalize()
        .context("canonicalizing the project-tools repository root")?;
    let expected = repository_root.join(POSTGRES_MIGRATION_ACCEPTANCE_MIGRATIONS_RELATIVE_PATH);
    let canonical = expected
        .canonicalize()
        .context("canonicalizing schemas/migrations")?;
    if canonical != expected || !canonical.is_dir() {
        bail!("schemas/migrations must be the canonical repository-owned migration directory");
    }
    Ok(canonical)
}

fn repository_root_from_manifest_dir(manifest_dir: &Path) -> Result<PathBuf> {
    let crates_directory = manifest_dir
        .parent()
        .context("project-tools manifest directory has no crates parent")?;
    let repository_root = crates_directory
        .parent()
        .context("project-tools crates directory has no repository parent")?;
    Ok(repository_root.to_path_buf())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn postgres_migration_acceptance_action_names_form_a_closed_command_set() {
        assert_eq!(
            PostgresMigrationAcceptanceAction::parse("migration-acceptance-status"),
            Some(PostgresMigrationAcceptanceAction::Status)
        );
        assert_eq!(
            PostgresMigrationAcceptanceAction::parse("migration-acceptance-migrate"),
            Some(PostgresMigrationAcceptanceAction::Migrate)
        );
        assert_eq!(
            PostgresMigrationAcceptanceAction::parse("migration-acceptance-verify"),
            Some(PostgresMigrationAcceptanceAction::Verify)
        );
        assert_eq!(
            PostgresMigrationAcceptanceAction::parse("other-migrate"),
            None
        );
    }

    #[test]
    fn postgres_migration_acceptance_directory_is_derived_only_from_the_crate_location() {
        let repository_root =
            repository_root_from_manifest_dir(Path::new("/repo/crates/project-tools")).unwrap();
        assert_eq!(repository_root, Path::new("/repo"));
        assert_eq!(
            repository_root.join(POSTGRES_MIGRATION_ACCEPTANCE_MIGRATIONS_RELATIVE_PATH),
            Path::new("/repo/schemas/migrations")
        );
    }

    #[test]
    fn postgres_migration_acceptance_principal_contract_is_exact() {
        assert!(postgres_migration_acceptance_migration_principal_is_expected("ple_migrator"));
        assert!(!postgres_migration_acceptance_migration_principal_is_expected("postgres"));
        assert!(!postgres_migration_acceptance_migration_principal_is_expected("ple_app"));
    }

    #[test]
    fn top_level_splitter_preserves_quoted_and_dollar_quoted_semicolons() {
        let statements = top_level_sql_statements(
            "SET LOCAL ROLE ple_api_owner;\n\
             CREATE FUNCTION ple_api.example() RETURNS void LANGUAGE plpgsql AS $body$\n\
             BEGIN\n\
                 PERFORM ';';\n\
             END\n\
             $body$;\n\
             -- a comment containing ; must remain with the next statement\n\
             RESET ROLE;",
        )
        .unwrap();

        assert_eq!(statements.len(), 3);
        assert_eq!(statements[0], "SET LOCAL ROLE ple_api_owner;");
        assert!(statements[1].contains("PERFORM ';';"));
        assert!(statements[1].ends_with("$body$;"));
        assert!(statements[2].ends_with("RESET ROLE;"));
    }

    #[test]
    fn top_level_splitter_preserves_nested_block_comments() {
        let statements = top_level_sql_statements(
            "/* outer ; /* inner ; */ still outer ; */\nSELECT 1;\nSELECT 2;",
        )
        .unwrap();

        assert_eq!(statements.len(), 2);
        assert!(statements[0].ends_with("SELECT 1;"));
        assert_eq!(statements[1], "SELECT 2;");
    }

    #[test]
    fn top_level_splitter_rejects_an_unterminated_delimiter() {
        let error = top_level_sql_statements("SELECT $$unfinished;").unwrap_err();
        assert!(error.to_string().contains("unterminated SQL delimiter"));
    }
}

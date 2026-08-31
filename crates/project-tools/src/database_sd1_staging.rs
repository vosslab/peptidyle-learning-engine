//! Closed runtime loading for the non-runtime SD1 migration epoch.

use anyhow::{Context, Result, bail};
use learning_data_access::postgres::{MigrationStatus, Pool};
use sqlx::migrate::Migrator;
use sqlx::{Acquire, AssertSqlSafe, Row};
use std::path::{Path, PathBuf};
use std::time::Instant;

// The pre-production schema is a single fresh baseline.  `schemas/migrations`
// is therefore both the authoring source and the ledger comparison source.
const STAGED_MIGRATIONS_RELATIVE_PATH: &str = "schemas/migrations";
const STAGED_MIGRATION_PRINCIPAL: &str = "ple_migrator";
const FIRST_STAGED_MIGRATION_VERSION: i64 = 2026082901;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum Sd1StagedAction {
    Status,
    Migrate,
    Verify,
}

impl Sd1StagedAction {
    pub(super) fn parse(value: &str) -> Option<Self> {
        match value {
            "sd1-staged-status" => Some(Self::Status),
            "sd1-staged-migrate" => Some(Self::Migrate),
            "sd1-staged-verify" => Some(Self::Verify),
            _ => None,
        }
    }
}

pub(super) async fn run(action: Sd1StagedAction, pool: &Pool) -> Result<()> {
    require_staged_migration_principal(pool).await?;
    let directory = staged_migrations_directory()?;
    let migrator = load_staged_migrator(&directory).await?;
    match action {
        Sd1StagedAction::Status => {
            let status = staged_status(pool, &directory).await?;
            super::print_status(&status);
            Ok(())
        }
        Sd1StagedAction::Migrate => migrate(pool, &directory, &migrator).await,
        Sd1StagedAction::Verify => verify(pool, &directory).await,
    }
}

async fn migrate(pool: &Pool, directory: &Path, migrator: &Migrator) -> Result<()> {
    let mut connection = pool
        .acquire()
        .await
        .context("acquiring the SD1 baseline migration connection")?;
    // SQLx records its own migration ledger in the bootstrap-owned `public`
    // schema. Keep that lookup explicit while the migrations safely switch to
    // the application schema owners.
    sqlx::query("SET search_path TO pg_catalog, public")
        .execute(&mut *connection)
        .await
        .context("pinning the SQLx migration ledger search path")?;
    apply_staged_migrations(&mut connection, migrator)
        .await
        .context("applying the canonical repository-owned SD1 baseline migrations")?;

    let status = staged_status(pool, directory).await?;
    require_compatible(&status)?;
    println!("database sd1-staged-migrate: complete and compatible");
    Ok(())
}

async fn apply_staged_migrations(
    connection: &mut sqlx::pool::PoolConnection<sqlx::Postgres>,
    migrator: &Migrator,
) -> Result<()> {
    // SQLx sends an entire migration file as one raw SQL batch. PostgreSQL
    // analyzes policy expressions before an in-file SET LOCAL ROLE establishes
    // the owning schema role. SD1 migrations deliberately establish ownership
    // in the same transaction as the DDL, so execute complete top-level SQL
    // statements in order while retaining SQLx's ledger shape and checksum.
    // ASVS 8.2.1, 8.2.2: each policy is compiled by its owning schema role.
    let ledger_exists = sqlx::query_scalar::<_, Option<String>>(
        "SELECT to_regclass('public._sqlx_migrations')::text",
    )
    .fetch_one(&mut **connection)
    .await
    .context("checking the SD1 migration ledger")?
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
        .context("creating the SD1 migration ledger")?;
    }

    let rows = sqlx::query(
        "SELECT version, success, checksum FROM public._sqlx_migrations ORDER BY version",
    )
    .fetch_all(&mut **connection)
    .await
    .context("reading the SD1 migration ledger")?;
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
                bail!("SD1 migration {} is recorded as dirty", migration.version);
            }
            if checksum.as_slice() != migration.checksum.as_ref() {
                bail!(
                    "SD1 migration {} checksum does not match its ledger entry",
                    migration.version
                );
            }
            continue;
        }
        if migration.no_tx {
            bail!(
                "SD1 migration {} must use transactional DDL",
                migration.version
            );
        }

        let started = Instant::now();
        let mut transaction = connection
            .begin()
            .await
            .with_context(|| format!("starting SD1 migration {}", migration.version))?;
        for statement in top_level_sql_statements(migration.sql.as_str())? {
            // The splitter receives only the repository-owned migration text
            // loaded from the canonical directory above; no caller input
            // reaches this execution path.
            sqlx::raw_sql(AssertSqlSafe(statement.to_owned()))
                .execute(&mut *transaction)
                .await
                .with_context(|| {
                    format!("executing SD1 migration {} statement", migration.version)
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
        .with_context(|| format!("recording SD1 migration {}", migration.version))?;
        transaction
            .commit()
            .await
            .with_context(|| format!("committing SD1 migration {}", migration.version))?;
        sqlx::query("UPDATE public._sqlx_migrations SET execution_time = $1 WHERE version = $2")
            .bind(i64::try_from(started.elapsed().as_nanos()).unwrap_or(i64::MAX))
            .bind(migration.version)
            .execute(&mut **connection)
            .await
            .with_context(|| format!("timing SD1 migration {}", migration.version))?;
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
        bail!("SD1 migration contains an unterminated SQL delimiter");
    }
    let trailing = sql[start..].trim();
    if !trailing.is_empty() {
        statements.push(trailing);
    }
    Ok(statements)
}

async fn load_staged_migrator(directory: &Path) -> Result<Migrator> {
    let migrator = Migrator::new(directory)
        .await
        .context("loading the canonical repository-owned SD1 baseline migrations")?;
    let first_up_migration = migrator
        .iter()
        .find(|migration| !migration.migration_type.is_down_migration())
        .context("the canonical SD1 baseline migration epoch is empty")?;
    if first_up_migration.version != FIRST_STAGED_MIGRATION_VERSION {
        bail!(
            "the canonical SD1 baseline migration epoch must begin at {FIRST_STAGED_MIGRATION_VERSION}"
        );
    }
    Ok(migrator)
}

async fn require_staged_migration_principal(pool: &Pool) -> Result<()> {
    let role = learning_data_access::postgres::migration_principal(pool)
        .await
        .context("checking the connected SD1 staged migration role")?;
    if !staged_migration_principal_is_expected(&role) {
        bail!(
            "SD1 staged database commands require PostgreSQL role {STAGED_MIGRATION_PRINCIPAL}; connected as {role}"
        );
    }
    Ok(())
}

fn staged_migration_principal_is_expected(role: &str) -> bool {
    role == STAGED_MIGRATION_PRINCIPAL
}

async fn verify(pool: &Pool, directory: &Path) -> Result<()> {
    let status = staged_status(pool, directory).await?;
    require_compatible(&status)?;
    println!("database sd1-staged-verify: compatible");
    Ok(())
}

async fn staged_status(pool: &Pool, directory: &Path) -> Result<MigrationStatus> {
    learning_data_access::postgres::migration_status_from_directory(pool, directory)
        .await
        .context("comparing the SQLx ledger with the canonical SD1 baseline migrations")
}

fn require_compatible(status: &MigrationStatus) -> Result<()> {
    if !status.is_compatible() {
        super::print_status(status);
        bail!("SD1 staged migration ledger is not an exact successful checksum match");
    }
    Ok(())
}

fn staged_migrations_directory() -> Result<PathBuf> {
    let repository_root = repository_root_from_manifest_dir(Path::new(env!("CARGO_MANIFEST_DIR")))?
        .canonicalize()
        .context("canonicalizing the project-tools repository root")?;
    let expected = repository_root.join(STAGED_MIGRATIONS_RELATIVE_PATH);
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
    fn staged_action_names_form_a_closed_command_set() {
        assert_eq!(
            Sd1StagedAction::parse("sd1-staged-status"),
            Some(Sd1StagedAction::Status)
        );
        assert_eq!(
            Sd1StagedAction::parse("sd1-staged-migrate"),
            Some(Sd1StagedAction::Migrate)
        );
        assert_eq!(
            Sd1StagedAction::parse("sd1-staged-verify"),
            Some(Sd1StagedAction::Verify)
        );
        assert_eq!(Sd1StagedAction::parse("staged-migrate"), None);
    }

    #[test]
    fn staged_directory_is_derived_only_from_the_crate_location() {
        let repository_root =
            repository_root_from_manifest_dir(Path::new("/repo/crates/project-tools")).unwrap();
        assert_eq!(repository_root, Path::new("/repo"));
        assert_eq!(
            repository_root.join(STAGED_MIGRATIONS_RELATIVE_PATH),
            Path::new("/repo/schemas/migrations")
        );
    }

    #[test]
    fn staged_principal_contract_is_exact() {
        assert!(staged_migration_principal_is_expected("ple_migrator"));
        assert!(!staged_migration_principal_is_expected("postgres"));
        assert!(!staged_migration_principal_is_expected("ple_app"));
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

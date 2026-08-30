//! Closed runtime loading for the non-runtime SD1 migration epoch.

use anyhow::{Context, Result, bail};
use learning_data_access::postgres::{MigrationStatus, Pool};
use sqlx::migrate::Migrator;
use std::path::{Path, PathBuf};

const STAGED_MIGRATIONS_RELATIVE_PATH: &str = "schemas/staged_migrations";
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
    migrator
        .run(pool)
        .await
        .context("applying the canonical repository-owned SD1 staged migrations")?;

    let status = staged_status(pool, directory).await?;
    require_compatible(&status)?;
    println!("database sd1-staged-migrate: complete and compatible");
    Ok(())
}

async fn load_staged_migrator(directory: &Path) -> Result<Migrator> {
    let migrator = Migrator::new(directory)
        .await
        .context("loading the canonical repository-owned SD1 staged migrations")?;
    let first_up_migration = migrator
        .iter()
        .find(|migration| !migration.migration_type.is_down_migration())
        .context("the canonical SD1 staged migration epoch is empty")?;
    if first_up_migration.version != FIRST_STAGED_MIGRATION_VERSION {
        bail!(
            "the canonical SD1 staged migration epoch must begin at {FIRST_STAGED_MIGRATION_VERSION}"
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
        .context("comparing the SQLx ledger with the canonical SD1 staged migrations")
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
        .context("canonicalizing schemas/staged_migrations")?;
    if canonical != expected || !canonical.is_dir() {
        bail!(
            "schemas/staged_migrations must be the canonical repository-owned migration directory"
        );
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
            Path::new("/repo/schemas/staged_migrations")
        );
    }

    #[test]
    fn staged_principal_contract_is_exact() {
        assert!(staged_migration_principal_is_expected("ple_migrator"));
        assert!(!staged_migration_principal_is_expected("postgres"));
        assert!(!staged_migration_principal_is_expected("ple_app"));
    }
}

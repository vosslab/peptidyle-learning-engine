//! One locked coordinator for structural initialization and SQLx forward migrations.

use anyhow::{Context, Result, bail, ensure};
use learning_data_access::postgres::{Pool, acquire_schema_lifecycle};
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::libpq_environment::LibpqEnvironment;

const IMAGE_SCHEMA_ROOT: &str = "/opt/ple/schemas";
const BASE_MANIFEST_RELATIVE_PATH: &str = "base_schema/install.sql";
const MIGRATION_PRINCIPAL: &str = "ple_migrator";
/// PostgreSQL 17 client path supplied by the pinned Debian migrator image.
const PSQL_EXECUTABLE: &str = "/usr/bin/psql";

/// Mutating database operations owned by this coordinator.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum DatabaseCoordinatorAction {
    /// Install the fixed base schema into a genuinely empty database.
    Initialize,
    /// Apply recognized pending SQLx forward migrations.
    Migrate,
}

/// Runs one mutating database lifecycle operation under one PLE schema lock.
pub(crate) async fn run(
    action: DatabaseCoordinatorAction,
    pool: &Pool,
    database_url: &str,
) -> Result<()> {
    let mut lifecycle = acquire_schema_lifecycle(pool)
        .await
        .context("acquiring the database schema lifecycle lock")?;
    let principal = lifecycle
        .migration_principal()
        .await
        .context("checking the database migration principal")?;
    ensure!(
        principal == MIGRATION_PRINCIPAL,
        "database lifecycle commands require PostgreSQL role {MIGRATION_PRINCIPAL}"
    );

    match action {
        DatabaseCoordinatorAction::Initialize => {
            if !lifecycle
                .base_is_installed()
                .await
                .context("checking whether the base schema is installed")?
            {
                install_base_schema(database_url).await?;
            }
            lifecycle
                .verify_base_projection()
                .await
                .context("verifying the installed base schema")?;
            if lifecycle
                .base_is_pre_production()
                .await
                .context("checking the base release lifecycle")?
            {
                lifecycle
                    .verify_privileged()
                    .await
                    .context("verifying the initialized database schema")?;
            } else {
                apply_and_verify(&mut lifecycle).await?;
            }
            println!("database initialize: complete and compatible");
        }
        DatabaseCoordinatorAction::Migrate => {
            if !lifecycle
                .base_is_installed()
                .await
                .context("checking whether the base schema is installed")?
            {
                bail!("database migrate requires an initialized database; run database initialize");
            }
            lifecycle
                .verify_base_projection()
                .await
                .context("verifying the installed base schema")?;
            if lifecycle
                .base_is_pre_production()
                .await
                .context("checking the base release lifecycle")?
            {
                bail!(
                    "database migrate is unavailable while the base release is pre-production; update the base schema directly until production freeze"
                );
            }
            apply_and_verify(&mut lifecycle).await?;
            println!("database migrate: complete and compatible");
        }
    }
    Ok(())
}

async fn apply_and_verify(
    lifecycle: &mut learning_data_access::postgres::SchemaLifecycleGuard,
) -> Result<()> {
    lifecycle
        .apply_forward_migrations()
        .await
        .context("applying recognized SQLx forward migrations")?;
    lifecycle
        .verify_privileged()
        .await
        .context("verifying the initialized database schema")?;
    Ok(())
}

async fn install_base_schema(database_url: &str) -> Result<()> {
    let manifest = base_manifest_path()?;
    let environment =
        LibpqEnvironment::from_database_url(database_url, "database administration URL")?;
    tokio::task::spawn_blocking(move || run_psql_manifest(&manifest, &environment))
        .await
        .context("waiting for the PostgreSQL base installation command")??;
    Ok(())
}

fn run_psql_manifest(manifest: &Path, environment: &LibpqEnvironment) -> Result<()> {
    let status = Command::new(PSQL_EXECUTABLE)
        .env_clear()
        .env(
            "PATH",
            "/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin",
        )
        .envs(environment.variables())
        .arg("-X")
        .arg("--set=ON_ERROR_STOP=1")
        .arg("--single-transaction")
        .arg("--file")
        .arg(manifest)
        .status()
        .context("starting the PostgreSQL base installation command")?;
    if !status.success() {
        bail!("PostgreSQL base installation command failed ({status})");
    }
    Ok(())
}

fn base_manifest_path() -> Result<PathBuf> {
    let source_schema_root =
        repository_root_from_manifest_dir(Path::new(env!("CARGO_MANIFEST_DIR")))?.join("schemas");
    for schema_root in [PathBuf::from(IMAGE_SCHEMA_ROOT), source_schema_root] {
        let Ok(schema_root) = schema_root.canonicalize() else {
            continue;
        };
        let manifest = schema_root.join(BASE_MANIFEST_RELATIVE_PATH);
        let Ok(manifest) = manifest.canonicalize() else {
            continue;
        };
        if path_is_within(&schema_root, &manifest) && manifest.is_file() {
            return Ok(manifest);
        }
    }
    bail!("the fixed base schema manifest is unavailable")
}

fn repository_root_from_manifest_dir(manifest_dir: &Path) -> Result<PathBuf> {
    manifest_dir
        .parent()
        .and_then(Path::parent)
        .map(Path::to_path_buf)
        .context("locating the repository root from the project-tools manifest")
}

fn path_is_within(root: &Path, path: &Path) -> bool {
    path.strip_prefix(root).is_ok()
}

//! Explicit database migration control for the pre-data SQLx epoch.

use anyhow::{Context, Result, bail};
use learning_data_access::postgres::{MigrationDisposition, MigrationStatus, Pool};
use std::path::Path;

const USAGE: &str = "usage: cargo tools database <status|migrate|verify> [--migrations-dir PATH] (migrate requires PLE_MIGRATION_DATABASE_URL; status/verify read DATABASE_URL or PLE_MIGRATION_DATABASE_URL)";

/// Runs one read-only or explicitly mutating database command.
pub fn run(args: &[String]) -> Result<()> {
    if matches!(args, [flag] if flag == "--help" || flag == "-h") {
        println!("{USAGE}");
        return Ok(());
    }
    let (action, migrations_dir) = match args {
        [action] => (action.as_str(), None),
        [action, flag, path] if action == "status" && flag == "--migrations-dir" => {
            (action.as_str(), Some(Path::new(path)))
        }
        _ => bail!(USAGE),
    };
    if migrations_dir.is_some() && action != "status" {
        bail!("{USAGE}");
    }
    let database_url = database_url_for(action)?;
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .context("creating the database administration runtime")?;
    runtime.block_on(async {
        let pool = learning_data_access::postgres::lazy_pool(&database_url)
            .context("database administration URL is not a valid PostgreSQL connection URL")?;
        run_action(action, &pool, migrations_dir).await
    })
}

fn database_url_for(action: &str) -> Result<String> {
    let value = match action {
        "migrate" => std::env::var("PLE_MIGRATION_DATABASE_URL")
            .context("PLE_MIGRATION_DATABASE_URL must be set for database migration")?,
        "status" | "verify" => std::env::var("PLE_MIGRATION_DATABASE_URL")
            .or_else(|_| std::env::var("DATABASE_URL"))
            .context("DATABASE_URL or PLE_MIGRATION_DATABASE_URL must be set for database administration")?,
        _ => return Ok(String::new()),
    };
    if value.trim().is_empty() {
        bail!("database administration URL must not be empty");
    }
    Ok(value)
}

fn migration_role_is_allowed(role: &str) -> bool {
    role != "ple_app"
}

async fn run_action(action: &str, pool: &Pool, migrations_dir: Option<&Path>) -> Result<()> {
    match action {
        "status" => {
            let status = match migrations_dir {
                Some(directory) => learning_data_access::postgres::migration_status_from_directory(
                    pool, directory,
                )
                .await
                .context(
                    "reading the SQLx migration ledger against the supplied migration directory",
                )?,
                None => learning_data_access::postgres::migration_status(pool)
                    .await
                    .context("reading the SQLx migration ledger")?,
            };
            print_status(&status);
            Ok(())
        }
        "migrate" => {
            let role = learning_data_access::postgres::migration_principal(pool)
                .await
                .context("checking the connected migration role")?;
            if !migration_role_is_allowed(&role) {
                bail!("database migrate refuses the ple_app application role");
            }
            learning_data_access::postgres::apply_migrations(pool)
                .await
                .context("applying the embedded SQLx database epoch")?;
            learning_data_access::postgres::verify_application_schema(pool)
                .await
                .context("verifying the migrated application schema")?;
            println!("database migrate: complete and compatible");
            Ok(())
        }
        "verify" => {
            learning_data_access::postgres::verify_application_schema(pool)
                .await
                .context("verifying the application database epoch")?;
            println!("database verify: compatible");
            Ok(())
        }
        _ => bail!("unknown database action {action}; {USAGE}"),
    }
}

fn print_status(status: &MigrationStatus) {
    println!(
        "database status: ledger {}",
        if status.ledger_present() {
            "present"
        } else {
            "absent"
        }
    );
    for entry in status.entries() {
        let disposition = match entry.disposition() {
            MigrationDisposition::Applied => "applied",
            MigrationDisposition::Pending => "pending",
            MigrationDisposition::Modified => "modified",
            MigrationDisposition::Dirty => "dirty",
        };
        println!(
            "  {} {}: {disposition}",
            entry.version(),
            entry.description()
        );
    }
    for version in status.unexpected_applied_versions() {
        println!("  {version}: unexpected applied migration");
    }
    println!(
        "database status: {}",
        if status.is_compatible() {
            "compatible"
        } else {
            "not compatible"
        }
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn migration_refuses_the_application_principal() {
        assert!(!migration_role_is_allowed("ple_app"));
        assert!(migration_role_is_allowed("ple_migration"));
    }
}

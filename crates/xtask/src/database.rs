//! Explicit database migration control for the pre-data SQLx epoch.

use anyhow::{Context, Result, bail};
use store::postgres::{MigrationDisposition, MigrationStatus, Pool};

const USAGE: &str = "usage: cargo xtask database <status|migrate|verify> (reads DATABASE_URL)";

/// Runs one read-only or explicitly mutating database command.
pub fn run(args: &[String]) -> Result<()> {
    if matches!(args, [flag] if flag == "--help" || flag == "-h") {
        println!("{USAGE}");
        return Ok(());
    }
    let [action] = args else {
        bail!("{USAGE}");
    };
    let database_url = std::env::var("DATABASE_URL")
        .context("DATABASE_URL must be set for database administration")?;
    if database_url.trim().is_empty() {
        bail!("DATABASE_URL must not be empty");
    }
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .context("creating the database administration runtime")?;
    runtime.block_on(async {
        let pool = store::postgres::lazy_pool(&database_url)
            .context("DATABASE_URL is not a valid PostgreSQL connection URL")?;
        run_action(action, &pool).await
    })
}

async fn run_action(action: &str, pool: &Pool) -> Result<()> {
    match action {
        "status" => {
            let status = store::postgres::migration_status(pool)
                .await
                .context("reading the SQLx migration ledger")?;
            print_status(&status);
            Ok(())
        }
        "migrate" => {
            store::postgres::apply_migrations(pool)
                .await
                .context("applying the embedded SQLx database epoch")?;
            store::postgres::verify_application_schema(pool)
                .await
                .context("verifying the migrated application schema")?;
            println!("database migrate: complete and compatible");
            Ok(())
        }
        "verify" => {
            store::postgres::verify_application_schema(pool)
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

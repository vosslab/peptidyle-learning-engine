//! Explicit database migration control for the pre-data SQLx epoch.

use acceptance_runtime::AcceptanceRuntime;
use anyhow::{Context, Result, bail};
use learning_data_access::postgres::{MigrationDisposition, MigrationStatus, Pool};
use std::path::Path;

const USAGE: &str = "usage: cargo tools database <status|migrate|verify> [--migrations-dir PATH] [--acceptance-runtime] (ordinary mode reads PLE_MIGRATION_DATABASE_URL or DATABASE_URL)";

enum DatabaseConnection {
    Acceptance(AcceptanceRuntime),
    Environment(String),
}

/// Runs one read-only or explicitly mutating database command.
pub fn run(args: &[String]) -> Result<()> {
    if matches!(args, [flag] if flag == "--help" || flag == "-h") {
        println!("{USAGE}");
        return Ok(());
    }
    let (action, migrations_dir, acceptance_runtime) = parse_arguments(args)?;
    if migrations_dir.is_some() && action != "status" {
        bail!("{USAGE}");
    }
    let connection = database_connection_for(action, acceptance_runtime)?;
    let tokio_runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .context("creating the database administration runtime")?;
    tokio_runtime.block_on(async {
        let pool = learning_data_access::postgres::lazy_pool(match &connection {
            DatabaseConnection::Acceptance(runtime) => runtime.admin_url().expose(),
            DatabaseConnection::Environment(url) => url,
        })
        .context("database administration URL is not a valid PostgreSQL connection URL")?;
        run_action(action, &pool, migrations_dir).await
    })
}

fn parse_arguments(args: &[String]) -> Result<(&str, Option<&Path>, bool)> {
    let action = args
        .first()
        .map(String::as_str)
        .ok_or_else(|| anyhow::anyhow!(USAGE))?;
    if !matches!(action, "status" | "migrate" | "verify") {
        bail!(USAGE);
    }
    let mut migrations_dir = None;
    let mut acceptance_runtime = false;
    let mut index = 1;
    while index < args.len() {
        match args[index].as_str() {
            "--acceptance-runtime" if !acceptance_runtime => {
                acceptance_runtime = true;
                index += 1;
            }
            "--migrations-dir" if migrations_dir.is_none() && index + 1 < args.len() => {
                migrations_dir = Some(Path::new(&args[index + 1]));
                index += 2;
            }
            _ => bail!(USAGE),
        }
    }
    Ok((action, migrations_dir, acceptance_runtime))
}

fn database_connection_for(action: &str, acceptance_runtime: bool) -> Result<DatabaseConnection> {
    if acceptance_runtime {
        let runtime = AcceptanceRuntime::load().map_err(|error| {
            anyhow::anyhow!("acceptance runtime is required for database administration: {error}")
        })?;
        return Ok(DatabaseConnection::Acceptance(runtime));
    }
    let value = match action {
        "migrate" => std::env::var("PLE_MIGRATION_DATABASE_URL")
            .context("PLE_MIGRATION_DATABASE_URL must be set for database migration")?,
        "status" | "verify" => std::env::var("PLE_MIGRATION_DATABASE_URL")
            .or_else(|_| std::env::var("DATABASE_URL"))
            .context("DATABASE_URL or PLE_MIGRATION_DATABASE_URL must be set for database administration")?,
        _ => return Ok(DatabaseConnection::Environment(String::new())),
    };
    if value.trim().is_empty() {
        bail!("database administration URL must not be empty");
    }
    Ok(DatabaseConnection::Environment(value))
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
            learning_data_access::postgres::verify_base_course_freshness_capability(pool)
                .await
                .context("verifying the migrated Base Course freshness capability")?;
            println!("database migrate: complete and compatible");
            Ok(())
        }
        "verify" => {
            learning_data_access::postgres::verify_application_schema(pool)
                .await
                .context("verifying the application database epoch")?;
            learning_data_access::postgres::verify_base_course_freshness_capability(pool)
                .await
                .context("verifying the Base Course freshness capability")?;
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

    #[test]
    fn acceptance_runtime_is_an_explicit_database_mode() {
        let args = vec!["status".to_string(), "--acceptance-runtime".to_string()];
        let (action, migrations_dir, acceptance_runtime) = parse_arguments(&args).unwrap();
        assert_eq!(action, "status");
        assert!(migrations_dir.is_none());
        assert!(acceptance_runtime);
    }

    #[test]
    fn migration_directory_and_acceptance_mode_can_be_combined() {
        let args = vec![
            "status".to_string(),
            "--migrations-dir".to_string(),
            "schemas/migrations".to_string(),
            "--acceptance-runtime".to_string(),
        ];
        let (action, migrations_dir, acceptance_runtime) = parse_arguments(&args).unwrap();
        assert_eq!(action, "status");
        assert_eq!(migrations_dir, Some(Path::new("schemas/migrations")));
        assert!(acceptance_runtime);
    }

    #[test]
    fn unknown_database_action_is_rejected_before_loading_credentials() {
        let args = vec!["unknown".to_string(), "--acceptance-runtime".to_string()];
        assert!(parse_arguments(&args).is_err());
    }
}

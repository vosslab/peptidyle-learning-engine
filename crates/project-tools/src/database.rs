//! Explicit database migration control for the pre-data SQLx epoch.

use acceptance_runtime::{AcceptanceRuntime, PostgresMigrationAcceptanceRuntime};
use anyhow::{Context, Result, bail};
use learning_data_access::postgres::{MigrationCheck, MigrationCheckResult, Pool};
use std::path::Path;

#[path = "database_postgres_migration_acceptance.rs"]
mod database_postgres_migration_acceptance;

use database_postgres_migration_acceptance::PostgresMigrationAcceptanceAction;

const USAGE: &str = "usage: cargo tools database <status|migrate|verify> [--migrations-dir PATH] [--acceptance-runtime] | cargo tools database <migration-acceptance-status|migration-acceptance-migrate|migration-acceptance-verify> --acceptance-runtime (ordinary mode reads PLE_MIGRATION_DATABASE_URL or DATABASE_URL)";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum DatabaseAction {
    Status,
    Migrate,
    Verify,
    PostgresMigrationAcceptance(PostgresMigrationAcceptanceAction),
}

impl DatabaseAction {
    fn parse(value: &str) -> Option<Self> {
        match value {
            "status" => Some(Self::Status),
            "migrate" => Some(Self::Migrate),
            "verify" => Some(Self::Verify),
            _ => PostgresMigrationAcceptanceAction::parse(value)
                .map(Self::PostgresMigrationAcceptance),
        }
    }

    const fn accepts_migrations_directory(self) -> bool {
        matches!(self, Self::Status)
    }

    const fn requires_acceptance_runtime(self) -> bool {
        matches!(self, Self::PostgresMigrationAcceptance(_))
    }
}

enum DatabaseConnection {
    Acceptance(AcceptanceRuntime),
    PostgresMigrationAcceptance(PostgresMigrationAcceptanceRuntime),
    Environment(String),
}

/// Runs one read-only or explicitly mutating database command.
pub fn run(args: &[String]) -> Result<()> {
    if matches!(args, [flag] if flag == "--help" || flag == "-h") {
        println!("{USAGE}");
        return Ok(());
    }
    let (action, migrations_dir, acceptance_runtime) = parse_arguments(args)?;
    let connection = database_connection_for(action, acceptance_runtime)?;
    let tokio_runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .context("creating the database administration runtime")?;
    tokio_runtime.block_on(async {
        let pool = learning_data_access::postgres::lazy_pool(match &connection {
            DatabaseConnection::Acceptance(runtime) => runtime.admin_url().expose(),
            DatabaseConnection::PostgresMigrationAcceptance(runtime) => {
                runtime.postgres_migrator_url().expose()
            }
            DatabaseConnection::Environment(url) => url,
        })
        .context("database administration URL is not a valid PostgreSQL connection URL")?;
        run_action(action, &pool, migrations_dir).await
    })
}

fn parse_arguments(args: &[String]) -> Result<(DatabaseAction, Option<&Path>, bool)> {
    let action = args
        .first()
        .and_then(|value| DatabaseAction::parse(value))
        .ok_or_else(|| anyhow::anyhow!(USAGE))?;
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
    if migrations_dir.is_some() && !action.accepts_migrations_directory() {
        bail!(USAGE);
    }
    if action.requires_acceptance_runtime() && !acceptance_runtime {
        bail!(
            "PostgreSQL Migration Acceptance Runtime commands require --acceptance-runtime; {USAGE}"
        );
    }
    Ok((action, migrations_dir, acceptance_runtime))
}

fn database_connection_for(
    action: DatabaseAction,
    acceptance_runtime: bool,
) -> Result<DatabaseConnection> {
    if acceptance_runtime {
        return match action {
            DatabaseAction::PostgresMigrationAcceptance(_) => {
                PostgresMigrationAcceptanceRuntime::load()
                    .map(DatabaseConnection::PostgresMigrationAcceptance)
                    .map_err(|error| {
                        anyhow::anyhow!(
                            "PostgreSQL Migration Acceptance Runtime is required: {error}"
                        )
                    })
            }
            _ => AcceptanceRuntime::load()
                .map(DatabaseConnection::Acceptance)
                .map_err(|error| {
                    anyhow::anyhow!(
                        "acceptance runtime is required for database administration: {error}"
                    )
                }),
        };
    }
    let value = match action {
        DatabaseAction::Migrate => std::env::var("PLE_MIGRATION_DATABASE_URL")
            .context("PLE_MIGRATION_DATABASE_URL must be set for database migration")?,
        DatabaseAction::Status | DatabaseAction::Verify => std::env::var(
            "PLE_MIGRATION_DATABASE_URL",
        )
        .or_else(|_| std::env::var("DATABASE_URL"))
        .context(
            "DATABASE_URL or PLE_MIGRATION_DATABASE_URL must be set for database administration",
        )?,
        DatabaseAction::PostgresMigrationAcceptance(_) => {
            bail!("PostgreSQL Migration Acceptance Runtime commands require --acceptance-runtime")
        }
    };
    if value.trim().is_empty() {
        bail!("database administration URL must not be empty");
    }
    Ok(DatabaseConnection::Environment(value))
}

fn migration_role_is_allowed(role: &str) -> bool {
    role != "ple_app"
}

async fn run_action(
    action: DatabaseAction,
    pool: &Pool,
    migrations_dir: Option<&Path>,
) -> Result<()> {
    match action {
        DatabaseAction::Status => {
            let status = match migrations_dir {
                Some(directory) => learning_data_access::postgres::migration_status_from_directory(
                    pool, directory,
                )
                .await
                .context(
                    "reading the SQLx migration ledger against the supplied migration directory",
                )?,
                None => learning_data_access::postgres::migration_check(pool)
                    .await
                    .context("reading the SQLx migration ledger")?,
            };
            print_status(&status);
            Ok(())
        }
        DatabaseAction::Migrate => {
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
        DatabaseAction::Verify => {
            learning_data_access::postgres::verify_application_schema(pool)
                .await
                .context("verifying the application database epoch")?;
            println!("database verify: compatible");
            Ok(())
        }
        DatabaseAction::PostgresMigrationAcceptance(action) => {
            database_postgres_migration_acceptance::run(action, pool).await
        }
    }
}

fn print_status(status: &MigrationCheck) {
    println!(
        "database status: ledger {}",
        if status.ledger_present() {
            "present"
        } else {
            "absent"
        }
    );
    for entry in status.entries() {
        let result = match entry.result() {
            MigrationCheckResult::Applied => "applied",
            MigrationCheckResult::Pending => "pending",
            MigrationCheckResult::Changed => "changed",
            MigrationCheckResult::Incomplete => "incomplete",
        };
        println!("  {} {}: {result}", entry.version(), entry.description());
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
        assert_eq!(action, DatabaseAction::Status);
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
        assert_eq!(action, DatabaseAction::Status);
        assert_eq!(migrations_dir, Some(Path::new("schemas/migrations")));
        assert!(acceptance_runtime);
    }

    #[test]
    fn unknown_database_action_is_rejected_before_loading_credentials() {
        let args = vec!["unknown".to_string(), "--acceptance-runtime".to_string()];
        assert!(parse_arguments(&args).is_err());
    }

    #[test]
    fn migration_acceptance_mutation_requires_the_closed_runtime() {
        let missing_authority = vec!["migration-acceptance-migrate".to_string()];
        assert!(parse_arguments(&missing_authority).is_err());

        let accepted = vec![
            "migration-acceptance-migrate".to_string(),
            "--acceptance-runtime".to_string(),
        ];
        let (action, migrations_dir, acceptance_runtime) = parse_arguments(&accepted).unwrap();
        assert_eq!(
            action,
            DatabaseAction::PostgresMigrationAcceptance(PostgresMigrationAcceptanceAction::Migrate)
        );
        assert!(migrations_dir.is_none());
        assert!(acceptance_runtime);
    }

    #[test]
    fn migration_acceptance_commands_reject_caller_selected_migration_directories() {
        for action in [
            "migration-acceptance-status",
            "migration-acceptance-migrate",
            "migration-acceptance-verify",
        ] {
            let args = vec![
                action.to_string(),
                "--migrations-dir".to_string(),
                "/tmp/caller-selected".to_string(),
                "--acceptance-runtime".to_string(),
            ];
            assert!(parse_arguments(&args).is_err(), "{action} accepted a path");
        }
    }
}

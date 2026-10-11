//! Canonical database initialization, forward migration, and verification commands.

use acceptance_runtime::AcceptanceRuntime;
use anyhow::{Context, Result, bail};

use crate::database_coordinator::{self, DatabaseCoordinatorAction};

const USAGE: &str = "usage: cargo tools database <initialize|migrate|verify> [--acceptance-runtime] (initialize and migrate read PLE_MIGRATION_DATABASE_URL; verify reads DATABASE_URL)";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum DatabaseAction {
    Initialize,
    Migrate,
    Verify,
}

impl DatabaseAction {
    fn parse(value: &str) -> Option<Self> {
        match value {
            "initialize" => Some(Self::Initialize),
            "migrate" => Some(Self::Migrate),
            "verify" => Some(Self::Verify),
            _ => None,
        }
    }
}

enum DatabaseConnection {
    Acceptance(AcceptanceRuntime),
    Environment(String),
}

/// Runs one canonical database lifecycle command.
pub fn run(args: &[String]) -> Result<()> {
    if matches!(args, [flag] if flag == "--help" || flag == "-h") {
        println!("{USAGE}");
        return Ok(());
    }
    let (action, acceptance_runtime) = parse_arguments(args)?;
    let connection = database_connection_for(action, acceptance_runtime)?;
    let tokio_runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .context("creating the database administration runtime")?;
    tokio_runtime.block_on(async {
        let database_url = match &connection {
            DatabaseConnection::Acceptance(runtime) => runtime.migration_url().expose(),
            DatabaseConnection::Environment(url) => url,
        };
        let pool = learning_data_access::postgres::lazy_pool(database_url)
            .context("database administration URL is not a valid PostgreSQL connection URL")?;
        match action {
            DatabaseAction::Initialize => {
                database_coordinator::run(
                    DatabaseCoordinatorAction::Initialize,
                    &pool,
                    database_url,
                )
                .await
            }
            DatabaseAction::Migrate => {
                database_coordinator::run(DatabaseCoordinatorAction::Migrate, &pool, database_url)
                    .await
            }
            DatabaseAction::Verify => {
                learning_data_access::postgres::verify_application_schema(&pool)
                    .await
                    .context("verifying the application database schema")?;
                println!("database verify: compatible");
                Ok(())
            }
        }
    })
}

fn parse_arguments(args: &[String]) -> Result<(DatabaseAction, bool)> {
    let action = args
        .first()
        .and_then(|value| DatabaseAction::parse(value))
        .ok_or_else(|| anyhow::anyhow!(USAGE))?;
    let acceptance_runtime = match &args[1..] {
        [] => false,
        [flag] if flag == "--acceptance-runtime" && action != DatabaseAction::Verify => true,
        _ => bail!(USAGE),
    };
    Ok((action, acceptance_runtime))
}

fn database_connection_for(
    action: DatabaseAction,
    acceptance_runtime: bool,
) -> Result<DatabaseConnection> {
    if acceptance_runtime {
        return AcceptanceRuntime::load()
            .map(DatabaseConnection::Acceptance)
            .map_err(|error| anyhow::anyhow!("database acceptance runtime is required: {error}"));
    }
    let value = environment_database_url_for(action, |name| std::env::var(name))?;
    Ok(DatabaseConnection::Environment(value))
}

fn environment_database_url_for(
    action: DatabaseAction,
    mut lookup: impl for<'a> FnMut(&'a str) -> std::result::Result<String, std::env::VarError>,
) -> Result<String> {
    let (name, purpose) = match action {
        DatabaseAction::Initialize | DatabaseAction::Migrate => (
            "PLE_MIGRATION_DATABASE_URL",
            "database initialization or migration",
        ),
        // ASVS 8.3.1: schema verification runs through the restricted
        // application connection, never through the migrator's authority.
        DatabaseAction::Verify => ("DATABASE_URL", "application schema verification"),
    };
    let value = lookup(name).with_context(|| format!("{name} must be set for {purpose}"))?;
    if value.trim().is_empty() {
        bail!("{name} must not be empty");
    }
    Ok(value)
}

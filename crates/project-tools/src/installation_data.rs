//! One bounded installation action for the optional ordinary Live Demo graph.

use std::collections::BTreeMap;
use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use anyhow::{Context, Result, bail, ensure};
use learning_data_access::{SessionId, SessionTokenHash};
use question_model::WorkspaceId;
use uuid::Uuid;

use crate::libpq_environment::LibpqEnvironment;
use crate::pilot_content;

const USAGE: &str = "usage: cargo tools installation-data <apply|provision [--without-live-demo]>";
const PSQL_EXECUTABLE: &str = "/usr/bin/psql";
const IMAGE_SCHEMA_ROOT: &str = "/opt/ple/schemas/installation_data";
const PILOT_WORKSPACE_ID: &str = "00000000-0000-0000-0000-000000000201";

pub(crate) const LIVE_DEMO_ELENA_ACCOUNT_ID: &str = "00000000-0000-0000-0000-000000000101";
pub(crate) const LIVE_DEMO_MARY_ACCOUNT_ID: &str = "00000000-0000-0000-0000-000000000102";
pub(crate) const LIVE_DEMO_JACK_ACCOUNT_ID: &str = "00000000-0000-0000-0000-000000000103";
pub(crate) const LIVE_DEMO_AVERY_ACCOUNT_ID: &str = "00000000-0000-0000-0000-000000000104";
pub(crate) const LIVE_DEMO_COURSE_SHORT_NAME: &str = "BCHM 301";
pub(crate) const LIVE_DEMO_COURSE_LONG_NAME: &str = "Biochemistry 301: Proteins and Peptides";
pub(crate) const LIVE_DEMO_ASSIGNMENT_TITLE: &str = "Chapter 1 Pilot Practice";
pub(crate) const LIVE_DEMO_MARY_ROSTER_ID: &str = "BIO301-MARY";

/// Runs the fixed optional teaching-data installation operation.
pub(crate) fn run(args: &[String]) -> Result<()> {
    if matches!(args, [flag] if flag == "--help" || flag == "-h") {
        println!("{USAGE}");
        return Ok(());
    }
    run_command(parse_arguments(args)?, apply)
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum InstallationDataCommand {
    Apply,
    Provision { include_live_demo: bool },
}

fn parse_arguments(args: &[String]) -> Result<InstallationDataCommand> {
    match args {
        [action] if action == "apply" => Ok(InstallationDataCommand::Apply),
        [action] if action == "provision" => Ok(InstallationDataCommand::Provision {
            include_live_demo: true,
        }),
        [action, flag] if action == "provision" && flag == "--without-live-demo" => {
            Ok(InstallationDataCommand::Provision {
                include_live_demo: false,
            })
        }
        _ => bail!("{USAGE}"),
    }
}

fn run_command<F>(command: InstallationDataCommand, apply: F) -> Result<()>
where
    F: FnOnce() -> Result<()>,
{
    match command {
        InstallationDataCommand::Apply => apply(),
        InstallationDataCommand::Provision {
            include_live_demo: false,
        } => {
            println!("installation-data: Live Demo provisioning skipped");
            Ok(())
        }
        InstallationDataCommand::Provision {
            include_live_demo: true,
        } => {
            apply()?;
            crate::installation_data_activity::provision()
        }
    }
}

/// Applies the Pilot publication and PostgreSQL-owned Live Demo graph.
fn apply() -> Result<()> {
    validate_publisher_environment()?;

    let migration_database_url = required_environment("PLE_MIGRATION_DATABASE_URL")?;
    let session_id = SessionId::generate().map_err(anyhow::Error::msg)?;
    let session_id_text = session_id.as_uuid().to_string();
    let token_hash = fresh_session_token_hash()?;
    let workspace = WorkspaceId::from_uuid(
        Uuid::parse_str(PILOT_WORKSPACE_ID).expect("the fixed Pilot workspace ID is valid"),
    );

    run_manifest(
        &migration_database_url,
        "prepublication_context.sql",
        &BTreeMap::from([
            ("pilot_publication_session_id", session_id_text.clone()),
            (
                "pilot_publication_session_token_hash",
                token_hash.to_string(),
            ),
        ]),
    )?;
    let publications = pilot_content::publish_with_context(token_hash, workspace)
        .context("publishing the ordinary Pilot Question set")?;
    pilot_content::validate_publication_mapping_json(&publications)
        .context("validating the ordinary Pilot publication mapping")?;
    run_manifest(
        &migration_database_url,
        "install.sql",
        &BTreeMap::from([
            ("pilot_publication_session_id", session_id_text),
            ("pilot_question_publications", publications.clone()),
        ]),
    )?;
    println!("{publications}");
    Ok(())
}

fn fresh_session_token_hash() -> Result<SessionTokenHash> {
    let mut token = [0_u8; 32];
    getrandom::getrandom(&mut token)
        .map_err(|_| anyhow::anyhow!("generating the temporary Pilot session token failed"))?;
    Ok(hash_session_token(token))
}

fn hash_session_token(token: [u8; 32]) -> SessionTokenHash {
    SessionTokenHash::compute(&token)
}

fn validate_publisher_environment() -> Result<()> {
    for name in [
        "DATABASE_URL",
        "PLE_STORAGE_TOPOLOGY",
        "PLE_S3_REGION",
        "PLE_PUBLIC_ASSETS_BUCKET",
        "PLE_PRIVATE_CONTENT_BUCKET",
        "PLE_STUDENT_RECORDS_BUCKET",
        "PLE_TEMP_PROCESSING_BUCKET",
        "PLE_QUESTION_ID_SECRET_FILE",
    ] {
        required_environment(name)?;
    }
    if std::env::var("PLE_STORAGE_TOPOLOGY").ok().as_deref() == Some("disposable-local") {
        for name in [
            "PLE_S3_ENDPOINT",
            "AWS_ACCESS_KEY_ID",
            "AWS_SECRET_ACCESS_KEY",
        ] {
            required_environment(name)?;
        }
    }
    Ok(())
}

fn required_environment(name: &str) -> Result<String> {
    let value = std::env::var(name).with_context(|| format!("{name} must be set"))?;
    ensure!(
        !value.is_empty() && !value.contains(['\0', '\n', '\r']),
        "{name} must not be empty"
    );
    Ok(value)
}

fn run_manifest(
    database_url: &str,
    filename: &str,
    variables: &BTreeMap<&str, String>,
) -> Result<()> {
    let manifest = fixed_manifest_path(filename)?;
    let environment =
        LibpqEnvironment::from_database_url(database_url, "installation database URL")?;
    let script = psql_script(&manifest, variables)?;
    let mut child = Command::new(PSQL_EXECUTABLE)
        .env_clear()
        .env(
            "PATH",
            "/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin",
        )
        .envs(environment.variables())
        .arg("-X")
        .arg("--set=ON_ERROR_STOP=1")
        .arg("--single-transaction")
        .stdout(Stdio::null())
        .stdin(Stdio::piped())
        .spawn()
        .context("starting the fixed installation-data PostgreSQL command")?;
    child
        .stdin
        .take()
        .context("opening installation-data PostgreSQL input")?
        .write_all(script.as_bytes())
        .context("supplying the fixed installation-data PostgreSQL input")?;
    let status = child
        .wait()
        .context("waiting for the fixed installation-data PostgreSQL command")?;
    ensure!(
        status.success(),
        "fixed installation-data PostgreSQL command failed ({status})"
    );
    Ok(())
}

fn fixed_manifest_path(filename: &str) -> Result<PathBuf> {
    ensure!(
        matches!(filename, "prepublication_context.sql" | "install.sql"),
        "installation-data manifest is not recognized"
    );
    let source_root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .context("locating the repository root from project-tools")?
        .join("schemas/installation_data");
    for root in [PathBuf::from(IMAGE_SCHEMA_ROOT), source_root] {
        let Ok(root) = root.canonicalize() else {
            continue;
        };
        let path = root.join(filename);
        let Ok(path) = path.canonicalize() else {
            continue;
        };
        if path.is_file() && path.strip_prefix(&root).is_ok() {
            return Ok(path);
        }
    }
    bail!("the fixed installation-data manifest is unavailable")
}

fn psql_script(manifest: &Path, variables: &BTreeMap<&str, String>) -> Result<String> {
    let mut script = String::new();
    for (name, value) in variables {
        ensure!(
            name.bytes()
                .all(|byte| byte.is_ascii_lowercase() || byte == b'_'),
            "installation-data variable name is invalid"
        );
        ensure!(
            psql_value_is_safe(value),
            "installation-data variable is invalid"
        );
        script.push_str("\\set ");
        script.push_str(name);
        script.push_str(" '");
        script.push_str(value);
        script.push_str("'\n");
    }
    let rendered_manifest = manifest
        .to_str()
        .context("installation-data manifest path is not UTF-8")?;
    ensure!(
        rendered_manifest.starts_with(IMAGE_SCHEMA_ROOT)
            || rendered_manifest.ends_with("/schemas/installation_data/prepublication_context.sql")
            || rendered_manifest.ends_with("/schemas/installation_data/install.sql"),
        "installation-data manifest path is invalid"
    );
    script.push_str("\\ir ");
    script.push_str(rendered_manifest);
    script.push('\n');
    Ok(script)
}

fn psql_value_is_safe(value: &str) -> bool {
    !value.is_empty()
        && value.bytes().all(|byte| {
            byte.is_ascii_alphanumeric()
                || matches!(
                    byte,
                    b'-' | b'_' | b'{' | b'}' | b'[' | b']' | b':' | b',' | b'"'
                )
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_only_the_fixed_installation_commands() {
        assert_eq!(
            parse_arguments(&["apply".to_owned()]).unwrap(),
            InstallationDataCommand::Apply
        );
        assert_eq!(
            parse_arguments(&["provision".to_owned()]).unwrap(),
            InstallationDataCommand::Provision {
                include_live_demo: true
            }
        );
        assert_eq!(
            parse_arguments(&["provision".to_owned(), "--without-live-demo".to_owned()]).unwrap(),
            InstallationDataCommand::Provision {
                include_live_demo: false
            }
        );
        assert!(parse_arguments(&[]).is_err());
        assert!(parse_arguments(&["apply".to_owned(), "--file".to_owned()]).is_err());
    }

    #[test]
    fn opt_out_returns_before_environment_reads_or_mutation() {
        let applied = std::cell::Cell::new(false);
        run_command(
            InstallationDataCommand::Provision {
                include_live_demo: false,
            },
            || {
                applied.set(true);
                bail!("apply must not run")
            },
        )
        .unwrap();
        assert!(!applied.get());
    }

    #[test]
    fn psql_variables_reject_control_and_quote_injection() {
        assert!(psql_value_is_safe("ABC-123"));
        assert!(psql_value_is_safe(r#"{"sourceSha256":"a1"}"#));
        assert!(!psql_value_is_safe("bad'quote"));
        assert!(!psql_value_is_safe("bad\nline"));
    }

    #[test]
    fn final_manifest_receives_the_session_and_exact_publication_mapping() {
        let script = psql_script(
            Path::new("/opt/ple/schemas/installation_data/install.sql"),
            &BTreeMap::from([
                (
                    "pilot_publication_session_id",
                    "00000000-0000-0000-0000-000000000001".to_string(),
                ),
                (
                    "pilot_question_publications",
                    r#"{"pilot":{"sourceSha256":"abc","questionRevision":{"questionId":"ABC-1234","revisionNumber":1}}}"#.to_string(),
                ),
            ]),
        )
        .unwrap();
        assert!(script.contains("\\set pilot_publication_session_id"));
        assert!(script.contains("\\set pilot_question_publications"));
        assert!(script.ends_with("\\ir /opt/ple/schemas/installation_data/install.sql\n"));
    }

    #[test]
    fn session_hash_is_the_full_sha256_of_all_opaque_token_bytes() {
        let mut token = [0_u8; 32];
        token[31] = 1;
        assert_ne!(hash_session_token([0_u8; 32]), hash_session_token(token));
        assert_eq!(hash_session_token(token).to_string().len(), 64);
    }
}

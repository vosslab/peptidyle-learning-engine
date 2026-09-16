//! Installation publication for bundled curriculum and the optional Live Demo graph.

use std::collections::BTreeMap;
use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use anyhow::{Context, Result, bail, ensure};
use learning_data_access::{
    BlueprintCourseStore, SessionId, SessionTokenHash,
    postgres::{PostgresBlueprintCourseStore, lazy_pool},
};
use question_model::{
    BlueprintAvailability, BlueprintCourseReadAccess, BlueprintCourseReference, WorkspaceId,
};
use uuid::Uuid;

use crate::libpq_environment::LibpqEnvironment;
use crate::{curriculum_content, pilot_content};

const USAGE: &str = "usage: cargo tools installation-data <apply|provision [--without-live-demo]>";
const PSQL_EXECUTABLE: &str = "/usr/bin/psql";
const IMAGE_SCHEMA_ROOT: &str = "/opt/ple/schemas/installation_data";
const IMAGE_GENETICS_CONTENT_ROOT: &str = "/opt/ple/content/genetics";
const BUNDLED_GENETICS_MANIFEST: &str = "manifest.yaml";
const PILOT_WORKSPACE_ID: &str = "00000000-0000-0000-0000-000000000201";
const EXAMPLE_CONTENT_WORKSPACE_ID: &str = "00000000-0000-0000-0000-000000000202";

pub(crate) const LIVE_DEMO_ELENA_ACCOUNT_ID: &str = "00000000-0000-0000-0000-000000000101";
pub(crate) const LIVE_DEMO_MARY_ACCOUNT_ID: &str = "00000000-0000-0000-0000-000000000102";
pub(crate) const LIVE_DEMO_JACK_ACCOUNT_ID: &str = "00000000-0000-0000-0000-000000000103";
pub(crate) const LIVE_DEMO_AVERY_ACCOUNT_ID: &str = "00000000-0000-0000-0000-000000000104";
pub(crate) const LIVE_DEMO_COURSE_SHORT_NAME: &str = "BCHM 301";
pub(crate) const LIVE_DEMO_COURSE_LONG_NAME: &str = "Biochemistry 301: Proteins and Peptides";
pub(crate) const LIVE_DEMO_ASSESSMENT_TITLE: &str = "Chapter 1 Pilot Practice";

/// Runs the fixed bundled curriculum and optional teaching-data installation operation.
pub(crate) fn run(args: &[String]) -> Result<()> {
    if matches!(args, [flag] if flag == "--help" || flag == "-h") {
        println!("{USAGE}");
        return Ok(());
    }
    run_command(parse_arguments(args)?)
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

fn run_command(command: InstallationDataCommand) -> Result<()> {
    validate_publisher_environment()?;
    // ASVS 2.3.1: bundled authorship declares its vocabulary before either
    // ordinary publisher validates Question classification.
    run_manifest(
        &required_environment("PLE_MIGRATION_DATABASE_URL")?,
        "content_vocabulary.sql",
        &BTreeMap::new(),
    )
    .context("provisioning the declared bundled content vocabulary")?;
    match command {
        InstallationDataCommand::Apply => {
            // Preserve the long-standing Live Demo identifiers on a default
            // installation before adding the reusable bundled Blueprint.
            apply_live_demo()?;
            apply_bundled_genetics()
        }
        InstallationDataCommand::Provision {
            include_live_demo: false,
        } => {
            apply_bundled_genetics()?;
            println!("installation-data: Live Demo provisioning skipped");
            Ok(())
        }
        InstallationDataCommand::Provision {
            include_live_demo: true,
        } => {
            apply_live_demo()?;
            apply_bundled_genetics()?;
            crate::installation_data_activity::provision()
        }
    }
}

/// Applies the Pilot publication and PostgreSQL-owned Live Demo graph.
fn apply_live_demo() -> Result<()> {
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
    let live_demo_blueprint =
        crate::installation_data_blueprint::create_live_demo_blueprint(token_hash, &publications)
            .context("creating the ordinary Live Demo Blueprint Course")?;
    ensure!(
        Uuid::parse_str(&live_demo_blueprint.assessment_reference)
            .is_ok_and(
                |value| value.hyphenated().to_string() == live_demo_blueprint.assessment_reference
            ),
        "Live Demo Blueprint Store receipt is not a canonical Assessment reference"
    );
    run_manifest(
        &migration_database_url,
        "install.sql",
        &BTreeMap::from([
            ("pilot_publication_session_id", session_id_text),
            ("pilot_question_publications", publications.clone()),
            (
                "live_demo_blueprint_public_reference",
                live_demo_blueprint.blueprint_public_reference,
            ),
            (
                "live_demo_blueprint_assessment_reference",
                live_demo_blueprint.assessment_reference,
            ),
        ]),
    )?;
    println!("{publications}");
    Ok(())
}

/// Publishes the shipped Genetics Blueprint through the ordinary authoring and
/// Question-publication path. The temporary capability exists only for this
/// installation operation and is revoked even when publication fails.
fn apply_bundled_genetics() -> Result<()> {
    validate_publisher_environment()?;

    let migration_database_url = required_environment("PLE_MIGRATION_DATABASE_URL")?;
    let session_id = SessionId::generate().map_err(anyhow::Error::msg)?;
    let session_id_text = session_id.as_uuid().to_string();
    let token_hash = fresh_session_token_hash()?;
    run_manifest(
        &migration_database_url,
        "bundled_curriculum_context.sql",
        &BTreeMap::from([
            ("example_content_session_id", session_id_text.clone()),
            ("example_content_session_token_hash", token_hash.to_string()),
        ]),
    )?;

    let publication = publish_bundled_genetics(token_hash);
    let cleanup = run_manifest(
        &migration_database_url,
        "revoke_example_content_session.sql",
        &BTreeMap::from([("example_content_session_id", session_id_text)]),
    );
    match (publication, cleanup) {
        (Ok(receipt), Ok(())) => {
            println!("{}", receipt.installation_summary());
            Ok(())
        }
        (Err(publication), Ok(())) => Err(publication),
        (Ok(_), Err(cleanup)) => Err(cleanup.context("revoking the temporary PLE Example Content session")),
        (Err(publication), Err(cleanup)) => Err(publication.context(format!(
            "publishing bundled Genetics failed; the temporary PLE Example Content session also could not be revoked: {cleanup:#}"
        ))),
    }
}

fn publish_bundled_genetics(
    session: SessionTokenHash,
) -> Result<curriculum_content::publication::Receipt> {
    let root = fixed_genetics_content_root()?;
    let manifest = curriculum_content::load_from_root(&root, Path::new(BUNDLED_GENETICS_MANIFEST))
        .context("loading the fixed bundled Genetics curriculum")?;
    let workspace = WorkspaceId::from_uuid(
        Uuid::parse_str(EXAMPLE_CONTENT_WORKSPACE_ID)
            .expect("the fixed PLE Example Content workspace ID is valid"),
    );
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .context("creating the bundled Genetics publication runtime")?;
    runtime.block_on(async move {
        let receipt = curriculum_content::publication::publish_with_context(
            session, workspace, manifest, &root,
        )
        .await
        .context("publishing the bundled Genetics Blueprint through the ordinary publisher")?;
        let reference = BlueprintCourseReference::new(receipt.blueprint_reference())
            .map_err(anyhow::Error::msg)
            .context("resolving the bundled Genetics Blueprint receipt")?;
        let database_url = required_environment("DATABASE_URL")?;
        let pool = lazy_pool(&database_url)
            .context("bundled Genetics Blueprint database URL is invalid")?;
        let store = PostgresBlueprintCourseStore::new(pool);
        let blueprint = store
            .load_blueprint_course(session, reference)
            .await
            .context("loading the retained bundled Genetics Blueprint")?;
        // ASVS 8.2.2, 2.3.1: only the installation publisher's validated
        // retained example is made Public, through its ordinary owner API.
        ensure!(
            blueprint.read_access == BlueprintCourseReadAccess::BlueprintCourseOwner,
            "bundled Genetics Blueprint is not owned by the installation publisher"
        );
        ensure!(
            matches!(
                blueprint.availability,
                BlueprintAvailability::Private | BlueprintAvailability::Public
            ),
            "bundled Genetics Blueprint is neither Private nor Public"
        );
        if blueprint.availability == BlueprintAvailability::Private {
            store
                .publish_blueprint(session, reference, blueprint.metadata_etag)
                .await
                .context("making the bundled Genetics example Blueprint Public")?;
        }
        let published = store
            .load_blueprint_course(session, reference)
            .await
            .context("reloading the Public bundled Genetics Blueprint")?;
        ensure!(
            published.availability == BlueprintAvailability::Public
                && published.read_access == BlueprintCourseReadAccess::BlueprintCourseOwner
                && published.current_revision == blueprint.current_revision,
            "bundled Genetics publication changed ownership or Revision, or is not Public"
        );
        Ok(receipt)
    })
}

fn fixed_genetics_content_root() -> Result<PathBuf> {
    let source_root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .context("locating the repository root from project-tools")?
        .join("content/genetics");
    for root in [PathBuf::from(IMAGE_GENETICS_CONTENT_ROOT), source_root] {
        let Ok(root) = root.canonicalize() else {
            continue;
        };
        if root.is_dir() {
            return Ok(root);
        }
    }
    bail!("the fixed bundled Genetics content root is unavailable")
}

fn fresh_session_token_hash() -> Result<SessionTokenHash> {
    // ASVS 7.2.2, 7.2.3: every publisher capability starts as fresh CSPRNG
    // material; no static installation credential is retained or logged.
    let mut token = [0_u8; 32];
    getrandom::fill(&mut token)
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
        matches!(
            filename,
            "prepublication_context.sql"
                | "content_vocabulary.sql"
                | "install.sql"
                | "bundled_curriculum_context.sql"
                | "revoke_example_content_session.sql"
        ),
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
    // ASVS 1.2.4: this is a fixed trusted manifest; only a deliberately small
    // opaque-value alphabet may cross the psql variable boundary.
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
            || [
                "prepublication_context.sql",
                "content_vocabulary.sql",
                "install.sql",
                "bundled_curriculum_context.sql",
                "revoke_example_content_session.sql",
            ]
            .iter()
            .any(|name| rendered_manifest.ends_with(&format!("/schemas/installation_data/{name}"))),
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
                    r#"{"pilot":{"sourceSha256":"abc","questionRevision":{"questionId":"ABC1-X234","revisionNumber":1}}}"#.to_string(),
                ),
                (
                    "live_demo_blueprint_public_reference",
                    "BP00000C".to_string(),
                ),
                (
                    "live_demo_blueprint_assessment_reference",
                    "00000000-0000-0000-0000-000000000012".to_string(),
                ),
            ]),
        )
        .unwrap();
        assert!(script.contains("\\set pilot_publication_session_id"));
        assert!(script.contains("\\set pilot_question_publications"));
        assert!(script.contains("\\set live_demo_blueprint_public_reference 'BP00000C'"));
        assert!(script.contains(
            "\\set live_demo_blueprint_assessment_reference '00000000-0000-0000-0000-000000000012'"
        ));
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

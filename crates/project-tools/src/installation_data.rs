//! Trusted installer context around the shared, ordinary HTTP curriculum importer.

use std::collections::BTreeMap;
use std::io::{Read as _, Write as _};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use anyhow::{Context, Result, bail, ensure};
use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use learning_data_access::{SessionId, SessionTokenHash};
use serde::{Deserialize, Serialize};

use crate::libpq_environment::LibpqEnvironment;

const USAGE: &str =
    "usage: cargo tools installation-data <prepare [--without-live-demo]|finalize|revoke>";
const PSQL_EXECUTABLE: &str = "/usr/bin/psql";
const IMAGE_SCHEMA_ROOT: &str = "/opt/ple/schemas/installation_data";

pub(crate) const LIVE_DEMO_ELENA_EMAIL: &str = "elena.martinez@live-demo.invalid";
pub(crate) const LIVE_DEMO_MARY_EMAIL: &str = "mary.okafor@biology.roosevelt.edu";
pub(crate) const LIVE_DEMO_JACK_EMAIL: &str = "jack.nguyen@biology.roosevelt.edu";
pub(crate) const LIVE_DEMO_AVERY_EMAIL: &str = "avery.thompson@biology.roosevelt.edu";
pub(crate) const LIVE_DEMO_COURSE_SHORT_NAME: &str = "BCHM 355";
pub(crate) const LIVE_DEMO_COURSE_LONG_NAME: &str = "Biochemistry";

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct ImportSession {
    session_id: uuid::Uuid,
    session_cookie: String,
    include_live_demo: bool,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct FinalizeInput {
    session: ImportSession,
    import_result: ImportResult,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ImportResult {
    blueprints: BTreeMap<String, question_model::BlueprintCourseId>,
    questions: u64,
    pools: u64,
    #[serde(rename = "pointDefaults")]
    _point_defaults: Vec<BTreeMap<String, String>>,
    complete: bool,
    errors: Vec<String>,
}

impl ImportResult {
    fn validate(&self) -> Result<()> {
        // ASVS 2.2.1, 2.3.1: adopt only a completed, error-free curriculum import.
        ensure!(
            self.complete && self.errors.is_empty(),
            "curriculum import is incomplete or reported errors"
        );
        ensure!(
            self.questions > 0 && self.pools > 0,
            "curriculum import returned no Questions or Pools"
        );
        ensure!(
            self.blueprints.len() == 4,
            "curriculum import must return all four Blueprint Courses"
        );
        for course in ["genetics", "biochemistry", "biotechnology", "biostatistics"] {
            ensure!(
                self.blueprints.contains_key(course),
                "curriculum Blueprint is missing"
            );
        }
        Ok(())
    }
}

/// Runs private installer phases; the host owns Python, source discovery, and HTTP import.
pub(crate) fn run(args: &[String]) -> Result<()> {
    match args {
        [flag] if flag == "--help" || flag == "-h" => println!("{USAGE}"),
        [action] if action == "prepare" => prepare(true)?,
        [action, flag] if action == "prepare" && flag == "--without-live-demo" => prepare(false)?,
        [action] if action == "finalize" => finalize(read_input()?)?,
        [action] if action == "revoke" => revoke(&read_input::<ImportSession>()?)?,
        _ => bail!("{USAGE}"),
    }
    Ok(())
}

fn read_input<T: serde::de::DeserializeOwned>() -> Result<T> {
    // ASVS 2.2.1, 13.3.2: credentials cross a bounded private stdin pipe, never argv.
    let mut input = String::new();
    std::io::stdin()
        .take(16 * 1024)
        .read_to_string(&mut input)?;
    serde_json::from_str(&input).context("installer input is invalid")
}

fn prepare(include_live_demo: bool) -> Result<()> {
    let database = required_environment("PLE_MIGRATION_DATABASE_URL")?;
    run_manifest(&database, "content_vocabulary.sql", &BTreeMap::new())?;
    // ASVS 7.2.2, 7.2.3: each installation import gets fresh CSPRNG session material.
    let mut token = [0_u8; 32];
    getrandom::fill(&mut token).map_err(|_| anyhow::anyhow!("generating import session failed"))?;
    let session_id = SessionId::generate()
        .map_err(anyhow::Error::msg)?
        .as_uuid()
        .to_owned();
    let token_hash = SessionTokenHash::compute(&token);
    let (manifest, id_key, hash_key) = if include_live_demo {
        (
            "prepublication_context.sql",
            "pilot_publication_session_id",
            "pilot_publication_session_token_hash",
        )
    } else {
        (
            "bundled_curriculum_context.sql",
            "example_content_session_id",
            "example_content_session_token_hash",
        )
    };
    run_manifest(
        &database,
        manifest,
        &BTreeMap::from([
            (id_key, session_id.to_string()),
            (hash_key, token_hash.to_string()),
        ]),
    )?;
    let token_wire = URL_SAFE_NO_PAD.encode(token);
    // This stdout is captured only by the trusted host coordinator.
    println!(
        "{}",
        serde_json::to_string(&ImportSession {
            session_id,
            session_cookie: format!("__Host-ple_session={token_wire}"),
            include_live_demo,
        })?
    );
    Ok(())
}

fn finalize(input: FinalizeInput) -> Result<()> {
    input.import_result.validate()?;
    if input.session.include_live_demo {
        let blueprint = &input.import_result.blueprints["biochemistry"];
        let course = crate::installation_data_activity::adopt_blueprint(
            &input.session.session_cookie,
            blueprint,
        )?;
        run_manifest(
            &required_environment("PLE_MIGRATION_DATABASE_URL")?,
            "install.sql",
            &BTreeMap::from([("live_demo_course_instance_id", course)]),
        )?;
        crate::installation_data_activity::provision()?;
    }
    // Finalization never needs the import credential after adoption.
    revoke(&input.session)
}

fn revoke(session: &ImportSession) -> Result<()> {
    let encoded = session
        .session_cookie
        .strip_prefix("__Host-ple_session=")
        .context("import session cookie is invalid")?;
    let token: [u8; 32] = URL_SAFE_NO_PAD
        .decode(encoded)
        .context("import session cookie is invalid")?
        .try_into()
        .map_err(|_| anyhow::anyhow!("import session cookie is invalid"))?;
    run_manifest(
        &required_environment("PLE_MIGRATION_DATABASE_URL")?,
        "revoke_import_session.sql",
        &BTreeMap::from([
            ("import_session_id", session.session_id.to_string()),
            (
                "import_session_token_hash",
                SessionTokenHash::compute(&token).to_string(),
            ),
        ]),
    )
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
                | "revoke_import_session.sql"
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
                "revoke_import_session.sql",
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
    fn python_import_report_requires_completed_error_free_curriculum() {
        let blueprints: BTreeMap<_, _> =
            ["genetics", "biochemistry", "biotechnology", "biostatistics"]
                .into_iter()
                .enumerate()
                .map(|(index, name)| {
                    (
                        name,
                        question_model::BlueprintCourseId::from_debug_serial(index as u128),
                    )
                })
                .collect();
        let mut report = serde_json::json!({
            "blueprints": blueprints,
            "questions": 8,
            "pools": 4,
            "pointDefaults": [{
                "source": "matching.pg",
                "points": "1",
                "reason": "Algorithmic choice count is not exposed in source metadata"
            }],
            "complete": true,
            "errors": []
        });
        let completed: ImportResult = serde_json::from_value(report.clone()).unwrap();
        completed.validate().unwrap();

        for (complete, errors) in [
            (false, serde_json::json!([])),
            (true, serde_json::json!(["publication failed"])),
        ] {
            report["complete"] = complete.into();
            report["errors"] = errors;
            let result: ImportResult = serde_json::from_value(report.clone()).unwrap();
            assert_eq!(
                result.validate().unwrap_err().to_string(),
                "curriculum import is incomplete or reported errors"
            );
        }
    }
}

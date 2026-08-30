//! Private runtime boundary for the disposable SD1 staged database oracle.

use std::fmt;
#[cfg(unix)]
use std::path::Path;

#[cfg(unix)]
use serde::Deserialize;

#[cfg(unix)]
use crate::{
    Identity, MANIFEST_NAME, MAX_MANIFEST_BYTES, MAX_URL_BYTES, OWNER, PROJECT,
    open_private_directory, open_private_directory_at, parse_database_url, read_private_file_at,
    reject_yaml_extensions, validate_locator,
};
use crate::{PostgresUrl, RuntimeError};

#[cfg(unix)]
const KIND: &str = "ple.sd1_staged_database_acceptance";
#[cfg(unix)]
const PROFILE: &str = "sd1_staged_database";
#[cfg(unix)]
const MIGRATOR_ROLE: &str = "ple_migrator";
#[cfg(unix)]
const MIGRATOR_URL_PATH: &str = "secrets/postgres-migrator.url";

/// Validated private runtime for the disposable SD1 staged database oracle.
pub struct Sd1StagedDatabaseRuntime {
    postgres_migrator_url: PostgresUrl,
}

impl Sd1StagedDatabaseRuntime {
    /// Loads the SD1 runtime selected by its absolute non-secret manifest locator.
    pub fn load() -> Result<Self, RuntimeError> {
        #[cfg(not(unix))]
        {
            Err(RuntimeError::UnsupportedPlatform)
        }
        #[cfg(unix)]
        {
            let locator = std::env::var("PLE_ACCEPTANCE_RUNTIME_MANIFEST")
                .map_err(|_| RuntimeError::Locator)?;
            load_from_locator_unix(Path::new(&locator))
        }
    }

    /// Returns the validated URL for the exact non-runtime SD1 migrator.
    pub fn postgres_migrator_url(&self) -> &PostgresUrl {
        &self.postgres_migrator_url
    }
}

impl fmt::Debug for Sd1StagedDatabaseRuntime {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("Sd1StagedDatabaseRuntime(REDACTED)")
    }
}

#[cfg(unix)]
pub(super) fn load_from_locator_unix(
    locator: &Path,
) -> Result<Sd1StagedDatabaseRuntime, RuntimeError> {
    let workspace_path = validate_locator(locator)?;
    let workspace = open_private_directory(workspace_path, RuntimeError::Workspace)?;
    let manifest = read_private_file_at(
        &workspace,
        MANIFEST_NAME,
        MAX_MANIFEST_BYTES,
        RuntimeError::Manifest,
    )?;
    reject_yaml_extensions(&manifest)?;
    // ASVS 1.5.2, 2.2.1, and 15.3.3: deserialize one closed manifest shape.
    let manifest: Sd1RuntimeManifest =
        serde_yaml_ng::from_slice(&manifest).map_err(|_| RuntimeError::Schema)?;
    validate_manifest(&manifest)?;

    let secrets = open_private_directory_at(&workspace, "secrets", RuntimeError::SecretPath)?;
    let (postgres_migrator_url, _) = parse_database_url(
        read_private_file_at(
            &secrets,
            "postgres-migrator.url",
            MAX_URL_BYTES,
            RuntimeError::SecretFile,
        )?,
        MIGRATOR_ROLE,
    )?;
    Ok(Sd1StagedDatabaseRuntime {
        postgres_migrator_url,
    })
}

#[cfg(unix)]
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Sd1RuntimeManifest {
    schema_version: u8,
    kind: String,
    identity: Identity,
    secrets: Sd1Secrets,
}

#[cfg(unix)]
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Sd1Secrets {
    postgres_migrator_url: String,
}

#[cfg(unix)]
fn validate_manifest(manifest: &Sd1RuntimeManifest) -> Result<(), RuntimeError> {
    if manifest.schema_version != 1 {
        return Err(RuntimeError::Schema);
    }
    // ASVS 2.2.3 and 13.2.4: identity, kind, profile, and locator are one allowlist.
    if manifest.kind != KIND
        || manifest.identity.owner != OWNER
        || manifest.identity.project != PROJECT
        || manifest.identity.profile != PROFILE
    {
        return Err(RuntimeError::Identity);
    }
    if manifest.secrets.postgres_migrator_url != MIGRATOR_URL_PATH {
        return Err(RuntimeError::SecretPath);
    }
    Ok(())
}

#[cfg(all(test, unix))]
mod tests;

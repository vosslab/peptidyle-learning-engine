//! Strict private runtime loading for disposable PostgreSQL acceptance lanes.
//!
//! This crate is intentionally independent of product and domain crates. It
//! validates the generated handoff before exposing either PostgreSQL URL.

#[cfg(unix)]
use std::ffi::CString;
use std::fmt;
#[cfg(unix)]
use std::fs::{self, File, OpenOptions};
#[cfg(unix)]
use std::io::{self, Read};
#[cfg(unix)]
use std::os::fd::{AsRawFd, FromRawFd};
#[cfg(unix)]
use std::os::unix::fs::{MetadataExt, OpenOptionsExt};
use std::path::Path;

use serde::Deserialize;
use url::Url;

const MANIFEST_NAME: &str = "runtime.yaml";
const MAX_MANIFEST_BYTES: usize = 4_096;
const MAX_URL_BYTES: usize = 4_096;
const MAX_COMPOSE_ENVIRONMENT_BYTES: usize = 16_384;
const CAPABILITY_BYTES: usize = 32;
const OWNER: &str = "live-demo-browser";
const PROJECT: &str = "ple-live-demo-browser";
const DATABASE_BASELINE_PROFILE: &str = "database_baseline";
const MIGRATION_ROLE: &str = "ple_migrator";
const DATABASE_NAME: &str = "ple_e2e_baseline";
const PASSWORD_LENGTH: usize = 32;

/// A redacted failure from the runtime boundary.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuntimeError {
    UnsupportedPlatform,
    Locator,
    Workspace,
    Manifest,
    Schema,
    Identity,
    SecretPath,
    SecretFile,
    SecretContent,
    DatabaseUrl,
}

impl fmt::Display for RuntimeError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = match self {
            Self::UnsupportedPlatform => "acceptance runtime requires Unix private-file support",
            Self::Locator => "acceptance runtime manifest locator is invalid",
            Self::Workspace => "acceptance runtime workspace is invalid",
            Self::Manifest => "acceptance runtime manifest is invalid",
            Self::Schema => "acceptance runtime manifest schema is invalid",
            Self::Identity => "acceptance runtime identity is invalid",
            Self::SecretPath => "acceptance runtime secret path is invalid",
            Self::SecretFile => "acceptance runtime secret file is invalid",
            Self::SecretContent => "acceptance runtime secret content is invalid",
            Self::DatabaseUrl => "acceptance runtime database URL is invalid",
        };
        formatter.write_str(message)
    }
}

impl std::error::Error for RuntimeError {}

/// A PostgreSQL URL that is only revealed at the explicit connection boundary.
#[derive(Clone, PartialEq, Eq)]
pub struct PostgresUrl(String);

impl PostgresUrl {
    /// Reveals this already-validated URL to a database connection constructor.
    pub fn expose(&self) -> &str {
        &self.0
    }
}

impl fmt::Debug for PostgresUrl {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("PostgresUrl(REDACTED)")
    }
}

/// Validated generated input for one disposable PostgreSQL acceptance lane.
#[derive(Debug)]
pub struct AcceptanceRuntime {
    migration_url: PostgresUrl,
}

impl AcceptanceRuntime {
    /// Loads the generated runtime selected by its non-secret manifest locator.
    pub fn load() -> Result<Self, RuntimeError> {
        #[cfg(not(unix))]
        {
            return Err(RuntimeError::UnsupportedPlatform);
        }
        #[cfg(unix)]
        {
            let locator = std::env::var("PLE_ACCEPTANCE_RUNTIME_MANIFEST")
                .map_err(|_| RuntimeError::Locator)?;
            load_from_locator_unix(Path::new(&locator))
        }
    }

    pub fn migration_url(&self) -> &PostgresUrl {
        &self.migration_url
    }
}

#[cfg(unix)]
fn load_from_locator_unix(locator: &Path) -> Result<AcceptanceRuntime, RuntimeError> {
    let workspace_path = validate_locator(locator)?;
    let workspace = open_private_directory(workspace_path, RuntimeError::Workspace)?;
    load_from_workspace_descriptor(&workspace)
}

#[cfg(unix)]
fn validate_locator(locator: &Path) -> Result<&Path, RuntimeError> {
    if !locator.is_absolute() || locator.file_name().is_none_or(|name| name != MANIFEST_NAME) {
        return Err(RuntimeError::Locator);
    }
    locator.parent().ok_or(RuntimeError::Locator)
}

#[cfg(unix)]
fn load_from_workspace_descriptor(workspace: &File) -> Result<AcceptanceRuntime, RuntimeError> {
    let manifest = read_private_file_at(
        workspace,
        MANIFEST_NAME,
        MAX_MANIFEST_BYTES,
        RuntimeError::Manifest,
    )?;
    reject_yaml_extensions(&manifest)?;
    let manifest: RuntimeManifest =
        serde_yaml_ng::from_slice(&manifest).map_err(|_| RuntimeError::Schema)?;
    validate_manifest(&manifest)?;

    let secrets = open_private_directory_at(workspace, "secrets", RuntimeError::SecretPath)?;
    let _ = read_private_file_at(
        &secrets,
        "compose.env",
        MAX_COMPOSE_ENVIRONMENT_BYTES,
        RuntimeError::SecretFile,
    )?;
    let capability = read_private_file_at(
        &secrets,
        "cleanup.capability",
        CAPABILITY_BYTES,
        RuntimeError::SecretFile,
    )?;
    if capability.len() != CAPABILITY_BYTES {
        return Err(RuntimeError::SecretContent);
    }
    let (migration_url, _) = parse_database_url(
        read_private_file_at(
            &secrets,
            "postgres-migrator.url",
            MAX_URL_BYTES,
            RuntimeError::SecretFile,
        )?,
        MIGRATION_ROLE,
    )?;
    Ok(AcceptanceRuntime { migration_url })
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RuntimeManifest {
    schema_version: u8,
    kind: String,
    identity: Identity,
    secrets: Secrets,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Identity {
    owner: String,
    project: String,
    profile: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Secrets {
    compose_environment: String,
    cleanup_capability: String,
    postgres_admin_url: String,
    postgres_migrator_url: String,
    postgres_admin_password: String,
}

fn validate_manifest(manifest: &RuntimeManifest) -> Result<(), RuntimeError> {
    if manifest.schema_version != 1 {
        return Err(RuntimeError::Schema);
    }
    if manifest.identity.owner != OWNER || manifest.identity.project != PROJECT {
        return Err(RuntimeError::Identity);
    }
    if (manifest.kind.as_str(), manifest.identity.profile.as_str())
        != (
            "ple.disposable_postgres_acceptance",
            DATABASE_BASELINE_PROFILE,
        )
    {
        return Err(RuntimeError::Identity);
    }
    let shared_paths_are_exact = manifest.secrets.compose_environment == "secrets/compose.env"
        && manifest.secrets.cleanup_capability == "secrets/cleanup.capability"
        && manifest.secrets.postgres_admin_url == "secrets/postgres-admin.url"
        && manifest.secrets.postgres_migrator_url == "secrets/postgres-migrator.url"
        && manifest.secrets.postgres_admin_password == "secrets/postgres-admin.password";
    if !shared_paths_are_exact {
        return Err(RuntimeError::SecretPath);
    }
    Ok(())
}

fn reject_yaml_extensions(bytes: &[u8]) -> Result<(), RuntimeError> {
    let text = std::str::from_utf8(bytes).map_err(|_| RuntimeError::Schema)?;
    if text.contains(['&', '*', '!'])
        || text
            .lines()
            .any(|line| matches!(line.trim(), "---" | "..."))
    {
        return Err(RuntimeError::Schema);
    }
    Ok(())
}

#[cfg(unix)]
fn open_private_directory(path: &Path, error: RuntimeError) -> Result<File, RuntimeError> {
    let metadata = fs::symlink_metadata(path).map_err(|_| RuntimeError::Workspace)?;
    if !metadata.file_type().is_dir() || metadata.file_type().is_symlink() {
        return Err(RuntimeError::Workspace);
    }
    let file = OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_DIRECTORY | libc::O_NOFOLLOW)
        .open(path)
        .map_err(|_| error)?;
    validate_private_directory(&file, error)?;
    Ok(file)
}

#[cfg(unix)]
fn open_private_directory_at(
    parent: &File,
    name: &str,
    error: RuntimeError,
) -> Result<File, RuntimeError> {
    let file = openat(
        parent,
        name,
        libc::O_RDONLY | libc::O_DIRECTORY | libc::O_NOFOLLOW,
        error,
    )?;
    validate_private_directory(&file, error)?;
    Ok(file)
}

#[cfg(unix)]
fn validate_private_directory(file: &File, error: RuntimeError) -> Result<(), RuntimeError> {
    let opened = file.metadata().map_err(|_| error)?;
    if !opened.is_dir() || opened.uid() != current_uid()? || opened.mode() & 0o7777 != 0o700 {
        return Err(error);
    }
    Ok(())
}

#[cfg(unix)]
fn read_private_file_at(
    parent: &File,
    name: &str,
    maximum: usize,
    error: RuntimeError,
) -> Result<Vec<u8>, RuntimeError> {
    let mut file = openat(parent, name, libc::O_RDONLY | libc::O_NOFOLLOW, error)?;
    let opened = file.metadata().map_err(|_| error)?;
    if !opened.is_file() || opened.uid() != current_uid()? || opened.mode() & 0o7777 != 0o600 {
        return Err(error);
    }
    read_bounded(&mut file, maximum).map_err(|_| error)
}

#[cfg(unix)]
fn openat(
    parent: &File,
    name: &str,
    flags: libc::c_int,
    error: RuntimeError,
) -> Result<File, RuntimeError> {
    let name = CString::new(name).map_err(|_| error)?;
    // ASVS 1.5/2.2: resolve one fixed child only beneath the retained descriptor.
    let descriptor = unsafe { libc::openat(parent.as_raw_fd(), name.as_ptr(), flags) };
    if descriptor < 0 {
        return Err(error);
    }
    // SAFETY: `openat` returned a new owned descriptor and this File owns it once.
    Ok(unsafe { File::from_raw_fd(descriptor) })
}

#[cfg(unix)]
fn read_bounded(file: &mut File, maximum: usize) -> io::Result<Vec<u8>> {
    let mut bytes = Vec::with_capacity(maximum.min(1024));
    file.take((maximum + 1) as u64).read_to_end(&mut bytes)?;
    if bytes.len() > maximum {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "bounded read exceeded",
        ));
    }
    Ok(bytes)
}

fn parse_database_url(bytes: Vec<u8>, role: &str) -> Result<(PostgresUrl, String), RuntimeError> {
    if !bytes.is_ascii() || !bytes.ends_with(b"\n") || bytes[..bytes.len() - 1].contains(&b'\n') {
        return Err(RuntimeError::SecretContent);
    }
    let text =
        std::str::from_utf8(&bytes[..bytes.len() - 1]).map_err(|_| RuntimeError::SecretContent)?;
    if text.is_empty() || text.contains('%') {
        return Err(RuntimeError::SecretContent);
    }
    let parsed = Url::parse(text).map_err(|_| RuntimeError::DatabaseUrl)?;
    let loopback = matches!(
        parsed.host_str(),
        Some("127.0.0.1") | Some("::1") | Some("[::1]")
    );
    let password = parsed.password().filter(|password| !password.is_empty());
    if parsed.scheme() != "postgres"
        || !loopback
        || parsed.port().filter(|port| *port != 0).is_none()
        || parsed.username() != role
        || password.is_none()
        || !password.is_some_and(valid_password)
        || parsed.path() != format!("/{DATABASE_NAME}")
        || parsed.query().is_some()
        || parsed.fragment().is_some()
    {
        return Err(RuntimeError::DatabaseUrl);
    }
    Ok((PostgresUrl(text.to_owned()), password.unwrap().to_owned()))
}

fn valid_password(password: &str) -> bool {
    password.len() == PASSWORD_LENGTH
        && password
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
}

#[cfg(unix)]
fn current_uid() -> Result<u32, RuntimeError> {
    // ASVS 1.5/2.2: the checked descriptor, owner, and exact mode form one boundary.
    Ok(unsafe { libc::geteuid() })
}

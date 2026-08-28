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
const MAX_PASSWORD_BYTES: usize = 33;
const MAX_COMPOSE_ENVIRONMENT_BYTES: usize = 16_384;
const CAPABILITY_BYTES: usize = 32;
const OWNER: &str = "live-demo-browser";
const PROJECT: &str = "ple-live-demo-browser";
const PROFILE: &str = "database_baseline";
const ADMIN_ROLE: &str = "ple_e2e_migrator";
const GRADER_ROLE: &str = "ple_grading_reader";
const FAST_PATH_ROLE: &str = "ple_accepted_submission_fast_path_login";
const RECOVERY_ROLE: &str = "ple_accepted_submission_recovery_login";
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
    admin_url: PostgresUrl,
    grader_url: PostgresUrl,
    fast_path_url: PostgresUrl,
    recovery_url: PostgresUrl,
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

    pub fn admin_url(&self) -> &PostgresUrl {
        &self.admin_url
    }

    pub fn grader_url(&self) -> &PostgresUrl {
        &self.grader_url
    }

    /// Returns the validated URL for the exact accepted-submission fast path.
    pub fn fast_path_url(&self) -> &PostgresUrl {
        &self.fast_path_url
    }

    /// Returns the validated URL for generic accepted-submission recovery.
    pub fn recovery_url(&self) -> &PostgresUrl {
        &self.recovery_url
    }
}

#[cfg(all(test, unix))]
fn load_from_workspace_unix(workspace: &Path) -> Result<AcceptanceRuntime, RuntimeError> {
    let workspace = open_private_directory(workspace, RuntimeError::Workspace)?;
    load_from_workspace_descriptor(&workspace)
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
    invoke_test_hook_after_secrets_open();
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
    let (admin_url, admin_url_password) = parse_database_url(
        read_private_file_at(
            &secrets,
            "postgres-admin.url",
            MAX_URL_BYTES,
            RuntimeError::SecretFile,
        )?,
        ADMIN_ROLE,
    )?;
    let admin_password = parse_password(read_private_file_at(
        &secrets,
        "postgres-admin.password",
        MAX_PASSWORD_BYTES,
        RuntimeError::SecretFile,
    )?)?;
    if admin_password != admin_url_password {
        return Err(RuntimeError::SecretContent);
    }
    let (grader_url, _) = parse_database_url(
        read_private_file_at(
            &secrets,
            "postgres-grader.url",
            MAX_URL_BYTES,
            RuntimeError::SecretFile,
        )?,
        GRADER_ROLE,
    )?;
    let (fast_path_url, _) = parse_database_url(
        read_private_file_at(
            &secrets,
            "postgres-fast-path.url",
            MAX_URL_BYTES,
            RuntimeError::SecretFile,
        )?,
        FAST_PATH_ROLE,
    )?;
    let (recovery_url, _) = parse_database_url(
        read_private_file_at(
            &secrets,
            "postgres-recovery.url",
            MAX_URL_BYTES,
            RuntimeError::SecretFile,
        )?,
        RECOVERY_ROLE,
    )?;
    Ok(AcceptanceRuntime {
        admin_url,
        grader_url,
        fast_path_url,
        recovery_url,
    })
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
    postgres_admin_password: String,
    postgres_grader_url: String,
    postgres_fast_path_url: String,
    postgres_recovery_url: String,
}

fn validate_manifest(manifest: &RuntimeManifest) -> Result<(), RuntimeError> {
    if manifest.schema_version != 1 || manifest.kind != "ple.disposable_postgres_acceptance" {
        return Err(RuntimeError::Schema);
    }
    if manifest.identity.owner != OWNER
        || manifest.identity.project != PROJECT
        || manifest.identity.profile != PROFILE
    {
        return Err(RuntimeError::Identity);
    }
    if manifest.secrets.compose_environment != "secrets/compose.env"
        || manifest.secrets.cleanup_capability != "secrets/cleanup.capability"
        || manifest.secrets.postgres_admin_url != "secrets/postgres-admin.url"
        || manifest.secrets.postgres_admin_password != "secrets/postgres-admin.password"
        || manifest.secrets.postgres_grader_url != "secrets/postgres-grader.url"
        || manifest.secrets.postgres_fast_path_url != "secrets/postgres-fast-path.url"
        || manifest.secrets.postgres_recovery_url != "secrets/postgres-recovery.url"
    {
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

#[cfg(all(test, unix))]
std::thread_local! {
    static TEST_HOOK: std::cell::RefCell<Option<Box<dyn FnOnce()>>> =
        const { std::cell::RefCell::new(None) };
}

#[cfg(all(test, unix))]
fn invoke_test_hook_after_secrets_open() {
    if let Some(hook) = TEST_HOOK.with(|hook| hook.borrow_mut().take()) {
        hook();
    }
}

#[cfg(all(not(test), unix))]
fn invoke_test_hook_after_secrets_open() {}

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

fn parse_password(bytes: Vec<u8>) -> Result<String, RuntimeError> {
    if !bytes.is_ascii() || !bytes.ends_with(b"\n") || bytes[..bytes.len() - 1].contains(&b'\n') {
        return Err(RuntimeError::SecretContent);
    }
    let password =
        std::str::from_utf8(&bytes[..bytes.len() - 1]).map_err(|_| RuntimeError::SecretContent)?;
    if !valid_password(password) {
        return Err(RuntimeError::SecretContent);
    }
    Ok(password.to_owned())
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

#[cfg(all(test, unix))]
mod tests {
    use super::*;
    use std::os::unix::fs::{PermissionsExt, symlink};
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::time::{SystemTime, UNIX_EPOCH};

    static WORKSPACE_COUNTER: AtomicU64 = AtomicU64::new(0);

    fn temp_workspace() -> PathBuf {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let counter = WORKSPACE_COUNTER.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!("acceptance-runtime-{timestamp}-{counter}"));
        fs::create_dir(&path).unwrap();
        fs::set_permissions(&path, fs::Permissions::from_mode(0o700)).unwrap();
        fs::create_dir(path.join("secrets")).unwrap();
        fs::set_permissions(path.join("secrets"), fs::Permissions::from_mode(0o700)).unwrap();
        fs::write(
            path.join("runtime.yaml"),
            b"schema_version: 1\nkind: ple.disposable_postgres_acceptance\nidentity:\n  owner: live-demo-browser\n  project: ple-live-demo-browser\n  profile: database_baseline\nsecrets:\n  compose_environment: secrets/compose.env\n  cleanup_capability: secrets/cleanup.capability\n  postgres_admin_url: secrets/postgres-admin.url\n  postgres_admin_password: secrets/postgres-admin.password\n  postgres_grader_url: secrets/postgres-grader.url\n  postgres_fast_path_url: secrets/postgres-fast-path.url\n  postgres_recovery_url: secrets/postgres-recovery.url\n",
        )
        .unwrap();
        fs::set_permissions(path.join("runtime.yaml"), fs::Permissions::from_mode(0o600)).unwrap();
        for (name, contents) in [
            ("compose.env", b"POSTGRES_PASSWORD_FILE=/run/ple-runtime/postgres-password\n".as_slice()),
            ("cleanup.capability", b"12345678901234567890123456789012".as_slice()),
            ("postgres-admin.url", b"postgres://ple_e2e_migrator:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa@127.0.0.1:15432/ple_e2e_baseline\n".as_slice()),
            ("postgres-admin.password", b"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa\n".as_slice()),
            ("postgres-grader.url", b"postgres://ple_grading_reader:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb@127.0.0.1:15432/ple_e2e_baseline\n".as_slice()),
            ("postgres-fast-path.url", b"postgres://ple_accepted_submission_fast_path_login:cccccccccccccccccccccccccccccccc@127.0.0.1:15432/ple_e2e_baseline\n".as_slice()),
            ("postgres-recovery.url", b"postgres://ple_accepted_submission_recovery_login:dddddddddddddddddddddddddddddddd@127.0.0.1:15432/ple_e2e_baseline\n".as_slice()),
        ] {
            fs::write(path.join("secrets").join(name), contents).unwrap();
            fs::set_permissions(
                path.join("secrets").join(name),
                fs::Permissions::from_mode(0o600),
            )
            .unwrap();
        }
        path
    }

    fn install_after_secrets_open_hook(hook: impl FnOnce() + 'static) {
        TEST_HOOK.with(|slot| *slot.borrow_mut() = Some(Box::new(hook)));
    }

    #[test]
    fn loads_inline_runtime_and_redacts_debug() {
        let workspace = temp_workspace();
        let runtime = load_from_workspace_unix(&workspace).unwrap();
        assert_eq!(
            runtime.admin_url().expose(),
            "postgres://ple_e2e_migrator:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa@127.0.0.1:15432/ple_e2e_baseline"
        );
        assert_eq!(
            runtime.fast_path_url().expose(),
            "postgres://ple_accepted_submission_fast_path_login:cccccccccccccccccccccccccccccccc@127.0.0.1:15432/ple_e2e_baseline"
        );
        assert_eq!(
            runtime.recovery_url().expose(),
            "postgres://ple_accepted_submission_recovery_login:dddddddddddddddddddddddddddddddd@127.0.0.1:15432/ple_e2e_baseline"
        );
        assert!(format!("{:?}", runtime.admin_url()).contains("REDACTED"));
        assert!(!format!("{:?}", runtime.admin_url()).contains("aaaaaaaa"));
        fs::remove_dir_all(workspace).unwrap();
    }

    #[test]
    fn manifest_locator_is_absolute_and_names_exactly_runtime_yaml() {
        let workspace = temp_workspace();
        assert_eq!(
            validate_locator(Path::new("runtime.yaml")).unwrap_err(),
            RuntimeError::Locator
        );
        assert_eq!(
            validate_locator(&workspace.join("other.yaml")).unwrap_err(),
            RuntimeError::Locator
        );
        assert_eq!(
            validate_locator(&workspace.join(MANIFEST_NAME)).unwrap(),
            workspace.as_path()
        );
        fs::remove_dir_all(workspace).unwrap();
    }

    #[test]
    fn retained_secrets_descriptor_ignores_a_swapped_secrets_pathname() {
        let workspace = temp_workspace();
        let original = workspace.join("original-secrets");
        let replacement = workspace.join("secrets");
        let replacement_for_hook = replacement.clone();
        install_after_secrets_open_hook(move || {
            fs::rename(&replacement_for_hook, &original).unwrap();
            fs::create_dir(&replacement_for_hook).unwrap();
            fs::set_permissions(&replacement_for_hook, fs::Permissions::from_mode(0o700)).unwrap();
            fs::write(
                replacement_for_hook.join("postgres-admin.url"),
                b"postgres://ple_e2e_migrator:replacement@127.0.0.1:15432/replacement\n",
            )
            .unwrap();
            fs::set_permissions(
                replacement_for_hook.join("postgres-admin.url"),
                fs::Permissions::from_mode(0o600),
            )
            .unwrap();
        });
        let runtime = load_from_workspace_unix(&workspace).unwrap();
        assert_eq!(
            runtime.admin_url().expose(),
            "postgres://ple_e2e_migrator:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa@127.0.0.1:15432/ple_e2e_baseline"
        );
        fs::remove_dir_all(workspace).unwrap();
    }

    #[test]
    fn rejects_secret_symlink_wrong_mode_oversize_and_path_escape() {
        let workspace = temp_workspace();
        let secret = workspace.join("secrets/postgres-admin.url");
        fs::remove_file(&secret).unwrap();
        symlink(workspace.join("secrets/postgres-grader.url"), &secret).unwrap();
        assert_eq!(
            load_from_workspace_unix(&workspace).unwrap_err(),
            RuntimeError::SecretFile
        );
        fs::remove_file(&secret).unwrap();
        fs::write(
            &secret,
            b"postgres://ple_e2e_migrator:synthetic@127.0.0.1:15432/db\n",
        )
        .unwrap();
        fs::set_permissions(&secret, fs::Permissions::from_mode(0o644)).unwrap();
        assert_eq!(
            load_from_workspace_unix(&workspace).unwrap_err(),
            RuntimeError::SecretFile
        );
        fs::set_permissions(&secret, fs::Permissions::from_mode(0o600)).unwrap();
        fs::write(&secret, vec![b'x'; MAX_URL_BYTES + 1]).unwrap();
        assert_eq!(
            load_from_workspace_unix(&workspace).unwrap_err(),
            RuntimeError::SecretFile
        );
        fs::write(
            workspace.join("runtime.yaml"),
            b"schema_version: 1\nkind: ple.disposable_postgres_acceptance\nidentity:\n  owner: live-demo-browser\n  project: ple-live-demo-browser\n  profile: database_baseline\nsecrets:\n  compose_environment: ../compose.env\n  cleanup_capability: secrets/cleanup.capability\n  postgres_admin_url: secrets/postgres-admin.url\n  postgres_admin_password: secrets/postgres-admin.password\n  postgres_grader_url: secrets/postgres-grader.url\n  postgres_fast_path_url: secrets/postgres-fast-path.url\n  postgres_recovery_url: secrets/postgres-recovery.url\n",
        )
        .unwrap();
        assert_eq!(
            load_from_workspace_unix(&workspace).unwrap_err(),
            RuntimeError::SecretPath
        );
        fs::remove_dir_all(workspace).unwrap();
    }

    #[test]
    fn rejects_schema_extensions_and_role_mismatch_without_url_disclosure() {
        let workspace = temp_workspace();
        fs::write(
            workspace.join("runtime.yaml"),
            b"schema_version: 1\nkind: ple.disposable_postgres_acceptance\nidentity: &identity\n  owner: live-demo-browser\n  project: ple-live-demo-browser\n  profile: database_baseline\nsecrets:\n  compose_environment: secrets/compose.env\n  cleanup_capability: secrets/cleanup.capability\n  postgres_admin_url: secrets/postgres-admin.url\n  postgres_admin_password: secrets/postgres-admin.password\n  postgres_grader_url: secrets/postgres-grader.url\n",
        )
        .unwrap();
        let error = load_from_workspace_unix(&workspace).unwrap_err();
        assert_eq!(error, RuntimeError::Schema);
        assert!(!error.to_string().contains("aaaaaaaa"));
        fs::write(
            workspace.join("runtime.yaml"),
            b"schema_version: 1\nkind: ple.disposable_postgres_acceptance\nidentity:\n  owner: live-demo-browser\n  project: ple-live-demo-browser\n  profile: database_baseline\nsecrets:\n  compose_environment: secrets/compose.env\n  cleanup_capability: secrets/cleanup.capability\n  postgres_admin_url: secrets/postgres-admin.url\n  postgres_admin_password: secrets/postgres-admin.password\n  postgres_grader_url: secrets/postgres-grader.url\n  postgres_fast_path_url: secrets/postgres-fast-path.url\n  postgres_recovery_url: secrets/postgres-recovery.url\n",
        )
        .unwrap();
        fs::set_permissions(
            workspace.join("runtime.yaml"),
            fs::Permissions::from_mode(0o600),
        )
        .unwrap();
        fs::write(
            workspace.join("secrets/postgres-admin.url"),
            b"postgres://ple_grading_reader:synthetic-grader@127.0.0.1:15432/ple_e2e_baseline\n",
        )
        .unwrap();
        assert_eq!(
            load_from_workspace_unix(&workspace).unwrap_err(),
            RuntimeError::DatabaseUrl
        );
        fs::remove_dir_all(workspace).unwrap();
    }

    #[test]
    fn rejects_empty_password_fragment_query_and_extra_database_path() {
        let workspace = temp_workspace();
        let admin = workspace.join("secrets/postgres-admin.url");
        for value in [
            "postgres://ple_e2e_migrator:@127.0.0.1:15432/ple_e2e_baseline\n",
            "postgres://ple_e2e_migrator:synthetic@127.0.0.1:15432/ple_e2e_baseline#fragment\n",
            "postgres://ple_e2e_migrator:synthetic@127.0.0.1:15432/ple_e2e_baseline?sslmode=disable\n",
            "postgres://ple_e2e_migrator:synthetic@127.0.0.1:15432/one/two\n",
            "postgres://ple_e2e_migrator:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa%61@127.0.0.1:15432/ple_e2e_baseline\n",
        ] {
            fs::write(&admin, value).unwrap();
            assert_eq!(
                load_from_workspace_unix(&workspace).unwrap_err(),
                if value.contains('%') {
                    RuntimeError::SecretContent
                } else {
                    RuntimeError::DatabaseUrl
                }
            );
        }
        fs::remove_dir_all(workspace).unwrap();
    }

    #[test]
    fn rejects_shared_invalid_url_and_admin_password_cases() {
        let workspace = temp_workspace();
        let admin = workspace.join("secrets/postgres-admin.url");
        for value in [
            b"postgres://ple_e2e_migrator:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa@127.0.0.1:0/ple_e2e_baseline\n".as_slice(),
            b"postgres://ple_e2e_migrator:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa@127.0.0.1:15432/not_ple_e2e_baseline\n".as_slice(),
        ] {
            fs::write(&admin, value).unwrap();
            assert_eq!(
                load_from_workspace_unix(&workspace).unwrap_err(),
                RuntimeError::DatabaseUrl
            );
        }
        fs::write(
            &admin,
            b"postgres://ple_e2e_migrator:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa@127.0.0.1:15432/ple_e2e_baseline\n",
        )
        .unwrap();
        fs::write(
            workspace.join("secrets/postgres-admin.password"),
            b"bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb\n",
        )
        .unwrap();
        assert_eq!(
            load_from_workspace_unix(&workspace).unwrap_err(),
            RuntimeError::SecretContent
        );
        fs::write(&admin, vec![b'x'; MAX_URL_BYTES + 1]).unwrap();
        assert_eq!(
            load_from_workspace_unix(&workspace).unwrap_err(),
            RuntimeError::SecretFile
        );
        fs::remove_dir_all(workspace).unwrap();
    }

    #[test]
    fn rejects_unknown_and_duplicate_schema_keys() {
        let workspace = temp_workspace();
        for manifest in [
            b"schema_version: 1\nkind: ple.disposable_postgres_acceptance\nidentity:\n  owner: live-demo-browser\n  project: ple-live-demo-browser\n  profile: database_baseline\nsecrets:\n  compose_environment: secrets/compose.env\n  cleanup_capability: secrets/cleanup.capability\n  postgres_admin_url: secrets/postgres-admin.url\n  postgres_admin_password: secrets/postgres-admin.password\n  postgres_grader_url: secrets/postgres-grader.url\nextra: rejected\n".as_slice(),
            b"schema_version: 1\nschema_version: 1\nkind: ple.disposable_postgres_acceptance\nidentity:\n  owner: live-demo-browser\n  project: ple-live-demo-browser\n  profile: database_baseline\nsecrets:\n  compose_environment: secrets/compose.env\n  cleanup_capability: secrets/cleanup.capability\n  postgres_admin_url: secrets/postgres-admin.url\n  postgres_admin_password: secrets/postgres-admin.password\n  postgres_grader_url: secrets/postgres-grader.url\n".as_slice(),
            b"schema_version: true\nkind: ple.disposable_postgres_acceptance\nidentity:\n  owner: live-demo-browser\n  project: ple-live-demo-browser\n  profile: database_baseline\nsecrets:\n  compose_environment: secrets/compose.env\n  cleanup_capability: secrets/cleanup.capability\n  postgres_admin_url: secrets/postgres-admin.url\n  postgres_admin_password: secrets/postgres-admin.password\n  postgres_grader_url: secrets/postgres-grader.url\n".as_slice(),
        ] {
            fs::write(workspace.join("runtime.yaml"), manifest).unwrap();
            assert_eq!(
                load_from_workspace_unix(&workspace).unwrap_err(),
                RuntimeError::Schema
            );
        }
        fs::remove_dir_all(workspace).unwrap();
    }
}

use std::fs;
use std::os::unix::fs::{PermissionsExt, symlink};
use std::path::PathBuf;

use super::*;

const URL: &[u8] =
    b"postgres://ple_migrator:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa@127.0.0.1:15432/ple_e2e_baseline\n";

fn postgres_migration_acceptance_workspace() -> PathBuf {
    let root = crate::tests::temp_workspace();
    fs::write(
        root.join(MANIFEST_NAME),
        b"schema_version: 1\nkind: ple.postgres_migration_acceptance\nidentity:\n  owner: live-demo-browser\n  project: ple-live-demo-browser\n  profile: postgres_migration_acceptance\nsecrets:\n  postgres_migrator_url: secrets/postgres-migrator.url\n",
    )
    .unwrap();
    for entry in fs::read_dir(root.join("secrets")).unwrap() {
        fs::remove_file(entry.unwrap().path()).unwrap();
    }
    let url = root.join("secrets/postgres-migrator.url");
    fs::write(&url, URL).unwrap();
    fs::set_permissions(url, fs::Permissions::from_mode(0o600)).unwrap();
    root
}

#[test]
fn loads_only_the_migrator_url_and_redacts_debug() {
    let workspace = postgres_migration_acceptance_workspace();
    let runtime = load_from_locator_unix(&workspace.join(MANIFEST_NAME)).unwrap();
    assert_eq!(
        runtime.postgres_migrator_url().expose(),
        "postgres://ple_migrator:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa@127.0.0.1:15432/ple_e2e_baseline"
    );
    assert_eq!(
        format!("{runtime:?}"),
        "PostgresMigrationAcceptanceRuntime(REDACTED)"
    );
    fs::remove_dir_all(workspace).unwrap();
}

#[test]
fn rejects_other_profile_kinds_and_baseline_loader_use() {
    let workspace = postgres_migration_acceptance_workspace();
    let manifest_path = workspace.join(MANIFEST_NAME);
    let manifest = fs::read_to_string(&manifest_path).unwrap();
    for replacement in [
        manifest.replace(
            "kind: ple.postgres_migration_acceptance",
            "kind: ple.disposable_postgres_acceptance",
        ),
        manifest.replace(
            "profile: postgres_migration_acceptance",
            "profile: database_baseline",
        ),
    ] {
        fs::write(&manifest_path, replacement).unwrap();
        assert_eq!(
            load_from_locator_unix(&manifest_path).unwrap_err(),
            RuntimeError::Identity
        );
    }
    fs::write(&manifest_path, manifest).unwrap();
    assert_eq!(
        crate::load_from_locator_unix(&manifest_path).unwrap_err(),
        RuntimeError::Schema
    );
    fs::remove_dir_all(workspace).unwrap();
}

#[test]
fn closes_manifest_identity_and_secret_shape() {
    let workspace = postgres_migration_acceptance_workspace();
    let manifest_path = workspace.join(MANIFEST_NAME);
    let manifest = fs::read_to_string(&manifest_path).unwrap();
    for (replacement, expected_error) in [
        (
            manifest.replace("owner: live-demo-browser", "owner: other"),
            RuntimeError::Identity,
        ),
        (
            manifest.replace("project: ple-live-demo-browser", "project: other"),
            RuntimeError::Identity,
        ),
        (
            manifest.replace(
                "secrets/postgres-migrator.url",
                "secrets/postgres-admin.url",
            ),
            RuntimeError::SecretPath,
        ),
        (
            format!("{manifest}extra: rejected\n"),
            RuntimeError::Schema,
        ),
        (
            manifest.replace("identity:\n", "identity: &identity\n"),
            RuntimeError::Schema,
        ),
        (
            manifest.replacen("schema_version: 1\n", "schema_version: 1\nschema_version: 1\n", 1),
            RuntimeError::Schema,
        ),
        (
            manifest.replace(
                "  postgres_migrator_url: secrets/postgres-migrator.url\n",
                "  postgres_migrator_url: secrets/postgres-migrator.url\n  postgres_admin_url: secrets/postgres-admin.url\n",
            ),
            RuntimeError::Schema,
        ),
    ] {
        fs::write(&manifest_path, replacement).unwrap();
        assert_eq!(
            load_from_locator_unix(&manifest_path).unwrap_err(),
            expected_error
        );
    }
    fs::remove_dir_all(workspace).unwrap();
}

#[test]
fn requires_an_absolute_runtime_yaml_locator() {
    let workspace = postgres_migration_acceptance_workspace();
    assert_eq!(
        load_from_locator_unix(Path::new("runtime.yaml")).unwrap_err(),
        RuntimeError::Locator
    );
    assert_eq!(
        load_from_locator_unix(&workspace.join("other.yaml")).unwrap_err(),
        RuntimeError::Locator
    );
    fs::remove_dir_all(workspace).unwrap();
}

#[test]
fn rejects_wrong_role_host_port_database_and_url_extensions() {
    let workspace = postgres_migration_acceptance_workspace();
    let url_path = workspace.join("secrets/postgres-migrator.url");
    for url in [
        "postgres://ple_e2e_migrator:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa@127.0.0.1:15432/ple_e2e_baseline\n",
        "postgres://ple_migrator:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa@example.test:15432/ple_e2e_baseline\n",
        "postgres://ple_migrator:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa@127.0.0.1/ple_e2e_baseline\n",
        "postgres://ple_migrator:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa@127.0.0.1:0/ple_e2e_baseline\n",
        "postgres://ple_migrator:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa@127.0.0.1:15432/other\n",
        "postgres://ple_migrator:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa@127.0.0.1:15432/ple_e2e_baseline?sslmode=disable\n",
        "postgres://ple_migrator:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa@127.0.0.1:15432/ple_e2e_baseline#fragment\n",
        "postgres://ple_migrator:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa%61@127.0.0.1:15432/ple_e2e_baseline\n",
    ] {
        fs::write(&url_path, url).unwrap();
        assert!(load_from_locator_unix(&workspace.join(MANIFEST_NAME)).is_err());
    }
    fs::remove_dir_all(workspace).unwrap();
}

#[test]
fn rejects_non_private_workspace_manifest_secret_and_symlink() {
    let workspace = postgres_migration_acceptance_workspace();
    let manifest_path = workspace.join(MANIFEST_NAME);
    fs::set_permissions(&workspace, fs::Permissions::from_mode(0o750)).unwrap();
    assert_eq!(
        load_from_locator_unix(&manifest_path).unwrap_err(),
        RuntimeError::Workspace
    );
    fs::set_permissions(&workspace, fs::Permissions::from_mode(0o700)).unwrap();

    let secrets = workspace.join("secrets");
    fs::set_permissions(&secrets, fs::Permissions::from_mode(0o750)).unwrap();
    assert_eq!(
        load_from_locator_unix(&manifest_path).unwrap_err(),
        RuntimeError::SecretPath
    );
    fs::set_permissions(&secrets, fs::Permissions::from_mode(0o700)).unwrap();

    fs::set_permissions(&manifest_path, fs::Permissions::from_mode(0o644)).unwrap();
    assert_eq!(
        load_from_locator_unix(&manifest_path).unwrap_err(),
        RuntimeError::Manifest
    );
    fs::set_permissions(&manifest_path, fs::Permissions::from_mode(0o600)).unwrap();

    let url_path = workspace.join("secrets/postgres-migrator.url");
    fs::set_permissions(&url_path, fs::Permissions::from_mode(0o640)).unwrap();
    assert_eq!(
        load_from_locator_unix(&manifest_path).unwrap_err(),
        RuntimeError::SecretFile
    );
    fs::set_permissions(&url_path, fs::Permissions::from_mode(0o600)).unwrap();
    fs::write(&url_path, vec![b'x'; MAX_URL_BYTES + 1]).unwrap();
    assert_eq!(
        load_from_locator_unix(&manifest_path).unwrap_err(),
        RuntimeError::SecretFile
    );
    fs::write(&url_path, URL).unwrap();
    fs::remove_file(&url_path).unwrap();
    let outside = workspace.join("outside.url");
    fs::write(&outside, URL).unwrap();
    fs::set_permissions(&outside, fs::Permissions::from_mode(0o600)).unwrap();
    symlink(&outside, &url_path).unwrap();
    assert_eq!(
        load_from_locator_unix(&manifest_path).unwrap_err(),
        RuntimeError::SecretFile
    );
    fs::remove_dir_all(workspace).unwrap();
}

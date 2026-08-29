use super::*;
use std::fs;
use std::os::unix::fs::PermissionsExt;

fn course_appearance_workspace() -> std::path::PathBuf {
    let workspace = super::tests::temp_workspace();
    fs::write(
        workspace.join(MANIFEST_NAME),
        b"schema_version: 1\nkind: ple.disposable_postgres_minio_acceptance\nidentity:\n  owner: live-demo-browser\n  project: ple-live-demo-browser\n  profile: course_appearance_cross_store\nsecrets:\n  compose_environment: secrets/compose.env\n  cleanup_capability: secrets/cleanup.capability\n  postgres_admin_url: secrets/postgres-admin.url\n  postgres_admin_password: secrets/postgres-admin.password\n  postgres_grader_url: secrets/postgres-grader.url\n  postgres_fast_path_url: secrets/postgres-fast-path.url\n  postgres_recovery_url: secrets/postgres-recovery.url\n  minio_endpoint: secrets/minio-endpoint.url\n  minio_region: secrets/minio-region\n  minio_access_key_id: secrets/minio-access-key-id\n  minio_secret_access_key: secrets/minio-secret-access-key\n",
    )
    .unwrap();
    for (name, contents) in [
        ("minio-endpoint.url", b"http://127.0.0.1:19000\n".as_slice()),
        ("minio-region", b"us-east-1\n".as_slice()),
        (
            "minio-access-key-id",
            b"dddddddddddddddddddddddddddddddd\n".as_slice(),
        ),
        (
            "minio-secret-access-key",
            b"eeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee\n".as_slice(),
        ),
    ] {
        let path = workspace.join("secrets").join(name);
        fs::write(&path, contents).unwrap();
        fs::set_permissions(path, fs::Permissions::from_mode(0o600)).unwrap();
    }
    workspace
}

#[test]
fn course_appearance_runtime_loads_typed_redacted_cross_store_inputs() {
    let workspace = course_appearance_workspace();
    let shared = load_from_locator_unix(&workspace.join(MANIFEST_NAME)).unwrap();
    assert_eq!(
        shared.admin_url().expose(),
        "postgres://ple_e2e_migrator:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa@127.0.0.1:15432/ple_e2e_baseline"
    );
    let runtime = load_course_appearance_from_locator_unix(&workspace.join(MANIFEST_NAME)).unwrap();
    assert_eq!(
        runtime.admin_url().expose(),
        "postgres://ple_e2e_migrator:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa@127.0.0.1:15432/ple_e2e_baseline"
    );
    assert_eq!(runtime.minio().endpoint_url(), "http://127.0.0.1:19000");
    assert_eq!(runtime.minio().region(), "us-east-1");
    assert_eq!(
        runtime.minio().access_key_id(),
        "dddddddddddddddddddddddddddddddd"
    );
    assert_eq!(
        runtime.minio().secret_access_key(),
        "eeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee"
    );
    assert_eq!(format!("{:?}", runtime.minio()), "MinioRuntime(REDACTED)");
    fs::remove_dir_all(workspace).unwrap();
}

#[test]
fn profile_kind_and_identity_pairs_are_closed() {
    let baseline = super::tests::temp_workspace();
    assert_eq!(
        load_course_appearance_from_locator_unix(&baseline.join(MANIFEST_NAME)).unwrap_err(),
        RuntimeError::Identity
    );
    fs::remove_dir_all(baseline).unwrap();

    let cross_store = course_appearance_workspace();
    assert_eq!(
        load_from_workspace_unix(&cross_store).unwrap_err(),
        RuntimeError::Identity
    );
    let manifest = fs::read_to_string(cross_store.join(MANIFEST_NAME)).unwrap();
    fs::write(
        cross_store.join(MANIFEST_NAME),
        manifest.replace(
            "kind: ple.disposable_postgres_minio_acceptance",
            "kind: ple.disposable_postgres_acceptance",
        ),
    )
    .unwrap();
    assert_eq!(
        load_from_locator_unix(&cross_store.join(MANIFEST_NAME)).unwrap_err(),
        RuntimeError::Identity
    );
    fs::remove_dir_all(cross_store).unwrap();
}

#[test]
fn cross_store_profile_closes_manifest_shape_and_object_store_values() {
    let baseline = super::tests::temp_workspace();
    let baseline_manifest = fs::read_to_string(baseline.join(MANIFEST_NAME)).unwrap();
    fs::write(
        baseline.join(MANIFEST_NAME),
        baseline_manifest.replace(
            "  postgres_recovery_url: secrets/postgres-recovery.url\n",
            "  postgres_recovery_url: secrets/postgres-recovery.url\n  minio_endpoint: secrets/minio-endpoint.url\n",
        ),
    )
    .unwrap();
    assert_eq!(
        load_from_workspace_unix(&baseline).unwrap_err(),
        RuntimeError::SecretPath
    );
    fs::remove_dir_all(baseline).unwrap();

    let workspace = course_appearance_workspace();
    let manifest = fs::read_to_string(workspace.join(MANIFEST_NAME)).unwrap();
    fs::write(
        workspace.join(MANIFEST_NAME),
        manifest.replace("  minio_region: secrets/minio-region\n", ""),
    )
    .unwrap();
    assert_eq!(
        load_course_appearance_from_locator_unix(&workspace.join(MANIFEST_NAME)).unwrap_err(),
        RuntimeError::SecretPath
    );
    fs::write(workspace.join(MANIFEST_NAME), manifest).unwrap();
    let manifest = fs::read_to_string(workspace.join(MANIFEST_NAME)).unwrap();
    fs::write(
        workspace.join(MANIFEST_NAME),
        format!("{manifest}  unrelated_secret: secrets/unrelated\n"),
    )
    .unwrap();
    assert_eq!(
        load_course_appearance_from_locator_unix(&workspace.join(MANIFEST_NAME)).unwrap_err(),
        RuntimeError::Schema
    );
    fs::write(workspace.join(MANIFEST_NAME), manifest).unwrap();
    for (manifest_replacement, secret_name, value) in [
        (
            Some(("secrets/minio-region", "secrets/not-minio-region")),
            "minio-region",
            b"us-east-1\n".as_slice(),
        ),
        (
            None,
            "minio-endpoint.url",
            b"https://127.0.0.1:19000\n".as_slice(),
        ),
        (None, "minio-region", b"not-us-east-1\n".as_slice()),
        (
            None,
            "minio-access-key-id",
            b"-aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa\n".as_slice(),
        ),
        (
            None,
            "minio-secret-access-key",
            b"-aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa\n".as_slice(),
        ),
        (None, "minio-secret-access-key", b"too-short\n".as_slice()),
    ] {
        if let Some((expected, replacement)) = manifest_replacement {
            let manifest = fs::read_to_string(workspace.join(MANIFEST_NAME)).unwrap();
            fs::write(
                workspace.join(MANIFEST_NAME),
                manifest.replace(expected, replacement),
            )
            .unwrap();
            assert_eq!(
                load_course_appearance_from_locator_unix(&workspace.join(MANIFEST_NAME))
                    .unwrap_err(),
                RuntimeError::SecretPath
            );
            fs::write(
                workspace.join(MANIFEST_NAME),
                fs::read_to_string(workspace.join(MANIFEST_NAME))
                    .unwrap()
                    .replace(replacement, expected),
            )
            .unwrap();
        } else {
            let path = workspace.join("secrets").join(secret_name);
            let original = fs::read(&path).unwrap();
            fs::write(&path, value).unwrap();
            assert_eq!(
                load_course_appearance_from_locator_unix(&workspace.join(MANIFEST_NAME))
                    .unwrap_err(),
                RuntimeError::SecretContent
            );
            fs::write(path, original).unwrap();
        }
    }
    fs::remove_dir_all(workspace).unwrap();
}

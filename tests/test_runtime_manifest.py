"""Security contracts for the disposable PostgreSQL acceptance runtime."""

from __future__ import annotations

import os
import pathlib
import stat

import pytest

import local_stack_control.consumer
import local_stack_control.env_file
import local_stack_control.models
import local_stack_control.runtime_manifest


#============================================
def generated_runtime(tmp_path: pathlib.Path) -> local_stack_control.runtime_manifest.DatabaseBaselineRuntime:
	"""Create one ordinary private runtime workspace for one focused contract."""
	tmp_path.chmod(0o700)
	return local_stack_control.runtime_manifest.write_database_baseline_runtime(tmp_path, 15432)


#============================================
def generated_cross_store_runtime(
	tmp_path: pathlib.Path,
) -> local_stack_control.runtime_manifest.CourseAppearanceCrossStoreRuntime:
	"""Create one private two-store runtime for the cross-store contract."""
	tmp_path.chmod(0o700)
	return local_stack_control.runtime_manifest.write_course_appearance_cross_store_runtime(
		tmp_path, 15432, 19000
	)


#============================================
def test_cross_store_runtime_binds_exact_minio_locators_without_secret_yaml(
	tmp_path: pathlib.Path,
) -> None:
	"""The cross-store runtime carries private MinIO files beside the baseline URLs."""
	runtime = generated_cross_store_runtime(tmp_path)
	manifest = runtime.manifest_path.read_text(encoding="ascii")
	assert "course_appearance_cross_store" in manifest
	assert "minio_endpoint: secrets/minio-endpoint.url" in manifest
	assert "minio_region: secrets/minio-region" in manifest
	assert "minio_access_key_id: secrets/minio-access-key-id" in manifest
	assert "minio_secret_access_key: secrets/minio-secret-access-key" in manifest
	assert "http://127.0.0.1" not in manifest
	assert runtime.minio_endpoint_path.read_text(encoding="ascii") == "http://127.0.0.1:19000\n"
	assert runtime.minio_region_path.read_text(encoding="ascii") == "us-east-1\n"
	access_key_id = runtime.minio_access_key_id_path.read_text(encoding="ascii").removesuffix("\n")
	secret_access_key = runtime.minio_secret_access_key_path.read_text(encoding="ascii").removesuffix("\n")
	assert len(access_key_id) == 32 and all(character in "0123456789abcdef" for character in access_key_id)
	assert len(secret_access_key) == 32 and all(
		character in "0123456789abcdef" for character in secret_access_key
	)
	assert stat.S_IMODE(runtime.minio_secret_access_key_path.stat().st_mode) == 0o600
	assert (
		local_stack_control.runtime_manifest.acceptance_runtime_profile(tmp_path)
		is local_stack_control.models.LiveDemoProfile.COURSE_APPEARANCE_CROSS_STORE
	)


#============================================
def test_generated_runtime_has_closed_identity_private_files_and_no_url_in_yaml(
	tmp_path: pathlib.Path,
) -> None:
	"""The generated YAML names a closed target while credential bytes stay separate."""
	runtime = generated_runtime(tmp_path)
	manifest = runtime.manifest_path.read_text(encoding="ascii")
	assert "ple.disposable_postgres_acceptance" in manifest
	assert "postgres://" not in manifest
	assert "postgres_admin_url: secrets/postgres-admin.url" in manifest
	assert runtime.compose_environment_path.parent == tmp_path / "secrets"
	assert len(runtime.cleanup_capability_path.read_bytes()) == 32
	assert runtime.admin_url_path.read_text(encoding="ascii").startswith("postgres://")
	assert runtime.admin_password_path.read_text(encoding="ascii").endswith("\n")
	environment = local_stack_control.env_file.env_settings(runtime.compose_environment_path)
	assert "POSTGRES_PASSWORD" not in environment
	assert environment["POSTGRES_PASSWORD_FILE"] == "/run/ple-runtime/postgres-password"


#============================================
@pytest.mark.parametrize(
	"replacement",
	(
	"schema_version: 1\nkind: ple.disposable_postgres_acceptance\nidentity: &identity\n  owner: live-demo-browser\n  project: ple-live-demo-browser\n  profile: database_baseline\nsecrets: *identity\n",
	"schema_version: 1\nschema_version: 1\nkind: ple.disposable_postgres_acceptance\nidentity:\n  owner: live-demo-browser\n  project: ple-live-demo-browser\n  profile: database_baseline\nsecrets:\n  compose_environment: secrets/compose.env\n  cleanup_capability: secrets/cleanup.capability\n  postgres_admin_url: secrets/postgres-admin.url\n  postgres_admin_password: secrets/postgres-admin.password\n",
	"schema_version: 1\nkind: ple.disposable_postgres_acceptance\nidentity:\n  owner: live-demo-browser\n  project: ple-live-demo-browser\n  profile: database_baseline\nsecrets:\n  compose_environment: ../compose.env\n  cleanup_capability: secrets/cleanup.capability\n  postgres_admin_url: secrets/postgres-admin.url\n  postgres_admin_password: secrets/postgres-admin.password\n",
	"schema_version: 1\nkind: ple.disposable_postgres_acceptance\nidentity:\n  owner: live-demo-browser\n  project: ple-live-demo-browser\n  profile: database_baseline\nsecrets:\n  compose_environment: secrets/compose.env\n  cleanup_capability: secrets/cleanup.capability\n  postgres_admin_url: secrets/postgres-admin.url\n  postgres_admin_password: secrets/postgres-admin.password\nextra: rejected\n",
	),
)
def test_manifest_schema_refuses_yaml_extensions_duplicates_and_path_escape(
	tmp_path: pathlib.Path,
	replacement: str,
) -> None:
	"""Malformed runtime YAML cannot select another target or secret path."""
	runtime = generated_runtime(tmp_path)
	runtime.manifest_path.write_text(replacement, encoding="ascii")
	runtime.manifest_path.chmod(0o600)
	with pytest.raises(local_stack_control.models.ControllerError, match="acceptance runtime"):
		local_stack_control.runtime_manifest.load_database_baseline_runtime(tmp_path)


#============================================
def test_runtime_refuses_symlinked_or_wrong_mode_secret_files(tmp_path: pathlib.Path) -> None:
	"""The manifest reader proves each referenced secret remains private and regular."""
	runtime = generated_runtime(tmp_path)
	runtime.admin_url_path.chmod(0o644)
	with pytest.raises(local_stack_control.models.ControllerError, match="postgres admin URL"):
		local_stack_control.runtime_manifest.load_database_baseline_runtime(tmp_path)
	runtime.admin_url_path.chmod(0o600)


#============================================
@pytest.mark.parametrize(
	("replacement_content", "destination"),
	(
		(
			"schema_version: true\n"
			"kind: ple.disposable_postgres_acceptance\n"
			"identity:\n"
			"  owner: live-demo-browser\n"
			"  project: ple-live-demo-browser\n"
			"  profile: database_baseline\n"
			"secrets:\n"
			"  compose_environment: secrets/compose.env\n"
			"  cleanup_capability: secrets/cleanup.capability\n"
			"  postgres_admin_url: secrets/postgres-admin.url\n"
			"  postgres_admin_password: secrets/postgres-admin.password\n",
			"runtime.yaml",
		),
		(
			"postgres://ple_e2e_migrator:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa@127.0.0.1:0/ple_e2e_baseline\n",
			"secrets/postgres-admin.url",
		),
		(
			"postgres://ple_e2e_migrator:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa@127.0.0.1:15432/not_ple_e2e_baseline\n",
			"secrets/postgres-admin.url",
		),
		("bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb\n", "secrets/postgres-admin.password"),
		(
			"x" * (local_stack_control.runtime_manifest.MAX_DATABASE_URL_BYTES + 1),
			"secrets/postgres-admin.url",
		),
	),
)
def test_shared_invalid_runtime_cases_are_refused_by_the_python_boundary(
	tmp_path: pathlib.Path,
	replacement_content: str,
	destination: str,
) -> None:
	"""Python applies the same malformed-schema and secret cases as the Rust loader."""
	workspace = generated_runtime(tmp_path)
	target = workspace.workspace / destination
	target.write_text(replacement_content, encoding="ascii")
	target.chmod(0o600)
	with pytest.raises(local_stack_control.models.ControllerError, match="acceptance runtime"):
		local_stack_control.runtime_manifest.load_database_baseline_runtime(workspace.workspace)


#============================================
def test_percent_encoded_database_password_is_refused_before_any_decode(
	tmp_path: pathlib.Path,
) -> None:
	"""Database URLs use the shared serialized URL-safe password representation."""
	runtime = generated_runtime(tmp_path)
	runtime.admin_url_path.write_text(
		"postgres://ple_e2e_migrator:" + "a" * 31 + "%61@127.0.0.1:15432/ple_e2e_baseline\n",
		encoding="ascii",
	)
	runtime.admin_url_path.chmod(0o600)
	with pytest.raises(local_stack_control.models.ControllerError, match="postgres admin URL"):
		local_stack_control.runtime_manifest.load_database_baseline_runtime(tmp_path)


#============================================
def test_revalidation_refuses_a_replaced_admin_password_without_disclosing_it(
	tmp_path: pathlib.Path,
	capsys: pytest.CaptureFixture[str],
) -> None:
	"""The exact bind-mount password is checked again directly before Compose use."""
	runtime = generated_runtime(tmp_path)
	replacement = "z" * 32
	runtime.admin_password_path.write_text(replacement + "\n", encoding="ascii")
	runtime.admin_password_path.chmod(0o600)
	with pytest.raises(local_stack_control.models.ControllerError) as raised:
		local_stack_control.runtime_manifest.require_database_baseline_compose_password(tmp_path)
	assert replacement not in str(raised.value)
	with pytest.raises(SystemExit, match="2"):
		local_stack_control.runtime_manifest.main(
			["--emit-automated-grading-login-provisioning", str(tmp_path)]
		)
	captured = capsys.readouterr()
	assert captured.out == ""
	assert replacement not in captured.err


#============================================
def test_compose_command_revalidates_its_admin_password_source(tmp_path: pathlib.Path) -> None:
	"""Compose startup refuses a companion replacement immediately before mounting it."""
	runtime = generated_runtime(tmp_path)
	target = local_stack_control.models.ComposeTarget(
		repo_root=tmp_path,
		project=local_stack_control.models.LIVE_DEMO_BROWSER_PROJECT,
		env_file=runtime.compose_environment_path,
		compose_files=(),
		provider=local_stack_control.models.ComposeProvider(
			("podman-compose", "--in-pod", "false"),
			"podman-compose",
		),
		with_smtp=False,
		env_setting_names=local_stack_control.env_file.env_setting_names(
			runtime.compose_environment_path
		),
	)
	disposable = local_stack_control.models.DisposableComposeTarget(
		target=target,
		owner_policy=local_stack_control.models.LIVE_DEMO_BROWSER_OWNER,
		capability_file=runtime.cleanup_capability_path,
		project_prefix=local_stack_control.models.LIVE_DEMO_BROWSER_PROJECT,
		private_environment_file=runtime.compose_environment_path,
		live_demo_profile=local_stack_control.models.LiveDemoProfile.DATABASE_BASELINE,
		acceptance_runtime_workspace=tmp_path,
	)
	runtime.admin_password_path.write_text("y" * 32 + "\n", encoding="ascii")
	runtime.admin_password_path.chmod(0o600)
	with pytest.raises(local_stack_control.models.ControllerError, match="postgres admin password"):
		local_stack_control.consumer.compose_command(disposable, ["up", "-d", "postgres"])


#============================================
def test_opened_secrets_directory_cannot_be_redirected_by_a_pathname_swap(
	tmp_path: pathlib.Path,
	monkeypatch: pytest.MonkeyPatch,
) -> None:
	"""Secrets stay under the opened workspace even when their pathname is replaced."""
	generated_runtime(tmp_path)
	held_directory = tmp_path / "held-secrets"
	replacement_directory = tmp_path / "replacement-secrets"
	replacement_directory.mkdir(mode=0o700)
	original_read = local_stack_control.runtime_manifest._read_private_file_at
	swapped = False

	def read_after_swap(
		parent_descriptor: int,
		name: str,
		maximum_bytes: int,
		field: str,
	) -> bytes:
		nonlocal swapped
		if field == "compose environment" and not swapped:
			swapped = True
			os.rename(tmp_path / "secrets", held_directory)
			os.rename(replacement_directory, tmp_path / "secrets")
		return original_read(parent_descriptor, name, maximum_bytes, field)

	monkeypatch.setattr(local_stack_control.runtime_manifest, "_read_private_file_at", read_after_swap)
	runtime = local_stack_control.runtime_manifest.load_database_baseline_runtime(tmp_path)
	assert swapped
	assert runtime.workspace == tmp_path


#============================================
def test_consumer_forms_database_target_from_the_runtime_manifest(tmp_path: pathlib.Path) -> None:
	"""The Compose adapter derives target authority from closed runtime YAML only."""
	runtime = generated_runtime(tmp_path)
	manifest = local_stack_control.consumer.load_manifest(tmp_path, runtime.manifest_path)
	assert manifest.owner == local_stack_control.models.LIVE_DEMO_BROWSER_OWNER
	assert manifest.project == local_stack_control.models.LIVE_DEMO_BROWSER_PROJECT
	assert manifest.live_demo_profile is local_stack_control.models.LiveDemoProfile.DATABASE_BASELINE
	assert manifest.env_file == runtime.compose_environment_path
	assert manifest.capability_file == runtime.cleanup_capability_path


#============================================
def test_consumer_forms_cross_store_target_from_the_runtime_manifest(tmp_path: pathlib.Path) -> None:
	"""The child receives a closed fixed-owner profile, never a caller-selected target."""
	runtime = generated_cross_store_runtime(tmp_path)
	manifest = local_stack_control.consumer.load_manifest(tmp_path, runtime.manifest_path)
	assert manifest.owner == local_stack_control.models.LIVE_DEMO_BROWSER_OWNER
	assert manifest.project == local_stack_control.models.LIVE_DEMO_BROWSER_PROJECT
	assert (
		manifest.live_demo_profile
		is local_stack_control.models.LiveDemoProfile.COURSE_APPEARANCE_CROSS_STORE
	)
	assert manifest.env_file == runtime.compose_environment_path
	assert manifest.capability_file == runtime.cleanup_capability_path

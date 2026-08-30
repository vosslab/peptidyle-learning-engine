"""Strict private runtime handoff for the disposable PostgreSQL acceptance lane."""

from __future__ import annotations

import dataclasses
import hashlib
import hmac
import os
import pathlib
import re
import secrets
import stat
import sys
import urllib.parse
from collections.abc import Mapping

import yaml

import local_stack_control.models
import local_stack_control.private_files


MANIFEST_NAME = "runtime.yaml"
SECRETS_DIRECTORY = "secrets"
COMPOSE_ENVIRONMENT = "secrets/compose.env"
CLEANUP_CAPABILITY = "secrets/cleanup.capability"
POSTGRES_ADMIN_URL = "secrets/postgres-admin.url"
POSTGRES_GRADER_URL = "secrets/postgres-grader.url"
POSTGRES_FAST_PATH_URL = "secrets/postgres-fast-path.url"
POSTGRES_RECOVERY_URL = "secrets/postgres-recovery.url"
POSTGRES_ADMIN_PASSWORD = "secrets/postgres-admin.password"
MINIO_ENDPOINT = "secrets/minio-endpoint.url"
MINIO_REGION = "secrets/minio-region"
MINIO_ACCESS_KEY_ID = "secrets/minio-access-key-id"
MINIO_SECRET_ACCESS_KEY = "secrets/minio-secret-access-key"
DATABASE_NAME = "ple_e2e_baseline"
SD1_RUNTIME_DIRECTORY = "sd1"
SD1_RUNTIME_KIND = "ple.sd1_staged_database_acceptance"
SD1_RUNTIME_PROFILE = "sd1_staged_database"
SD1_MIGRATOR_ROLE = "ple_migrator"
SD1_MIGRATOR_URL = "secrets/postgres-migrator.url"
POSTGRES_USER = "ple_e2e_migrator"
GRADER_USER = "ple_grading_reader"
FAST_PATH_USER = "ple_accepted_submission_fast_path_login"
RECOVERY_USER = "ple_accepted_submission_recovery_login"


@dataclasses.dataclass(frozen=True)
class _ServiceLoginProfile:
	"""Describe one baseline login and whether its object ACLs are migration-owned."""

	login: str
	roles: tuple[str, ...]
	revoke_object_privileges: bool


SERVICE_LOGIN_PROFILES = (
	_ServiceLoginProfile(
		GRADER_USER,
		("ple_grader",),
		False,
	),
	_ServiceLoginProfile(
		FAST_PATH_USER,
		("ple_accepted_submission_execution_fast_path",),
		True,
	),
	_ServiceLoginProfile(
		RECOVERY_USER,
		("ple_accepted_submission_execution",),
		True,
	),
)
GRADER_LOGIN_PROFILES = SERVICE_LOGIN_PROFILES[:1]
NEUTRAL_LOGIN_PROFILES = SERVICE_LOGIN_PROFILES[1:]


MAX_MANIFEST_BYTES = 4_096
MAX_COMPOSE_ENVIRONMENT_BYTES = 16_384
MAX_DATABASE_URL_BYTES = 4_096
ADMIN_PASSWORD_BYTES = 32
ADMIN_PASSWORD_FILE_BYTES = ADMIN_PASSWORD_BYTES + 1
PASSWORD_PATTERN = re.compile(r"^[A-Za-z0-9_-]{32}$")
MINIO_CREDENTIAL_PATTERN = re.compile(r"^[0-9a-f]{32}$")


@dataclasses.dataclass(frozen=True)
class DatabaseBaselineRuntime:
	"""Validated non-secret locators for one closed database-baseline runtime."""

	workspace: pathlib.Path
	manifest_path: pathlib.Path
	compose_environment_path: pathlib.Path
	cleanup_capability_path: pathlib.Path
	admin_url_path: pathlib.Path
	grader_url_path: pathlib.Path
	fast_path_url_path: pathlib.Path
	recovery_url_path: pathlib.Path
	admin_password_path: pathlib.Path


@dataclasses.dataclass(frozen=True)
class CourseAppearanceCrossStoreRuntime:
	"""Validated private locators for the closed PostgreSQL and MinIO oracle."""

	workspace: pathlib.Path
	manifest_path: pathlib.Path
	compose_environment_path: pathlib.Path
	cleanup_capability_path: pathlib.Path
	admin_url_path: pathlib.Path
	grader_url_path: pathlib.Path
	fast_path_url_path: pathlib.Path
	recovery_url_path: pathlib.Path
	admin_password_path: pathlib.Path
	minio_endpoint_path: pathlib.Path
	minio_region_path: pathlib.Path
	minio_access_key_id_path: pathlib.Path
	minio_secret_access_key_path: pathlib.Path


class _StrictRuntimeLoader(yaml.SafeLoader):
	"""Safe YAML loader that rejects duplicate mapping keys."""


#============================================
def _construct_mapping(
	loader: _StrictRuntimeLoader,
	node: yaml.MappingNode,
	deep: bool = False,
) -> dict[object, object]:
	"""Construct one mapping while requiring unique scalar keys."""
	result: dict[object, object] = {}
	for key_node, value_node in node.value:
		key = loader.construct_object(key_node, deep=deep)
		if not isinstance(key, str) or key in result:
			raise local_stack_control.models.ControllerError(
				"acceptance runtime manifest schema is invalid"
			)
		result[key] = loader.construct_object(value_node, deep=deep)
	return result


_StrictRuntimeLoader.add_constructor(
	yaml.resolver.BaseResolver.DEFAULT_MAPPING_TAG,
	_construct_mapping,
)


#============================================
def _error(message: str) -> local_stack_control.models.ControllerError:
	"""Build one redacted runtime-manifest error."""
	return local_stack_control.models.ControllerError(message)


#============================================
def _require_private_platform() -> None:
	"""Require the Unix no-follow primitives that make this file boundary meaningful."""
	if not hasattr(os, "O_NOFOLLOW") or not hasattr(os, "O_DIRECTORY"):
		raise _error("acceptance runtime requires Unix private-file support")


#============================================
def _open_private_workspace(directory: pathlib.Path) -> int:
	"""Open one current-user exact-mode workspace and retain its descriptor."""
	try:
		metadata = os.lstat(directory)
	except OSError as error:
		raise _error("acceptance runtime workspace is unavailable") from error
	if stat.S_ISLNK(metadata.st_mode) or not stat.S_ISDIR(metadata.st_mode):
		raise _error("acceptance runtime workspace is invalid")
	flags = os.O_RDONLY | os.O_DIRECTORY | os.O_NOFOLLOW
	try:
		descriptor = os.open(directory, flags)
	except OSError as error:
		raise _error("acceptance runtime workspace is unavailable") from error
	try:
		opened = os.fstat(descriptor)
	except OSError as error:
		os.close(descriptor)
		raise _error("acceptance runtime workspace is unavailable") from error
	if (
		not stat.S_ISDIR(opened.st_mode)
		or opened.st_uid != os.getuid()
		or stat.S_IMODE(opened.st_mode) != 0o700
		or (metadata.st_dev, metadata.st_ino) != (opened.st_dev, opened.st_ino)
	):
		os.close(descriptor)
		raise _error("acceptance runtime workspace is invalid")
	os.set_inheritable(descriptor, False)
	return descriptor


#============================================
def _open_private_directory_at(parent_descriptor: int, name: str, field: str) -> int:
	"""Open one exact mode-0700 child directory beneath its retained parent."""
	try:
		metadata = os.stat(name, dir_fd=parent_descriptor, follow_symlinks=False)
		if stat.S_ISLNK(metadata.st_mode) or not stat.S_ISDIR(metadata.st_mode):
			raise _error(f"acceptance runtime {field} is invalid")
		descriptor = os.open(
			name,
			os.O_RDONLY | os.O_DIRECTORY | os.O_NOFOLLOW,
			dir_fd=parent_descriptor,
		)
	except OSError as error:
		raise _error(f"acceptance runtime {field} is invalid") from error
	try:
		opened = os.fstat(descriptor)
	except OSError as error:
		os.close(descriptor)
		raise _error(f"acceptance runtime {field} is invalid") from error
	if (
		not stat.S_ISDIR(opened.st_mode)
		or opened.st_uid != os.getuid()
		or stat.S_IMODE(opened.st_mode) != 0o700
		or (metadata.st_dev, metadata.st_ino) != (opened.st_dev, opened.st_ino)
	):
		os.close(descriptor)
		raise _error(f"acceptance runtime {field} is invalid")
	os.set_inheritable(descriptor, False)
	return descriptor


#============================================
def _read_private_file_at(
	parent_descriptor: int,
	name: str,
	maximum_bytes: int,
	field: str,
) -> bytes:
	"""Read one bounded 0600 file beneath a retained directory descriptor."""
	if maximum_bytes < 1:
		raise _error(f"acceptance runtime {field} is invalid")
	try:
		metadata = os.stat(name, dir_fd=parent_descriptor, follow_symlinks=False)
		if stat.S_ISLNK(metadata.st_mode) or not stat.S_ISREG(metadata.st_mode):
			raise _error(f"acceptance runtime {field} is invalid")
		descriptor = os.open(name, os.O_RDONLY | os.O_NOFOLLOW, dir_fd=parent_descriptor)
	except OSError as error:
		raise _error(f"acceptance runtime {field} is invalid") from error
	try:
		opened = os.fstat(descriptor)
		if (
			not stat.S_ISREG(opened.st_mode)
			or opened.st_uid != os.getuid()
			or stat.S_IMODE(opened.st_mode) != 0o600
			or (metadata.st_dev, metadata.st_ino) != (opened.st_dev, opened.st_ino)
		):
			raise _error(f"acceptance runtime {field} is invalid")
		try:
			return local_stack_control.private_files.read_bounded_descriptor(descriptor, maximum_bytes)
		except local_stack_control.models.ControllerError as error:
			raise _error(f"acceptance runtime {field} is invalid") from error
	finally:
		os.close(descriptor)


#============================================
def _parse_yaml(manifest: bytes) -> Mapping[str, object]:
	"""Parse one ordinary single-document YAML mapping with a closed surface."""
	try:
		text = manifest.decode("ascii")
	except UnicodeDecodeError as error:
		raise _error("acceptance runtime manifest schema is invalid") from error
	if any(token in text for token in ("&", "*", "!")) or any(
		line.strip() in ("---", "...") for line in text.splitlines()
	):
		raise _error("acceptance runtime manifest schema is invalid")
	loader = _StrictRuntimeLoader(text)
	try:
		parsed = loader.get_single_data()
	except (local_stack_control.models.ControllerError, yaml.YAMLError) as error:
		raise _error("acceptance runtime manifest schema is invalid") from error
	finally:
		loader.dispose()
	if not isinstance(parsed, dict):
		raise _error("acceptance runtime manifest schema is invalid")
	return parsed


#============================================
def _require_mapping(value: object, keys: tuple[str, ...], field: str) -> Mapping[str, object]:
	"""Require one exact string-keyed mapping at a schema level."""
	if not isinstance(value, dict) or tuple(sorted(value)) != tuple(sorted(keys)):
		raise _error(f"acceptance runtime {field} is invalid")
	if any(not isinstance(item, str) for item in value):
		raise _error(f"acceptance runtime {field} is invalid")
	return value


#============================================
def _url_secret(
	secrets_descriptor: int,
	name: str,
	expected_user: str,
	field: str,
) -> str:
	"""Validate one bounded loopback PostgreSQL connection URL without revealing it."""
	content = _read_private_file_at(secrets_descriptor, name, MAX_DATABASE_URL_BYTES, field)
	if (
		not content.isascii()
		or not content.endswith(b"\n")
		or b"\n" in content[:-1]
	):
		raise _error(f"acceptance runtime {field} is invalid")
	try:
		parsed = urllib.parse.urlsplit(content[:-1].decode("ascii"))
	except ValueError as error:
		raise _error(f"acceptance runtime {field} is invalid") from error
	try:
		password = parsed.password or ""
		valid = (
			parsed.scheme == "postgres"
			and parsed.hostname in ("127.0.0.1", "::1")
			and parsed.port is not None and parsed.port > 0
			and parsed.username == expected_user
			and PASSWORD_PATTERN.fullmatch(password) is not None
			and parsed.path == f"/{DATABASE_NAME}"
			and parsed.query == ""
			and parsed.fragment == ""
		)
	except ValueError as error:
		raise _error(f"acceptance runtime {field} is invalid") from error
	if not valid:
		raise _error(f"acceptance runtime {field} is invalid")
	return password


#============================================
def _admin_password_secret(secrets_descriptor: int, expected_password: str) -> None:
	"""Bind the Compose password file to the validated administrative URL secret."""
	content = _read_private_file_at(
		secrets_descriptor,
		"postgres-admin.password",
		ADMIN_PASSWORD_FILE_BYTES,
		"postgres admin password",
	)
	if (
		not content.isascii()
		or len(content) != ADMIN_PASSWORD_FILE_BYTES
		or not content.endswith(b"\n")
	):
		raise _error("acceptance runtime postgres admin password is invalid")
	password = content[:-1].decode("ascii")
	if PASSWORD_PATTERN.fullmatch(password) is None or not hmac.compare_digest(password, expected_password):
		raise _error("acceptance runtime postgres admin password is invalid")


#============================================
def load_database_baseline_runtime(workspace: pathlib.Path) -> DatabaseBaselineRuntime:
	"""Load the only accepted database-baseline target from its private workspace."""
	_require_private_platform()
	workspace = workspace.absolute()
	workspace_descriptor = _open_private_workspace(workspace)
	try:
		manifest = _parse_yaml(
			_read_private_file_at(
				workspace_descriptor,
				MANIFEST_NAME,
				MAX_MANIFEST_BYTES,
				"manifest",
			)
		)
		return _load_runtime_from_workspace_descriptor(workspace, workspace_descriptor, manifest)
	finally:
		os.close(workspace_descriptor)


#============================================
def require_database_baseline_compose_password(workspace: pathlib.Path) -> None:
	"""Revalidate the exact bind-mounted administrative password immediately before Compose."""
	load_database_baseline_runtime(workspace)


#============================================
def acceptance_runtime_profile(workspace: pathlib.Path) -> local_stack_control.models.LiveDemoProfile:
	"""Read the closed profile discriminator through the private manifest boundary."""
	# ASVS 1.5.2 and 2.2.1: parse only the private, closed manifest shape before dispatch.
	_require_private_platform()
	workspace = workspace.absolute()
	workspace_descriptor = _open_private_workspace(workspace)
	try:
		manifest = _parse_yaml(
			_read_private_file_at(
				workspace_descriptor, MANIFEST_NAME, MAX_MANIFEST_BYTES, "manifest"
			)
		)
		if tuple(sorted(manifest)) != ("identity", "kind", "schema_version", "secrets"):
			raise _error("acceptance runtime manifest schema is invalid")
		identity = _require_mapping(manifest["identity"], ("owner", "project", "profile"), "identity")
		if identity["owner"] != local_stack_control.models.LIVE_DEMO_BROWSER_OWNER or identity["project"] != local_stack_control.models.LIVE_DEMO_BROWSER_PROJECT:
			raise _error("acceptance runtime identity is invalid")
		try:
			return local_stack_control.models.live_demo_profile(identity["profile"])
		except TypeError as error:
			raise _error("acceptance runtime identity is invalid") from error
	finally:
		os.close(workspace_descriptor)


#============================================
def load_course_appearance_cross_store_runtime(
	workspace: pathlib.Path,
) -> CourseAppearanceCrossStoreRuntime:
	"""Load the closed PostgreSQL and MinIO acceptance target from private state."""
	_require_private_platform()
	workspace = workspace.absolute()
	workspace_descriptor = _open_private_workspace(workspace)
	try:
		manifest = _parse_yaml(
			_read_private_file_at(
				workspace_descriptor, MANIFEST_NAME, MAX_MANIFEST_BYTES, "manifest"
			)
		)
		return _load_course_appearance_cross_store_runtime(
			workspace, workspace_descriptor, manifest
		)
	finally:
		os.close(workspace_descriptor)


#============================================
def require_course_appearance_cross_store_compose_credentials(
	workspace: pathlib.Path,
) -> None:
	"""Revalidate the complete cross-store secret set immediately before Compose."""
	load_course_appearance_cross_store_runtime(workspace)


#============================================
def _load_course_appearance_cross_store_runtime(
	workspace: pathlib.Path,
	workspace_descriptor: int,
	manifest: Mapping[str, object],
) -> CourseAppearanceCrossStoreRuntime:
	"""Validate the exact cross-store schema and return only private locators."""
	# ASVS 1.5.2 and 2.2.3: exact keys bind the profile, secret locators, and paired services.
	if tuple(sorted(manifest)) != ("identity", "kind", "schema_version", "secrets"):
		raise _error("acceptance runtime manifest schema is invalid")
	if (
		type(manifest["schema_version"]) is not int
		or manifest["schema_version"] != 1
		or manifest["kind"] != "ple.disposable_postgres_minio_acceptance"
	):
		raise _error("acceptance runtime manifest schema is invalid")
	identity = _require_mapping(manifest["identity"], ("owner", "project", "profile"), "identity")
	if identity != {
		"owner": local_stack_control.models.LIVE_DEMO_BROWSER_OWNER,
		"project": local_stack_control.models.LIVE_DEMO_BROWSER_PROJECT,
		"profile": local_stack_control.models.LiveDemoProfile.COURSE_APPEARANCE_CROSS_STORE.value,
	}:
		raise _error("acceptance runtime identity is invalid")
	expected = {
		"compose_environment": COMPOSE_ENVIRONMENT,
		"cleanup_capability": CLEANUP_CAPABILITY,
		"postgres_admin_url": POSTGRES_ADMIN_URL,
		"postgres_grader_url": POSTGRES_GRADER_URL,
		"postgres_fast_path_url": POSTGRES_FAST_PATH_URL,
		"postgres_recovery_url": POSTGRES_RECOVERY_URL,
		"postgres_admin_password": POSTGRES_ADMIN_PASSWORD,
		"minio_endpoint": MINIO_ENDPOINT,
		"minio_region": MINIO_REGION,
		"minio_access_key_id": MINIO_ACCESS_KEY_ID,
		"minio_secret_access_key": MINIO_SECRET_ACCESS_KEY,
	}
	if _require_mapping(manifest["secrets"], tuple(expected), "secrets") != expected:
		raise _error("acceptance runtime secret paths are invalid")
	secrets_descriptor = _open_private_directory_at(
		workspace_descriptor, SECRETS_DIRECTORY, "secrets directory"
	)
	try:
		_read_private_file_at(secrets_descriptor, "compose.env", MAX_COMPOSE_ENVIRONMENT_BYTES, "compose environment")
		capability = _read_private_file_at(secrets_descriptor, "cleanup.capability", 32, "cleanup capability")
		if len(capability) != 32:
			raise _error("acceptance runtime cleanup capability is invalid")
		admin_password = _url_secret(secrets_descriptor, "postgres-admin.url", POSTGRES_USER, "postgres admin URL")
		_admin_password_secret(secrets_descriptor, admin_password)
		_url_secret(secrets_descriptor, "postgres-grader.url", GRADER_USER, "postgres grader URL")
		_url_secret(secrets_descriptor, "postgres-fast-path.url", FAST_PATH_USER, "postgres fast-path URL")
		_url_secret(secrets_descriptor, "postgres-recovery.url", RECOVERY_USER, "postgres recovery URL")
		_minio_endpoint_secret(secrets_descriptor)
		_minio_text_secret(secrets_descriptor, "minio-region", "us-east-1", "minio region")
		_minio_text_secret(secrets_descriptor, "minio-access-key-id", None, "minio access key")
		_minio_text_secret(secrets_descriptor, "minio-secret-access-key", None, "minio secret key")
	finally:
		os.close(secrets_descriptor)
	return CourseAppearanceCrossStoreRuntime(
		workspace=workspace,
		manifest_path=workspace / MANIFEST_NAME,
		compose_environment_path=workspace / COMPOSE_ENVIRONMENT,
		cleanup_capability_path=workspace / CLEANUP_CAPABILITY,
		admin_url_path=workspace / POSTGRES_ADMIN_URL,
		grader_url_path=workspace / POSTGRES_GRADER_URL,
		fast_path_url_path=workspace / POSTGRES_FAST_PATH_URL,
		recovery_url_path=workspace / POSTGRES_RECOVERY_URL,
		admin_password_path=workspace / POSTGRES_ADMIN_PASSWORD,
		minio_endpoint_path=workspace / MINIO_ENDPOINT,
		minio_region_path=workspace / MINIO_REGION,
		minio_access_key_id_path=workspace / MINIO_ACCESS_KEY_ID,
		minio_secret_access_key_path=workspace / MINIO_SECRET_ACCESS_KEY,
	)


#============================================
def _minio_endpoint_secret(secrets_descriptor: int) -> None:
	"""Require one bounded loopback-only MinIO endpoint locator."""
	content = _read_private_file_at(secrets_descriptor, "minio-endpoint.url", 1_024, "minio endpoint")
	if not content.isascii() or not content.endswith(b"\n") or b"\n" in content[:-1]:
		raise _error("acceptance runtime minio endpoint is invalid")
	try:
		parsed = urllib.parse.urlsplit(content[:-1].decode("ascii"))
		valid = (
			parsed.scheme == "http"
			and parsed.hostname in ("127.0.0.1", "::1")
			and parsed.port is not None
			and 1024 <= parsed.port <= 65535
			and parsed.path == ""
			and parsed.query == ""
			and parsed.fragment == ""
		)
	except ValueError as error:
		raise _error("acceptance runtime minio endpoint is invalid") from error
	if not valid:
		raise _error("acceptance runtime minio endpoint is invalid")


#============================================
def _minio_text_secret(
	secrets_descriptor: int,
	name: str,
	expected: str | None,
	field: str,
) -> None:
	"""Require one bounded printable MinIO credential or fixed region value."""
	content = _read_private_file_at(secrets_descriptor, name, 128, field)
	if not content.isascii() or not content.endswith(b"\n") or b"\n" in content[:-1]:
		raise _error(f"acceptance runtime {field} is invalid")
	value = content[:-1].decode("ascii")
	if expected is not None:
		valid = hmac.compare_digest(value, expected)
	else:
		valid = MINIO_CREDENTIAL_PATTERN.fullmatch(value) is not None
	if not valid:
		raise _error(f"acceptance runtime {field} is invalid")


#============================================
def _load_runtime_from_workspace_descriptor(
	workspace: pathlib.Path,
	workspace_descriptor: int,
	manifest: Mapping[str, object],
) -> DatabaseBaselineRuntime:
	"""Validate one parsed runtime and return its non-secret file locators."""
	runtime, _passwords = _validated_runtime_and_service_passwords(
		workspace,
		workspace_descriptor,
		manifest,
	)
	return runtime


#============================================
def _validated_runtime_and_service_passwords(
	workspace: pathlib.Path,
	workspace_descriptor: int,
	manifest: Mapping[str, object],
) -> tuple[DatabaseBaselineRuntime, tuple[str, ...]]:
	"""Validate one parsed runtime while retaining its opened workspace directory."""
	if tuple(sorted(manifest)) != ("identity", "kind", "schema_version", "secrets"):
		raise _error("acceptance runtime manifest schema is invalid")
	if (
		type(manifest["schema_version"]) is not int
		or manifest["schema_version"] != 1
		or manifest["kind"] != "ple.disposable_postgres_acceptance"
	):
		raise _error("acceptance runtime manifest schema is invalid")
	identity = _require_mapping(manifest["identity"], ("owner", "project", "profile"), "identity")
	if identity != {
		"owner": local_stack_control.models.LIVE_DEMO_BROWSER_OWNER,
		"project": local_stack_control.models.LIVE_DEMO_BROWSER_PROJECT,
		"profile": local_stack_control.models.LiveDemoProfile.DATABASE_BASELINE.value,
	}:
		raise _error("acceptance runtime identity is invalid")
	secret_values = _require_mapping(
		manifest["secrets"],
		(
			"compose_environment",
			"cleanup_capability",
			"postgres_admin_url",
			"postgres_grader_url",
			"postgres_fast_path_url",
			"postgres_recovery_url",
			"postgres_admin_password",
		),
		"secrets",
	)
	expected = {
		"compose_environment": COMPOSE_ENVIRONMENT,
		"cleanup_capability": CLEANUP_CAPABILITY,
		"postgres_admin_url": POSTGRES_ADMIN_URL,
		"postgres_grader_url": POSTGRES_GRADER_URL,
		"postgres_fast_path_url": POSTGRES_FAST_PATH_URL,
		"postgres_recovery_url": POSTGRES_RECOVERY_URL,
		"postgres_admin_password": POSTGRES_ADMIN_PASSWORD,
	}
	if secret_values != expected:
		raise _error("acceptance runtime secret paths are invalid")
	secrets_descriptor = _open_private_directory_at(
		workspace_descriptor,
		SECRETS_DIRECTORY,
		"secrets directory",
	)
	try:
		_read_private_file_at(
			secrets_descriptor,
			"compose.env",
			MAX_COMPOSE_ENVIRONMENT_BYTES,
			"compose environment",
		)
		capability = _read_private_file_at(
			secrets_descriptor,
			"cleanup.capability",
			32,
			"cleanup capability",
		)
		if len(capability) != 32:
			raise _error("acceptance runtime cleanup capability is invalid")
		admin_password = _url_secret(
			secrets_descriptor,
			"postgres-admin.url",
			POSTGRES_USER,
			"postgres admin URL",
		)
		_admin_password_secret(secrets_descriptor, admin_password)
		grader_password = _url_secret(
			secrets_descriptor,
			"postgres-grader.url",
			GRADER_USER,
			"postgres grader URL",
		)
		fast_path_password = _url_secret(
			secrets_descriptor,
			"postgres-fast-path.url",
			FAST_PATH_USER,
			"postgres fast-path URL",
		)
		recovery_password = _url_secret(
			secrets_descriptor,
			"postgres-recovery.url",
			RECOVERY_USER,
			"postgres recovery URL",
		)
	finally:
		os.close(secrets_descriptor)
	runtime = DatabaseBaselineRuntime(
		workspace=workspace,
		manifest_path=workspace / MANIFEST_NAME,
		compose_environment_path=workspace / COMPOSE_ENVIRONMENT,
		cleanup_capability_path=workspace / CLEANUP_CAPABILITY,
		admin_url_path=workspace / POSTGRES_ADMIN_URL,
		grader_url_path=workspace / POSTGRES_GRADER_URL,
		fast_path_url_path=workspace / POSTGRES_FAST_PATH_URL,
		recovery_url_path=workspace / POSTGRES_RECOVERY_URL,
		admin_password_path=workspace / POSTGRES_ADMIN_PASSWORD,
	)
	return runtime, (grader_password, fast_path_password, recovery_password)


#============================================
def _emit_service_login_profiles(
	workspace: pathlib.Path,
	profiles: tuple[_ServiceLoginProfile, ...],
) -> None:
	"""Write one fixed transaction for the selected closed login profiles."""
	_require_private_platform()
	workspace = workspace.absolute()
	workspace_descriptor = _open_private_workspace(workspace)
	try:
		manifest = _parse_yaml(
			_read_private_file_at(
				workspace_descriptor,
				MANIFEST_NAME,
				MAX_MANIFEST_BYTES,
				"manifest",
			)
		)
		_runtime, passwords = _validated_runtime_and_service_passwords(
			workspace,
			workspace_descriptor,
			manifest,
		)
	finally:
		os.close(workspace_descriptor)
	passwords_by_profile = dict(zip(SERVICE_LOGIN_PROFILES, passwords, strict=True))
	statements = ["BEGIN;"]
	for profile in profiles:
		statements.extend(_service_login_sql(profile, passwords_by_profile[profile]))
	statements.append("COMMIT;")
	print("\n".join(statements))


#============================================
def emit_grader_login_provisioning(workspace: pathlib.Path) -> None:
	"""Write validated SQL for the migration-owned grader capability login."""
	_emit_service_login_profiles(workspace, GRADER_LOGIN_PROFILES)


#============================================
def emit_accepted_submission_login_provisioning(workspace: pathlib.Path) -> None:
	"""Write validated SQL for the neutral accepted-submission capability logins."""
	_emit_service_login_profiles(workspace, NEUTRAL_LOGIN_PROFILES)


#============================================
def _service_login_sql(profile: _ServiceLoginProfile, password: str) -> list[str]:
	"""Build fixed SQL for one allowlisted baseline service-login profile."""
	if profile not in SERVICE_LOGIN_PROFILES or PASSWORD_PATTERN.fullmatch(password) is None:
		raise _error("acceptance runtime service-login profile is invalid")
	login = profile.login
	statements = [
		"DO $$",
		"BEGIN",
		f"\tIF NOT EXISTS (SELECT 1 FROM pg_roles WHERE rolname = '{login}') THEN",
		f"\t\tCREATE ROLE {login} LOGIN;",
		"\tEND IF;",
		"END",
		"$$;",
		"DO $$",
		"DECLARE membership record;",
		"BEGIN",
		"\tFOR membership IN",
		"\t\tSELECT parent.rolname AS parent_name, member.rolname AS member_name",
		"\t\tFROM pg_auth_members AS grant_map",
		"\t\tJOIN pg_roles AS parent ON parent.oid = grant_map.roleid",
		"\t\tJOIN pg_roles AS member ON member.oid = grant_map.member",
		f"\t\tWHERE member.rolname = '{login}'",
		"\tLOOP",
		"\t\tEXECUTE format('REVOKE %I FROM %I', membership.parent_name, membership.member_name);",
		"\tEND LOOP;",
		"END",
		"$$;",
		f"ALTER ROLE {login}",
		"\tLOGIN NOSUPERUSER NOCREATEDB NOCREATEROLE NOINHERIT",
		f"\tNOREPLICATION NOBYPASSRLS CONNECTION LIMIT 8 PASSWORD '{password}';",
	]
	if profile.revoke_object_privileges:
		statements.extend(
			[
				"DO $$",
				"BEGIN",
				f"\tEXECUTE format('REVOKE ALL PRIVILEGES ON DATABASE %I FROM {login}', current_database());",
				"END",
				"$$;",
				f"REVOKE ALL PRIVILEGES ON SCHEMA public FROM {login};",
				f"REVOKE ALL PRIVILEGES ON ALL TABLES IN SCHEMA public FROM {login};",
				f"REVOKE ALL PRIVILEGES ON ALL SEQUENCES IN SCHEMA public FROM {login};",
				f"REVOKE ALL PRIVILEGES ON ALL FUNCTIONS IN SCHEMA public FROM {login};",
			]
		)
	statements.extend(
		f"GRANT {role} TO {login} WITH INHERIT FALSE, SET TRUE, ADMIN FALSE;"
		for role in profile.roles
	)
	return statements


#============================================
def main(argv: list[str] | None = None) -> None:
	"""Provide explicit pipe-only helpers for closed service-login provisioning phases."""
	arguments = sys.argv[1:] if argv is None else argv
	commands = {
		"--emit-grader-login-provisioning": emit_grader_login_provisioning,
		"--emit-accepted-submission-login-provisioning": emit_accepted_submission_login_provisioning,
		"--emit-sd1-staged-bootstrap": emit_sd1_staged_bootstrap,
	}
	if len(arguments) != 2 or arguments[0] not in commands:
		print(
			"usage: python3 -m local_stack_control.runtime_manifest "
			"--emit-grader-login-provisioning WORKSPACE",
			file=sys.stderr,
		)
		print(
			"   or: python3 -m local_stack_control.runtime_manifest "
			"--emit-accepted-submission-login-provisioning WORKSPACE",
			file=sys.stderr,
		)
		print(
			"   or: python3 -m local_stack_control.runtime_manifest "
			"--emit-sd1-staged-bootstrap WORKSPACE",
			file=sys.stderr,
		)
		raise SystemExit(2)
	try:
		commands[arguments[0]](pathlib.Path(arguments[1]))
	except local_stack_control.models.ControllerError:
		print("acceptance runtime service-login provisioning is unavailable", file=sys.stderr)
		raise SystemExit(2) from None


#============================================
def _write_private_file_at(parent_descriptor: int, name: str, content: bytes) -> None:
	"""Create one new 0600 file beneath a retained private directory descriptor."""
	try:
		descriptor = os.open(
			name,
			os.O_WRONLY | os.O_CREAT | os.O_EXCL | os.O_NOFOLLOW,
			0o600,
			dir_fd=parent_descriptor,
		)
	except OSError as error:
		raise _error("acceptance runtime private state is unavailable") from error
	try:
		os.fchmod(descriptor, 0o600)
		local_stack_control.private_files.write_all(descriptor, content)
		os.fsync(descriptor)
		metadata = os.fstat(descriptor)
		if (
			not stat.S_ISREG(metadata.st_mode)
			or metadata.st_uid != os.getuid()
			or stat.S_IMODE(metadata.st_mode) != 0o600
		):
			raise _error("acceptance runtime private state is invalid")
	finally:
		os.close(descriptor)


#============================================
def write_database_baseline_runtime(workspace: pathlib.Path, port: int) -> DatabaseBaselineRuntime:
	"""Create the complete closed runtime handoff before starting the private child."""
	_require_private_platform()
	workspace = workspace.absolute()
	workspace_descriptor = _open_private_workspace(workspace)
	if not isinstance(port, int) or isinstance(port, bool) or not 1024 <= port <= 65535:
		os.close(workspace_descriptor)
		raise _error("acceptance runtime port is invalid")
	try:
		try:
			os.mkdir(SECRETS_DIRECTORY, 0o700, dir_fd=workspace_descriptor)
		except OSError as error:
			raise _error("acceptance runtime private state is unavailable") from error
		secrets_descriptor = _open_private_directory_at(
			workspace_descriptor,
			SECRETS_DIRECTORY,
			"secrets directory",
		)
		try:
			postgres_password = secrets.token_urlsafe(24)
			grader_password = secrets.token_urlsafe(24)
			fast_path_password = secrets.token_urlsafe(24)
			recovery_password = secrets.token_urlsafe(24)
			capability = secrets.token_bytes(32)
			capability_digest = hashlib.sha256(capability).hexdigest()
			endpoint = f"@127.0.0.1:{port}/{DATABASE_NAME}\n"
			base = (
				f"postgres://{POSTGRES_USER}:"
				f"{urllib.parse.quote(postgres_password, safe='')}{endpoint}"
			)
			grader = (
				f"postgres://{GRADER_USER}:"
				f"{urllib.parse.quote(grader_password, safe='')}{endpoint}"
			)
			fast_path = (
				f"postgres://{FAST_PATH_USER}:"
				f"{urllib.parse.quote(fast_path_password, safe='')}{endpoint}"
			)
			recovery = (
				f"postgres://{RECOVERY_USER}:"
				f"{urllib.parse.quote(recovery_password, safe='')}{endpoint}"
			)
			password_path = workspace / POSTGRES_ADMIN_PASSWORD
			compose_environment = (
				f"POSTGRES_USER={POSTGRES_USER}\n"
				f"POSTGRES_DB=postgres\n"
				"POSTGRES_PASSWORD_FILE=/run/ple-runtime/postgres-password\n"
				f"PLE_DATABASE_BASELINE_POSTGRES_PASSWORD_FILE={password_path}\n"
				f"PLE_POSTGRES_HOST_PORT={port}\n"
				f"PLE_E2E_OWNER={local_stack_control.models.LIVE_DEMO_BROWSER_OWNER}\n"
				f"PLE_DISPOSABLE_CAPABILITY_SHA256={capability_digest}\n"
			).encode("ascii")
			manifest = (
				"schema_version: 1\n"
				"kind: ple.disposable_postgres_acceptance\n"
				"identity:\n"
				f"  owner: {local_stack_control.models.LIVE_DEMO_BROWSER_OWNER}\n"
				f"  project: {local_stack_control.models.LIVE_DEMO_BROWSER_PROJECT}\n"
				"  profile: database_baseline\n"
				"secrets:\n"
				f"  compose_environment: {COMPOSE_ENVIRONMENT}\n"
				f"  cleanup_capability: {CLEANUP_CAPABILITY}\n"
				f"  postgres_admin_url: {POSTGRES_ADMIN_URL}\n"
				f"  postgres_grader_url: {POSTGRES_GRADER_URL}\n"
				f"  postgres_fast_path_url: {POSTGRES_FAST_PATH_URL}\n"
				f"  postgres_recovery_url: {POSTGRES_RECOVERY_URL}\n"
				f"  postgres_admin_password: {POSTGRES_ADMIN_PASSWORD}\n"
			).encode("ascii")
			_write_private_file_at(secrets_descriptor, "compose.env", compose_environment)
			_write_private_file_at(secrets_descriptor, "cleanup.capability", capability)
			_write_private_file_at(secrets_descriptor, "postgres-admin.url", base.encode("ascii"))
			_write_private_file_at(secrets_descriptor, "postgres-grader.url", grader.encode("ascii"))
			_write_private_file_at(secrets_descriptor, "postgres-fast-path.url", fast_path.encode("ascii"))
			_write_private_file_at(secrets_descriptor, "postgres-recovery.url", recovery.encode("ascii"))
			_write_private_file_at(
				secrets_descriptor,
				"postgres-admin.password",
				postgres_password.encode("ascii") + b"\n",
			)
			_write_private_file_at(workspace_descriptor, MANIFEST_NAME, manifest)
		finally:
			os.close(secrets_descriptor)
	finally:
		os.close(workspace_descriptor)
	return load_database_baseline_runtime(workspace)


#============================================
def write_sd1_staged_database_runtime(workspace: pathlib.Path, port: int) -> pathlib.Path:
	"""Create the nested private runtime used only by staged SD1 migration tools."""
	# ASVS 1.2.4, 2.2.1, and 8.3.1: the migrator URL is generated privately and
	# the child receives only its exact closed profile locator.
	_require_private_platform()
	workspace = workspace.absolute()
	if not isinstance(port, int) or isinstance(port, bool) or not 1024 <= port <= 65535:
		raise _error("acceptance runtime port is invalid")
	workspace_descriptor = _open_private_workspace(workspace)
	try:
		try:
			os.mkdir(SD1_RUNTIME_DIRECTORY, 0o700, dir_fd=workspace_descriptor)
		except OSError as error:
			raise _error("SD1 acceptance runtime is unavailable") from error
		sd1_descriptor = _open_private_directory_at(
			workspace_descriptor, SD1_RUNTIME_DIRECTORY, "SD1 runtime directory"
		)
		try:
			try:
				os.mkdir(SECRETS_DIRECTORY, 0o700, dir_fd=sd1_descriptor)
			except OSError as error:
				raise _error("SD1 acceptance runtime is unavailable") from error
			secrets_descriptor = _open_private_directory_at(
				sd1_descriptor, SECRETS_DIRECTORY, "SD1 secrets directory"
			)
			try:
				password = secrets.token_urlsafe(24)
				endpoint = f"@127.0.0.1:{port}/{DATABASE_NAME}\n"
				url = (
					f"postgres://{SD1_MIGRATOR_ROLE}:"
					f"{urllib.parse.quote(password, safe='')}{endpoint}"
				).encode("ascii")
				manifest = (
					"schema_version: 1\n"
					f"kind: {SD1_RUNTIME_KIND}\n"
					"identity:\n"
					f"  owner: {local_stack_control.models.LIVE_DEMO_BROWSER_OWNER}\n"
					f"  project: {local_stack_control.models.LIVE_DEMO_BROWSER_PROJECT}\n"
					f"  profile: {SD1_RUNTIME_PROFILE}\n"
					"secrets:\n"
					f"  postgres_migrator_url: {SD1_MIGRATOR_URL}\n"
				).encode("ascii")
				_write_private_file_at(secrets_descriptor, "postgres-migrator.url", url)
				_write_private_file_at(sd1_descriptor, MANIFEST_NAME, manifest)
			finally:
				os.close(secrets_descriptor)
		finally:
			os.close(sd1_descriptor)
	finally:
		os.close(workspace_descriptor)
	return workspace / SD1_RUNTIME_DIRECTORY / MANIFEST_NAME


#============================================
def emit_sd1_staged_bootstrap(workspace: pathlib.Path) -> None:
	"""Emit private SQL that binds the bootstrap-created role to its URL secret."""
	_require_private_platform()
	workspace = workspace.absolute()
	workspace_descriptor = _open_private_workspace(workspace)
	try:
		sd1_descriptor = _open_private_directory_at(
			workspace_descriptor, SD1_RUNTIME_DIRECTORY, "SD1 runtime directory"
		)
		try:
			secrets_descriptor = _open_private_directory_at(
				sd1_descriptor, SECRETS_DIRECTORY, "SD1 secrets directory"
			)
			try:
				password = _url_secret(
					secrets_descriptor,
					"postgres-migrator.url",
					SD1_MIGRATOR_ROLE,
					"SD1 migrator URL",
				)
			finally:
				os.close(secrets_descriptor)
		finally:
			os.close(sd1_descriptor)
	finally:
		os.close(workspace_descriptor)
	print("BEGIN;")
	print(f"ALTER ROLE {SD1_MIGRATOR_ROLE} PASSWORD '{password}';")
	print("COMMIT;")


#============================================
def write_course_appearance_cross_store_runtime(
	workspace: pathlib.Path,
	postgres_port: int,
	minio_port: int,
) -> CourseAppearanceCrossStoreRuntime:
	"""Create the private two-store runtime before the lease-owned child starts."""
	_require_private_platform()
	workspace = workspace.absolute()
	if (
		not isinstance(postgres_port, int)
		or isinstance(postgres_port, bool)
		or not isinstance(minio_port, int)
		or isinstance(minio_port, bool)
		or not 1024 <= postgres_port <= 65535
		or not 1024 <= minio_port <= 65535
		or postgres_port == minio_port
	):
		raise _error("acceptance runtime port is invalid")
	workspace_descriptor = _open_private_workspace(workspace)
	try:
		try:
			os.mkdir(SECRETS_DIRECTORY, 0o700, dir_fd=workspace_descriptor)
		except OSError as error:
			raise _error("acceptance runtime private state is unavailable") from error
		secrets_descriptor = _open_private_directory_at(workspace_descriptor, SECRETS_DIRECTORY, "secrets directory")
		try:
			postgres_password = secrets.token_urlsafe(24)
			grader_password = secrets.token_urlsafe(24)
			fast_path_password = secrets.token_urlsafe(24)
			recovery_password = secrets.token_urlsafe(24)
			# Hex remains opaque data through the shell and MinIO CLI boundary.
			minio_access_key_id = secrets.token_hex(16)
			minio_secret_access_key = secrets.token_hex(16)
			capability = secrets.token_bytes(32)
			capability_digest = hashlib.sha256(capability).hexdigest()
			endpoint = f"@127.0.0.1:{postgres_port}/{DATABASE_NAME}\n"
			def database_url(user: str, password: str) -> bytes:
				value = f"postgres://{user}:{urllib.parse.quote(password, safe='')}{endpoint}"
				return value.encode("ascii")
			password_path = workspace / POSTGRES_ADMIN_PASSWORD
			compose_environment = (
				f"POSTGRES_USER={POSTGRES_USER}\n"
				"POSTGRES_DB=postgres\n"
				"POSTGRES_PASSWORD_FILE=/run/ple-runtime/postgres-password\n"
				f"PLE_DATABASE_BASELINE_POSTGRES_PASSWORD_FILE={password_path}\n"
				f"PLE_POSTGRES_HOST_PORT={postgres_port}\n"
				f"MINIO_ROOT_USER={minio_access_key_id}\n"
				f"MINIO_ROOT_PASSWORD={minio_secret_access_key}\n"
				f"PLE_MINIO_API_HOST_PORT={minio_port}\n"
				f"PLE_E2E_OWNER={local_stack_control.models.LIVE_DEMO_BROWSER_OWNER}\n"
				f"PLE_DISPOSABLE_CAPABILITY_SHA256={capability_digest}\n"
			).encode("ascii")
			manifest = (
				"schema_version: 1\n"
				"kind: ple.disposable_postgres_minio_acceptance\n"
				"identity:\n"
				f"  owner: {local_stack_control.models.LIVE_DEMO_BROWSER_OWNER}\n"
				f"  project: {local_stack_control.models.LIVE_DEMO_BROWSER_PROJECT}\n"
				"  profile: course_appearance_cross_store\n"
				"secrets:\n"
				f"  compose_environment: {COMPOSE_ENVIRONMENT}\n"
				f"  cleanup_capability: {CLEANUP_CAPABILITY}\n"
				f"  postgres_admin_url: {POSTGRES_ADMIN_URL}\n"
				f"  postgres_grader_url: {POSTGRES_GRADER_URL}\n"
				f"  postgres_fast_path_url: {POSTGRES_FAST_PATH_URL}\n"
				f"  postgres_recovery_url: {POSTGRES_RECOVERY_URL}\n"
				f"  postgres_admin_password: {POSTGRES_ADMIN_PASSWORD}\n"
				f"  minio_endpoint: {MINIO_ENDPOINT}\n"
				f"  minio_region: {MINIO_REGION}\n"
				f"  minio_access_key_id: {MINIO_ACCESS_KEY_ID}\n"
				f"  minio_secret_access_key: {MINIO_SECRET_ACCESS_KEY}\n"
			).encode("ascii")
			_write_private_file_at(secrets_descriptor, "compose.env", compose_environment)
			_write_private_file_at(secrets_descriptor, "cleanup.capability", capability)
			_write_private_file_at(secrets_descriptor, "postgres-admin.url", database_url(POSTGRES_USER, postgres_password))
			_write_private_file_at(secrets_descriptor, "postgres-grader.url", database_url(GRADER_USER, grader_password))
			_write_private_file_at(secrets_descriptor, "postgres-fast-path.url", database_url(FAST_PATH_USER, fast_path_password))
			_write_private_file_at(secrets_descriptor, "postgres-recovery.url", database_url(RECOVERY_USER, recovery_password))
			_write_private_file_at(secrets_descriptor, "postgres-admin.password", postgres_password.encode("ascii") + b"\n")
			_write_private_file_at(secrets_descriptor, "minio-endpoint.url", f"http://127.0.0.1:{minio_port}\n".encode("ascii"))
			_write_private_file_at(secrets_descriptor, "minio-region", b"us-east-1\n")
			_write_private_file_at(secrets_descriptor, "minio-access-key-id", minio_access_key_id.encode("ascii") + b"\n")
			_write_private_file_at(secrets_descriptor, "minio-secret-access-key", minio_secret_access_key.encode("ascii") + b"\n")
			_write_private_file_at(workspace_descriptor, MANIFEST_NAME, manifest)
		finally:
			os.close(secrets_descriptor)
	finally:
		os.close(workspace_descriptor)
	return load_course_appearance_cross_store_runtime(workspace)


if __name__ == "__main__":
	main()

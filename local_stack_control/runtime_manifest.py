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
POSTGRES_ADMIN_PASSWORD = "secrets/postgres-admin.password"
DATABASE_NAME = "ple_e2e_baseline"
POSTGRES_USER = "ple_e2e_migrator"
GRADER_USER = "ple_grading_reader"
MAX_MANIFEST_BYTES = 4_096
MAX_COMPOSE_ENVIRONMENT_BYTES = 16_384
MAX_DATABASE_URL_BYTES = 4_096
ADMIN_PASSWORD_BYTES = 32
ADMIN_PASSWORD_FILE_BYTES = ADMIN_PASSWORD_BYTES + 1
PASSWORD_PATTERN = re.compile(r"^[A-Za-z0-9_-]{32}$")


@dataclasses.dataclass(frozen=True)
class DatabaseBaselineRuntime:
	"""Validated non-secret locators for one closed database-baseline runtime."""

	workspace: pathlib.Path
	manifest_path: pathlib.Path
	compose_environment_path: pathlib.Path
	cleanup_capability_path: pathlib.Path
	admin_url_path: pathlib.Path
	grader_url_path: pathlib.Path
	admin_password_path: pathlib.Path


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
def _load_runtime_from_workspace_descriptor(
	workspace: pathlib.Path,
	workspace_descriptor: int,
	manifest: Mapping[str, object],
) -> DatabaseBaselineRuntime:
	"""Validate one parsed runtime and return its non-secret file locators."""
	runtime, _grader_password = _validated_runtime_and_grader_password(
		workspace,
		workspace_descriptor,
		manifest,
	)
	return runtime


#============================================
def _validated_runtime_and_grader_password(
	workspace: pathlib.Path,
	workspace_descriptor: int,
	manifest: Mapping[str, object],
) -> tuple[DatabaseBaselineRuntime, str]:
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
			"postgres_admin_password",
		),
		"secrets",
	)
	expected = {
		"compose_environment": COMPOSE_ENVIRONMENT,
		"cleanup_capability": CLEANUP_CAPABILITY,
		"postgres_admin_url": POSTGRES_ADMIN_URL,
		"postgres_grader_url": POSTGRES_GRADER_URL,
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
	finally:
		os.close(secrets_descriptor)
	runtime = DatabaseBaselineRuntime(
		workspace=workspace,
		manifest_path=workspace / MANIFEST_NAME,
		compose_environment_path=workspace / COMPOSE_ENVIRONMENT,
		cleanup_capability_path=workspace / CLEANUP_CAPABILITY,
		admin_url_path=workspace / POSTGRES_ADMIN_URL,
		grader_url_path=workspace / POSTGRES_GRADER_URL,
		admin_password_path=workspace / POSTGRES_ADMIN_PASSWORD,
	)
	return runtime, grader_password


#============================================
def emit_grader_password_update(workspace: pathlib.Path) -> None:
	"""Write one fixed, validated grader-password update statement to standard output."""
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
		_runtime, grader_password = _validated_runtime_and_grader_password(
			workspace,
			workspace_descriptor,
			manifest,
		)
	finally:
		os.close(workspace_descriptor)
	print(f"ALTER ROLE {GRADER_USER} PASSWORD '{grader_password}';")


#============================================
def main(argv: list[str] | None = None) -> None:
	"""Provide the one pipe-only helper for the disposable grader password update."""
	arguments = sys.argv[1:] if argv is None else argv
	if len(arguments) != 2 or arguments[0] != "--emit-grader-password-update":
		print("usage: python3 -m local_stack_control.runtime_manifest --emit-grader-password-update WORKSPACE", file=sys.stderr)
		raise SystemExit(2)
	try:
		emit_grader_password_update(pathlib.Path(arguments[1]))
	except local_stack_control.models.ControllerError:
		print("acceptance runtime grader password update is unavailable", file=sys.stderr)
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
			capability = secrets.token_bytes(32)
			capability_digest = hashlib.sha256(capability).hexdigest()
			base = f"postgres://{POSTGRES_USER}:{urllib.parse.quote(postgres_password, safe='')}@127.0.0.1:{port}/{DATABASE_NAME}\n"
			grader = f"postgres://{GRADER_USER}:{urllib.parse.quote(grader_password, safe='')}@127.0.0.1:{port}/{DATABASE_NAME}\n"
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
				f"  postgres_admin_password: {POSTGRES_ADMIN_PASSWORD}\n"
			).encode("ascii")
			_write_private_file_at(secrets_descriptor, "compose.env", compose_environment)
			_write_private_file_at(secrets_descriptor, "cleanup.capability", capability)
			_write_private_file_at(secrets_descriptor, "postgres-admin.url", base.encode("ascii"))
			_write_private_file_at(secrets_descriptor, "postgres-grader.url", grader.encode("ascii"))
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


if __name__ == "__main__":
	main()

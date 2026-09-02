"""Environment-file ownership and child-environment sanitization."""

import os
import pathlib
import re
import stat

import local_stack_control.models


SETTING_PATTERN = re.compile(r"^[A-Za-z_][A-Za-z0-9_]*$")
TRACKED_STACK_SELECTION_NAMES = (
	"PLE_GATEWAY_IMAGE_SHA256",
	"PLE_POSTGRES_IMAGE_SHA256",
	"PLE_MINIO_IMAGE_SHA256",
	"PLE_MINIO_MC_IMAGE_SHA256",
	"PLE_SECRET_INIT_IMAGE_SHA256",
	"PLE_WEBWORK_RENDERER_IMAGE",
	"PLE_WEBWORK_RENDERER_BASE_URL",
	"PLE_WEBWORK_RENDERER_ID",
	"PLE_WEBWORK_REQUEST_TIMEOUT_SECONDS",
	"PLE_WEBWORK_MAX_RESPONSE_BYTES",
)
REMOTE_ENGINE_SELECTOR_NAMES = frozenset(
	{
		"CONTAINER_HOST",
		"CONTAINER_CONNECTION",
		"CONTAINER_SSHKEY",
		"CONTAINER_TLS_VERIFY",
		"CONTAINER_CERT_PATH",
		"PODMAN_HOST",
		"PODMAN_CONNECTION",
		"PODMAN_SSHKEY",
		"PODMAN_TLS_VERIFY",
		"PODMAN_CERT_PATH",
		"DOCKER_HOST",
		"DOCKER_CONTEXT",
		"DOCKER_TLS",
		"DOCKER_TLS_VERIFY",
		"DOCKER_CERT_PATH",
		"DOCKER_MACHINE_NAME",
	}
)


#============================================
def sanitized_runtime_environment(base_environment: dict[str, str]) -> dict[str, str]:
	"""Remove ambient selectors that could redirect Podman to another engine.

	PLE intentionally uses the caller's default Podman connection.  That is the
	rootless Podman machine on macOS, but it must not be overridden by shell
	variables that select a remote socket, SSH target, TLS client, or Docker
	context for an individual controller command.
	"""
	environment = dict(base_environment)
	for name in REMOTE_ENGINE_SELECTOR_NAMES:
		environment.pop(name, None)
	return environment


#============================================
def env_settings(env_file: pathlib.Path) -> dict[str, str]:
	"""Read one strict NAME=value file into its declared settings."""
	if not env_file.exists():
		raise local_stack_control.models.ControllerError(f"{env_file} does not exist")
	values: dict[str, str] = {}
	for line_number, line in enumerate(env_file.read_text(encoding="utf-8").splitlines(), start=1):
		stripped = line.strip()
		if stripped == "" or stripped.startswith("#"):
			continue
		if "=" not in line:
			raise local_stack_control.models.ControllerError(
				f"{env_file}:{line_number} is not a NAME=value declaration"
			)
		name, value = line.split("=", 1)
		if SETTING_PATTERN.fullmatch(name) is None:
			raise local_stack_control.models.ControllerError(
				f"{env_file}:{line_number} has an invalid setting name"
			)
		if name in values:
			raise local_stack_control.models.ControllerError(
				f"{env_file}:{line_number} duplicates {name}"
			)
		values[name] = value
	return values


#============================================
def env_setting_names(env_file: pathlib.Path, allow_missing: bool = False) -> tuple[str, ...]:
	"""Parse declared setting names without returning their values."""
	if not env_file.exists() and allow_missing:
		return ()
	values = env_settings(env_file)
	if "COMPOSE_PROJECT_NAME" in values:
		raise local_stack_control.models.ControllerError(
			f"COMPOSE_PROJECT_NAME must not be declared in {env_file}"
		)
	return tuple(values)


#============================================
def tracked_stack_selections(repo_root: pathlib.Path) -> dict[str, str]:
	"""Return the tracked image and renderer selections for disposable stacks."""
	values = env_settings(repo_root / "containers/env.example")
	result: dict[str, str] = {}
	for name in TRACKED_STACK_SELECTION_NAMES:
		if name not in values or values[name] == "":
			raise local_stack_control.models.ControllerError(
				f"containers/env.example must select {name}"
			)
		result[name] = values[name]
	return result


#============================================
def add_tracked_selections(
	repo_root: pathlib.Path,
	env_file: pathlib.Path,
	names: tuple[str, ...],
) -> tuple[str, ...]:
	"""Add named tracked selections to one private disposable environment.

	The tracked example owns image selection.  A disposable runner owns only its
	private credentials, ports, and cleanup capability, so it receives selected
	base-image values from that tracked source before Compose runs.
	"""
	values = env_settings(env_file)
	selections = tracked_stack_selections(repo_root)
	additions: list[str] = []
	for name in names:
		if name not in selections:
			raise local_stack_control.models.ControllerError(
				f"containers/env.example does not define tracked selection {name}"
			)
		if name in values:
			if values[name] != selections[name]:
				raise local_stack_control.models.ControllerError(
					f"{env_file} must use the tracked selection for {name}"
				)
			continue
		additions.append(f"{name}={selections[name]}")
	if len(additions) > 0:
		with env_file.open("a", encoding="utf-8") as handle:
			handle.write("".join(f"{line}\n" for line in additions))
	return env_setting_names(env_file)


#============================================
def mutation_env_file_errors(env_file: pathlib.Path) -> tuple[str, ...]:
	"""Return security errors for an existing mutating env file."""
	errors: list[str] = []
	if env_file.is_symlink():
		errors.append(f"{env_file} must not be a symbolic link")
		return tuple(errors)
	if not env_file.exists():
		errors.append(f"{env_file} does not exist")
		return tuple(errors)
	if not env_file.is_file():
		errors.append(f"{env_file} must be a regular file")
		return tuple(errors)

	file_stat = env_file.stat()
	if file_stat.st_uid != os.getuid():
		errors.append(f"{env_file} must be owned by the current user")
	mode = stat.S_IMODE(file_stat.st_mode)
	if mode != 0o600:
		errors.append(f"{env_file} must have mode 0600")
	if not os.access(env_file, os.R_OK):
		errors.append(f"{env_file} must be readable")
	return tuple(errors)


#============================================
def require_mutation_env_file(env_file: pathlib.Path) -> tuple[str, ...]:
	"""Validate a private env file and return its declared names."""
	errors = mutation_env_file_errors(env_file)
	if len(errors) > 0:
		detail = "; ".join(errors)
		raise local_stack_control.models.ControllerError(detail)
	result = env_setting_names(env_file)
	return result


#============================================
def sanitized_environment(
	base_environment: dict[str, str],
	env_names: tuple[str, ...],
	project: str,
) -> dict[str, str]:
	"""Give the selected env file and project authority over child processes."""
	environment = sanitized_runtime_environment(base_environment)
	# An absent default env file is a first-start condition, not permission for
	# an inherited PLE setting or Compose control to become configuration.  The
	# selected env file (when present) and this explicit project remain the only
	# lifecycle authority for every child process.
	for name in tuple(environment):
		if name.startswith("PLE_") or name.startswith("COMPOSE_"):
			environment.pop(name)
	for name in env_names:
		environment.pop(name, None)
	environment["COMPOSE_PROJECT_NAME"] = project
	return environment


#============================================
def sanitized_acceptance_environment(base_environment: dict[str, str]) -> dict[str, str]:
	"""Build one coherent child environment for aggregate acceptance.

	The aggregate acceptance runner owns the PLE stack configuration it creates.
	All ``PLE_*`` values are project-specific inputs (including future settings),
	as are Compose's ``COMPOSE_*`` controls.  Namespace boundaries are safer
	than a growing denylist.  The aggregate includes Playwright lanes whose
	workers select colored output, so its child environment carries that one
	color policy without a contradictory ``NO_COLOR`` request.  Ordinary
	operating system variables such as PATH, HOME, and locale stay available to
	the shell and browser tools.
	"""
	environment = sanitized_runtime_environment(base_environment)
	for name in tuple(environment):
		if name.startswith("PLE_") or name.startswith("COMPOSE_"):
			environment.pop(name)
	environment.pop("NO_COLOR", None)
	return environment

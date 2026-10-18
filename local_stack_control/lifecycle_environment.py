"""Private environment bootstrap and read-only lifecycle configuration validation."""

import os
import secrets
import pathlib

import local_stack_control.models
import local_stack_control.process
import local_stack_control.env_file
import local_stack_control.private_files
import local_stack_control.live_demo_seed
import local_stack_control.local_environment
import local_stack_control.lifecycle_commands
import local_stack_control.lifecycle_profiles
import local_stack_control.lifecycle_validation
import local_stack_control.local_totp_authenticator


LIVE_DEMO_PERSONA_SETTINGS = tuple(
	account.setting for account in local_stack_control.live_demo_seed.SEEDED_ACCOUNTS
)


#============================================
def bootstrap_default_state(
	target: local_stack_control.models.ComposeTarget | local_stack_control.models.DisposableComposeTarget,
	runner: local_stack_control.process.CommandRunner | None = None,
) -> None:
	"""Create only absent default-local state before selected-env validation."""
	selected = local_stack_control.lifecycle_profiles.target_of(target)
	if not local_stack_control.lifecycle_profiles.uses_local_teaching_state(target):
		if not selected.env_file.exists():
			raise local_stack_control.models.ControllerError(
				"a custom mutating env file must already exist and have mode 0600"
			)
		return
	if local_stack_control.lifecycle_profiles.is_default_target(selected):
		local_stack_control.local_environment.bootstrap_default_environment(
			selected.repo_root,
			selected.env_file,
			selected.repo_root / "containers/env.example",
		)
	# A new default is created mode 0600 before this point.  Every preexisting
	# selected teaching environment is rejected before parsing or replacement.
	local_stack_control.env_file.require_mutation_env_file(selected.env_file)
	configure_default_environment(selected, runner)
	runtime_directory = selected.env_file.parent
	secret_directory = runtime_directory / ".secrets"
	invitation_path = secret_directory / "invitation_token_secret"
	local_stack_control.local_environment.bootstrap_secret32_file(invitation_path)
	local_stack_control.local_totp_authenticator.bootstrap_local_totp_material(secret_directory)


#============================================
def configure_default_environment(
	target: local_stack_control.models.ComposeTarget,
	runner: local_stack_control.process.CommandRunner | None,
) -> None:
	"""Fill only missing or template default settings in the supported private environment."""
	local_stack_control.env_file.require_mutation_env_file(target.env_file)
	values = local_stack_control.env_file.env_settings(target.env_file)
	runtime_directory = target.env_file.parent
	secret_directory = runtime_directory / ".secrets"
	gateway_port = values.get("PLE_GATEWAY_HOST_PORT", "8080")
	# ASVS 11.5.1: private defaults retain at least 128 bits of CSPRNG entropy.
	defaults = {
		"POSTGRES_PASSWORD": os.urandom(24).hex(),
		"MINIO_ROOT_PASSWORD": os.urandom(24).hex(),
		"PLE_INVITATION_TOKEN_SECRET_HOST_FILE": str(secret_directory / "invitation_token_secret"),
		"PLE_LOCAL_SYSADMIN_TOTP_SEED_HOST_FILE": str(
			secret_directory / local_stack_control.local_totp_authenticator.MORGAN_TOTP_SEED_FILE
		),
		"PLE_LOCAL_SYSADMIN_TOTP_SEED_KEY_HOST_FILE": str(
			secret_directory / local_stack_control.local_totp_authenticator.MORGAN_TOTP_SEED_KEY_FILE
		),
		"PLE_LOCAL_SYSADMIN_TOTP_AUTHENTICATOR_ARTIFACT": str(
			secret_directory / local_stack_control.local_totp_authenticator.MORGAN_TOTP_ARTIFACT_FILE
		),
		"PLE_WEBWORK_RENDERER_VERSION_FILE": str(secret_directory / "question-renderer-version"),
		"PLE_PUBLIC_ASSET_BASE_URL": f"https://localhost:{gateway_port}/public-assets",
		"PLE_GATEWAY_HOST_PORT": "8080",
		"PLE_WEBWORK_RENDERER_BASE_URL": "http://webwork-renderer:3000/",
		"PLE_WEBWORK_REQUEST_TIMEOUT_SECONDS": "15",
		"PLE_WEBWORK_MAX_RESPONSE_BYTES": "1048576",
		"PLE_WEBWORK_PROBLEM_JWT_SECRET": os.urandom(32).hex(),
		"PLE_WEBWORK_SESSION_JWT_SECRET": os.urandom(32).hex(),
		"PLE_WEBWORK_RENDERER_ID": "vosslab-webwork-pg-renderer",
		"PLE_PUBLISHER_S3_ACCESS_KEY_ID": secrets.token_hex(16),
		"PLE_PUBLISHER_S3_SECRET_ACCESS_KEY": secrets.token_hex(32),
	}
	changed = False
	for name, value in defaults.items():
		if values.get(name, "") in ("", "change-me-before-first-run", "openwebwork-webwork2"):
			values[name] = value
			changed = True
	if runner is not None:
		gateway_port = choose_default_gateway_port(target, values, runner)
		if values.get("PLE_GATEWAY_HOST_PORT") != gateway_port:
			values["PLE_GATEWAY_HOST_PORT"] = gateway_port
			changed = True
	if changed:
		content = "".join(f"{name}={value}\n" for name, value in values.items()).encode("utf-8")
		# ASVS 13.3.2: replacement keeps private settings accessible only to their owner.
		local_stack_control.private_files.write_atomic_file(target.env_file, content, 0o600)


#============================================
def remove_live_demo_persona_configuration(
	target: local_stack_control.models.ComposeTarget,
) -> None:
	"""Keep opt-out and non-browser profiles free of the Demo persona selector."""
	local_stack_control.env_file.require_mutation_env_file(target.env_file)
	settings = local_stack_control.env_file.env_settings(target.env_file)
	if not any(name in settings for name in LIVE_DEMO_PERSONA_SETTINGS):
		return
	for name in LIVE_DEMO_PERSONA_SETTINGS:
		settings.pop(name, None)
	content = "".join(f"{name}={value}\n" for name, value in settings.items()).encode("utf-8")
	local_stack_control.private_files.write_atomic_file(target.env_file, content, 0o600)


#============================================
def choose_default_gateway_port(
	target: local_stack_control.models.ComposeTarget,
	values: dict[str, str],
	runner: local_stack_control.process.CommandRunner,
) -> str:
	"""Keep a running default gateway or choose the first free teaching port."""
	configured = values.get("PLE_GATEWAY_HOST_PORT", "8080")
	if not configured.isdecimal() or not 1 <= int(configured) <= 65535:
		raise local_stack_control.models.ControllerError("selected gateway port is invalid")
	if not port_is_listening(target, runner, configured):
		return configured
	if default_gateway_running(target, runner):
		return configured
	if target.project != local_stack_control.models.DEFAULT_PROJECT:
		raise local_stack_control.models.ControllerError(
			"the selected teaching gateway port is occupied"
		)
	for candidate in range(8000, 8100):
		candidate_text = str(candidate)
		if not port_is_listening(target, runner, candidate_text):
			return candidate_text
	raise local_stack_control.models.ControllerError("no local gateway port is available from 8000 through 8099")


#============================================
def port_is_listening(
	target: local_stack_control.models.ComposeTarget,
	runner: local_stack_control.process.CommandRunner,
	port: str,
) -> bool:
	"""Read one loopback listener state through the injected command boundary."""
	result = runner.run(
		["lsof", "-nP", f"-iTCP:{port}", "-sTCP:LISTEN", "-t"],
		local_stack_control.lifecycle_commands.child_environment(target),
		target.repo_root,
	)
	if result.returncode not in (0, 1):
		raise local_stack_control.models.ControllerError("cannot inspect local gateway port")
	return result.returncode == 0 and result.stdout.strip() != ""


#============================================
def default_gateway_running(
	target: local_stack_control.models.ComposeTarget,
	runner: local_stack_control.process.CommandRunner,
) -> bool:
	"""Recognize the selected default gateway without accepting an arbitrary listener."""
	result = runner.run(
		["podman", "ps", "--format", "{{.Names}}"],
		local_stack_control.lifecycle_commands.child_environment(target),
		target.repo_root,
	)
	if not result.ok():
		raise local_stack_control.models.ControllerError("cannot inspect the default gateway")
	return "containers_gateway_1" in result.stdout.splitlines()


#============================================
def validate_static(target: local_stack_control.models.ComposeTarget) -> dict[str, str]:
	"""Validate selected settings, secret file contracts, and Compose topology read-only."""
	request = local_stack_control.lifecycle_validation.LifecycleRequest(
		target=target, release=False, skip_build=False, headless=True, mutation=False
	)
	values = local_stack_control.lifecycle_validation.validate_request(request)
	required = (
		"POSTGRES_USER", "POSTGRES_PASSWORD", "POSTGRES_DB", "MINIO_ROOT_USER",
		"MINIO_ROOT_PASSWORD",
		"PLE_INVITATION_TOKEN_SECRET_HOST_FILE",
		"PLE_LOCAL_SYSADMIN_TOTP_SEED_HOST_FILE",
		"PLE_LOCAL_SYSADMIN_TOTP_SEED_KEY_HOST_FILE",
		"PLE_LOCAL_SYSADMIN_TOTP_AUTHENTICATOR_ARTIFACT",
		"PLE_WEBWORK_RENDERER_ID", "PLE_WEBWORK_PROBLEM_JWT_SECRET",
		"PLE_WEBWORK_SESSION_JWT_SECRET",
		"PLE_WEBWORK_RENDERER_VERSION_FILE",
	)
	require_values(values, required)
	for name in ("PLE_INVITATION_TOKEN_SECRET_HOST_FILE",):
		path = absolute_value_path(target.repo_root, values[name])
		local_stack_control.local_environment.read_secret32_file(path)
	local_stack_control.local_totp_authenticator.require_local_totp_material(
		absolute_value_path(target.repo_root, values["PLE_LOCAL_SYSADMIN_TOTP_SEED_HOST_FILE"]),
		absolute_value_path(target.repo_root, values["PLE_LOCAL_SYSADMIN_TOTP_SEED_KEY_HOST_FILE"]),
		absolute_value_path(target.repo_root, values["PLE_LOCAL_SYSADMIN_TOTP_AUTHENTICATOR_ARTIFACT"]),
	)
	return values


#============================================
def require_values(values: dict[str, str], names: tuple[str, ...]) -> None:
	"""Require nonempty selected configuration without returning private values."""
	for name in names:
		if name not in values or values[name] == "":
			raise local_stack_control.models.ControllerError(
				f"selected environment is missing required {name}"
			)


#============================================
def absolute_value_path(repo_root: pathlib.Path, value: str) -> pathlib.Path:
	"""Resolve an environment file path without accepting an empty value."""
	if value == "":
		raise local_stack_control.models.ControllerError("selected private file path is empty")
	path = pathlib.Path(value)
	result = path if path.is_absolute() else repo_root / path
	return result

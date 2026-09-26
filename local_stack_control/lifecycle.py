"""Typed local-stack start, restart, validation, and diagnostic orchestration."""

import dataclasses
import pathlib
import os
import collections.abc
import secrets

import local_stack_control.compose
import local_stack_control.disposable_stack_cleanup
import local_stack_control.discovery
import local_stack_control.lifecycle_provision
import local_stack_control.lifecycle_renderer
import local_stack_control.env_file
import local_stack_control.image_cleanup
import local_stack_control.lifecycle_validation
import local_stack_control.lifecycle_wait
import local_stack_control.local_totp_authenticator
import local_stack_control.lifecycle_diagnostics
import local_stack_control.lifecycle_commands
import local_stack_control.lifecycle_database
import local_stack_control.lifecycle_migrations
import local_stack_control.local_environment
import local_stack_control.lifecycle_profiles
import local_stack_control.models
import local_stack_control.private_files
import local_stack_control.process
import local_stack_control.process_logins
import local_stack_control.renderer
import local_stack_control.status
import local_stack_control.live_demo_gateway
import local_stack_control.live_demo_seed
import local_stack_control.lifecycle_browser
import local_stack_control.service_singletons


MIGRATION_DATABASE_OWNER = local_stack_control.lifecycle_database.MIGRATION_DATABASE_OWNER
MIGRATION_ROLE = local_stack_control.lifecycle_database.MIGRATION_ROLE
LIVE_DEMO_PERSONA_SETTINGS = tuple(
	account.setting for account in local_stack_control.live_demo_seed.SEEDED_ACCOUNTS
)


@dataclasses.dataclass(frozen=True)
class LifecycleOptions:
	"""Explicit lifecycle intent after the public CLI has parsed it once."""

	timeout_seconds: float
	build: bool
	release: bool
	open_browser: bool


@dataclasses.dataclass(frozen=True)
class LifecycleResult:
	"""Non-secret completed lifecycle summary."""

	project: str
	gateway_url: str
	renderer_oci_id: str


StatusRead = collections.abc.Callable[[], local_stack_control.models.StatusReport]
ReadinessPoll = collections.abc.Callable[[StatusRead, float], local_stack_control.models.StatusReport]


LifecycleTarget = (
	local_stack_control.models.ComposeTarget
	| local_stack_control.models.DisposableComposeTarget
)


# Lifecycle retains its established public facade while lifecycle_commands owns
# child execution, selected environments, and failure redaction.
child_environment = local_stack_control.lifecycle_commands.child_environment
compose_run = local_stack_control.lifecycle_commands.compose_run
report_step = local_stack_control.lifecycle_commands.report_step
require_command = local_stack_control.lifecycle_commands.require_command
validate_compose = local_stack_control.lifecycle_commands.validate_compose
renderer_readiness_report = local_stack_control.lifecycle_renderer.renderer_readiness_report
wait_for_renderer_ready = local_stack_control.lifecycle_renderer.wait_for_renderer_ready
attest_renderer = local_stack_control.lifecycle_renderer.attest_renderer
probe_renderer = local_stack_control.lifecycle_renderer.probe_renderer
require_attested_running_renderer = (
	local_stack_control.lifecycle_renderer.require_attested_running_renderer
)
require_running_renderer = local_stack_control.lifecycle_renderer.require_running_renderer
question_renderer_version_directory = (
	local_stack_control.lifecycle_renderer.question_renderer_version_directory
)
write_question_renderer_version = (
	local_stack_control.lifecycle_renderer.write_question_renderer_version
)
require_question_renderer_version = (
	local_stack_control.lifecycle_renderer.require_question_renderer_version
)
provision_ready_installation_data = (
	local_stack_control.lifecycle_provision.provision_ready_installation_data
)
record_live_demo_persona_account_ids = (
	local_stack_control.lifecycle_provision.record_live_demo_persona_account_ids
)
provision_local_sysadmin_totp = (
	local_stack_control.lifecycle_provision.provision_local_sysadmin_totp
)


#============================================
def target_of(
	target: local_stack_control.models.ComposeTarget | local_stack_control.models.DisposableComposeTarget,
) -> local_stack_control.models.ComposeTarget:
	"""Return the common target without changing its owner authority."""
	if isinstance(target, local_stack_control.models.DisposableComposeTarget):
		return target.target
	return target


#============================================
def bootstrap_default_state(
	target: local_stack_control.models.ComposeTarget | local_stack_control.models.DisposableComposeTarget,
	runner: local_stack_control.process.CommandRunner | None = None,
) -> None:
	"""Create only absent default-local state before selected-env validation."""
	selected = target_of(target)
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
	result = runner.run(["lsof", "-nP", f"-iTCP:{port}", "-sTCP:LISTEN", "-t"], child_environment(target), target.repo_root)
	if result.returncode not in (0, 1):
		raise local_stack_control.models.ControllerError("cannot inspect local gateway port")
	return result.returncode == 0 and result.stdout.strip() != ""


#============================================
def default_gateway_running(
	target: local_stack_control.models.ComposeTarget,
	runner: local_stack_control.process.CommandRunner,
) -> bool:
	"""Recognize the selected default gateway without accepting an arbitrary listener."""
	result = runner.run(["podman", "ps", "--format", "{{.Names}}"], child_environment(target), target.repo_root)
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


#============================================
def validate_lifecycle(
	target: local_stack_control.models.ComposeTarget | local_stack_control.models.DisposableComposeTarget,
	runner: local_stack_control.process.CommandRunner,
	repo_root: pathlib.Path,
) -> str:
	"""Perform read-only validation with no bootstrap, engine start, or Compose mutation."""
	selected = target_of(target)
	if selected.repo_root != repo_root:
		raise local_stack_control.models.ControllerError("lifecycle repository root does not match target")
	validate_static(selected)
	values = local_stack_control.env_file.env_settings(selected.env_file)
	local_stack_control.renderer.inspect_renderer_oci_id(
		runner, repo_root, values["PLE_WEBWORK_RENDERER_IMAGE"], child_environment(selected)
	)
	return validate_compose(selected, runner, repo_root)


#============================================
def start_lifecycle(
	target: local_stack_control.models.ComposeTarget | local_stack_control.models.DisposableComposeTarget,
	runner: local_stack_control.process.CommandRunner,
	repo_root: pathlib.Path,
	options: LifecycleOptions,
) -> LifecycleResult:
	"""Serialize cleanup, every image build, and container attachment as one cycle."""
	selected = target_of(target)
	require_lifecycle_inputs(selected, repo_root, options)
	require_disposable_ownership(target)
	bootstrap_default_state(target, runner)
	with local_stack_control.image_cleanup.image_build_lease(repo_root):
		return _start_lifecycle(target, runner, repo_root, options)


#============================================
def _start_lifecycle(
	target: local_stack_control.models.ComposeTarget | local_stack_control.models.DisposableComposeTarget,
	runner: local_stack_control.process.CommandRunner,
	repo_root: pathlib.Path,
	options: LifecycleOptions,
) -> LifecycleResult:
	"""Start one selected stack in durable dependency order without reparsing CLI input."""
	selected = target_of(target)
	local_stack_control.env_file.require_mutation_env_file(selected.env_file)
	values = validate_static(selected)
	local_stack_control.lifecycle_validation.require_mutation_engine(runner, repo_root, True)
	validate_compose(selected, runner, repo_root)
	local_stack_control.service_singletons.require_for_target(target, runner, repo_root)
	environment = child_environment(selected)
	build_artifacts(runner, repo_root, options)
	build_object_storage(runner, repo_root, environment)
	if options.build:
		# `api` owns the shared application image.  Rebuild it before any
		# application service starts so the selected stack cannot reuse a stale
		# tag after a Rust source change.
		compose_run(selected, runner, ["build", "api"])
	report_step("ensuring the WeBWorK renderer image (pulls on first use)")
	oci_id = local_stack_control.renderer.ensure_renderer_oci_id(
		runner, repo_root, values["PLE_WEBWORK_RENDERER_IMAGE"], environment, options.build
	)
	# Reconcile the complete selected project before starting dependency stages.
	# Podman Compose applies --remove-orphans to the services named by a partial
	# `up`; using it with the later --no-deps application subset can remove the
	# database and renderer that subset requires.  A full project down preserves
	# named volumes while removing both current containers and obsolete services.
	compose_run(selected, runner, ["down", "--remove-orphans"])
	compose_run(selected, runner, ["--profile", "maintenance", "run", "--rm", "--no-deps", "-T", "postgres-major-guard"])
	compose_run(selected, runner, ["up", "-d", "postgres"])
	report_step("waiting for PostgreSQL readiness")
	wait_for_postgres(selected, runner, values, options)
	initial_database_install = (
		local_stack_control.lifecycle_migrations.database_operation_for(target)
		== "initialize"
	)
	report_step("synchronizing the database baseline")
	synchronize_database(target, runner, values, options)
	report_step("running database migrations (builds the migrator image on first use)")
	run_migrations(target, runner, repo_root, values, environment)
	if local_stack_control.lifecycle_profiles.uses_local_teaching_state(target):
		report_step("setting up service logins and verifying the migrated schema")
		local_stack_control.process_logins.setup_service_logins(
			selected, runner, values, child_environment(selected)
		)
		verify_migrated_application_schema(selected, runner)
	compose_run(selected, runner, ["up", "-d", "minio", "createbuckets"])
	report_step("waiting for object storage buckets")
	wait_for_one_shot(selected, runner, options, "createbuckets")
	compose_run(selected, runner, ["up", "-d", "--force-recreate", "--no-deps", "webwork-renderer"])
	report_step("waiting for the WeBWorK renderer")
	wait_for_renderer_ready(selected, runner, options, oci_id)
	attest_renderer(selected, runner, repo_root, values, oci_id)
	report_step("running API initializers")
	run_api_initializers(selected, runner, options)
	provision_installation_data = should_provision_installation_data(
		target, initial_database_install
	)
	if not retains_live_demo_persona_configuration(target):
		remove_live_demo_persona_configuration(selected)
	compose_run(selected, runner, ["build", "gateway"])
	application_services = ["api", "worker", "public-asset-publisher", "gateway"]
	application_scale_arguments = local_stack_control.lifecycle_profiles.application_scale_arguments(
		target, tuple(application_services)
	)
	compose_run(
		selected,
		runner,
		[
			"up", "-d", "--force-recreate", "--no-deps",
			*application_scale_arguments,
			*application_services,
		],
	)
	report_step("waiting for the complete stack to report ready")
	gateway_url = wait_for_complete_ready(target, runner, options)
	if provision_installation_data:
		report_step("provisioning installation data (Live Demo Course)")
		provision_ready_installation_data(
			target, runner
		)
		if retains_live_demo_persona_configuration(target):
			report_step("recording minted Live Demo Account IDs")
			record_live_demo_persona_account_ids(target, runner)
			compose_run(
				selected,
				runner,
				["up", "-d", "--force-recreate", "--no-deps", "api"],
			)
			wait_for_complete_ready(target, runner, options)
	if retains_live_demo_persona_configuration(target):
		report_step("provisioning the local sysadmin TOTP authenticator")
		provision_local_sysadmin_totp(target, runner)
	if options.open_browser:
		local_stack_control.lifecycle_browser.open_browser(runner, repo_root, gateway_url)
	return LifecycleResult(selected.project, gateway_url, oci_id)


#============================================
def restart_lifecycle(
	target: (
		local_stack_control.models.ComposeTarget
		| local_stack_control.models.DisposableComposeTarget
	),
	runner: local_stack_control.process.CommandRunner,
	repo_root: pathlib.Path,
	service: str,
	options: LifecycleOptions,
) -> LifecycleResult:
	"""Recreate only a proven stateless service and reprove the selected stack."""
	selected = target_of(target)
	require_lifecycle_inputs(selected, repo_root, options)
	require_disposable_ownership(target)
	if service not in local_stack_control.models.restartable_services():
		raise local_stack_control.models.ControllerError("restart is limited to stateless services")
	if options.build or options.release or options.open_browser:
		raise local_stack_control.models.ControllerError("restart accepts no build, release, or browser-open intent")
	local_stack_control.env_file.require_mutation_env_file(selected.env_file)
	values = validate_static(selected)
	local_stack_control.lifecycle_validation.require_mutation_engine(runner, repo_root, True)
	require_restart_baseline(target, runner, service)
	oci_id = local_stack_control.renderer.inspect_renderer_oci_id(
		runner, repo_root, values["PLE_WEBWORK_RENDERER_IMAGE"], child_environment(selected)
	)
	if service == "webwork-renderer":
		require_question_renderer_version(selected, values, oci_id)
	elif service == "api":
		require_attested_running_renderer(selected, runner, values, oci_id)
		probe_renderer(selected, runner, repo_root, oci_id)
	if service == "api":
		run_api_initializers(selected, runner, options)
	arguments = local_stack_control.lifecycle_profiles.recreate_arguments(target, service)
	compose_run(selected, runner, arguments)
	if service == "webwork-renderer":
		wait_for_renderer_ready(selected, runner, options, oci_id)
		attest_renderer(selected, runner, repo_root, values, oci_id)
	gateway_url = wait_for_complete_ready(target, runner, options)
	return LifecycleResult(selected.project, gateway_url, oci_id)


APPLICATION_REBUILD_SERVICES = ("api", "worker", "public-asset-publisher")


#============================================
def rebuild_application_lifecycle(
	target: (
		local_stack_control.models.ComposeTarget
		| local_stack_control.models.DisposableComposeTarget
	),
	runner: local_stack_control.process.CommandRunner,
	repo_root: pathlib.Path,
	options: LifecycleOptions,
) -> LifecycleResult:
	"""Rebuild the shared application image and recreate its three services."""
	selected = target_of(target)
	require_lifecycle_inputs(selected, repo_root, options)
	require_disposable_ownership(target)
	if options.release or options.open_browser:
		raise local_stack_control.models.ControllerError(
			"rebuild-application accepts no release or browser-open intent"
		)
	local_stack_control.env_file.require_mutation_env_file(selected.env_file)
	values = validate_static(selected)
	local_stack_control.lifecycle_validation.require_mutation_engine(runner, repo_root, True)
	require_application_rebuild_baseline(target, runner)
	local_stack_control.service_singletons.require_for_target(target, runner, repo_root)
	oci_id = local_stack_control.renderer.inspect_renderer_oci_id(
		runner, repo_root, values["PLE_WEBWORK_RENDERER_IMAGE"], child_environment(selected)
	)
	require_attested_running_renderer(selected, runner, values, oci_id)
	probe_renderer(selected, runner, repo_root, oci_id)
	with local_stack_control.image_cleanup.image_build_lease(repo_root):
		compose_run(selected, runner, ["build", "api"])
		run_api_initializers(selected, runner, options)
		for service in APPLICATION_REBUILD_SERVICES:
			arguments = local_stack_control.lifecycle_profiles.recreate_arguments(target, service)
			compose_run(selected, runner, arguments)
	gateway_url = wait_for_complete_ready(target, runner, options)
	return LifecycleResult(selected.project, gateway_url, oci_id)


#============================================
def require_application_rebuild_baseline(
	target: LifecycleTarget,
	runner: local_stack_control.process.CommandRunner,
) -> None:
	"""Require stateful and gateway services healthy before replacing the application group."""
	report = status_report(target, runner)
	rebuild = set(APPLICATION_REBUILD_SERVICES)
	for item in report.services:
		if item.service in rebuild:
			continue
		if not item.healthy:
			raise local_stack_control.models.ControllerError(
				f"required service {item.service} is not ready: "
				f"state={item.state} health={item.health or '-'} "
				f"instances={item.instances} exit_code={item.exit_code}; "
				"inspect local_stack.py status or restart the disposable Live Demo "
				"with ./devel/capture_screenshots.sh --fresh"
			)
	for service in APPLICATION_REBUILD_SERVICES:
		matching = tuple(item for item in report.services if item.service == service)
		if len(matching) != 1 or matching[0].state == "ambiguous":
			raise local_stack_control.models.ControllerError(
				"selected application service is absent or has unexpected instance cardinality"
			)


#============================================
def require_lifecycle_inputs(target: local_stack_control.models.ComposeTarget, repo_root: pathlib.Path, options: LifecycleOptions) -> None:
	"""Reject mismatched targets and nonpositive caller-owned timeout before effects."""
	if target.repo_root != repo_root or options.timeout_seconds <= 0:
		raise local_stack_control.models.ControllerError("lifecycle target or timeout is invalid")


#============================================
def require_disposable_ownership(target: local_stack_control.models.ComposeTarget | local_stack_control.models.DisposableComposeTarget) -> None:
	"""Retain the explicit capability proof when the selected target is disposable."""
	if isinstance(target, local_stack_control.models.DisposableComposeTarget):
		local_stack_control.compose.require_disposable_ownership(target)


#============================================
def build_object_storage(
	runner: local_stack_control.process.CommandRunner,
	repo_root: pathlib.Path,
	environment: dict[str, str],
) -> None:
	"""Stream the cached source build without passing private Compose settings."""
	# ASVS 15.2.4: the Containerfile uses official sources and base images.
	report_step("Building MinIO image (object storage server and client)")
	status = runner.stream(
		["podman", "build", "-f", str(repo_root / "containers" / "Containerfile.object_storage"),
			"-t", "localhost/ple-object-storage:reviewed", str(repo_root / "containers")],
		environment,
		repo_root,
	)
	if status != 0:
		raise local_stack_control.models.ControllerError("object storage source build failed; see the build log")


#============================================
def build_artifacts(runner: local_stack_control.process.CommandRunner, repo_root: pathlib.Path, options: LifecycleOptions) -> None:
	"""Build the production browser artifact or require its complete reusable bundle."""
	if not options.build:
		if not (repo_root / "dist/index.html").is_file() or not (repo_root / "dist/main.js").is_file():
			raise local_stack_control.models.ControllerError("reuse build requires a complete dist bundle")
		return
	profile = "--release" if options.release else "--debug"
	environment = local_stack_control.env_file.sanitized_runtime_environment(
		local_stack_control.process.current_environment()
	)
	report_step("host build ./build.sh " + profile + " (Rust and browser bundle; slow when cold)")
	result = runner.run(["./build.sh", profile], environment, repo_root)
	require_command(result, "host artifact build")


#============================================
def wait_for_one_shot(target: local_stack_control.models.ComposeTarget, runner: local_stack_control.process.CommandRunner, options: LifecycleOptions, service: str) -> None:
	"""Await one required completed service through label-derived status."""
	def read_report() -> local_stack_control.models.StatusReport:
		report = status_report(target, runner)
		matching = tuple(item for item in report.services if item.service == service)
		if len(matching) > 1:
			raise local_stack_control.models.ControllerError(
				f"required one-shot {service} has duplicate instances"
			)
		if len(matching) == 0:
			return dataclasses.replace(
				report, ok=False, state="starting", message=f"one-shot {service} is pending"
			)
		one_shot = matching[0]
		if one_shot.state == "exited" and one_shot.exit_code not in (None, 0):
			raise local_stack_control.models.ControllerError(
				f"required one-shot {service} failed; retained stack resources are available for diagnostics"
			)
		complete = one_shot.complete
		return dataclasses.replace(
			report,
			ok=complete,
			state="ready" if complete else "starting",
			message=f"one-shot {service} is {'complete' if complete else 'running'}",
		)
	report = local_stack_control.lifecycle_wait.poll_ready(read_report, options.timeout_seconds)
	if not any(item.service == service and item.complete for item in report.services):
		raise local_stack_control.models.ControllerError("required one-shot did not complete successfully")


#============================================
def wait_for_postgres(target: local_stack_control.models.ComposeTarget, runner: local_stack_control.process.CommandRunner, values: dict[str, str], options: LifecycleOptions) -> None:
	"""Await PostgreSQL readiness without placing credentials in argv."""
	argv = local_stack_control.compose.compose_argv(
		target, ["exec", "-T", "postgres", "pg_isready", "-U", values["POSTGRES_USER"], "-d", values["POSTGRES_DB"]]
	)
	last_result: local_stack_control.models.CommandResult | None = None
	def read_report() -> local_stack_control.models.StatusReport:
		nonlocal last_result
		result = runner.run(argv, child_environment(target), target.repo_root)
		last_result = result
		if result.ok():
			return ready_report(target)
		return unavailable_report(target)
	try:
		local_stack_control.lifecycle_wait.poll_ready(read_report, options.timeout_seconds)
	except local_stack_control.models.ControllerError as error:
		if last_result is None:
			raise
		detail = local_stack_control.lifecycle_diagnostics.redacted_failure_detail(
			last_result, tuple(values.values())
		)
		if last_result.stdout == "" and last_result.stderr == "":
			log_argv = local_stack_control.compose.compose_argv(
				target, ["logs", "--no-color", "--tail", "20", "postgres"]
			)
			log_result = runner.run(
				log_argv, child_environment(target), target.repo_root
			)
			detail = local_stack_control.lifecycle_diagnostics.redacted_postgres_service_detail(
				log_result, tuple(values.values())
			)
		raise local_stack_control.models.ControllerError(
			f"{error}; PostgreSQL readiness detail: {detail}"
		) from error


#============================================
def synchronize_database(
	target: local_stack_control.models.ComposeTarget
	| local_stack_control.models.DisposableComposeTarget,
	runner: local_stack_control.process.CommandRunner,
	values: dict[str, str],
	options: LifecycleOptions,
	*,
	poll_ready: ReadinessPoll = local_stack_control.lifecycle_wait.poll_ready,
) -> None:
	"""Synchronize the local login across PostgreSQL's bounded startup handoff."""
	if not local_stack_control.lifecycle_profiles.uses_local_teaching_state(target):
		return
	selected = target_of(target)
	password = values["POSTGRES_PASSWORD"]
	environment = child_environment(selected)
	environment["PGPASSWORD"] = password
	argv = local_stack_control.compose.compose_argv(selected, ["exec", "-T", "postgres", "psql", "-v", "ON_ERROR_STOP=1", "-U", values["POSTGRES_USER"], "-d", values["POSTGRES_DB"]])
	sql = postgres_role_sql(values["POSTGRES_USER"], password)
	private_values = local_stack_control.disposable_stack_cleanup.private_environment_values(
		selected.env_file
	)
	def read_report() -> local_stack_control.models.StatusReport:
		result = runner.run(argv, environment, selected.repo_root, sql)
		if result.ok():
			return ready_report(selected)
		detail = local_stack_control.lifecycle_diagnostics.redacted_failure_detail(
			result, private_values
		)
		return dataclasses.replace(
			unavailable_report(selected),
			message=f"database login synchronization is pending ({detail})",
		)
	poll_ready(read_report, options.timeout_seconds)


#============================================
run_migrations = local_stack_control.lifecycle_migrations.run_migrations


#============================================
verify_migrated_application_schema = (
	local_stack_control.lifecycle_migrations.verify_migrated_application_schema
)


#============================================
migration_database_url_for = local_stack_control.lifecycle_migrations.migration_database_url_for


#============================================
migration_principal_bootstrap_sql = (
	local_stack_control.lifecycle_migrations.migration_principal_bootstrap_sql
)




#============================================
postgres_role_sql = local_stack_control.lifecycle_migrations.postgres_role_sql


#============================================
database_url = local_stack_control.lifecycle_migrations.database_url


#============================================
def run_api_initializers(target: local_stack_control.models.ComposeTarget, runner: local_stack_control.process.CommandRunner, options: LifecycleOptions) -> None:
	"""Refresh API-owned initializers before recreating API-owned stateless services."""
	for service in ("identity-secret-init",):
		compose_run(target, runner, ["up", "-d", "--force-recreate", "--no-deps", service])
		wait_for_one_shot(target, runner, options, service)


#============================================
def should_provision_installation_data(
	target: LifecycleTarget,
	initial_database_install: bool,
) -> bool:
	"""Decide whether this initial local teaching install needs bundled content."""
	return (
		initial_database_install
		and local_stack_control.lifecycle_profiles.uses_local_teaching_state(target)
	)


#============================================
def retains_live_demo_persona_configuration(
	target: LifecycleTarget,
) -> bool:
	"""Keep the closed selector only for a default or browser-profile Live Demo."""
	return (
		local_stack_control.lifecycle_profiles.is_default_target(target_of(target))
		or (
			isinstance(target, local_stack_control.models.DisposableComposeTarget)
			and target.owner_policy == local_stack_control.models.LIVE_DEMO_BROWSER_OWNER
			and target.live_demo_profile is local_stack_control.models.LiveDemoProfile.BROWSER
		)
	)


#============================================
#============================================
def status_report(
	target: LifecycleTarget,
	runner: local_stack_control.process.CommandRunner,
) -> local_stack_control.models.StatusReport:
	"""Discover the selected target through existing label-derived ownership logic."""
	selected = target_of(target)
	snapshot = local_stack_control.discovery.discover_snapshot(
		runner, selected.repo_root, selected.project
	)
	return local_stack_control.status.build_target_report(target, snapshot)


#============================================
def ready_report(target: local_stack_control.models.ComposeTarget) -> local_stack_control.models.StatusReport:
	"""Build a minimal ready sentinel for a direct PostgreSQL command poll."""
	return dataclasses.replace(unavailable_report(target), ok=True, state="ready", message="PostgreSQL is ready")


#============================================
def unavailable_report(
	target: local_stack_control.models.ComposeTarget,
	message: str = "PostgreSQL is starting",
) -> local_stack_control.models.StatusReport:
	"""Build a minimal retry sentinel for one direct readiness probe."""
	snapshot = local_stack_control.models.ProjectSnapshot(target.project, (), (), ())
	return local_stack_control.models.StatusReport(target.project, target.with_smtp, snapshot, (), False, "starting", message)


#============================================
def require_complete_ready(
	target: LifecycleTarget,
	runner: local_stack_control.process.CommandRunner,
) -> None:
	"""Require one already-ready full stack before a stateless restart."""
	local_stack_control.lifecycle_wait.require_ready(status_report(target, runner))
	selected = target_of(target)
	local_stack_control.service_singletons.require_for_target(target, runner, selected.repo_root)


#============================================
def require_restart_baseline(
	target: LifecycleTarget,
	runner: local_stack_control.process.CommandRunner,
	selected_service: str,
) -> None:
	"""Require every non-selected service healthy while permitting one recoverable outage."""
	require_restart_report(status_report(target, runner), selected_service)


#============================================
def require_restart_report(
	report: local_stack_control.models.StatusReport,
	selected_service: str,
) -> None:
	"""Validate a restart baseline without treating the selected service as healthy."""
	matching = tuple(item for item in report.services if item.service == selected_service)
	if len(matching) != 1 or matching[0].state == "ambiguous":
		raise local_stack_control.models.ControllerError(
			"selected restart service is absent or has unexpected instance cardinality"
		)
	for item in report.services:
		if item.service != selected_service and not item.healthy:
			raise local_stack_control.models.ControllerError(
				"a non-selected required service is not healthy"
			)


#============================================
def wait_for_complete_ready(
	target: LifecycleTarget,
	runner: local_stack_control.process.CommandRunner,
	options: LifecycleOptions,
) -> str:
	"""Require loopback gateway health and complete label-derived semantic readiness."""
	selected = target_of(target)
	url = local_stack_control.live_demo_gateway.gateway_url(selected)
	def read_report() -> local_stack_control.models.StatusReport:
		result = runner.run(
			local_stack_control.live_demo_gateway.health_probe_argv(url),
			child_environment(selected),
			selected.repo_root,
		)
		if result.ok():
			return status_report(target, runner)
		# Name the probe that failed; the gateway fronts every service, so a silent
		# gateway is the usual final symptom of an api, worker, or TLS problem behind it.
		probe_detail = result.stderr.strip().splitlines()
		detail = probe_detail[-1] if probe_detail else "no response"
		return unavailable_report(selected, f"gateway health probe at {url} failed: {detail}")
	local_stack_control.lifecycle_wait.poll_ready(read_report, options.timeout_seconds)
	require_complete_ready(target, runner)
	return url

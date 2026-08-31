"""Typed local-stack start, restart, validation, and diagnostic orchestration."""

import dataclasses
import pathlib
import os
import collections.abc

import local_stack_control.compose
import local_stack_control.disposable_stack_adapter
import local_stack_control.discovery
import local_stack_control.env_file
import local_stack_control.image_cleanup
import local_stack_control.lifecycle_validation
import local_stack_control.lifecycle_wait
import local_stack_control.lifecycle_diagnostics
import local_stack_control.lifecycle_commands
import local_stack_control.local_environment
import local_stack_control.lifecycle_profiles
import local_stack_control.models
import local_stack_control.private_files
import local_stack_control.process
import local_stack_control.process_logins
import local_stack_control.renderer
import local_stack_control.status
import local_stack_control.live_demo_gateway


LOCAL_INSTRUCTOR_ID = "00000000-0000-0000-0000-000000000101"
LOCAL_MARY_ID = "00000000-0000-0000-0000-000000000102"
LOCAL_JACK_ID = "00000000-0000-0000-0000-000000000103"
LOCAL_APPROVAL_CANDIDATE_ID = "00000000-0000-0000-0000-000000000104"
LOCAL_SYSADMIN_ID = "00000000-0000-0000-0000-000000000105"
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
require_command = local_stack_control.lifecycle_commands.require_command
validate_compose = local_stack_control.lifecycle_commands.validate_compose


#============================================
def target_of(
	target: local_stack_control.models.ComposeTarget | local_stack_control.models.DisposableComposeTarget,
) -> local_stack_control.models.ComposeTarget:
	"""Return the common target without changing its owner authority."""
	if isinstance(target, local_stack_control.models.DisposableComposeTarget):
		return target.target
	return target


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
	question_path = secret_directory / "question_id_secret"
	local_stack_control.local_environment.bootstrap_secret32_file(invitation_path)
	local_stack_control.local_environment.bootstrap_secret32_file(question_path)


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
	defaults = {
		"POSTGRES_PASSWORD": os.urandom(24).hex(),
		"MINIO_ROOT_PASSWORD": os.urandom(24).hex(),
		"PLE_LOCAL_AUTOMATED_GRADING_PASSWORD": os.urandom(24).hex(),
		"PLE_INVITATION_TOKEN_SECRET_HOST_FILE": str(secret_directory / "invitation_token_secret"),
		"PLE_QUESTION_ID_SECRET_HOST_FILE": str(secret_directory / "question_id_secret"),
		"PLE_WEBWORK_RENDERER_VERSION_FILE": str(secret_directory / "question-renderer-version"),
		"PLE_PUBLIC_ASSET_BASE_URL": "http://127.0.0.1:9000/public-assets",
		"PLE_WEBAUTHN_RP_ID": "localhost",
		"PLE_WEBAUTHN_RP_NAME": "Peptidyle Learning Engine",
		"PLE_GATEWAY_HOST_PORT": "8080",
		"PLE_WEBAUTHN_ORIGIN": "http://localhost:8080",
		"PLE_WEBWORK_RENDERER_BASE_URL": "http://webwork-renderer:3000/",
		"PLE_WEBWORK_REQUEST_TIMEOUT_SECONDS": "15",
		"PLE_WEBWORK_MAX_RESPONSE_BYTES": "1048576",
		"PLE_WEBWORK_PROBLEM_JWT_SECRET": os.urandom(32).hex(),
		"PLE_WEBWORK_SESSION_JWT_SECRET": os.urandom(32).hex(),
		"PLE_WEBWORK_RENDERER_ID": "vosslab-webwork-pg-renderer",
	}
	if local_stack_control.live_demo_gateway.is_tls_target(target):
		defaults.update({
			"PLE_LIVE_DEMO_ELENA_INSTRUCTOR_USER_ID": LOCAL_INSTRUCTOR_ID,
			"PLE_LIVE_DEMO_MARY_STUDENT_USER_ID": LOCAL_MARY_ID,
			"PLE_LIVE_DEMO_JACK_STUDENT_USER_ID": LOCAL_JACK_ID,
			"PLE_LIVE_DEMO_AVERY_STUDENT_USER_ID": LOCAL_APPROVAL_CANDIDATE_ID,
			"PLE_LIVE_DEMO_SYSADMIN_USER_ID": LOCAL_SYSADMIN_ID,
		})
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
		origin = f"http://localhost:{gateway_port}"
		if values.get("PLE_WEBAUTHN_ORIGIN", "") in ("", "http://localhost:8080"):
			values["PLE_WEBAUTHN_ORIGIN"] = origin
			changed = True
	if changed:
		content = "".join(f"{name}={value}\n" for name, value in values.items()).encode("utf-8")
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
		"MINIO_ROOT_PASSWORD", "PLE_LOCAL_AUTOMATED_GRADING_PASSWORD",
		"PLE_INVITATION_TOKEN_SECRET_HOST_FILE", "PLE_QUESTION_ID_SECRET_HOST_FILE",
		"PLE_WEBWORK_RENDERER_ID", "PLE_WEBWORK_PROBLEM_JWT_SECRET",
		"PLE_WEBWORK_SESSION_JWT_SECRET",
		"PLE_WEBWORK_RENDERER_VERSION_FILE",
	)
	if local_stack_control.live_demo_gateway.is_tls_target(target):
		required = required + (
			"PLE_LIVE_DEMO_ELENA_INSTRUCTOR_USER_ID", "PLE_LIVE_DEMO_MARY_STUDENT_USER_ID",
			"PLE_LIVE_DEMO_JACK_STUDENT_USER_ID", "PLE_LIVE_DEMO_AVERY_STUDENT_USER_ID",
			"PLE_LIVE_DEMO_SYSADMIN_USER_ID",
		)
	require_values(values, required)
	for name in (
		"PLE_POSTGRES_IMAGE_SHA256", "PLE_MINIO_IMAGE_SHA256", "PLE_MINIO_MC_IMAGE_SHA256",
		"PLE_GATEWAY_IMAGE_SHA256", "PLE_SECRET_INIT_IMAGE_SHA256",
	):
		require_digest(values, name)
	for name in ("PLE_INVITATION_TOKEN_SECRET_HOST_FILE", "PLE_QUESTION_ID_SECRET_HOST_FILE"):
		path = absolute_value_path(target.repo_root, values[name])
		local_stack_control.local_environment.read_secret32_file(path)
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
def require_digest(values: dict[str, str], name: str) -> None:
	"""Require one lower-case 64-character image manifest digest."""
	value = values.get(name, "")
	if len(value) != 64 or any(character not in "0123456789abcdef" for character in value):
		raise local_stack_control.models.ControllerError(f"selected environment has invalid {name}")


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
	local_stack_control.process.require_rootless_local_engine(runner, repo_root)
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
	"""Start one selected stack in durable dependency order without reparsing CLI input."""
	selected = target_of(target)
	require_lifecycle_inputs(selected, repo_root, options)
	require_disposable_ownership(target)
	bootstrap_default_state(target, runner)
	local_stack_control.env_file.require_mutation_env_file(selected.env_file)
	values = validate_static(selected)
	local_stack_control.lifecycle_validation.require_mutation_engine(runner, repo_root, True)
	validate_compose(selected, runner, repo_root)
	environment = child_environment(selected)
	build_artifacts(runner, repo_root, options)
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
	wait_for_postgres(selected, runner, values, options)
	synchronize_database(target, runner, values, options)
	run_migrations(runner, repo_root, values, environment)
	if local_stack_control.lifecycle_profiles.uses_local_teaching_state(target):
		local_stack_control.process_logins.setup_service_logins(
			selected, runner, values, child_environment(selected)
		)
	compose_run(selected, runner, ["up", "-d", "minio", "createbuckets"])
	wait_for_one_shot(selected, runner, options, "createbuckets")
	compose_run(selected, runner, ["up", "-d", "--force-recreate", "--no-deps", "webwork-renderer"])
	wait_for_renderer_ready(selected, runner, options, oci_id)
	attest_renderer(selected, runner, repo_root, values, oci_id)
	run_api_initializers(selected, runner, options)
	compose_run(selected, runner, ["build", "api", "gateway"])
	application_services = ["api", "gateway"]
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
	gateway_url = wait_for_complete_ready(target, runner, options)
	if local_stack_control.lifecycle_profiles.is_default_target(selected):
		local_stack_control.image_cleanup.prune_superseded_images(runner, repo_root)
	if options.open_browser:
		open_browser(runner, repo_root, gateway_url)
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
	else:
		require_attested_running_renderer(selected, runner, values, oci_id)
		if service == "api":
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
	def read_report() -> local_stack_control.models.StatusReport:
		result = runner.run(argv, child_environment(target), target.repo_root)
		if result.ok():
			return ready_report(target)
		return unavailable_report(target)
	local_stack_control.lifecycle_wait.poll_ready(read_report, options.timeout_seconds)


#============================================
def renderer_readiness_report(
	report: local_stack_control.models.StatusReport,
	oci_id: str,
) -> local_stack_control.models.StatusReport:
	"""Classify only the selected renderer before API or gateway recreation."""
	containers = tuple(
		item for item in report.snapshot.containers if item.service == "webwork-renderer"
	)
	if len(containers) != 1:
		raise local_stack_control.models.ControllerError(
			"renderer service is missing or ambiguous"
		)
	container = containers[0]
	if container.state == "exited":
		raise local_stack_control.models.ControllerError(
			"renderer exited before readiness; retained stack resources are available for diagnostics"
		)
	if container.image_id != oci_id:
		raise local_stack_control.models.ControllerError(
			"running renderer does not match the selected OCI configuration"
		)
	if not container.running or container.health != "healthy":
		return dataclasses.replace(
			report,
			ok=False,
			state="starting",
			message="renderer is starting",
		)
	local_stack_control.renderer.require_running_renderer(report, oci_id)
	return dataclasses.replace(report, ok=True, state="ready", message="renderer is ready")


#============================================
def wait_for_renderer_ready(
	target: local_stack_control.models.ComposeTarget,
	runner: local_stack_control.process.CommandRunner,
	options: LifecycleOptions,
	oci_id: str,
	*,
	read_status: StatusRead | None = None,
	poll_ready: ReadinessPoll = local_stack_control.lifecycle_wait.poll_ready,
) -> None:
	"""Await the one selected healthy renderer before its behavior is probed."""
	status_reader = read_status
	if status_reader is None:
		def status_reader() -> local_stack_control.models.StatusReport:
			return status_report(target, runner)

	def read_report() -> local_stack_control.models.StatusReport:
		report = status_reader()
		return renderer_readiness_report(report, oci_id)

	poll_ready(read_report, options.timeout_seconds)


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
	private_values = local_stack_control.disposable_stack_adapter.private_environment_values(
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
def run_migrations(runner: local_stack_control.process.CommandRunner, repo_root: pathlib.Path, values: dict[str, str], environment: dict[str, str]) -> None:
	"""Run migrations with the database URL only in the direct child environment."""
	child = dict(environment)
	migration_database_url = database_url(values)
	child["PLE_MIGRATION_DATABASE_URL"] = migration_database_url
	require_command(
		runner.run(["cargo", "tools", "database", "migrate"], child, repo_root),
		"database migration",
		(migration_database_url,),
	)




#============================================
def postgres_role_sql(role: str, password: str) -> str:
	"""Build a closed local-role command delivered over stdin, never argv or diagnostics."""
	if not role.replace("_", "").isalnum() or not password.replace("-", "").replace("_", "").isalnum():
		raise local_stack_control.models.ControllerError("local PostgreSQL role settings are invalid")
	result = f"ALTER ROLE {role} PASSWORD '{password}';\n"
	return result


#============================================
def database_url(values: dict[str, str]) -> str:
	"""Construct a child-only local PostgreSQL URL from selected private values."""
	port = values.get("PLE_POSTGRES_HOST_PORT", "5432")
	if not port.isdecimal():
		raise local_stack_control.models.ControllerError("selected PostgreSQL port is invalid")
	result = f"postgres://{values['POSTGRES_USER']}:{values['POSTGRES_PASSWORD']}@127.0.0.1:{port}/{values['POSTGRES_DB']}"
	return result


#============================================
def attest_renderer(target: local_stack_control.models.ComposeTarget, runner: local_stack_control.process.CommandRunner, repo_root: pathlib.Path, values: dict[str, str], oci_id: str) -> None:
	"""Prove renderer identity and behavior before replacing its private attestation."""
	require_running_renderer(target, runner, oci_id)
	probe_renderer(target, runner, repo_root, oci_id)
	write_question_renderer_version(target, values, oci_id)


#============================================
def probe_renderer(target: local_stack_control.models.ComposeTarget, runner: local_stack_control.process.CommandRunner, repo_root: pathlib.Path, oci_id: str) -> None:
	"""Exercise the exact selected renderer through its label-resolved container."""
	container = local_stack_control.renderer.require_running_renderer(status_report(target, runner), oci_id)
	probe = (repo_root / "containers/webwork/probe_render_api.sh").read_text(encoding="utf-8")
	result = runner.run(["podman", "exec", "-i", container.id, "bash", "-s", "--", "--exercise"], {name: value for name, value in child_environment(target).items() if name in ("PATH", "HOME")}, repo_root, probe)
	require_command(result, "renderer render and grade probe")


#============================================
def require_attested_running_renderer(target: local_stack_control.models.ComposeTarget, runner: local_stack_control.process.CommandRunner, values: dict[str, str], oci_id: str) -> None:
	"""Require the running renderer and its private preexisting OCI attestation."""
	require_running_renderer(target, runner, oci_id)
	require_question_renderer_version(target, values, oci_id)


#============================================
def require_running_renderer(target: local_stack_control.models.ComposeTarget, runner: local_stack_control.process.CommandRunner, oci_id: str) -> None:
	"""Require the single selected renderer to be healthy and image-matched."""
	local_stack_control.renderer.require_running_renderer(status_report(target, runner), oci_id)


#============================================
def question_renderer_version_directory(target: local_stack_control.models.ComposeTarget, values: dict[str, str]) -> pathlib.Path:
	"""Resolve the fixed private Question Renderer Version directory."""
	version_path = absolute_value_path(target.repo_root, values["PLE_WEBWORK_RENDERER_VERSION_FILE"])
	if version_path.name != local_stack_control.renderer.QUESTION_RENDERER_VERSION_NAME:
		raise local_stack_control.models.ControllerError("selected Question Renderer Version path has an invalid name")
	return version_path.parent


#============================================
def write_question_renderer_version(target: local_stack_control.models.ComposeTarget, values: dict[str, str], oci_id: str) -> None:
	"""Record the exact Question Renderer Version after a successful probe."""
	version = local_stack_control.models.QuestionRendererVersion(values["PLE_WEBWORK_RENDERER_IMAGE"], oci_id)
	local_stack_control.renderer.write_question_renderer_version(question_renderer_version_directory(target, values), version)


#============================================
def require_question_renderer_version(target: local_stack_control.models.ComposeTarget, values: dict[str, str], oci_id: str) -> None:
	"""Require the current Question Renderer Version before renderer recovery."""
	local_stack_control.renderer.require_question_renderer_version(
		question_renderer_version_directory(target, values), oci_id
	)


#============================================
#============================================
def run_api_initializers(target: local_stack_control.models.ComposeTarget, runner: local_stack_control.process.CommandRunner, options: LifecycleOptions) -> None:
	"""Refresh API-owned initializers before recreating API-owned stateless services."""
	for service in ("identity-secret-init",):
		compose_run(target, runner, ["up", "-d", "--force-recreate", "--no-deps", service])
		wait_for_one_shot(target, runner, options, service)


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
def unavailable_report(target: local_stack_control.models.ComposeTarget) -> local_stack_control.models.StatusReport:
	"""Build a minimal retry sentinel for a direct PostgreSQL command poll."""
	snapshot = local_stack_control.models.ProjectSnapshot(target.project, (), (), ())
	return local_stack_control.models.StatusReport(target.project, target.with_smtp, snapshot, (), False, "starting", "PostgreSQL is starting")


#============================================
def require_complete_ready(
	target: LifecycleTarget,
	runner: local_stack_control.process.CommandRunner,
) -> None:
	"""Require one already-ready full stack before a stateless restart."""
	local_stack_control.lifecycle_wait.require_ready(status_report(target, runner))


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
		return unavailable_report(selected)
	local_stack_control.lifecycle_wait.poll_ready(read_report, options.timeout_seconds)
	require_complete_ready(target, runner)
	return url


#============================================
#============================================
def open_browser(runner: local_stack_control.process.CommandRunner, repo_root: pathlib.Path, url: str) -> None:
	"""Open the proven loopback URL using an argument-array platform opener."""
	result = runner.run(["open", url], local_stack_control.env_file.sanitized_runtime_environment(local_stack_control.process.current_environment()), repo_root)
	if not result.ok():
		result = runner.run(["xdg-open", url], local_stack_control.env_file.sanitized_runtime_environment(local_stack_control.process.current_environment()), repo_root)
		require_command(result, "browser open")

"""Typed local-stack start, restart, validation, and diagnostic orchestration."""

import dataclasses
import pathlib
import os
import collections.abc

import local_stack_control.compose
import local_stack_control.discovery
import local_stack_control.env_file
import local_stack_control.image_cleanup
import local_stack_control.lifecycle_validation
import local_stack_control.lifecycle_wait
import local_stack_control.lifecycle_diagnostics
import local_stack_control.local_environment
import local_stack_control.local_identity
import local_stack_control.models
import local_stack_control.private_files
import local_stack_control.process
import local_stack_control.renderer
import local_stack_control.status
import local_stack_control.chapter_one
import local_stack_control.base_course_lifecycle
import local_stack_control.live_demo_claim_context


LOCAL_TENANT_ID = "00000000-0000-0000-0000-000000000100"
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


RendererStatusRead = collections.abc.Callable[[], local_stack_control.models.StatusReport]
RendererPoll = collections.abc.Callable[[RendererStatusRead, float], local_stack_control.models.StatusReport]


BaseCourseLifecycleReceipt = local_stack_control.base_course_lifecycle.Receipt


#============================================
def target_of(
	target: local_stack_control.models.ComposeTarget | local_stack_control.models.DisposableComposeTarget,
) -> local_stack_control.models.ComposeTarget:
	"""Return the common target without changing its owner authority."""
	if isinstance(target, local_stack_control.models.DisposableComposeTarget):
		return target.target
	return target


#============================================
def is_default_target(target: local_stack_control.models.ComposeTarget) -> bool:
	"""Return whether the exact target is eligible for default local bootstrap."""
	result = (
		target.project == local_stack_control.models.DEFAULT_PROJECT
		and local_stack_control.local_environment.is_default_local_environment(
			target.repo_root, target.env_file
		)
	)
	return result


#============================================
def is_teaching_profile(
	target: local_stack_control.models.ComposeTarget | local_stack_control.models.DisposableComposeTarget,
) -> bool:
	"""Return whether a closed disposable owner selected local teaching state."""
	result = (
		isinstance(target, local_stack_control.models.DisposableComposeTarget)
		and target.owner_policy in {"live-demo-baseline", "ui-walkthrough"}
	)
	return result


#============================================
def uses_local_teaching_state(
	target: local_stack_control.models.ComposeTarget | local_stack_control.models.DisposableComposeTarget,
) -> bool:
	"""Return whether target ownership authorizes local identities and teaching seed state."""
	selected = target_of(target)
	result = is_default_target(selected) or is_teaching_profile(target)
	return result


#============================================
def private_runtime_paths(repo_root: pathlib.Path, env_file: pathlib.Path) -> tuple[pathlib.Path, ...]:
	"""Return fixed local private paths beside one default or disposable environment."""
	directory = env_file.parent
	paths = (
		directory / "local-login.txt",
		directory / "local-identities.json",
		directory / ".secrets" / "invitation_token_secret",
		directory / ".secrets" / "question_id_secret",
		directory / local_stack_control.models.DEFAULT_LIVE_DEMO_SYSADMIN_CLAIM_CONTEXT_FILE,
		directory / ".secrets",
	)
	return paths


#============================================
def bootstrap_default_state(
	target: local_stack_control.models.ComposeTarget | local_stack_control.models.DisposableComposeTarget,
	runner: local_stack_control.process.CommandRunner | None = None,
) -> None:
	"""Create only absent default-local state before selected-env validation."""
	selected = target_of(target)
	if not uses_local_teaching_state(target):
		if not selected.env_file.exists():
			raise local_stack_control.models.ControllerError(
				"a custom mutating env file must already exist and have mode 0600"
			)
		return
	if is_default_target(selected):
		local_stack_control.local_environment.bootstrap_default_environment(
			selected.repo_root,
			selected.env_file,
			selected.repo_root / "containers/env.example",
		)
	# A new default is created mode 0600 before this point.  Every preexisting
	# selected teaching environment is rejected before parsing or replacement.
	local_stack_control.env_file.require_mutation_env_file(selected.env_file)
	configure_default_environment(selected, runner)
	credential_path, identity_path, invitation_path, question_path, _, _ = private_runtime_paths(
		selected.repo_root, selected.env_file
	)
	configuration = local_stack_control.local_identity.LocalIdentityConfiguration(
		credential_file=credential_path,
		identity_file=identity_path,
		tenant_id=LOCAL_TENANT_ID,
		instructor_id=LOCAL_INSTRUCTOR_ID,
		student_id=LOCAL_MARY_ID,
	)
	local_stack_control.local_identity.bootstrap_local_identities(configuration)
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
		"PLE_LOCAL_GRADER_PASSWORD": os.urandom(24).hex(),
		"PLE_INVITATION_TOKEN_SECRET_HOST_FILE": str(secret_directory / "invitation_token_secret"),
		"PLE_QUESTION_ID_SECRET_HOST_FILE": str(secret_directory / "question_id_secret"),
		"PLE_LIVE_DEMO_SYSADMIN_CLAIM_CONTEXT_HOST_FILE": str(
			runtime_directory / local_stack_control.models.DEFAULT_LIVE_DEMO_SYSADMIN_CLAIM_CONTEXT_FILE
		),
		"PLE_LIVE_DEMO_ELENA_INSTRUCTOR_USER_ID": LOCAL_INSTRUCTOR_ID,
		"PLE_LIVE_DEMO_MARY_STUDENT_USER_ID": LOCAL_MARY_ID,
		"PLE_LIVE_DEMO_JACK_STUDENT_USER_ID": LOCAL_JACK_ID,
		"PLE_LIVE_DEMO_AVERY_STUDENT_USER_ID": LOCAL_APPROVAL_CANDIDATE_ID,
		"PLE_LIVE_DEMO_SYSADMIN_USER_ID": LOCAL_SYSADMIN_ID,
		"PLE_WEBWORK_PROVENANCE_FILE": str(secret_directory / "webwork-renderer.provenance"),
		"PLE_LOCAL_AUTH_HOST_FILE": str(runtime_directory / "local-identities.json"),
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
		target=target, release=False, skip_build=False, no_open=True, mutation=False
	)
	values = local_stack_control.lifecycle_validation.validate_request(request)
	required = (
		"POSTGRES_USER", "POSTGRES_PASSWORD", "POSTGRES_DB", "MINIO_ROOT_USER",
		"MINIO_ROOT_PASSWORD", "PLE_LOCAL_GRADER_PASSWORD", "PLE_LOCAL_AUTH_HOST_FILE",
		"PLE_INVITATION_TOKEN_SECRET_HOST_FILE", "PLE_QUESTION_ID_SECRET_HOST_FILE",
		"PLE_LIVE_DEMO_SYSADMIN_CLAIM_CONTEXT_HOST_FILE",
		"PLE_LIVE_DEMO_ELENA_INSTRUCTOR_USER_ID", "PLE_LIVE_DEMO_MARY_STUDENT_USER_ID",
		"PLE_LIVE_DEMO_JACK_STUDENT_USER_ID", "PLE_LIVE_DEMO_AVERY_STUDENT_USER_ID",
		"PLE_LIVE_DEMO_SYSADMIN_USER_ID",
		"PLE_WEBWORK_RENDERER_ID", "PLE_WEBWORK_PROBLEM_JWT_SECRET",
		"PLE_WEBWORK_SESSION_JWT_SECRET",
		"PLE_WEBWORK_PROVENANCE_FILE",
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
	claim_path = absolute_value_path(target.repo_root, values["PLE_LIVE_DEMO_SYSADMIN_CLAIM_CONTEXT_HOST_FILE"])
	if claim_path.exists() or claim_path.is_symlink():
		local_stack_control.live_demo_claim_context.read_context(claim_path)
	if target.with_smtp:
		validate_smtp(values, target.repo_root)
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
def validate_smtp(values: dict[str, str], repo_root: pathlib.Path) -> None:
	"""Validate optional SMTP settings and the bounded private password file."""
	require_values(values, (
		"PLE_SMTP_RELAY", "PLE_SMTP_PORT", "PLE_SMTP_TLS_MODE", "PLE_SMTP_USERNAME",
		"PLE_SMTP_PASSWORD_HOST_FILE", "PLE_SMTP_FROM", "PLE_PUBLIC_APP_BASE_URL",
	))
	try:
		port = int(values["PLE_SMTP_PORT"])
	except ValueError as error:
		raise local_stack_control.models.ControllerError("selected SMTP port is invalid") from error
	if port < 1 or port > 65535 or values["PLE_SMTP_TLS_MODE"] not in ("starttls", "implicit-tls"):
		raise local_stack_control.models.ControllerError("selected SMTP settings are invalid")
	if "://" in values["PLE_SMTP_RELAY"] or not values["PLE_PUBLIC_APP_BASE_URL"].startswith("https://"):
		raise local_stack_control.models.ControllerError("selected SMTP endpoint settings are invalid")
	local_stack_control.private_files.read_current_user_private_file(
		absolute_value_path(repo_root, values["PLE_SMTP_PASSWORD_HOST_FILE"]), 4096
	)


#============================================
def validate_lifecycle(
	target: local_stack_control.models.ComposeTarget | local_stack_control.models.DisposableComposeTarget,
	runner: local_stack_control.process.CommandRunner,
	repo_root: pathlib.Path,
) -> None:
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
	compose_result = runner.run(
		local_stack_control.compose.compose_argv(selected, ["config"]), child_environment(selected), repo_root
	)
	require_command(compose_result, "Compose configuration validation")


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
	oci_id = local_stack_control.renderer.inspect_renderer_oci_id(
		runner, repo_root, values["PLE_WEBWORK_RENDERER_IMAGE"], environment
	)
	build_artifacts(runner, repo_root, options)
	# Reconcile the complete selected project before starting dependency stages.
	# Podman Compose applies --remove-orphans to the services named by a partial
	# `up`; using it with the later --no-deps application subset can remove the
	# database and renderer that subset requires.  A full project down preserves
	# named volumes while removing both current containers and obsolete services.
	compose_run(selected, runner, ["down", "--remove-orphans"])
	compose_run(selected, runner, ["--profile", "maintenance", "run", "--rm", "--no-deps", "-T", "postgres-major-guard"])
	compose_run(selected, runner, ["up", "-d", "postgres"])
	wait_for_postgres(selected, runner, values, options)
	synchronize_database(target, runner, values)
	run_migrations(runner, repo_root, values, environment)
	base_course = prepare_installed_base_course(
		runner, repo_root, target, values, environment
	)
	provision_grading_role(selected, runner, values)
	compose_run(selected, runner, ["up", "-d", "minio", "createbuckets"])
	wait_for_one_shot(selected, runner, options, "createbuckets")
	finalize_installed_base_course(
		runner, repo_root, target, values, environment, base_course
	)
	compose_run(selected, runner, ["up", "-d", "--force-recreate", "--no-deps", "webwork-renderer"])
	wait_for_renderer_ready(selected, runner, options, oci_id)
	attest_renderer(selected, runner, repo_root, values, oci_id)
	if is_teaching_profile(target):
		publish_chapter_one(runner, repo_root, target, values, environment)
	run_api_initializers(selected, runner, options)
	compose_run(selected, runner, ["build", "api", "gateway"])
	application_services = ["api", "worker"]
	if selected.with_smtp:
		application_services.append("invitation-delivery-worker")
	application_services.append("gateway")
	compose_run(
		selected,
		runner,
		[
			"up", "-d", "--force-recreate", "--no-deps",
			*application_services,
		],
	)
	gateway_url = wait_for_complete_ready(selected, runner, options)
	if is_default_target(selected):
		local_stack_control.image_cleanup.prune_superseded_images(runner, repo_root)
	if options.open_browser:
		open_browser(runner, repo_root, gateway_url)
	return LifecycleResult(selected.project, gateway_url, oci_id)


#============================================
def restart_lifecycle(
	target: local_stack_control.models.ComposeTarget | local_stack_control.models.DisposableComposeTarget,
	runner: local_stack_control.process.CommandRunner,
	repo_root: pathlib.Path,
	service: str,
	options: LifecycleOptions,
) -> LifecycleResult:
	"""Recreate only a proven stateless service and reprove the selected stack."""
	selected = target_of(target)
	require_lifecycle_inputs(selected, repo_root, options)
	require_disposable_ownership(target)
	if service not in local_stack_control.models.restartable_services(selected.with_smtp):
		raise local_stack_control.models.ControllerError("restart is limited to stateless services")
	if options.build or options.release or options.open_browser:
		raise local_stack_control.models.ControllerError("restart accepts no build, release, or browser-open intent")
	local_stack_control.env_file.require_mutation_env_file(selected.env_file)
	values = validate_static(selected)
	local_stack_control.lifecycle_validation.require_mutation_engine(runner, repo_root, True)
	require_restart_baseline(selected, runner, service)
	oci_id = local_stack_control.renderer.inspect_renderer_oci_id(
		runner, repo_root, values["PLE_WEBWORK_RENDERER_IMAGE"], child_environment(selected)
	)
	if service == "webwork-renderer":
		require_renderer_restart_provenance(selected, values, oci_id)
	else:
		require_attested_running_renderer(selected, runner, values, oci_id)
		if service == "api":
			probe_renderer(selected, runner, repo_root, oci_id)
	if service == "api":
		run_api_initializers(selected, runner, options)
	if service == "invitation-delivery-worker":
		run_smtp_initializer(selected, runner, options)
	compose_run(selected, runner, ["up", "-d", "--force-recreate", "--no-deps", service])
	if service == "webwork-renderer":
		wait_for_renderer_ready(selected, runner, options, oci_id)
		attest_renderer(selected, runner, repo_root, values, oci_id)
	gateway_url = wait_for_complete_ready(selected, runner, options)
	return LifecycleResult(selected.project, gateway_url, oci_id)


#============================================
def validate_compose(target: local_stack_control.models.ComposeTarget, runner: local_stack_control.process.CommandRunner, repo_root: pathlib.Path) -> None:
	"""Validate selected Compose interpolation after the mutation engine is proven."""
	compose_result = runner.run(
		local_stack_control.compose.compose_argv(target, ["config"]), child_environment(target), repo_root
	)
	require_command(compose_result, "Compose configuration validation")


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
def child_environment(target: local_stack_control.models.ComposeTarget) -> dict[str, str]:
	"""Build the selected environment authority for each child process."""
	return local_stack_control.compose.target_environment(
		target, local_stack_control.process.current_environment()
	)


#============================================
def compose_run(target: local_stack_control.models.ComposeTarget, runner: local_stack_control.process.CommandRunner, arguments: list[str]) -> None:
	"""Run one non-secret Compose operation and retain failures for diagnosis."""
	result = runner.run(local_stack_control.compose.compose_argv(target, arguments), child_environment(target), target.repo_root)
	require_command(result, "selected Compose operation")


#============================================
def require_command(result: local_stack_control.models.CommandResult, operation: str) -> None:
	"""Convert child failure into bounded non-secret lifecycle guidance."""
	if not result.ok():
		detail = local_stack_control.lifecycle_diagnostics.redacted_failure_detail(result)
		raise local_stack_control.models.ControllerError(
			f"{operation} failed ({detail}); retained stack resources are available for diagnostics"
		)


#============================================
def build_artifacts(runner: local_stack_control.process.CommandRunner, repo_root: pathlib.Path, options: LifecycleOptions) -> None:
	"""Build host artifacts or require both browser outputs before any storage mutation."""
	if not options.build:
		if not (repo_root / "dist/index.html").is_file() or not (repo_root / "dist/main.js").is_file():
			raise local_stack_control.models.ControllerError("reuse build requires a complete dist bundle")
		return
	profile = "--release" if options.release else "--debug"
	environment = local_stack_control.env_file.sanitized_runtime_environment(
		local_stack_control.process.current_environment()
	)
	environment["PLE_BROWSER_LOCAL_DEVELOPMENT_AUTH"] = "1"
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
	read_status: RendererStatusRead | None = None,
	poll_ready: RendererPoll = local_stack_control.lifecycle_wait.poll_ready,
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
def synchronize_database(target: local_stack_control.models.ComposeTarget | local_stack_control.models.DisposableComposeTarget, runner: local_stack_control.process.CommandRunner, values: dict[str, str]) -> None:
	"""Synchronize the local database login only for the default local target."""
	if not uses_local_teaching_state(target):
		return
	selected = target_of(target)
	password = values["POSTGRES_PASSWORD"]
	environment = child_environment(selected)
	environment["PGPASSWORD"] = password
	argv = local_stack_control.compose.compose_argv(selected, ["exec", "-T", "postgres", "psql", "-v", "ON_ERROR_STOP=1", "-U", values["POSTGRES_USER"], "-d", values["POSTGRES_DB"]])
	sql = postgres_role_sql(values["POSTGRES_USER"], password)
	require_command(runner.run(argv, environment, selected.repo_root, sql), "local database login synchronization")


#============================================
def run_migrations(runner: local_stack_control.process.CommandRunner, repo_root: pathlib.Path, values: dict[str, str], environment: dict[str, str]) -> None:
	"""Run migrations with the database URL only in the direct child environment."""
	child = dict(environment)
	child["PLE_MIGRATION_DATABASE_URL"] = database_url(values)
	require_command(runner.run(["cargo", "tools", "database", "migrate"], child, repo_root), "database migration")


#============================================
def prepare_installed_base_course(
	runner: local_stack_control.process.CommandRunner,
	repo_root: pathlib.Path,
	target: local_stack_control.models.ComposeTarget
	| local_stack_control.models.DisposableComposeTarget,
	values: dict[str, str],
	environment: dict[str, str],
) -> BaseCourseLifecycleReceipt | None:
	"""Classify the migrated Base Course state before starting object storage."""
	if not uses_local_teaching_state(target):
		return None
	child = dict(environment)
	child["PLE_MIGRATION_DATABASE_URL"] = database_url(values)
	child["PLE_QUESTION_ID_SECRET_FILE"] = values["PLE_QUESTION_ID_SECRET_HOST_FILE"]
	result = run_base_course_phase(runner, repo_root, child, "prepare")
	return result


#============================================
def finalize_installed_base_course(
	runner: local_stack_control.process.CommandRunner,
	repo_root: pathlib.Path,
	target: local_stack_control.models.ComposeTarget
	| local_stack_control.models.DisposableComposeTarget,
	values: dict[str, str],
	environment: dict[str, str],
	preparation: BaseCourseLifecycleReceipt | None,
) -> None:
	"""Finish an installing baseline after ordinary object-storage readiness."""
	if preparation is None:
		return
	selected = target_of(target)
	if preparation.install_state == "complete":
		write_base_course_diagnostic(selected, preparation.raw_output)
		ensure_live_demo_claim_context(selected, values, preparation)
		return
	local_stack_control.base_course_lifecycle.ensure_storage_receipt(
		selected, runner, preparation, child_environment(selected)
	)
	child = dict(environment)
	child["PLE_MIGRATION_DATABASE_URL"] = database_url(values)
	child["PLE_QUESTION_ID_SECRET_FILE"] = values["PLE_QUESTION_ID_SECRET_HOST_FILE"]
	completed = run_base_course_phase(
		runner, repo_root, child, "install", preparation.storage_receipt_json
	)
	if completed.install_state != "complete":
		raise local_stack_control.models.ControllerError(
			"installed Base Course install did not complete"
		)
	write_base_course_diagnostic(selected, completed.raw_output)
	ensure_live_demo_claim_context(selected, values, completed)


#============================================
def ensure_live_demo_claim_context(
	target: local_stack_control.models.ComposeTarget,
	values: dict[str, str],
	receipt: BaseCourseLifecycleReceipt,
) -> None:
	"""Bind the private Sysadmin proof to the completed Rust lifecycle receipt."""
	path = absolute_value_path(target.repo_root, values["PLE_LIVE_DEMO_SYSADMIN_CLAIM_CONTEXT_HOST_FILE"])
	local_stack_control.live_demo_claim_context.ensure_context(
		path, receipt.installation_generation, values["PLE_LIVE_DEMO_SYSADMIN_USER_ID"]
	)


#============================================
def run_base_course_phase(
	runner: local_stack_control.process.CommandRunner,
	repo_root: pathlib.Path,
	child: dict[str, str],
	phase: str,
	storage_receipt: str | None = None,
) -> BaseCourseLifecycleReceipt:
	"""Invoke one closed lifecycle phase and decode its authoritative response."""
	argv = [
		"cargo",
		"tools",
		"base-course",
		"--apply-migrations",
		"--tenant",
		LOCAL_TENANT_ID,
		"--instructor",
		LOCAL_INSTRUCTOR_ID,
		"--mary",
		LOCAL_MARY_ID,
		"--jack",
		LOCAL_JACK_ID,
		"--approval-candidate",
		LOCAL_APPROVAL_CANDIDATE_ID,
		"--sysadmin",
		LOCAL_SYSADMIN_ID,
		"--lifecycle-phase",
		phase,
	]
	if storage_receipt is not None:
		argv.extend(["--storage-receipt", storage_receipt])
	result = runner.run(argv, child, repo_root)
	require_command(result, f"installed Base Course {phase}")
	return local_stack_control.base_course_lifecycle.decode(result.stdout, phase)


#============================================
def write_base_course_diagnostic(
	target: local_stack_control.models.ComposeTarget,
	output: str,
) -> None:
	"""Write host-only diagnostics only after a complete Rust-owned lifecycle result."""
	manifest = target.env_file.parent / local_stack_control.models.DEFAULT_BASE_COURSE_MANIFEST_FILE
	local_stack_control.private_files.write_atomic_file(
		manifest,
		output.encode("utf-8"),
		0o600,
	)


#============================================
def provision_grading_role(target: local_stack_control.models.ComposeTarget, runner: local_stack_control.process.CommandRunner, values: dict[str, str]) -> None:
	"""Set the restricted grading role password through a minimum child environment."""
	environment = child_environment(target)
	environment["PGPASSWORD"] = values["POSTGRES_PASSWORD"]
	environment["PLE_LOCAL_GRADER_PASSWORD"] = values["PLE_LOCAL_GRADER_PASSWORD"]
	argv = local_stack_control.compose.compose_argv(target, ["exec", "-T", "postgres", "psql", "-v", "ON_ERROR_STOP=1", "-U", values["POSTGRES_USER"], "-d", values["POSTGRES_DB"]])
	sql = postgres_role_sql("ple_grading_reader", values["PLE_LOCAL_GRADER_PASSWORD"])
	require_command(runner.run(argv, environment, target.repo_root, sql), "grading role provisioning")


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
	write_renderer_provenance(target, values, oci_id)


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
	require_renderer_restart_provenance(target, values, oci_id)


#============================================
def require_running_renderer(target: local_stack_control.models.ComposeTarget, runner: local_stack_control.process.CommandRunner, oci_id: str) -> None:
	"""Require the single selected renderer to be healthy and image-matched."""
	local_stack_control.renderer.require_running_renderer(status_report(target, runner), oci_id)


#============================================
def renderer_provenance_directory(target: local_stack_control.models.ComposeTarget, values: dict[str, str]) -> pathlib.Path:
	"""Resolve the fixed selected private provenance directory."""
	provenance_path = absolute_value_path(target.repo_root, values["PLE_WEBWORK_PROVENANCE_FILE"])
	if provenance_path.name != local_stack_control.renderer.PROVENANCE_NAME:
		raise local_stack_control.models.ControllerError("selected renderer provenance path has an invalid name")
	return provenance_path.parent


#============================================
def write_renderer_provenance(target: local_stack_control.models.ComposeTarget, values: dict[str, str], oci_id: str) -> None:
	"""Atomically replace the private renderer attestation after a successful probe."""
	provenance = local_stack_control.models.RendererProvenance(values["PLE_WEBWORK_RENDERER_IMAGE"], oci_id)
	local_stack_control.renderer.write_provenance(renderer_provenance_directory(target, values), provenance)


#============================================
def require_renderer_restart_provenance(target: local_stack_control.models.ComposeTarget, values: dict[str, str], oci_id: str) -> None:
	"""Require the preexisting renderer attestation before a renderer recovery mutation."""
	local_stack_control.renderer.require_restart_provenance(
		renderer_provenance_directory(target, values), oci_id
	)


#============================================
def publish_chapter_one(runner: local_stack_control.process.CommandRunner, repo_root: pathlib.Path, target: local_stack_control.models.ComposeTarget | local_stack_control.models.DisposableComposeTarget, values: dict[str, str], environment: dict[str, str]) -> None:
	"""Run the default-only canonical publisher with private capabilities off argv."""
	selected = target_of(target)
	manifest = selected.env_file.parent / "local-chapter-one-pilot.json"
	existing_manifest = manifest if manifest.exists() or manifest.is_symlink() else None
	request = local_stack_control.chapter_one.ChapterOneSeedRequest(
		repo_root=repo_root,
		database_url=database_url(values),
		tenant_id=LOCAL_TENANT_ID,
		instructor_id=LOCAL_INSTRUCTOR_ID,
		student_id=LOCAL_MARY_ID,
		s3_endpoint="http://127.0.0.1:" + values.get("PLE_MINIO_API_HOST_PORT", "9000"),
		aws_access_key_id=values["MINIO_ROOT_USER"],
		aws_secret_access_key=values["MINIO_ROOT_PASSWORD"],
		question_id_secret_file=absolute_value_path(repo_root, values["PLE_QUESTION_ID_SECRET_HOST_FILE"]),
		manifest_path=manifest,
		existing_manifest_path=existing_manifest,
	)
	local_stack_control.chapter_one.publish_with_runner(request, runner)


#============================================
def run_api_initializers(target: local_stack_control.models.ComposeTarget, runner: local_stack_control.process.CommandRunner, options: LifecycleOptions) -> None:
	"""Refresh API-owned initializers before recreating API-owned stateless services."""
	if target.with_smtp:
		run_smtp_initializer(target, runner, options)
	for service in ("identity-secret-init",):
		compose_run(target, runner, ["up", "-d", "--force-recreate", "--no-deps", service])
		wait_for_one_shot(target, runner, options, service)


#============================================
def run_smtp_initializer(target: local_stack_control.models.ComposeTarget, runner: local_stack_control.process.CommandRunner, options: LifecycleOptions) -> None:
	"""Refresh the SMTP credential copy before one SMTP-consuming service starts."""
	if not target.with_smtp:
		raise local_stack_control.models.ControllerError("SMTP initializer requires the SMTP topology")
	compose_run(target, runner, ["up", "-d", "--force-recreate", "--no-deps", "smtp-secret-init"])
	wait_for_one_shot(target, runner, options, "smtp-secret-init")


#============================================
def status_report(target: local_stack_control.models.ComposeTarget, runner: local_stack_control.process.CommandRunner) -> local_stack_control.models.StatusReport:
	"""Discover the selected target through existing label-derived ownership logic."""
	snapshot = local_stack_control.discovery.discover_snapshot(runner, target.repo_root, target.project)
	return local_stack_control.status.build_report(target.project, target.with_smtp, snapshot)


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
def require_complete_ready(target: local_stack_control.models.ComposeTarget, runner: local_stack_control.process.CommandRunner) -> None:
	"""Require one already-ready full stack before a stateless restart."""
	local_stack_control.lifecycle_wait.require_ready(status_report(target, runner))


#============================================
def require_restart_baseline(
	target: local_stack_control.models.ComposeTarget,
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
	if len(matching) != 1 or matching[0].instances > 1 or matching[0].state == "ambiguous":
		raise local_stack_control.models.ControllerError(
			"selected restart service is absent from the declared topology or has duplicate instances"
		)
	for item in report.services:
		if item.service != selected_service and not item.healthy:
			raise local_stack_control.models.ControllerError(
				"a non-selected required service is not healthy"
			)


#============================================
def wait_for_complete_ready(target: local_stack_control.models.ComposeTarget, runner: local_stack_control.process.CommandRunner, options: LifecycleOptions) -> str:
	"""Require loopback gateway health and complete label-derived semantic readiness."""
	url = gateway_url(target)
	def read_report() -> local_stack_control.models.StatusReport:
		result = runner.run(["curl", "--fail", "--silent", "--show-error", "--max-time", "2", "--output", "/dev/null", url + "health"], child_environment(target), target.repo_root)
		if result.ok():
			return status_report(target, runner)
		return unavailable_report(target)
	local_stack_control.lifecycle_wait.poll_ready(read_report, options.timeout_seconds)
	require_complete_ready(target, runner)
	return url


#============================================
def gateway_url(target: local_stack_control.models.ComposeTarget) -> str:
	"""Derive the non-secret loopback gateway URL from selected environment authority."""
	values = local_stack_control.env_file.env_settings(target.env_file)
	port = values.get("PLE_GATEWAY_HOST_PORT", "8080")
	if not port.isdecimal() or not 1 <= int(port) <= 65535:
		raise local_stack_control.models.ControllerError("selected gateway port is invalid")
	return f"http://127.0.0.1:{port}/"


#============================================
def open_browser(runner: local_stack_control.process.CommandRunner, repo_root: pathlib.Path, url: str) -> None:
	"""Open the proven loopback URL using an argument-array platform opener."""
	result = runner.run(["open", url], local_stack_control.env_file.sanitized_runtime_environment(local_stack_control.process.current_environment()), repo_root)
	if not result.ok():
		result = runner.run(["xdg-open", url], local_stack_control.env_file.sanitized_runtime_environment(local_stack_control.process.current_environment()), repo_root)
		require_command(result, "browser open")

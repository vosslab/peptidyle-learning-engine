"""Offline behavioral contracts for the typed WP-PY-L1 lifecycle core."""

import dataclasses
import os
import pathlib

import pytest

import local_stack_control.lifecycle
import local_stack_control.local_environment
import local_stack_control.lifecycle_profiles
import local_stack_control.models
import local_stack_control.process
import local_stack_control.renderer
import local_stack_control.env_file
import local_stack_control.image_cleanup
import local_stack_control.status


class UnexpectedRunner(local_stack_control.process.CommandRunner):
	"""Reject every child process unless a test explicitly supplies an expectation."""

	#============================================
	def run(
		self,
		argv: list[str],
		environment: dict[str, str] | None = None,
		cwd: pathlib.Path | None = None,
		stdin: str | None = None,
	) -> local_stack_control.models.CommandResult:
		raise AssertionError(f"unexpected lifecycle command: {argv}")

	#============================================
	def stream(
		self,
		argv: list[str],
		environment: dict[str, str] | None = None,
		cwd: pathlib.Path | None = None,
	) -> int:
		raise AssertionError(f"unexpected lifecycle stream: {argv}")


class GatewayPortRunner(local_stack_control.process.CommandRunner):
	"""Return exact injected listener and default-gateway observations."""

	def __init__(self, listening: tuple[str, ...], gateway_running: bool) -> None:
		"""Store fixed non-network observations for one port selection decision."""
		self.listening = listening
		self.gateway_running = gateway_running

	#============================================
	def run(self, argv: list[str], environment: dict[str, str] | None = None, cwd: pathlib.Path | None = None, stdin: str | None = None) -> local_stack_control.models.CommandResult:
		"""Answer only the expected lsof and Podman name requests."""
		if argv[:2] == ["lsof", "-nP"]:
			port = argv[2].split(":")[1]
			return local_stack_control.models.CommandResult(tuple(argv), 0 if port in self.listening else 1, "1" if port in self.listening else "", "")
		if argv[:3] == ["podman", "ps", "--format"]:
			stdout = "containers_gateway_1\n" if self.gateway_running else ""
			return local_stack_control.models.CommandResult(tuple(argv), 0, stdout, "")
		raise AssertionError(f"unexpected port decision command: {argv}")

	#============================================
	def stream(self, argv: list[str], environment: dict[str, str] | None = None, cwd: pathlib.Path | None = None) -> int:
		"""Keep port decisions captured and deterministic."""
		raise AssertionError("gateway port selection does not stream commands")


#============================================
def lifecycle_target(tmp_path: pathlib.Path, project: str, env_name: str) -> local_stack_control.models.ComposeTarget:
	"""Build one selected target without reading a tracked configuration file."""
	env_file = tmp_path / env_name
	return local_stack_control.models.ComposeTarget(
		repo_root=tmp_path,
		project=project,
		env_file=env_file,
		compose_files=(),
		provider=local_stack_control.models.ComposeProvider(("podman", "compose"), "podman compose"),
		with_smtp=False,
		env_setting_names=(),
	)


#============================================
def live_demo_target(
	tmp_path: pathlib.Path,
	profile: local_stack_control.models.LiveDemoProfile,
) -> local_stack_control.models.DisposableComposeTarget:
	"""Build one fixed live-demo target with closed profile metadata."""
	selected = lifecycle_target(
		tmp_path,
		local_stack_control.models.LIVE_DEMO_BROWSER_PROJECT,
		"live/env.local",
	)
	disposable = local_stack_control.models.DisposableComposeTarget(
		target=selected,
		owner_policy=local_stack_control.models.LIVE_DEMO_BROWSER_OWNER,
		capability_file=tmp_path / "capability",
		project_prefix=local_stack_control.models.LIVE_DEMO_BROWSER_PROJECT,
		private_environment_file=selected.env_file,
		live_demo_profile=profile,
	)
	return disposable


#============================================
def readiness_container(
	service: str,
	identifier: int,
	*,
	healthy: bool = True,
) -> local_stack_control.models.ContainerResource:
	"""Build one deterministic required-service readiness observation."""
	one_shot = service in local_stack_control.models.BASE_ONE_SHOT_SERVICES
	running = healthy and not one_shot
	state = "exited" if one_shot else "running"
	health: str | None = "healthy"
	if service == "worker" or one_shot:
		health = None
	if not healthy:
		state = "exited"
		health = None
	container = local_stack_control.models.ContainerResource(
		id=f"{service}-{identifier}",
		names=(),
		project=local_stack_control.models.LIVE_DEMO_BROWSER_PROJECT,
		service=service,
		state=state,
		running=running,
		exit_code=0 if healthy else 1,
		health=health,
		image="local/image",
		ports=(),
	)
	return container


#============================================
def replica_readiness_snapshot(
	api_instances: int,
	*,
	unhealthy_api: int | None = None,
	postgres_instances: int = 1,
) -> local_stack_control.models.ProjectSnapshot:
	"""Build the complete fixed replica topology without invoking Compose."""
	containers: list[local_stack_control.models.ContainerResource] = []
	for service in local_stack_control.models.BASE_ONE_SHOT_SERVICES:
		containers.append(readiness_container(service, 0))
	for service in local_stack_control.models.BASE_LONG_RUNNING_SERVICES:
		instances = 1
		if service == "api":
			instances = api_instances
		elif service == "postgres":
			instances = postgres_instances
		for identifier in range(instances):
			containers.append(
				readiness_container(
					service,
					identifier,
					healthy=service != "api" or identifier != unhealthy_api,
				)
			)
	snapshot = local_stack_control.models.ProjectSnapshot(
		local_stack_control.models.LIVE_DEMO_BROWSER_PROJECT,
		tuple(containers),
		(),
		(),
	)
	return snapshot


#============================================
def test_validation_rejects_invalid_selected_env_before_any_process(tmp_path: pathlib.Path) -> None:
	"""Read-only validation refuses malformed selected configuration without a child effect."""
	target = lifecycle_target(tmp_path, "custom", "custom.env")
	target.env_file.write_text("PLE_WEBWORK_RENDERER_IMAGE=unsafe;image\n", encoding="ascii")
	target.env_file.chmod(0o600)
	with pytest.raises(local_stack_control.models.ControllerError):
		local_stack_control.lifecycle.validate_lifecycle(target, UnexpectedRunner(), tmp_path)
	assert target.env_file.exists()


#============================================
def test_custom_start_refuses_missing_environment_before_engine_mutation(tmp_path: pathlib.Path) -> None:
	"""A custom target never inherits default bootstrap authority."""
	target = lifecycle_target(tmp_path, "custom", "custom.env")
	options = local_stack_control.lifecycle.LifecycleOptions(1.0, False, False, False)
	with pytest.raises(local_stack_control.models.ControllerError):
		local_stack_control.lifecycle.start_lifecycle(target, UnexpectedRunner(), tmp_path, options)
	assert not target.env_file.exists()


#============================================
def test_default_bootstrap_preserves_existing_selected_value(tmp_path: pathlib.Path) -> None:
	"""Default bootstrap fills first-run settings without replacing a configured value."""
	target = lifecycle_target(tmp_path, "containers", "containers/env.local")
	target.env_file.parent.mkdir()
	target.env_file.write_text("KEEP=value\n", encoding="ascii")
	target.env_file.chmod(0o600)
	local_stack_control.lifecycle.bootstrap_default_state(target)
	assert "KEEP=value\n" in target.env_file.read_text(encoding="ascii")


#============================================
@pytest.mark.parametrize("unsafe_mode", (0o644, 0o640))
def test_existing_default_environment_is_refused_before_default_generation(
	tmp_path: pathlib.Path,
	unsafe_mode: int,
) -> None:
	"""An unsafe preexisting default environment remains unchanged and starts no process."""
	target = lifecycle_target(tmp_path, "containers", "containers/env.local")
	target.env_file.parent.mkdir()
	content = b"KEEP=unchanged\n"
	target.env_file.write_bytes(content)
	target.env_file.chmod(unsafe_mode)
	with pytest.raises(local_stack_control.models.ControllerError):
		local_stack_control.lifecycle.bootstrap_default_state(target, UnexpectedRunner())
	assert target.env_file.read_bytes() == content


#============================================
def test_existing_default_environment_with_foreign_owner_is_refused_before_generation(
	tmp_path: pathlib.Path,
	monkeypatch: pytest.MonkeyPatch,
) -> None:
	"""Current-user ownership is required before reading an existing default environment."""
	target = lifecycle_target(tmp_path, "containers", "containers/env.local")
	target.env_file.parent.mkdir()
	content = b"KEEP=unchanged\n"
	target.env_file.write_bytes(content)
	target.env_file.chmod(0o600)
	owner_id = os.getuid()
	monkeypatch.setattr(local_stack_control.env_file.os, "getuid", lambda: owner_id + 1)
	with pytest.raises(local_stack_control.models.ControllerError):
		local_stack_control.lifecycle.bootstrap_default_state(target, UnexpectedRunner())
	assert target.env_file.read_bytes() == content


#============================================
def test_failed_one_shot_refuses_before_polling_or_later_lifecycle_work(
	tmp_path: pathlib.Path,
	monkeypatch: pytest.MonkeyPatch,
) -> None:
	"""A named initializer exit failure is terminal rather than a readiness timeout."""
	target = lifecycle_target(tmp_path, "containers", "containers/env.local")
	failed = local_stack_control.models.StackServiceStatus(
		service="createbuckets", instances=1, present=True, running=False,
		healthy=False, complete=False, state="exited", health=None, exit_code=1,
	)
	report = local_stack_control.models.StatusReport(
		project="containers", with_smtp=False,
		snapshot=local_stack_control.models.ProjectSnapshot("containers", (), (), ()),
		services=(failed,), ok=False, state="failed", message="a required service failed",
	)
	monkeypatch.setattr(local_stack_control.lifecycle, "status_report", lambda target, runner: report)
	options = local_stack_control.lifecycle.LifecycleOptions(1.0, False, False, False)
	with pytest.raises(local_stack_control.models.ControllerError):
		local_stack_control.lifecycle.wait_for_one_shot(target, UnexpectedRunner(), options, "createbuckets")


#============================================
def test_completed_requested_one_shot_does_not_wait_for_later_initializers(
	tmp_path: pathlib.Path,
	monkeypatch: pytest.MonkeyPatch,
) -> None:
	"""Storage setup advances after its own successful result, before API-only initialization."""
	target = lifecycle_target(tmp_path, "containers", "containers/env.local")
	complete = local_stack_control.models.StackServiceStatus(
		service="createbuckets", instances=1, present=True, running=False,
		healthy=True, complete=True, state="exited", health=None, exit_code=0,
	)
	report = local_stack_control.models.StatusReport(
		project="containers", with_smtp=False,
		snapshot=local_stack_control.models.ProjectSnapshot("containers", (), (), ()),
		services=(complete,), ok=False, state="starting", message="later initializers are pending",
	)
	monkeypatch.setattr(local_stack_control.lifecycle, "status_report", lambda target, runner: report)
	options = local_stack_control.lifecycle.LifecycleOptions(1.0, False, False, False)
	local_stack_control.lifecycle.wait_for_one_shot(target, UnexpectedRunner(), options, "createbuckets")


#============================================
@pytest.mark.parametrize(
	"teaching_profile",
	(False, True, local_stack_control.models.LiveDemoProfile.REPLICA_RESTART),
)
def test_start_orders_required_effects_before_semantic_readiness(
	tmp_path: pathlib.Path,
	monkeypatch: pytest.MonkeyPatch,
	teaching_profile: bool | local_stack_control.models.LiveDemoProfile,
) -> None:
	"""A start classifies migrated data before storage and preserves later dependencies."""
	replica_profile = (
		teaching_profile is local_stack_control.models.LiveDemoProfile.REPLICA_RESTART
	)
	if replica_profile:
		target = live_demo_target(tmp_path, teaching_profile)
		base_target = target.target
	else:
		base_target = lifecycle_target(
			tmp_path,
			"ple_live_demo_baseline_test" if teaching_profile else "containers",
			"walk/env.local" if teaching_profile else "containers/env.local",
		)
		target = base_target
	if teaching_profile is True:
		target = local_stack_control.models.DisposableComposeTarget(
			target=base_target,
			owner_policy="live-demo-baseline",
			capability_file=tmp_path / "capability",
			project_prefix="ple_live_demo_baseline_",
			private_environment_file=base_target.env_file,
		)
	options = local_stack_control.lifecycle.LifecycleOptions(1.0, False, False, False)
	events: list[str] = []
	compose_arguments: list[list[str]] = []
	readiness_targets: list[
		local_stack_control.models.ComposeTarget
		| local_stack_control.models.DisposableComposeTarget
	] = []
	values = {"PLE_WEBWORK_RENDERER_IMAGE": "localhost/renderer:tag"}

	def mark(name: str) -> None:
		events.append(name)

	def compose_mark(
		target: local_stack_control.models.ComposeTarget,
		runner: local_stack_control.process.CommandRunner,
		arguments: list[str],
	) -> None:
		del target, runner
		compose_arguments.append(arguments)
		if "createbuckets" in arguments:
			mark("storage")
		elif "gateway" in arguments:
			mark("api")
		else:
			mark("maintenance")

	monkeypatch.setattr(local_stack_control.lifecycle, "require_lifecycle_inputs", lambda target, root, options: mark("inputs"))
	monkeypatch.setattr(local_stack_control.lifecycle, "require_disposable_ownership", lambda target: mark("ownership"))
	monkeypatch.setattr(local_stack_control.lifecycle, "bootstrap_default_state", lambda target, runner: mark("bootstrap"))
	monkeypatch.setattr(local_stack_control.env_file, "require_mutation_env_file", lambda path: mark("environment"))
	monkeypatch.setattr(local_stack_control.lifecycle, "validate_static", lambda target: values)
	monkeypatch.setattr(local_stack_control.lifecycle_validation, "require_mutation_engine", lambda runner, root, start: mark("engine"))
	monkeypatch.setattr(local_stack_control.lifecycle, "validate_compose", lambda target, runner, root: mark("compose-validation"))
	monkeypatch.setattr(local_stack_control.lifecycle, "child_environment", lambda target: {})
	monkeypatch.setattr(
		local_stack_control.renderer,
		"ensure_renderer_oci_id",
		lambda runner, root, reference, environment, build: mark("renderer-image") or "sha256:" + "a" * 64,
	)
	monkeypatch.setattr(local_stack_control.lifecycle, "build_artifacts", lambda runner, root, options: mark("build"))
	monkeypatch.setattr(local_stack_control.lifecycle, "compose_run", compose_mark)
	monkeypatch.setattr(local_stack_control.lifecycle, "wait_for_one_shot", lambda target, runner, options, service: mark("storage-ready" if service == "createbuckets" else "api-initialized"))
	monkeypatch.setattr(local_stack_control.lifecycle, "wait_for_postgres", lambda target, runner, values, options: mark("database-ready"))
	monkeypatch.setattr(local_stack_control.lifecycle, "synchronize_database", lambda target, runner, values, options: mark("database-login"))
	monkeypatch.setattr(local_stack_control.lifecycle, "run_migrations", lambda runner, root, values, environment: mark("migrated"))
	monkeypatch.setattr(
		local_stack_control.process_logins,
		"provision",
		lambda target, runner, values, environment: mark("process-logins"),
	)
	monkeypatch.setattr(
		local_stack_control.base_course_logins,
		"provision",
		lambda target, runner, values, environment: mark("base-course-logins")
		or ("postgres://installer", "postgres://app"),
	)
	monkeypatch.setattr(
		local_stack_control.lifecycle,
		"prepare_installed_base_course",
		lambda runner, root, target, values, environment, database_urls: mark("prepared"),
	)
	monkeypatch.setattr(
		local_stack_control.lifecycle,
		"finalize_installed_base_course",
		lambda runner, root, target, values, environment, preparation, database_urls:
			mark("seeded"),
	)
	monkeypatch.setattr(local_stack_control.lifecycle, "provision_grading_role", lambda target, runner, values: mark("grading-role"))
	monkeypatch.setattr(local_stack_control.lifecycle, "wait_for_renderer_ready", lambda target, runner, options, identity: mark("renderer-ready"))
	monkeypatch.setattr(local_stack_control.lifecycle, "attest_renderer", lambda target, runner, root, values, identity: mark("renderer-probed"))
	monkeypatch.setattr(local_stack_control.lifecycle, "publish_chapter_one", lambda runner, root, target, values, environment: mark("chapter-one"))
	monkeypatch.setattr(local_stack_control.lifecycle, "run_api_initializers", lambda target, runner, options: mark("api-initializers"))
	def mark_ready(
		selected_target: local_stack_control.models.ComposeTarget
		| local_stack_control.models.DisposableComposeTarget,
		runner: local_stack_control.process.CommandRunner,
		selected_options: local_stack_control.lifecycle.LifecycleOptions,
	) -> str:
		del runner, selected_options
		readiness_targets.append(selected_target)
		mark("ready")
		return "http://127.0.0.1:8080/"

	monkeypatch.setattr(local_stack_control.lifecycle, "wait_for_complete_ready", mark_ready)
	monkeypatch.setattr(
		local_stack_control.image_cleanup,
		"prune_superseded_images",
		lambda runner, root: mark("image-prune"),
	)

	local_stack_control.lifecycle.start_lifecycle(target, UnexpectedRunner(), tmp_path, options)
	assert (
		events.index("migrated")
		< events.index("process-logins")
		< events.index("base-course-logins")
		< events.index("prepared")
		< events.index("storage")
	)
	assert events.index("prepared") < events.index("storage-ready")
	assert events.index("storage-ready") < events.index("seeded")
	assert events.index("renderer-ready") < events.index("renderer-probed")
	assert events.index("build") < events.index("renderer-image") < events.index("maintenance")
	if teaching_profile:
		assert events.index("renderer-probed") < events.index("chapter-one") < events.index("api-initializers")
		assert "image-prune" not in events
	else:
		assert events.index("renderer-probed") < events.index("api-initializers")
		assert events.index("ready") < events.index("image-prune")
	assert events.index("api-initializers") < events.index("api") < events.index("ready")
	assert readiness_targets == [target]
	application_start = next(
		arguments
		for arguments in compose_arguments
		if arguments[0] == "up" and "gateway" in arguments
	)
	project_reconcile = next(
		arguments
		for arguments in compose_arguments
		if arguments[0] == "down"
	)
	maintenance_start = next(
		arguments
		for arguments in compose_arguments
		if arguments[0] == "--profile"
	)
	database_start = next(
		arguments
		for arguments in compose_arguments
		if arguments == ["up", "-d", "postgres"]
	)
	storage_start = next(
		arguments
		for arguments in compose_arguments
		if arguments == ["up", "-d", "minio", "createbuckets"]
	)
	assert project_reconcile == ["down", "--remove-orphans"]
	assert compose_arguments.index(project_reconcile) < compose_arguments.index(maintenance_start)
	assert compose_arguments.index(database_start) < compose_arguments.index(storage_start)
	assert "--force-recreate" in application_start
	assert "--remove-orphans" not in application_start
	assert "--no-deps" in application_start
	if replica_profile:
		assert application_start == [
			"up", "-d", "--force-recreate", "--no-deps",
			"--scale", "api=2", "api", "worker", "gateway",
		]
	else:
		assert "--scale" not in application_start


#============================================
@pytest.mark.parametrize(
	("profile", "expected_api_instances"),
	(
		(local_stack_control.models.LiveDemoProfile.BROWSER, 1),
		(local_stack_control.models.LiveDemoProfile.WEBWORK_RENDER_RPC, 1),
		(local_stack_control.models.LiveDemoProfile.REPLICA_RESTART, 2),
	),
)
def test_live_demo_profiles_own_their_expected_api_cardinality(
	tmp_path: pathlib.Path,
	profile: local_stack_control.models.LiveDemoProfile,
	expected_api_instances: int,
) -> None:
	"""Each closed profile determines readiness cardinality without a caller knob."""
	target = live_demo_target(tmp_path, profile)
	count = local_stack_control.lifecycle_profiles.expected_long_running_count(
		target, "api"
	)

	assert count == expected_api_instances


#============================================
def test_replica_profile_accepts_exactly_two_healthy_api_instances(
	tmp_path: pathlib.Path,
) -> None:
	"""The complete fixed replica topology is ready with two healthy APIs."""
	target = live_demo_target(
		tmp_path, local_stack_control.models.LiveDemoProfile.REPLICA_RESTART
	)
	report = local_stack_control.status.build_target_report(
		target, replica_readiness_snapshot(2)
	)
	api = next(service for service in report.services if service.service == "api")

	assert report.ok and api.instances == 2 and api.healthy


#============================================
@pytest.mark.parametrize(
	("api_instances", "expected_state", "expected_service_state"),
	((1, "partially-active", "missing"), (3, "failed", "ambiguous")),
)
def test_replica_profile_rejects_api_cardinality_below_or_above_two(
	tmp_path: pathlib.Path,
	api_instances: int,
	expected_state: str,
	expected_service_state: str,
) -> None:
	"""Replica readiness distinguishes missing and unexpected extra API instances."""
	target = live_demo_target(
		tmp_path, local_stack_control.models.LiveDemoProfile.REPLICA_RESTART
	)
	report = local_stack_control.status.build_target_report(
		target, replica_readiness_snapshot(api_instances)
	)
	api = next(service for service in report.services if service.service == "api")

	assert report.state == expected_state and api.state == expected_service_state


#============================================
def test_replica_profile_requires_every_api_instance_healthy(
	tmp_path: pathlib.Path,
) -> None:
	"""One unhealthy API keeps an exact two-instance observation unready."""
	target = live_demo_target(
		tmp_path, local_stack_control.models.LiveDemoProfile.REPLICA_RESTART
	)
	report = local_stack_control.status.build_target_report(
		target, replica_readiness_snapshot(2, unhealthy_api=1)
	)
	api = next(service for service in report.services if service.service == "api")

	assert not report.ok and not api.healthy


#============================================
def test_replica_profile_still_requires_one_postgres_instance(
	tmp_path: pathlib.Path,
) -> None:
	"""The API exception does not weaken duplicate protection for PostgreSQL."""
	target = live_demo_target(
		tmp_path, local_stack_control.models.LiveDemoProfile.REPLICA_RESTART
	)
	report = local_stack_control.status.build_target_report(
		target, replica_readiness_snapshot(2, postgres_instances=2)
	)
	postgres = next(
		service for service in report.services if service.service == "postgres"
	)

	assert report.state == "failed" and postgres.state == "ambiguous"


#============================================
def test_restart_rejects_storage_service_without_a_process(tmp_path: pathlib.Path) -> None:
	"""Restart rejects persistent storage before examining or changing the stack."""
	target = lifecycle_target(tmp_path, "containers", "containers/env.local")
	options = local_stack_control.lifecycle.LifecycleOptions(1.0, False, False, False)
	with pytest.raises(local_stack_control.models.ControllerError):
		local_stack_control.lifecycle.restart_lifecycle(target, UnexpectedRunner(), tmp_path, "postgres", options)
	assert not target.env_file.exists()


#============================================
def test_smtp_delivery_restart_refreshes_credential_copy_before_recreate(
	tmp_path: pathlib.Path,
	monkeypatch: pytest.MonkeyPatch,
) -> None:
	"""A delivery-worker restart sees a completed fresh SMTP credential copy."""
	target = dataclasses.replace(lifecycle_target(tmp_path, "containers", "containers/env.local"), with_smtp=True)
	options = local_stack_control.lifecycle.LifecycleOptions(1.0, False, False, False)
	events: list[str] = []
	values = {"PLE_WEBWORK_RENDERER_IMAGE": "localhost/renderer:tag"}

	monkeypatch.setattr(local_stack_control.env_file, "require_mutation_env_file", lambda path: None)
	monkeypatch.setattr(local_stack_control.lifecycle, "validate_static", lambda target: values)
	monkeypatch.setattr(local_stack_control.lifecycle_validation, "require_mutation_engine", lambda *args: None)
	monkeypatch.setattr(local_stack_control.lifecycle, "require_restart_baseline", lambda *args: None)
	monkeypatch.setattr(local_stack_control.lifecycle, "child_environment", lambda target: {})
	monkeypatch.setattr(local_stack_control.renderer, "inspect_renderer_oci_id", lambda *args: "sha256:" + "a" * 64)
	monkeypatch.setattr(local_stack_control.lifecycle, "require_attested_running_renderer", lambda *args: None)
	monkeypatch.setattr(local_stack_control.lifecycle, "run_smtp_initializer", lambda *args: events.append("smtp-copy"))
	monkeypatch.setattr(local_stack_control.lifecycle, "compose_run", lambda target, runner, arguments: events.append("delivery-recreated"))
	monkeypatch.setattr(local_stack_control.lifecycle, "wait_for_complete_ready", lambda *args: events.append("ready") or "http://127.0.0.1:8080/")

	local_stack_control.lifecycle.restart_lifecycle(
		target,
		UnexpectedRunner(),
		tmp_path,
		"invitation-delivery-worker",
		options,
	)

	assert events.index("smtp-copy") < events.index("delivery-recreated") < events.index("ready")


#============================================
def test_replica_api_restart_preserves_scale_and_typed_readiness(
	tmp_path: pathlib.Path,
	monkeypatch: pytest.MonkeyPatch,
) -> None:
	"""A fixed replica restart recreates two APIs and retains its profile for readiness."""
	target = live_demo_target(
		tmp_path, local_stack_control.models.LiveDemoProfile.REPLICA_RESTART
	)
	options = local_stack_control.lifecycle.LifecycleOptions(1.0, False, False, False)
	compose_arguments: list[list[str]] = []
	readiness_targets: list[
		local_stack_control.models.ComposeTarget
		| local_stack_control.models.DisposableComposeTarget
	] = []
	values = {"PLE_WEBWORK_RENDERER_IMAGE": "localhost/renderer:tag"}

	monkeypatch.setattr(local_stack_control.lifecycle, "require_disposable_ownership", lambda target: None)
	monkeypatch.setattr(local_stack_control.env_file, "require_mutation_env_file", lambda path: None)
	monkeypatch.setattr(local_stack_control.lifecycle, "validate_static", lambda target: values)
	monkeypatch.setattr(local_stack_control.lifecycle_validation, "require_mutation_engine", lambda *args: None)
	monkeypatch.setattr(local_stack_control.lifecycle, "require_restart_baseline", lambda *args: None)
	monkeypatch.setattr(local_stack_control.lifecycle, "child_environment", lambda target: {})
	monkeypatch.setattr(local_stack_control.renderer, "inspect_renderer_oci_id", lambda *args: "sha256:" + "a" * 64)
	monkeypatch.setattr(local_stack_control.lifecycle, "require_attested_running_renderer", lambda *args: None)
	monkeypatch.setattr(local_stack_control.lifecycle, "probe_renderer", lambda *args: None)
	monkeypatch.setattr(local_stack_control.lifecycle, "run_api_initializers", lambda *args: None)
	monkeypatch.setattr(
		local_stack_control.lifecycle,
		"compose_run",
		lambda selected, runner, arguments: compose_arguments.append(arguments),
	)
	monkeypatch.setattr(
		local_stack_control.lifecycle,
		"wait_for_complete_ready",
		lambda selected, runner, selected_options: (
			readiness_targets.append(selected) or "https://localhost:55001/"
		),
	)

	local_stack_control.lifecycle.restart_lifecycle(
		target, UnexpectedRunner(), tmp_path, "api", options
	)

	assert compose_arguments == [[
		"up", "-d", "--force-recreate", "--no-deps",
		"--scale", "api=2", "api",
	]]
	assert readiness_targets == [target]


#============================================
def test_webwork_profile_renderer_restart_arguments_have_no_scale(
	tmp_path: pathlib.Path,
) -> None:
	"""The WebWork profile recreates its singleton renderer without scaling."""
	target = live_demo_target(
		tmp_path, local_stack_control.models.LiveDemoProfile.WEBWORK_RENDER_RPC
	)
	arguments = local_stack_control.lifecycle_profiles.recreate_arguments(
		target, "webwork-renderer"
	)

	assert arguments == [
		"up", "-d", "--force-recreate", "--no-deps", "webwork-renderer",
	]


#============================================
def restart_status(service: str, *, healthy: bool = True, instances: int = 1) -> local_stack_control.models.StackServiceStatus:
	"""Build one semantic restart-baseline observation without an engine fixture."""
	return local_stack_control.models.StackServiceStatus(
		service=service,
		instances=instances,
		present=instances > 0,
		running=healthy,
		healthy=healthy,
		complete=healthy if service in local_stack_control.models.BASE_ONE_SHOT_SERVICES else False,
		state="running" if healthy else ("ambiguous" if instances > 1 else "exited"),
		health="healthy" if healthy else None,
		exit_code=None if healthy else 137,
	)


#============================================
def restart_report(*statuses: local_stack_control.models.StackServiceStatus) -> local_stack_control.models.StatusReport:
	"""Build a status report for deterministic recovery-policy tests."""
	return local_stack_control.models.StatusReport(
		project="containers",
		with_smtp=False,
		snapshot=local_stack_control.models.ProjectSnapshot("containers", (), (), ()),
		services=statuses,
		ok=False,
		state="failed",
		message="renderer recovery is required",
	)


#============================================
def renderer_report(
	*,
	state: str,
	health: str | None,
	exit_code: int | None,
	image_id: str = "sha256:" + "a" * 64,
) -> local_stack_control.models.StatusReport:
	"""Build one label-derived renderer observation for readiness behavior."""
	container = local_stack_control.models.ContainerResource(
		id="renderer", names=("renderer",), project="containers",
		service="webwork-renderer", state=state, running=state == "running",
		exit_code=exit_code, health=health, image="localhost/renderer:tag",
		ports=(), image_id=image_id,
	)
	snapshot = local_stack_control.models.ProjectSnapshot("containers", (container,), (), ())
	return local_stack_control.status.build_report("containers", False, snapshot)


#============================================
def test_renderer_wait_accepts_a_healthy_selected_container_after_starting(
	tmp_path: pathlib.Path,
) -> None:
	"""Renderer startup polls its own health without depending on later services."""
	target = lifecycle_target(tmp_path, "containers", "containers/env.local")
	options = local_stack_control.lifecycle.LifecycleOptions(1.0, False, False, False)
	oci_id = "sha256:" + "a" * 64
	reports = iter((
		renderer_report(state="running", health="starting", exit_code=None),
		renderer_report(state="running", health="healthy", exit_code=None),
	))

	def poll_until_healthy(
		read_report: local_stack_control.lifecycle.StatusRead,
		timeout_seconds: float,
	) -> local_stack_control.models.StatusReport:
		starting = read_report()
		assert starting.state == "starting"
		return read_report()

	local_stack_control.lifecycle.wait_for_renderer_ready(
		target, UnexpectedRunner(), options, oci_id,
		read_status=lambda: next(reports), poll_ready=poll_until_healthy,
	)


#============================================
@pytest.mark.parametrize("exit_code", (0, 1))
def test_renderer_readiness_rejects_terminal_exit_before_probe(exit_code: int) -> None:
	"""A terminal renderer exit fails immediately instead of becoming a probe race."""
	with pytest.raises(local_stack_control.models.ControllerError):
		local_stack_control.lifecycle.renderer_readiness_report(
			renderer_report(state="exited", health=None, exit_code=exit_code),
			"sha256:" + "a" * 64,
		)


#============================================
def test_renderer_readiness_rejects_duplicate_labelled_containers() -> None:
	"""A renderer proof never selects one instance from an ambiguous service."""
	container = local_stack_control.models.ContainerResource(
		id="renderer-a", names=("renderer-a",), project="containers",
		service="webwork-renderer", state="running", running=True, exit_code=None,
		health="healthy", image="localhost/renderer:tag", ports=(),
		image_id="sha256:" + "a" * 64,
	)
	duplicate = dataclasses.replace(container, id="renderer-b", names=("renderer-b",))
	snapshot = local_stack_control.models.ProjectSnapshot("containers", (container, duplicate), (), ())
	report = local_stack_control.status.build_report("containers", False, snapshot)
	with pytest.raises(local_stack_control.models.ControllerError):
		local_stack_control.lifecycle.renderer_readiness_report(report, "sha256:" + "a" * 64)


#============================================
def complete_restart_statuses(
	selected: str,
	*,
	selected_healthy: bool,
	instances: int = 1,
) -> tuple[local_stack_control.models.StackServiceStatus, ...]:
	"""Return the declared topology with only the selected restart service variable."""
	services = local_stack_control.status.required_one_shots(False) + local_stack_control.status.required_long_running(False)
	return tuple(
		restart_status(
			service,
			healthy=selected_healthy if service == selected else True,
			instances=instances if service == selected else 1,
		)
		for service in services
	)


#============================================
def test_restart_baseline_allows_selected_renderer_recovery() -> None:
	"""A stopped selected renderer remains recoverable when every dependency is healthy."""
	report = restart_report(*complete_restart_statuses("webwork-renderer", selected_healthy=False))
	local_stack_control.lifecycle.require_restart_report(report, "webwork-renderer")


#============================================
def test_restart_baseline_refuses_an_unrelated_unhealthy_service() -> None:
	"""Renderer recovery does not conceal a separate required-service failure."""
	statuses = list(complete_restart_statuses("webwork-renderer", selected_healthy=False))
	statuses[-2] = restart_status("worker", healthy=False)
	with pytest.raises(local_stack_control.models.ControllerError):
		local_stack_control.lifecycle.require_restart_report(
			restart_report(*statuses), "webwork-renderer"
		)


#============================================
def test_restart_baseline_refuses_duplicate_selected_service() -> None:
	"""A selected restart service must still resolve to exactly one labelled instance."""
	report = restart_report(*complete_restart_statuses(
		"webwork-renderer", selected_healthy=False, instances=2,
	))
	with pytest.raises(local_stack_control.models.ControllerError):
		local_stack_control.lifecycle.require_restart_report(report, "webwork-renderer")


#============================================
def test_renderer_restart_checks_existing_provenance_before_recreate(
	tmp_path: pathlib.Path,
	monkeypatch: pytest.MonkeyPatch,
) -> None:
	"""Renderer recovery keeps its preexisting image attestation ahead of the mutation boundary."""
	target = lifecycle_target(tmp_path, "containers", "containers/env.local")
	options = local_stack_control.lifecycle.LifecycleOptions(1.0, False, False, False)
	events: list[str] = []
	compose_arguments: list[list[str]] = []
	values = {"PLE_WEBWORK_RENDERER_IMAGE": "localhost/renderer:tag"}

	def mark_compose(
		selected: local_stack_control.models.ComposeTarget,
		runner: local_stack_control.process.CommandRunner,
		arguments: list[str],
	) -> None:
		del selected, runner
		compose_arguments.append(arguments)
		events.append("recreate")

	monkeypatch.setattr(local_stack_control.env_file, "require_mutation_env_file", lambda path: None)
	monkeypatch.setattr(local_stack_control.lifecycle, "validate_static", lambda selected: values)
	monkeypatch.setattr(local_stack_control.lifecycle_validation, "require_mutation_engine", lambda runner, root, start: None)
	monkeypatch.setattr(local_stack_control.lifecycle, "require_restart_baseline", lambda selected, runner, service: events.append("baseline"))
	monkeypatch.setattr(local_stack_control.lifecycle, "child_environment", lambda selected: {})
	monkeypatch.setattr(local_stack_control.renderer, "inspect_renderer_oci_id", lambda runner, root, reference, environment: events.append("image") or "sha256:" + "a" * 64)
	monkeypatch.setattr(local_stack_control.lifecycle, "require_renderer_restart_provenance", lambda selected, selected_values, oci_id: events.append("provenance"))
	monkeypatch.setattr(local_stack_control.lifecycle, "compose_run", mark_compose)
	monkeypatch.setattr(local_stack_control.lifecycle, "wait_for_renderer_ready", lambda selected, runner, selected_options, oci_id: events.append("renderer-ready"))
	monkeypatch.setattr(local_stack_control.lifecycle, "attest_renderer", lambda selected, runner, root, selected_values, oci_id: events.append("attest"))
	monkeypatch.setattr(local_stack_control.lifecycle, "wait_for_complete_ready", lambda selected, runner, selected_options: events.append("ready") or "http://127.0.0.1:8080/")

	local_stack_control.lifecycle.restart_lifecycle(
		target, UnexpectedRunner(), tmp_path, "webwork-renderer", options
	)
	assert events.index("provenance") < events.index("recreate") < events.index("renderer-ready") < events.index("attest")
	assert compose_arguments == [[
		"up", "-d", "--force-recreate", "--no-deps", "webwork-renderer",
	]]


#============================================
def test_renderer_provenance_is_replaceable_private_attestation(tmp_path: pathlib.Path) -> None:
	"""A new renderer proof atomically replaces a previous valid local provenance record."""
	first = local_stack_control.models.RendererProvenance("localhost/renderer:one", "sha256:" + "a" * 64)
	second = local_stack_control.models.RendererProvenance("localhost/renderer:two", "sha256:" + "b" * 64)
	local_stack_control.renderer.write_provenance(tmp_path, first)
	local_stack_control.renderer.write_provenance(tmp_path, second)
	assert local_stack_control.renderer.load_provenance(tmp_path) == second


#============================================
def test_closed_teaching_owner_is_a_teaching_profile_but_custom_target_is_not(
	tmp_path: pathlib.Path,
) -> None:
	"""Only the declared disposable teaching owner receives bootstrap authority."""
	target = lifecycle_target(tmp_path, "walk", "walk.env")
	disposable = local_stack_control.models.DisposableComposeTarget(
		target=target, owner_policy="live-demo-baseline", capability_file=tmp_path / "capability",
		project_prefix="ple_live_demo_baseline_", private_environment_file=target.env_file,
	)
	assert local_stack_control.lifecycle_profiles.uses_local_teaching_state(disposable)
	assert not local_stack_control.lifecycle_profiles.uses_local_teaching_state(target)


#============================================
def test_live_teaching_bootstrap_keeps_seed_inputs_without_local_auth_files(
	tmp_path: pathlib.Path,
) -> None:
	"""The TLS owner creates seed inputs without introducing local-file credentials."""
	target = lifecycle_target(tmp_path, "ple-live-demo-browser", "live/env.local")
	target.env_file.parent.mkdir()
	target.env_file.write_text("\n", encoding="ascii")
	target.env_file.chmod(0o600)
	disposable = local_stack_control.models.DisposableComposeTarget(
		target=target,
		owner_policy="live-demo-browser",
		capability_file=tmp_path / "capability",
		project_prefix="ple-live-demo-browser",
		private_environment_file=target.env_file,
		live_demo_profile=local_stack_control.models.LiveDemoProfile.BROWSER,
	)

	local_stack_control.lifecycle.bootstrap_default_state(disposable, GatewayPortRunner((), False))
	values = local_stack_control.env_file.env_settings(target.env_file)
	secret_directory = target.env_file.parent / ".secrets"
	invitation_path = secret_directory / "invitation_token_secret"
	question_path = secret_directory / "question_id_secret"

	assert "PLE_LOCAL_AUTH_HOST_FILE" not in values
	assert not (target.env_file.parent / "local-login.txt").exists()
	assert not (target.env_file.parent / "local-identities.json").exists()
	assert invitation_path.is_file() and question_path.is_file()


#============================================
def test_busy_default_port_selects_first_free_teaching_port_or_keeps_running_gateway(tmp_path: pathlib.Path) -> None:
	"""First startup avoids an unrelated 8080 listener while retaining its own active gateway."""
	target = lifecycle_target(tmp_path, "containers", "containers/env.local")
	values = {"PLE_GATEWAY_HOST_PORT": "8080"}
	available = GatewayPortRunner(("8080",), False)
	running = GatewayPortRunner(("8080",), True)
	assert local_stack_control.lifecycle.choose_default_gateway_port(target, values, available) == "8000"
	assert local_stack_control.lifecycle.choose_default_gateway_port(target, values, running) == "8080"
	custom = lifecycle_target(
		tmp_path,
		"custom-project",
		"walk/env.local",
	)
	with pytest.raises(local_stack_control.models.ControllerError):
		local_stack_control.lifecycle.choose_default_gateway_port(custom, values, available)


#============================================
def test_default_environment_symlink_is_not_a_bootstrap_target(tmp_path: pathlib.Path) -> None:
	"""Lexical default-path matching rejects a symlink before a private bootstrap can follow it."""
	default_path = tmp_path / "containers/env.local"
	default_path.parent.mkdir()
	other = tmp_path / "other.env"
	other.write_text("SAFE=value\n", encoding="ascii")
	default_path.symlink_to(other)
	assert not local_stack_control.local_environment.is_default_local_environment(tmp_path, default_path)


#============================================
def test_compose_failure_retains_redacted_bounded_child_diagnostics(tmp_path: pathlib.Path) -> None:
	"""Compose failures retain the useful tail only after exact private-value redaction."""
	target = lifecycle_target(tmp_path, "containers", "containers/env.local")
	target.env_file.parent.mkdir()
	target.env_file.write_text("PLE_TEST_SECRET=private-value\n", encoding="ascii")
	target.env_file.chmod(0o600)
	result = local_stack_control.models.CommandResult(
		("podman", "compose"), 1, "x" * 400 + "stdout useful private-value", "stderr useful private-value",
	)
	detail = local_stack_control.lifecycle_diagnostics.redacted_failure_detail(
		result, ("private-value",)
	)
	assert len(detail) == local_stack_control.lifecycle_diagnostics.MAXIMUM_DIAGNOSTIC_CHARACTERS
	assert "stdout useful [private]" in detail
	class FailureRunner(UnexpectedRunner):
		def run(self, argv: list[str], environment: dict[str, str] | None = None, cwd: pathlib.Path | None = None, stdin: str | None = None) -> local_stack_control.models.CommandResult:
			return result
	with pytest.raises(local_stack_control.models.ControllerError) as error:
		local_stack_control.lifecycle.compose_run(target, FailureRunner(), ["up", "-d"])
	message = str(error.value)
	assert "stderr useful [private]" in message
	assert "private-value" not in message
	assert len(message) <= 512


#============================================
def test_unspecified_private_values_keep_failure_detail_generic() -> None:
	"""Non-Compose callers retain the safe generic message without a redaction authority."""
	result = local_stack_control.models.CommandResult(("command",), 1, "useful child output", "useful child error")
	with pytest.raises(local_stack_control.models.ControllerError) as error:
		local_stack_control.lifecycle.require_command(result, "other operation")
	assert "child reported a failure" in str(error.value)
	assert "useful child" not in str(error.value)


#============================================
def test_teaching_environment_paths_follow_the_selected_private_environment(tmp_path: pathlib.Path) -> None:
	"""Teaching-profile defaults keep secrets and identity projection beside its selected env file."""
	target = lifecycle_target(
		tmp_path,
		"ple_live_demo_baseline_test",
		"walk/env.local",
	)
	target.env_file.parent.mkdir()
	target.env_file.write_text("PLE_GATEWAY_HOST_PORT=8123\n", encoding="ascii")
	target.env_file.chmod(0o600)
	local_stack_control.lifecycle.configure_default_environment(target, None)
	values = target.env_file.read_text(encoding="ascii")
	assert str(target.env_file.parent / ".secrets/invitation_token_secret") in values
	assert "PLE_LOCAL_AUTH_HOST_FILE" not in values

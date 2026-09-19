"""Renderer wait and stateless restart contracts for the local stack."""

import dataclasses
import pathlib

import pytest

import local_stack_control.lifecycle
import local_stack_control.lifecycle_profiles
import local_stack_control.lifecycle_validation
import local_stack_control.models
import local_stack_control.process
import local_stack_control.renderer
import local_stack_control.env_file
import local_stack_control.status
import local_stack_lifecycle_helpers


UnexpectedRunner = local_stack_lifecycle_helpers.UnexpectedRunner
lifecycle_target = local_stack_lifecycle_helpers.lifecycle_target
live_demo_target = local_stack_lifecycle_helpers.live_demo_target


#============================================
def test_restart_rejects_storage_service_without_a_process(tmp_path: pathlib.Path) -> None:
	"""Restart rejects persistent storage before examining or changing the stack."""
	target = lifecycle_target(tmp_path, "containers", "containers/env.local")
	options = local_stack_control.lifecycle.LifecycleOptions(1.0, False, False, False)
	with pytest.raises(local_stack_control.models.ControllerError):
		local_stack_control.lifecycle.restart_lifecycle(target, UnexpectedRunner(), tmp_path, "postgres", options)
	assert not target.env_file.exists()


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
	statuses[-2] = restart_status("api", healthy=False)
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
def test_renderer_restart_checks_current_version_before_recreate(
	tmp_path: pathlib.Path,
	monkeypatch: pytest.MonkeyPatch,
) -> None:
	"""Renderer recovery keeps its current version proof ahead of the mutation boundary."""
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
	monkeypatch.setattr(local_stack_control.lifecycle, "require_question_renderer_version", lambda selected, selected_values, oci_id: events.append("version"))
	monkeypatch.setattr(local_stack_control.lifecycle, "compose_run", mark_compose)
	monkeypatch.setattr(local_stack_control.lifecycle, "wait_for_renderer_ready", lambda selected, runner, selected_options, oci_id: events.append("renderer-ready"))
	monkeypatch.setattr(local_stack_control.lifecycle, "attest_renderer", lambda selected, runner, root, selected_values, oci_id: events.append("attest"))
	monkeypatch.setattr(local_stack_control.lifecycle, "wait_for_complete_ready", lambda selected, runner, selected_options: events.append("ready") or "http://127.0.0.1:8080/")

	local_stack_control.lifecycle.restart_lifecycle(
		target, UnexpectedRunner(), tmp_path, "webwork-renderer", options
	)
	assert events.index("version") < events.index("recreate") < events.index("renderer-ready") < events.index("attest")
	assert compose_arguments == [[
		"up", "-d", "--force-recreate", "--no-deps", "webwork-renderer",
	]]


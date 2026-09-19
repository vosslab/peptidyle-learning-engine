"""Renderer wait, attestation, and Question Renderer Version lifecycle helpers."""

import collections.abc
import dataclasses
import pathlib

import local_stack_control.lifecycle_commands
import local_stack_control.lifecycle_wait
import local_stack_control.models
import local_stack_control.process
import local_stack_control.renderer


StatusRead = collections.abc.Callable[[], local_stack_control.models.StatusReport]
ReadinessPoll = collections.abc.Callable[[StatusRead, float], local_stack_control.models.StatusReport]


#============================================
def lifecycle_status_report(
	target: local_stack_control.models.ComposeTarget,
	runner: local_stack_control.process.CommandRunner,
) -> local_stack_control.models.StatusReport:
	"""Read selected-stack status through the lifecycle facade after it has loaded."""
	import local_stack_control.lifecycle
	return local_stack_control.lifecycle.status_report(target, runner)


#============================================
def renderer_version_file_path(
	repo_root: pathlib.Path,
	value: str,
) -> pathlib.Path:
	"""Resolve a Question Renderer Version path through the lifecycle facade."""
	import local_stack_control.lifecycle
	return local_stack_control.lifecycle.absolute_value_path(repo_root, value)


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
	options: "local_stack_control.lifecycle.LifecycleOptions",
	oci_id: str,
	*,
	read_status: StatusRead | None = None,
	poll_ready: ReadinessPoll = local_stack_control.lifecycle_wait.poll_ready,
) -> None:
	"""Await the one selected healthy renderer before its behavior is probed."""
	status_reader = read_status
	if status_reader is None:
		def status_reader() -> local_stack_control.models.StatusReport:
			return lifecycle_status_report(target, runner)

	def read_report() -> local_stack_control.models.StatusReport:
		report = status_reader()
		return renderer_readiness_report(report, oci_id)

	poll_ready(read_report, options.timeout_seconds)


#============================================
def attest_renderer(
	target: local_stack_control.models.ComposeTarget,
	runner: local_stack_control.process.CommandRunner,
	repo_root: pathlib.Path,
	values: dict[str, str],
	oci_id: str,
) -> None:
	"""Prove renderer identity and behavior before replacing its private attestation."""
	require_running_renderer(target, runner, oci_id)
	probe_renderer(target, runner, repo_root, oci_id)
	write_question_renderer_version(target, values, oci_id)


#============================================
def probe_renderer(
	target: local_stack_control.models.ComposeTarget,
	runner: local_stack_control.process.CommandRunner,
	repo_root: pathlib.Path,
	oci_id: str,
) -> None:
	"""Exercise the exact selected renderer through its label-resolved container."""
	container = local_stack_control.renderer.require_running_renderer(
		lifecycle_status_report(target, runner), oci_id
	)
	probe = (repo_root / "containers/webwork/probe_render_api.sh").read_text(encoding="utf-8")
	result = runner.run(
		["podman", "exec", "-i", container.id, "bash", "-s", "--", "--exercise"],
		{
			name: value
			for name, value in local_stack_control.lifecycle_commands.child_environment(target).items()
			if name in ("PATH", "HOME")
		},
		repo_root,
		probe,
	)
	local_stack_control.lifecycle_commands.require_command(result, "renderer render and grade probe")


#============================================
def require_attested_running_renderer(
	target: local_stack_control.models.ComposeTarget,
	runner: local_stack_control.process.CommandRunner,
	values: dict[str, str],
	oci_id: str,
) -> None:
	"""Require the running renderer and its private preexisting OCI attestation."""
	require_running_renderer(target, runner, oci_id)
	require_question_renderer_version(target, values, oci_id)


#============================================
def require_running_renderer(
	target: local_stack_control.models.ComposeTarget,
	runner: local_stack_control.process.CommandRunner,
	oci_id: str,
) -> None:
	"""Require the single selected renderer to be healthy and image-matched."""
	local_stack_control.renderer.require_running_renderer(
		lifecycle_status_report(target, runner), oci_id
	)


#============================================
def question_renderer_version_directory(
	target: local_stack_control.models.ComposeTarget,
	values: dict[str, str],
) -> pathlib.Path:
	"""Resolve the fixed private Question Renderer Version directory."""
	version_path = renderer_version_file_path(
		target.repo_root, values["PLE_WEBWORK_RENDERER_VERSION_FILE"]
	)
	if version_path.name != local_stack_control.renderer.QUESTION_RENDERER_VERSION_NAME:
		raise local_stack_control.models.ControllerError(
			"selected Question Renderer Version path has an invalid name"
		)
	return version_path.parent


#============================================
def write_question_renderer_version(
	target: local_stack_control.models.ComposeTarget,
	values: dict[str, str],
	oci_id: str,
) -> None:
	"""Record the exact Question Renderer Version after a successful probe."""
	version = local_stack_control.models.QuestionRendererVersion(
		values["PLE_WEBWORK_RENDERER_IMAGE"], oci_id
	)
	local_stack_control.renderer.write_question_renderer_version(
		question_renderer_version_directory(target, values), version
	)


#============================================
def require_question_renderer_version(
	target: local_stack_control.models.ComposeTarget,
	values: dict[str, str],
	oci_id: str,
) -> None:
	"""Require the current Question Renderer Version before renderer recovery."""
	local_stack_control.renderer.require_question_renderer_version(
		question_renderer_version_directory(target, values), oci_id
	)

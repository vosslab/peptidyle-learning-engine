"""Fail-closed Podman ownership checks for one disposable walkthrough project."""

import pathlib
import shlex

import local_stack_control.discovery
import local_stack_control.models
import local_stack_control.process

import walklib.models


#============================================
def assert_no_stale_project_resources(
	project_name: str,
	repository_root: pathlib.Path,
	runner: local_stack_control.process.CommandRunner,
) -> None:
	"""Refuse a project claim when shared label discovery finds any owned resource."""
	try:
		snapshot = local_stack_control.discovery.discover_snapshot(
			runner, repository_root, project_name
		)
	except local_stack_control.models.ControllerError as error:
		raise walklib.models.RunnerError("cannot inspect walkthrough resources before E2E") from error
	if len(snapshot.containers) + len(snapshot.volumes) + len(snapshot.networks) > 0:
		raise walklib.models.RunnerError(
			"generated walkthrough project has stale labelled resources; retry the walkthrough"
		)


#============================================
def keep_instruction(
	project_name: str,
) -> str:
	"""Return one exact read-only command for inspecting a retained disposable project."""
	ps_arguments = [
		"podman", "ps", "--all", "--filter",
		f"label={local_stack_control.models.COMPOSE_PROJECT_LABELS[0]}={project_name}",
	]
	command = shlex.join(ps_arguments)
	message = (
		f"UI walkthrough: preserving generated project {project_name}; volumes are retained. "
		f"Inspect without changing it: {command}"
	)
	return message

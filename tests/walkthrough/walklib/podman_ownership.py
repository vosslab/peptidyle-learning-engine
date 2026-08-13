"""Fail-closed Podman ownership checks for one disposable walkthrough project."""

import shlex

import walklib.models


PROJECT_LABEL_KEYS = (
	"io.podman.compose.project",
	"com.docker.compose.project",
)
PROJECT_RESOURCE_COMMANDS = (
	("containers", ("podman", "ps", "--all", "--quiet")),
	("volumes", ("podman", "volume", "ls", "--quiet")),
	("networks", ("podman", "network", "ls", "--quiet")),
)


#============================================
def assert_no_stale_project_resources(
	project_name: str,
	run_command: walklib.models.CommandRunner,
) -> None:
	"""Refuse to claim a disposable project when any recognized labeled resource exists."""
	for resource_type, command_prefix in PROJECT_RESOURCE_COMMANDS:
		for label_key in PROJECT_LABEL_KEYS:
			label = f"{label_key}={project_name}"
			command = [*command_prefix, "--filter", f"label={label}"]
			result = run_command(command, None)
			if result.returncode != 0:
				raise walklib.models.RunnerError(
					f"cannot inspect walkthrough {resource_type} before E2E"
				)
			if result.stdout.strip():
				raise walklib.models.RunnerError(
					f"generated walkthrough project has stale {resource_type}; retry the walkthrough"
				)


#============================================
def keep_instruction(
	project_name: str,
) -> str:
	"""Return one exact read-only command for inspecting a retained disposable project."""
	ps_arguments = [
		"podman", "ps", "--all", "--filter",
		f"label=io.podman.compose.project={project_name}",
	]
	command = shlex.join(ps_arguments)
	message = (
		f"UI walkthrough: preserving generated project {project_name}; volumes are retained. "
		f"Inspect without changing it: {command}"
	)
	return message

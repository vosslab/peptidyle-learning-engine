"""Platform browser opener for proven loopback URLs."""

import pathlib

import local_stack_control.env_file
import local_stack_control.lifecycle_commands
import local_stack_control.process


#============================================
def open_browser(
	runner: local_stack_control.process.CommandRunner,
	repo_root: pathlib.Path,
	url: str,
) -> None:
	"""Open the proven loopback URL using an argument-array platform opener."""
	environment = local_stack_control.env_file.sanitized_runtime_environment(
		local_stack_control.process.current_environment()
	)
	result = runner.run(["open", url], environment, repo_root)
	if not result.ok():
		result = runner.run(["xdg-open", url], environment, repo_root)
		local_stack_control.lifecycle_commands.require_command(result, "browser open")

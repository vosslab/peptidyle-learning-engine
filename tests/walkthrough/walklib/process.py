"""Subprocess boundary for the UI walkthrough runner."""

import subprocess
import pathlib
import os

import local_stack_control.env_file
import local_stack_control.models
import local_stack_control.process

import tests.walkthrough.walklib as walklib
import tests.walkthrough.walklib.models as models


def command_result(
	command: list[str],
	environ: dict[str, str] | None,
	stdin: str | None = None,
) -> models.CommandResult:
	"""Run one argv-array command without a shell and capture its process result."""
	base_environment = dict(os.environ) if environ is None else environ
	effective_environment = local_stack_control.env_file.sanitized_runtime_environment(
		base_environment
	)
	completed = subprocess.run(
		command,
		env=effective_environment,
		text=True,
		capture_output=True,
		check=False,
		input=stdin,
	)
	result = walklib.models.CommandResult(
		completed.returncode,
		completed.stdout,
		completed.stderr,
	)
	return result


class WalkthroughControllerRunner(local_stack_control.process.CommandRunner):
	"""Adapt the walkthrough's injectable subprocess boundary to the controller API."""

	def __init__(self, run_command: walklib.models.CommandRunner) -> None:
		self.run_command = run_command

	#============================================
	def run(
		self,
		argv: list[str],
		environment: dict[str, str] | None = None,
		cwd: pathlib.Path | None = None,
		stdin: str | None = None,
	) -> local_stack_control.models.CommandResult:
		"""Capture one controller command through the walkthrough-owned boundary."""
		# The trusted runner changes to the repository root before lifecycle work.
		# The injectable boundary owns executable input; cwd remains controller metadata.
		del cwd
		base_environment = (
			local_stack_control.process.current_environment()
			if environment is None
			else environment
		)
		effective_environment = local_stack_control.env_file.sanitized_runtime_environment(
			base_environment
		)
		result = self.run_command(argv, effective_environment, stdin)
		adapted = local_stack_control.models.CommandResult(
			argv=tuple(argv),
			returncode=result.returncode,
			stdout=result.stdout,
			stderr=result.stderr,
		)
		return adapted

	#============================================
	def stream(
		self,
		argv: list[str],
		environment: dict[str, str] | None = None,
		cwd: pathlib.Path | None = None,
	) -> int:
		"""Run one controller-owned mutation through the same bounded runner."""
		result = self.run(argv, environment, cwd)
		return result.returncode

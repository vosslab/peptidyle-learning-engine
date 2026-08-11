"""Subprocess boundary for the UI walkthrough runner."""

import subprocess

import walklib.models


def command_result(
	command: list[str],
	environ: dict[str, str] | None,
) -> walklib.models.CommandResult:
	"""Run one argv-array command without a shell and capture its process result."""
	completed = subprocess.run(
		command,
		env=environ,
		text=True,
		capture_output=True,
		check=False,
	)
	result = walklib.models.CommandResult(
		completed.returncode,
		completed.stdout,
		completed.stderr,
	)
	return result

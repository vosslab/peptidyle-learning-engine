"""Redaction-aware child-command boundary for lifecycle orchestration."""

import pathlib

import local_stack_control.compose
import local_stack_control.disposable_stack_adapter
import local_stack_control.lifecycle_diagnostics
import local_stack_control.models
import local_stack_control.process


#============================================
def child_environment(target: local_stack_control.models.ComposeTarget) -> dict[str, str]:
	"""Build the selected environment authority for one lifecycle child process."""
	result = local_stack_control.compose.target_environment(
		target, local_stack_control.process.current_environment()
	)
	return result


#============================================
def require_command(
	result: local_stack_control.models.CommandResult,
	operation: str,
	private_values: tuple[str, ...] = (),
) -> None:
	"""Convert a child failure into bounded non-secret lifecycle guidance.

	The caller supplies only values deliberately authorized for redaction.  Command
	arguments remain structured arrays at the process boundary (ASVS 1.2.5).
	"""
	if not result.ok():
		detail = local_stack_control.lifecycle_diagnostics.redacted_failure_detail(
			result, private_values
		)
		raise local_stack_control.models.ControllerError(
			f"{operation} failed ({detail}); retained stack resources are available for diagnostics"
		)


#============================================
def validate_compose(
	target: local_stack_control.models.ComposeTarget,
	runner: local_stack_control.process.CommandRunner,
	repo_root: pathlib.Path,
) -> str:
	"""Validate selected Compose interpolation and return its rendered topology."""
	result = runner.run(
		local_stack_control.compose.compose_argv(target, ["config"]),
		child_environment(target),
		repo_root,
	)
	private_values = local_stack_control.disposable_stack_adapter.private_environment_values(
		target.env_file
	)
	require_command(result, "Compose configuration validation", private_values)
	return result.stdout


#============================================
def compose_run(
	target: local_stack_control.models.ComposeTarget,
	runner: local_stack_control.process.CommandRunner,
	arguments: list[str],
) -> None:
	"""Run one selected Compose operation with its private values redacted on failure."""
	result = runner.run(
		local_stack_control.compose.compose_argv(target, arguments),
		child_environment(target),
		target.repo_root,
	)
	private_values = local_stack_control.disposable_stack_adapter.private_environment_values(
		target.env_file
	)
	require_command(result, "selected Compose operation", private_values)

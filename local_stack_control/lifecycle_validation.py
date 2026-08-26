"""Typed, read-only lifecycle request and configuration validation."""

import dataclasses
import pathlib

import local_stack_control.compose
import local_stack_control.env_file
import local_stack_control.models
import local_stack_control.process
import local_stack_control.renderer


@dataclasses.dataclass(frozen=True)
class LifecycleRequest:
	"""Intent already parsed by the public controller."""

	target: local_stack_control.models.ComposeTarget
	release: bool
	skip_build: bool
	headless: bool
	mutation: bool


#============================================
def validate_request(request: LifecycleRequest) -> dict[str, str]:
	"""Validate selected configuration without starting an engine or Compose."""
	if request.skip_build and not request.mutation:
		raise local_stack_control.models.ControllerError(
			"--skip-build applies only to a start operation"
		)
	values = local_stack_control.env_file.env_settings(request.target.env_file)
	if request.mutation:
		local_stack_control.compose.require_default_mutation_target(request.target)
	renderer_reference = values.get("PLE_WEBWORK_RENDERER_IMAGE")
	if renderer_reference is None:
		raise local_stack_control.models.ControllerError(
			"selected environment does not declare PLE_WEBWORK_RENDERER_IMAGE"
		)
	local_stack_control.renderer.validate_renderer_reference(renderer_reference)
	return values


#============================================
def require_mutation_engine(
	runner: local_stack_control.process.CommandRunner,
	repo_root: pathlib.Path,
	start_selected_machine: bool = False,
) -> None:
	"""Prove a rootless default engine, optionally starting only its local machine."""
	environment = local_stack_control.env_file.sanitized_runtime_environment(
		local_stack_control.process.current_environment()
	)
	result = runner.run(["podman", "info", "--format", "json"], environment, repo_root)
	if not result.ok() and start_selected_machine:
		started = runner.run(["podman", "machine", "start"], environment, repo_root)
		if not started.ok():
			raise local_stack_control.models.ControllerError(
				"the selected local Podman machine could not be started"
			)
	local_stack_control.process.require_rootless_local_engine(runner, repo_root)

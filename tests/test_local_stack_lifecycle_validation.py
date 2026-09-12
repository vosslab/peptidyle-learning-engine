"""Local Stack lifecycle input-validation contracts."""

import pathlib

import pytest

import local_stack_control.lifecycle
import local_stack_control.models
import local_stack_control.process


class UnexpectedRunner(local_stack_control.process.CommandRunner):
	"""Reject every process when validating a pre-mutation boundary."""

	def run(self, argv: list[str], environment: dict[str, str] | None = None, cwd: pathlib.Path | None = None, stdin: str | None = None) -> local_stack_control.models.CommandResult:
		raise AssertionError(f"unexpected lifecycle command: {argv}")

	def stream(self, argv: list[str], environment: dict[str, str] | None = None, cwd: pathlib.Path | None = None) -> int:
		raise AssertionError("input validation does not stream commands")


def lifecycle_target(tmp_path: pathlib.Path, project: str, env_name: str) -> local_stack_control.models.ComposeTarget:
	"""Build one target without reading any tracked private environment."""
	return local_stack_control.models.ComposeTarget(
		repo_root=tmp_path, project=project, env_file=tmp_path / env_name,
		compose_files=(), provider=local_stack_control.models.ComposeProvider(("podman", "compose"), "podman compose"),
		with_smtp=False, env_setting_names=(),
	)


def test_validation_rejects_invalid_selected_env_before_any_process(tmp_path: pathlib.Path) -> None:
	"""Read-only validation refuses malformed selected configuration without effects."""
	target = lifecycle_target(tmp_path, "custom", "custom.env")
	target.env_file.write_text("PLE_WEBWORK_RENDERER_IMAGE=unsafe;image\n", encoding="ascii")
	target.env_file.chmod(0o600)
	with pytest.raises(local_stack_control.models.ControllerError):
		local_stack_control.lifecycle.validate_lifecycle(target, UnexpectedRunner(), tmp_path)
	assert target.env_file.exists()


def test_custom_start_refuses_missing_environment_before_engine_mutation(tmp_path: pathlib.Path) -> None:
	"""A custom target never inherits default bootstrap authority."""
	target = lifecycle_target(tmp_path, "custom", "custom.env")
	options = local_stack_control.lifecycle.LifecycleOptions(1.0, True, False, False)
	with pytest.raises(local_stack_control.models.ControllerError):
		local_stack_control.lifecycle.start_lifecycle(target, UnexpectedRunner(), tmp_path, options)
	assert not target.env_file.exists()

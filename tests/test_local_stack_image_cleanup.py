"""Behavioral tests for engine-wide pre-build unused-image pruning."""

import pathlib

import pytest

import local_stack_control.image_cleanup
import local_stack_control.models
import local_stack_control.process


class RecordingRunner(local_stack_control.process.CommandRunner):
	"""Record the fixed cleanup command without contacting Podman."""

	def __init__(self, returncode: int) -> None:
		self.returncode = returncode
		self.calls: list[list[str]] = []

	#============================================
	def run(
		self,
		argv: list[str],
		environment: dict[str, str] | None = None,
		cwd: pathlib.Path | None = None,
		stdin: str | None = None,
	) -> local_stack_control.models.CommandResult:
		"""Return one selected engine outcome."""
		del environment, cwd, stdin
		self.calls.append(argv)
		return local_stack_control.models.CommandResult(tuple(argv), self.returncode, "", "")

	#============================================
	def stream(
		self,
		argv: list[str],
		environment: dict[str, str] | None = None,
		cwd: pathlib.Path | None = None,
	) -> int:
		"""Reject unexpected streaming at the cleanup boundary."""
		del argv, environment, cwd
		raise AssertionError("cleanup must use the captured command boundary")


#============================================
@pytest.mark.parametrize("returncode", [0, 1, 2, 7])
def test_prebuild_prune_uses_exact_command_and_blocks_on_failure(
	tmp_path: pathlib.Path, returncode: int,
) -> None:
	"""Pruning removes only dangling layers, keeps every tagged image, and never tolerates failure.

	`-a` would also delete the reviewed renderer, pulled service images, and any of the
	operator's other tagged images whose containers are stopped, turning every start into
	a cold build.
	"""
	runner = RecordingRunner(returncode)
	if returncode:
		with pytest.raises(local_stack_control.models.ControllerError, match="pruning failed"):
			local_stack_control.image_cleanup.remove_obsolete_images_before_build(runner, tmp_path)
	else:
		local_stack_control.image_cleanup.remove_obsolete_images_before_build(runner, tmp_path)
	assert runner.calls == [["podman", "image", "prune", "-f"]]


#============================================
def test_image_build_lease_rejects_overlapping_transactions_and_releases(tmp_path: pathlib.Path) -> None:
	"""Use real host directory flock behavior without a replaceable lockfile."""
	(tmp_path / "containers").mkdir()
	with local_stack_control.image_cleanup.image_build_lease(tmp_path):
		with pytest.raises(local_stack_control.models.ControllerError, match="active"):
			with local_stack_control.image_cleanup.image_build_lease(tmp_path):
				raise AssertionError("overlapping transaction entered")
	with local_stack_control.image_cleanup.image_build_lease(tmp_path):
		pass

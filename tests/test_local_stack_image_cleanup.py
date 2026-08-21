"""Behavioral tests for ordinary-stack unused-image cleanup."""

import pathlib

import pytest

import local_stack_control.image_cleanup
import local_stack_control.models
import local_stack_control.process


class RecordingRunner(local_stack_control.process.CommandRunner):
	"""Record image cleanup commands and return selected results."""

	def __init__(
		self,
		results: tuple[tuple[int, str], ...],
	) -> None:
		"""Store deterministic subprocess outcomes."""
		self.results = iter(results)
		self.calls: list[list[str]] = []

	#============================================
	def run(
		self,
		argv: list[str],
		environment: dict[str, str] | None = None,
		cwd: pathlib.Path | None = None,
		stdin: str | None = None,
	) -> local_stack_control.models.CommandResult:
		"""Record one command without contacting Podman."""
		del environment, cwd, stdin
		self.calls.append(argv)
		return_code, stdout = next(self.results)
		result = local_stack_control.models.CommandResult(
			tuple(argv), return_code, stdout, ""
		)
		return result

	#============================================
	def stream(
		self,
		argv: list[str],
		environment: dict[str, str] | None = None,
		cwd: pathlib.Path | None = None,
	) -> int:
		"""Reject streaming because cleanup must remain captured."""
		del argv, environment, cwd
		raise AssertionError("image cleanup must not stream a subprocess")


#============================================
def test_reviewed_local_stack_prunes_every_unused_image(
	tmp_path: pathlib.Path,
) -> None:
	"""Inactive disposable tags are removed before the pre-approved prune."""
	runner = RecordingRunner(
		(
			(
				0,
				'[{"Image":"localhost/ple-ui-walkthrough-1111111111111111_gateway:latest"}]',
			),
			(
				0,
				"""[
					{"Names":["localhost/containers_gateway:latest",
					"localhost/ple-ui-walkthrough-1111111111111111_gateway:latest",
					"localhost/ple-ui-walkthrough-2222222222222222_gateway:latest"]},
					{"Names":["localhost/peptidyle-learning-engine:ple-webwork-browser-333333333333"]},
					{"Repository":"<none>","Tag":"<none>"},
					{"Names":["docker.io/library/postgres:17"]}
				]""",
			),
			(0, ""),
			(0, ""),
			(0, ""),
		)
	)
	local_stack_control.image_cleanup.prune_superseded_images(runner, tmp_path)
	assert runner.calls == [
		["podman", "ps", "--all", "--format", "json"],
		["podman", "images", "--all", "--format", "json"],
		[
			"podman",
			"image",
			"rm",
			"localhost/peptidyle-learning-engine:ple-webwork-browser-333333333333",
		],
		[
			"podman",
			"image",
			"rm",
			"localhost/ple-ui-walkthrough-2222222222222222_gateway:latest",
		],
		["podman", "image", "prune", "--all", "--force"],
	]


#============================================
def test_owned_image_cleanup_failure_fails_the_ready_lifecycle(
	tmp_path: pathlib.Path,
) -> None:
	"""Cleanup failure is visible instead of silently accumulating replaced images."""
	runner = RecordingRunner(((0, "[]"), (0, "[]"), (7, "")))
	with pytest.raises(local_stack_control.models.ControllerError):
		local_stack_control.image_cleanup.prune_superseded_images(runner, tmp_path)


#============================================
def test_image_cleanup_refuses_unexpected_podman_inventory(
	tmp_path: pathlib.Path,
) -> None:
	"""A malformed inventory cannot broaden or partially execute cleanup."""
	runner = RecordingRunner(((0, "not-json"),))
	with pytest.raises(local_stack_control.models.ControllerError):
		local_stack_control.image_cleanup.prune_superseded_images(runner, tmp_path)
	assert runner.calls == [["podman", "ps", "--all", "--format", "json"]]

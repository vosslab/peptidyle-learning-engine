"""Offline cleanup behavior for the explicit UI walkthrough runner."""

import importlib
import json
import pathlib
import sys


WALKTHROUGH_DIRECTORY = pathlib.Path(__file__).resolve().parent / "walkthrough"
sys.path.insert(0, str(WALKTHROUGH_DIRECTORY))
walkthrough = importlib.import_module("walklib.runner")


class CleanupCommands:
	"""Return a configured cleanup result without launching Podman."""

	def __init__(self, cleanup_code: int = 0) -> None:
		self.cleanup_code = cleanup_code

	def __call__(self, command: list[str], _environ: dict[str, str] | None) -> object:
		if command[-2:] == ["down", "--remove-orphans"]:
			return walkthrough.CommandResult(self.cleanup_code, "", "cleanup failure")
		return walkthrough.CommandResult(0, "", "")


def runner_for(tmp_path: pathlib.Path, commands: CleanupCommands) -> object:
	"""Build one runner with its selected explicit environment file."""
	repository = tmp_path / "repository"
	env_file = repository / "containers/env.local"
	env_file.parent.mkdir(parents=True)
	env_file.write_text("PLE_GATEWAY_HOST_PORT=3010\n", encoding="ascii")
	inputs = walkthrough.resolve_inputs(walkthrough.parse_args(["--master-seed", "42"]), repository)
	return walkthrough.WalkthroughRunner(inputs, repository, {}, commands)


def test_finish_removes_private_state_and_does_not_publish_secret(tmp_path: pathlib.Path) -> None:
	"""A successful lifecycle removes its private handoff before publishing the receipt."""
	runner = runner_for(tmp_path, CleanupCommands())
	runner.prepare_report_directory()
	runner.prepare_journey_state()
	private_directory = runner.private_state_directory
	secret = "student=private"
	runner.write_private_child_inputs(
		walkthrough.walklib.models.ArrangementChildInputs(tmp_path / f"{secret}-manifest.json")
	)

	assert runner.finish(True) == 0
	assert private_directory is not None and not private_directory.exists()
	assert secret not in runner.report_path.read_text(encoding="ascii")


def test_cleanup_failure_cannot_return_a_passing_receipt(tmp_path: pathlib.Path) -> None:
	"""A failed Podman cleanup changes the visible receipt and process status to failure."""
	runner = runner_for(tmp_path, CleanupCommands(7))
	runner.compose_command = ["podman", "compose"]
	runner.stack_launch_attempted = True
	runner.prepare_report_directory()

	assert runner.finish(True) == 1
	assert json.loads(runner.report_path.read_text(encoding="ascii"))["status"] == "FAIL"


def test_j2_failure_receipt_keeps_only_the_last_safe_visible_stage(tmp_path: pathlib.Path) -> None:
	"""A failed J2 child reports progress without retaining its browser output or inputs."""
	runner = runner_for(tmp_path, CleanupCommands())
	runner.prepare_report_directory()
	runner.prepare_journey_state()
	checkpoint = runner.j2_checkpoint_file
	assert checkpoint is not None
	checkpoint.write_text("feedback_visible\n", encoding="ascii")
	runner.report_stage = "playwright_j2"

	assert runner.finish(False) == 1
	receipt = json.loads(runner.report_path.read_text(encoding="ascii"))
	assert receipt["j2Checkpoint"] == "feedback_visible"

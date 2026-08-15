"""Offline cleanup behavior for the explicit UI walkthrough runner."""

import json
import pathlib

import pytest

import tests.walkthrough.walklib.models as walkthrough_models
import tests.walkthrough.walklib.runner as walkthrough



class CleanupCommands:
	"""Return a configured cleanup result without launching Podman."""

	def __init__(
		self,
		cleanup_code: int = 0,
		image_cleanup_code: int = 0,
		missing_image: str = "",
		with_owned_volume: bool = False,
	) -> None:
		self.cleanup_code = cleanup_code
		self.image_cleanup_code = image_cleanup_code
		self.missing_image = missing_image
		self.with_owned_volume = with_owned_volume
		self.project = ""
		self.capability_digest = ""
		self.calls: list[tuple[list[str], dict[str, str] | None]] = []

	def __call__(self, command: list[str], environ: dict[str, str] | None, stdin: str | None = None) -> object:
		del stdin
		self.calls.append((command, environ))
		if command[:4] == ["podman", "info", "--format", "json"]:
			return walkthrough.CommandResult(0, '{"host":{"security":{"rootless":true}}}', "")
		if self.with_owned_volume and command[:3] == ["podman", "volume", "ls"]:
			return walkthrough.CommandResult(
				0,
				json.dumps(
					[
						{
							"Name": "walkthrough-retained",
							"Labels": {
								"io.podman.compose.project": self.project,
								"org.peptidyle.disposable.capability-sha256": self.capability_digest,
							},
						}
					]
				),
				"",
			)
		if "down" in command:
			return walkthrough.CommandResult(self.cleanup_code, "", "cleanup failure")
		if command[:3] == ["podman", "image", "exists"] and command[-1] == self.missing_image:
			return walkthrough.CommandResult(1, "", "")
		if command[:3] == ["podman", "image", "rm"]:
			return walkthrough.CommandResult(self.image_cleanup_code, "", "image cleanup failure")
		return walkthrough.CommandResult(0, "", "")


def runner_for(tmp_path: pathlib.Path, commands: CleanupCommands) -> object:
	"""Build one runner with its selected explicit environment file."""
	repository = tmp_path / "repository"
	env_file = repository / "containers/env.local"
	env_file.parent.mkdir(parents=True)
	env_file.write_text("PLE_GATEWAY_HOST_PORT=3010\n", encoding="ascii")
	(repository / "containers/compose.yaml").write_text("services: {}\n", encoding="ascii")
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
		walkthrough_models.ArrangementChildInputs(tmp_path / f"{secret}-manifest.json")
	)

	assert runner.finish(True) == 0
	assert private_directory is not None and not private_directory.exists()
	assert secret not in runner.report_path.read_text(encoding="ascii")


def test_compose_cleanup_failure_retains_private_recovery_state(
	tmp_path: pathlib.Path,
	capsys: pytest.CaptureFixture[str],
) -> None:
	"""A failed typed cleanup keeps its private recovery evidence and fails the run."""
	commands = CleanupCommands(7, with_owned_volume=True)
	runner = runner_for(tmp_path, commands)
	runner.prepare_report_directory()
	runner.prepare_journey_state()
	runner.create_private_stack_environment()
	commands.project = runner.compose_project_name
	private_env = runner.private_env_file
	assert private_env is not None
	commands.capability_digest = next(
		line.split("=", 1)[1]
		for line in private_env.read_text(encoding="ascii").splitlines()
		if line.startswith("PLE_DISPOSABLE_CAPABILITY_SHA256=")
	)
	private_directory = runner.private_state_directory
	secret = "student=private"
	runner.write_private_child_inputs(
		walkthrough_models.ArrangementChildInputs(tmp_path / f"{secret}-manifest.json")
	)
	runner.stack_launch_attempted = True

	result = runner.finish(True)
	output = capsys.readouterr().err

	assert result == 1 and private_directory is not None and private_directory.exists()
	assert str(private_directory) in output and secret not in output
	assert any("down" in command for command, _environment in commands.calls)


#============================================
def test_image_cleanup_failure_cannot_return_a_passing_receipt(tmp_path: pathlib.Path) -> None:
	"""An exact generated-image removal failure fails the run rather than passing it."""
	commands = CleanupCommands(image_cleanup_code=7)
	runner = runner_for(tmp_path, commands)
	runner.compose_command = ["podman", "compose"]
	runner.prepare_report_directory()
	runner.prepare_journey_state()
	runner.create_private_stack_environment()
	runner.stack_launch_attempted = True

	assert runner.finish(True) == 1
	assert json.loads(runner.report_path.read_text(encoding="ascii"))["stage"] == "cleanup"
	assert not any("down" in command for command, _environment in commands.calls)


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

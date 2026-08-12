"""Offline behavioral tests for explicit UI walkthrough child inputs."""

import importlib
import pathlib
import sys

import pytest


WALKTHROUGH_DIRECTORY = pathlib.Path(__file__).resolve().parent / "walkthrough"
sys.path.insert(0, str(WALKTHROUGH_DIRECTORY))
walkthrough = importlib.import_module("walklib.runner")


class RecordingCommands:
	"""Record runner-owned process boundaries without starting child processes."""

	def __init__(self) -> None:
		self.calls: list[tuple[list[str], dict[str, str] | None]] = []

	def __call__(self, command: list[str], environ: dict[str, str] | None) -> object:
		self.calls.append((command, environ))
		return walkthrough.CommandResult(0, "", "")


def write_env_file(repository: pathlib.Path, relative_path: str, port: int) -> pathlib.Path:
	"""Write one selected Compose environment file for an offline test."""
	path = repository / relative_path
	path.parent.mkdir(parents=True, exist_ok=True)
	path.write_text(f"PLE_GATEWAY_HOST_PORT={port}\n", encoding="ascii")
	return path


def resolved_inputs(repository: pathlib.Path, *arguments: str) -> object:
	"""Resolve a fixed seed plus the small command-line variation under test."""
	return walkthrough.resolve_inputs(
		walkthrough.parse_args(["--master-seed", "42", *arguments]), repository
	)


def test_selected_env_file_wins_and_ple_values_are_not_forwarded(tmp_path: pathlib.Path) -> None:
	"""Child configuration comes from the selected explicit file, not ambient PLE values."""
	repository = tmp_path / "repository"
	write_env_file(repository, "containers/env.local", 3010)
	write_env_file(repository, "config/walkthrough.env", 3011)
	runner = walkthrough.WalkthroughRunner(
		resolved_inputs(repository, "--env-file", "config/walkthrough.env"),
		repository,
		{"PLE_GATEWAY_HOST_PORT": "3999", "PLE_SECRET": "not-forwarded"},
		RecordingCommands(),
	)

	assert walkthrough.effective_gateway_port(runner.inputs) == 3011
	assert not any(key.startswith("PLE_") for key in runner.sanitized_child_environment())


def test_private_child_handoff_redacts_credentials_and_is_removed(tmp_path: pathlib.Path) -> None:
	"""The explicit private handoff contains no credential material and has a bounded lifetime."""
	repository = tmp_path / "repository"
	write_env_file(repository, "containers/env.local", 3010)
	manifest = repository / "containers/local-chapter-one-pilot.json"
	manifest.write_text("{}", encoding="ascii")
	runner = walkthrough.WalkthroughRunner(
		resolved_inputs(repository), repository, {}, RecordingCommands()
	)
	runner.prepare_journey_state()
	runner.write_private_child_inputs(
		walkthrough.walklib.models.ArrangementChildInputs(manifest)
	)
	input_path = runner.child_inputs_file
	assert input_path is not None
	assert "chapterOneManifestFile" in input_path.read_text(encoding="ascii")
	runner.remove_private_state()
	assert not input_path.exists()


def test_playwright_uses_standard_config_and_no_hidden_ple_protocol(tmp_path: pathlib.Path) -> None:
	"""The browser child receives its state at the standard config boundary."""
	repository = tmp_path / "repository"
	write_env_file(repository, "containers/env.local", 3010)
	commands = RecordingCommands()
	runner = walkthrough.WalkthroughRunner(
		resolved_inputs(repository),
		repository,
		{"PLE_UI_WALKTHROUGH_MASTER_SEED": "999"},
		commands,
	)
	runner.prepare_journey_state()
	runner.run_playwright_specification("tests/playwright/ui_walkthrough_keyboard_j1.spec.ts")
	command, environment = commands.calls[0]
	config_path = runner.playwright_config_file
	assert config_path is not None
	config_source = config_path.read_text(encoding="ascii")
	runner.remove_private_state()

	assert "--config" in command
	assert config_path.suffix == ".mts"
	assert str(repository / "tests/playwright/ui_walkthrough_config_factory.ts") in config_source
	assert str(repository / "tests/playwright") in config_source
	assert not any(key.startswith("PLE_") for key in environment or {})


def test_unsafe_explicit_paths_are_rejected_before_child_actions(tmp_path: pathlib.Path) -> None:
	"""The runner rejects unsafe selected input paths during preflight."""
	repository = tmp_path / "repository"
	env_file = write_env_file(repository, "containers/env.local", 3010)
	env_file.unlink()
	env_file.symlink_to("outside.env")

	with pytest.raises(walkthrough.RunnerError, match="must not be a symlink"):
		resolved_inputs(repository)

"""Lifecycle steps announce themselves before running so a long start is inspectable."""

import pathlib

import pytest

import local_stack_control.lifecycle_commands
import local_stack_control.disposable_stack_cleanup
import local_stack_control.models
import local_stack_control.process


class RecordingRunner(local_stack_control.process.CommandRunner):
	"""Accept every command and record it."""

	def __init__(self) -> None:
		self.argv: list[list[str]] = []

	#============================================
	def run(
		self,
		argv: list[str],
		environment: dict[str, str] | None = None,
		cwd: pathlib.Path | None = None,
		stdin: str | None = None,
	) -> local_stack_control.models.CommandResult:
		self.argv.append(argv)
		return local_stack_control.models.CommandResult(tuple(argv), 0, "", "")

	#============================================
	def stream(
		self,
		argv: list[str],
		environment: dict[str, str] | None = None,
		cwd: pathlib.Path | None = None,
	) -> int:
		raise AssertionError("lifecycle steps capture output; streaming is unexpected")


#============================================
def test_compose_run_reports_its_step_on_stderr_before_running(
	tmp_path: pathlib.Path,
	monkeypatch: pytest.MonkeyPatch,
	capsys: pytest.CaptureFixture[str],
) -> None:
	"""Each captured Compose operation leaves one non-secret step line in the supervisor log."""
	target = local_stack_control.models.ComposeTarget(
		repo_root=tmp_path,
		project="ple-live-demo-browser",
		env_file=tmp_path / "env.local",
		compose_files=(),
		provider=local_stack_control.models.ComposeProvider(("podman", "compose"), "podman compose"),
		with_smtp=False,
		env_setting_names=(),
	)
	monkeypatch.setattr(local_stack_control.lifecycle_commands, "child_environment", lambda *args: {})
	monkeypatch.setattr(
		local_stack_control.disposable_stack_cleanup, "private_environment_values", lambda *args: ()
	)
	runner = RecordingRunner()
	local_stack_control.lifecycle_commands.compose_run(target, runner, ["up", "-d", "postgres"])
	captured = capsys.readouterr()
	assert "Step: compose up -d postgres" in captured.err
	assert runner.argv[0][-3:] == ["up", "-d", "postgres"]

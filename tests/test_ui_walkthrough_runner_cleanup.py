"""Focused offline cleanup and report-lifecycle tests for the UI walkthrough runner."""

import dataclasses
import importlib
import io
import json
import pathlib
import shutil
import stat
import sys
import tempfile
import unittest
from contextlib import redirect_stderr, redirect_stdout
from unittest import mock

WALKTHROUGH_DIRECTORY = pathlib.Path(__file__).resolve().parent / "walkthrough"
sys.path.insert(0, str(WALKTHROUGH_DIRECTORY))
walkthrough = importlib.import_module("walklib.runner")


class FakeCommands:
	"""Capture offline command requests while returning successful results."""

	def __init__(self) -> None:
		self.commands: list[tuple[list[str], dict[str, str] | None]] = []

	def __call__(
		self,
		command: list[str],
		environ: dict[str, str] | None,
	) -> object:
		self.commands.append((command, environ))
		return walkthrough.CommandResult(0, "", "")


class CleanupFailureCommands(FakeCommands):
	"""Fail only the cleanup boundary."""

	def __init__(self, cleanup_error: BaseException | None = None) -> None:
		super().__init__()
		self.cleanup_error = cleanup_error

	def __call__(
		self,
		command: list[str],
		environ: dict[str, str] | None,
	) -> object:
		self.commands.append((command, environ))
		if command[-2:] == ["down", "--remove-orphans"]:
			if self.cleanup_error is not None:
				raise self.cleanup_error
			return walkthrough.CommandResult(7, "", "controlled cleanup failure")
		return walkthrough.CommandResult(0, "", "")


class UiWalkthroughRunnerCleanupTests(unittest.TestCase):
	"""Prove cleanup and public-report lifecycle behavior without launching services."""

	def make_repository(self, temporary_directory: pathlib.Path) -> pathlib.Path:
		"""Create only the small local files consumed during offline preflight."""
		repository_root = temporary_directory / "repository"
		env_directory = repository_root / "containers"
		env_directory.mkdir(parents=True)
		(env_directory / "env.local").write_text("PLE_GATEWAY_HOST_PORT=3010\n", encoding="ascii")
		return repository_root

	def resolve(self, repository_root: pathlib.Path, extra_arguments: list[str]) -> object:
		"""Resolve a baseline valid CLI plus one focused build-selection variation."""
		args = walkthrough.parse_args(["--master-seed", "42"] + extra_arguments)
		return walkthrough.resolve_inputs(args, repository_root)

	def test_cleanup_requires_runner_launch_and_keep_blocks_down(self) -> None:
		"""Cleanup cannot remove an unlaunched stack and --keep preserves a runner-started stack."""
		with tempfile.TemporaryDirectory() as temporary_name:
			repository_root = self.make_repository(pathlib.Path(temporary_name))
			commands = FakeCommands()
			inputs = self.resolve(repository_root, [])
			runner = walkthrough.WalkthroughRunner(inputs, repository_root, {}, commands)
			runner.compose_command = ["podman", "compose"]
			runner.prepare_report_directory()
			runner.finish(False)
			self.assertEqual(commands.commands, [])
			keep_inputs = dataclasses.replace(inputs, keep=True)
			keep_runner = walkthrough.WalkthroughRunner(keep_inputs, repository_root, {}, commands)
			keep_runner.compose_command = ["podman", "compose"]
			keep_runner.stack_launch_attempted = True
			keep_runner.prepare_report_directory()
			keep_runner.finish(False)
			self.assertEqual(commands.commands, [])

	def test_cleanup_nonzero_downgrades_success_to_redacted_failure(self) -> None:
		"""A failed down command cannot leave either a PASS report or a successful process status."""
		with tempfile.TemporaryDirectory() as temporary_name:
			repository_root = self.make_repository(pathlib.Path(temporary_name))
			inputs = self.resolve(repository_root, [])
			commands = CleanupFailureCommands()
			runner = walkthrough.WalkthroughRunner(inputs, repository_root, {}, commands)
			runner.compose_command = ["podman", "compose"]
			runner.stack_launch_attempted = True
			runner.prepare_report_directory()
			stdout = io.StringIO()
			stderr = io.StringIO()
			with redirect_stdout(stdout), redirect_stderr(stderr):
				status = runner.finish(True)
			self.assertEqual(status, 1)
			self.assertNotIn("PASS", stdout.getvalue())
			self.assertIn("cleanup failed", stderr.getvalue())
			report = json.loads(runner.report_path.read_text(encoding="ascii"))
			self.assertEqual((report["status"], report["stage"]), ("FAIL", "cleanup"))

	def test_cleanup_os_error_downgrades_success_to_redacted_failure(self) -> None:
		"""A cleanup process-start error also fails closed and leaves a readable failure record."""
		with tempfile.TemporaryDirectory() as temporary_name:
			repository_root = self.make_repository(pathlib.Path(temporary_name))
			inputs = self.resolve(repository_root, [])
			commands = CleanupFailureCommands(OSError("controlled cleanup OSError"))
			runner = walkthrough.WalkthroughRunner(inputs, repository_root, {}, commands)
			runner.compose_command = ["podman", "compose"]
			runner.stack_launch_attempted = True
			runner.prepare_report_directory()
			with redirect_stdout(io.StringIO()), redirect_stderr(io.StringIO()):
				status = runner.finish(True)
			self.assertEqual(status, 1)
			report = json.loads(runner.report_path.read_text(encoding="ascii"))
			self.assertEqual(report["status"], "FAIL")
			self.assertEqual(report["stage"], "cleanup")

	def test_invalid_env_decode_is_a_concise_main_failure(self) -> None:
		"""Malformed selected env bytes do not escape the public CLI with a traceback."""
		with tempfile.TemporaryDirectory() as temporary_name:
			repository_root = self.make_repository(pathlib.Path(temporary_name))
			env_file = repository_root / "containers" / "env.local"
			env_file.write_bytes(b"PLE_GATEWAY_HOST_PORT=3010\xff\n")
			stderr = io.StringIO()
			with redirect_stderr(stderr):
				status = walkthrough.main(["--master-seed", "42", "--env-file", str(env_file)])
			self.assertEqual(status, 1)
			self.assertIn("operational error during preflight", stderr.getvalue())
			self.assertNotIn("Traceback", stderr.getvalue())

	def test_report_write_failure_returns_nonzero_without_report_claim(self) -> None:
		"""An atomic report failure stays concise and does not pretend that a report was written."""
		with tempfile.TemporaryDirectory() as temporary_name:
			repository_root = self.make_repository(pathlib.Path(temporary_name))
			inputs = self.resolve(repository_root, [])
			runner = walkthrough.WalkthroughRunner(inputs, repository_root, {}, FakeCommands())
			runner.prepare_report_directory()
			stderr = io.StringIO()
			with mock.patch.object(walkthrough.os, "open", side_effect=OSError("full disk")):
				with redirect_stderr(stderr):
					status = runner.finish(False)
			self.assertEqual(status, 1)
			self.assertIn("could not write walkthrough report", stderr.getvalue())
			self.assertFalse(runner.report_path.exists())

	def test_report_directory_is_recreated_after_playwright_artifact_cleanup(self) -> None:
		"""A complete Playwright cleanup cannot erase the final private PASS report."""
		with tempfile.TemporaryDirectory() as temporary_name:
			repository_root = self.make_repository(pathlib.Path(temporary_name))
			inputs = self.resolve(repository_root, [])
			runner = walkthrough.WalkthroughRunner(inputs, repository_root, {}, FakeCommands())
			runner.prepare_report_directory()
			shutil.rmtree(repository_root / "test-results")
			status = runner.finish(True)
			self.assertEqual(status, 0)
			report = json.loads(runner.report_path.read_text(encoding="ascii"))
			self.assertEqual((report["status"], report["stage"]), ("PASS", "complete"))
			self.assertEqual(stat.S_IMODE(runner.report_path.stat().st_mode), 0o600)

	def test_report_symlink_replacement_fails_closed_without_pass_record(self) -> None:
		"""Replacing the report directory with a link cannot redirect a final PASS record."""
		with tempfile.TemporaryDirectory() as temporary_name:
			repository_root = self.make_repository(pathlib.Path(temporary_name))
			inputs = self.resolve(repository_root, [])
			runner = walkthrough.WalkthroughRunner(inputs, repository_root, {}, FakeCommands())
			runner.prepare_report_directory()
			outside_directory = repository_root / "outside"
			outside_directory.mkdir()
			runner.report_directory.rmdir()
			runner.report_directory.symlink_to(outside_directory, target_is_directory=True)
			stdout = io.StringIO()
			stderr = io.StringIO()
			with redirect_stdout(stdout), redirect_stderr(stderr):
				status = runner.finish(True)
			self.assertEqual(status, 1)
			self.assertNotIn("PASS", stdout.getvalue())
			self.assertIn("could not write walkthrough report", stderr.getvalue())
			self.assertFalse((outside_directory / inputs.report_basename).exists())


if __name__ == "__main__":
	unittest.main()

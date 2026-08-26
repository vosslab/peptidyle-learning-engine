"""Offline contracts for the fixed production-browser developer commands."""

import pathlib

import pytest

import local_stack_control.browser_suite_developer
import local_stack_control.cli
import local_stack_control.commands
import local_stack_control.models
import local_stack_control.process


class RecordingRunner(local_stack_control.process.CommandRunner):
	"""Record the optional browser opener without executing a host command."""

	def __init__(self, opener_succeeds: bool = True) -> None:
		"""Create a runner with one deterministic opener result."""
		self.opener_succeeds = opener_succeeds
		self.argvs: list[list[str]] = []

	def run(
		self,
		argv: list[str],
		environment: dict[str, str] | None = None,
		cwd: pathlib.Path | None = None,
		stdin: str | None = None,
	) -> local_stack_control.models.CommandResult:
		"""Record an opener request and return its configured result."""
		self.argvs.append(argv)
		returncode = 0 if self.opener_succeeds or argv[0] == "xdg-open" else 1
		return local_stack_control.models.CommandResult(tuple(argv), returncode, "", "")

	def stream(
		self,
		argv: list[str],
		environment: dict[str, str] | None = None,
		cwd: pathlib.Path | None = None,
	) -> int:
		"""Reject streaming because developer commands use their owner protocol."""
		raise AssertionError("developer commands must not stream a generic lifecycle command")


#============================================
def receipt() -> local_stack_control.browser_suite_developer.DeveloperStartReceipt:
	"""Return the fixed safe developer origin owned by the browser supervisor."""
	return local_stack_control.browser_suite_developer.DeveloperStartReceipt(
		"https://localhost:55324/", "ple-live-demo-browser"
	)


#============================================
def test_start_parser_exposes_only_the_opening_convenience() -> None:
	"""The public start surface cannot select another environment or artifact."""
	args = local_stack_control.cli.build_parser().parse_args(["start", "--headless"])
	assert args.headless
	with pytest.raises(SystemExit):
		local_stack_control.cli.build_parser().parse_args(["start", "--project", "other"])


#============================================
def test_start_uses_the_fixed_owner_and_opens_its_safe_origin(
	tmp_path: pathlib.Path,
	monkeypatch: pytest.MonkeyPatch,
	capsys: pytest.CaptureFixture[str],
) -> None:
	"""Start delegates build, seed, and origin selection to the fixed owner."""
	events: list[str] = []

	def reconcile(
		root: pathlib.Path,
		runner: local_stack_control.process.CommandRunner,
	) -> str:
		"""Record the mandatory fixed-owner cleanup before replacement."""
		assert root == tmp_path
		assert isinstance(runner, RecordingRunner)
		events.append("reconcile")
		return receipt().project

	def start_owner(
		root: pathlib.Path,
	) -> local_stack_control.browser_suite_developer.DeveloperStartReceipt:
		"""Record the owner input and return its verified public receipt."""
		assert root == tmp_path
		events.append("start")
		return receipt()

	monkeypatch.setattr(
		local_stack_control.browser_suite_developer,
		"reconcile_developer_session",
		reconcile,
	)
	monkeypatch.setattr(
		local_stack_control.browser_suite_developer,
		"start_developer_session",
		start_owner,
	)
	monkeypatch.setattr(
		local_stack_control.lifecycle,
		"start_lifecycle",
		lambda *unused: (_ for _ in ()).throw(AssertionError("generic lifecycle must stay outside start")),
	)
	runner = RecordingRunner()
	result = local_stack_control.cli.run(["start"], runner, tmp_path)
	output = capsys.readouterr().out
	assert result == 0
	assert events == ["reconcile", "start"]
	assert runner.argvs == [["open", receipt().origin]]
	assert "Stop with: ./run_live_demo.sh stop" in output


#============================================
def test_start_headless_preserves_the_same_canonical_session(
	tmp_path: pathlib.Path,
	monkeypatch: pytest.MonkeyPatch,
) -> None:
	"""Headless mode changes only local presentation, never the production session."""
	def reconcile(
		root: pathlib.Path,
		runner: local_stack_control.process.CommandRunner,
	) -> str:
		"""Return the fixed project after the replacement boundary is prepared."""
		return receipt().project

	def start_owner(
		root: pathlib.Path,
	) -> local_stack_control.browser_suite_developer.DeveloperStartReceipt:
		"""Return the fixed safe developer receipt."""
		return receipt()

	monkeypatch.setattr(
		local_stack_control.browser_suite_developer,
		"reconcile_developer_session",
		reconcile,
	)
	monkeypatch.setattr(
		local_stack_control.browser_suite_developer,
		"start_developer_session",
		start_owner,
	)
	runner = RecordingRunner()
	result = local_stack_control.cli.run(["start", "--headless"], runner, tmp_path)
	assert result == 0
	assert runner.argvs == []


#============================================
def test_start_replaces_an_existing_owner_before_launching_the_fresh_session(
	tmp_path: pathlib.Path,
	monkeypatch: pytest.MonkeyPatch,
) -> None:
	"""Start completes owner-scoped cleanup before acquiring the fresh lease."""
	events: list[str] = []

	def reconcile(
		root: pathlib.Path,
		runner: local_stack_control.process.CommandRunner,
	) -> str:
		"""Record the owner-scoped cleanup boundary."""
		events.append("reconcile")
		return receipt().project

	def start_owner(
		root: pathlib.Path,
	) -> local_stack_control.browser_suite_developer.DeveloperStartReceipt:
		"""Record the fresh owner launch."""
		events.append("start")
		return receipt()

	monkeypatch.setattr(
		local_stack_control.browser_suite_developer,
		"reconcile_developer_session",
		reconcile,
	)
	monkeypatch.setattr(
		local_stack_control.browser_suite_developer,
		"start_developer_session",
		start_owner,
	)
	result = local_stack_control.cli.run(["start", "--headless"], RecordingRunner(), tmp_path)
	assert result == 0
	assert events == ["reconcile", "start"]


#============================================
def test_stop_uses_authenticated_owner_cleanup(
	tmp_path: pathlib.Path,
	monkeypatch: pytest.MonkeyPatch,
	capsys: pytest.CaptureFixture[str],
) -> None:
	"""Stop requests and reports the exact fixed-owner cleanup receipt."""
	stopped: list[pathlib.Path] = []

	def stop_owner(root: pathlib.Path) -> local_stack_control.browser_suite_developer.DeveloperStartReceipt:
		"""Record authenticated owner cleanup without running Podman."""
		stopped.append(root)
		return receipt()

	monkeypatch.setattr(local_stack_control.browser_suite_developer, "request_stop", stop_owner)
	monkeypatch.setattr(
		local_stack_control.cleanup,
		"stop_plan",
		lambda *unused: (_ for _ in ()).throw(AssertionError("generic cleanup must stay outside stop")),
	)
	result = local_stack_control.cli.run(["stop"], RecordingRunner(), tmp_path)
	assert result == 0
	assert stopped == [tmp_path]
	assert "ple-live-demo-browser" in capsys.readouterr().out


#============================================
def test_stop_purges_an_interrupted_fixed_owner(
	tmp_path: pathlib.Path,
	monkeypatch: pytest.MonkeyPatch,
	capsys: pytest.CaptureFixture[str],
) -> None:
	"""Stop reconciles an orphan only through the lease-bound fixed reset owner."""
	events: list[str] = []

	def unavailable_owner(root: pathlib.Path) -> local_stack_control.browser_suite_developer.DeveloperStartReceipt:
		"""Represent an interrupted supervisor whose containers can remain."""
		assert root == tmp_path
		raise local_stack_control.browser_suite_developer.DeveloperBrowserSuiteError(
			"developer browser session is not running"
		)

	monkeypatch.setattr(local_stack_control.browser_suite_developer, "request_stop", unavailable_owner)
	monkeypatch.setattr(
		local_stack_control.browser_suite_developer,
		"purge_orphaned_session",
		lambda root, runner: (
			events.append("purge"),
			local_stack_control.models.LIVE_DEMO_BROWSER_PROJECT,
		)[1],
	)
	result = local_stack_control.cli.run(["stop"], RecordingRunner(), tmp_path)
	assert result == 0
	assert events == ["purge"]
	assert "ple-live-demo-browser" in capsys.readouterr().out


#============================================
def test_stop_lease_reconciles_owner_protocol_failures(
	tmp_path: pathlib.Path,
	monkeypatch: pytest.MonkeyPatch,
	capsys: pytest.CaptureFixture[str],
) -> None:
	"""A released failed owner yields to exact lease-bound orphan cleanup."""
	events: list[str] = []

	def failed_stop(root: pathlib.Path) -> local_stack_control.browser_suite_developer.DeveloperStartReceipt:
		"""Return one protocol failure from the fixed supervisor."""
		raise local_stack_control.browser_suite_developer.DeveloperBrowserSuiteError(
			"developer browser supervisor cleanup failed"
		)

	monkeypatch.setattr(local_stack_control.browser_suite_developer, "request_stop", failed_stop)
	monkeypatch.setattr(
		local_stack_control.browser_suite_developer,
		"purge_orphaned_session",
		lambda root, runner: (
			events.append("purge"),
			local_stack_control.models.LIVE_DEMO_BROWSER_PROJECT,
		)[1],
	)
	result = local_stack_control.cli.run(["stop"], RecordingRunner(), tmp_path)
	assert result == 0
	assert events == ["purge"]
	assert "ple-live-demo-browser" in capsys.readouterr().out

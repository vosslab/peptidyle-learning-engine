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
	args = local_stack_control.cli.build_parser().parse_args(["start", "--no-open"])
	assert args.no_open
	with pytest.raises(SystemExit):
		local_stack_control.cli.build_parser().parse_args(["start", "--project", "other"])


#============================================
def test_start_uses_the_fixed_owner_and_opens_its_safe_origin(
	tmp_path: pathlib.Path,
	monkeypatch: pytest.MonkeyPatch,
	capsys: pytest.CaptureFixture[str],
) -> None:
	"""Start delegates build, seed, and origin selection to the fixed owner."""
	started: list[pathlib.Path] = []

	def start_owner(root: pathlib.Path) -> local_stack_control.browser_suite_developer.DeveloperStartReceipt:
		"""Record the owner input and return its verified public receipt."""
		started.append(root)
		return receipt()

	monkeypatch.setattr(local_stack_control.browser_suite_developer, "start_developer_session", start_owner)
	monkeypatch.setattr(
		local_stack_control.lifecycle,
		"start_lifecycle",
		lambda *unused: (_ for _ in ()).throw(AssertionError("generic lifecycle must stay outside start")),
	)
	runner = RecordingRunner()
	result = local_stack_control.cli.run(["start"], runner, tmp_path)
	output = capsys.readouterr().out
	assert result == 0
	assert started == [tmp_path]
	assert runner.argvs == [["open", receipt().origin]]
	assert "Stop with: python3 local_stack.py stop" in output


#============================================
def test_start_no_open_preserves_the_same_canonical_session(
	tmp_path: pathlib.Path,
	monkeypatch: pytest.MonkeyPatch,
	) -> None:
	"""No-open changes only local presentation, never the production session."""
	monkeypatch.setattr(local_stack_control.browser_suite_developer, "start_developer_session", lambda root: receipt())
	runner = RecordingRunner()
	result = local_stack_control.cli.run(["start", "--no-open"], runner, tmp_path)
	assert result == 0
	assert runner.argvs == []


#============================================
def test_start_reports_an_existing_owner_without_generic_recovery(
	tmp_path: pathlib.Path,
	monkeypatch: pytest.MonkeyPatch,
	capsys: pytest.CaptureFixture[str],
) -> None:
	"""An active fixed owner stays authoritative instead of starting another stack."""
	def already_running(root: pathlib.Path) -> local_stack_control.browser_suite_developer.DeveloperStartReceipt:
		"""Represent the owner's single-flight refusal."""
		raise local_stack_control.models.ControllerError("developer browser session already running")

	monkeypatch.setattr(local_stack_control.browser_suite_developer, "start_developer_session", already_running)
	result = local_stack_control.cli.run(["start", "--no-open"], RecordingRunner(), tmp_path)
	assert result == 2
	assert "already running" in capsys.readouterr().err


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
@pytest.mark.parametrize(
	"message",
	("developer browser session is not running", "developer browser supervisor cleanup failed"),
)
def test_stop_reports_owner_protocol_failures(
	tmp_path: pathlib.Path,
	monkeypatch: pytest.MonkeyPatch,
	capsys: pytest.CaptureFixture[str],
	message: str,
) -> None:
	"""The CLI keeps stop failures at the concise controller error boundary."""
	def failed_stop(root: pathlib.Path) -> local_stack_control.browser_suite_developer.DeveloperStartReceipt:
		"""Return one protocol failure from the fixed supervisor."""
		raise local_stack_control.models.ControllerError(message)

	monkeypatch.setattr(local_stack_control.browser_suite_developer, "request_stop", failed_stop)
	result = local_stack_control.cli.run(["stop"], RecordingRunner(), tmp_path)
	assert result == 2
	assert message in capsys.readouterr().err

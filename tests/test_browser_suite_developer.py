"""Deterministic contracts for the fixed live-demo developer supervisor."""

import os
import sys
import time
import signal
import pathlib
import threading

import pytest

import local_stack_control.browser_suite_developer
import local_stack_control.browser_suite_developer_operations
import local_stack_control.browser_suite_developer_start
import local_stack_control.browser_suite_lease
import local_stack_control.browser_suite_reset
import local_stack_control.lifecycle_diagnostics
import local_stack_control.models
import local_stack_control.process


def _receipt() -> local_stack_control.browser_suite_developer.DeveloperControlReceipt:
	"""Return one fixed private receipt used only to exercise protocol decoding."""
	return local_stack_control.browser_suite_developer.DeveloperControlReceipt(
		42, "a" * 64, "b" * 64, "c" * 64, "https://localhost:55324/", "ple-live-demo-browser"
	)


#============================================
def _write_receipt(root: pathlib.Path) -> None:
	"""Publish one valid private receipt through the same descriptor writer as production."""
	with local_stack_control.browser_suite_lease.BrowserSuiteLease.acquire(root):
		descriptor = local_stack_control.browser_suite_developer._checked_root_descriptor(root)
		try:
			local_stack_control.browser_suite_developer._write_private_file(
				descriptor,
				local_stack_control.browser_suite_developer.CONTROL_NAME,
				local_stack_control.browser_suite_developer._control_value(_receipt()),
			)
			local_stack_control.browser_suite_developer._write_private_file(
				descriptor,
				local_stack_control.browser_suite_developer.LAUNCH_NAME,
				local_stack_control.browser_suite_developer._launch_value("c" * 64),
			)
		finally:
			os.close(descriptor)


#============================================
def test_control_receipt_requires_checked_private_mode(tmp_path: pathlib.Path) -> None:
	"""A replaced or broadly-readable receipt cannot direct a developer shutdown."""
	_write_receipt(tmp_path)
	path = (
		tmp_path
		/ local_stack_control.browser_suite_lease.LIVE_DEMO_BROWSER_STATE_DIRECTORY
		/ local_stack_control.browser_suite_developer.CONTROL_NAME
	)
	path.chmod(0o644)
	with pytest.raises(local_stack_control.browser_suite_developer.DeveloperBrowserSuiteError):
		local_stack_control.browser_suite_developer.read_control_receipt(tmp_path)


#============================================
def test_existing_wrong_socket_directory_mode_fails_without_mutation(
	tmp_path: pathlib.Path,
	monkeypatch: pytest.MonkeyPatch,
) -> None:
	"""An existing local-control path is checked before mode changes or socket use."""
	directory = tmp_path / "socket-control"
	directory.mkdir(mode=0o755)
	directory.chmod(0o755)
	monkeypatch.setattr(local_stack_control.browser_suite_developer, "SOCKET_DIRECTORY", directory)
	with pytest.raises(local_stack_control.browser_suite_developer.DeveloperBrowserSuiteError):
		local_stack_control.browser_suite_developer._socket_directory_descriptor()
	assert directory.stat().st_mode & 0o777 == 0o755


#============================================
def test_stop_request_authenticates_live_supervisor_not_recycled_pid() -> None:
	"""Matching a PID alone never authorizes a stale or unrelated supervisor stop."""
	stale = _receipt()
	live = local_stack_control.browser_suite_developer.DeveloperControlReceipt(
		42, "d" * 64, "e" * 64, stale.launch_id, stale.origin, stale.project
	)
	request = local_stack_control.browser_suite_developer._request_value(stale)
	assert not local_stack_control.browser_suite_developer._validate_stop_request(request, live)


#============================================
def test_stop_waits_for_a_lease_holding_supervisor_to_publish_its_receipt(
	monkeypatch: pytest.MonkeyPatch,
	tmp_path: pathlib.Path,
) -> None:
	"""A stop during startup waits for the owner instead of attempting unsafe cleanup."""
	observations: list[str] = []

	def read_receipt(_root: pathlib.Path) -> local_stack_control.browser_suite_developer.DeveloperControlReceipt:
		observations.append("read")
		if len(observations) == 1:
			raise local_stack_control.browser_suite_developer.DeveloperBrowserSuiteError(
				"Developer Browser Suite is not running"
			)
		return _receipt()

	monkeypatch.setattr(local_stack_control.browser_suite_developer, "read_control_receipt", read_receipt)
	monkeypatch.setattr(
		local_stack_control.browser_suite_developer,
		"_browser_suite_lease_is_held",
		lambda _root: True,
	)
	monkeypatch.setattr(local_stack_control.browser_suite_developer.time, "sleep", lambda _seconds: None)
	result = local_stack_control.browser_suite_developer._wait_for_authenticated_control_receipt(
		tmp_path, 1.0
	)
	assert result == _receipt()


#============================================
def test_start_waits_for_private_ready_receipt(tmp_path: pathlib.Path) -> None:
	"""The parent reports the canonical origin only after the child receipt exists."""
	_write_receipt(tmp_path)
	def spawn(
		root: pathlib.Path,
		_lease: local_stack_control.browser_suite_lease.BrowserSuiteLease,
	) -> object:
		with pytest.raises(local_stack_control.browser_suite_developer.DeveloperBrowserSuiteError):
			local_stack_control.browser_suite_developer.read_control_receipt(root)
		launch_id = local_stack_control.browser_suite_developer._read_launch_id(root)
		descriptor = local_stack_control.browser_suite_developer._checked_root_descriptor(root)
		try:
			receipt = local_stack_control.browser_suite_developer.DeveloperControlReceipt(
				42, "a" * 64, "b" * 64, launch_id, "https://localhost:55324/", "ple-live-demo-browser"
			)
			local_stack_control.browser_suite_developer._write_private_file(
				descriptor,
				local_stack_control.browser_suite_developer.CONTROL_NAME,
				local_stack_control.browser_suite_developer._control_value(receipt),
			)
		finally:
			os.close(descriptor)
		return object()

	result = local_stack_control.browser_suite_developer_start.start_developer_browser_suite(
		tmp_path, 0.5, spawn
	)
	assert result == local_stack_control.browser_suite_developer.DeveloperStartReceipt(
		"https://localhost:55324/", "ple-live-demo-browser"
	)


#============================================
def test_stale_private_control_receipt_never_owns_the_suite_lease(tmp_path: pathlib.Path) -> None:
	"""A crashed supervisor's old receipt cannot prevent the next fixed owner from resetting."""
	_write_receipt(tmp_path)
	with local_stack_control.browser_suite_lease.BrowserSuiteLease.acquire(tmp_path) as lease:
		assert lease.workspace == (
			tmp_path
			/ local_stack_control.browser_suite_lease.LIVE_DEMO_BROWSER_STATE_DIRECTORY
			/ local_stack_control.browser_suite_lease.WORKSPACE_NAME
		)


#============================================
def test_orphan_purge_removes_owned_resources_workspace_and_control_state(
	tmp_path: pathlib.Path,
	monkeypatch: pytest.MonkeyPatch,
) -> None:
	"""A released owner can be recovered without caller-selected cleanup scope."""
	_write_receipt(tmp_path)
	with local_stack_control.browser_suite_lease.BrowserSuiteLease.acquire(tmp_path) as lease:
		workspace = lease.reset_workspace()
		(workspace / "abandoned").write_text("private\n", encoding="ascii")
	events: list[str] = []
	empty = local_stack_control.models.ProjectSnapshot("ple-live-demo-browser", (), (), ())
	monkeypatch.setattr(
		local_stack_control.browser_suite_reset,
		"reset_live_demo_browser",
		lambda lease, runner, root: (events.append("reset"), empty)[1],
	)
	project = local_stack_control.browser_suite_developer.purge_orphaned_developer_browser_suite(
		tmp_path,
		local_stack_control.process.SubprocessRunner(),
	)
	assert project == "ple-live-demo-browser"
	assert events == ["reset"]
	workspace = (
		tmp_path
		/ local_stack_control.browser_suite_lease.LIVE_DEMO_BROWSER_STATE_DIRECTORY
		/ local_stack_control.browser_suite_lease.WORKSPACE_NAME
	)
	assert tuple(workspace.iterdir()) == ()
	with pytest.raises(
		local_stack_control.browser_suite_developer.DeveloperBrowserSuiteError,
		match="Developer Browser Suite is not running",
	):
		local_stack_control.browser_suite_developer.read_control_receipt(tmp_path)


#============================================
def test_start_early_supervisor_exit_terminates_child_then_exact_resets_fixed_owner(
	tmp_path: pathlib.Path,
	monkeypatch: pytest.MonkeyPatch,
) -> None:
	"""An exited unready supervisor immediately yields its lease to exact cleanup."""
	events: list[str] = []
	empty = local_stack_control.models.ProjectSnapshot("ple-live-demo-browser", (), (), ())
	monkeypatch.setattr(
		local_stack_control.browser_suite_reset,
		"reset_live_demo_browser",
		lambda _lease, _runner, _root: (events.append("reset"), empty)[1],
	)
	class ExitedChild:
		"""Minimal process-like supervisor that reports one early exit."""

		def poll(self) -> int:
			events.append("poll")
			return 1

	child = ExitedChild()
	with pytest.raises(local_stack_control.browser_suite_developer.DeveloperBrowserSuiteError):
		local_stack_control.browser_suite_developer_start.start_developer_browser_suite(
			tmp_path,
			0.5,
			lambda _root, _lease: child,
			lambda observed, _timeout: events.append("terminated") if observed is child else None,
		)
	assert events == ["poll", "terminated", "reset"]
	with local_stack_control.browser_suite_lease.BrowserSuiteLease.acquire(tmp_path) as lease:
		assert tuple(lease.reset_workspace().iterdir()) == ()


#============================================
def test_failed_start_receipt_retains_only_redacted_operator_evidence(
	tmp_path: pathlib.Path,
	monkeypatch: pytest.MonkeyPatch,
) -> None:
	"""A launch receipt survives workspace cleanup without retaining a private value."""
	directory = tmp_path / "socket-control"
	directory.mkdir(mode=0o700)
	directory.chmod(0o700)
	monkeypatch.setattr(local_stack_control.browser_suite_developer, "SOCKET_DIRECTORY", directory)
	local_stack_control.browser_suite_developer._write_failure_receipt(
		tmp_path,
		local_stack_control.browser_suite_developer.DeveloperFailureReceipt(
			"launch", 1, "Error: PLE_RENDERER_TOKEN=private-value /private/credential-file",
		),
	)
	receipt = local_stack_control.browser_suite_developer._read_failure_receipt(tmp_path)
	assert receipt == local_stack_control.browser_suite_developer.DeveloperFailureReceipt(
		"launch", 1, "Error: [private] [path]"
	)
	assert "private-value" not in receipt.diagnostic


#============================================
def test_launch_diagnostic_retains_the_first_actionable_error() -> None:
	"""A redacted child failure survives receipts and trailing Podman boilerplate."""
	result = local_stack_control.models.CommandResult(
		("podman", "compose"),
		1,
		'{"biochemistry":{"sourceSha256":"private-value"}}\n' * 80,
		"\x1b[4m>>>> Executing external compose provider <<<<\x1b[0m\n"
		"Error: Live Demo Course discovery request did not complete: private-value\n"
		"Error: executing podman compose: exit status 1\n",
	)
	detail = local_stack_control.lifecycle_diagnostics.redacted_failure_detail(
		result, ("private-value",)
	)
	diagnostic = local_stack_control.browser_suite_developer._launch_diagnostic(
		"developer browser stack launch failed: ERROR: Installation content provisioning failed "
		f"({detail}); retained stack resources are available for diagnostics\n"
		"Error: executing podman compose after failure: exit status 1\n"
	)
	assert diagnostic.startswith("Error: Live Demo Course discovery request did not complete")
	assert "private-value" not in diagnostic


#============================================
def test_launch_diagnostic_retains_causes_but_excludes_trailing_diagnostics() -> None:
	"""Keep an anyhow chain or one direct cause, but never trailing diagnostic noise."""
	diagnostic = local_stack_control.browser_suite_developer._launch_diagnostic(
		"Error: launch failed\n"
		"Caused by:\n"
		"    permission denied for PLE_RENDERER_TOKEN=private-value /private/socket\n"
		"stack backtrace:\n"
		"   0: arbitrary backtrace\n"
		"status: ignored\n"
		"Caused by:\n"
		"    0: wrapper cause\n"
		"    1: root cause\n"
		"    2: detail cause\n"
		"unindented status\n"
	)
	assert diagnostic == (
		"Error: launch failed Caused by: permission denied for [private] [path] "
		"Caused by: 0: wrapper cause 1: root cause 2: detail cause"
	)


#============================================
def _write_child_script(tmp_path: pathlib.Path, body: str) -> pathlib.Path:
	"""Write one small Python child used to stand in for the stack launch command."""
	script = tmp_path / "_launch_child.py"
	script.write_text(body, encoding="ascii")
	return script


#============================================
def test_launch_output_streams_to_stderr_and_is_retained_for_diagnostics(
	tmp_path: pathlib.Path,
	capfd: pytest.CaptureFixture[str],
) -> None:
	"""Launch output reaches the supervisor log live and still feeds the failure diagnostic."""
	script = _write_child_script(
		tmp_path,
		"import sys\n"
		"print('line one', flush=True)\n"
		"print('Error: line two', file=sys.stderr, flush=True)\n"
		"raise SystemExit(1)\n",
	)
	active: list[object] = []
	returncode, output = local_stack_control.browser_suite_developer_operations.stream_launch(
		[sys.executable, str(script)], dict(os.environ), tmp_path, active
	)
	assert returncode == 1
	assert "line one" in output and "Error: line two" in output
	captured = capfd.readouterr().err
	assert "line one" in captured and "Error: line two" in captured
	assert active == []


#============================================
def test_interrupt_terminates_the_running_launch_process_group(tmp_path: pathlib.Path) -> None:
	"""An interrupt stops a long launch within seconds instead of after it completes."""
	script = _write_child_script(tmp_path, "import time\ntime.sleep(30)\n")
	active: list[object] = []
	results: list[tuple[int, str]] = []
	def run_launch() -> None:
		results.append(
			local_stack_control.browser_suite_developer_operations.stream_launch(
				[sys.executable, str(script)], dict(os.environ), tmp_path, active
			)
		)
	worker = threading.Thread(target=run_launch)
	started = time.monotonic()
	worker.start()
	while not active and time.monotonic() - started < 5:
		time.sleep(0.01)
	local_stack_control.browser_suite_developer_operations.interrupt_launch(active)
	worker.join(timeout=5)
	assert not worker.is_alive()
	assert results[0][0] != 0
	assert time.monotonic() - started < 5


#============================================
def test_supervisor_termination_signal_interrupts_a_blocked_launch(
	tmp_path: pathlib.Path,
	monkeypatch: pytest.MonkeyPatch,
) -> None:
	"""SIGTERM during launch forwards an interrupt so cleanup does not wait for the build."""
	directory = tmp_path / "socket-control"
	directory.mkdir(mode=0o700)
	directory.chmod(0o700)
	monkeypatch.setattr(local_stack_control.browser_suite_developer, "SOCKET_DIRECTORY", directory)
	empty = local_stack_control.models.ProjectSnapshot("ple-live-demo-browser", (), (), ())
	monkeypatch.setattr(
		local_stack_control.browser_suite_reset,
		"reset_live_demo_browser",
		lambda _lease, _runner, _root: empty,
	)
	events: list[str] = []
	interrupted = threading.Event()
	def start(
		_lease: local_stack_control.browser_suite_lease.BrowserSuiteLease,
		_root: pathlib.Path,
		_workspace: pathlib.Path,
	) -> local_stack_control.browser_suite_developer.RunningDeveloperStack:
		events.append("start")
		if not interrupted.wait(timeout=5):
			raise AssertionError("launch was never interrupted")
		raise local_stack_control.browser_suite_developer.DeveloperBrowserSuiteError("launch interrupted")
	def interrupt() -> None:
		events.append("interrupt")
		interrupted.set()
	operations = local_stack_control.browser_suite_developer.DeveloperOperations(
		start,
		lambda _running, _root: events.append("stop"),
		lambda _lease, _root: events.append("verify_empty"),
		interrupt,
	)
	threading.Timer(0.1, lambda: os.kill(os.getpid(), signal.SIGTERM)).start()
	with pytest.raises(local_stack_control.browser_suite_developer.DeveloperBrowserSuiteError):
		local_stack_control.browser_suite_developer.run_supervisor(tmp_path, operations)
	assert events == ["start", "interrupt", "verify_empty"]


#============================================
def test_supervisor_log_tail_reports_phase_and_last_line(tmp_path: pathlib.Path) -> None:
	"""The heartbeat reads the newest phase marker and last non-empty supervisor line."""
	log_path = tmp_path / "supervisor.log"
	phase, last_line = local_stack_control.browser_suite_developer_start.log_tail(log_path)
	assert (phase, last_line) == ("starting", "")
	log_path.write_text(
		"[phase] reset\n[phase] launch\nStep: ./build.sh --debug\n\nStep: compose up -d postgres\n\n",
		encoding="ascii",
	)
	phase, last_line = local_stack_control.browser_suite_developer_start.log_tail(log_path)
	assert (phase, last_line) == ("launch", "Step: compose up -d postgres")
	# A long build pushes the marker out of the bounded tail; the known phase survives.
	with log_path.open("a", encoding="ascii") as log:
		log.write("build output line\n" * 400)
	phase, last_line = local_stack_control.browser_suite_developer_start.log_tail(log_path, "launch")
	assert (phase, last_line) == ("launch", "build output line")


#============================================
def test_start_keeps_waiting_while_supervisor_log_grows_then_reports_a_stall(
	tmp_path: pathlib.Path,
	monkeypatch: pytest.MonkeyPatch,
) -> None:
	"""Progress in the supervisor log extends the wait; silence ends it with a stall diagnosis."""
	events: list[str] = []
	empty = local_stack_control.models.ProjectSnapshot("ple-live-demo-browser", (), (), ())
	monkeypatch.setattr(
		local_stack_control.browser_suite_reset,
		"reset_live_demo_browser",
		lambda _lease, _runner, _root: (events.append("reset"), empty)[1],
	)
	log_path = (
		tmp_path
		/ local_stack_control.browser_suite_lease.LIVE_DEMO_BROWSER_STATE_DIRECTORY
		/ local_stack_control.browser_suite_developer.SUPERVISOR_LOG_NAME
	)
	class LiveChild:
		"""Process-like supervisor that never exits during the test."""

		def poll(self) -> None:
			return None

	def append_progress() -> None:
		"""Append log lines for 0.4s, longer than the injected stall window."""
		finish = time.monotonic() + 0.4
		while time.monotonic() < finish:
			with log_path.open("a", encoding="ascii") as log:
				log.write("[phase] launch\nStep: still building\n")
			time.sleep(0.02)

	def spawn(
		_root: pathlib.Path,
		_lease: local_stack_control.browser_suite_lease.BrowserSuiteLease,
	) -> object:
		log_path.parent.mkdir(parents=True, exist_ok=True)
		log_path.touch()
		threading.Thread(target=append_progress).start()
		return LiveChild()

	started = time.monotonic()
	with pytest.raises(
		local_stack_control.browser_suite_developer.DeveloperBrowserSuiteError,
		match="stalled during launch.*Step: still building",
	):
		local_stack_control.browser_suite_developer_start.start_developer_browser_suite(
			tmp_path,
			5.0,
			spawn,
			lambda _child, _timeout: events.append("terminated"),
			stall_seconds=0.15,
		)
	elapsed = time.monotonic() - started
	assert 0.4 < elapsed < 3.0
	assert events == ["terminated", "reset"]


#============================================
def test_start_keyboard_interrupt_terminates_child_and_resets(
	tmp_path: pathlib.Path,
	monkeypatch: pytest.MonkeyPatch,
) -> None:
	"""Ctrl-C during a start stops the supervisor and resets the fixed suite, then reports."""
	events: list[str] = []
	empty = local_stack_control.models.ProjectSnapshot("ple-live-demo-browser", (), (), ())
	monkeypatch.setattr(
		local_stack_control.browser_suite_reset,
		"reset_live_demo_browser",
		lambda _lease, _runner, _root: (events.append("reset"), empty)[1],
	)
	def interrupted_read(_root: pathlib.Path) -> local_stack_control.browser_suite_developer.DeveloperControlReceipt:
		raise KeyboardInterrupt
	monkeypatch.setattr(
		local_stack_control.browser_suite_developer, "read_control_receipt", interrupted_read
	)
	with pytest.raises(
		local_stack_control.browser_suite_developer.DeveloperBrowserSuiteError,
		match="interrupted",
	):
		local_stack_control.browser_suite_developer_start.start_developer_browser_suite(
			tmp_path,
			5.0,
			lambda _root, _lease: object(),
			lambda _child, _timeout: events.append("terminated"),
		)
	assert events == ["terminated", "reset"]


#============================================
class _PodmanRunner(local_stack_control.process.CommandRunner):
	"""Runner double answering only the Podman reachability probe."""

	def __init__(self, returncode: int, stdout: str, stderr: str) -> None:
		self.returncode = returncode
		self.stdout = stdout
		self.stderr = stderr
		self.argv: list[list[str]] = []

	def run(
		self,
		argv: list[str],
		environment: dict[str, str] | None = None,
		cwd: pathlib.Path | None = None,
		stdin: str | None = None,
	) -> local_stack_control.models.CommandResult:
		self.argv.append(argv)
		return local_stack_control.models.CommandResult(
			tuple(argv), self.returncode, self.stdout, self.stderr
		)

	def stream(
		self,
		argv: list[str],
		environment: dict[str, str] | None = None,
		cwd: pathlib.Path | None = None,
	) -> int:
		raise AssertionError("stream is not used by the reachability probe")


#============================================
def test_require_podman_reachable_names_the_recovery_command(tmp_path: pathlib.Path) -> None:
	"""A stopped Podman machine fails in seconds with the command that fixes it."""
	runner = _PodmanRunner(125, "", "Cannot connect to Podman. Please verify your connection\n")
	with pytest.raises(
		local_stack_control.browser_suite_developer.DeveloperBrowserSuiteError,
		match="Cannot connect to Podman.*podman machine start",
	):
		local_stack_control.browser_suite_developer_start.require_podman_reachable(runner, tmp_path)
	assert runner.argv == [["podman", "info", "--format", "{{.Host.Arch}}"]]
	local_stack_control.browser_suite_developer_start.require_podman_reachable(
		_PodmanRunner(0, "arm64\n", ""), tmp_path
	)

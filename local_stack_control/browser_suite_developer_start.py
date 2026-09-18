"""Parent-side start of the fixed Developer Browser Suite supervisor.

The supervisor itself lives in `browser_suite_developer`.  This module owns the
operator-facing wait: it spawns the supervisor, reports what the supervisor log says
it is doing, keeps waiting while that log keeps growing, and turns a dead, silent,
over-long, or interrupted start into one stated failure plus a reset suite.
"""

# Standard Library
import os
import sys
import time
import secrets
import pathlib
import subprocess
from collections.abc import Callable

# local repo modules
import local_stack_control.browser_suite_lease
import local_stack_control.browser_suite_developer
from local_stack_control.browser_suite_private_state import (
	CONTROL_NAME,
	LAUNCH_NAME,
	RESULT_NAME,
	DeveloperBrowserSuiteError,
	_checked_root_descriptor,
	_remove_private_entry,
	_write_private_file,
)
import local_stack_control.env_file
import local_stack_control.process


# A slow first start is not a failure; a silent one is.  The parent keeps waiting
# while the supervisor log keeps growing and gives up only after START_STALL_SECONDS
# without new output, or at this ceiling.  A cold Podman-machine build measured over
# ten minutes, so the ceiling is an operator recovery bound, not a performance gate.
DEVELOPER_START_WAIT_SECONDS = 3600.0
START_STALL_SECONDS = 300.0
# A first start builds Rust, pulls images, and installs the database; print progress so a
# quiet terminal is distinguishable from a stalled one.
START_HEARTBEAT_SECONDS = 15.0
# How long a terminated supervisor gets to stop its launch and clean up.
SUPERVISOR_TERMINATE_WAIT_SECONDS = 120.0
SUPERVISOR_LOG_TAIL_BYTES = 4096
MAXIMUM_HEARTBEAT_LINE_CHARACTERS = 120
DeveloperStartReceipt = local_stack_control.browser_suite_developer.DeveloperStartReceipt


#============================================
def supervisor_log_path(repository_root: pathlib.Path) -> pathlib.Path:
	"""Return the private supervisor log beside the fixed suite's state."""
	result = (
		repository_root
		/ local_stack_control.browser_suite_lease.LIVE_DEMO_BROWSER_STATE_DIRECTORY
		/ local_stack_control.browser_suite_developer.SUPERVISOR_LOG_NAME
	)
	return result


#============================================
def log_tail(log_path: pathlib.Path, known_phase: str = "starting") -> tuple[str, str]:
	"""Return the newest phase marker and last non-empty line of a bounded log tail.

	A long build pushes the last `[phase]` marker out of the bounded tail, so the
	caller passes the phase it saw last and gets it back when the tail has none.
	"""
	try:
		with log_path.open("rb") as log:
			log.seek(0, os.SEEK_END)
			size = log.tell()
			log.seek(max(0, size - SUPERVISOR_LOG_TAIL_BYTES))
			tail = log.read().decode("ascii", "replace")
	except OSError:
		return known_phase, ""
	phase = known_phase
	last_line = ""
	for line in tail.splitlines():
		stripped = line.strip()
		if stripped == "":
			continue
		if stripped.startswith("[phase] "):
			phase = stripped[len("[phase] "):]
		last_line = stripped
	return phase, last_line[:MAXIMUM_HEARTBEAT_LINE_CHARACTERS]


#============================================
def _log_progress(log_path: pathlib.Path) -> tuple[int, int] | None:
	"""Return the log size and mtime as evidence that the supervisor is still working."""
	try:
		metadata = log_path.stat()
	except OSError:
		return None
	return metadata.st_size, metadata.st_mtime_ns


#============================================
def require_podman_reachable(
	runner: local_stack_control.process.CommandRunner,
	repository_root: pathlib.Path,
) -> None:
	"""Fail within seconds, with the recovery command, when Podman cannot answer.

	The macOS machine image carries its own server version, so a client/server
	version difference is normal after a Homebrew upgrade and is not checked here.
	A forwarder left running from the pre-upgrade install is fixed by
	`podman machine stop; podman machine start`; readiness failure evidence in the
	supervisor log is where that shows up.
	"""
	environment = local_stack_control.env_file.sanitized_runtime_environment(
		local_stack_control.process.current_environment()
	)
	result = runner.run(["podman", "info", "--format", "{{.Host.Arch}}"], environment, repository_root)
	if result.ok():
		return
	detail = result.stderr.strip().splitlines()
	cause = detail[0] if detail else "podman info exited " + str(result.returncode)
	raise DeveloperBrowserSuiteError(
		"Podman is not reachable (" + cause + "); start it with 'podman machine start' and retry"
	)


#============================================
def _terminate_child(child: object, timeout_seconds: float) -> None:
	"""Terminate the exact spawned supervisor before reclaiming its inherited lease."""
	if not isinstance(child, subprocess.Popen):
		raise DeveloperBrowserSuiteError("developer browser supervisor handle is invalid")
	if child.poll() is None:
		child.terminate()
		try:
			child.wait(timeout=timeout_seconds)
		except subprocess.TimeoutExpired:
			child.kill()
			child.wait(timeout=timeout_seconds)


#============================================
def _recover_failed_start(repository_root: pathlib.Path) -> None:
	"""Reacquire the browser-suite lease and prove the fixed live-demo project is empty."""
	local_stack_control.browser_suite_developer.purge_orphaned_developer_browser_suite(
		repository_root,
		local_stack_control.process.SubprocessRunner(),
	)


#============================================
def _child_exited_before_ready(child: object) -> bool:
	"""Return whether a process-like supervisor exited before its ready receipt."""
	poll = getattr(child, "poll", None)
	return callable(poll) and poll() is not None


#============================================
def _child_returncode(child: object) -> int | None:
	"""Read only an already-complete supervisor's numeric exit result."""
	poll = getattr(child, "poll", None)
	result = poll() if callable(poll) else None
	return result if isinstance(result, int) else None


#============================================
def _print_start_heartbeat(elapsed: int, log_path: pathlib.Path, first: bool, known_phase: str) -> str:
	"""Tell the operator what the supervisor is doing right now; return the phase seen."""
	phase, last_line = log_tail(log_path, known_phase)
	detail = last_line if last_line != "" else "waiting for the first supervisor output"
	print(f"Live Demo starting ({elapsed}s, phase {phase}): {detail}", flush=True)
	if first:
		print(f"  Follow the full log with: tail -f {log_path}", flush=True)
	return phase


#============================================
def _prepare_launch(repository_root: pathlib.Path) -> tuple[local_stack_control.browser_suite_lease.BrowserSuiteLease, str]:
	"""Hold the lease, clear stale receipts, and record the launch identity for one child."""
	developer = local_stack_control.browser_suite_developer
	# The probe shares the browser-suite lease. It makes stale receipts powerless
	# before any child can publish readiness (ASVS 15.4.2 and 15.4.3).
	lease = local_stack_control.browser_suite_lease.BrowserSuiteLease.acquire(repository_root)
	root_descriptor = -1
	launch_id = secrets.token_hex(32)
	try:
		developer._remove_failure_receipt(repository_root)
		root_descriptor = _checked_root_descriptor(repository_root)
		_remove_private_entry(root_descriptor, CONTROL_NAME)
		_remove_private_entry(root_descriptor, LAUNCH_NAME)
		_remove_private_entry(root_descriptor, RESULT_NAME)
		developer._remove_socket_path(repository_root)
		_write_private_file(root_descriptor, LAUNCH_NAME, developer._launch_value(launch_id))
	except BaseException:
		lease.release()
		raise
	finally:
		if root_descriptor >= 0:
			os.close(root_descriptor)
	return lease, launch_id


#============================================
def _spawn_supervisor(
	root: pathlib.Path,
	held_lease: local_stack_control.browser_suite_lease.BrowserSuiteLease,
	without_live_demo: bool,
) -> object:
	"""Start the detached supervisor with its output appended to the private log."""
	descriptors = held_lease.inherited_descriptors()
	arguments = [
		sys.executable,
		"-m",
		"local_stack_control.browser_suite_developer",
		"supervisor",
		str(descriptors[0]),
		str(descriptors[1]),
		str(descriptors[2]),
	]
	if without_live_demo:
		arguments.append("--without-live-demo")
	# Keep supervisor diagnostics in a private log so a long first start is inspectable.
	log_descriptor = os.open(
		supervisor_log_path(root),
		os.O_WRONLY | os.O_CREAT | os.O_APPEND,
		0o600,
	)
	try:
		return subprocess.Popen(
			arguments,
			cwd=root,
			stdin=subprocess.DEVNULL,
			stdout=log_descriptor,
			stderr=log_descriptor,
			close_fds=True,
			pass_fds=descriptors,
			start_new_session=True,
		)
	finally:
		os.close(log_descriptor)


#============================================
def _wait_for_ready(
	repository_root: pathlib.Path,
	child: object,
	launch_id: str,
	timeout_seconds: float,
	stall_seconds: float,
) -> tuple[DeveloperStartReceipt | None, str, BaseException | None, str]:
	"""Wait for the ready receipt and say why the wait ended when it did not arrive.

	Returns the receipt, the ending, the last receipt failure, and the last phase seen.
	The ending is "" for ready, else "exited", "stalled", "ceiling", or "interrupted".
	"""
	developer = local_stack_control.browser_suite_developer
	failure: BaseException | None = None
	result: DeveloperStartReceipt | None = None
	ending = ""
	phase = "starting"
	log_path = supervisor_log_path(repository_root)
	started = time.monotonic()
	deadline = started + timeout_seconds
	next_heartbeat = started + START_HEARTBEAT_SECONDS
	heartbeats = 0
	progress = _log_progress(log_path)
	last_progress = started
	try:
		while True:
			now = time.monotonic()
			try:
				receipt = developer.read_control_receipt(repository_root)
				if not secrets.compare_digest(receipt.launch_id, launch_id):
					raise DeveloperBrowserSuiteError("developer browser supervisor published another launch")
				result = DeveloperStartReceipt(receipt.origin, receipt.project)
				break
			except DeveloperBrowserSuiteError as error:
				failure = error
			if _child_exited_before_ready(child):
				ending = "exited"
				break
			# New log bytes are the evidence that a slow build is still a live build.
			current = _log_progress(log_path)
			if current != progress:
				progress = current
				last_progress = now
			if now - last_progress >= stall_seconds:
				ending = "stalled"
				break
			if now >= deadline:
				ending = "ceiling"
				break
			if now >= next_heartbeat:
				phase = _print_start_heartbeat(int(now - started), log_path, heartbeats == 0, phase)
				heartbeats += 1
				next_heartbeat += START_HEARTBEAT_SECONDS
			time.sleep(0.05)
	except KeyboardInterrupt:
		ending = "interrupted"
		print("Interrupted; stopping the Live Demo supervisor and resetting the suite...", flush=True)
	phase, _last_line = log_tail(log_path, phase)
	return result, ending, failure, phase


#============================================
def start_developer_browser_suite(
	repository_root: pathlib.Path,
	timeout_seconds: float = DEVELOPER_START_WAIT_SECONDS,
	spawn: Callable[[pathlib.Path, local_stack_control.browser_suite_lease.BrowserSuiteLease], object] | None = None,
	child_terminator: Callable[[object, float], None] = _terminate_child,
	without_live_demo: bool = False,
	stall_seconds: float = START_STALL_SECONDS,
) -> DeveloperStartReceipt:
	"""Launch the background lease owner and return only its fixed HTTPS origin.

	The wait ends on the ready receipt, on the child exiting, on `stall_seconds`
	without new supervisor log output, on the `timeout_seconds` ceiling, or on
	Ctrl-C.  Every ending but the first terminates the child and resets the suite.
	"""
	developer = local_stack_control.browser_suite_developer
	if timeout_seconds <= 0 or stall_seconds <= 0:
		raise DeveloperBrowserSuiteError("developer browser start timeout is invalid")
	lease, launch_id = _prepare_launch(repository_root)
	def default_spawn(
		root: pathlib.Path,
		held_lease: local_stack_control.browser_suite_lease.BrowserSuiteLease,
	) -> object:
		return _spawn_supervisor(root, held_lease, without_live_demo)
	launcher = default_spawn if spawn is None else spawn
	handoff_started = False
	child: object | None = None
	try:
		child = launcher(repository_root, lease)
		handoff_started = True
	finally:
		if handoff_started:
			lease.detach_for_supervisor_handoff()
		else:
			lease.release()
	result, ending, failure, phase = _wait_for_ready(
		repository_root, child, launch_id, timeout_seconds, stall_seconds
	)
	if result is not None:
		return result
	if child is None:
		raise DeveloperBrowserSuiteError("developer browser supervisor did not start")
	log_path = supervisor_log_path(repository_root)
	_phase, last_line = log_tail(log_path, phase)
	cleanup_failures: list[BaseException] = []
	try:
		child_terminator(child, SUPERVISOR_TERMINATE_WAIT_SECONDS)
	except BaseException as error:
		cleanup_failures.append(error)
	try:
		_recover_failed_start(repository_root)
	except BaseException as error:
		cleanup_failures.append(error)
	if cleanup_failures:
		raise BaseExceptionGroup("developer browser failed-start cleanup failures", cleanup_failures)
	receipt = developer._read_failure_receipt(repository_root)
	if receipt is not None and ending == "exited":
		returncode = _child_returncode(child)
		if receipt.returncode != returncode:
			receipt = developer.DeveloperFailureReceipt(receipt.phase, returncode, receipt.diagnostic)
			developer._write_failure_receipt(repository_root, receipt)
		exit_detail = "unknown" if receipt.returncode is None else str(receipt.returncode)
		raise DeveloperBrowserSuiteError(
			"developer browser supervisor failed during " + receipt.phase
			+ " (exit " + exit_detail + "): " + receipt.diagnostic
		)
	evidence = "; last log line: " + last_line if last_line != "" else ""
	if ending == "stalled":
		raise DeveloperBrowserSuiteError(
			f"developer browser supervisor stalled during {phase}: no supervisor output for "
			f"{int(stall_seconds)}s{evidence}; see {log_path}"
		)
	if ending == "ceiling":
		raise DeveloperBrowserSuiteError(
			f"developer browser supervisor exceeded the {int(timeout_seconds)}s start ceiling "
			f"during {phase}{evidence}; see {log_path}"
		)
	if ending == "interrupted":
		raise DeveloperBrowserSuiteError(
			f"Live Demo start interrupted during {phase}; the supervisor was stopped and the suite reset"
		)
	if failure is not None:
		raise DeveloperBrowserSuiteError("developer browser supervisor did not become ready") from failure
	raise DeveloperBrowserSuiteError("developer browser supervisor did not become ready")

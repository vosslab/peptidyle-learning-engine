"""Owner-created browser children for bounded real-stack scenario protocols."""

import dataclasses
import errno
import os
import pathlib
import secrets
import signal
import subprocess
import time

import local_stack_control.env_file
import local_stack_control.process


class BrowserChildError(RuntimeError):
	"""An owner-created browser child escaped its bounded process lifecycle."""


@dataclasses.dataclass
class BrowserChild:
	"""One Playwright child and its leak-inspection identity."""

	process: subprocess.Popen[bytes]
	session: local_stack_control.process.ProcessSession


def launch(
	argv: list[str],
	environment: dict[str, str],
	root: pathlib.Path,
) -> BrowserChild:
	"""Start one inherited-output Playwright child with owner-selected private environment."""
	marker = "ple-owner-" + secrets.token_hex(16)
	child_environment = dict(environment)
	child_environment["PLE_BROWSER_SUITE_OWNER_SESSION"] = marker
	child_environment = local_stack_control.env_file.sanitized_runtime_environment(
		child_environment
	)
	process = subprocess.Popen(
		argv,
		cwd=root,
		env=child_environment,
		start_new_session=True,
	)
	session = local_stack_control.process.ProcessSession(
		process.pid, time.time_ns(), "process-group-or-marker", marker
	)
	return BrowserChild(process, session)


def _group_is_live(process_group_id: int) -> bool:
	"""Check the unique child process group without enumerating unrelated processes."""
	try:
		os.killpg(process_group_id, 0)
	except ProcessLookupError:
		return False
	except PermissionError as error:
		raise BrowserChildError("browser child process group cannot be inspected") from error
	return True


def _terminate_group(process_group_id: int, signal_value: signal.Signals) -> None:
	"""Signal every member of the private start-new-session process group."""
	try:
		os.killpg(process_group_id, signal_value)
	except ProcessLookupError:
		return
	except OSError as error:
		if error.errno != errno.ESRCH:
			raise BrowserChildError("browser child process group cannot be terminated") from error


def _marker_descendant_is_live(marker: str) -> bool:
	"""Find escaped descendants by the private owner marker without exposing command text."""
	if marker == "":
		return False
	probe = subprocess.Popen(
		["ps", "-axeww", "-o", "command="],
		stdout=subprocess.PIPE,
		stderr=subprocess.PIPE,
		text=True,
	)
	stdout, _stderr = probe.communicate()
	if probe.returncode != 0:
		raise BrowserChildError("browser child marker descendants cannot be inspected")
	return any(marker in line for line in stdout.splitlines())


def _clear_lingering_group(process_group_id: int, timeout_seconds: float) -> None:
	"""Terminate descendants left by an already-exited leader within one bounded cleanup window."""
	if not _group_is_live(process_group_id):
		return
	_terminate_group(process_group_id, signal.SIGTERM)
	deadline = time.monotonic() + timeout_seconds
	while time.monotonic() < deadline:
		if not _group_is_live(process_group_id):
			return
		time.sleep(0.05)
	_terminate_group(process_group_id, signal.SIGKILL)
	deadline = time.monotonic() + timeout_seconds
	while time.monotonic() < deadline:
		if not _group_is_live(process_group_id):
			return
		time.sleep(0.05)
	if _group_is_live(process_group_id):
		raise BrowserChildError("browser child process group remains after reaping")


def reap(child: BrowserChild, timeout_seconds: float) -> int:
	"""Boundedly reap a child, then terminate its complete private process group if needed."""
	if timeout_seconds <= 0:
		raise BrowserChildError("browser child reap timeout is invalid")
	try:
		returncode = child.process.wait(timeout=timeout_seconds)
	except subprocess.TimeoutExpired:
		_terminate_group(child.session.process_group_id, signal.SIGTERM)
		try:
			returncode = child.process.wait(timeout=timeout_seconds)
		except subprocess.TimeoutExpired:
			_terminate_group(child.session.process_group_id, signal.SIGKILL)
			returncode = child.process.wait(timeout=timeout_seconds)
	_clear_lingering_group(child.session.process_group_id, timeout_seconds)
	if _marker_descendant_is_live(child.session.owner_marker):
		raise BrowserChildError("browser child marker descendant remains after reaping")
	return returncode

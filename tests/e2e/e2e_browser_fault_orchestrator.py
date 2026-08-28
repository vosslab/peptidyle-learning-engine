"""Closed owner-child protocol for the real learner gateway-recovery journey."""

import dataclasses
import fcntl
import json
import os
import pathlib
import re
import secrets
import socket
import stat
from collections.abc import Callable

import local_stack_control.process

import e2e_browser_suite_children

MAXIMUM_MARKER_BYTES = 1_024
MAXIMUM_MESSAGE_BYTES = 256
MAXIMUM_SOCKET_PATH_BYTES = 100
PROTOCOL_VERSION = 1
PROTOCOL_TIMEOUT_SECONDS = 600.0
SHORT_PROTOCOL_DIRECTORY = pathlib.Path("/private/tmp/ple-live-demo-browser-fault")
TOKEN_LENGTH = 43
SOCKET_NAME_PATTERN = "fault-"
SOCKET_NAME = re.compile(r"^fault-[0-9a-f]{24}\.sock$")
GATEWAY_SUBMIT_OUTAGE_PHASES = (
	"response_selected",
	"gateway_stopped",
	"network_recovery_visible",
	"gateway_recovered",
	"completed",
)
DETERMINISTIC_GRADER_EXCEPTION_PHASES = (
	"submission_ready",
	"ordinary_worker_stopped",
	"accepted_pending_visible",
	"fault_worker_started",
	"fault_worker_exception_visible",
	"instructor_retry_visible",
	"ordinary_worker_recovered",
	"completed",
)
PHASES = GATEWAY_SUBMIT_OUTAGE_PHASES
ALL_PHASES = frozenset(
	GATEWAY_SUBMIT_OUTAGE_PHASES + DETERMINISTIC_GRADER_EXCEPTION_PHASES
)


class FaultProtocolError(RuntimeError):
	"""A private fault handshake violated its fixed scenario protocol."""


@dataclasses.dataclass(frozen=True)
class FaultScenarioRequest:
	"""The owner-only values needed for one gateway-recovery browser child."""

	root: pathlib.Path
	private_directory: pathlib.Path
	manifest_path: pathlib.Path
	scenario_id: str
	namespace: str
	playwright_argv: list[str]
	playwright_environment: dict[str, str]


@dataclasses.dataclass(frozen=True)
class FaultScenarioResult:
	"""Safe public projection of the lifecycle-controlled transition."""

	fault_transition: str
	fault_injected: bool
	fault_recovered: bool
	child_session: local_stack_control.process.ProcessSession


Command = Callable[[list[str]], local_stack_control.process.SessionCommandResult]
ChildLauncher = Callable[[list[str], dict[str, str], pathlib.Path], e2e_browser_suite_children.BrowserChild]
SessionRecorder = Callable[[local_stack_control.process.ProcessSession], None]
FileIdentity = tuple[int, int]


@dataclasses.dataclass
class ProtocolDirectory:
	"""One held fixed short-path directory for a bounded owner-child handshake."""

	path: pathlib.Path
	directory_descriptor: int
	directory_identity: FileIdentity
	lock_descriptor: int


def _identity(metadata: os.stat_result) -> FileIdentity:
	return metadata.st_dev, metadata.st_ino


def _require_owner_private(metadata: os.stat_result, kind: str) -> FileIdentity:
	if (metadata.st_mode & 0o777) != 0o600:
		raise FaultProtocolError(f"fault protocol {kind} mode is invalid")
	if hasattr(os, "getuid") and metadata.st_uid != os.getuid():
		raise FaultProtocolError(f"fault protocol {kind} ownership is invalid")
	return _identity(metadata)


def _private_file(path: pathlib.Path, content: str) -> None:
	file_descriptor = os.open(path, os.O_WRONLY | os.O_CREAT | os.O_EXCL, 0o600)
	try:
		output = content.encode("ascii")
		written = 0
		while written < len(output):
			written += os.write(file_descriptor, output[written:])
		os.fsync(file_descriptor)
	finally:
		os.close(file_descriptor)


def _token() -> str:
	value = secrets.token_urlsafe(32)
	if len(value) != TOKEN_LENGTH:
		raise FaultProtocolError("fault protocol token has an invalid length")
	return value


def _marker_value(request: FaultScenarioRequest, phase: str, token: str) -> dict[str, object]:
	return {
		"kind": "phase",
		"namespace": request.namespace,
		"phase": phase,
		"scenarioId": request.scenario_id,
		"token": token,
		"version": PROTOCOL_VERSION,
	}


def _marker_path(
	directory: pathlib.Path,
	phase: str,
	allowed_phases: tuple[str, ...] = PHASES,
) -> pathlib.Path:
	if phase not in allowed_phases:
		raise FaultProtocolError("fault protocol phase is invalid")
	return directory / f"fault-{phase}.json"


def _canonical(value: object) -> str:
	return json.dumps(value, sort_keys=True, separators=(",", ":"), ensure_ascii=True)


def _write_marker(
	directory: pathlib.Path,
	request: FaultScenarioRequest,
	phase: str,
	token: str,
	allowed_phases: tuple[str, ...] = PHASES,
) -> None:
	_private_file(
		_marker_path(directory, phase, allowed_phases),
		_canonical(_marker_value(request, phase, token)),
	)


def _read_bounded_file(file_descriptor: int, size: int) -> str:
	contents = bytearray()
	while len(contents) <= MAXIMUM_MARKER_BYTES:
		part = os.read(file_descriptor, MAXIMUM_MARKER_BYTES + 1 - len(contents))
		if part == b"":
			break
		contents.extend(part)
	if len(contents) > MAXIMUM_MARKER_BYTES or len(contents) != size:
		raise FaultProtocolError("fault protocol marker is not a bounded stable file")
	try:
		return contents.decode("ascii")
	except UnicodeDecodeError as error:
		raise FaultProtocolError("fault protocol marker is malformed") from error


def _require_marker(
	directory: pathlib.Path,
	request: FaultScenarioRequest,
	phase: str,
	token: str,
	allowed_phases: tuple[str, ...] = PHASES,
) -> None:
	"""Read one marker descriptor-relatively without following path replacements."""
	path = _marker_path(directory, phase, allowed_phases)
	directory_descriptor = -1
	file_descriptor = -1
	try:
		directory_descriptor = os.open(directory, os.O_RDONLY | os.O_DIRECTORY | os.O_NOFOLLOW)
		file_descriptor = os.open(path.name, os.O_RDONLY | os.O_NOFOLLOW, dir_fd=directory_descriptor)
		metadata = os.fstat(file_descriptor)
		if not stat.S_ISREG(metadata.st_mode) or metadata.st_size > MAXIMUM_MARKER_BYTES:
			raise FaultProtocolError("fault protocol marker is not a bounded regular file")
		_require_owner_private(metadata, "marker")
		contents = _read_bounded_file(file_descriptor, metadata.st_size)
	except OSError as error:
		raise FaultProtocolError("fault protocol marker is unavailable") from error
	finally:
		if file_descriptor >= 0:
			os.close(file_descriptor)
		if directory_descriptor >= 0:
			os.close(directory_descriptor)
	try:
		value = json.loads(contents)
	except json.JSONDecodeError as error:
		raise FaultProtocolError("fault protocol marker is malformed") from error
	if value != _marker_value(request, phase, token):
		raise FaultProtocolError("fault protocol marker identity is invalid")
	if contents != _canonical(value):
		raise FaultProtocolError("fault protocol marker is not canonical")


def _require_marker_order(
	directory: pathlib.Path,
	completed: tuple[str, ...],
	allowed_phases: tuple[str, ...] = PHASES,
) -> None:
	if any(phase not in allowed_phases for phase in completed):
		raise FaultProtocolError("fault protocol marker order is invalid")
	expected = {f"fault-{phase}.json" for phase in completed}
	actual = {path.name for path in directory.iterdir()}
	if "fault.lock" not in actual:
		if actual != expected:
			raise FaultProtocolError("fault protocol marker order is invalid")
		return
	sockets = {name for name in actual if SOCKET_NAME.fullmatch(name) is not None}
	if actual != expected | sockets | {"fault.lock"} or len(sockets) != 1:
		raise FaultProtocolError("fault protocol marker order is invalid")


def _message(phase: str, token: str) -> bytes:
	value = {"kind": "phase", "phase": phase, "token": token, "version": PROTOCOL_VERSION}
	return _canonical(value).encode("ascii") + b"\n"


def _authentication(request: FaultScenarioRequest, token: str) -> bytes:
	value = {
		"kind": "hello",
		"namespace": request.namespace,
		"scenarioId": request.scenario_id,
		"token": token,
		"version": PROTOCOL_VERSION,
	}
	return _canonical(value).encode("ascii") + b"\n"


def _accepted(token: str) -> bytes:
	return _canonical({"kind": "accepted", "token": token, "version": PROTOCOL_VERSION}).encode("ascii") + b"\n"


def _send(channel: socket.socket, phase: str, token: str) -> None:
	channel.sendall(_message(phase, token))


def _receive_line(channel: socket.socket, description: str) -> bytes:
	data = bytearray()
	while True:
		part = channel.recv(MAXIMUM_MESSAGE_BYTES + 1 - len(data))
		if part == b"":
			raise FaultProtocolError(f"fault protocol socket closed before {description}")
		data.extend(part)
		if len(data) > MAXIMUM_MESSAGE_BYTES:
			raise FaultProtocolError(f"fault protocol {description} is too large")
		boundary = data.find(b"\n")
		if boundary >= 0:
			if boundary != len(data) - 1:
				raise FaultProtocolError(f"fault protocol {description} has trailing data")
			return bytes(data)


def _require_exact_json(data: bytes, expected: dict[str, object], description: str) -> None:
	try:
		value = json.loads(data[:-1].decode("ascii"))
	except (UnicodeDecodeError, json.JSONDecodeError) as error:
		raise FaultProtocolError(f"fault protocol {description} is malformed") from error
	canonical = _canonical(expected).encode("ascii") + b"\n"
	if value != expected or data != canonical:
		raise FaultProtocolError(f"fault protocol {description} identity is invalid")


def _receive(channel: socket.socket, phase: str, token: str) -> None:
	data = _receive_line(channel, "socket message")
	_require_exact_json(
		data,
		{"kind": "phase", "phase": phase, "token": token, "version": PROTOCOL_VERSION},
		"socket message",
	)


def _receive_accepted(channel: socket.socket, token: str) -> None:
	data = _receive_line(channel, "acceptance")
	_require_exact_json(
		data,
		{"kind": "accepted", "token": token, "version": PROTOCOL_VERSION},
		"acceptance",
	)


def _authenticate(channel: socket.socket, request: FaultScenarioRequest, token: str) -> None:
	data = _receive_line(channel, "authentication")
	expected = {
		"kind": "hello",
		"namespace": request.namespace,
		"scenarioId": request.scenario_id,
		"token": token,
		"version": PROTOCOL_VERSION,
	}
	_require_exact_json(data, expected, "authentication")
	channel.sendall(_accepted(token))


def _require_success(result: local_stack_control.process.SessionCommandResult, action: str) -> None:
	if result.returncode != 0:
		raise FaultProtocolError("fault protocol lifecycle " + action + " failed")


def _require_private_directory(metadata: os.stat_result) -> FileIdentity:
	if not stat.S_ISDIR(metadata.st_mode) or stat.S_ISLNK(metadata.st_mode):
		raise FaultProtocolError("fault protocol private directory is invalid")
	if (metadata.st_mode & 0o777) != 0o700:
		raise FaultProtocolError("fault protocol private directory mode is invalid")
	if hasattr(os, "getuid") and metadata.st_uid != os.getuid():
		raise FaultProtocolError("fault protocol private directory ownership is invalid")
	return _identity(metadata)


def _safe_protocol_entry(directory_descriptor: int, name: str) -> None:
	metadata = os.stat(name, dir_fd=directory_descriptor, follow_symlinks=False)
	if name in {f"fault-{phase}.json" for phase in ALL_PHASES}:
		if not stat.S_ISREG(metadata.st_mode):
			raise FaultProtocolError("fault protocol cleanup marker is invalid")
		_require_owner_private(metadata, "cleanup marker")
	elif SOCKET_NAME.fullmatch(name) is not None:
		if not stat.S_ISSOCK(metadata.st_mode):
			raise FaultProtocolError("fault protocol cleanup socket is invalid")
		_require_owner_private(metadata, "cleanup socket")
	else:
		raise FaultProtocolError("fault protocol private directory has an unexpected entry")
	os.unlink(name, dir_fd=directory_descriptor)


def _reset_protocol_entries(directory_descriptor: int) -> None:
	for name in os.listdir(directory_descriptor):
		if name != "fault.lock":
			_safe_protocol_entry(directory_descriptor, name)
	entries = tuple(os.listdir(directory_descriptor))
	if entries != ("fault.lock",):
		raise FaultProtocolError("fault protocol private directory is not lock-only")


def _create_protocol_directory() -> ProtocolDirectory:
	parent = SHORT_PROTOCOL_DIRECTORY.parent
	parent_descriptor = -1
	directory_descriptor = -1
	lock_descriptor = -1
	try:
		parent_descriptor = os.open(parent, os.O_RDONLY | os.O_DIRECTORY | os.O_NOFOLLOW)
		try:
			os.mkdir(SHORT_PROTOCOL_DIRECTORY.name, 0o700, dir_fd=parent_descriptor)
		except FileExistsError:
			pass
		metadata = os.stat(
			SHORT_PROTOCOL_DIRECTORY.name, dir_fd=parent_descriptor, follow_symlinks=False
		)
		directory_identity = _require_private_directory(metadata)
		directory_descriptor = os.open(
			SHORT_PROTOCOL_DIRECTORY.name,
			os.O_RDONLY | os.O_DIRECTORY | os.O_NOFOLLOW,
			dir_fd=parent_descriptor,
		)
		if _require_private_directory(os.fstat(directory_descriptor)) != directory_identity:
			raise FaultProtocolError("fault protocol private directory identity changed")
		lock_descriptor = os.open(
			"fault.lock",
			os.O_RDWR | os.O_CREAT | os.O_NOFOLLOW,
			0o600,
			dir_fd=directory_descriptor,
		)
		lock_metadata = os.fstat(lock_descriptor)
		if not stat.S_ISREG(lock_metadata.st_mode):
			raise FaultProtocolError("fault protocol lock is invalid")
		_require_owner_private(lock_metadata, "lock")
		fcntl.flock(lock_descriptor, fcntl.LOCK_EX | fcntl.LOCK_NB)
		_reset_protocol_entries(directory_descriptor)
		result = ProtocolDirectory(
			SHORT_PROTOCOL_DIRECTORY,
			directory_descriptor,
			directory_identity,
			lock_descriptor,
		)
		directory_descriptor = -1
		lock_descriptor = -1
		return result
	except OSError as error:
		raise FaultProtocolError("fault protocol private directory is unavailable") from error
	finally:
		if lock_descriptor >= 0:
			os.close(lock_descriptor)
		if directory_descriptor >= 0:
			os.close(directory_descriptor)
		if parent_descriptor >= 0:
			os.close(parent_descriptor)


def _socket_path(directory: pathlib.Path) -> pathlib.Path:
	path = directory / f"{SOCKET_NAME_PATTERN}{secrets.token_hex(12)}.sock"
	if len(os.fsencode(path)) > MAXIMUM_SOCKET_PATH_BYTES:
		raise FaultProtocolError("fault protocol socket path is too long")
	return path


def _socket_identity(path: pathlib.Path) -> FileIdentity:
	metadata = path.lstat()
	if not stat.S_ISSOCK(metadata.st_mode):
		raise FaultProtocolError("fault protocol socket is invalid")
	return _require_owner_private(metadata, "socket")


def _unlink_socket(path: pathlib.Path, expected_identity: FileIdentity) -> None:
	if _socket_identity(path) != expected_identity:
		raise FaultProtocolError("fault protocol socket identity changed")
	path.unlink()


def _close_protocol_descriptors(protocol: ProtocolDirectory) -> None:
	"""Release the lock and directory descriptors after every cleanup outcome."""
	lock_descriptor = protocol.lock_descriptor
	directory_descriptor = protocol.directory_descriptor
	protocol.lock_descriptor = -1
	protocol.directory_descriptor = -1
	try:
		if lock_descriptor >= 0:
			fcntl.flock(lock_descriptor, fcntl.LOCK_UN)
	finally:
		try:
			if lock_descriptor >= 0:
				os.close(lock_descriptor)
		finally:
			if directory_descriptor >= 0:
				os.close(directory_descriptor)


def _remove_protocol_directory(
	protocol: ProtocolDirectory,
	endpoint_path: pathlib.Path | None = None,
	expected_endpoint_identity: FileIdentity | None = None,
	endpoint_quarantined: bool = False,
) -> None:
	"""Remove a lock-only directory, preserving any rejected endpoint replacement."""
	parent_descriptor = -1
	try:
		if _require_private_directory(os.fstat(protocol.directory_descriptor)) != protocol.directory_identity:
			raise FaultProtocolError("fault protocol private directory identity changed")
		if endpoint_quarantined:
			raise FaultProtocolError("fault protocol endpoint replacement is quarantined")
		if endpoint_path is not None and endpoint_path.exists():
			if expected_endpoint_identity is None:
				raise FaultProtocolError("fault protocol endpoint identity is unavailable")
			if _socket_identity(endpoint_path) != expected_endpoint_identity:
				raise FaultProtocolError("fault protocol endpoint identity changed")
		_reset_protocol_entries(protocol.directory_descriptor)
		os.unlink("fault.lock", dir_fd=protocol.directory_descriptor)
		os.fsync(protocol.directory_descriptor)
		_close_protocol_descriptors(protocol)
		parent_descriptor = os.open(
			protocol.path.parent, os.O_RDONLY | os.O_DIRECTORY | os.O_NOFOLLOW
		)
		metadata = os.stat(protocol.path.name, dir_fd=parent_descriptor, follow_symlinks=False)
		if _require_private_directory(metadata) != protocol.directory_identity:
			raise FaultProtocolError("fault protocol private directory identity changed")
		os.rmdir(protocol.path.name, dir_fd=parent_descriptor)
	except OSError as error:
		raise FaultProtocolError("fault protocol private directory cleanup failed") from error
	finally:
		if parent_descriptor >= 0:
			os.close(parent_descriptor)
		if protocol.lock_descriptor >= 0:
			_close_protocol_descriptors(protocol)


def reset_stale_protocol_directory() -> bool:
	"""Remove one lock-safe stale fault channel before a held suite run."""
	try:
		SHORT_PROTOCOL_DIRECTORY.lstat()
	except FileNotFoundError:
		return True
	protocol = _create_protocol_directory()
	_remove_protocol_directory(protocol)
	return not SHORT_PROTOCOL_DIRECTORY.exists()


def require_protocol_directory_absent() -> bool:
	"""Require final lifecycle cleanup to leave no fixed private fault directory."""
	try:
		SHORT_PROTOCOL_DIRECTORY.lstat()
	except FileNotFoundError:
		return True
	raise FaultProtocolError("fault protocol private directory remains after cleanup")


def run_gateway_submit_outage(
	request: FaultScenarioRequest,
	run_command: Command,
	launch_child: ChildLauncher = e2e_browser_suite_children.launch,
	reap_child: Callable[[e2e_browser_suite_children.BrowserChild, float], int] = e2e_browser_suite_children.reap,
	record_session: SessionRecorder = lambda _session: None,
) -> FaultScenarioResult:
	"""Drive the only accepted gateway outage in strict UI-visible phase order."""
	protocol: ProtocolDirectory | None = None
	directory: pathlib.Path | None = None
	socket_path: pathlib.Path | None = None
	socket_identity: FileIdentity | None = None
	listener: socket.socket | None = None
	channel: socket.socket | None = None
	child: e2e_browser_suite_children.BrowserChild | None = None
	injected = False
	recovered = False
	child_reaped = False
	endpoint_quarantined = False
	failures: list[BaseException] = []
	try:
		protocol = _create_protocol_directory()
		directory = protocol.path
		socket_path = _socket_path(directory)
		listener = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
		listener.settimeout(PROTOCOL_TIMEOUT_SECONDS)
		listener.bind(str(socket_path))
		os.chmod(socket_path, 0o600)
		socket_identity = _socket_identity(socket_path)
		listener.listen(1)
		environment = dict(request.playwright_environment)
		environment["PLE_BROWSER_SUITE_FAULT_SOCKET_PATH"] = str(socket_path)
		token = _token()
		environment["PLE_BROWSER_SUITE_FAULT_TOKEN"] = token
		child = launch_child(request.playwright_argv, environment, request.root)
		record_session(child.session)
		channel, _address = listener.accept()
		channel.settimeout(PROTOCOL_TIMEOUT_SECONDS)
		_authenticate(channel, request, token)
		_receive(channel, "response_selected", token)
		_require_marker(directory, request, "response_selected", token)
		_require_marker_order(directory, ("response_selected",))
		_require_success(run_command(["stop-outage-service"]), "stop")
		injected = True
		_write_marker(directory, request, "gateway_stopped", token)
		_send(channel, "gateway_stopped", token)
		_receive(channel, "network_recovery_visible", token)
		_require_marker(directory, request, "network_recovery_visible", token)
		_require_marker_order(directory, ("response_selected", "gateway_stopped", "network_recovery_visible"))
		_require_success(
			run_command(["restart", "--service", "gateway", "--timeout-seconds", "240"]), "restart"
		)
		recovered = True
		_write_marker(directory, request, "gateway_recovered", token)
		_send(channel, "gateway_recovered", token)
		_receive(channel, "completed", token)
		_require_marker(directory, request, "completed", token)
		_require_marker_order(directory, PHASES)
		child_returncode = reap_child(child, PROTOCOL_TIMEOUT_SECONDS)
		child_reaped = True
		if child_returncode != 0:
			raise FaultProtocolError("fault protocol browser child failed")
	except BaseException as error:
		failures.append(error)
	finally:
		if injected and not recovered:
			try:
				_require_success(
					run_command(["restart", "--service", "gateway", "--timeout-seconds", "240"]),
					"recovery",
				)
				recovered = True
			except BaseException as error:
				failures.append(error)
		if child is not None and not child_reaped:
			try:
				reap_child(child, 5.0)
			except BaseException as error:
				failures.append(error)
		if channel is not None:
			try:
				channel.close()
			except BaseException as error:
				failures.append(error)
		if listener is not None:
			try:
				listener.close()
			except BaseException as error:
				failures.append(error)
		if socket_path is not None and socket_identity is not None:
			try:
				_unlink_socket(socket_path, socket_identity)
			except BaseException as error:
				endpoint_quarantined = True
				failures.append(error)
		if protocol is not None:
			try:
				_remove_protocol_directory(
					protocol,
					socket_path,
					socket_identity,
					endpoint_quarantined,
				)
			except BaseException as error:
				failures.append(error)
	if failures:
		if len(failures) == 1:
			raise failures[0]
		raise BaseExceptionGroup("fault protocol failures", failures)
	if child is None:
		raise FaultProtocolError("fault protocol did not launch a browser child")
	return FaultScenarioResult("gateway_submit_outage", injected, recovered, child.session)


def run_deterministic_grader_exception(
	request: FaultScenarioRequest,
	run_command: Command,
	launch_child: ChildLauncher = e2e_browser_suite_children.launch,
	reap_child: Callable[
		[e2e_browser_suite_children.BrowserChild, float], int
	] = e2e_browser_suite_children.reap,
	record_session: SessionRecorder = lambda _session: None,
) -> FaultScenarioResult:
	"""Drive the closed one-claim grader exception and visible Instructor retry.

	The browser never selects a database target or asks for a fault.  Its visible
	submission supplies the sole accepted execution; the profile-bound adapter
	stops the ordinary worker, starts the feature-only one-claim worker, then
	restores ordinary recovery after Elena's normal retry action.
	"""
	protocol: ProtocolDirectory | None = None
	directory: pathlib.Path | None = None
	socket_path: pathlib.Path | None = None
	socket_identity: FileIdentity | None = None
	listener: socket.socket | None = None
	channel: socket.socket | None = None
	child: e2e_browser_suite_children.BrowserChild | None = None
	worker_stopped = False
	worker_recovered = False
	child_reaped = False
	endpoint_quarantined = False
	failures: list[BaseException] = []
	try:
		protocol = _create_protocol_directory()
		directory = protocol.path
		socket_path = _socket_path(directory)
		listener = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
		listener.settimeout(PROTOCOL_TIMEOUT_SECONDS)
		listener.bind(str(socket_path))
		os.chmod(socket_path, 0o600)
		socket_identity = _socket_identity(socket_path)
		listener.listen(1)
		environment = dict(request.playwright_environment)
		environment["PLE_BROWSER_SUITE_FAULT_SOCKET_PATH"] = str(socket_path)
		token = _token()
		environment["PLE_BROWSER_SUITE_FAULT_TOKEN"] = token
		child = launch_child(request.playwright_argv, environment, request.root)
		record_session(child.session)
		channel, _address = listener.accept()
		channel.settimeout(PROTOCOL_TIMEOUT_SECONDS)
		_authenticate(channel, request, token)
		_receive(channel, "submission_ready", token)
		_require_marker(
			directory, request, "submission_ready", token,
			DETERMINISTIC_GRADER_EXCEPTION_PHASES,
		)
		_require_marker_order(
			directory, ("submission_ready",), DETERMINISTIC_GRADER_EXCEPTION_PHASES,
		)
		_require_success(run_command(["stop-outage-service"]), "stop ordinary worker")
		worker_stopped = True
		_write_marker(
			directory, request, "ordinary_worker_stopped", token,
			DETERMINISTIC_GRADER_EXCEPTION_PHASES,
		)
		_send(channel, "ordinary_worker_stopped", token)
		_receive(channel, "accepted_pending_visible", token)
		_require_marker(
			directory, request, "accepted_pending_visible", token,
			DETERMINISTIC_GRADER_EXCEPTION_PHASES,
		)
		_require_marker_order(
			directory,
			("submission_ready", "ordinary_worker_stopped", "accepted_pending_visible"),
			DETERMINISTIC_GRADER_EXCEPTION_PHASES,
		)
		_require_success(
			run_command(["run-automated-grading-fault-worker"]),
			"start deterministic fault worker",
		)
		_write_marker(
			directory, request, "fault_worker_started", token,
			DETERMINISTIC_GRADER_EXCEPTION_PHASES,
		)
		_send(channel, "fault_worker_started", token)
		_receive(channel, "fault_worker_exception_visible", token)
		_require_marker(
			directory, request, "fault_worker_exception_visible", token,
			DETERMINISTIC_GRADER_EXCEPTION_PHASES,
		)
		_require_marker_order(
			directory,
			(
				"submission_ready", "ordinary_worker_stopped", "accepted_pending_visible",
				"fault_worker_started", "fault_worker_exception_visible",
			),
			DETERMINISTIC_GRADER_EXCEPTION_PHASES,
		)
		_receive(channel, "instructor_retry_visible", token)
		_require_marker(
			directory, request, "instructor_retry_visible", token,
			DETERMINISTIC_GRADER_EXCEPTION_PHASES,
		)
		_require_success(
			run_command(["restart", "--service", "worker", "--timeout-seconds", "240"]),
			"restart ordinary worker",
		)
		worker_recovered = True
		_write_marker(
			directory, request, "ordinary_worker_recovered", token,
			DETERMINISTIC_GRADER_EXCEPTION_PHASES,
		)
		_send(channel, "ordinary_worker_recovered", token)
		_receive(channel, "completed", token)
		_require_marker(
			directory, request, "completed", token,
			DETERMINISTIC_GRADER_EXCEPTION_PHASES,
		)
		_require_marker_order(
			directory,
			DETERMINISTIC_GRADER_EXCEPTION_PHASES,
			DETERMINISTIC_GRADER_EXCEPTION_PHASES,
		)
		child_returncode = reap_child(child, PROTOCOL_TIMEOUT_SECONDS)
		child_reaped = True
		if child_returncode != 0:
			raise FaultProtocolError("fault protocol browser child failed")
	except BaseException as error:
		failures.append(error)
	finally:
		if worker_stopped and not worker_recovered:
			try:
				_require_success(
					run_command(["restart", "--service", "worker", "--timeout-seconds", "240"]),
					"restore ordinary worker",
				)
				worker_recovered = True
			except BaseException as error:
				failures.append(error)
		if child is not None and not child_reaped:
			try:
				reap_child(child, 5.0)
			except BaseException as error:
				failures.append(error)
		if channel is not None:
			try:
				channel.close()
			except BaseException as error:
				failures.append(error)
		if listener is not None:
			try:
				listener.close()
			except BaseException as error:
				failures.append(error)
		if socket_path is not None and socket_identity is not None:
			try:
				_unlink_socket(socket_path, socket_identity)
			except BaseException as error:
				endpoint_quarantined = True
				failures.append(error)
		if protocol is not None:
			try:
				_remove_protocol_directory(
					protocol, socket_path, socket_identity, endpoint_quarantined
				)
			except BaseException as error:
				failures.append(error)
	if failures:
		if len(failures) == 1:
			raise failures[0]
		raise BaseExceptionGroup("fault protocol failures", failures)
	if child is None:
		raise FaultProtocolError("fault protocol did not launch a browser child")
	return FaultScenarioResult("deterministic_grader_exception", True, worker_recovered, child.session)

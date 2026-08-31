"""Persistent fixed-owner lifecycle for the production live-demo browser stack."""

from __future__ import annotations

import dataclasses
import hashlib
import json
import os
import pathlib
import secrets
import signal
import socket
import stat
import subprocess
import sys
import time
from collections.abc import Callable

import local_stack_control.browser_suite_lease
import local_stack_control.browser_suite_reset
import local_stack_control.live_demo_target
import local_stack_control.env_file
import local_stack_control.models
import local_stack_control.process


CONTROL_NAME = "developer-control.json"
LAUNCH_NAME = "developer-launch.json"
RESULT_NAME = "developer-result.json"
SOCKET_NAME = "developer-control.sock"
SOCKET_DIRECTORY = pathlib.Path("/private/tmp") / "ple-live-demo-browser-control"
MAXIMUM_CONTROL_BYTES = 1024
LIFECYCLE_LAUNCH_TIMEOUT_SECONDS = 240.0
DEVELOPER_STOP_WAIT_SECONDS = 20.0
# The parent wait covers a clean host build before the child's separately
# bounded service-readiness stages. This is an operator recovery ceiling, not
# a startup-performance acceptance requirement.
DEVELOPER_START_WAIT_SECONDS = 600.0


class DeveloperBrowserSuiteError(local_stack_control.models.ControllerError):
	"""A concise fixed-owner developer lifecycle failure."""


@dataclasses.dataclass(frozen=True)
class DeveloperControlReceipt:
	"""Private capability-bearing receipt for one live supervisor only."""

	pid: int
	supervisor_id: str
	capability: str
	launch_id: str
	origin: str
	project: str


@dataclasses.dataclass(frozen=True)
class DeveloperStartReceipt:
	"""Safe result returned after the fixed HTTPS stack becomes ready."""

	origin: str
	project: str


@dataclasses.dataclass(frozen=True)
class DeveloperCompletionReceipt:
	"""Private completion evidence matching one authenticated stop request."""

	supervisor_id: str
	capability: str
	completed: bool


@dataclasses.dataclass(frozen=True)
class RunningDeveloperStack:
	"""Private launch state retained exclusively by the supervisor."""

	manifest_path: pathlib.Path
	origin: str


@dataclasses.dataclass(frozen=True)
class DeveloperOperations:
	"""External stack operations kept injectable for deterministic lifecycle tests."""

	start: Callable[[local_stack_control.browser_suite_lease.BrowserSuiteLease, pathlib.Path, pathlib.Path], RunningDeveloperStack]
	stop: Callable[[RunningDeveloperStack, pathlib.Path], None]
	verify_empty: Callable[[local_stack_control.browser_suite_lease.BrowserSuiteLease, pathlib.Path], None]


#============================================
def _checked_root_descriptor(repository_root: pathlib.Path) -> int:
	"""Open the immutable private root through the shared checked-lease authority."""
	try:
		descriptor, _identity = local_stack_control.browser_suite_lease._open_checked_directory(
			repository_root / local_stack_control.browser_suite_lease.LIVE_DEMO_BROWSER_STATE_DIRECTORY, 0o700
		)
	except local_stack_control.browser_suite_lease.BrowserSuiteError as error:
		raise DeveloperBrowserSuiteError("developer browser control state is unavailable") from error
	return descriptor


#============================================
def _require_control_name(name: str) -> None:
	"""Keep all developer control paths fixed below the checked private root."""
	if name not in (CONTROL_NAME, LAUNCH_NAME, RESULT_NAME, SOCKET_NAME):
		raise DeveloperBrowserSuiteError("developer browser control state is unavailable")


#============================================
def _write_private_file(root_descriptor: int, name: str, content: bytes) -> None:
	"""Atomically publish a bounded mode-0600 control receipt (ASVS 5.3.2)."""
	_require_control_name(name)
	if len(content) > MAXIMUM_CONTROL_BYTES:
		raise DeveloperBrowserSuiteError("developer browser control receipt is invalid")
	temporary = "." + name + ".new"
	try:
		file_descriptor = os.open(
			temporary,
			os.O_WRONLY | os.O_CREAT | os.O_EXCL | getattr(os, "O_NOFOLLOW", 0),
			0o600,
			dir_fd=root_descriptor,
		)
		with os.fdopen(file_descriptor, "wb") as output:
			output.write(content)
			output.flush()
			os.fsync(output.fileno())
		os.replace(temporary, name, src_dir_fd=root_descriptor, dst_dir_fd=root_descriptor)
		os.fsync(root_descriptor)
	except OSError as error:
		try:
			os.unlink(temporary, dir_fd=root_descriptor)
		except OSError:
			pass
		raise DeveloperBrowserSuiteError("developer browser control state is unavailable") from error


#============================================
def _read_private_file(repository_root: pathlib.Path, name: str) -> bytes:
	"""Read one fixed private receipt only after checking type, owner, and mode."""
	_require_control_name(name)
	root_descriptor = _checked_root_descriptor(repository_root)
	try:
		try:
			metadata = os.stat(name, dir_fd=root_descriptor, follow_symlinks=False)
			file_descriptor = os.open(name, os.O_RDONLY | getattr(os, "O_NOFOLLOW", 0), dir_fd=root_descriptor)
		except OSError as error:
			raise DeveloperBrowserSuiteError("developer browser session is not running") from error
		try:
			opened = os.fstat(file_descriptor)
			if (
				not stat.S_ISREG(metadata.st_mode)
				or metadata.st_uid != os.getuid()
				or stat.S_IMODE(metadata.st_mode) != 0o600
				or not stat.S_ISREG(opened.st_mode)
				or opened.st_uid != os.getuid()
				or stat.S_IMODE(opened.st_mode) != 0o600
				or (metadata.st_dev, metadata.st_ino) != (opened.st_dev, opened.st_ino)
			):
				raise DeveloperBrowserSuiteError("developer browser control state is unavailable")
			content = os.read(file_descriptor, MAXIMUM_CONTROL_BYTES + 1)
		finally:
			os.close(file_descriptor)
	finally:
		os.close(root_descriptor)
	if len(content) > MAXIMUM_CONTROL_BYTES:
		raise DeveloperBrowserSuiteError("developer browser control receipt is invalid")
	return content


#============================================
def _remove_private_entry(root_descriptor: int, name: str) -> None:
	"""Remove a fixed control artifact without following a replacement link."""
	_require_control_name(name)
	try:
		metadata = os.stat(name, dir_fd=root_descriptor, follow_symlinks=False)
	except FileNotFoundError:
		return
	except OSError as error:
		raise DeveloperBrowserSuiteError("developer browser control state is unavailable") from error
	if metadata.st_uid != os.getuid() or stat.S_IMODE(metadata.st_mode) & 0o022:
		raise DeveloperBrowserSuiteError("developer browser control state is unavailable")
	try:
		os.unlink(name, dir_fd=root_descriptor)
	except OSError as error:
		raise DeveloperBrowserSuiteError("developer browser control state is unavailable") from error


#============================================
def _control_value(receipt: DeveloperControlReceipt) -> bytes:
	"""Encode the private bounded control receipt with an unpredictable capability."""
	value = {
		"capability": receipt.capability,
		"launchId": receipt.launch_id,
		"origin": receipt.origin,
		"pid": receipt.pid,
		"project": receipt.project,
		"schemaVersion": 1,
		"supervisorId": receipt.supervisor_id,
	}
	result = json.dumps(value, sort_keys=True, separators=(",", ":"), ensure_ascii=True).encode("ascii")
	return result


#============================================
def read_control_receipt(repository_root: pathlib.Path) -> DeveloperControlReceipt:
	"""Decode the only accepted fixed developer control receipt."""
	try:
		value = json.loads(_read_private_file(repository_root, CONTROL_NAME).decode("ascii"))
	except (UnicodeDecodeError, json.JSONDecodeError) as error:
		raise DeveloperBrowserSuiteError("developer browser control receipt is invalid") from error
	if not isinstance(value, dict) or set(value) != {
		"capability", "launchId", "origin", "pid", "project", "schemaVersion", "supervisorId"
	}:
		raise DeveloperBrowserSuiteError("developer browser control receipt is invalid")
	if (
		value["schemaVersion"] != 1
		or value["project"] != local_stack_control.models.LIVE_DEMO_BROWSER_PROJECT
		or not isinstance(value["pid"], int)
		or value["pid"] < 1
	):
		raise DeveloperBrowserSuiteError("developer browser control receipt is invalid")
	for name in ("capability", "launchId", "supervisorId"):
		if not isinstance(value[name], str) or len(value[name]) != 64:
			raise DeveloperBrowserSuiteError("developer browser control receipt is invalid")
		try:
			bytes.fromhex(value[name])
		except ValueError as error:
			raise DeveloperBrowserSuiteError("developer browser control receipt is invalid") from error
	if not isinstance(value["origin"], str) or not value["origin"].startswith("https://localhost:"):
		raise DeveloperBrowserSuiteError("developer browser control receipt is invalid")
	result = DeveloperControlReceipt(
		value["pid"], value["supervisorId"], value["capability"], value["launchId"], value["origin"], value["project"]
	)
	return result


#============================================
def _completion_value(receipt: DeveloperCompletionReceipt) -> bytes:
	"""Encode the bounded private cleanup result for the authenticated stop caller."""
	value = {
		"capability": receipt.capability,
		"completed": receipt.completed,
		"schemaVersion": 1,
		"supervisorId": receipt.supervisor_id,
	}
	result = json.dumps(value, sort_keys=True, separators=(",", ":"), ensure_ascii=True).encode("ascii")
	return result


#============================================
def _read_completion_receipt(repository_root: pathlib.Path) -> DeveloperCompletionReceipt:
	"""Decode a single fixed completion receipt without accepting stale authority."""
	try:
		value = json.loads(_read_private_file(repository_root, RESULT_NAME).decode("ascii"))
	except (UnicodeDecodeError, json.JSONDecodeError) as error:
		raise DeveloperBrowserSuiteError("developer browser cleanup receipt is invalid") from error
	if not isinstance(value, dict) or set(value) != {
		"capability", "completed", "schemaVersion", "supervisorId"
	}:
		raise DeveloperBrowserSuiteError("developer browser cleanup receipt is invalid")
	if value["schemaVersion"] != 1 or not isinstance(value["completed"], bool):
		raise DeveloperBrowserSuiteError("developer browser cleanup receipt is invalid")
	for name in ("capability", "supervisorId"):
		if not isinstance(value[name], str) or len(value[name]) != 64:
			raise DeveloperBrowserSuiteError("developer browser cleanup receipt is invalid")
		try:
			bytes.fromhex(value[name])
		except ValueError as error:
			raise DeveloperBrowserSuiteError("developer browser cleanup receipt is invalid") from error
	result = DeveloperCompletionReceipt(value["supervisorId"], value["capability"], value["completed"])
	return result


#============================================
def _launch_value(launch_id: str) -> bytes:
	"""Encode the private launch identity that binds one parent to one supervisor."""
	value = {"launchId": launch_id, "schemaVersion": 1}
	result = json.dumps(value, sort_keys=True, separators=(",", ":"), ensure_ascii=True).encode("ascii")
	return result


#============================================
def _read_launch_id(repository_root: pathlib.Path) -> str:
	"""Read the one private launch identity created while the handoff lease was held."""
	try:
		value = json.loads(_read_private_file(repository_root, LAUNCH_NAME).decode("ascii"))
	except (UnicodeDecodeError, json.JSONDecodeError) as error:
		raise DeveloperBrowserSuiteError("developer browser launch receipt is invalid") from error
	if not isinstance(value, dict) or set(value) != {"launchId", "schemaVersion"}:
		raise DeveloperBrowserSuiteError("developer browser launch receipt is invalid")
	if value["schemaVersion"] != 1 or not isinstance(value["launchId"], str) or len(value["launchId"]) != 64:
		raise DeveloperBrowserSuiteError("developer browser launch receipt is invalid")
	try:
		bytes.fromhex(value["launchId"])
	except ValueError as error:
		raise DeveloperBrowserSuiteError("developer browser launch receipt is invalid") from error
	result = value["launchId"]
	return result


#============================================
def _socket_directory_descriptor() -> int:
	"""Open the dedicated mode-0700 current-user local-control directory."""
	try:
		SOCKET_DIRECTORY.mkdir(mode=0o700)
	except FileExistsError:
		pass
	try:
		descriptor, _identity = local_stack_control.browser_suite_lease._open_checked_directory(
			SOCKET_DIRECTORY, 0o700
		)
	except (OSError, local_stack_control.browser_suite_lease.BrowserSuiteError) as error:
		raise DeveloperBrowserSuiteError("developer browser control socket is unavailable") from error
	return descriptor


#============================================
def _socket_path(repository_root: pathlib.Path) -> pathlib.Path:
	"""Return a short fixed endpoint inside the checked private control directory."""
	repository_descriptor, _repository_identity = (
		local_stack_control.browser_suite_lease._open_checked_directory(
			repository_root,
			None,
		)
	)
	try:
		identity = os.fstat(repository_descriptor)
	finally:
		os.close(repository_descriptor)
	material = str(repository_root.resolve()) + ":" + str(identity.st_dev) + ":" + str(identity.st_ino)
	digest = hashlib.sha256(material.encode("utf-8")).hexdigest()[:24]
	result = SOCKET_DIRECTORY / (digest + ".sock")
	return result


#============================================
def _remove_socket_path(repository_root: pathlib.Path) -> None:
	"""Remove the fixed local endpoint only when it is this user's socket."""
	path = _socket_path(repository_root)
	directory_descriptor = _socket_directory_descriptor()
	try:
		metadata = os.stat(path.name, dir_fd=directory_descriptor, follow_symlinks=False)
	except FileNotFoundError:
		os.close(directory_descriptor)
		return
	except OSError as error:
		os.close(directory_descriptor)
		raise DeveloperBrowserSuiteError("developer browser control state is unavailable") from error
	if not stat.S_ISSOCK(metadata.st_mode) or metadata.st_uid != os.getuid() or stat.S_IMODE(metadata.st_mode) & 0o022:
		os.close(directory_descriptor)
		raise DeveloperBrowserSuiteError("developer browser control state is unavailable")
	try:
		os.unlink(path.name, dir_fd=directory_descriptor)
	except OSError as error:
		raise DeveloperBrowserSuiteError("developer browser control state is unavailable") from error
	finally:
		os.close(directory_descriptor)


#============================================
def _request_value(receipt: DeveloperControlReceipt) -> bytes:
	"""Encode a narrow stop request that authenticates the live socket peer."""
	value = {
		"action": "stop",
		"capability": receipt.capability,
		"supervisorId": receipt.supervisor_id,
	}
	result = json.dumps(value, sort_keys=True, separators=(",", ":"), ensure_ascii=True).encode("ascii")
	return result


#============================================
def _validate_stop_request(content: bytes, receipt: DeveloperControlReceipt) -> bool:
	"""Accept exactly the active capability and supervisor identity, never a PID alone."""
	if len(content) > MAXIMUM_CONTROL_BYTES:
		return False
	try:
		value = json.loads(content.decode("ascii"))
	except (UnicodeDecodeError, json.JSONDecodeError):
		return False
	if not isinstance(value, dict) or set(value) != {"action", "capability", "supervisorId"}:
		return False
	result = (
		value["action"] == "stop"
		and isinstance(value["capability"], str)
		and isinstance(value["supervisorId"], str)
		and secrets.compare_digest(value["capability"], receipt.capability)
		and secrets.compare_digest(value["supervisorId"], receipt.supervisor_id)
	)
	return result


#============================================
def request_stop(
	repository_root: pathlib.Path,
	timeout_seconds: float = DEVELOPER_STOP_WAIT_SECONDS,
) -> DeveloperStartReceipt:
	"""Request a bounded authenticated shutdown and wait for owner-validated cleanup."""
	if timeout_seconds <= 0:
		raise DeveloperBrowserSuiteError("developer browser stop timeout is invalid")
	receipt = read_control_receipt(repository_root)
	path = _socket_path(repository_root)
	directory_descriptor = _socket_directory_descriptor()
	try:
		metadata = os.stat(path.name, dir_fd=directory_descriptor, follow_symlinks=False)
		if not stat.S_ISSOCK(metadata.st_mode) or metadata.st_uid != os.getuid() or stat.S_IMODE(metadata.st_mode) & 0o022:
			raise DeveloperBrowserSuiteError("developer browser control state is unavailable")
		with socket.socket(socket.AF_UNIX, socket.SOCK_STREAM) as client:
			client.settimeout(timeout_seconds)
			client.connect(str(path))
			client.sendall(_request_value(receipt))
			response = client.recv(64)
	except OSError as error:
		raise DeveloperBrowserSuiteError("developer browser supervisor is unavailable") from error
	finally:
		os.close(directory_descriptor)
	if response != b'{"accepted":true}':
		raise DeveloperBrowserSuiteError("developer browser supervisor rejected stop request")
	deadline = time.monotonic() + timeout_seconds
	while time.monotonic() < deadline:
		try:
			completion = _read_completion_receipt(repository_root)
		except DeveloperBrowserSuiteError as error:
			if str(error) != "developer browser session is not running":
				time.sleep(0.05)
				continue
		else:
			if (
				secrets.compare_digest(completion.capability, receipt.capability)
				and secrets.compare_digest(completion.supervisor_id, receipt.supervisor_id)
			):
				if not completion.completed:
					raise DeveloperBrowserSuiteError("developer browser supervisor cleanup failed")
				result = DeveloperStartReceipt(receipt.origin, receipt.project)
				return result
		time.sleep(0.05)
	raise DeveloperBrowserSuiteError("developer browser supervisor did not complete cleanup")



#============================================
def _adapter_argv(action: str, manifest_path: pathlib.Path, arguments: tuple[str, ...] = ()) -> list[str]:
	"""Form one closed adapter command for the shared fixed target."""
	result = [
		sys.executable,
		"-m",
		"local_stack_control.disposable_stack_command",
		action,
		"--manifest",
		str(manifest_path),
	]
	result.extend(arguments)
	return result


#============================================
def default_operations() -> DeveloperOperations:
	"""Use the same production manifest, gateway, and auth path as Playwright."""
	def start_stack(
		lease: local_stack_control.browser_suite_lease.BrowserSuiteLease,
		root: pathlib.Path,
		workspace: pathlib.Path,
	) -> RunningDeveloperStack:
		"""Start the production browser stack and wait for its declared readiness."""
		runner = local_stack_control.process.SubprocessRunner()
		selections = local_stack_control.env_file.canonical_stack_selections(root)
		ports = local_stack_control.live_demo_target.random_ports()
		local_stack_control.process.require_available_loopback_ports(
			ports.as_tuple(), runner, root
		)
		target = local_stack_control.live_demo_target.write_private_target(
			workspace,
			local_stack_control.models.LiveDemoProfile.BROWSER,
			ports,
			selections,
		)
		local_stack_control.live_demo_target.validate_production_auth_render(
			runner, root, target.manifest_path
		)
		argv = _adapter_argv(
			"launch", target.manifest_path, ("--timeout-seconds", str(int(LIFECYCLE_LAUNCH_TIMEOUT_SECONDS)))
		)
		result = local_stack_control.process.stream_in_owner_session(
			runner, argv, None, root
		)
		if result.returncode != 0:
			raise DeveloperBrowserSuiteError("developer browser stack launch failed")
		return RunningDeveloperStack(target.manifest_path, target.origin)

	def stop_stack(running: RunningDeveloperStack, root: pathlib.Path) -> None:
		"""Stop the production browser stack and report cleanup failures."""
		result = local_stack_control.process.stream_in_owner_session(
			local_stack_control.process.SubprocessRunner(),
			_adapter_argv("cleanup", running.manifest_path),
			None,
			root,
		)
		if result.returncode != 0:
			raise DeveloperBrowserSuiteError("developer browser stack cleanup failed")

	def verify_empty_workspace(
		lease: local_stack_control.browser_suite_lease.BrowserSuiteLease,
		root: pathlib.Path,
	) -> None:
		"""Verify that owned resources and private workspace artifacts are absent."""
		snapshot = local_stack_control.browser_suite_reset.reset_live_demo_browser(
			lease, local_stack_control.process.SubprocessRunner(), root
		)
		if snapshot.containers or snapshot.volumes or snapshot.networks:
			raise DeveloperBrowserSuiteError("developer browser cleanup left owned resources")
		workspace = lease.reset_workspace()
		if tuple(workspace.iterdir()):
			raise DeveloperBrowserSuiteError("developer browser cleanup left private workspace artifacts")

	return DeveloperOperations(start_stack, stop_stack, verify_empty_workspace)


#============================================
def run_supervisor(
	repository_root: pathlib.Path,
	operations: DeveloperOperations | None = None,
	acquire_browser_suite_lease: Callable[[pathlib.Path], local_stack_control.browser_suite_lease.BrowserSuiteLease] = local_stack_control.browser_suite_lease.BrowserSuiteLease.acquire,
	install_signal_handlers: bool = True,
	inherited_descriptors: tuple[int, int, int] | None = None,
) -> None:
	"""Hold the actual lease until an authenticated stop or termination cleans the fixed stack."""
	active_operations = default_operations() if operations is None else operations
	lease = (
		acquire_browser_suite_lease(repository_root)
		if inherited_descriptors is None
		else local_stack_control.browser_suite_lease.BrowserSuiteLease.adopt(
			repository_root, *inherited_descriptors
		)
	)
	running: RunningDeveloperStack | None = None
	server: socket.socket | None = None
	root_descriptor = -1
	control: DeveloperControlReceipt | None = None
	stop_requested = False
	failures: list[BaseException] = []

	def request_supervisor_termination(_signal_number: int, _frame: object) -> None:
		"""Request supervisor cleanup after receiving a termination signal."""
		nonlocal stop_requested
		stop_requested = True

	previous_int: object | None = None
	previous_term: object | None = None
	if install_signal_handlers:
		previous_int = signal.signal(signal.SIGINT, request_supervisor_termination)
		previous_term = signal.signal(signal.SIGTERM, request_supervisor_termination)
	try:
		local_stack_control.browser_suite_reset.reset_live_demo_browser(
			lease, local_stack_control.process.SubprocessRunner(), repository_root
		)
		workspace = lease.reset_workspace()
		running = active_operations.start(lease, repository_root, workspace)
		launch_id = _read_launch_id(repository_root)
		control = DeveloperControlReceipt(
			os.getpid(), secrets.token_hex(32), secrets.token_hex(32), launch_id, running.origin,
			local_stack_control.models.LIVE_DEMO_BROWSER_PROJECT,
		)
		root_descriptor = _checked_root_descriptor(repository_root)
		_remove_private_entry(root_descriptor, RESULT_NAME)
		_remove_socket_path(repository_root)
		server = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
		server.bind(str(_socket_path(repository_root)))
		os.chmod(_socket_path(repository_root), 0o600)
		server.listen(1)
		server.settimeout(0.2)
		_write_private_file(root_descriptor, CONTROL_NAME, _control_value(control))
		_remove_private_entry(root_descriptor, LAUNCH_NAME)
		while not stop_requested:
			try:
				connection, _address = server.accept()
			except TimeoutError:
				continue
			with connection:
				content = connection.recv(MAXIMUM_CONTROL_BYTES + 1)
				if _validate_stop_request(content, control):
					connection.sendall(b'{"accepted":true}')
					stop_requested = True
				else:
					connection.sendall(b'{"accepted":false}')
	except BaseException as error:
		failures.append(error)
	finally:
		if server is not None:
			server.close()
		if root_descriptor >= 0:
			try:
				_remove_socket_path(repository_root)
			except BaseException as error:
				failures.append(error)
		try:
			if running is not None:
				active_operations.stop(running, repository_root)
		except BaseException as error:
			failures.append(error)
		try:
			active_operations.verify_empty(lease, repository_root)
		except BaseException as error:
			failures.append(error)
		if root_descriptor >= 0 and control is not None:
			try:
				_remove_private_entry(root_descriptor, CONTROL_NAME)
				completion = DeveloperCompletionReceipt(
					control.supervisor_id, control.capability, not failures
				)
				_write_private_file(root_descriptor, RESULT_NAME, _completion_value(completion))
			except BaseException as error:
				failures.append(error)
			os.close(root_descriptor)
		lease.release()
		if install_signal_handlers:
			if previous_int is None or previous_term is None:
				raise DeveloperBrowserSuiteError("developer browser signal state is unavailable")
			signal.signal(signal.SIGINT, previous_int)
			signal.signal(signal.SIGTERM, previous_term)
	if len(failures) == 1:
		raise failures[0]
	if failures:
		raise BaseExceptionGroup("developer browser supervisor failures", failures)


#============================================
def clear_stale_control_state(repository_root: pathlib.Path) -> None:
	"""Clear an old receipt only after acquiring the real lease that proves no owner is live."""
	lease = local_stack_control.browser_suite_lease.BrowserSuiteLease.acquire(repository_root)
	root_descriptor = -1
	try:
		root_descriptor = _checked_root_descriptor(repository_root)
		_remove_private_entry(root_descriptor, CONTROL_NAME)
		_remove_private_entry(root_descriptor, LAUNCH_NAME)
		_remove_private_entry(root_descriptor, RESULT_NAME)
		_remove_socket_path(repository_root)
	finally:
		if root_descriptor >= 0:
			os.close(root_descriptor)
		root_descriptor = -1
		lease.release()


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
def purge_orphaned_session(
	repository_root: pathlib.Path,
	runner: local_stack_control.process.CommandRunner,
) -> str:
	"""Reacquire the fixed lease and purge one interrupted browser owner.

	The caller reaches this path only after its authenticated supervisor protocol
	is unavailable.  The lease proves no live owner remains; the reset then uses
	the closed project/label registry rather than a caller-selected target.
	"""
	lease = local_stack_control.browser_suite_lease.BrowserSuiteLease.acquire(repository_root)
	try:
		# ASVS 15.4.3: the lease stays held while engine validation and exact
		# project reconciliation run, so another owner cannot enter between them.
		local_stack_control.process.require_rootless_local_engine(runner, repository_root)
		snapshot = local_stack_control.browser_suite_reset.reset_live_demo_browser(
			lease, runner, repository_root
		)
		if snapshot.containers or snapshot.volumes or snapshot.networks:
			raise DeveloperBrowserSuiteError("developer browser purge left owned resources")
		workspace = lease.reset_workspace()
		if tuple(workspace.iterdir()):
			raise DeveloperBrowserSuiteError("developer browser purge left workspace artifacts")
		root_descriptor = _checked_root_descriptor(repository_root)
		try:
			_remove_private_entry(root_descriptor, CONTROL_NAME)
			_remove_private_entry(root_descriptor, LAUNCH_NAME)
			_remove_private_entry(root_descriptor, RESULT_NAME)
		finally:
			os.close(root_descriptor)
		_remove_socket_path(repository_root)
	finally:
		lease.release()
	return local_stack_control.models.LIVE_DEMO_BROWSER_PROJECT


#============================================
def reconcile_developer_session(
	repository_root: pathlib.Path,
	runner: local_stack_control.process.CommandRunner,
) -> str:
	"""Stop an active owner or purge one lease-proven orphan before replacement.

	ASVS 2.3.1 and 15.4.3: the fixed owner completes cleanup before the next
	owner may acquire the same lease. ASVS 8.2.2: orphan recovery remains bound
	to the immutable live-demo project and its verified resource labels.
	"""
	try:
		result = request_stop(repository_root)
		project = result.project
	except DeveloperBrowserSuiteError:
		# Any unavailable or incomplete control protocol can fall back only after
		# reacquiring the same exclusive lease. A live owner keeps the lease and
		# this path therefore fails before engine or Podman work.
		project = purge_orphaned_session(repository_root, runner)
	return project


#============================================
def _recover_failed_start(repository_root: pathlib.Path) -> None:
	"""Reacquire the browser-suite lease and prove the fixed live-demo project is empty."""
	purge_orphaned_session(
		repository_root,
		local_stack_control.process.SubprocessRunner(),
	)


#============================================
def _child_exited_before_ready(child: object) -> bool:
	"""Return whether a process-like supervisor exited before its ready receipt."""
	poll = getattr(child, "poll", None)
	return callable(poll) and poll() is not None


#============================================
def start_developer_session(
	repository_root: pathlib.Path,
	timeout_seconds: float = DEVELOPER_START_WAIT_SECONDS,
	spawn: Callable[[pathlib.Path, local_stack_control.browser_suite_lease.BrowserSuiteLease], object] | None = None,
	child_terminator: Callable[[object, float], None] = _terminate_child,
) -> DeveloperStartReceipt:
	"""Launch the background lease owner and return only its fixed HTTPS origin."""
	if timeout_seconds <= 0:
		raise DeveloperBrowserSuiteError("developer browser start timeout is invalid")
	# The probe shares the browser-suite lease. It makes stale receipts powerless
	# before any child can publish readiness (ASVS 15.4.2 and 15.4.3).
	lease = local_stack_control.browser_suite_lease.BrowserSuiteLease.acquire(repository_root)
	root_descriptor = -1
	launch_id = secrets.token_hex(32)
	try:
		root_descriptor = _checked_root_descriptor(repository_root)
		_remove_private_entry(root_descriptor, CONTROL_NAME)
		_remove_private_entry(root_descriptor, LAUNCH_NAME)
		_remove_private_entry(root_descriptor, RESULT_NAME)
		_remove_socket_path(repository_root)
		_write_private_file(root_descriptor, LAUNCH_NAME, _launch_value(launch_id))
	except BaseException:
		lease.release()
		raise
	finally:
		if root_descriptor >= 0:
			os.close(root_descriptor)
	def default_spawn(
		root: pathlib.Path,
		held_lease: local_stack_control.browser_suite_lease.BrowserSuiteLease,
	) -> object:
		descriptors = held_lease.inherited_descriptors()
		return subprocess.Popen(
			[
				sys.executable,
				"-m",
				"local_stack_control.browser_suite_developer",
				"supervisor",
				str(descriptors[0]),
				str(descriptors[1]),
				str(descriptors[2]),
			],
			cwd=root,
			stdin=subprocess.DEVNULL,
			stdout=subprocess.DEVNULL,
			stderr=subprocess.DEVNULL,
			close_fds=True,
			pass_fds=descriptors,
			start_new_session=True,
		)
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
	failure: BaseException | None = None
	result: DeveloperStartReceipt | None = None
	deadline = time.monotonic() + timeout_seconds
	while time.monotonic() < deadline:
		try:
			receipt = read_control_receipt(repository_root)
			if not secrets.compare_digest(receipt.launch_id, launch_id):
				raise DeveloperBrowserSuiteError("developer browser supervisor published another launch")
			result = DeveloperStartReceipt(receipt.origin, receipt.project)
			break
		except DeveloperBrowserSuiteError as error:
			failure = error
			if _child_exited_before_ready(child):
				break
			time.sleep(0.05)
	if result is not None:
		return result
	if child is None:
		raise DeveloperBrowserSuiteError("developer browser supervisor did not start")
	cleanup_failures: list[BaseException] = []
	try:
		child_terminator(child, timeout_seconds)
	except BaseException as error:
		cleanup_failures.append(error)
	try:
		_recover_failed_start(repository_root)
	except BaseException as error:
		cleanup_failures.append(error)
	if cleanup_failures:
		raise BaseExceptionGroup("developer browser failed-start cleanup failures", cleanup_failures)
	if failure is not None:
		raise DeveloperBrowserSuiteError("developer browser supervisor did not become ready") from failure
	raise DeveloperBrowserSuiteError("developer browser supervisor did not become ready")


#============================================
def main() -> None:
	"""Run the private supervisor child selected by the future local-stack CLI."""
	arguments = sys.argv[1:]
	if len(arguments) != 4 or arguments[0] != "supervisor":
		raise DeveloperBrowserSuiteError("developer browser supervisor has an invalid invocation")
	try:
		descriptors = int(arguments[1]), int(arguments[2]), int(arguments[3])
	except ValueError as error:
		raise DeveloperBrowserSuiteError("developer browser supervisor has an invalid invocation") from error
	run_supervisor(pathlib.Path.cwd(), inherited_descriptors=descriptors)


if __name__ == "__main__":
	main()

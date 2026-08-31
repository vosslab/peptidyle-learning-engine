#!/usr/bin/env python3
"""Prove the private fault channel reaches a real Playwright worker subprocess."""

import os
import pathlib
import signal
import socket
import subprocess
import sys

SCRIPT_ROOT = pathlib.Path(__file__).resolve().parents[2]
SCRIPT_E2E_DIRECTORY = SCRIPT_ROOT / "tests" / "e2e"
sys.path.insert(0, str(SCRIPT_ROOT))
sys.path.insert(0, str(SCRIPT_E2E_DIRECTORY))

import e2e_browser_fault_orchestrator

PROCESS_TIMEOUT_SECONDS = 30.0


def request() -> e2e_browser_fault_orchestrator.FaultScenarioRequest:
	"""Return the one closed identity expected by the worker-only spec."""
	return e2e_browser_fault_orchestrator.FaultScenarioRequest(
		SCRIPT_ROOT,
		SCRIPT_ROOT / "target",
		SCRIPT_ROOT / "target" / "gateway-recovery.manifest",
		"learner_gateway_recovery",
		"bs1-0123456789ab-learner_gateway_recovery",
		[],
		{},
	)


def terminate_group(process: subprocess.Popen[str]) -> tuple[str, str]:
	"""Stop and reap the complete test process group after a bounded timeout."""
	try:
		os.killpg(process.pid, signal.SIGTERM)
	except ProcessLookupError:
		pass
	try:
		return process.communicate(timeout=5.0)
	except subprocess.TimeoutExpired:
		os.killpg(process.pid, signal.SIGKILL)
		return process.communicate(timeout=5.0)


def cleanup(
	protocol: e2e_browser_fault_orchestrator.ProtocolDirectory,
	socket_path: pathlib.Path,
	socket_identity: e2e_browser_fault_orchestrator.FileIdentity | None,
	listener: socket.socket | None,
	channel: socket.socket | None,
	process: subprocess.Popen[str] | None,
) -> list[BaseException]:
	"""Release all proof resources while preserving a rejected endpoint replacement."""
	failures: list[BaseException] = []
	if process is not None and process.poll() is None:
		try:
			terminate_group(process)
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
	endpoint_quarantined = False
	if socket_identity is None:
		endpoint_quarantined = socket_path.exists()
	else:
		try:
			e2e_browser_fault_orchestrator._unlink_socket(socket_path, socket_identity)
		except BaseException as error:
			endpoint_quarantined = True
			failures.append(error)
	try:
		e2e_browser_fault_orchestrator._remove_protocol_directory(
			protocol,
			socket_path,
			socket_identity,
			endpoint_quarantined,
		)
	except BaseException as error:
		failures.append(error)
	return failures


def raise_failures(failures: list[BaseException]) -> None:
	"""Preserve the protocol failure alongside every independent cleanup failure."""
	if len(failures) == 1:
		raise failures[0]
	if failures:
		raise BaseExceptionGroup("Playwright worker protocol proof failures", failures)


def run() -> None:
	"""Run the no-browser Playwright worker through the exact private socket protocol."""
	protocol = e2e_browser_fault_orchestrator._create_protocol_directory()
	socket_path = e2e_browser_fault_orchestrator._socket_path(protocol.path)
	socket_identity: e2e_browser_fault_orchestrator.FileIdentity | None = None
	listener: socket.socket | None = None
	channel: socket.socket | None = None
	process: subprocess.Popen[str] | None = None
	failures: list[BaseException] = []
	try:
		listener = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
		listener.settimeout(PROCESS_TIMEOUT_SECONDS)
		listener.bind(str(socket_path))
		os.chmod(socket_path, 0o600)
		socket_identity = e2e_browser_fault_orchestrator._socket_identity(socket_path)
		listener.listen(1)
		environment = dict(os.environ)
		token = e2e_browser_fault_orchestrator._token()
		environment["PLE_BROWSER_SUITE_FAULT_SOCKET_PATH"] = str(socket_path)
		environment["PLE_BROWSER_SUITE_FAULT_TOKEN"] = token
		process = subprocess.Popen(
			[
				"npx",
				"playwright",
				"test",
				"--config",
				"tests/playwright/e2e/fault_handshake_worker.config.ts",
				"--workers=1",
			],
			cwd=SCRIPT_ROOT,
			env=environment,
			stdout=subprocess.PIPE,
			stderr=subprocess.PIPE,
			text=True,
			start_new_session=True,
		)
		channel, _address = listener.accept()
		channel.settimeout(PROCESS_TIMEOUT_SECONDS)
		selected = request()
		e2e_browser_fault_orchestrator._authenticate(channel, selected, token)
		e2e_browser_fault_orchestrator._receive(channel, "response_selected", token)
		e2e_browser_fault_orchestrator._require_marker(
			protocol.path, selected, "response_selected", token
		)
		e2e_browser_fault_orchestrator._send(channel, "gateway_stopped", token)
		e2e_browser_fault_orchestrator._receive(channel, "network_recovery_visible", token)
		e2e_browser_fault_orchestrator._require_marker(
			protocol.path, selected, "network_recovery_visible", token
		)
		e2e_browser_fault_orchestrator._send(channel, "gateway_recovered", token)
		e2e_browser_fault_orchestrator._receive(channel, "completed", token)
		e2e_browser_fault_orchestrator._require_marker(protocol.path, selected, "completed", token)
		try:
			stdout, stderr = process.communicate(timeout=PROCESS_TIMEOUT_SECONDS)
		except subprocess.TimeoutExpired:
			stdout, stderr = terminate_group(process)
			raise RuntimeError("Playwright worker protocol proof timed out\n" + stdout + stderr)
		if process.returncode != 0:
			raise RuntimeError("Playwright worker protocol proof failed\n" + stdout + stderr)
	except BaseException as error:
		failures.append(error)
	finally:
		failures.extend(cleanup(protocol, socket_path, socket_identity, listener, channel, process))
	raise_failures(failures)


def main() -> None:
	"""Run the bounded worker-boundary proof as an explicit E2E command."""
	run()


if __name__ == "__main__":
	main()

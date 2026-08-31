"""Offline closed-protocol coverage for the student gateway recovery owner."""

import dataclasses
import os
import pathlib
import signal
import socket
import subprocess
import sys

import pytest

import file_utils
import local_stack_control.process

E2E_DIRECTORY = pathlib.Path(file_utils.get_repo_root()) / "tests" / "e2e"
sys.path.insert(0, str(E2E_DIRECTORY))

import e2e_browser_fault_orchestrator as fault_orchestrator
import e2e_browser_suite_children


def request(tmp_path: pathlib.Path) -> fault_orchestrator.FaultScenarioRequest:
	"""Build one private scenario request without any lifecycle or browser process."""
	return fault_orchestrator.FaultScenarioRequest(
		tmp_path,
		tmp_path,
		tmp_path / "disposable.manifest",
		"learner_gateway_recovery",
		"bs1-0123456789ab-learner_gateway_recovery",
		["npx", "playwright", "test"],
		{},
	)


def private_marker(
	directory: pathlib.Path,
	selected: fault_orchestrator.FaultScenarioRequest,
	phase: str,
	token: str,
) -> pathlib.Path:
	"""Write one correct marker, then return its fixed filename."""
	fault_orchestrator._write_marker(directory, selected, phase, token)
	return directory / f"fault-{phase}.json"


@pytest.mark.parametrize(
	"kind",
	("missing", "symlink", "directory", "oversize", "wrong_mode", "wrong_owner"),
)
def test_marker_rejects_unsafe_files(
	tmp_path: pathlib.Path, monkeypatch: pytest.MonkeyPatch, kind: str
) -> None:
	"""The descriptor-relative marker reader accepts only private canonical regular files."""
	selected = request(tmp_path)
	directory = tmp_path / "handshake"
	directory.mkdir(mode=0o700)
	token = "a" * 43
	path = directory / "fault-response_selected.json"
	if kind == "missing":
		pass
	elif kind == "symlink":
		target = tmp_path / "target.json"
		target.write_text("{}", encoding="ascii")
		path.symlink_to(target)
	elif kind == "directory":
		path.mkdir()
	elif kind == "oversize":
		path.write_text("x" * 1_025, encoding="ascii")
		path.chmod(0o600)
	else:
		private_marker(directory, selected, "response_selected", token)
		if kind == "wrong_mode":
			path.chmod(0o644)
		if kind == "wrong_owner":
			monkeypatch.setattr(os, "getuid", lambda: path.stat().st_uid + 1)
	with pytest.raises(fault_orchestrator.FaultProtocolError):
		fault_orchestrator._require_marker(directory, selected, "response_selected", token)


def test_marker_rejects_identity_canonical_and_order_violations(tmp_path: pathlib.Path) -> None:
	"""Stale, altered, and unexpected marker data cannot advance the lifecycle."""
	selected = request(tmp_path)
	directory = tmp_path / "handshake"
	directory.mkdir(mode=0o700)
	token = "b" * 43
	path = private_marker(directory, selected, "response_selected", token)
	other = dataclasses.replace(selected, namespace="bs1-abcdefabcdef-learner_gateway_recovery")
	with pytest.raises(fault_orchestrator.FaultProtocolError, match="identity"):
		fault_orchestrator._require_marker(directory, other, "response_selected", token)
	path.write_text('{"namespace":"bs1-0123456789ab-learner_gateway_recovery"}', encoding="ascii")
	path.chmod(0o600)
	with pytest.raises(fault_orchestrator.FaultProtocolError, match="identity"):
		fault_orchestrator._require_marker(directory, selected, "response_selected", token)
	path.unlink()
	private_marker(directory, selected, "response_selected", token)
	(path.parent / "fault-completed.json").write_text("{}", encoding="ascii")
	with pytest.raises(fault_orchestrator.FaultProtocolError, match="order"):
		fault_orchestrator._require_marker_order(directory, ("response_selected",))


@pytest.mark.parametrize(
	"message, expected",
	(
		(b"not-json\n", "malformed"),
		(b'{"phase":"response_selected","token":"wrong"}\n', "identity"),
		(b'{"phase":"response_selected","token":"c"}\nextra', "trailing"),
		(b"x" * 257, "too large"),
	),
)
def test_socket_rejects_noncanonical_notifications(message: bytes, expected: str) -> None:
	"""One bounded canonical line is required for every socket notification."""
	owner, child = socket.socketpair()
	try:
		child.sendall(message)
		with pytest.raises(fault_orchestrator.FaultProtocolError, match=expected):
			fault_orchestrator._receive(owner, "response_selected", "c")
	finally:
		owner.close()
		child.close()


def test_socket_accepts_one_authenticated_canonical_notification() -> None:
	"""The private socket accepts its exact closed notification shape."""
	owner, child = socket.socketpair()
	try:
		fault_orchestrator._send(child, "response_selected", "d")
		fault_orchestrator._receive(owner, "response_selected", "d")
	finally:
		owner.close()
		child.close()


@pytest.mark.parametrize("field", ("namespace", "scenarioId", "token", "version", "noncanonical"))
def test_authentication_rejects_stale_or_noncanonical_identity(
	tmp_path: pathlib.Path, field: str
) -> None:
	"""The owner admits only this scenario's canonical worker authentication."""
	selected = request(tmp_path)
	owner, child = socket.socketpair()
	try:
		value = {
			"kind": "hello",
			"namespace": selected.namespace,
			"scenarioId": selected.scenario_id,
			"token": "e" * 43,
			"version": 1,
		}
		if field == "namespace":
			value["namespace"] = "bs1-abcdefabcdef-learner_gateway_recovery"
		elif field == "scenarioId":
			value["scenarioId"] = "other_scenario"
		elif field == "token":
			value["token"] = "f" * 43
		elif field == "version":
			value["version"] = 2
		if field == "noncanonical":
			child.sendall(b'{"version":1,"token":"eeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee","scenarioId":"learner_gateway_recovery","namespace":"bs1-0123456789ab-learner_gateway_recovery","kind":"hello"}\n')
		else:
			child.sendall(fault_orchestrator._canonical(value).encode("ascii") + b"\n")
		with pytest.raises(fault_orchestrator.FaultProtocolError, match="identity"):
			fault_orchestrator._authenticate(owner, selected, "e" * 43)
	finally:
		owner.close()
		child.close()


class FakeProcess:
	"""A deterministic reaped child process for protocol-only owner tests."""

	def wait(self, timeout: float) -> int:
		return 0

	def terminate(self) -> None:
		return None

	def kill(self) -> None:
		return None


class TimeoutProcess:
	"""A child leader which needs both group signals before it can be reaped."""

	def __init__(self) -> None:
		self.waits = 0

	def wait(self, timeout: float) -> int:
		self.waits += 1
		if self.waits < 3:
			raise subprocess.TimeoutExpired("browser", timeout)
		return 7

	def terminate(self) -> None:
		raise AssertionError("group termination owns child shutdown")

	def kill(self) -> None:
		raise AssertionError("group termination owns child shutdown")


def test_reap_terminates_the_complete_start_new_session_group(
	monkeypatch: pytest.MonkeyPatch,
) -> None:
	"""Timeout recovery sends TERM then KILL to the owner-created process group."""
	signals: list[signal.Signals] = []
	monkeypatch.setattr(
		e2e_browser_suite_children,
		"_terminate_group",
		lambda _group, signal_value: signals.append(signal_value),
	)
	monkeypatch.setattr(e2e_browser_suite_children, "_group_is_live", lambda _group: False)
	monkeypatch.setattr(e2e_browser_suite_children, "_marker_descendant_is_live", lambda _marker: False)
	child = e2e_browser_suite_children.BrowserChild(
		TimeoutProcess(), local_stack_control.process.ProcessSession(777, 1, "injected", "marker")
	)
	assert e2e_browser_suite_children.reap(child, 0.01) == 7
	assert signals == [signal.SIGTERM, signal.SIGKILL]


def test_reap_terminates_descendants_when_their_leader_has_already_exited(
	monkeypatch: pytest.MonkeyPatch,
) -> None:
	"""A zero-status leader cannot hide a surviving same-group browser descendant."""
	signals: list[signal.Signals] = []
	group_states = iter((True, False))
	monkeypatch.setattr(
		e2e_browser_suite_children,
		"_terminate_group",
		lambda _group, signal_value: signals.append(signal_value),
	)
	monkeypatch.setattr(
		e2e_browser_suite_children,
		"_group_is_live",
		lambda _group: next(group_states),
	)
	monkeypatch.setattr(e2e_browser_suite_children, "_marker_descendant_is_live", lambda _marker: False)
	child = e2e_browser_suite_children.BrowserChild(
		FakeProcess(), local_stack_control.process.ProcessSession(778, 1, "injected", "marker")
	)
	assert e2e_browser_suite_children.reap(child, 0.01) == 0
	assert signals == [signal.SIGTERM]


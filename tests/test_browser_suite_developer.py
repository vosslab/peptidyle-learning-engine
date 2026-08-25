"""Deterministic contracts for the fixed live-demo developer supervisor."""

from __future__ import annotations

import os
import pathlib

import pytest

import local_stack_control.browser_suite_developer
import local_stack_control.browser_suite_lease
import local_stack_control.browser_suite_reset
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
	path = tmp_path / "target" / "live-demo-browser" / "developer-control.json"
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

	result = local_stack_control.browser_suite_developer.start_developer_session(
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
		assert lease.workspace == tmp_path / "target" / "live-demo-browser" / "workspace"


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
	project = local_stack_control.browser_suite_developer.purge_orphaned_session(
		tmp_path,
		local_stack_control.process.SubprocessRunner(),
	)
	assert project == "ple-live-demo-browser"
	assert events == ["reset"]
	assert tuple((tmp_path / "target" / "live-demo-browser" / "workspace").iterdir()) == ()
	with pytest.raises(
		local_stack_control.browser_suite_developer.DeveloperBrowserSuiteError,
		match="session is not running",
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
		local_stack_control.browser_suite_developer.start_developer_session(
			tmp_path,
			0.5,
			lambda _root, _lease: child,
			lambda observed, _timeout: events.append("terminated") if observed is child else None,
		)
	assert events == ["poll", "terminated", "reset"]
	with local_stack_control.browser_suite_lease.BrowserSuiteLease.acquire(tmp_path) as lease:
		assert tuple(lease.reset_workspace().iterdir()) == ()

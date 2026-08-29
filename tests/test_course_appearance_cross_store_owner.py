"""Offline lifecycle contracts for the leased course-appearance cross-store owner."""

from __future__ import annotations

import pathlib

import pytest

import local_stack_control.browser_suite_reset
import local_stack_control.course_appearance_cross_store_owner
import local_stack_control.models


class FakeLease:
	"""Record one fixed workspace lifecycle without acquiring host locks."""

	def __init__(self, root: pathlib.Path) -> None:
		self.repository_root = root
		self.workspace = root / "workspace"
		self.workspace.mkdir()
		self.released = False

	def reset_workspace(self) -> pathlib.Path:
		for child in self.workspace.iterdir():
			child.unlink()
		return self.workspace

	def release(self) -> None:
		self.released = True


#============================================
def empty_snapshot() -> local_stack_control.models.ProjectSnapshot:
	"""Build the exact fixed owner inventory used by the profile lifecycle."""
	return local_stack_control.models.ProjectSnapshot(
		project=local_stack_control.models.LIVE_DEMO_BROWSER_PROJECT,
		containers=(),
		volumes=(),
		networks=(),
	)


#============================================
def test_cross_store_owner_resets_then_checks_two_ports_then_runs_child(
	tmp_path: pathlib.Path,
) -> None:
	"""The shared lease owner keeps both stores in one sequential cleanup boundary."""
	lease = FakeLease(tmp_path)
	events: list[str] = []

	def reset(
		selected_lease: FakeLease, _runner: object, root: pathlib.Path
	) -> local_stack_control.models.ProjectSnapshot:
		assert selected_lease is lease
		assert root == tmp_path
		events.append("reset")
		return empty_snapshot()

	def oracle(root: pathlib.Path, workspace: pathlib.Path, ports: tuple[int, ...]) -> None:
		assert root == tmp_path
		assert workspace == lease.workspace
		assert ports == (48500, 49500)
		events.append("oracle")

	def check_ports(ports: tuple[int, ...], _runner: object, root: pathlib.Path) -> None:
		assert ports == (48500, 49500)
		assert root == tmp_path
		events.append("ports")

	monkeypatch = pytest.MonkeyPatch()
	monkeypatch.setattr(local_stack_control.browser_suite_reset, "reset_live_demo_browser", reset)
	try:
		local_stack_control.course_appearance_cross_store_owner.run_owned_course_appearance_cross_store(
			tmp_path,
			oracle_runner=oracle,
			lease_factory=lambda _root: lease,
			reset_runner_factory=object,
			ports_selector=lambda: (48500, 49500),
			port_checker=check_ports,
		)
	finally:
		monkeypatch.undo()
	assert events == ["reset", "ports", "oracle", "reset"]
	assert lease.released


#============================================
def test_cross_store_owner_refuses_duplicated_ports_before_child_execution(
	tmp_path: pathlib.Path,
) -> None:
	"""Distinct fixed loopback bindings are required before any child can start."""
	lease = FakeLease(tmp_path)

	def reset(
		_selected_lease: FakeLease, _runner: object, _root: pathlib.Path
	) -> local_stack_control.models.ProjectSnapshot:
		return empty_snapshot()

	monkeypatch = pytest.MonkeyPatch()
	monkeypatch.setattr(local_stack_control.browser_suite_reset, "reset_live_demo_browser", reset)
	try:
		with pytest.raises(local_stack_control.models.ControllerError, match="ports are invalid"):
			local_stack_control.course_appearance_cross_store_owner.run_owned_course_appearance_cross_store(
				tmp_path,
				oracle_runner=lambda _root, _workspace, _ports: pytest.fail("child ran"),
				lease_factory=lambda _root: lease,
				reset_runner_factory=object,
				ports_selector=lambda: (48500, 48500),
				port_checker=lambda _ports, _runner, _root: pytest.fail("ports checked"),
			)
	finally:
		monkeypatch.undo()
	assert lease.released

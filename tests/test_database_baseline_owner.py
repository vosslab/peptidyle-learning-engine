"""Offline lifecycle contracts for the serial database baseline owner."""

from __future__ import annotations

import pathlib
import stat

import pytest

import local_stack_control.database_baseline_owner
import local_stack_control.models


class FakeLease:
	"""Record the fixed workspace lifecycle without touching host locks."""

	def __init__(self, root: pathlib.Path) -> None:
		self.repository_root = root
		self.workspace = root / "workspace"
		self.workspace.mkdir()
		self.released = False
		self.workspace_resets = 0

	def reset_workspace(self) -> pathlib.Path:
		self.workspace_resets += 1
		for child in self.workspace.iterdir():
			child.unlink()
		return self.workspace

	def release(self) -> None:
		self.released = True


#============================================
def empty_snapshot() -> local_stack_control.models.ProjectSnapshot:
	"""Build the sole fixed owner inventory used by the lifecycle tests."""
	return local_stack_control.models.ProjectSnapshot(
		project=local_stack_control.models.LIVE_DEMO_BROWSER_PROJECT,
		containers=(),
		volumes=(),
		networks=(),
	)


#============================================
def test_private_oracle_child_receives_only_owner_created_runtime_input(
	tmp_path: pathlib.Path,
	monkeypatch: pytest.MonkeyPatch,
) -> None:
	"""The child launch receives the fixed command and its private handoff."""
	calls: list[tuple[list[str], pathlib.Path, dict[str, str], bool]] = []

	class Completed:
		"""Return a successful child receipt without executing a shell process."""

		returncode = 0

	def run(
		argv: list[str],
		cwd: pathlib.Path,
		env: dict[str, str],
		check: bool,
	) -> Completed:
		calls.append((argv, cwd, env, check))
		return Completed()

	monkeypatch.setattr(local_stack_control.database_baseline_owner.subprocess, "run", run)
	local_stack_control.database_baseline_owner._run_oracle(tmp_path, tmp_path, 48500)
	assert len(calls) == 1
	argv, cwd, environment, check = calls[0]
	assert argv == ["bash", "tests/e2e/e2e_database_baseline.sh", "--owned-child"]
	assert cwd == tmp_path
	assert check is False
	assert environment["PLE_DATABASE_BASELINE_WORKSPACE"] == str(tmp_path)
	assert environment["PLE_DATABASE_BASELINE_PORT"] == "48500"
	owner_input = pathlib.Path(environment["PLE_DATABASE_BASELINE_OWNER_INPUT"])
	assert owner_input.parent == tmp_path
	assert owner_input.read_bytes() == b"lease-held\n"
	assert stat.S_IMODE(owner_input.stat().st_mode) == 0o600


#============================================
def test_database_baseline_owner_resets_before_and_after_the_private_oracle(
	tmp_path: pathlib.Path,
) -> None:
	"""The public database gate shares the browser lease and leaves it empty."""
	lease = FakeLease(tmp_path)
	events: list[str] = []

	def reset(
		selected_lease: FakeLease,
		_runner: object,
		root: pathlib.Path,
	) -> local_stack_control.models.ProjectSnapshot:
		assert selected_lease is lease
		assert root == tmp_path
		events.append("reset")
		return empty_snapshot()

	def oracle(root: pathlib.Path, workspace: pathlib.Path, port: int) -> None:
		assert root == tmp_path
		assert workspace == lease.workspace
		assert port == 48500
		events.append("oracle")

	def check_port(
		ports: tuple[int, ...],
		_runner: object,
		root: pathlib.Path,
	) -> None:
		assert ports == (48500,)
		assert root == tmp_path
		events.append("port")

	monkeypatch = pytest.MonkeyPatch()
	monkeypatch.setattr(
		local_stack_control.browser_suite_reset,
		"reset_live_demo_browser",
		reset,
	)
	try:
		local_stack_control.database_baseline_owner.run_owned_database_baseline(
			tmp_path,
			oracle_runner=oracle,
			lease_factory=lambda _root: lease,
			reset_runner_factory=object,
			port_selector=lambda: 48500,
			port_checker=check_port,
		)
	finally:
		monkeypatch.undo()
	assert events == ["reset", "port", "oracle", "reset"]
	assert lease.workspace_resets == 2
	assert lease.released


#============================================
def test_database_baseline_owner_still_finally_resets_after_oracle_failure(
	tmp_path: pathlib.Path,
) -> None:
	"""An oracle error cannot retain the shared stack for a later browser run."""
	lease = FakeLease(tmp_path)
	resets = 0

	def reset(
		_selected_lease: FakeLease,
		_runner: object,
		_root: pathlib.Path,
	) -> local_stack_control.models.ProjectSnapshot:
		nonlocal resets
		resets += 1
		return empty_snapshot()

	def failing_oracle(_root: pathlib.Path, _workspace: pathlib.Path, _port: int) -> None:
		raise local_stack_control.models.ControllerError("oracle proof failed")

	monkeypatch = pytest.MonkeyPatch()
	monkeypatch.setattr(
		local_stack_control.browser_suite_reset,
		"reset_live_demo_browser",
		reset,
	)
	try:
		with pytest.raises(local_stack_control.models.ControllerError, match="oracle proof failed"):
			local_stack_control.database_baseline_owner.run_owned_database_baseline(
				tmp_path,
				oracle_runner=failing_oracle,
				lease_factory=lambda _root: lease,
				reset_runner_factory=object,
				port_selector=lambda: 48500,
				port_checker=lambda _ports, _runner, _root: None,
			)
	finally:
		monkeypatch.undo()
	assert resets == 2
	assert lease.released

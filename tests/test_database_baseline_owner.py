"""Offline lifecycle contracts for the serial database baseline owner."""

from __future__ import annotations

import pathlib
import stat

import pytest

import file_utils
import local_stack_control.database_baseline_owner
import local_stack_control.models
import local_stack_control.runtime_manifest


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
def test_private_oracle_child_receives_only_owner_created_runtime_manifest(
	tmp_path: pathlib.Path,
	monkeypatch: pytest.MonkeyPatch,
) -> None:
	"""The child launch receives only a non-secret runtime-manifest locator."""
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
	assert argv == [
		"bash",
		str(tmp_path / "tests/e2e/e2e_database_baseline.sh"),
		"--owned-child",
		"--runtime-manifest",
		"runtime.yaml",
	]
	assert cwd == tmp_path
	assert check is False
	assert not any(name.startswith("PLE_") or name.startswith("COMPOSE_") for name in environment)
	runtime = local_stack_control.runtime_manifest.load_database_baseline_runtime(tmp_path)
	assert runtime.manifest_path == tmp_path / "runtime.yaml"
	assert stat.S_IMODE(runtime.manifest_path.stat().st_mode) == 0o600
	assert stat.S_IMODE((tmp_path / "secrets").stat().st_mode) == 0o700
	assert all(
		stat.S_IMODE(path.stat().st_mode) == 0o600
		for path in (tmp_path / "secrets").iterdir()
	)


#============================================
def test_database_baseline_shell_selects_the_private_project_tools_runtime() -> None:
	"""Migration administration opts into the non-secret runtime-file boundary."""
	shell = pathlib.Path(file_utils.get_repo_root()) / "tests/e2e/e2e_database_baseline.sh"
	content = shell.read_text(encoding="utf-8")
	assert 'cargo run --manifest-path "$REPO_ROOT/Cargo.toml"' in content
	assert 'database "$@" --acceptance-runtime' in content
	assert 'cargo test --manifest-path "$REPO_ROOT/Cargo.toml"' in content
	assert 'export PLE_ACCEPTANCE_RUNTIME_MANIFEST="$RUNTIME_MANIFEST_PATH"' in content
	assert content.count("PLE_ACCEPTANCE_RUNTIME_MANIFEST") == 1
	assert "PLE_TEST_DATABASE_URL" not in content
	assert "PLE_TEST_GRADER_DATABASE_URL" not in content


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

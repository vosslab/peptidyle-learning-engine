"""Offline contracts for the fixed live-demo browser reset action."""

import pathlib

import pytest

import local_stack_control.browser_suite_reset
import local_stack_control.browser_suite_lease
import local_stack_control.models
import local_stack_control.process


class RecordingRunner(local_stack_control.process.CommandRunner):
	"""Record exact reset mutations without invoking Podman."""

	def __init__(self) -> None:
		"""Start with an empty mutation record."""
		self.commands: list[tuple[str, ...]] = []

	def run(
		self,
		argv: list[str],
		environment: dict[str, str] | None = None,
		cwd: pathlib.Path | None = None,
		stdin: str | None = None,
	) -> local_stack_control.models.CommandResult:
		"""Record one exact command as a successful offline engine result."""
		self.commands.append(tuple(argv))
		return local_stack_control.models.CommandResult(tuple(argv), 0, "", "")

	def stream(
		self,
		argv: list[str],
		environment: dict[str, str] | None = None,
		cwd: pathlib.Path | None = None,
	) -> int:
		"""Reject streaming because reset uses captured exact commands."""
		raise AssertionError("reset must not use streaming commands")


#============================================
def snapshot(
	owner: str | None = "live-demo-browser",
	service: str = "gateway",
) -> local_stack_control.models.ProjectSnapshot:
	"""Build one valid exact browser inventory with every resource class."""
	project = local_stack_control.models.LIVE_DEMO_BROWSER_PROJECT
	result = local_stack_control.models.ProjectSnapshot(
		project=project,
		containers=(
			local_stack_control.models.ContainerResource(
				id="container-one", names=("gateway",), project=project, service=service,
				state="running", running=True, exit_code=None, health="healthy", image="gateway",
				ports=(), owner=owner,
			),
		),
		volumes=(local_stack_control.models.VolumeResource(f"{project}_ple_pgdata", project, owner=owner),),
		networks=(local_stack_control.models.NetworkResource(f"{project}_default", project, owner=owner),),
	)
	return result


#============================================
def test_reset_uses_podman_dependency_order_for_exact_verified_resources(
	monkeypatch: pytest.MonkeyPatch,
	tmp_path: pathlib.Path,
) -> None:
	"""Podman resolves dependency-bearing containers before volume cleanup."""
	runner = RecordingRunner()
	initial = snapshot()
	without_containers = local_stack_control.models.ProjectSnapshot(
		initial.project, (), initial.volumes, initial.networks,
	)
	empty = local_stack_control.models.ProjectSnapshot(initial.project, (), (), ())
	values = iter((initial, without_containers, empty))
	monkeypatch.setattr(local_stack_control.browser_suite_reset, "_browser_snapshot", lambda *args: next(values))
	with local_stack_control.browser_suite_lease.BrowserSuiteLease.acquire(tmp_path) as lease:
		local_stack_control.browser_suite_reset.reset_live_demo_browser(lease, runner, tmp_path)
	assert runner.commands == [
		("podman", "rm", "-f", "--depend", "container-one"),
		("podman", "volume", "rm", "ple-live-demo-browser_ple_pgdata"),
		("podman", "network", "rm", "ple-live-demo-browser_default"),
	]


#============================================
def test_reset_reinventories_after_dependency_cleanup(
	monkeypatch: pytest.MonkeyPatch,
	tmp_path: pathlib.Path,
) -> None:
	"""A dependency removal refreshes IDs before the next exact engine operation."""
	runner = RecordingRunner()
	project = local_stack_control.models.LIVE_DEMO_BROWSER_PROJECT
	first = snapshot()
	second = local_stack_control.models.ContainerResource(
		id="container-two", names=("postgres",), project=project, service="postgres",
		state="running", running=True, exit_code=None, health="healthy", image="postgres",
		ports=(), owner="live-demo-browser",
	)
	with_containers = local_stack_control.models.ProjectSnapshot(
		project, (first.containers[0], second), first.volumes, first.networks,
	)
	without_containers = local_stack_control.models.ProjectSnapshot(
		project, (), first.volumes, first.networks,
	)
	empty = local_stack_control.models.ProjectSnapshot(project, (), (), ())
	values = iter((with_containers, without_containers, empty))
	monkeypatch.setattr(local_stack_control.browser_suite_reset, "_browser_snapshot", lambda *args: next(values))
	with local_stack_control.browser_suite_lease.BrowserSuiteLease.acquire(tmp_path) as lease:
		local_stack_control.browser_suite_reset.reset_live_demo_browser(lease, runner, tmp_path)
	assert runner.commands[0] == ("podman", "rm", "-f", "--depend", "container-one")
	assert all("container-two" not in command for command in runner.commands)


#============================================
def test_empty_reset_is_a_successful_no_op(monkeypatch: pytest.MonkeyPatch, tmp_path: pathlib.Path) -> None:
	"""No retained exact browser resource requires no engine mutation."""
	runner = RecordingRunner()
	empty = local_stack_control.models.ProjectSnapshot(
		local_stack_control.models.LIVE_DEMO_BROWSER_PROJECT, (), (), ()
	)
	monkeypatch.setattr(local_stack_control.browser_suite_reset, "_browser_snapshot", lambda *args: empty)
	with local_stack_control.browser_suite_lease.BrowserSuiteLease.acquire(tmp_path) as lease:
		assert local_stack_control.browser_suite_reset.reset_live_demo_browser(lease, runner, tmp_path) == empty
	assert runner.commands == []


#============================================
@pytest.mark.parametrize("owner", (None, "normal", "foreign"))
def test_foreign_or_missing_owner_label_stops_before_mutation(
	monkeypatch: pytest.MonkeyPatch,
	tmp_path: pathlib.Path,
	owner: str | None,
) -> None:
	"""A non-browser label is an ownership failure, never a reset candidate."""
	runner = RecordingRunner()
	monkeypatch.setattr(local_stack_control.browser_suite_reset, "_browser_snapshot", lambda *args: snapshot(owner))
	with pytest.raises(local_stack_control.models.ControllerError, match="foreign resource"):
		with local_stack_control.browser_suite_lease.BrowserSuiteLease.acquire(tmp_path) as lease:
			local_stack_control.browser_suite_reset.reset_live_demo_browser(lease, runner, tmp_path)
	assert runner.commands == []


#============================================
def test_unknown_declared_topology_stops_before_mutation(
	monkeypatch: pytest.MonkeyPatch,
	tmp_path: pathlib.Path,
) -> None:
	"""A project label alone cannot enlarge the browser reset topology."""
	runner = RecordingRunner()
	monkeypatch.setattr(
		local_stack_control.browser_suite_reset,
		"_browser_snapshot",
		lambda *args: snapshot(service="unrelated-service"),
	)
	with pytest.raises(local_stack_control.models.ControllerError, match="foreign resource"):
		with local_stack_control.browser_suite_lease.BrowserSuiteLease.acquire(tmp_path) as lease:
			local_stack_control.browser_suite_reset.reset_live_demo_browser(lease, runner, tmp_path)
	assert runner.commands == []


#============================================
def test_interrupted_reset_retries_only_remaining_valid_resources(
	monkeypatch: pytest.MonkeyPatch,
	tmp_path: pathlib.Path,
) -> None:
	"""A next holder inventories again and completes an interrupted exact reset."""
	runner = RecordingRunner()
	remaining = snapshot()
	without_containers = local_stack_control.models.ProjectSnapshot(
		remaining.project, (), remaining.volumes, remaining.networks,
	)
	empty = local_stack_control.models.ProjectSnapshot(
		local_stack_control.models.LIVE_DEMO_BROWSER_PROJECT, (), (), ()
	)
	values = iter((remaining, without_containers, empty))
	monkeypatch.setattr(local_stack_control.browser_suite_reset, "_browser_snapshot", lambda *args: next(values))
	with local_stack_control.browser_suite_lease.BrowserSuiteLease.acquire(tmp_path) as lease:
		local_stack_control.browser_suite_reset.reset_live_demo_browser(lease, runner, tmp_path)
	assert runner.commands[-2:] == [
		("podman", "volume", "rm", "ple-live-demo-browser_ple_pgdata"),
		("podman", "network", "rm", "ple-live-demo-browser_default"),
	]

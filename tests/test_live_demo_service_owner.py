"""Offline exact contracts for the shared live-demo service-oracle owner."""

# Standard Library
import dataclasses
import json
import pathlib
import shutil
import stat
import sys

# PIP3 modules
import pytest

# local repo modules
import file_utils
import local_stack_control.models
import local_stack_control.process


E2E_DIRECTORY = pathlib.Path(file_utils.get_repo_root()) / "tests" / "e2e"
sys.path.insert(0, str(E2E_DIRECTORY))

import e2e_browser_suite_oracles
import e2e_live_demo_service_input as service_input
import e2e_live_demo_service_owner as service_owner


class OfflineRunner(local_stack_control.process.CommandRunner):
	"""Reject unowned subprocess calls in dependency-injected owner tests."""

	#============================================
	def run(
		self,
		argv: list[str],
		environment: dict[str, str] | None = None,
		cwd: pathlib.Path | None = None,
		stdin: str | None = None,
	) -> local_stack_control.models.CommandResult:
		"""Reject captured subprocess execution."""
		raise AssertionError("offline service-owner test reached a subprocess")

	#============================================
	def stream(
		self,
		argv: list[str],
		environment: dict[str, str] | None = None,
		cwd: pathlib.Path | None = None,
	) -> int:
		"""Reject streamed subprocess execution."""
		raise AssertionError("offline service-owner test reached a subprocess")


class FakeLease:
	"""Record the fixed workspace lifecycle without acquiring a real checkout lock."""

	def __init__(self, root: pathlib.Path, events: list[str]) -> None:
		"""Create one private workspace path for an injected owner run."""
		self.repository_root = root
		self.workspace = root / "target" / "live-demo-browser" / "workspace"
		self.events = events
		self.released = False

	#============================================
	def reset_workspace(self) -> pathlib.Path:
		"""Clear and recreate the private workspace while recording ordering."""
		self.events.append("workspace")
		if self.workspace.exists():
			shutil.rmtree(self.workspace)
		self.workspace.mkdir(parents=True, mode=0o700)
		return self.workspace

	#============================================
	def release(self) -> None:
		"""Record release after final proof and receipt publication."""
		self.events.append("release")
		self.released = True


#============================================
def write_private(path: pathlib.Path, contents: str) -> None:
	"""Write one inline test input with the production ABI mode."""
	path.write_text(contents, encoding="ascii")
	path.chmod(0o600)


#============================================
def oracle_input(
	tmp_path: pathlib.Path,
	oracle: str = "webwork_render_rpc",
) -> service_input.LiveDemoServiceOracleInputV1:
	"""Build one valid inline V1 value rooted in a temporary workspace."""
	workspace = tmp_path / "workspace"
	result = service_input.LiveDemoServiceOracleInputV1(
		oracle,
		"https://localhost:55001/",
		workspace / "disposable.manifest",
		workspace / "service-oracle-seed-manifest.json",
		workspace,
	)
	return result


#============================================
def empty_snapshot() -> local_stack_control.models.ProjectSnapshot:
	"""Return the exact empty fixed-project inventory proof."""
	result = local_stack_control.models.ProjectSnapshot(
		local_stack_control.models.LIVE_DEMO_BROWSER_PROJECT,
		(),
		(),
		(),
	)
	return result


#============================================
def install_child(root: pathlib.Path, oracle: str) -> None:
	"""Create the exact registry artifact needed by one offline owner test."""
	child = service_owner.child_program(oracle)
	path = root / child.relative_path
	path.parent.mkdir(parents=True, exist_ok=True)
	path.write_text("offline child placeholder\n", encoding="ascii")


#============================================
def offline_dependencies(
	tmp_path: pathlib.Path,
	oracle: str,
	child_returncode: int = 0,
) -> tuple[
	service_owner.LiveDemoServiceDependencies,
	list[str],
	list[service_owner.LiveDemoServiceReceipt],
]:
	"""Build a complete offline owner whose seams record their exact order."""
	events: list[str] = []
	receipts: list[service_owner.LiveDemoServiceReceipt] = []
	install_child(tmp_path, oracle)

	def acquire(root: pathlib.Path) -> FakeLease:
		events.append("lease")
		return FakeLease(root, events)

	def runner_factory() -> local_stack_control.process.CommandRunner:
		events.append("runner")
		return OfflineRunner()

	def reset(
		lease: object,
		runner: local_stack_control.process.CommandRunner,
		root: pathlib.Path,
	) -> local_stack_control.models.ProjectSnapshot:
		events.append("reset")
		return empty_snapshot()

	def reset_ports() -> local_stack_control.live_demo_target.LiveDemoPorts:
		events.append("ports")
		return local_stack_control.live_demo_target.LiveDemoPorts(53501, 54001, 54501, 55001)

	def check_ports(
		ports: tuple[int, int, int, int],
		runner: local_stack_control.process.CommandRunner,
		root: pathlib.Path,
	) -> None:
		events.append("port-check")

	def selections(root: pathlib.Path) -> dict[str, str]:
		events.append("selections")
		return {}

	def write_target(
		workspace: pathlib.Path,
		profile: local_stack_control.models.LiveDemoProfile,
		ports: local_stack_control.live_demo_target.LiveDemoPorts,
		selected: object,
	) -> local_stack_control.live_demo_target.LiveDemoTarget:
		events.append("target")
		manifest = workspace / "disposable.manifest"
		write_private(manifest, "private manifest\n")
		result = local_stack_control.live_demo_target.LiveDemoTarget(
			profile,
			manifest,
			workspace / "env.local",
			workspace / "disposable.capability",
			"https://localhost:55001/",
			ports,
			workspace / "claim-context.json",
		)
		return result

	def validate_topology(
		runner: local_stack_control.process.CommandRunner,
		root: pathlib.Path,
		manifest: pathlib.Path,
	) -> None:
		events.append("topology")

	def launch(
		runner: local_stack_control.process.CommandRunner,
		root: pathlib.Path,
		target: local_stack_control.live_demo_target.LiveDemoTarget,
		session_recorder: service_owner.SessionRecorder,
	) -> service_owner.CompletedOwnerCommand:
		events.append("launch")
		return service_owner.CompletedOwnerCommand(0, True)

	def seed(
		selected_oracle: str,
		runner: local_stack_control.process.CommandRunner,
		root: pathlib.Path,
		target: local_stack_control.live_demo_target.LiveDemoTarget,
		workspace: pathlib.Path,
	) -> pathlib.Path:
		events.append("seed")
		path = workspace / service_owner.SEED_MANIFEST_NAME
		write_private(path, '{"assignmentId":"private"}')
		return path

	def child_runner(
		child: service_owner.ChildProgram,
		input_path: pathlib.Path,
		runner: local_stack_control.process.CommandRunner,
		root: pathlib.Path,
		session_recorder: service_owner.SessionRecorder,
	) -> service_owner.CompletedOwnerCommand:
		events.append("child")
		value = service_input.read_private_input(input_path, oracle)
		assert value.manifest_path.parent == value.workspace_path
		return service_owner.CompletedOwnerCommand(child_returncode, True)

	def cleanup(
		runner: local_stack_control.process.CommandRunner,
		root: pathlib.Path,
		target: local_stack_control.live_demo_target.LiveDemoTarget,
		session_recorder: service_owner.SessionRecorder,
	) -> service_owner.CompletedOwnerCommand:
		events.append("cleanup")
		return service_owner.CompletedOwnerCommand(0, True)

	def owner_processes(
		sessions: tuple[local_stack_control.process.ProcessSession, ...],
	) -> tuple[e2e_browser_suite_oracles.ProcessIdentity, ...]:
		events.append("processes")
		return ()

	def report(receipt: service_owner.LiveDemoServiceReceipt) -> None:
		events.append("report")
		receipts.append(receipt)

	dependencies = service_owner.LiveDemoServiceDependencies(
		root=tmp_path,
		lease_factory=acquire,
		runner_factory=runner_factory,
		selection_reader=selections,
		port_factory=reset_ports,
		port_checker=check_ports,
		target_writer=write_target,
		topology_validator=validate_topology,
		lifecycle_launcher=launch,
		seed_runner=seed,
		child_runner=child_runner,
		lifecycle_cleaner=cleanup,
		resetter=reset,
		owner_process_reader=owner_processes,
		receipt_reporter=report,
	)
	return dependencies, events, receipts


#============================================
def test_private_input_round_trip_is_canonical_and_mode_0600(tmp_path: pathlib.Path) -> None:
	"""The child sees the exact V1 value through one private canonical JSON file."""
	value = oracle_input(tmp_path)
	value.workspace_path.mkdir()
	path = value.workspace_path / "service-oracle-input.json"
	service_input.write_private_input(path, value)
	assert service_input.read_private_input(path, value.oracle) == value
	assert stat.S_IMODE(path.stat().st_mode) == 0o600


#============================================
def test_private_input_v1_field_abi_is_exact(tmp_path: pathlib.Path) -> None:
	"""The private ABI carries locators only and no stack/auth selection authority."""
	value = oracle_input(tmp_path)
	assert value.as_value() == {
		"schemaVersion": 1,
		"oracle": "webwork_render_rpc",
		"baseUrl": "https://localhost:55001/",
		"manifestPath": str(value.workspace_path / "disposable.manifest"),
		"seedManifestPath": str(
			value.workspace_path / "service-oracle-seed-manifest.json"
		),
		"workspacePath": str(value.workspace_path),
	}


#============================================
def test_private_input_rejects_unknown_fields_and_noncanonical_json(
	tmp_path: pathlib.Path,
) -> None:
	"""Extensions and alternate JSON encodings cannot silently enlarge the child ABI."""
	value = oracle_input(tmp_path)
	value.workspace_path.mkdir()
	path = value.workspace_path / "service-oracle-input.json"
	payload = value.as_value()
	payload["command"] = ["podman", "compose"]
	write_private(path, json.dumps(payload, indent=2))
	with pytest.raises(service_input.LiveDemoServiceInputError, match="invalid shape"):
		service_input.read_private_input(path)


#============================================
def test_private_input_rejects_insecure_mode_and_foreign_origin(tmp_path: pathlib.Path) -> None:
	"""The file and network boundaries both fail closed before an oracle request."""
	value = oracle_input(tmp_path)
	value.workspace_path.mkdir()
	path = value.workspace_path / "service-oracle-input.json"
	path.write_text(service_input.canonical_json(value), encoding="ascii")
	path.chmod(0o644)
	with pytest.raises(service_input.LiveDemoServiceInputError, match="file is unsafe"):
		service_input.read_private_input(path)
	with pytest.raises(service_input.LiveDemoServiceInputError, match="baseUrl"):
		service_input.decode_value({**value.as_value(), "baseUrl": "https://example.test/"})


#============================================
@pytest.mark.parametrize(
	("oracle", "profile", "child_name"),
	(
		(
			"webwork_render_rpc",
			local_stack_control.models.LiveDemoProfile.WEBWORK_RENDER_RPC,
			"e2e_webwork_render_rpc_child.py",
		),
		(
			"replica_restart",
			local_stack_control.models.LiveDemoProfile.REPLICA_RESTART,
			"e2e_replica_restart_child.mjs",
		),
	),
)
def test_closed_registry_binds_each_oracle_to_one_profile_and_child(
	oracle: str,
	profile: local_stack_control.models.LiveDemoProfile,
	child_name: str,
) -> None:
	"""No public selection can supply a profile, interpreter, command, or path."""
	child = service_owner.child_program(oracle)
	assert (child.profile, child.relative_path.name) == (profile, child_name)
	assert service_owner.selection_parser().parse_args([oracle]).oracle == oracle


#============================================
def test_missing_migration_child_fails_before_lease_or_allocation(tmp_path: pathlib.Path) -> None:
	"""An SO3/SO4 child placeholder never starts or acquires the shared fixture."""
	dependencies, events, receipts = offline_dependencies(tmp_path, "webwork_render_rpc")
	(tmp_path / service_owner.child_program("webwork_render_rpc").relative_path).unlink()
	with pytest.raises(service_owner.LiveDemoServiceOwnerError, match="not installed"):
		service_owner.run_owned_oracle("webwork_render_rpc", dependencies)
	assert (events, receipts) == ([], [])


#============================================
def test_lease_contention_precedes_ports_workspace_build_and_podman(
	tmp_path: pathlib.Path,
) -> None:
	"""A competing browser or developer holder stops every service-owner side effect."""
	dependencies, events, receipts = offline_dependencies(tmp_path, "replica_restart")
	dependencies = dataclasses.replace(
		dependencies,
		lease_factory=local_stack_control.browser_suite_lease.BrowserSuiteLease.acquire,
	)
	with local_stack_control.browser_suite_lease.BrowserSuiteLease.acquire(tmp_path):
		with pytest.raises(local_stack_control.browser_suite_lease.BrowserSuiteError):
			service_owner.run_owned_oracle("replica_restart", dependencies)
	assert (events, receipts) == ([], [])


#============================================
def test_successful_child_cleans_and_publishes_before_lease_release(
	tmp_path: pathlib.Path,
) -> None:
	"""Success reaps the child, cleans, resets, proves emptiness, then reports under lease."""
	dependencies, events, receipts = offline_dependencies(tmp_path, "webwork_render_rpc")
	receipt = service_owner.run_owned_oracle("webwork_render_rpc", dependencies)
	assert events == [
		"lease", "runner", "reset", "workspace", "ports", "port-check", "selections",
		"target", "topology", "launch", "seed", "child", "cleanup", "reset",
		"workspace", "processes", "report", "release",
	]
	assert receipts == [receipt]


#============================================
def test_failing_child_still_cleans_resets_reports_and_releases(tmp_path: pathlib.Path) -> None:
	"""A nonzero assertion child remains a failing run after complete final cleanup."""
	dependencies, events, receipts = offline_dependencies(tmp_path, "replica_restart", 7)
	with pytest.raises(service_owner.LiveDemoServiceOwnerError, match="assertion child failed"):
		service_owner.run_owned_oracle("replica_restart", dependencies)
	assert events[-6:] == ["cleanup", "reset", "workspace", "processes", "report", "release"]
	assert len(receipts) == 1 and receipts[0].child_reaped and receipts[0].inventory_empty


#============================================
def test_interrupted_wait_terminates_child_before_final_cleanup(tmp_path: pathlib.Path) -> None:
	"""An interrupt cannot hide or leave running the immediately registered child group."""
	dependencies, events, receipts = offline_dependencies(tmp_path, "webwork_render_rpc")
	running = {"child": False}
	observed_sessions: list[local_stack_control.process.ProcessSession] = []

	def interrupted_child(
		child: service_owner.ChildProgram,
		input_path: pathlib.Path,
		runner: local_stack_control.process.CommandRunner,
		root: pathlib.Path,
		session_recorder: service_owner.SessionRecorder,
	) -> service_owner.CompletedOwnerCommand:
		events.append("child")
		session = local_stack_control.process.ProcessSession(8101, 1, "injected", "marker")

		def wait() -> int:
			"""Inject interruption while the exact child is still running."""
			events.append("child-wait")
			running["child"] = True
			raise KeyboardInterrupt

		def terminate() -> None:
			"""Record deterministic exact-group termination and reaping."""
			events.append("child-terminated")
			running["child"] = False

		def spawn(
			argv: list[str],
			environment: dict[str, str],
			cwd: pathlib.Path,
			recorder: service_owner.SessionRecorder,
		) -> service_owner.OwnedProcess:
			"""Return the deterministic injected owner process."""
			events.append("child-register")
			recorder(session)
			return service_owner.OwnedProcess(session, wait, terminate)

		return service_owner.run_owned_argv(
			[sys.executable, "offline-child"], {}, root, session_recorder, spawn
		)

	def process_reader(
		sessions: tuple[local_stack_control.process.ProcessSession, ...],
	) -> tuple[e2e_browser_suite_oracles.ProcessIdentity, ...]:
		events.append("processes")
		observed_sessions.extend(sessions)
		return ()

	dependencies = dataclasses.replace(
		dependencies,
		child_runner=interrupted_child,
		owner_process_reader=process_reader,
	)
	with pytest.raises(KeyboardInterrupt):
		service_owner.run_owned_oracle("webwork_render_rpc", dependencies)
	assert not running["child"] and observed_sessions == [
		local_stack_control.process.ProcessSession(8101, 1, "injected", "marker")
	]
	assert (
		events.index("child-register") < events.index("child-wait")
		and events.index("child-terminated") < events.index("cleanup")
		and receipts[0].inventory_empty
	)


#============================================
def test_completed_wait_drains_descendants_before_return(tmp_path: pathlib.Path) -> None:
	"""A normally exiting direct child cannot leave its owned process group behind."""
	events: list[str] = []
	session = local_stack_control.process.ProcessSession(8102, 1, "injected", "marker")

	def wait() -> int:
		"""Arrange a successful direct child while its descendant remains live."""
		events.append("wait")
		return 0

	def terminate() -> None:
		"""Record the mandatory normal-path group drain."""
		events.append("drain")

	def spawn(
		argv: list[str],
		environment: dict[str, str],
		root: pathlib.Path,
		recorder: service_owner.SessionRecorder,
	) -> service_owner.OwnedProcess:
		"""Return one completed direct child with an independently drainable group."""
		events.append("register")
		recorder(session)
		return service_owner.OwnedProcess(session, wait, terminate)

	result = service_owner.run_owned_argv(
		[sys.executable, "offline-child"], {}, tmp_path, lambda _session: None, spawn
	)
	assert result == service_owner.CompletedOwnerCommand(0, True, session)
	assert events == ["register", "wait", "drain"]


#============================================
def test_owner_process_leak_blocks_receipt_but_releases_lease(tmp_path: pathlib.Path) -> None:
	"""A descendant left after final reset prevents a truthful public cleanup receipt."""
	dependencies, events, receipts = offline_dependencies(tmp_path, "webwork_render_rpc")

	def leaked_process(
		sessions: tuple[local_stack_control.process.ProcessSession, ...],
	) -> tuple[e2e_browser_suite_oracles.ProcessIdentity, ...]:
		events.append("processes")
		return (e2e_browser_suite_oracles.ProcessIdentity(7, 1, 7),)

	dependencies = dataclasses.replace(dependencies, owner_process_reader=leaked_process)
	with pytest.raises(service_owner.LiveDemoServiceOwnerError, match="owner-process inventory") as raised:
		service_owner.run_owned_oracle("webwork_render_rpc", dependencies)
	assert isinstance(raised.value.__cause__, service_owner.OwnerProcessInventoryNonemptyError)
	assert raised.value.__cause__.identities == (
		e2e_browser_suite_oracles.ProcessIdentity(7, 1, 7),
	)
	assert events[-2:] == ["processes", "release"] and receipts == []


#============================================
def assert_stage_failure(
	error: service_owner.LiveDemoServiceOwnerError,
	category: str,
) -> None:
	"""Require one public stage category while retaining its private causal evidence."""
	assert str(error) == category
	assert isinstance(error.__cause__, RuntimeError)


#============================================
def test_lifecycle_cleanup_exception_has_a_safe_stage_category(tmp_path: pathlib.Path) -> None:
	"""Cleanup tool text becomes one fixed public lifecycle category."""
	dependencies, _, _ = offline_dependencies(tmp_path, "webwork_render_rpc")

	def fail_cleanup(*args: object) -> service_owner.CompletedOwnerCommand:
		raise RuntimeError("private boundary failure")

	dependencies = dataclasses.replace(dependencies, lifecycle_cleaner=fail_cleanup)
	with pytest.raises(service_owner.LiveDemoServiceOwnerError) as raised:
		service_owner.run_owned_oracle("webwork_render_rpc", dependencies)
	assert_stage_failure(raised.value, service_owner.LIFECYCLE_CLEANUP_FAILURE)


#============================================
def test_final_reset_exception_has_a_safe_stage_category(tmp_path: pathlib.Path) -> None:
	"""Final reset failures remain distinct from workspace and process proof failures."""
	dependencies, events, _ = offline_dependencies(tmp_path, "webwork_render_rpc")
	reset_calls = 0

	def fail_final_reset(*args: object) -> local_stack_control.models.ProjectSnapshot:
		nonlocal reset_calls
		reset_calls += 1
		events.append("reset")
		if reset_calls == 2:
			raise RuntimeError("private boundary failure")
		return empty_snapshot()

	dependencies = dataclasses.replace(dependencies, resetter=fail_final_reset)
	with pytest.raises(service_owner.LiveDemoServiceOwnerError) as raised:
		service_owner.run_owned_oracle("webwork_render_rpc", dependencies)
	assert_stage_failure(raised.value, service_owner.FINAL_RESET_INVENTORY_FAILURE)
	assert events[-3:] == ["workspace", "processes", "release"]


#============================================
def test_final_inventory_validation_has_the_final_reset_stage_category(
	tmp_path: pathlib.Path,
) -> None:
	"""An exact-reset inventory mismatch is owned by the same final-reset stage."""
	dependencies, _, _ = offline_dependencies(tmp_path, "webwork_render_rpc")
	reset_calls = 0

	def foreign_final_reset(*args: object) -> local_stack_control.models.ProjectSnapshot:
		nonlocal reset_calls
		reset_calls += 1
		if reset_calls == 2:
			return local_stack_control.models.ProjectSnapshot("foreign-project", (), (), ())
		return empty_snapshot()

	dependencies = dataclasses.replace(dependencies, resetter=foreign_final_reset)
	with pytest.raises(service_owner.LiveDemoServiceOwnerError) as raised:
		service_owner.run_owned_oracle("webwork_render_rpc", dependencies)
	assert str(raised.value) == service_owner.FINAL_RESET_INVENTORY_FAILURE
	assert isinstance(raised.value.__cause__, service_owner.LiveDemoServiceOwnerError)


#============================================
def test_final_workspace_exception_has_a_safe_stage_category(tmp_path: pathlib.Path) -> None:
	"""Final workspace proof failures remain independently typed after reset succeeds."""
	dependencies, events, _ = offline_dependencies(tmp_path, "webwork_render_rpc")

	class WorkspaceFailureLease(FakeLease):
		"""Leave one private item only after the final workspace reset."""

		def __init__(self, root: pathlib.Path, local_events: list[str]) -> None:
			super().__init__(root, local_events)
			self.reset_calls = 0

		def reset_workspace(self) -> pathlib.Path:
			self.reset_calls += 1
			result = super().reset_workspace()
			if self.reset_calls == 2:
				write_private(result / "unexpected-private-item", "private boundary failure")
			return result

	def acquire(root: pathlib.Path) -> WorkspaceFailureLease:
		events.append("lease")
		return WorkspaceFailureLease(root, events)

	dependencies = dataclasses.replace(dependencies, lease_factory=acquire)
	with pytest.raises(service_owner.LiveDemoServiceOwnerError) as raised:
		service_owner.run_owned_oracle("webwork_render_rpc", dependencies)
	assert_stage_failure(raised.value, service_owner.FINAL_WORKSPACE_FAILURE)


#============================================
def test_owner_process_exception_has_a_safe_stage_category(tmp_path: pathlib.Path) -> None:
	"""Process inventory tool text becomes one fixed final-process category."""
	dependencies, _, _ = offline_dependencies(tmp_path, "webwork_render_rpc")

	def fail_processes(
		sessions: tuple[local_stack_control.process.ProcessSession, ...],
	) -> tuple[e2e_browser_suite_oracles.ProcessIdentity, ...]:
		raise RuntimeError("private boundary failure")

	dependencies = dataclasses.replace(dependencies, owner_process_reader=fail_processes)
	with pytest.raises(service_owner.LiveDemoServiceOwnerError) as raised:
		service_owner.run_owned_oracle("webwork_render_rpc", dependencies)
	assert_stage_failure(raised.value, service_owner.FINAL_OWNER_PROCESS_READ_FAILURE)


#============================================
@pytest.mark.parametrize(
	("error_type", "category"),
	(
		(
			e2e_browser_suite_oracles.OwnerProcessIdentitySpawnExhaustedError,
			service_owner.FINAL_OWNER_PROCESS_IDENTITY_SPAWN_EXHAUSTED_FAILURE,
		),
		(
			e2e_browser_suite_oracles.OwnerProcessIdentitySpawnPermissionError,
			service_owner.FINAL_OWNER_PROCESS_IDENTITY_SPAWN_PERMISSION_FAILURE,
		),
		(
			e2e_browser_suite_oracles.OwnerProcessIdentitySpawnUnavailableError,
			service_owner.FINAL_OWNER_PROCESS_IDENTITY_SPAWN_UNAVAILABLE_FAILURE,
		),
		(
			e2e_browser_suite_oracles.OwnerProcessIdentitySpawnOtherError,
			service_owner.FINAL_OWNER_PROCESS_IDENTITY_SPAWN_OTHER_FAILURE,
		),
		(
			e2e_browser_suite_oracles.OwnerProcessIdentityOutputError,
			service_owner.FINAL_OWNER_PROCESS_IDENTITY_OUTPUT_FAILURE,
		),
		(
			e2e_browser_suite_oracles.OwnerProcessIdentityExitError,
			service_owner.FINAL_OWNER_PROCESS_IDENTITY_EXIT_FAILURE,
		),
		(
			e2e_browser_suite_oracles.OwnerProcessIdentityDecodeError,
			service_owner.FINAL_OWNER_PROCESS_IDENTITY_DECODE_FAILURE,
		),
		(
			e2e_browser_suite_oracles.OwnerProcessMarkerProbeError,
			service_owner.FINAL_OWNER_PROCESS_MARKER_PROBE_FAILURE,
		),
	),
)
def test_typed_process_reader_failures_keep_their_public_categories(
	tmp_path: pathlib.Path,
	error_type: type[e2e_browser_suite_oracles.BrowserSuiteOracleError],
	category: str,
) -> None:
	"""Typed oracle causes survive aggregation as fixed terminal owner categories."""
	dependencies, _, _ = offline_dependencies(tmp_path, "webwork_render_rpc")

	def fail_processes(
		sessions: tuple[local_stack_control.process.ProcessSession, ...],
	) -> tuple[e2e_browser_suite_oracles.ProcessIdentity, ...]:
		raise error_type("private process probe failure")

	dependencies = dataclasses.replace(dependencies, owner_process_reader=fail_processes)
	with pytest.raises(service_owner.LiveDemoServiceOwnerError) as raised:
		service_owner.run_owned_oracle("webwork_render_rpc", dependencies)
	assert str(raised.value) == category
	assert isinstance(raised.value.__cause__, error_type)


#============================================
def test_owner_marker_descendant_has_the_nonempty_stage_category(tmp_path: pathlib.Path) -> None:
	"""A raw marker match is verified remaining ownership, never an inventory read error."""
	dependencies, _, _ = offline_dependencies(tmp_path, "webwork_render_rpc")

	def marker_descendant(
		sessions: tuple[local_stack_control.process.ProcessSession, ...],
	) -> tuple[e2e_browser_suite_oracles.ProcessIdentity, ...]:
		raise e2e_browser_suite_oracles.OwnerMarkerDescendantError(
			"private owner marker descendant"
		)

	dependencies = dataclasses.replace(dependencies, owner_process_reader=marker_descendant)
	with pytest.raises(service_owner.LiveDemoServiceOwnerError) as raised:
		service_owner.run_owned_oracle("webwork_render_rpc", dependencies)
	assert str(raised.value) == service_owner.FINAL_OWNER_PROCESS_NONEMPTY_FAILURE
	assert isinstance(raised.value.__cause__, e2e_browser_suite_oracles.OwnerMarkerDescendantError)


#============================================
def test_owner_process_invalid_inventory_has_the_read_validation_category(
	tmp_path: pathlib.Path,
) -> None:
	"""An invalid inventory cannot be represented as verified nonempty evidence."""
	dependencies, _, _ = offline_dependencies(tmp_path, "webwork_render_rpc")

	def invalid_processes(
		sessions: tuple[local_stack_control.process.ProcessSession, ...],
	) -> object:
		return ("private process text",)

	dependencies = dataclasses.replace(dependencies, owner_process_reader=invalid_processes)
	with pytest.raises(service_owner.LiveDemoServiceOwnerError) as raised:
		service_owner.run_owned_oracle("webwork_render_rpc", dependencies)
	assert str(raised.value) == service_owner.FINAL_OWNER_PROCESS_RETURN_SHAPE_FAILURE
	assert isinstance(raised.value.__cause__, RuntimeError)


#============================================
def test_receipt_report_exception_has_a_safe_stage_category(tmp_path: pathlib.Path) -> None:
	"""Receipt transport errors cannot expose a serialized receipt or private details."""
	dependencies, _, _ = offline_dependencies(tmp_path, "webwork_render_rpc")

	def fail_report(receipt: service_owner.LiveDemoServiceReceipt) -> None:
		raise RuntimeError("private boundary failure")

	dependencies = dataclasses.replace(dependencies, receipt_reporter=fail_report)
	with pytest.raises(service_owner.LiveDemoServiceOwnerError) as raised:
		service_owner.run_owned_oracle("webwork_render_rpc", dependencies)
	assert_stage_failure(raised.value, service_owner.RECEIPT_REPORT_FAILURE)


#============================================
def test_lease_release_exception_has_a_safe_stage_category(tmp_path: pathlib.Path) -> None:
	"""Lease release errors remain visible without exposing lock implementation details."""
	dependencies, events, _ = offline_dependencies(tmp_path, "webwork_render_rpc")

	class ReleaseFailureLease(FakeLease):
		"""Record the release attempt before raising a private lock failure."""

		def release(self) -> None:
			super().release()
			raise RuntimeError("private boundary failure")

	def acquire(root: pathlib.Path) -> ReleaseFailureLease:
		events.append("lease")
		return ReleaseFailureLease(root, events)

	dependencies = dataclasses.replace(dependencies, lease_factory=acquire)
	with pytest.raises(service_owner.LiveDemoServiceOwnerError) as raised:
		service_owner.run_owned_oracle("webwork_render_rpc", dependencies)
	assert_stage_failure(raised.value, service_owner.LEASE_RELEASE_FAILURE)


#============================================
def test_grouped_stage_failures_keep_two_safe_terminal_categories(tmp_path: pathlib.Path) -> None:
	"""Two stage wrappers retain their private causes and public grouped identities."""
	dependencies, _, receipts = offline_dependencies(tmp_path, "webwork_render_rpc")

	def fail_cleanup(*args: object) -> service_owner.CompletedOwnerCommand:
		raise RuntimeError("private boundary failure")

	def fail_processes(
		sessions: tuple[local_stack_control.process.ProcessSession, ...],
	) -> tuple[e2e_browser_suite_oracles.ProcessIdentity, ...]:
		raise RuntimeError("private boundary failure")

	dependencies = dataclasses.replace(
		dependencies,
		lifecycle_cleaner=fail_cleanup,
		owner_process_reader=fail_processes,
	)
	with pytest.raises(ExceptionGroup) as raised:
		service_owner.run_owned_oracle("webwork_render_rpc", dependencies)
	assert tuple(str(error) for error in raised.value.exceptions) == (
		service_owner.LIFECYCLE_CLEANUP_FAILURE,
		service_owner.FINAL_OWNER_PROCESS_READ_FAILURE,
	)
	assert service_owner.public_failure_message(raised.value) == (
		service_owner.LIFECYCLE_CLEANUP_FAILURE
		+ "; "
		+ service_owner.FINAL_OWNER_PROCESS_READ_FAILURE
	)
	assert receipts == []


#============================================
def test_grouped_process_drain_failure_keeps_its_public_category() -> None:
	"""A normal-path drain failure is not silently replaced by the generic terminal text."""
	error = ExceptionGroup(
		"private lifecycle details",
		(
			service_owner.LiveDemoServiceOwnerError(service_owner.PROCESS_GROUP_DRAIN_FAILURE),
			RuntimeError("private secondary failure"),
		),
	)
	assert service_owner.public_failure_message(error) == service_owner.PROCESS_GROUP_DRAIN_FAILURE


#============================================
def test_command_line_reports_safe_grouped_failure(
	monkeypatch: pytest.MonkeyPatch,
	capsys: pytest.CaptureFixture[str],
) -> None:
	"""The terminal path preserves safe grouped categories instead of becoming generic."""
	def fail() -> None:
		"""Raise the exact cleanup/proof combination from a connected owner run."""
		raise ExceptionGroup(
			"private stack details",
			(
				service_owner.LiveDemoServiceOwnerError(
					"service-oracle lifecycle cleanup did not complete"
				),
				service_owner.LiveDemoServiceOwnerError(
					service_owner.FINAL_OWNER_PROCESS_NONEMPTY_FAILURE
				),
			),
		)

	monkeypatch.setattr(service_owner, "main", fail)
	with pytest.raises(SystemExit) as raised:
		service_owner.command_line_main()
	assert raised.value.code == 1
	assert capsys.readouterr().err == (
		"FAIL: service-oracle lifecycle cleanup did not complete; "
		"service-oracle final owner-process inventory is not empty\n"
	)


#============================================
def test_unrecognized_owner_error_uses_generic_terminal_category() -> None:
	"""Unexpected owner text cannot enter the public terminal failure surface."""
	error = service_owner.LiveDemoServiceOwnerError("private seed manifest at /tmp/secret")
	assert service_owner.public_failure_message(error) == "live-demo service oracle did not complete"


#============================================
def test_public_receipt_contains_only_redacted_target_and_cleanup_facts(
	tmp_path: pathlib.Path,
) -> None:
	"""Private locators and seed/auth material never enter the public JSON projection."""
	dependencies, _, _ = offline_dependencies(tmp_path, "webwork_render_rpc")
	receipt = service_owner.run_owned_oracle("webwork_render_rpc", dependencies)
	value = json.loads(receipt.as_json())
	assert set(value) == {"oracle", "project", "origin", "cleanup"}
	public_json = receipt.as_json()
	assert str(tmp_path) not in public_json and service_owner.SEED_MANIFEST_NAME not in public_json


#============================================
def test_private_seed_manifest_requires_canonical_course_id() -> None:
	"""SO3 course selection receives a private validated ID that never enters the receipt."""
	manifest = {
		"assignmentId": "00000000-0000-4000-8000-000000000001",
		"courseId": "00000000-0000-4000-8000-000000000002",
		"enrollmentId": "00000000-0000-4000-8000-000000000003",
		"problemId": "00000000-0000-4000-8000-000000000004",
		"questionId": "ABC-1234",
		"versionId": "00000000-0000-4000-8000-000000000005",
	}
	canonical = service_owner._canonical_seed_manifest(json.dumps(manifest))
	assert json.loads(canonical)["courseId"] == manifest["courseId"]
	with pytest.raises(service_owner.LiveDemoServiceOwnerError, match="invalid manifest"):
		service_owner._canonical_seed_manifest(
			json.dumps({**manifest, "courseId": "not-a-course-id"})
		)


#============================================
def test_private_course_id_has_no_public_receipt_projection() -> None:
	"""The course-selection identifier remains private even after successful cleanup."""
	course_id = "00000000-0000-4000-8000-000000000002"
	receipt = service_owner.LiveDemoServiceReceipt(
		"webwork_render_rpc",
		local_stack_control.models.LIVE_DEMO_BROWSER_PROJECT,
		"https://localhost:55001/",
		True,
		True,
		True,
		True,
		True,
		True,
		True,
	)
	assert course_id not in receipt.as_json()


#============================================
def test_child_environment_has_one_service_oracle_control(
	tmp_path: pathlib.Path,
	monkeypatch: pytest.MonkeyPatch,
) -> None:
	"""Ambient PLE and Compose settings cannot add authority to an assertion child."""
	monkeypatch.setenv("COMPOSE_FILE", "ambient-compose.yaml")
	monkeypatch.setenv("NODE_TLS_REJECT_UNAUTHORIZED", "0")
	monkeypatch.setenv("PLE_UNSAFE_CONTROL", "ambient")
	monkeypatch.setenv("PYTHONDONTWRITEBYTECODE", "ambient")
	monkeypatch.setenv("PYTHONUNBUFFERED", "ambient")
	environment = service_owner.child_environment(tmp_path / "input.json")
	controls = {
		name: value
		for name, value in environment.items()
		if name.startswith("PLE_") or name.startswith("COMPOSE_")
	}
	assert controls == {
		service_owner.ORACLE_INPUT_ENVIRONMENT_NAME: str(tmp_path / "input.json")
	}
	assert (
		{
			"PYTHONDONTWRITEBYTECODE": environment["PYTHONDONTWRITEBYTECODE"],
			"PYTHONUNBUFFERED": environment["PYTHONUNBUFFERED"],
		},
		{"COMPOSE_FILE", "NODE_TLS_REJECT_UNAUTHORIZED", "PLE_UNSAFE_CONTROL"}
		& set(environment),
	) == (service_owner.OWNER_RUNTIME_PYTHON_ENVIRONMENT, set())

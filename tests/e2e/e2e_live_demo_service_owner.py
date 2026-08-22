"""Own one closed browser-free service oracle in the fixed live-demo stack."""

# Standard Library
import argparse
import dataclasses
import json
import os
import pathlib
import re
import secrets
import shutil
import signal
import stat
import subprocess
import sys
import time
import uuid
from collections.abc import Callable, Mapping, Sequence


SCRIPT_REPOSITORY_ROOT = pathlib.Path(__file__).resolve().parents[2]
sys.path.insert(0, str(SCRIPT_REPOSITORY_ROOT))

# local repo modules
import local_stack_control.browser_suite_lease
import local_stack_control.browser_suite_reset
import local_stack_control.consumer
import local_stack_control.env_file
import local_stack_control.lifecycle
import local_stack_control.live_demo_target
import local_stack_control.models
import local_stack_control.process

import e2e_browser_suite_oracles
import e2e_live_demo_service_input


ORACLE_INPUT_ENVIRONMENT_NAME = "PLE_LIVE_DEMO_SERVICE_ORACLE_INPUT_FILE"
ORACLE_INPUT_NAME = "service-oracle-input.json"
SEED_MANIFEST_NAME = "service-oracle-seed-manifest.json"
SEED_MANIFEST_MAXIMUM_BYTES = 16_384
PROCESS_GROUP_DRAIN_TIMEOUT_SECONDS = 5.0
PROCESS_GROUP_DRAIN_INTERVAL_SECONDS = 0.05
QUESTION_ID_PATTERN = re.compile(r"^[0-9A-HJKMNP-TV-Z]{3}-[0-9A-HJKMNP-TV-Z]{4}$")
TENANT_ID = local_stack_control.lifecycle.LOCAL_TENANT_ID
ELENA_ID = local_stack_control.lifecycle.LOCAL_INSTRUCTOR_ID
MARY_ID = local_stack_control.lifecycle.LOCAL_MARY_ID
OWNER_RUNTIME_ENVIRONMENT_NAMES = (
	"CARGO_HOME",
	"HOME",
	"LANG",
	"LC_ALL",
	"LC_CTYPE",
	"PATH",
	"RUSTUP_HOME",
	"TEMP",
	"TMP",
	"TMPDIR",
)
LIFECYCLE_CLEANUP_FAILURE = "service-oracle lifecycle cleanup did not complete"
FINAL_RESET_INVENTORY_FAILURE = "service-oracle final reset/inventory verification did not complete"
FINAL_WORKSPACE_FAILURE = "service-oracle final workspace verification did not complete"
FINAL_OWNER_PROCESS_IDENTITY_SPAWN_EXHAUSTED_FAILURE = "service-oracle final owner-process identity probe exhausted resources"
FINAL_OWNER_PROCESS_IDENTITY_SPAWN_PERMISSION_FAILURE = "service-oracle final owner-process identity probe permission was denied"
FINAL_OWNER_PROCESS_IDENTITY_SPAWN_UNAVAILABLE_FAILURE = "service-oracle final owner-process identity probe was unavailable"
FINAL_OWNER_PROCESS_IDENTITY_SPAWN_OTHER_FAILURE = "service-oracle final owner-process identity probe could not start"
FINAL_OWNER_PROCESS_IDENTITY_OUTPUT_FAILURE = "service-oracle final owner-process identity probe could not return output"
FINAL_OWNER_PROCESS_IDENTITY_EXIT_FAILURE = "service-oracle final owner-process identity probe exited unsuccessfully"
FINAL_OWNER_PROCESS_IDENTITY_DECODE_FAILURE = "service-oracle final owner-process identity data was invalid"
FINAL_OWNER_PROCESS_MARKER_PROBE_FAILURE = "service-oracle final owner-process marker probe failed"
FINAL_OWNER_PROCESS_RETURN_SHAPE_FAILURE = "service-oracle final owner-process inventory return was invalid"
FINAL_OWNER_PROCESS_READ_FAILURE = "service-oracle final owner-process inventory could not be read"
FINAL_OWNER_PROCESS_NONEMPTY_FAILURE = "service-oracle final owner-process inventory is not empty"
PROCESS_GROUP_DRAIN_FAILURE = "service-oracle process group remains after reaping"
RECEIPT_REPORT_FAILURE = "service-oracle receipt reporting did not complete"
LEASE_RELEASE_FAILURE = "service-oracle lease release did not complete"
PUBLIC_FAILURE_MESSAGES = frozenset(
	(
		"service-oracle lifecycle launch did not complete",
		"service-oracle assertion child failed",
		LIFECYCLE_CLEANUP_FAILURE,
		FINAL_RESET_INVENTORY_FAILURE,
		FINAL_WORKSPACE_FAILURE,
		FINAL_OWNER_PROCESS_IDENTITY_SPAWN_EXHAUSTED_FAILURE,
		FINAL_OWNER_PROCESS_IDENTITY_SPAWN_PERMISSION_FAILURE,
		FINAL_OWNER_PROCESS_IDENTITY_SPAWN_UNAVAILABLE_FAILURE,
		FINAL_OWNER_PROCESS_IDENTITY_SPAWN_OTHER_FAILURE,
		FINAL_OWNER_PROCESS_IDENTITY_OUTPUT_FAILURE,
		FINAL_OWNER_PROCESS_IDENTITY_EXIT_FAILURE,
		FINAL_OWNER_PROCESS_IDENTITY_DECODE_FAILURE,
		FINAL_OWNER_PROCESS_MARKER_PROBE_FAILURE,
		FINAL_OWNER_PROCESS_RETURN_SHAPE_FAILURE,
		FINAL_OWNER_PROCESS_READ_FAILURE,
		FINAL_OWNER_PROCESS_NONEMPTY_FAILURE,
		PROCESS_GROUP_DRAIN_FAILURE,
		RECEIPT_REPORT_FAILURE,
		LEASE_RELEASE_FAILURE,
		"service-oracle lifecycle produced no receipt",
	)
)
PUBLIC_FAILURE_MESSAGE_LIMIT = 2


class LiveDemoServiceOwnerError(local_stack_control.models.ControllerError):
	"""A concise fixed-stack service-oracle ownership failure."""


class OwnerProcessInventoryNonemptyError(RuntimeError):
	"""Private typed owner-process evidence retained behind one safe public category."""

	def __init__(
		self,
		identities: tuple[e2e_browser_suite_oracles.ProcessIdentity, ...],
	) -> None:
		"""Store internal process identities without formatting them into error text."""
		self.identities = identities
		super().__init__("service-oracle final owner-process inventory is not empty")


class OwnerProcessInventoryShapeError(RuntimeError):
	"""The process reader returned a value outside its fixed typed contract."""


@dataclasses.dataclass(frozen=True)
class ChildProgram:
	"""One closed internal assertion-child identity and interpreter."""

	oracle: str
	profile: local_stack_control.models.LiveDemoProfile
	relative_path: pathlib.Path
	interpreter: str


@dataclasses.dataclass(frozen=True)
class CompletedOwnerCommand:
	"""One awaited owner command outcome."""

	returncode: int
	reaped: bool
	session: local_stack_control.process.ProcessSession | None = None


@dataclasses.dataclass(frozen=True)
class OwnedProcess:
	"""One immediately registered owner process with exact reap authority."""

	session: local_stack_control.process.ProcessSession
	waiter: Callable[[], int]
	terminator: Callable[[], None]


@dataclasses.dataclass(frozen=True)
class LiveDemoServiceReceipt:
	"""Public-safe lifecycle evidence for one service oracle."""

	oracle: str
	project: str
	origin: str
	initial_reset_completed: bool
	lifecycle_cleanup_completed: bool
	final_reset_completed: bool
	inventory_empty: bool
	workspace_empty: bool
	child_reaped: bool
	owner_processes_empty: bool

	#============================================
	def as_json(self) -> str:
		"""Encode only the oracle identity, public target, and cleanup facts."""
		# ASVS 15.3.1: private paths, manifests, credentials, cookies, and seed IDs stay absent.
		value = {
			"oracle": self.oracle,
			"project": self.project,
			"origin": self.origin,
			"cleanup": {
				"initialResetCompleted": self.initial_reset_completed,
				"lifecycleCleanupCompleted": self.lifecycle_cleanup_completed,
				"finalResetCompleted": self.final_reset_completed,
				"inventoryEmpty": self.inventory_empty,
				"workspaceEmpty": self.workspace_empty,
				"childReaped": self.child_reaped,
				"ownerProcessesEmpty": self.owner_processes_empty,
			},
		}
		result = json.dumps(value, sort_keys=True, separators=(",", ":"), ensure_ascii=True)
		return result


LeaseFactory = Callable[
	[pathlib.Path], local_stack_control.browser_suite_lease.BrowserSuiteLease
]
RunnerFactory = Callable[[], local_stack_control.process.CommandRunner]
SelectionReader = Callable[[pathlib.Path], Mapping[str, str]]
PortFactory = Callable[[], local_stack_control.live_demo_target.LiveDemoPorts]
PortChecker = Callable[
	[tuple[int, int, int, int], local_stack_control.process.CommandRunner, pathlib.Path], None
]
TargetWriter = Callable[
	[
		pathlib.Path,
		local_stack_control.models.LiveDemoProfile,
		local_stack_control.live_demo_target.LiveDemoPorts,
		Mapping[str, str],
	],
	local_stack_control.live_demo_target.LiveDemoTarget,
]
TopologyValidator = Callable[
	[local_stack_control.process.CommandRunner, pathlib.Path, pathlib.Path], None
]
LifecycleAction = Callable[
	[
		local_stack_control.process.CommandRunner,
		pathlib.Path,
		local_stack_control.live_demo_target.LiveDemoTarget,
		Callable[[local_stack_control.process.ProcessSession], None],
	],
	CompletedOwnerCommand,
]
SeedRunner = Callable[
	[
		str,
		local_stack_control.process.CommandRunner,
		pathlib.Path,
		local_stack_control.live_demo_target.LiveDemoTarget,
		pathlib.Path,
	],
	pathlib.Path,
]
ChildRunner = Callable[
	[
		ChildProgram,
		pathlib.Path,
		local_stack_control.process.CommandRunner,
		pathlib.Path,
		Callable[[local_stack_control.process.ProcessSession], None],
	],
	CompletedOwnerCommand,
]
Resetter = Callable[
	[
		local_stack_control.browser_suite_lease.BrowserSuiteLease,
		local_stack_control.process.CommandRunner,
		pathlib.Path,
	],
	local_stack_control.models.ProjectSnapshot,
]
ReceiptReporter = Callable[[LiveDemoServiceReceipt], None]
OwnerProcessReader = Callable[
	[tuple[local_stack_control.process.ProcessSession, ...]],
	tuple[e2e_browser_suite_oracles.ProcessIdentity, ...],
]
SessionRecorder = Callable[[local_stack_control.process.ProcessSession], None]
ProcessSpawner = Callable[
	[list[str], dict[str, str], pathlib.Path, SessionRecorder], OwnedProcess
]


@dataclasses.dataclass(frozen=True)
class LiveDemoServiceDependencies:
	"""Explicit side-effect seams for the small lease-held owner."""

	root: pathlib.Path
	lease_factory: LeaseFactory
	runner_factory: RunnerFactory
	selection_reader: SelectionReader
	port_factory: PortFactory
	port_checker: PortChecker
	target_writer: TargetWriter
	topology_validator: TopologyValidator
	lifecycle_launcher: LifecycleAction
	seed_runner: SeedRunner
	child_runner: ChildRunner
	lifecycle_cleaner: LifecycleAction
	resetter: Resetter
	owner_process_reader: OwnerProcessReader
	receipt_reporter: ReceiptReporter


#============================================
def child_program(oracle: str) -> ChildProgram:
	"""Resolve one name through the closed internal child registry."""
	# ASVS 1.2.5 and 2.2.1: no caller controls an interpreter, command, or path.
	if oracle == "webwork_render_rpc":
		return ChildProgram(
			oracle,
			local_stack_control.models.LiveDemoProfile.WEBWORK_RENDER_RPC,
			pathlib.Path("tests/e2e/e2e_webwork_render_rpc_child.py"),
			sys.executable,
		)
	if oracle == "replica_restart":
		return ChildProgram(
			oracle,
			local_stack_control.models.LiveDemoProfile.REPLICA_RESTART,
			pathlib.Path("tests/e2e/e2e_replica_restart_child.mjs"),
			"node",
		)
	raise LiveDemoServiceOwnerError("unsupported live-demo service oracle")


#============================================
def require_child_program(root: pathlib.Path, child: ChildProgram) -> pathlib.Path:
	"""Require the fixed child artifact without following a migration placeholder."""
	path = root / child.relative_path
	try:
		metadata = path.lstat()
	except OSError as error:
		raise LiveDemoServiceOwnerError(
			"selected live-demo service oracle child is not installed"
		) from error
	if stat.S_ISLNK(metadata.st_mode) or not stat.S_ISREG(metadata.st_mode):
		raise LiveDemoServiceOwnerError(
			"selected live-demo service oracle child is not installed"
		)
	return path


#============================================
def _runtime_environment() -> dict[str, str]:
	"""Return the narrow ordinary host environment shared by owner commands."""
	base = local_stack_control.env_file.sanitized_runtime_environment(
		local_stack_control.process.current_environment()
	)
	result = {name: base[name] for name in OWNER_RUNTIME_ENVIRONMENT_NAMES if name in base}
	return result


#============================================
def child_environment(input_path: pathlib.Path) -> dict[str, str]:
	"""Grant an assertion child only its input locator beyond normal host settings."""
	result = _runtime_environment()
	result[ORACLE_INPUT_ENVIRONMENT_NAME] = str(input_path)
	return result


#============================================
def _adapter_argv(action: str, manifest_path: pathlib.Path) -> list[str]:
	"""Build one closed lifecycle-adapter action without generic Compose arguments."""
	if action == "launch":
		return [
			sys.executable,
			"-m",
			"local_stack_control._consumer_cli",
			"launch",
			"--manifest",
			str(manifest_path),
			"--timeout-seconds",
			"240",
		]
	if action == "cleanup":
		return [
			sys.executable,
			"-m",
			"local_stack_control._consumer_cli",
			"cleanup",
			"--manifest",
			str(manifest_path),
		]
	raise LiveDemoServiceOwnerError("unsupported service-oracle lifecycle action")


#============================================
def _terminate_process_group(
	child: subprocess.Popen[object],
	session: local_stack_control.process.ProcessSession,
) -> None:
	"""Drain the exact owner group after interruption or direct-child completion."""
	# ASVS 15.4.1 and 15.4.3: only the recorded new-session group can be signalled.
	if session.process_group_id != child.pid or session.process_group_id <= 0:
		raise LiveDemoServiceOwnerError("service-oracle process ownership is invalid")
	try:
		os.killpg(session.process_group_id, signal.SIGTERM)
	except ProcessLookupError:
		pass
	try:
		child.wait(timeout=5)
	except subprocess.TimeoutExpired:
		try:
			os.killpg(session.process_group_id, signal.SIGKILL)
		except ProcessLookupError:
			pass
		child.wait()
	deadline = time.monotonic() + PROCESS_GROUP_DRAIN_TIMEOUT_SECONDS
	while time.monotonic() < deadline:
		try:
			os.killpg(session.process_group_id, 0)
		except ProcessLookupError:
			return
		time.sleep(PROCESS_GROUP_DRAIN_INTERVAL_SECONDS)
	try:
		os.killpg(session.process_group_id, signal.SIGKILL)
	except ProcessLookupError:
		return
	deadline = time.monotonic() + PROCESS_GROUP_DRAIN_TIMEOUT_SECONDS
	while time.monotonic() < deadline:
		try:
			os.killpg(session.process_group_id, 0)
		except ProcessLookupError:
			return
		time.sleep(PROCESS_GROUP_DRAIN_INTERVAL_SECONDS)
	raise LiveDemoServiceOwnerError(PROCESS_GROUP_DRAIN_FAILURE)


#============================================
def _spawn_owned_process(
	argv: list[str],
	environment: dict[str, str],
	root: pathlib.Path,
	session_recorder: SessionRecorder,
) -> OwnedProcess:
	"""Spawn one exact new-session child and return its immediate ownership handle."""
	marker = "ple-owner-" + secrets.token_hex(16)
	effective_environment = local_stack_control.env_file.sanitized_runtime_environment(
		environment
	)
	effective_environment["PLE_BROWSER_SUITE_OWNER_SESSION"] = marker
	child: subprocess.Popen[object] | None = None
	session: local_stack_control.process.ProcessSession | None = None
	try:
		child = subprocess.Popen(argv, env=effective_environment, cwd=root, start_new_session=True)
		session = local_stack_control.process.ProcessSession(
			child.pid, time.time_ns(), "process-group-or-marker", marker
		)
		# ASVS 15.4.2: ownership is visible before control returns to a wait caller.
		session_recorder(session)
	except BaseException:
		if child is not None:
			fallback = session
			if fallback is None:
				fallback = local_stack_control.process.ProcessSession(
					child.pid, time.time_ns(), "process-group-or-marker", marker
				)
			_terminate_process_group(child, fallback)
		raise
	if child is None or session is None:
		raise LiveDemoServiceOwnerError("service-oracle process ownership is invalid")

	def wait() -> int:
		"""Wait for the exact direct child to return."""
		return child.wait()

	def terminate() -> None:
		"""Drain this exact process group after interruption or direct-child completion."""
		_terminate_process_group(child, session)

	result = OwnedProcess(session, wait, terminate)
	return result


#============================================
def run_owned_argv(
	argv: list[str],
	environment: dict[str, str],
	root: pathlib.Path,
	session_recorder: SessionRecorder,
	process_spawner: ProcessSpawner = _spawn_owned_process,
) -> CompletedOwnerCommand:
	"""Register immediately, then drain every exact owned descendant before return."""
	if shutil.which(argv[0]) is None:
		return CompletedOwnerCommand(127, True)
	owned: OwnedProcess | None = None
	try:
		owned = process_spawner(argv, environment, root, session_recorder)
		returncode = owned.waiter()
	except BaseException:
		if owned is not None:
			owned.terminator()
		raise
	if owned is None:
		raise LiveDemoServiceOwnerError("service-oracle process ownership is invalid")
	owned.terminator()
	return CompletedOwnerCommand(returncode, True, owned.session)


#============================================
def _run_lifecycle_action(
	action: str,
	runner: local_stack_control.process.CommandRunner,
	root: pathlib.Path,
	target: local_stack_control.live_demo_target.LiveDemoTarget,
	session_recorder: SessionRecorder,
) -> CompletedOwnerCommand:
	"""Run and await one closed lifecycle action in an owner process session."""
	if not isinstance(runner, local_stack_control.process.SubprocessRunner):
		returncode = runner.stream(
		_adapter_argv(action, target.manifest_path),
		_runtime_environment(),
		root,
	)
		return CompletedOwnerCommand(returncode, True)
	return run_owned_argv(
		_adapter_argv(action, target.manifest_path),
		_runtime_environment(),
		root,
		session_recorder,
	)


#============================================
def launch_lifecycle(
	runner: local_stack_control.process.CommandRunner,
	root: pathlib.Path,
	target: local_stack_control.live_demo_target.LiveDemoTarget,
	session_recorder: SessionRecorder,
) -> CompletedOwnerCommand:
	"""Build and launch the target through the structured production lifecycle."""
	return _run_lifecycle_action("launch", runner, root, target, session_recorder)


#============================================
def clean_lifecycle(
	runner: local_stack_control.process.CommandRunner,
	root: pathlib.Path,
	target: local_stack_control.live_demo_target.LiveDemoTarget,
	session_recorder: SessionRecorder,
) -> CompletedOwnerCommand:
	"""Clean the target through its exact structured lifecycle authority."""
	return _run_lifecycle_action("cleanup", runner, root, target, session_recorder)


#============================================
def _seed_argv(oracle: str, minio_port: int) -> list[str]:
	"""Return the closed host seed command for one service oracle."""
	result = [
		"cargo",
		"tools",
		"e2e-seed",
		"--apply-migrations",
		"--tenant",
		TENANT_ID,
		"--instructor",
		ELENA_ID,
		"--student",
		MARY_ID,
	]
	if oracle == "webwork_render_rpc":
		result.extend(
			[
				"--webwork-pilot",
				"--s3-endpoint",
				f"http://127.0.0.1:{minio_port}",
				"--s3-region",
				"us-east-1",
				"--private-content-bucket",
				"private-content",
			]
		)
	elif oracle != "replica_restart":
		raise LiveDemoServiceOwnerError("unsupported live-demo service oracle")
	return result


#============================================
def _seed_environment(
	oracle: str,
	target: local_stack_control.live_demo_target.LiveDemoTarget,
) -> dict[str, str]:
	"""Grant the host seed only its selected database, secret, and storage capabilities."""
	values = local_stack_control.env_file.env_settings(target.environment_path)
	question_secret = pathlib.Path(values["PLE_QUESTION_ID_SECRET_HOST_FILE"])
	local_stack_control.consumer.require_private_regular_file(
		question_secret, "service-oracle Question ID secret"
	)
	result = _runtime_environment()
	result["PLE_MIGRATION_DATABASE_URL"] = local_stack_control.lifecycle.database_url(values)
	result["PLE_QUESTION_ID_SECRET_FILE"] = str(question_secret)
	if oracle == "webwork_render_rpc":
		result["AWS_ACCESS_KEY_ID"] = values["MINIO_ROOT_USER"]
		result["AWS_SECRET_ACCESS_KEY"] = values["MINIO_ROOT_PASSWORD"]
	return result


#============================================
def _canonical_seed_manifest(output: str) -> str:
	"""Validate and canonicalize the bounded native seed receipt."""
	if len(output.encode("utf-8")) > SEED_MANIFEST_MAXIMUM_BYTES:
		raise LiveDemoServiceOwnerError("service-oracle seed returned an invalid manifest")
	try:
		value = json.loads(output)
	except json.JSONDecodeError as error:
		raise LiveDemoServiceOwnerError(
			"service-oracle seed returned an invalid manifest"
		) from error
	expected = {
		"assignmentId",
		"courseId",
		"enrollmentId",
		"problemId",
		"questionId",
		"versionId",
	}
	if (
		not isinstance(value, dict)
		or set(value) != expected
		or any(not isinstance(item, str) or item == "" for item in value.values())
	):
		raise LiveDemoServiceOwnerError("service-oracle seed returned an invalid manifest")
	for field in ("assignmentId", "courseId", "enrollmentId", "problemId", "versionId"):
		try:
			identifier = uuid.UUID(value[field])
		except ValueError as error:
			raise LiveDemoServiceOwnerError(
				"service-oracle seed returned an invalid manifest"
			) from error
		if str(identifier) != value[field]:
			raise LiveDemoServiceOwnerError("service-oracle seed returned an invalid manifest")
	if QUESTION_ID_PATTERN.fullmatch(value["questionId"]) is None:
		raise LiveDemoServiceOwnerError("service-oracle seed returned an invalid manifest")
	result = json.dumps(value, sort_keys=True, separators=(",", ":"), ensure_ascii=True)
	return result


#============================================
def _write_private_file(path: pathlib.Path, contents: str) -> None:
	"""Create one exact private ASCII file without a permissive mode window."""
	# ASVS 5.3.2 and 13.3.2: only the lease-owned fixed path receives opaque seed IDs.
	flags = os.O_WRONLY | os.O_CREAT | os.O_EXCL | getattr(os, "O_NOFOLLOW", 0)
	try:
		file_descriptor = os.open(path, flags, 0o600)
		with os.fdopen(file_descriptor, "wb") as output:
			output.write(contents.encode("ascii"))
			output.flush()
			os.fsync(output.fileno())
	except OSError as error:
		raise LiveDemoServiceOwnerError(
			"service-oracle seed manifest cannot be stored safely"
		) from error


#============================================
def seed_oracle(
	oracle: str,
	runner: local_stack_control.process.CommandRunner,
	root: pathlib.Path,
	target: local_stack_control.live_demo_target.LiveDemoTarget,
	workspace: pathlib.Path,
) -> pathlib.Path:
	"""Run one bounded data-only host seed and store its private manifest."""
	result = runner.run(
		_seed_argv(oracle, target.ports.minio_api),
		_seed_environment(oracle, target),
		root,
	)
	if not result.ok():
		raise LiveDemoServiceOwnerError("service-oracle seed did not complete")
	manifest_path = workspace / SEED_MANIFEST_NAME
	_write_private_file(manifest_path, _canonical_seed_manifest(result.stdout))
	return manifest_path


#============================================
def run_child(
	child: ChildProgram,
	input_path: pathlib.Path,
	runner: local_stack_control.process.CommandRunner,
	root: pathlib.Path,
	session_recorder: SessionRecorder,
) -> CompletedOwnerCommand:
	"""Run and reap the exact registry child with only its private input locator."""
	argv = [child.interpreter, str(root / child.relative_path)]
	if not isinstance(runner, local_stack_control.process.SubprocessRunner):
		returncode = runner.stream(argv, child_environment(input_path), root)
		return CompletedOwnerCommand(returncode, True)
	return run_owned_argv(
		argv,
		child_environment(input_path),
		root,
		session_recorder,
	)


#============================================
def _report_receipt(receipt: LiveDemoServiceReceipt) -> None:
	"""Print one canonical public-safe receipt."""
	print(receipt.as_json())


#============================================
def default_dependencies() -> LiveDemoServiceDependencies:
	"""Bind the owner to the current checkout's closed production boundaries."""
	result = LiveDemoServiceDependencies(
		root=SCRIPT_REPOSITORY_ROOT,
		lease_factory=local_stack_control.browser_suite_lease.BrowserSuiteLease.acquire,
		runner_factory=local_stack_control.process.SubprocessRunner,
		selection_reader=local_stack_control.env_file.canonical_stack_selections,
		port_factory=local_stack_control.live_demo_target.random_ports,
		port_checker=local_stack_control.process.require_available_loopback_ports,
		target_writer=local_stack_control.live_demo_target.write_private_target,
		topology_validator=local_stack_control.live_demo_target.validate_production_auth_render,
		lifecycle_launcher=launch_lifecycle,
		seed_runner=seed_oracle,
		child_runner=run_child,
		lifecycle_cleaner=clean_lifecycle,
		resetter=local_stack_control.browser_suite_reset.reset_live_demo_browser,
		owner_process_reader=e2e_browser_suite_oracles.owner_processes,
		receipt_reporter=_report_receipt,
	)
	return result


#============================================
def _require_target(
	target: local_stack_control.live_demo_target.LiveDemoTarget,
	child: ChildProgram,
	workspace: pathlib.Path,
) -> None:
	"""Require the shared target to preserve the selected fixed profile and workspace."""
	if (
		target.profile is not child.profile
		or target.project != local_stack_control.models.LIVE_DEMO_BROWSER_PROJECT
		or target.owner != local_stack_control.models.LIVE_DEMO_BROWSER_OWNER
		or target.manifest_path.parent != workspace
		or target.environment_path.parent != workspace
	):
		raise LiveDemoServiceOwnerError("service-oracle target does not match its closed profile")
	local_stack_control.consumer.require_private_regular_file(
		target.manifest_path, "service-oracle target manifest"
	)


#============================================
def _require_seed_manifest(path: pathlib.Path, workspace: pathlib.Path) -> None:
	"""Require the seed hook to return only its fixed private workspace artifact."""
	if path != workspace / SEED_MANIFEST_NAME:
		raise LiveDemoServiceOwnerError("service-oracle seed returned an invalid manifest path")
	local_stack_control.consumer.require_private_regular_file(
		path, "service-oracle seed manifest"
	)


#============================================
def _require_empty_snapshot(snapshot: local_stack_control.models.ProjectSnapshot) -> bool:
	"""Require the exact fixed-project inventory to contain no owned resources."""
	result = (
		snapshot.project == local_stack_control.models.LIVE_DEMO_BROWSER_PROJECT
		and not snapshot.containers
		and not snapshot.volumes
		and not snapshot.networks
	)
	if not result:
		raise LiveDemoServiceOwnerError("service-oracle final inventory is not empty")
	return result


#============================================
def _raise_failures(failures: list[BaseException]) -> None:
	"""Preserve an operation failure together with cleanup failures."""
	if len(failures) == 1:
		raise failures[0]
	if failures:
		raise BaseExceptionGroup("live-demo service-oracle lifecycle failures", failures)


#============================================
def _stage_failure(message: str, error: BaseException) -> LiveDemoServiceOwnerError:
	"""Keep an internal cause while publishing one fixed owner-stage category."""
	if message not in PUBLIC_FAILURE_MESSAGES:
		raise RuntimeError("service-oracle stage has no public failure category")
	result = LiveDemoServiceOwnerError(message)
	result.__cause__ = error
	return result


#============================================
def _public_failure_messages(error: BaseException) -> tuple[str, ...]:
	"""Return at most two fixed safe lifecycle messages from one grouped failure."""
	if isinstance(error, LiveDemoServiceOwnerError):
		message = str(error)
		return (message,) if message in PUBLIC_FAILURE_MESSAGES else ()
	if not isinstance(error, BaseExceptionGroup):
		return ()
	result: list[str] = []
	for nested in error.exceptions:
		for message in _public_failure_messages(nested):
			if message not in result:
				result.append(message)
			if len(result) == PUBLIC_FAILURE_MESSAGE_LIMIT:
				return tuple(result)
	return tuple(result)


#============================================
def public_failure_message(error: BaseException) -> str:
	"""Classify known owner failures without exposing internal exception text."""
	messages = _public_failure_messages(error)
	if not messages:
		return "live-demo service oracle did not complete"
	return "; ".join(messages)


#============================================
def run_owned_oracle(
	oracle: str,
	dependencies: LiveDemoServiceDependencies,
) -> LiveDemoServiceReceipt:
	"""Run one oracle with continuous lease ownership and exact final cleanup."""
	child = child_program(oracle)
	require_child_program(dependencies.root, child)
	# ASVS 15.4.1 and 15.4.3: the shared fixture lock precedes every allocation or mutation.
	lease = dependencies.lease_factory(dependencies.root)
	failures: list[BaseException] = []
	runner: local_stack_control.process.CommandRunner | None = None
	target: local_stack_control.live_demo_target.LiveDemoTarget | None = None
	receipt: LiveDemoServiceReceipt | None = None
	initial_reset_completed = False
	lifecycle_launch_attempted = False
	lifecycle_cleanup_completed = False
	final_reset_completed = False
	inventory_empty = False
	workspace_empty = False
	child_reaped = False
	owner_processes_empty = False
	owner_sessions: list[local_stack_control.process.ProcessSession] = []
	try:
		runner = dependencies.runner_factory()
		dependencies.resetter(lease, runner, dependencies.root)
		initial_reset_completed = True
		workspace = lease.reset_workspace()
		ports = dependencies.port_factory()
		dependencies.port_checker(ports.as_tuple(), runner, dependencies.root)
		selections = dependencies.selection_reader(dependencies.root)
		target = dependencies.target_writer(workspace, child.profile, ports, selections)
		_require_target(target, child, workspace)
		dependencies.topology_validator(runner, dependencies.root, target.manifest_path)
		lifecycle_launch_attempted = True
		launch = dependencies.lifecycle_launcher(
			runner, dependencies.root, target, owner_sessions.append
		)
		if not launch.reaped or launch.returncode != 0:
			raise LiveDemoServiceOwnerError("service-oracle lifecycle launch did not complete")
		seed_manifest = dependencies.seed_runner(
			oracle, runner, dependencies.root, target, workspace
		)
		_require_seed_manifest(seed_manifest, workspace)
		input_path = workspace / ORACLE_INPUT_NAME
		input_value = e2e_live_demo_service_input.LiveDemoServiceOracleInputV1(
			oracle,
			target.origin,
			target.manifest_path,
			seed_manifest,
			workspace,
		)
		e2e_live_demo_service_input.write_private_input(input_path, input_value)
		e2e_live_demo_service_input.read_private_input(input_path, oracle)
		child_result = dependencies.child_runner(
			child, input_path, runner, dependencies.root, owner_sessions.append
		)
		child_reaped = child_result.reaped
		if not child_result.reaped or child_result.returncode != 0:
			raise LiveDemoServiceOwnerError("service-oracle assertion child failed")
	except BaseException as error:
		failures.append(error)
	finally:
		if lifecycle_launch_attempted and runner is not None and target is not None:
			try:
				cleanup = dependencies.lifecycle_cleaner(
					runner, dependencies.root, target, owner_sessions.append
				)
				if not cleanup.reaped or cleanup.returncode != 0:
					raise RuntimeError("service-oracle lifecycle cleanup command failed")
				lifecycle_cleanup_completed = True
			except BaseException as error:
				failures.append(_stage_failure(LIFECYCLE_CLEANUP_FAILURE, error))
		if runner is not None:
			try:
				final_snapshot = dependencies.resetter(lease, runner, dependencies.root)
				final_reset_completed = True
				inventory_empty = _require_empty_snapshot(final_snapshot)
			except BaseException as error:
				failures.append(_stage_failure(FINAL_RESET_INVENTORY_FAILURE, error))
			try:
				final_workspace = lease.reset_workspace()
				workspace_empty = not tuple(final_workspace.iterdir())
				if not workspace_empty:
					raise RuntimeError("service-oracle final workspace is not empty")
			except BaseException as error:
				failures.append(_stage_failure(FINAL_WORKSPACE_FAILURE, error))
			try:
				owner_processes = dependencies.owner_process_reader(tuple(owner_sessions))
				if not isinstance(owner_processes, tuple) or not all(
					isinstance(item, e2e_browser_suite_oracles.ProcessIdentity)
					for item in owner_processes
				):
					raise OwnerProcessInventoryShapeError("service-oracle owner-process inventory is invalid")
			except e2e_browser_suite_oracles.OwnerMarkerDescendantError as error:
				failures.append(_stage_failure(FINAL_OWNER_PROCESS_NONEMPTY_FAILURE, error))
			except e2e_browser_suite_oracles.OwnerProcessIdentitySpawnExhaustedError as error:
				failures.append(_stage_failure(FINAL_OWNER_PROCESS_IDENTITY_SPAWN_EXHAUSTED_FAILURE, error))
			except e2e_browser_suite_oracles.OwnerProcessIdentitySpawnPermissionError as error:
				failures.append(_stage_failure(FINAL_OWNER_PROCESS_IDENTITY_SPAWN_PERMISSION_FAILURE, error))
			except e2e_browser_suite_oracles.OwnerProcessIdentitySpawnUnavailableError as error:
				failures.append(_stage_failure(FINAL_OWNER_PROCESS_IDENTITY_SPAWN_UNAVAILABLE_FAILURE, error))
			except e2e_browser_suite_oracles.OwnerProcessIdentitySpawnOtherError as error:
				failures.append(_stage_failure(FINAL_OWNER_PROCESS_IDENTITY_SPAWN_OTHER_FAILURE, error))
			except e2e_browser_suite_oracles.OwnerProcessIdentityOutputError as error:
				failures.append(_stage_failure(FINAL_OWNER_PROCESS_IDENTITY_OUTPUT_FAILURE, error))
			except e2e_browser_suite_oracles.OwnerProcessIdentityExitError as error:
				failures.append(_stage_failure(FINAL_OWNER_PROCESS_IDENTITY_EXIT_FAILURE, error))
			except e2e_browser_suite_oracles.OwnerProcessIdentityDecodeError as error:
				failures.append(_stage_failure(FINAL_OWNER_PROCESS_IDENTITY_DECODE_FAILURE, error))
			except e2e_browser_suite_oracles.OwnerProcessMarkerProbeError as error:
				failures.append(_stage_failure(FINAL_OWNER_PROCESS_MARKER_PROBE_FAILURE, error))
			except OwnerProcessInventoryShapeError as error:
				failures.append(_stage_failure(FINAL_OWNER_PROCESS_RETURN_SHAPE_FAILURE, error))
			except BaseException as error:
				failures.append(_stage_failure(FINAL_OWNER_PROCESS_READ_FAILURE, error))
			else:
				owner_processes_empty = not owner_processes
				if not owner_processes_empty:
					failures.append(
						_stage_failure(
							FINAL_OWNER_PROCESS_NONEMPTY_FAILURE,
							OwnerProcessInventoryNonemptyError(owner_processes),
						)
					)
		if (
			target is not None
			and final_reset_completed
			and inventory_empty
			and workspace_empty
			and owner_processes_empty
		):
			receipt = LiveDemoServiceReceipt(
				oracle,
				target.project,
				target.origin,
				initial_reset_completed,
				lifecycle_cleanup_completed,
				final_reset_completed,
				inventory_empty,
				workspace_empty,
				child_reaped,
				owner_processes_empty,
			)
			try:
				dependencies.receipt_reporter(receipt)
			except BaseException as error:
				failures.append(_stage_failure(RECEIPT_REPORT_FAILURE, error))
		try:
			lease.release()
		except BaseException as error:
			failures.append(_stage_failure(LEASE_RELEASE_FAILURE, error))
	_raise_failures(failures)
	if receipt is None:
		raise LiveDemoServiceOwnerError("service-oracle lifecycle produced no receipt")
	return receipt


#============================================
def selection_parser() -> argparse.ArgumentParser:
	"""Create the one-argument closed public oracle selector."""
	parser = argparse.ArgumentParser(
		description="Run one browser-free oracle in the fixed production-auth live-demo stack."
	)
	parser.add_argument("oracle", choices=e2e_live_demo_service_input.ORACLE_NAMES)
	return parser


#============================================
def main(argv: Sequence[str] | None = None) -> None:
	"""Parse one oracle and run it through the fixed lease-held owner."""
	arguments = sys.argv[1:] if argv is None else argv
	args = selection_parser().parse_args(list(arguments))
	run_owned_oracle(args.oracle, default_dependencies())


#============================================
def command_line_main() -> None:
	"""Present a bounded failure without exposing private lifecycle material."""
	try:
		main()
	except Exception as error:
		print("FAIL: " + public_failure_message(error), file=sys.stderr)
		raise SystemExit(1) from None


if __name__ == "__main__":
	command_line_main()

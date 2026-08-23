"""Own one selected production-browser journey in a disposable PLE stack."""
import argparse
import dataclasses
import json
import os
import pathlib
import re
import secrets
import sys
from collections.abc import Callable, Mapping, Sequence

SCRIPT_REPOSITORY_ROOT = pathlib.Path(__file__).resolve().parents[2]
sys.path.insert(0, str(SCRIPT_REPOSITORY_ROOT))

import local_stack_control.consumer
import local_stack_control.env_file
import local_stack_control.live_demo_claim_context
import local_stack_control.live_demo_target
import local_stack_control.lifecycle
import local_stack_control.models
import local_stack_control.private_state
import local_stack_control.process

import e2e_browser_scenario_contract
import e2e_browser_scenario_execution
import e2e_browser_screenshot_contract
import e2e_browser_screenshot_owner
import e2e_browser_screenshot_publisher
import e2e_browser_suite_evidence
import e2e_browser_suite_oracles
import e2e_browser_webauthn_continuation
import e2e_browser_scenario_webwork_delivery
browser_scenario_contract = e2e_browser_scenario_contract
browser_scenario_execution = e2e_browser_scenario_execution
browser_suite_oracles = e2e_browser_suite_oracles
browser_webauthn_continuation = e2e_browser_webauthn_continuation
webwork_delivery = e2e_browser_scenario_webwork_delivery
PRIVATE_STATE_RELATIVE_DIRECTORY = pathlib.Path("target") / "live-demo-browser"
PRIVATE_STATE_DIRECTORY_PREFIX = "run-"
LOCAL_SYSADMIN_ID = local_stack_control.live_demo_target.LOCAL_SYSADMIN_ID
MAXIMUM_TITLE_FILTER_CHARACTERS = 180
TITLE_FILTER_PATTERN = re.compile(r"^[A-Za-z0-9][A-Za-z0-9 ,.:_-]*$")
WEBWORK_SEED_RUNTIME_ENVIRONMENT_NAMES = (
	"PATH", "HOME", "CARGO_HOME", "RUSTUP_HOME", "TMPDIR", "LANG", "LC_ALL", "LC_CTYPE",
)
BrowserSuiteError = browser_scenario_execution.BrowserSuiteError
ScenarioRunReceipt = browser_scenario_execution.ScenarioRunReceipt
StateFactory = Callable[[pathlib.Path, pathlib.Path, str], local_stack_control.private_state.PrivateState]
InputWriter = Callable[[pathlib.Path, int, pathlib.Path, browser_scenario_contract.ScenarioContract], None]
PortChecker = Callable[[tuple[int, int, int, int], local_stack_control.process.CommandRunner, pathlib.Path], None]
LifecycleValidator = Callable[
	[local_stack_control.process.CommandRunner, pathlib.Path, pathlib.Path], None
]
CommandRunner = Callable[
	[
		local_stack_control.process.CommandRunner,
		list[str],
		pathlib.Path,
		dict[str, str] | None,
	],
	local_stack_control.process.SessionCommandResult,
]
ProviderReader = Callable[
	[local_stack_control.process.CommandRunner, pathlib.Path, pathlib.Path],
	browser_suite_oracles.ProviderReceipt,
]
InventoryReader = Callable[
	[
		str,
		pathlib.Path,
		local_stack_control.process.CommandRunner,
		pathlib.Path,
		browser_suite_oracles.ProviderReceipt,
		tuple[local_stack_control.process.ProcessSession, ...],
	],
	browser_suite_oracles.SuiteInventory,
]
WebworkCatalogSeeder = Callable[
	[
		local_stack_control.process.CommandRunner,
		pathlib.Path,
		pathlib.Path,
		int,
	],
	webwork_delivery.CatalogBaseline,
]
EvidenceLogReader = Callable[
	[
		local_stack_control.process.CommandRunner,
		pathlib.Path,
		pathlib.Path,
	],
	str,
]
@dataclasses.dataclass(frozen=True)
class BrowserSuiteSelection:
	"""One closed visible-journey selection owned by this real-stack runner."""
	scenario: str | None
	title_filter: str | None
	build_requested: bool
	spec_path: str | None = None
	screenshots: bool = False
@dataclasses.dataclass(frozen=True)
class BrowserSuiteReceipt:
	"""Non-secret evidence for one owned browser-suite lifecycle."""
	scenario: str
	origin: str
	project: str
	private_state_directory: str
	lifecycle_launch_attempted: bool
	lifecycle_launch_completed: bool
	cleanup_attempted: bool
	cleanup_completed: bool
	private_state_removed: bool
	origin_receipt: browser_suite_oracles.OriginReceipt
	before_inventory: browser_suite_oracles.SuiteInventory
	launched_inventory: browser_suite_oracles.SuiteInventory
	after_inventory: browser_suite_oracles.SuiteInventory
	scenario_receipts: tuple["ScenarioRunReceipt", ...] = ()
	final_fixture_evidence: e2e_browser_suite_evidence.FinalFixtureEvidence | None = None
	owner_process_sessions: tuple[local_stack_control.process.ProcessSession, ...] = dataclasses.field(default_factory=tuple, repr=False, compare=False)
	screenshot_evidence: e2e_browser_screenshot_publisher.ScreenshotEvidence | None = None
	def as_json(self) -> str:
		"""Encode stable public lifecycle evidence without private inputs."""
		value = {
			"scenario": self.scenario,
			"origin": self.origin,
			"project": self.project,
			"lifecycleLaunchAttempted": self.lifecycle_launch_attempted,
			"lifecycleLaunchCompleted": self.lifecycle_launch_completed,
			"cleanupAttempted": self.cleanup_attempted,
			"cleanupCompleted": self.cleanup_completed,
			"privateStateRemoved": self.private_state_removed,
			"originReceipt": e2e_browser_suite_evidence.origin_value(self.origin_receipt),
			"beforeInventory": browser_suite_oracles.public_inventory(self.before_inventory),
			"launchedInventory": browser_suite_oracles.public_inventory(self.launched_inventory),
			"afterInventory": browser_suite_oracles.public_inventory(self.after_inventory),
			"scenarioReceipts": [item.as_value() for item in self.scenario_receipts],
			"finalFixtureEvidence": None if self.final_fixture_evidence is None else self.final_fixture_evidence.as_value(),
			"screenshotEvidence": None if self.screenshot_evidence is None else self.screenshot_evidence.as_value(),
		}
		return json.dumps(value, sort_keys=True, separators=(",", ":"), ensure_ascii=True)
@dataclasses.dataclass(frozen=True)
class BrowserSuiteDependencies:
	"""Explicit external boundaries for production code and deterministic owner tests."""
	root: pathlib.Path
	runner: local_stack_control.process.CommandRunner
	selections: Mapping[str, str]
	ports: tuple[int, int, int, int]
	state_factory: StateFactory
	input_writer: InputWriter
	port_checker: PortChecker
	topology_validator: LifecycleValidator
	worker_readiness_checker: LifecycleValidator
	command_runner: CommandRunner
	provider_reader: ProviderReader
	inventory_reader: InventoryReader
	origin_checker: Callable[[pathlib.Path, str], browser_suite_oracles.OriginReceipt]
	cleanup_checker: Callable[[browser_suite_oracles.SuiteInventory], None]
	receipt_reporter: Callable[[BrowserSuiteReceipt], None]
	webwork_catalog_seeder: WebworkCatalogSeeder
	evidence_log_reader: EvidenceLogReader
	# The lease owner installs this private hand-off while it holds the fixed
	# workspace.  It is deliberately separate from the public receipt so image
	# bytes can never reach a reporter, JSON projection, repr, or comparison.
	_screenshot_collector: Callable[[e2e_browser_screenshot_publisher.PendingScreenshotPublication], None] | None = None
def repo_root() -> pathlib.Path:
	"""Return the checkout owning this disposable browser suite."""
	return pathlib.Path(__file__).resolve().parents[2]
def private_file(path: pathlib.Path, content: str | bytes) -> None:
	"""Create one exact mode-0600 private ASCII file."""
	file_descriptor = os.open(path, os.O_WRONLY | os.O_CREAT | os.O_EXCL, 0o600)
	with os.fdopen(file_descriptor, "wb") as output:
		if isinstance(content, str):
			output.write(content.encode("ascii"))
		else:
			output.write(content)
require_webauthn_path = browser_scenario_execution.require_webauthn_path
require_command_success = browser_scenario_execution.require_command_success
playwright_environment = browser_scenario_execution.playwright_environment
validate_browser_input = browser_scenario_execution.validate_browser_input
def adapter_argv(
	action: str,
	manifest_path: pathlib.Path,
	arguments: Sequence[str] = (),
) -> list[str]:
	"""Form one closed lifecycle-adapter invocation."""
	result = [
		sys.executable,
		"-m",
		"local_stack_control._consumer_cli",
		action,
		"--manifest",
		str(manifest_path),
	]
	result.extend(arguments)
	return result
def selection_parser() -> argparse.ArgumentParser:
	"""Create the small public selection interface for the canonical browser suite."""
	result = argparse.ArgumentParser(
		prog="run_playwright_tests.sh",
		description="Run the selected PLE production-browser journey in a fresh disposable stack.",
	)
	result.add_argument(
	"--build",
		action="store_true",
		help="request the production dist/ build owned by the disposable lifecycle",
	)
	result.add_argument("--screenshots", action="store_true", help="capture the closed real-stack visual corpus")
	result.add_argument(
		"--scenario", help="run one named canonical scenario",
	)
	result.add_argument(
		"--grep",
		dest="title_filter",
		help="run one literal title substring within the selected scenario",
	)
	result.add_argument(
		"spec_path",
		nargs="?",
		help="run one approved focused file",
	)
	return result
def require_title_filter(value: str | None) -> str | None:
	"""Accept a readable literal title filter rather than arbitrary Playwright arguments."""
	if value is None:
		return None
	if len(value) > MAXIMUM_TITLE_FILTER_CHARACTERS or TITLE_FILTER_PATTERN.fullmatch(value) is None:
		raise BrowserSuiteError("browser suite title filter must be a short literal test-title substring")
	return value
def parse_selection(argv: Sequence[str]) -> BrowserSuiteSelection:
	"""Resolve public selection through the explicit catalog before allocation."""
	args = selection_parser().parse_args(list(argv))
	selection = BrowserSuiteSelection(
		args.scenario,
		args.title_filter,
		args.build,
		args.spec_path,
		args.screenshots,
	)
	return validate_selection(selection)
def validate_selection(selection: BrowserSuiteSelection) -> BrowserSuiteSelection:
	"""Validate every caller path before it can allocate ports or private state."""
	if not isinstance(selection.build_requested, bool):
		raise BrowserSuiteError("browser suite selection is invalid")
	if not isinstance(selection.scenario, (str, type(None))):
		raise BrowserSuiteError("browser suite selection is invalid")
	if not isinstance(selection.spec_path, (str, type(None))):
		raise BrowserSuiteError("browser suite selection is invalid")
	if not isinstance(selection.screenshots, bool):
		raise BrowserSuiteError("browser suite selection is invalid")
	if selection.screenshots:
		if selection.scenario is not None or selection.spec_path is not None or selection.title_filter is not None:
			raise BrowserSuiteError("browser screenshot capture owns the complete closed selection")
		e2e_browser_screenshot_contract.validate()
		return BrowserSuiteSelection(None, None, True, None, True)
	try:
		browser_scenario_contract.resolve_selection(
			selection.scenario,
			selection.spec_path,
			require_title_filter(selection.title_filter),
		)
	except browser_scenario_contract.ScenarioContractError as error:
		raise BrowserSuiteError(str(error)) from error
	return BrowserSuiteSelection(
		selection.scenario,
		require_title_filter(selection.title_filter),
		selection.build_requested,
		selection.spec_path,
		False,
	)
def playwright_argv(
	contract: browser_scenario_contract.ScenarioContract | BrowserSuiteSelection,
	title_filter: str | None = None,
) -> list[str]:
	"""Build a child command solely from the registered contract."""
	if isinstance(contract, BrowserSuiteSelection):
		selection = validate_selection(contract)
		resolved = browser_scenario_contract.resolve_selection(
			selection.scenario,
			selection.spec_path,
			selection.title_filter,
		)
		if len(resolved) != 1:
			raise BrowserSuiteError("browser suite child argv requires one scenario")
		title_filter = selection.title_filter
		contract = resolved[0]
	result = browser_scenario_execution.playwright_command(contract, title_filter)
	return result
def run_command(
	runner: local_stack_control.process.CommandRunner,
	argv: list[str],
	root: pathlib.Path,
	environment: dict[str, str] | None = None,
) -> local_stack_control.process.SessionCommandResult:
	"""Stream one external boundary while retaining command arguments as an array."""
	result = local_stack_control.process.stream_in_owner_session(runner, argv, environment, root)
	return result
def write_browser_input(
	path: pathlib.Path,
	gateway_port: int,
	claim_context_path: pathlib.Path,
	contract: browser_scenario_contract.ScenarioContract,
) -> None:
	"""Project one selected scenario into Playwright's single strict private ABI."""
	browser_scenario_contract.validate_contract(contract)
	context = local_stack_control.live_demo_claim_context.read_context(claim_context_path)
	if context.sysadmin_user_id != LOCAL_SYSADMIN_ID:
		raise BrowserSuiteError("installed live-demo claim context has the wrong Sysadmin account")
	namespace = browser_scenario_contract.namespace_for(contract.scenario_id, secrets.token_hex(6))
	value: dict[str, object] = {
		"schemaVersion": browser_scenario_contract.SCHEMA_VERSION,
		"scenarioId": contract.scenario_id,
		"namespace": namespace,
		"baseUrl": f"https://localhost:{gateway_port}/",
		"personas": list(contract.personas),
		"baselineReads": list(contract.baseline_reads),
		"sysadminRequirement": contract.sysadmin_requirement,
		"visibleObservation": contract.visible_observation,
	}
	if contract.service_receipt is not None:
		value["serviceReceipt"] = contract.service_receipt
	if contract.fault_transition is not None:
		value["faultTransition"] = contract.fault_transition
	if contract.sysadmin_requirement == "unclaimed":
		value["sysadminOwnershipProof"] = context.ownership_proof
	content = json.dumps(value, separators=(",", ":"), ensure_ascii=True)
	private_file(path, content)
def require_worker_ready(
	runner: local_stack_control.process.CommandRunner,
	manifest_path: pathlib.Path,
	root: pathlib.Path,
) -> None:
	"""Require the production worker readiness receipt before Chromium starts."""
	result = runner.run(
		adapter_argv("read-evidence-logs", manifest_path, ["--claim", "worker_completion"]),
		cwd=root,
	)
	readiness_output = result.stdout + result.stderr
	readiness_marker = "peptidyle worker ready with 6 supported job families"
	if not result.ok() or readiness_marker not in readiness_output:
		raise BrowserSuiteError("live-demo worker did not reach its production-ready state")


def webwork_catalog_seed_argv(minio_port: int) -> list[str]:
	"""Form the catalog-only publication command without private values in argv."""
	return [
		"cargo",
		"tools",
		"e2e-seed",
		"--webwork-catalog-baseline",
		"--apply-migrations",
		"--tenant",
		local_stack_control.lifecycle.LOCAL_TENANT_ID,
		"--instructor",
		local_stack_control.lifecycle.LOCAL_INSTRUCTOR_ID,
		"--s3-endpoint",
		f"http://127.0.0.1:{minio_port}",
		"--s3-region",
		"us-east-1",
		"--private-content-bucket",
		"private-content",
	]


def webwork_catalog_seed_environment(directory: pathlib.Path) -> dict[str, str]:
	"""Grant the host publisher only the private capabilities it needs."""
	values = local_stack_control.env_file.env_settings(directory / "env.local")
	question_secret = pathlib.Path(values["PLE_QUESTION_ID_SECRET_HOST_FILE"])
	local_stack_control.consumer.require_private_regular_file(
		question_secret, "WebWork catalog Question ID secret"
	)
	base = local_stack_control.env_file.sanitized_runtime_environment(dict(os.environ))
	environment = {
		name: base[name]
		for name in WEBWORK_SEED_RUNTIME_ENVIRONMENT_NAMES
		if name in base
	}
	environment["PLE_MIGRATION_DATABASE_URL"] = local_stack_control.lifecycle.database_url(values)
	environment["PLE_QUESTION_ID_SECRET_FILE"] = str(question_secret)
	environment["AWS_ACCESS_KEY_ID"] = values["MINIO_ROOT_USER"]
	environment["AWS_SECRET_ACCESS_KEY"] = values["MINIO_ROOT_PASSWORD"]
	return environment


def seed_webwork_catalog_baseline(
	runner: local_stack_control.process.CommandRunner,
	root: pathlib.Path,
	directory: pathlib.Path,
	minio_port: int,
) -> webwork_delivery.CatalogBaseline:
	"""Publish the irreducible reviewed catalog baseline after the real stack is ready."""
	result = runner.run(
		webwork_catalog_seed_argv(minio_port),
		webwork_catalog_seed_environment(directory),
		root,
	)
	if not result.ok():
		raise BrowserSuiteError("WebWork catalog baseline publication failed")
	try:
		return webwork_delivery.decode_catalog_baseline_receipt(result.stdout)
	except webwork_delivery.WebworkDeliveryEvidenceError as error:
		raise BrowserSuiteError("WebWork catalog baseline receipt is invalid") from error


def redacted_renderer_evidence_logs(contents: str) -> str:
	"""Project service output to content-free renderer event markers only.

	ASVS 15.3.1 and 16.4.2: the suite retains no request, source, provider,
	answer, route, credential, or general log content in its scenario evidence.
	"""
	if not isinstance(contents, str):
		raise BrowserSuiteError("WebWork evidence logs are invalid")
	markers: list[str] = []
	for line in contents.splitlines():
		markers.extend(
			'ple.webwork.cache event="renderer_call"'
			for _match in webwork_delivery.RENDERER_CALL_PATTERN.finditer(line)
		)
	return "\n".join(markers)


def read_webwork_renderer_evidence_logs(
	runner: local_stack_control.process.CommandRunner,
	root: pathlib.Path,
	manifest_path: pathlib.Path,
) -> str:
	"""Read the label-resolved bounded API log window without forwarding raw logs."""
	argv = adapter_argv("read-evidence-logs", manifest_path, ["--claim", "renderer_delivery"])
	result = runner.run(argv, cwd=root)
	if not result.ok():
		raise BrowserSuiteError("WebWork renderer evidence log read failed")
	return redacted_renderer_evidence_logs(result.stdout + result.stderr)
def validate_live_compose_render(
	runner: local_stack_control.process.CommandRunner,
	root: pathlib.Path,
	manifest_path: pathlib.Path,
) -> None:
	"""Parse the shared production-auth topology before disposable services start."""
	local_stack_control.live_demo_target.validate_production_auth_render(
		runner, root, manifest_path
	)
def provider_receipt_for(
	runner: local_stack_control.process.CommandRunner,
	root: pathlib.Path,
	manifest_path: pathlib.Path,
) -> browser_suite_oracles.ProviderReceipt:
	"""Read the exact provider selected by the lifecycle adapter before any browser command."""
	manifest = local_stack_control.consumer.load_manifest(root, manifest_path)
	target = local_stack_control.consumer.disposable_target(runner, root, manifest)
	result = browser_suite_oracles.provider_receipt(target)
	return result
def require_canonical_selections(selections: Mapping[str, str]) -> None:
	"""Accept the complete line-safe canonical selection shape before allocation."""
	try:
		local_stack_control.live_demo_target.require_canonical_selections(selections)
	except local_stack_control.models.ControllerError as error:
		raise BrowserSuiteError(str(error)) from error
def raise_lifecycle_failures(failures: list[BaseException]) -> None:
	"""Raise one failure directly or every simultaneous lifecycle failure together."""
	if len(failures) == 1:
		raise failures[0]
	if failures:
		raise BaseExceptionGroup("browser suite lifecycle failures", failures)
def ordered_execution_contracts(
	contracts: Sequence[browser_scenario_contract.ScenarioContract],
	registry: Sequence[browser_scenario_contract.ScenarioContract] | None = None,
) -> tuple[browser_scenario_contract.ScenarioContract, ...]:
	"""Run the visible first claim immediately before the first claimed target."""
	selected = tuple(contracts)
	if not any(item.sysadmin_requirement == "claimed" for item in selected):
		return selected
	catalog = browser_scenario_contract.scenario_contracts() if registry is None else tuple(registry)
	transition = next((item for item in selected if item.sysadmin_requirement == "unclaimed"), None)
	if transition is None:
		transition = next((item for item in catalog if item.sysadmin_requirement == "unclaimed"), None)
	if transition is None:
		raise BrowserSuiteError(
			"claimed browser scenario requires a registered visible first-claim setup"
		)
	result: list[browser_scenario_contract.ScenarioContract] = []
	transition_added = False
	for contract in selected:
		if contract.sysadmin_requirement == "claimed" and not transition_added:
			result.append(transition)
			transition_added = True
		if contract is transition:
			continue
		result.append(contract)
	if not transition_added:
		result.append(transition)
	return tuple(result)
def default_dependencies() -> BrowserSuiteDependencies:
	"""Create the real external boundaries owned by the standalone runner."""
	root = repo_root()
	ports = local_stack_control.live_demo_target.random_ports().as_tuple()
	result = BrowserSuiteDependencies(
		root,
		local_stack_control.process.SubprocessRunner(),
		local_stack_control.env_file.canonical_stack_selections(root),
		ports,
		local_stack_control.private_state.prepare,
		write_browser_input,
		local_stack_control.process.require_available_loopback_ports,
		validate_live_compose_render,
		require_worker_ready,
		run_command,
		provider_receipt_for,
		browser_suite_oracles.inventory_for,
		browser_suite_oracles.origin_receipt_from_file,
		browser_suite_oracles.empty_after_cleanup,
		report_receipt,
		seed_webwork_catalog_baseline,
		read_webwork_renderer_evidence_logs,
	)
	return result


def report_receipt(receipt: BrowserSuiteReceipt) -> None:
	"""Print one public-safe browser-suite receipt."""
	print("Browser-suite receipt: " + receipt.as_json())


@dataclasses.dataclass
class BrowserSuiteLifecycleState:
	"""Mutable private state for one launch, scenario run, and cleanup sequence."""

	private_state: local_stack_control.private_state.PrivateState
	continuation_path: pathlib.Path
	origin: str
	project: str
	provider: browser_suite_oracles.ProviderReceipt
	before_inventory: browser_suite_oracles.SuiteInventory
	launched_inventory: browser_suite_oracles.SuiteInventory
	after_inventory: browser_suite_oracles.SuiteInventory
	origin_receipt: browser_suite_oracles.OriginReceipt
	scenario_receipts: list[ScenarioRunReceipt]
	sessions: list[local_stack_control.process.ProcessSession]
	screenshot_staging: pathlib.Path | None
	pending_screenshots: (
		e2e_browser_screenshot_publisher.PendingScreenshotPublication | None
	)
	capture_dist_digest: str | None
	failures: list[BaseException]
	manifest_path: pathlib.Path | None = None
	claim_context_path: pathlib.Path | None = None
	lifecycle_launch_attempted: bool = False
	lifecycle_launch_completed: bool = False
	cleanup_attempted: bool = False
	cleanup_completed: bool = False
	private_state_removed: bool = False


def prepare_lifecycle_state(
	selection: BrowserSuiteSelection,
	dependencies: BrowserSuiteDependencies,
) -> BrowserSuiteLifecycleState:
	"""Allocate private suite state after every public input boundary has passed."""
	private_state = dependencies.state_factory(
		dependencies.root,
		PRIVATE_STATE_RELATIVE_DIRECTORY,
		PRIVATE_STATE_DIRECTORY_PREFIX,
	)
	continuation_path = require_webauthn_path(
		browser_webauthn_continuation.continuation_path, private_state.directory
	)
	project = "not-created"
	origin = f"https://localhost:{dependencies.ports[3]}/"
	provider = browser_suite_oracles.ProviderReceipt("unavailable", (), False)
	empty_inventory = browser_suite_oracles.SuiteInventory(
		project, (), (), (), (), (), provider
	)
	result = BrowserSuiteLifecycleState(
		private_state=private_state,
		continuation_path=continuation_path,
		origin=origin,
		project=project,
		provider=provider,
		before_inventory=empty_inventory,
		launched_inventory=empty_inventory,
		after_inventory=empty_inventory,
		origin_receipt=browser_suite_oracles.unavailable_origin_receipt(origin),
		scenario_receipts=[],
		sessions=[],
		screenshot_staging=e2e_browser_screenshot_owner.prepare_staging(
			private_state.directory, selection.screenshots
		),
		pending_screenshots=None,
		capture_dist_digest=None,
		failures=[],
	)
	return result


def require_target_path(path: pathlib.Path | None, description: str) -> pathlib.Path:
	"""Require one target path after private target generation succeeds."""
	if path is None:
		raise BrowserSuiteError("browser suite target has no " + description)
	return path


def launch_production_stack(
	selection: BrowserSuiteSelection,
	dependencies: BrowserSuiteDependencies,
	lifecycle: BrowserSuiteLifecycleState,
) -> tuple[pathlib.Path, pathlib.Path]:
	"""Generate the private target and launch its production services."""
	live_target = local_stack_control.live_demo_target.write_private_target(
		lifecycle.private_state.directory,
		local_stack_control.models.LiveDemoProfile.BROWSER,
		local_stack_control.live_demo_target.ports_from_tuple(dependencies.ports),
		dependencies.selections,
	)
	lifecycle.project = live_target.project
	lifecycle.manifest_path = live_target.manifest_path
	lifecycle.claim_context_path = live_target.claim_context_path
	manifest_path = require_target_path(lifecycle.manifest_path, "manifest")
	claim_context_path = require_target_path(
		lifecycle.claim_context_path, "claim context"
	)
	lifecycle.provider = dependencies.provider_reader(
		dependencies.runner, dependencies.root, manifest_path
	)
	lifecycle.before_inventory = dependencies.inventory_reader(
		lifecycle.project,
		lifecycle.private_state.directory,
		dependencies.runner,
		dependencies.root,
		lifecycle.provider,
		tuple(lifecycle.sessions),
	)
	if selection.build_requested:
		print("Browser-suite: --build uses the production dist/ lifecycle build")
	else:
		print("Browser-suite: lifecycle builds the production dist/ bundle")
	print("Browser-suite: parsing the production-auth Compose topology")
	dependencies.topology_validator(dependencies.runner, dependencies.root, manifest_path)
	print("Browser-suite: starting the isolated production PLE stack")
	lifecycle.lifecycle_launch_attempted = True
	launch_argv = adapter_argv("launch", manifest_path, ["--timeout-seconds", "240"])
	launch_result = dependencies.command_runner(
		dependencies.runner,
		launch_argv,
		dependencies.root,
		None,
	)
	require_command_success(launch_result, launch_argv, lifecycle.sessions)
	lifecycle.lifecycle_launch_completed = True
	dependencies.worker_readiness_checker(
		dependencies.runner, manifest_path, dependencies.root
	)
	lifecycle.capture_dist_digest = e2e_browser_screenshot_owner.capture_dist_digest(
		dependencies.root, selection.screenshots
	)
	lifecycle.launched_inventory = dependencies.inventory_reader(
		lifecycle.project,
		lifecycle.private_state.directory,
		dependencies.runner,
		dependencies.root,
		lifecycle.provider,
		tuple(lifecycle.sessions),
	)
	result = manifest_path, claim_context_path
	return result


def execute_visible_scenarios(
	selection: BrowserSuiteSelection,
	contracts: Sequence[browser_scenario_contract.ScenarioContract],
	execution_contracts: Sequence[browser_scenario_contract.ScenarioContract],
	manifest_path: pathlib.Path,
	claim_context_path: pathlib.Path,
	dependencies: BrowserSuiteDependencies,
	lifecycle: BrowserSuiteLifecycleState,
) -> None:
	"""Execute the selected browser children inside the launched real stack."""
	scenario_dependencies = browser_scenario_execution.ScenarioExecutionDependencies(
		dependencies.root,
		dependencies.runner,
		dependencies.ports,
		dependencies.input_writer,
		dependencies.command_runner,
		dependencies.origin_checker,
		dependencies.webwork_catalog_seeder,
		dependencies.evidence_log_reader,
		adapter_argv,
	)
	scenario_request = browser_scenario_execution.ScenarioExecutionRequest(
		tuple(execution_contracts),
		tuple(contracts),
		selection.title_filter,
		lifecycle.private_state.directory,
		manifest_path,
		claim_context_path,
		lifecycle.continuation_path,
		lifecycle.origin,
		selection.screenshots,
		lifecycle.screenshot_staging,
		lifecycle.capture_dist_digest,
		lifecycle.sessions,
		scenario_dependencies,
	)
	scenario_result = browser_scenario_execution.execute_scenarios(scenario_request)
	lifecycle.origin_receipt = scenario_result.origin_receipt
	lifecycle.scenario_receipts = list(scenario_result.scenario_receipts)
	lifecycle.pending_screenshots = scenario_result.pending_screenshots
	if scenario_result.failure is not None:
		lifecycle.failures.append(scenario_result.failure)


def cleanup_and_observe(
	dependencies: BrowserSuiteDependencies,
	lifecycle: BrowserSuiteLifecycleState,
) -> None:
	"""Clean the launched stack, remove private state, and inspect final ownership."""
	if lifecycle.lifecycle_launch_attempted:
		lifecycle.cleanup_attempted = True
		try:
			manifest_path = require_target_path(lifecycle.manifest_path, "manifest")
			cleanup_argv = adapter_argv("cleanup", manifest_path)
			cleanup_result = dependencies.command_runner(
				dependencies.runner,
				cleanup_argv,
				dependencies.root,
				None,
			)
			require_command_success(
				cleanup_result, cleanup_argv, lifecycle.sessions
			)
			lifecycle.cleanup_completed = True
		except BaseException as error:
			lifecycle.failures.append(error)
	if not lifecycle.cleanup_attempted or lifecycle.cleanup_completed:
		try:
			lifecycle.private_state.remove()
			lifecycle.private_state_removed = True
		except BaseException as error:
			lifecycle.failures.append(error)
	try:
		lifecycle.after_inventory = dependencies.inventory_reader(
			lifecycle.project,
			lifecycle.private_state.directory,
			dependencies.runner,
			dependencies.root,
			lifecycle.provider,
			tuple(lifecycle.sessions),
		)
		if lifecycle.cleanup_completed and lifecycle.private_state_removed:
			dependencies.cleanup_checker(lifecycle.after_inventory)
	except BaseException as error:
		lifecycle.failures.append(error)


def lifecycle_receipt(
	contracts: Sequence[browser_scenario_contract.ScenarioContract],
	lifecycle: BrowserSuiteLifecycleState,
) -> BrowserSuiteReceipt:
	"""Project private lifecycle state into the stable public receipt."""
	screenshot_evidence = (
		None
		if lifecycle.pending_screenshots is None
		else e2e_browser_screenshot_publisher.evidence_for(
			lifecycle.pending_screenshots
		)
	)
	result = BrowserSuiteReceipt(
		contracts[0].scenario_id if len(contracts) == 1 else "all",
		lifecycle.origin,
		lifecycle.project,
		str(lifecycle.private_state.directory),
		lifecycle.lifecycle_launch_attempted,
		lifecycle.lifecycle_launch_completed,
		lifecycle.cleanup_attempted,
		lifecycle.cleanup_completed,
		lifecycle.private_state_removed,
		lifecycle.origin_receipt,
		lifecycle.before_inventory,
		lifecycle.launched_inventory,
		lifecycle.after_inventory,
		tuple(lifecycle.scenario_receipts),
		None,
		tuple(lifecycle.sessions),
		screenshot_evidence,
	)
	return result


def collect_screenshots_and_report(
	receipt: BrowserSuiteReceipt,
	dependencies: BrowserSuiteDependencies,
	lifecycle: BrowserSuiteLifecycleState,
) -> None:
	"""Transfer private screenshots to the lease owner and report public evidence."""
	if lifecycle.pending_screenshots is not None:
		if dependencies._screenshot_collector is None:
			lifecycle.failures.append(
				BrowserSuiteError("screenshot capture requires the lease-owned lifecycle")
			)
		else:
			try:
				dependencies._screenshot_collector(lifecycle.pending_screenshots)
			except BaseException as error:
				lifecycle.failures.append(error)
	# The pending bundle is intentionally retained only by the private collector.
	lifecycle.pending_screenshots = None
	try:
		dependencies.receipt_reporter(receipt)
	except BaseException as error:
		lifecycle.failures.append(error)


def run_selection(
	selection: BrowserSuiteSelection,
	dependencies: BrowserSuiteDependencies,
) -> BrowserSuiteReceipt:
	"""Run one validated journey and preserve operation, cleanup, removal, and reporting failures."""
	selection = validate_selection(selection)
	browser_scenario_contract.validate_registry()
	contracts = browser_scenario_contract.resolve_selection(
		selection.scenario,
		selection.spec_path,
		selection.title_filter,
	)
	if selection.screenshots:
		contracts = e2e_browser_screenshot_contract.ordered_contracts(
			browser_scenario_contract.scenario_contracts()
		)
	execution_contracts = ordered_execution_contracts(contracts)
	require_canonical_selections(dependencies.selections)
	dependencies.port_checker(dependencies.ports, dependencies.runner, dependencies.root)
	lifecycle = prepare_lifecycle_state(selection, dependencies)
	try:
		manifest_path, claim_context_path = launch_production_stack(
			selection, dependencies, lifecycle
		)
		execute_visible_scenarios(
			selection,
			contracts,
			execution_contracts,
			manifest_path,
			claim_context_path,
			dependencies,
			lifecycle,
		)
	except BaseException as error:
		lifecycle.failures.append(error)
	cleanup_and_observe(dependencies, lifecycle)
	receipt = lifecycle_receipt(contracts, lifecycle)
	collect_screenshots_and_report(receipt, dependencies, lifecycle)
	raise_lifecycle_failures(lifecycle.failures)
	return receipt
def run_selected_scenario(
	scenario: str,
	dependencies: BrowserSuiteDependencies,
) -> BrowserSuiteReceipt:
	"""Keep H0 callers on the default unfiltered canonical journey."""
	selection = BrowserSuiteSelection(scenario, None, False)
	return run_selection(selection, dependencies)
def main(argv: Sequence[str] | None = None) -> None:
	"""Run a closed public selection through the shared production-stack owner."""
	arguments = sys.argv[1:] if argv is None else argv
	selection = parse_selection(arguments)
	import e2e_browser_suite_lifecycle
	e2e_browser_suite_lifecycle.run_owned_selection(selection, default_dependencies)
def command_line_main() -> None:
	"""Present closed-selection errors without allocating a stack or printing a traceback."""
	try:
		main()
	except BrowserSuiteError as error:
		print("ERROR: " + str(error), file=sys.stderr)
		raise SystemExit(2) from error
if __name__ == "__main__":
	command_line_main()

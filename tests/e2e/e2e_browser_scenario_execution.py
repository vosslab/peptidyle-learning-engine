"""Execute selected Playwright scenarios inside one already-launched PLE stack."""
import dataclasses
import json
import pathlib
import re
import time
import collections.abc

import local_stack_control.models
import local_stack_control.process

import e2e_browser_fault_orchestrator
import e2e_browser_scenario_contract
import e2e_browser_scenario_webwork_delivery
import e2e_browser_screenshot_owner
import e2e_browser_screenshot_publisher
import e2e_browser_suite_evidence
import e2e_browser_suite_input
import e2e_browser_suite_oracles

browser_scenario_contract = e2e_browser_scenario_contract
browser_suite_oracles = e2e_browser_suite_oracles
webwork_delivery = e2e_browser_scenario_webwork_delivery

PLAYWRIGHT_RUNTIME_ENVIRONMENT_NAMES = (
	"HOME", "LANG", "LC_ALL", "LC_CTYPE", "PATH", "TEMP", "TMP", "TMPDIR",
)

InputWriter = collections.abc.Callable[
	[pathlib.Path, int, browser_scenario_contract.ScenarioContract], None
]
CommandRunner = collections.abc.Callable[
	[
		local_stack_control.process.CommandRunner,
		list[str],
		pathlib.Path,
		dict[str, str] | None,
	],
	local_stack_control.process.SessionCommandResult,
]
OriginChecker = collections.abc.Callable[
	[pathlib.Path, str], browser_suite_oracles.OriginReceipt
]
WebworkCatalogSeeder = collections.abc.Callable[
	[
		local_stack_control.process.CommandRunner,
		pathlib.Path,
		pathlib.Path,
		int,
	],
	webwork_delivery.CatalogBaseline,
]
EvidenceLogReader = collections.abc.Callable[
	[
		local_stack_control.process.CommandRunner,
		pathlib.Path,
		pathlib.Path,
	],
	str,
]
AdapterCommandBuilder = collections.abc.Callable[
	[str, pathlib.Path, collections.abc.Sequence[str]], list[str]
]


class BrowserSuiteError(local_stack_control.models.ControllerError):
	"""A concise production-browser suite infrastructure failure."""


@dataclasses.dataclass(frozen=True)
class ScenarioRunReceipt:
	"""Public evidence for exactly one child projection in a shared lifecycle."""

	scenario_id: str
	namespace: str
	expected_origin: str
	observed_page_origins: tuple[str, ...]
	observed_request_origins: tuple[str, ...]
	child_succeeded: bool
	observed_contexts: tuple[browser_suite_oracles.ContextOriginReceipt, ...] = ()
	fault_transition: str | None = None
	fault_injected: bool = False
	fault_recovered: bool = False
	screenshot_artifacts: tuple[
		e2e_browser_screenshot_publisher.ScreenshotArtifactEvidence, ...
	] = ()
	renderer_call_witness: webwork_delivery.RendererCallWitness | None = None

	def as_value(self) -> dict[str, object]:
		"""Return the stable public representation stored in the suite receipt."""
		result: dict[str, object] = {
			"scenario": self.scenario_id,
			"namespace": self.namespace,
			"expectedOrigin": self.expected_origin,
			"observedPageOrigins": self.observed_page_origins,
			"observedRequestOrigins": self.observed_request_origins,
			"observedContexts": e2e_browser_suite_evidence.context_origins_value(
				self.observed_contexts
			),
			"childSucceeded": self.child_succeeded,
			"faultTransition": self.fault_transition,
			"faultInjected": self.fault_injected,
			"faultRecovered": self.fault_recovered,
			"screenshotArtifacts": [item.as_value() for item in self.screenshot_artifacts],
		}
		if self.renderer_call_witness is not None:
			result["rendererCallWitness"] = self.renderer_call_witness.as_value()
		return result


@dataclasses.dataclass(frozen=True)
class ScenarioExecutionDependencies:
	"""External boundaries needed after the real stack has launched."""

	root: pathlib.Path
	runner: local_stack_control.process.CommandRunner
	ports: tuple[int, int, int, int]
	input_writer: InputWriter
	command_runner: CommandRunner
	origin_checker: OriginChecker
	webwork_catalog_seeder: WebworkCatalogSeeder
	evidence_log_reader: EvidenceLogReader
	adapter_command_builder: AdapterCommandBuilder


@dataclasses.dataclass(frozen=True)
class ScenarioExecutionRequest:
	"""Private execution context for one ordered scenario selection."""

	contracts: tuple[browser_scenario_contract.ScenarioContract, ...]
	selected_contracts: tuple[browser_scenario_contract.ScenarioContract, ...]
	title_filter: str | None
	state_directory: pathlib.Path
	manifest_path: pathlib.Path
	origin: str
	screenshot_mode: bool
	screenshot_staging: pathlib.Path | None
	capture_dist_digest: str | None
	finalize_screenshots: bool
	sessions: list[local_stack_control.process.ProcessSession]
	dependencies: ScenarioExecutionDependencies


@dataclasses.dataclass(frozen=True)
class ScenarioExecutionResult:
	"""Private aggregate returned to the lifecycle owner before cleanup."""

	origin_receipt: browser_suite_oracles.OriginReceipt
	scenario_receipts: tuple[ScenarioRunReceipt, ...]
	pending_screenshots: (
		e2e_browser_screenshot_publisher.PendingScreenshotPublication | None
	)
	failure: BaseException | None


@dataclasses.dataclass(frozen=True)
class WebworkObservation:
	"""Private renderer-evidence window for one WebWork browser child."""

	catalog_input_path: pathlib.Path | None = None
	issuance_acknowledgement_path: pathlib.Path | None = None
	before_logs: str | None = None
	started: float | None = None


@dataclasses.dataclass(frozen=True)
class PreparedScenario:
	"""Validated private inputs and command for one Playwright child."""

	contract: browser_scenario_contract.ScenarioContract
	namespace: str
	origin_receipt_path: pathlib.Path
	child_environment: dict[str, str]
	playwright_command: list[str]
	webwork: WebworkObservation


@dataclasses.dataclass(frozen=True)
class FaultAdapterRunner:
	"""Translate fault-orchestrator actions through the lifecycle adapter."""

	request: ScenarioExecutionRequest

	def __call__(
		self, arguments: list[str]
	) -> local_stack_control.process.SessionCommandResult:
		action = arguments[0]
		adapter_arguments = arguments[1:]
		command = self.request.dependencies.adapter_command_builder(
			action, self.request.manifest_path, adapter_arguments
		)
		result = self.request.dependencies.command_runner(
			self.request.dependencies.runner,
			command,
			self.request.dependencies.root,
			None,
		)
		self.request.sessions.append(result.session)
		return result


def require_command_success(
	result: local_stack_control.process.SessionCommandResult,
	argv: list[str],
	sessions: list[local_stack_control.process.ProcessSession],
) -> None:
	"""Record the owner session and fail closed when its command fails."""
	# ASVS 16.5.3: a failed child command cannot be interpreted as successful evidence.
	sessions.append(result.session)
	if result.returncode != 0:
		raise BrowserSuiteError("production browser-suite command failed: " + argv[0])


def playwright_environment(input_path: pathlib.Path) -> dict[str, str]:
	"""Pass a small runtime allowlist and owner-selected private paths to Playwright."""
	inherited = local_stack_control.process.current_environment()
	# ASVS 13.3.2: each browser child receives only its required private capabilities.
	environment = {
		name: inherited[name]
		for name in PLAYWRIGHT_RUNTIME_ENVIRONMENT_NAMES
		if name in inherited
	}
	environment["PLE_LIVE_DEMO_BROWSER_REQUIRED"] = "1"
	environment["PLE_LIVE_DEMO_BROWSER_INPUT_FILE"] = str(input_path)
	return environment


def validate_browser_input(
	path: pathlib.Path,
	gateway_port: int,
	contract: browser_scenario_contract.ScenarioContract,
	screenshot_mode: bool = False,
) -> None:
	"""Confirm the private Playwright ABI before Chromium can start."""
	# ASVS 2.2.1: validate the closed scenario input before the browser consumes it.
	try:
		e2e_browser_suite_input.validate(path, gateway_port, contract, screenshot_mode)
	except e2e_browser_suite_input.BrowserSuiteInputError as error:
		raise BrowserSuiteError(str(error)) from error


def playwright_command(
	contract: browser_scenario_contract.ScenarioContract,
	title_filter: str | None = None,
) -> list[str]:
	"""Build a Playwright child command solely from a registered contract."""
	browser_scenario_contract.validate_contract(contract)
	result = ["npx", "playwright", "test", contract.spec_path, "--workers=1"]
	if title_filter is not None:
		result.extend(("--grep", re.escape(title_filter)))
	return result


def require_webwork_catalog_baseline(
	function: collections.abc.Callable[..., object], *arguments: object
) -> webwork_delivery.CatalogBaseline:
	"""Keep provider receipt decoding behind the owner's non-secret error boundary."""
	try:
		value = function(*arguments)
	except webwork_delivery.WebworkDeliveryEvidenceError as error:
		raise BrowserSuiteError("WebWork catalog baseline receipt is invalid") from error
	if not isinstance(value, webwork_delivery.CatalogBaseline):
		raise BrowserSuiteError("WebWork catalog baseline receipt is invalid")
	return value


def prepare_webwork_observation(
	request: ScenarioExecutionRequest,
	contract: browser_scenario_contract.ScenarioContract,
	baseline: webwork_delivery.CatalogBaseline | None,
) -> tuple[webwork_delivery.CatalogBaseline | None, WebworkObservation]:
	"""Prepare the irreducible WebWork catalog and renderer evidence window."""
	if contract.scenario_id != webwork_delivery.SCENARIO_ID:
		return baseline, WebworkObservation()
	if baseline is None:
		baseline = require_webwork_catalog_baseline(
			request.dependencies.webwork_catalog_seeder,
			request.dependencies.runner,
			request.dependencies.root,
			request.state_directory,
			request.dependencies.ports[1],
		)
	catalog_input_path = request.state_directory / "webwork-catalog-baseline-input.json"
	webwork_delivery.write_catalog_baseline_input(catalog_input_path, baseline)
	webwork_delivery.validate_catalog_baseline_input(catalog_input_path)
	acknowledgement_path = (
		request.state_directory / "webwork-renderer-issuance-acknowledgement.json"
	)
	result = WebworkObservation(catalog_input_path, acknowledgement_path)
	return baseline, result


def prepare_scenario(
	request: ScenarioExecutionRequest,
	contract: browser_scenario_contract.ScenarioContract,
	webwork: WebworkObservation,
) -> PreparedScenario:
	"""Create and validate the private child inputs before Chromium starts."""
	baseline_receipt = request.state_directory / ".runtime" / "base-course.json"
	browser_scenario_contract.validate_installed_baseline(baseline_receipt, contract)
	input_path = request.state_directory / f"playwright-input-{contract.scenario_id}.json"
	origin_receipt_path = (
		request.state_directory / f"browser-origin-receipt-{contract.scenario_id}.json"
	)
	request.dependencies.input_writer(
		input_path,
		request.dependencies.ports[3],
		contract,
	)
	if request.screenshot_mode:
		e2e_browser_screenshot_owner.add_capture_input(input_path, contract)
	validate_browser_input(
		input_path,
		request.dependencies.ports[3],
		contract,
		request.screenshot_mode,
	)
	input_value = json.loads(input_path.read_text(encoding="ascii"))
	namespace = input_value["namespace"]
	if not isinstance(namespace, str):
		raise BrowserSuiteError("browser suite input namespace is invalid")
	print("Browser-suite: executing visible scenario " + contract.scenario_id)
	child_environment = playwright_environment(input_path)
	child_environment["PLE_LIVE_DEMO_BROWSER_ORIGIN_RECEIPT_FILE"] = str(
		origin_receipt_path
	)
	webwork = start_webwork_observation(request, webwork, child_environment)
	if request.screenshot_staging is not None:
		child_environment["PLE_BROWSER_SUITE_SCREENSHOT_STAGING"] = str(
			request.screenshot_staging
		)
	child_title_filter = (
		request.title_filter if contract in request.selected_contracts else None
	)
	command = playwright_command(contract, child_title_filter)
	result = PreparedScenario(
		contract,
		namespace,
		origin_receipt_path,
		child_environment,
		command,
		webwork,
	)
	return result


def start_webwork_observation(
	request: ScenarioExecutionRequest,
	webwork: WebworkObservation,
	child_environment: dict[str, str],
) -> WebworkObservation:
	"""Open the renderer evidence window immediately before its browser child."""
	if (
		webwork.catalog_input_path is None
		or webwork.issuance_acknowledgement_path is None
	):
		return webwork
	before_logs = request.dependencies.evidence_log_reader(
		request.dependencies.runner,
		request.dependencies.root,
		request.manifest_path,
	)
	started = time.monotonic()
	child_environment["PLE_WEBWORK_CATALOG_BASELINE_INPUT_FILE"] = str(
		webwork.catalog_input_path
	)
	child_environment["PLE_WEBWORK_RENDERER_ISSUANCE_ACK_FILE"] = str(
		webwork.issuance_acknowledgement_path
	)
	result = dataclasses.replace(webwork, before_logs=before_logs, started=started)
	return result


def run_prepared_scenario(
	request: ScenarioExecutionRequest,
	prepared: PreparedScenario,
) -> e2e_browser_fault_orchestrator.FaultScenarioResult | None:
	"""Run the ordinary child or its one registered real fault transition."""
	if prepared.contract.fault_transition is None:
		child_result = request.dependencies.command_runner(
			request.dependencies.runner,
			prepared.playwright_command,
			request.dependencies.root,
			prepared.child_environment,
		)
		require_command_success(
			child_result, prepared.playwright_command, request.sessions
		)
		return None
	fault_request = e2e_browser_fault_orchestrator.FaultScenarioRequest(
		request.dependencies.root,
		request.state_directory,
		request.manifest_path,
		prepared.contract.scenario_id,
		prepared.namespace,
		prepared.playwright_command,
		prepared.child_environment,
	)
	runner = FaultAdapterRunner(request)
	if prepared.contract.fault_transition == "gateway_submit_outage":
		return e2e_browser_fault_orchestrator.run_gateway_submit_outage(
			fault_request,
			runner,
			record_session=request.sessions.append,
		)
	if prepared.contract.fault_transition == "deterministic_grader_exception":
		return e2e_browser_fault_orchestrator.run_deterministic_grader_exception(
			fault_request,
			runner,
			record_session=request.sessions.append,
		)
	raise BrowserSuiteError("browser scenario fault transition is unsupported")


def renderer_call_witness(
	request: ScenarioExecutionRequest,
	prepared: PreparedScenario,
) -> webwork_delivery.RendererCallWitness | None:
	"""Validate visible WebWork issuance and its bounded renderer-call evidence."""
	acknowledgement_path = prepared.webwork.issuance_acknowledgement_path
	if acknowledgement_path is None:
		return None
	if prepared.webwork.before_logs is None or prepared.webwork.started is None:
		raise BrowserSuiteError("WebWork renderer evidence window is incomplete")
	try:
		webwork_delivery.validate_visible_issuance_acknowledgement(
			acknowledgement_path, prepared.namespace
		)
	except webwork_delivery.WebworkDeliveryEvidenceError as error:
		raise BrowserSuiteError(
			"WebWork visible issuance acknowledgement is invalid"
		) from error
	elapsed_seconds = max(1, int(time.monotonic() - prepared.webwork.started) + 1)
	after_logs = request.dependencies.evidence_log_reader(
		request.dependencies.runner,
		request.dependencies.root,
		request.manifest_path,
	)
	result = webwork_delivery.renderer_call_witness(
		prepared.webwork.before_logs, after_logs, elapsed_seconds
	)
	return result


def successful_scenario_receipt(
	request: ScenarioExecutionRequest,
	prepared: PreparedScenario,
	fault_result: e2e_browser_fault_orchestrator.FaultScenarioResult | None,
) -> tuple[ScenarioRunReceipt, browser_suite_oracles.OriginReceipt]:
	"""Validate child evidence and return its public-safe receipt."""
	origin_receipt = request.dependencies.origin_checker(
		prepared.origin_receipt_path, request.origin
	)
	witness = renderer_call_witness(request, prepared)
	receipt = ScenarioRunReceipt(
		scenario_id=prepared.contract.scenario_id,
		namespace=prepared.namespace,
		expected_origin=request.origin,
		observed_page_origins=origin_receipt.observed_page_origins,
		observed_request_origins=origin_receipt.observed_request_origins,
		child_succeeded=True,
		observed_contexts=origin_receipt.observed_contexts,
		fault_transition=(
			None if fault_result is None else fault_result.fault_transition
		),
		fault_injected=False if fault_result is None else fault_result.fault_injected,
		fault_recovered=False if fault_result is None else fault_result.fault_recovered,
		renderer_call_witness=witness,
	)
	return receipt, origin_receipt


def captured_screenshot_result(
	request: ScenarioExecutionRequest,
	receipts: list[ScenarioRunReceipt],
) -> tuple[
	list[ScenarioRunReceipt],
	e2e_browser_screenshot_publisher.PendingScreenshotPublication | None,
]:
	"""Validate staged screenshots and attach their evidence to scenario receipts."""
	if request.screenshot_staging is None:
		return receipts, None
	if not request.finalize_screenshots:
		return receipts, None
	try:
		pending = e2e_browser_screenshot_owner.pending_after_capture(
			request.dependencies.root,
			request.screenshot_staging,
			request.origin,
			request.capture_dist_digest,
		)
	except e2e_browser_screenshot_publisher.ScreenshotPublicationError as error:
		raise BrowserSuiteError(str(error)) from error
	with_artifacts = [
		dataclasses.replace(
			receipt,
			screenshot_artifacts=(
				e2e_browser_screenshot_owner.artifact_evidence_for_scenario(
					pending, receipt.scenario_id
				)
			),
		)
		for receipt in receipts
	]
	return with_artifacts, pending


def execute_scenarios(request: ScenarioExecutionRequest) -> ScenarioExecutionResult:
	"""Execute ordered visible scenarios and return evidence to the lifecycle owner."""
	origin_receipt = browser_suite_oracles.unavailable_origin_receipt(request.origin)
	receipts: list[ScenarioRunReceipt] = []
	baseline: webwork_delivery.CatalogBaseline | None = None
	failure: BaseException | None = None
	for contract in request.contracts:
		baseline, webwork = prepare_webwork_observation(request, contract, baseline)
		prepared = prepare_scenario(request, contract, webwork)
		try:
			fault_result = run_prepared_scenario(request, prepared)
			receipt, origin_receipt = successful_scenario_receipt(
				request, prepared, fault_result
			)
			receipts.append(receipt)
		except BaseException as error:
			receipts.append(
				ScenarioRunReceipt(
					prepared.contract.scenario_id,
					prepared.namespace,
					request.origin,
					(),
					(),
					False,
				)
			)
			message = (
				"browser scenario failed: "
				+ prepared.contract.scenario_id
				+ ": "
				+ str(error)
			)
			failure = BrowserSuiteError(message)
			break
	pending_screenshots = None
	if failure is None:
		receipts, pending_screenshots = captured_screenshot_result(request, receipts)
	result = ScenarioExecutionResult(
		origin_receipt,
		tuple(receipts),
		pending_screenshots,
		failure,
	)
	return result

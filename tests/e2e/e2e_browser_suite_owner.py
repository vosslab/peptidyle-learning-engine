"""Own one selected production-browser journey in a disposable PLE stack."""

import argparse
import base64
import dataclasses
import hashlib
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
import local_stack_control.lifecycle
import local_stack_control.models
import local_stack_control.private_state
import local_stack_control.process

import e2e_browser_scenario_contract
import e2e_browser_suite_oracles

browser_scenario_contract = e2e_browser_scenario_contract
browser_suite_oracles = e2e_browser_suite_oracles


POSTGRES_USER = "ple_live_demo_browser"
POSTGRES_DATABASE = "ple_live_demo_browser"
PRIVATE_STATE_RELATIVE_DIRECTORY = pathlib.Path("target") / "live-demo-browser"
PRIVATE_STATE_DIRECTORY_PREFIX = "run-"
LOCAL_SYSADMIN_ID = "00000000-0000-0000-0000-000000000105"
LIVE_DEMO_SCENARIO = "live_demo"
LIVE_DEMO_SPEC_PATH = "tests/playwright/e2e/live_demo.spec.ts"
MAXIMUM_TITLE_FILTER_CHARACTERS = 180
TITLE_FILTER_PATTERN = re.compile(r"^[A-Za-z0-9][A-Za-z0-9 ,.:_-]*$")
PRIVATE_INPUT_MAXIMUM_BYTES = 1_024
PRIVATE_PROOF_PATTERN = re.compile(r"^[A-Za-z0-9_-]{43}$")
REQUIRED_SELECTION_NAMES = (
	"PLE_WEBWORK_RENDERER_IMAGE",
	"PLE_WEBWORK_RENDERER_BASE_URL",
	"PLE_WEBWORK_RENDERER_ID",
	"PLE_WEBWORK_REQUEST_TIMEOUT_SECONDS",
	"PLE_WEBWORK_MAX_RESPONSE_BYTES",
	"PLE_GATEWAY_IMAGE_SHA256",
	"PLE_POSTGRES_IMAGE_SHA256",
	"PLE_MINIO_IMAGE_SHA256",
	"PLE_MINIO_MC_IMAGE_SHA256",
	"PLE_SECRET_INIT_IMAGE_SHA256",
)
PLAYWRIGHT_RUNTIME_ENVIRONMENT_NAMES = (
	"HOME",
	"LANG",
	"LC_ALL",
	"LC_CTYPE",
	"PATH",
	"TEMP",
	"TMP",
	"TMPDIR",
)


class BrowserSuiteError(local_stack_control.models.ControllerError):
	"""A concise production-browser suite infrastructure failure."""


StateFactory = Callable[
	[pathlib.Path, pathlib.Path, str], local_stack_control.private_state.PrivateState
]
InputWriter = Callable[
	[pathlib.Path, int, pathlib.Path, browser_scenario_contract.ScenarioContract], None
]
PortChecker = Callable[
	[tuple[int, int, int, int], local_stack_control.process.CommandRunner, pathlib.Path], None
]
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


@dataclasses.dataclass(frozen=True)
class BrowserSuiteSelection:
	"""One closed visible-journey selection owned by this real-stack runner."""

	scenario: str | None
	title_filter: str | None
	build_requested: bool
	spec_path: str | None = None


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

	def as_json(self) -> str:
		"""Encode stable public lifecycle evidence without private inputs."""
		value = {
			"scenario": self.scenario,
			"origin": self.origin,
			"project": self.project,
			"privateStateDirectory": self.private_state_directory,
			"lifecycleLaunchAttempted": self.lifecycle_launch_attempted,
			"lifecycleLaunchCompleted": self.lifecycle_launch_completed,
			"cleanupAttempted": self.cleanup_attempted,
			"cleanupCompleted": self.cleanup_completed,
			"privateStateRemoved": self.private_state_removed,
			"originReceipt": {
				"expectedOrigin": self.origin_receipt.expected_origin,
				"observedPageOrigins": self.origin_receipt.observed_page_origins,
				"observedRequestOrigins": self.origin_receipt.observed_request_origins,
			},
			"beforeInventory": browser_suite_oracles.public_inventory(self.before_inventory),
			"launchedInventory": browser_suite_oracles.public_inventory(self.launched_inventory),
			"afterInventory": browser_suite_oracles.public_inventory(self.after_inventory),
			"scenarioReceipts": [item.as_value() for item in self.scenario_receipts],
		}
		result = json.dumps(value, sort_keys=True, separators=(",", ":"), ensure_ascii=True)
		return result


@dataclasses.dataclass(frozen=True)
class ScenarioRunReceipt:
	"""Public evidence for exactly one child projection in a shared lifecycle."""

	scenario_id: str
	namespace: str
	expected_origin: str
	observed_page_origins: tuple[str, ...]
	observed_request_origins: tuple[str, ...]
	child_succeeded: bool

	def as_value(self) -> dict[str, object]:
		"""Return the stable public representation stored in the suite receipt."""
		return {
			"scenario": self.scenario_id,
			"namespace": self.namespace,
			"expectedOrigin": self.expected_origin,
			"observedPageOrigins": self.observed_page_origins,
			"observedRequestOrigins": self.observed_request_origins,
			"childSucceeded": self.child_succeeded,
		}


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


#============================================
def repo_root() -> pathlib.Path:
	"""Return the checkout owning this disposable browser suite."""
	result = pathlib.Path(__file__).resolve().parents[2]
	return result


#============================================
def private_file(path: pathlib.Path, content: str | bytes) -> None:
	"""Create one exact mode-0600 private ASCII file."""
	file_descriptor = os.open(path, os.O_WRONLY | os.O_CREAT | os.O_EXCL, 0o600)
	with os.fdopen(file_descriptor, "wb") as output:
		if isinstance(content, str):
			output.write(content.encode("ascii"))
		else:
			output.write(content)


#============================================
def random_port(base: int) -> int:
	"""Select one bounded owner-local loopback port."""
	result = base + secrets.randbelow(400)
	return result


#============================================
def canonical_secret32() -> str:
	"""Return one unpadded base64url encoding of exactly 32 random bytes."""
	encoded = base64.urlsafe_b64encode(secrets.token_bytes(32)).decode("ascii")
	result = encoded.rstrip("=")
	return result


#============================================
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


#============================================
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


#============================================
def require_title_filter(value: str | None) -> str | None:
	"""Accept a readable literal title filter rather than arbitrary Playwright arguments."""
	if value is None:
		return None
	if len(value) > MAXIMUM_TITLE_FILTER_CHARACTERS or TITLE_FILTER_PATTERN.fullmatch(value) is None:
		raise BrowserSuiteError("browser suite title filter must be a short literal test-title substring")
	return value


#============================================
def parse_selection(argv: Sequence[str]) -> BrowserSuiteSelection:
	"""Resolve public selection through the explicit catalog before allocation."""
	args = selection_parser().parse_args(list(argv))
	selection = BrowserSuiteSelection(
		args.scenario,
		args.title_filter,
		args.build,
		args.spec_path,
	)
	return validate_selection(selection)


#============================================
def validate_selection(selection: BrowserSuiteSelection) -> BrowserSuiteSelection:
	"""Validate every caller path before it can allocate ports or private state."""
	if not isinstance(selection.build_requested, bool):
		raise BrowserSuiteError("browser suite selection is invalid")
	if not isinstance(selection.scenario, (str, type(None))):
		raise BrowserSuiteError("browser suite selection is invalid")
	if not isinstance(selection.spec_path, (str, type(None))):
		raise BrowserSuiteError("browser suite selection is invalid")
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
	)


#============================================
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
	browser_scenario_contract.validate_contract(contract)
	result = ["npx", "playwright", "test", contract.spec_path, "--workers=1"]
	if title_filter is not None:
		result.extend(("--grep", re.escape(title_filter)))
	return result


#============================================
def run_command(
	runner: local_stack_control.process.CommandRunner,
	argv: list[str],
	root: pathlib.Path,
	environment: dict[str, str] | None = None,
) -> local_stack_control.process.SessionCommandResult:
	"""Stream one external boundary while retaining command arguments as an array."""
	result = local_stack_control.process.stream_in_owner_session(runner, argv, environment, root)
	return result


#============================================
def require_command_success(
	result: local_stack_control.process.SessionCommandResult,
	argv: list[str],
	sessions: list[local_stack_control.process.ProcessSession],
) -> None:
	"""Record every launched owner session before a nonzero command becomes a suite failure."""
	sessions.append(result.session)
	if result.returncode != 0:
		raise BrowserSuiteError("production browser-suite command failed: " + argv[0])


#============================================
def write_private_target(
	directory: pathlib.Path,
	postgres_port: int,
	minio_port: int,
	minio_console_port: int,
	gateway_port: int,
	selections: Mapping[str, str],
) -> tuple[str, pathlib.Path, pathlib.Path]:
	"""Create the ordinary-stack environment with production authentication."""
	project = "ple-live-demo-browser-" + secrets.token_hex(6)
	capability_path = directory / "disposable.capability"
	capability = secrets.token_bytes(32)
	private_file(capability_path, capability)
	capability_digest = hashlib.sha256(capability).hexdigest()
	invitation_path = directory / "invitation-secret"
	question_path = directory / "question-id-secret"
	private_file(invitation_path, canonical_secret32())
	private_file(question_path, canonical_secret32())
	renderer_provenance_path = directory / "webwork-renderer.provenance"
	claim_context_path = directory / "live-demo-sysadmin-claim-context.json"
	env_path = directory / "env.local"
	env_content = (
		f"POSTGRES_USER={POSTGRES_USER}\nPOSTGRES_PASSWORD={secrets.token_hex(24)}\n"
		f"POSTGRES_DB={POSTGRES_DATABASE}\nPLE_POSTGRES_HOST_PORT={postgres_port}\n"
		"MINIO_ROOT_USER=ple-live-demo-browser\n"
		f"MINIO_ROOT_PASSWORD={secrets.token_hex(24)}\n"
		f"PLE_MINIO_API_HOST_PORT={minio_port}\nPLE_MINIO_CONSOLE_HOST_PORT={minio_console_port}\n"
		f"PLE_GATEWAY_HOST_PORT={gateway_port}\nPLE_LOCAL_GRADER_PASSWORD={secrets.token_hex(24)}\n"
		f"PLE_PUBLIC_ASSET_BASE_URL=https://localhost:{gateway_port}/public-assets\n"
		"PLE_WEBAUTHN_RP_ID=localhost\nPLE_WEBAUTHN_RP_NAME=Peptidyle Learning Engine\n"
		f"PLE_WEBAUTHN_ORIGIN=https://localhost:{gateway_port}\n"
		"PLE_TRUSTED_PROXY_CIDRS=172.30.255.0/29\nPLE_STORAGE_TOPOLOGY=disposable-local\n"
		f"PLE_INVITATION_TOKEN_SECRET_HOST_FILE={invitation_path}\n"
		f"PLE_QUESTION_ID_SECRET_HOST_FILE={question_path}\n"
		f"PLE_LIVE_DEMO_SYSADMIN_CLAIM_CONTEXT_HOST_FILE={claim_context_path}\n"
		"PLE_LIVE_DEMO_ELENA_INSTRUCTOR_USER_ID=00000000-0000-0000-0000-000000000101\n"
		"PLE_LIVE_DEMO_MARY_STUDENT_USER_ID=00000000-0000-0000-0000-000000000102\n"
		"PLE_LIVE_DEMO_JACK_STUDENT_USER_ID=00000000-0000-0000-0000-000000000103\n"
		"PLE_LIVE_DEMO_AVERY_STUDENT_USER_ID=00000000-0000-0000-0000-000000000104\n"
		f"PLE_LIVE_DEMO_SYSADMIN_USER_ID={LOCAL_SYSADMIN_ID}\n"
		f"PLE_WEBWORK_RENDERER_IMAGE={selections['PLE_WEBWORK_RENDERER_IMAGE']}\n"
		f"PLE_WEBWORK_RENDERER_BASE_URL={selections['PLE_WEBWORK_RENDERER_BASE_URL']}\n"
		f"PLE_WEBWORK_RENDERER_ID={selections['PLE_WEBWORK_RENDERER_ID']}\n"
		f"PLE_WEBWORK_PROVENANCE_FILE={renderer_provenance_path}\n"
		f"PLE_WEBWORK_PROBLEM_JWT_SECRET={secrets.token_hex(32)}\n"
		f"PLE_WEBWORK_SESSION_JWT_SECRET={secrets.token_hex(32)}\n"
		f"PLE_WEBWORK_REQUEST_TIMEOUT_SECONDS={selections['PLE_WEBWORK_REQUEST_TIMEOUT_SECONDS']}\n"
		f"PLE_WEBWORK_MAX_RESPONSE_BYTES={selections['PLE_WEBWORK_MAX_RESPONSE_BYTES']}\n"
		f"PLE_GATEWAY_IMAGE_SHA256={selections['PLE_GATEWAY_IMAGE_SHA256']}\n"
		f"PLE_POSTGRES_IMAGE_SHA256={selections['PLE_POSTGRES_IMAGE_SHA256']}\n"
		f"PLE_MINIO_IMAGE_SHA256={selections['PLE_MINIO_IMAGE_SHA256']}\n"
		f"PLE_MINIO_MC_IMAGE_SHA256={selections['PLE_MINIO_MC_IMAGE_SHA256']}\n"
		f"PLE_SECRET_INIT_IMAGE_SHA256={selections['PLE_SECRET_INIT_IMAGE_SHA256']}\n"
		f"PLE_DISPOSABLE_CAPABILITY_SHA256={capability_digest}\n"
	)
	private_file(env_path, env_content)
	manifest_path = directory / "disposable.manifest"
	manifest_content = (
		"OWNER=live-demo-browser\n"
		f"PROJECT={project}\n"
		f"ENV_FILE={env_path}\n"
		f"CAPABILITY_FILE={capability_path}\n"
	)
	private_file(manifest_path, manifest_content)
	result = project, manifest_path, claim_context_path
	return result


#============================================
def playwright_environment(input_path: pathlib.Path) -> dict[str, str]:
	"""Pass a small runtime allowlist and one private input path to Playwright."""
	inherited = local_stack_control.process.current_environment()
	environment = {
		name: inherited[name]
		for name in PLAYWRIGHT_RUNTIME_ENVIRONMENT_NAMES
		if name in inherited
	}
	environment["PLE_LIVE_DEMO_BROWSER_REQUIRED"] = "1"
	environment["PLE_LIVE_DEMO_BROWSER_INPUT_FILE"] = str(input_path)
	return environment


#============================================
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
	if contract.sysadmin_requirement == "unclaimed":
		value["sysadminOwnershipProof"] = context.ownership_proof
	content = json.dumps(value, separators=(",", ":"), ensure_ascii=True)
	private_file(path, content)


#============================================
def validate_browser_input(
	path: pathlib.Path,
	gateway_port: int,
	contract: browser_scenario_contract.ScenarioContract,
) -> None:
	"""Confirm the private Playwright ABI before Chromium can start."""
	local_stack_control.consumer.require_private_regular_file(path, "browser-suite input")
	contents = path.read_text(encoding="ascii")
	if len(contents.encode("ascii")) > PRIVATE_INPUT_MAXIMUM_BYTES:
		raise BrowserSuiteError("browser-suite input is too large")
	try:
		value = json.loads(contents)
	except json.JSONDecodeError as error:
		raise BrowserSuiteError("browser-suite input is not valid JSON") from error
	if not isinstance(value, dict):
		raise BrowserSuiteError("browser-suite input has an invalid shape")
	required_keys = {
		"schemaVersion",
		"scenarioId",
		"namespace",
		"baseUrl",
		"personas",
		"baselineReads",
		"sysadminRequirement",
		"visibleObservation",
	}
	expected_keys = set(required_keys)
	if contract.service_receipt is not None:
		expected_keys.add("serviceReceipt")
	if contract.sysadmin_requirement == "unclaimed":
		expected_keys.add("sysadminOwnershipProof")
	if set(value) != expected_keys:
		raise BrowserSuiteError("browser-suite input has an invalid shape")
	expected_origin = f"https://localhost:{gateway_port}/"
	if (
		value["schemaVersion"] != browser_scenario_contract.SCHEMA_VERSION
		or value["scenarioId"] != contract.scenario_id
		or value["baseUrl"] != expected_origin
		or not isinstance(value["namespace"], str)
		or browser_scenario_contract.NAMESPACE_PATTERN.fullmatch(value["namespace"])
		is None
		or not value["namespace"].endswith("-" + contract.scenario_id)
	):
		raise BrowserSuiteError("browser-suite input has an invalid shape")
	if (
		not isinstance(value["personas"], list)
		or tuple(value["personas"]) != contract.personas
		or not isinstance(value["baselineReads"], list)
		or tuple(value["baselineReads"]) != contract.baseline_reads
		or value["sysadminRequirement"] != contract.sysadmin_requirement
		or value["visibleObservation"] != contract.visible_observation
		or value.get("serviceReceipt") != contract.service_receipt
	):
		raise BrowserSuiteError("browser-suite input has an invalid shape")
	proof = value.get("sysadminOwnershipProof")
	if contract.sysadmin_requirement == "unclaimed":
		if (
			not isinstance(proof, str)
			or PRIVATE_PROOF_PATTERN.fullmatch(proof) is None
		):
			raise BrowserSuiteError("browser-suite input has an invalid shape")
		try:
			decoded = base64.urlsafe_b64decode(proof + "=")
		except ValueError as error:
			raise BrowserSuiteError("browser-suite input has an invalid shape") from error
		canonical_proof = base64.urlsafe_b64encode(decoded).decode("ascii").rstrip("=")
		if len(decoded) != 32 or canonical_proof != proof:
			raise BrowserSuiteError("browser-suite input has an invalid shape")
	elif proof is not None:
		raise BrowserSuiteError("browser-suite input has an invalid shape")
	canonical_value: dict[str, object] = {
		"schemaVersion": value["schemaVersion"],
		"scenarioId": value["scenarioId"],
		"namespace": value["namespace"],
		"baseUrl": value["baseUrl"],
		"personas": value["personas"],
		"baselineReads": value["baselineReads"],
		"sysadminRequirement": value["sysadminRequirement"],
		"visibleObservation": value["visibleObservation"],
	}
	if contract.service_receipt is not None:
		canonical_value["serviceReceipt"] = value["serviceReceipt"]
	if contract.sysadmin_requirement == "unclaimed":
		canonical_value["sysadminOwnershipProof"] = value["sysadminOwnershipProof"]
	canonical = json.dumps(canonical_value, separators=(",", ":"), ensure_ascii=True)
	if contents != canonical:
		raise BrowserSuiteError("browser-suite input must use canonical ASCII JSON")


#============================================
def require_worker_ready(
	runner: local_stack_control.process.CommandRunner,
	manifest_path: pathlib.Path,
	root: pathlib.Path,
) -> None:
	"""Require the production worker readiness receipt before Chromium starts."""
	result = runner.run(adapter_argv("read-evidence-logs", manifest_path), cwd=root)
	readiness_output = result.stdout + result.stderr
	readiness_marker = "peptidyle worker ready with 6 supported job families"
	if not result.ok() or readiness_marker not in readiness_output:
		raise BrowserSuiteError("live-demo worker did not reach its production-ready state")


#============================================
def validate_live_compose_render(
	runner: local_stack_control.process.CommandRunner,
	root: pathlib.Path,
	manifest_path: pathlib.Path,
) -> None:
	"""Parse the live provider topology before disposable services start."""
	manifest = local_stack_control.consumer.load_manifest(root, manifest_path)
	disposable = local_stack_control.consumer.disposable_target(runner, root, manifest)
	local_stack_control.lifecycle.bootstrap_default_state(disposable)
	values = local_stack_control.env_file.env_settings(disposable.target.env_file)
	if "PLE_LOCAL_AUTH_HOST_FILE" in values:
		raise BrowserSuiteError("live-demo environment selected local-file authentication")
	rendered = local_stack_control.lifecycle.validate_lifecycle(disposable, runner, root)
	if "PLE_LOCAL_AUTH_HOST_FILE" in rendered or "/run/ple/local-identities.json" in rendered:
		raise BrowserSuiteError("live-demo Compose render retained a local-auth setting")


#============================================
def require_live_demo_selection(scenario: str) -> None:
	"""Select the one H0 production journey before lifecycle creation begins."""
	if scenario != LIVE_DEMO_SCENARIO:
		raise BrowserSuiteError("browser suite scenario must be live_demo")


#============================================
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


#============================================
def require_canonical_selections(selections: Mapping[str, str]) -> None:
	"""Accept the complete line-safe canonical selection shape before allocation."""
	for name in REQUIRED_SELECTION_NAMES:
		try:
			value = selections[name]
		except KeyError as error:
			raise BrowserSuiteError("browser suite selections omit " + name) from error
		if not isinstance(value, str) or value == "" or value.strip() != value:
			raise BrowserSuiteError("browser suite selection is unsafe: " + name)
		if "\n" in value or "\r" in value or "\x00" in value:
			raise BrowserSuiteError("browser suite selection is unsafe: " + name)
		try:
			value.encode("ascii")
		except UnicodeEncodeError as error:
			raise BrowserSuiteError("browser suite selection is unsafe: " + name) from error


#============================================
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


#============================================
def default_dependencies() -> BrowserSuiteDependencies:
	"""Create the real external boundaries owned by the standalone runner."""
	root = repo_root()
	ports = (random_port(53500), random_port(54000), random_port(54500), random_port(55000))
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
		lambda receipt: print("Browser-suite receipt: " + receipt.as_json()),
	)
	return result


#============================================
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
	execution_contracts = ordered_execution_contracts(contracts)
	require_canonical_selections(dependencies.selections)
	dependencies.port_checker(dependencies.ports, dependencies.runner, dependencies.root)
	state = dependencies.state_factory(
		dependencies.root,
		PRIVATE_STATE_RELATIVE_DIRECTORY,
		PRIVATE_STATE_DIRECTORY_PREFIX,
	)
	project = "not-created"
	origin = f"https://localhost:{dependencies.ports[3]}/"
	lifecycle_launch_attempted = False
	lifecycle_launch_completed = False
	cleanup_attempted = False
	cleanup_completed = False
	private_state_removed = False
	provider = browser_suite_oracles.ProviderReceipt("unavailable", (), False)
	empty_inventory = browser_suite_oracles.SuiteInventory(
		project, (), (), (), (), (), provider
	)
	before_inventory = empty_inventory
	launched_inventory = empty_inventory
	after_inventory = empty_inventory
	origin_receipt = browser_suite_oracles.unavailable_origin_receipt(origin)
	scenario_receipts: list[ScenarioRunReceipt] = []
	sessions: list[local_stack_control.process.ProcessSession] = []
	failures: list[BaseException] = []
	try:
		project, manifest_path, claim_context_path = write_private_target(
			state.directory,
			*dependencies.ports,
			dependencies.selections,
		)
		provider = dependencies.provider_reader(dependencies.runner, dependencies.root, manifest_path)
		before_inventory = dependencies.inventory_reader(
			project,
			state.directory,
			dependencies.runner,
			dependencies.root,
			provider,
			tuple(sessions),
		)
		if selection.build_requested:
			print("Browser-suite: --build uses the production dist/ lifecycle build")
		else:
			print("Browser-suite: lifecycle builds the production dist/ bundle")
		print("Browser-suite: parsing the production-auth Compose topology")
		dependencies.topology_validator(dependencies.runner, dependencies.root, manifest_path)
		print("Browser-suite: starting the isolated production PLE stack")
		lifecycle_launch_attempted = True
		launch_argv = adapter_argv("launch", manifest_path, ["--timeout-seconds", "240"])
		launch_result = dependencies.command_runner(
			dependencies.runner,
			launch_argv,
			dependencies.root,
			None,
		)
		require_command_success(launch_result, launch_argv, sessions)
		lifecycle_launch_completed = True
		dependencies.worker_readiness_checker(dependencies.runner, manifest_path, dependencies.root)
		launched_inventory = dependencies.inventory_reader(
			project,
			state.directory,
			dependencies.runner,
			dependencies.root,
			provider,
			tuple(sessions),
		)
		for contract in execution_contracts:
			baseline_receipt = state.directory / ".runtime" / "base-course.json"
			browser_scenario_contract.validate_installed_baseline(
				baseline_receipt,
				contract,
			)
			input_path = state.directory / f"playwright-input-{contract.scenario_id}.json"
			origin_receipt_path = state.directory / f"browser-origin-receipt-{contract.scenario_id}.json"
			dependencies.input_writer(input_path, dependencies.ports[3], claim_context_path, contract)
			validate_browser_input(input_path, dependencies.ports[3], contract)
			namespace = json.loads(input_path.read_text(encoding="ascii"))["namespace"]
			print("Browser-suite: executing visible scenario " + contract.scenario_id)
			child_environment = playwright_environment(input_path)
			child_environment["PLE_LIVE_DEMO_BROWSER_ORIGIN_RECEIPT_FILE"] = str(origin_receipt_path)
			child_title_filter = selection.title_filter if contract in contracts else None
			playwright_command = playwright_argv(contract, child_title_filter)
			try:
				child_result = dependencies.command_runner(
					dependencies.runner,
					playwright_command,
					dependencies.root,
					child_environment,
				)
				require_command_success(child_result, playwright_command, sessions)
				origin_receipt = dependencies.origin_checker(origin_receipt_path, origin)
				scenario_receipts.append(
					ScenarioRunReceipt(
						contract.scenario_id,
						namespace,
						origin,
						origin_receipt.observed_page_origins,
						origin_receipt.observed_request_origins,
						True,
					)
				)
			except BaseException as error:
				scenario_receipts.append(
					ScenarioRunReceipt(contract.scenario_id, namespace, origin, (), (), False)
				)
				message = "browser scenario failed: " + contract.scenario_id + ": " + str(error)
				failures.append(BrowserSuiteError(message))
				break
		print("Browser-suite: PASS")
	except BaseException as error:
		failures.append(error)
	if lifecycle_launch_attempted:
		cleanup_attempted = True
		try:
			cleanup_argv = adapter_argv("cleanup", manifest_path)
			cleanup_result = dependencies.command_runner(
				dependencies.runner,
				cleanup_argv,
				dependencies.root,
				None,
			)
			require_command_success(cleanup_result, cleanup_argv, sessions)
			cleanup_completed = True
		except BaseException as error:
			failures.append(error)
	if not cleanup_attempted or cleanup_completed:
		try:
			state.remove()
			private_state_removed = True
		except BaseException as error:
			failures.append(error)
	try:
		after_inventory = dependencies.inventory_reader(
			project,
			state.directory,
			dependencies.runner,
			dependencies.root,
			provider,
			tuple(sessions),
		)
		if cleanup_completed and private_state_removed:
			dependencies.cleanup_checker(after_inventory)
	except BaseException as error:
		failures.append(error)
	receipt = BrowserSuiteReceipt(
		contracts[0].scenario_id if len(contracts) == 1 else "all",
		origin,
		project,
		str(state.directory),
		lifecycle_launch_attempted,
		lifecycle_launch_completed,
		cleanup_attempted,
		cleanup_completed,
		private_state_removed,
		origin_receipt,
		before_inventory,
		launched_inventory,
		after_inventory,
		tuple(scenario_receipts),
	)
	try:
		dependencies.receipt_reporter(receipt)
	except BaseException as error:
		failures.append(error)
	raise_lifecycle_failures(failures)
	return receipt


#============================================
def run_selected_scenario(
	scenario: str,
	dependencies: BrowserSuiteDependencies,
) -> BrowserSuiteReceipt:
	"""Keep H0 callers on the default unfiltered canonical journey."""
	selection = BrowserSuiteSelection(scenario, None, False)
	result = run_selection(selection, dependencies)
	return result


#============================================
def main(argv: Sequence[str] | None = None) -> None:
	"""Run a closed public selection through the shared production-stack owner."""
	arguments = sys.argv[1:] if argv is None else argv
	selection = parse_selection(arguments)
	dependencies = default_dependencies()
	run_selection(selection, dependencies)


#============================================
def command_line_main() -> None:
	"""Present closed-selection errors without allocating a stack or printing a traceback."""
	try:
		main()
	except BrowserSuiteError as error:
		print("ERROR: " + str(error), file=sys.stderr)
		raise SystemExit(2) from error


if __name__ == "__main__":
	command_line_main()

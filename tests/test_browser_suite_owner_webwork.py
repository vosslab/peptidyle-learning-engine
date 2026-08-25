"""Offline contracts for the private WebWork renderer hand-off."""

import dataclasses
import json
import pathlib
import sys

import pytest

import local_stack_control.models
import local_stack_control.private_state
import local_stack_control.process

E2E_DIRECTORY = pathlib.Path(__file__).resolve().parent / "e2e"
sys.path.insert(0, str(E2E_DIRECTORY))

import e2e_browser_scenario_contract as browser_scenario_contract
import e2e_browser_scenario_webwork_delivery as webwork_delivery
import e2e_browser_suite_oracles as browser_suite_oracles
import e2e_browser_suite_owner as browser_suite_owner


#============================================
def webwork_contract() -> browser_scenario_contract.ScenarioContract:
	"""Return the production WebWork journey used by these focused owner cases."""
	return browser_scenario_contract.require_contract(webwork_delivery.SCENARIO_ID)


#============================================
def installed_receipt() -> str:
	"""Return one completed Base Course receipt for the private temporary workspace."""
	generation = "00000000-0000-0000-0000-000000000006"
	storage = json.dumps(
		{
			"schemaVersion": 1,
			"baselineVersion": "base-course-v1",
			"installationGeneration": generation,
			"storageReceiptBucket": "private-content",
			"storageReceiptKey": "ple/live-demo/base-course-install-receipt.json",
			"objectManifest": [],
		},
		separators=(",", ":"),
	)
	return json.dumps(
		{
			"schemaVersion": 1,
			"action": "installed",
			"installState": "complete",
			"baselineVersion": "base-course-v1",
			"objectManifest": [],
			"installationGeneration": generation,
			"storageReceiptBucket": "private-content",
			"storageReceiptKey": "ple/live-demo/base-course-install-receipt.json",
			"storageReceiptJson": storage,
			"storageReceiptSha256": "a" * 64,
			"completionReceiptSha256": "b" * 64,
			"manifest": {
				"assignmentId": "a",
				"enrollmentId": "e",
				"questionId": "q",
				"problemId": "p",
				"versionId": "v",
			},
		},
		separators=(",", ":"),
	)


class OfflineRunner(local_stack_control.process.CommandRunner):
	"""Provide the runner protocol without starting a subprocess."""

	def run(
		self,
		argv: list[str],
		environment: dict[str, str] | None = None,
		cwd: pathlib.Path | None = None,
		stdin: str | None = None,
	) -> local_stack_control.models.CommandResult:
		return local_stack_control.models.CommandResult(tuple(argv), 0, "", "")

	def stream(
		self,
		argv: list[str],
		environment: dict[str, str] | None = None,
		cwd: pathlib.Path | None = None,
	) -> int:
		return 0


#============================================
def selections() -> dict[str, str]:
	"""Return the closed image and renderer selection shape used by the owner."""
	return {
		"PLE_WEBWORK_RENDERER_IMAGE": "renderer",
		"PLE_WEBWORK_RENDERER_BASE_URL": "http://renderer",
		"PLE_WEBWORK_RENDERER_ID": "renderer-id",
		"PLE_WEBWORK_REQUEST_TIMEOUT_SECONDS": "10",
		"PLE_WEBWORK_MAX_RESPONSE_BYTES": "1000",
		"PLE_GATEWAY_IMAGE_SHA256": "gateway",
		"PLE_POSTGRES_IMAGE_SHA256": "postgres",
		"PLE_MINIO_IMAGE_SHA256": "minio",
		"PLE_MINIO_MC_IMAGE_SHA256": "minio-mc",
		"PLE_SECRET_INIT_IMAGE_SHA256": "secret-init",
	}


#============================================
def offline_dependencies(
	tmp_path: pathlib.Path,
	*,
	webwork_seed_failure: bool = False,
	webwork_seed_receipt: str | None = None,
	webwork_evidence_windows: list[str] | None = None,
	produce_webwork_acknowledgement: bool = True,
	webwork_acknowledgement_content: str | None = None,
) -> tuple[
	browser_suite_owner.BrowserSuiteDependencies,
	list[list[str]],
	list[browser_suite_owner.BrowserSuiteReceipt],
]:
	"""Build the narrow temporary owner boundary exercised by this module."""
	commands: list[list[str]] = []
	receipts: list[browser_suite_owner.BrowserSuiteReceipt] = []

	def write_input(
		path: pathlib.Path,
		port: int,
		contract: browser_scenario_contract.ScenarioContract,
	) -> None:
		value: dict[str, object] = {
			"schemaVersion": 2,
			"scenarioId": contract.scenario_id,
			"namespace": f"bs1-0123456789ab-{contract.scenario_id}",
			"baseUrl": f"https://localhost:{port}/",
			"personas": list(contract.personas),
			"baselineReads": list(contract.baseline_reads),
			"visibleObservation": contract.visible_observation,
		}
		if contract.service_receipt is not None:
			value["serviceReceipt"] = contract.service_receipt
		if contract.fault_transition is not None:
			value["faultTransition"] = contract.fault_transition
		browser_suite_owner.private_file(
			path, json.dumps(value, separators=(",", ":"))
		)

	def run_command(
		runner: local_stack_control.process.CommandRunner,
		argv: list[str],
		root: pathlib.Path,
		environment: dict[str, str] | None,
	) -> local_stack_control.process.SessionCommandResult:
		commands.append(argv)
		if argv[0] == "npx":
			assert environment is not None
			input_path = pathlib.Path(
				environment["PLE_LIVE_DEMO_BROWSER_INPUT_FILE"]
			)
			input_value = json.loads(input_path.read_text(encoding="ascii"))
			if (
				produce_webwork_acknowledgement
				and input_value["scenarioId"] == webwork_delivery.SCENARIO_ID
			):
				acknowledgement_path = pathlib.Path(
					environment["PLE_WEBWORK_RENDERER_ISSUANCE_ACK_FILE"]
				)
				content = webwork_acknowledgement_content
				if content is None:
					content = json.dumps(
						{
							"event": "visible_question_issued",
							"namespace": input_value["namespace"],
							"scenarioId": webwork_delivery.SCENARIO_ID,
							"schemaVersion": 1,
						},
						separators=(",", ":"),
					)
				browser_suite_owner.private_file(acknowledgement_path, content)
			origin_path = pathlib.Path(
				environment["PLE_LIVE_DEMO_BROWSER_ORIGIN_RECEIPT_FILE"]
			)
			browser_suite_owner.private_file(
				origin_path,
				json.dumps(
					{
						"pageOrigins": ["https://localhost:55001"],
						"requestOrigins": ["https://localhost:55001"],
					},
					separators=(",", ":"),
				),
			)
		return local_stack_control.process.SessionCommandResult(
			local_stack_control.process.ProcessSession(-1, 1, "injected", ""), 0
		)

	def make_state(
		root: pathlib.Path, relative_root: pathlib.Path, prefix: str
	) -> local_stack_control.private_state.PrivateState:
		state = local_stack_control.private_state.prepare(root, relative_root, prefix)
		baseline = state.directory / ".runtime"
		baseline.mkdir()
		browser_suite_owner.private_file(
			baseline / "base-course.json", installed_receipt()
		)
		return state

	def read_inventory(
		project: str,
		directory: pathlib.Path,
		runner: local_stack_control.process.CommandRunner,
		root: pathlib.Path,
		provider: browser_suite_oracles.ProviderReceipt,
		sessions: tuple[local_stack_control.process.ProcessSession, ...],
	) -> browser_suite_oracles.SuiteInventory:
		return browser_suite_oracles.SuiteInventory(
			project,
			(),
			(),
			(),
			browser_suite_oracles.private_artifacts(directory),
			(),
			provider,
		)

	def read_provider(
		runner: local_stack_control.process.CommandRunner,
		root: pathlib.Path,
		manifest: pathlib.Path,
	) -> browser_suite_oracles.ProviderReceipt:
		return browser_suite_oracles.ProviderReceipt(
			"podman-compose", ("podman-compose", "--in-pod", "false"), False
		)

	def seed_webwork_catalog(
		runner: local_stack_control.process.CommandRunner,
		root: pathlib.Path,
		directory: pathlib.Path,
		minio_port: int,
	) -> webwork_delivery.CatalogBaseline:
		commands.append(browser_suite_owner.webwork_catalog_seed_argv(minio_port))
		if webwork_seed_failure:
			raise browser_suite_owner.BrowserSuiteError(
				"WebWork catalog baseline publication failed"
			)
		contents = webwork_seed_receipt
		if contents is None:
			contents = (
				'{"questionId":"ABC-1234","title":'
				'"Biochemistry: Identify hydrophobic compounds from formulas"}'
			)
		return webwork_delivery.decode_catalog_baseline_receipt(contents)

	evidence_windows = list(webwork_evidence_windows or ())

	def read_webwork_logs(
		runner: local_stack_control.process.CommandRunner,
		root: pathlib.Path,
		manifest: pathlib.Path,
	) -> str:
		if not evidence_windows:
			return ""
		return browser_suite_owner.redacted_renderer_evidence_logs(
			evidence_windows.pop(0)
		)

	dependencies = browser_suite_owner.BrowserSuiteDependencies(
		root=tmp_path,
		runner=OfflineRunner(),
		selections=selections(),
		ports=(53501, 54001, 54501, 55001),
		state_factory=make_state,
		input_writer=write_input,
		port_checker=lambda ports, runner, root: None,
		topology_validator=lambda runner, root, manifest: None,
		worker_readiness_checker=lambda runner, manifest, root: None,
		command_runner=run_command,
		provider_reader=read_provider,
		inventory_reader=read_inventory,
		origin_checker=browser_suite_oracles.origin_receipt_from_file,
		cleanup_checker=browser_suite_oracles.empty_after_cleanup,
		receipt_reporter=receipts.append,
		webwork_catalog_seeder=seed_webwork_catalog,
		evidence_log_reader=read_webwork_logs,
	)
	return dependencies, commands, receipts

#============================================
def test_webwork_delivery_seed_and_renderer_witness_are_private_and_scenario_scoped(
	tmp_path: pathlib.Path,
) -> None:
	"""The one catalog seed and content-free renderer receipt stay inside WebWork's child."""
	contract = webwork_contract()
	prior_event = 'INFO ple.webwork.cache event="renderer_call" request="private"\n'
	dependencies, commands, receipts = offline_dependencies(
		tmp_path,
		webwork_evidence_windows=[prior_event, prior_event + 'INFO ple.webwork.cache event="renderer_call" answer="private"\n'],
	)
	child_environments: list[dict[str, str]] = []
	original_command_runner = dependencies.command_runner

	def observe_child_environment(
		runner: local_stack_control.process.CommandRunner,
		argv: list[str],
		root: pathlib.Path,
		environment: dict[str, str] | None,
	) -> local_stack_control.process.SessionCommandResult:
		if argv[0] == "npx":
			assert environment is not None
			child_environments.append(environment)
		return original_command_runner(runner, argv, root, environment)

	dependencies = dataclasses.replace(dependencies, command_runner=observe_child_environment)
	receipt = browser_suite_owner.run_selected_scenario(contract.scenario_id, dependencies)
	assert [command[0] for command in commands] == [
		sys.executable,
		"cargo",
		"npx",
		sys.executable,
	]
	assert commands[1] == browser_suite_owner.webwork_catalog_seed_argv(54001)
	assert len(child_environments) == 1
	assert set(child_environments[0]).intersection({
		"PLE_WEBWORK_CATALOG_BASELINE_INPUT_FILE",
		"PLE_WEBWORK_RENDERER_ISSUANCE_ACK_FILE",
	}) == {
		"PLE_WEBWORK_CATALOG_BASELINE_INPUT_FILE",
		"PLE_WEBWORK_RENDERER_ISSUANCE_ACK_FILE",
	}
	witness = receipt.scenario_receipts[0].renderer_call_witness
	assert witness is not None and witness.event_count == 1
	public = receipt.as_json()
	assert '"rendererCallWitness"' in public
	assert "webwork-catalog-baseline-input.json" not in public
	assert "webwork-renderer-issuance-acknowledgement.json" not in public
	assert 'request="private"' not in public and 'answer="private"' not in public
	assert receipt.private_state_removed
	assert len(receipts) == 1


#============================================
def test_webwork_private_capabilities_do_not_reach_neighboring_scenarios(
	tmp_path: pathlib.Path, monkeypatch: pytest.MonkeyPatch,
) -> None:
	"""A complete serial suite passes the WebWork hand-off to its one selected child."""
	direct = browser_scenario_contract.require_contract("direct_role_entry")
	authorization = browser_scenario_contract.require_contract("auth_authorization")
	webwork = webwork_contract()
	monkeypatch.setattr(
		browser_scenario_contract,
		"scenario_contracts",
		lambda: (direct, authorization, webwork),
	)
	dependencies, _commands, _receipts = offline_dependencies(
		tmp_path,
		webwork_evidence_windows=["", 'ple.webwork.cache event="renderer_call"'],
	)
	child_environments: dict[str, dict[str, str]] = {}
	original_command_runner = dependencies.command_runner

	def observe_children(
		runner: local_stack_control.process.CommandRunner,
		argv: list[str],
		root: pathlib.Path,
		environment: dict[str, str] | None,
	) -> local_stack_control.process.SessionCommandResult:
		if argv[0] == "npx":
			assert environment is not None
			child_environments[pathlib.Path(argv[3]).stem.removesuffix(".spec")] = environment
		return original_command_runner(runner, argv, root, environment)

	browser_suite_owner.run_selection(
		browser_suite_owner.BrowserSuiteSelection(None, None, False),
		dataclasses.replace(dependencies, command_runner=observe_children),
	)
	for name in (
		"PLE_WEBWORK_CATALOG_BASELINE_INPUT_FILE",
		"PLE_WEBWORK_RENDERER_ISSUANCE_ACK_FILE",
	):
		assert name not in child_environments["direct_role_entry"]
		assert name not in child_environments["auth_authorization"]
		assert name in child_environments["webwork_delivery"]


#============================================
@pytest.mark.parametrize(
	"seed_failure,seed_receipt,error",
	[
		(True, None, "publication failed"),
		(False, "{}", "receipt is invalid"),
	],
)
def test_webwork_delivery_seed_stops_before_browser_on_failure(
	tmp_path: pathlib.Path,
	seed_failure: bool,
	seed_receipt: str | None,
	error: str,
) -> None:
	"""Seed command and its two-field receipt fail closed before Chromium starts."""
	contract = webwork_contract()
	dependencies, commands, receipts = offline_dependencies(
		tmp_path,
		webwork_seed_failure=seed_failure,
		webwork_seed_receipt=seed_receipt,
	)
	with pytest.raises(browser_suite_owner.BrowserSuiteError, match=error):
		browser_suite_owner.run_selected_scenario(contract.scenario_id, dependencies)
	assert [item[0] for item in commands] == [sys.executable, "cargo", sys.executable]
	assert not [item for item in commands if item[0] == "npx"]
	assert receipts[0].cleanup_completed and receipts[0].private_state_removed


#============================================
@pytest.mark.parametrize(
	"produce_acknowledgement,acknowledgement,error",
	[
		(False, None, "acknowledgement is invalid"),
		(True, "{}", "acknowledgement is invalid"),
	],
)
def test_webwork_delivery_requires_a_namespace_bound_visible_issuance_acknowledgement(
	tmp_path: pathlib.Path,
	produce_acknowledgement: bool,
	acknowledgement: str | None,
	error: str,
) -> None:
	"""Renderer evidence follows a successful UI-issued acknowledgement only."""
	contract = webwork_contract()
	dependencies, commands, receipts = offline_dependencies(
		tmp_path,
		produce_webwork_acknowledgement=produce_acknowledgement,
		webwork_acknowledgement_content=acknowledgement,
		webwork_evidence_windows=["", 'ple.webwork.cache event="renderer_call"'],
	)
	with pytest.raises(browser_suite_owner.BrowserSuiteError, match=error):
		browser_suite_owner.run_selected_scenario(contract.scenario_id, dependencies)
	assert [item[0] for item in commands if item[0] == "npx"] == ["npx"]
	assert receipts[0].scenario_receipts[0].child_succeeded is False
	assert receipts[0].cleanup_completed and receipts[0].private_state_removed


#============================================
@pytest.mark.parametrize(
	"after_logs",
	[
		"",
		'ple.webwork.cache event="renderer_call"\nple.webwork.cache event="renderer_call"\nple.webwork.cache event="renderer_call"',
	],
)
def test_webwork_delivery_renderer_witness_requires_exactly_one_new_event(
	tmp_path: pathlib.Path, after_logs: str,
) -> None:
	"""A zero or duplicate renderer receipt cannot be credited to a visible journey."""
	contract = webwork_contract()
	dependencies, _commands, receipts = offline_dependencies(
		tmp_path,
		webwork_evidence_windows=['ple.webwork.cache event="renderer_call"', after_logs],
	)
	with pytest.raises(browser_suite_owner.BrowserSuiteError, match="one renderer"):
		browser_suite_owner.run_selected_scenario(contract.scenario_id, dependencies)
	assert receipts[0].scenario_receipts[0].child_succeeded is False
	assert receipts[0].cleanup_completed and receipts[0].private_state_removed


#============================================
def test_renderer_log_projection_discards_unrelated_and_sensitive_content() -> None:
	"""The owner keeps only a marker count, never raw API evidence-log fields."""
	contents = (
		'route=/api/answers source=private provider=renderer answer=secret\n'
		'INFO ple.webwork.cache event="renderer_call" request=/api/answers source=private\n'
	)
	assert browser_suite_owner.redacted_renderer_evidence_logs(contents) == (
		'ple.webwork.cache event="renderer_call"'
	)


#============================================
@pytest.mark.parametrize(
	"returncode,stdout,error",
	[
		(1, "", "publication failed"),
		(0, "{}", "receipt is invalid"),
	],
)
def test_default_webwork_seed_command_fails_closed_without_echoing_private_capabilities(
	tmp_path: pathlib.Path,
	returncode: int,
	stdout: str,
	error: str,
) -> None:
	"""The real Cargo boundary accepts only a successful two-field public receipt."""
	secret = tmp_path / "question-secret"
	browser_suite_owner.private_file(secret, "A" * 43)
	browser_suite_owner.private_file(
		tmp_path / "env.local",
		"POSTGRES_USER=owner\nPOSTGRES_PASSWORD=private-password\nPOSTGRES_DB=owner\n"
		"PLE_POSTGRES_HOST_PORT=54001\n"
		f"PLE_QUESTION_ID_SECRET_HOST_FILE={secret}\n"
		"MINIO_ROOT_USER=owner\nMINIO_ROOT_PASSWORD=private-object-password\n",
	)
	commands: list[tuple[list[str], dict[str, str] | None]] = []

	class FailingSeedRunner(local_stack_control.process.CommandRunner):
		"""Capture the host boundary without starting Cargo."""

		def run(
			self,
			argv: list[str],
			environment: dict[str, str] | None = None,
			cwd: pathlib.Path | None = None,
			stdin: str | None = None,
		) -> local_stack_control.models.CommandResult:
			commands.append((argv, environment))
			return local_stack_control.models.CommandResult(tuple(argv), returncode, stdout, "")

		def stream(
			self,
			argv: list[str],
			environment: dict[str, str] | None = None,
			cwd: pathlib.Path | None = None,
		) -> int:
			return 0

	with pytest.raises(browser_suite_owner.BrowserSuiteError, match=error):
		browser_suite_owner.seed_webwork_catalog_baseline(FailingSeedRunner(), tmp_path, tmp_path, 54001)
	assert commands[0][0] == browser_suite_owner.webwork_catalog_seed_argv(54001)
	assert "private-password" not in " ".join(commands[0][0])
	assert commands[0][1] is not None
	assert commands[0][1]["AWS_SECRET_ACCESS_KEY"] == "private-object-password"


#============================================
def test_default_renderer_log_reader_uses_the_label_resolved_redacted_adapter(
	tmp_path: pathlib.Path,
) -> None:
	"""Log collection captures a bounded adapter result and exposes only event markers."""
	manifest = tmp_path / "disposable.manifest"
	browser_suite_owner.private_file(manifest, "OWNER=live-demo-browser\n")
	commands: list[list[str]] = []

	class EvidenceRunner(local_stack_control.process.CommandRunner):
		"""Return a deliberately sensitive API log without subprocess execution."""

		def run(
			self,
			argv: list[str],
			environment: dict[str, str] | None = None,
			cwd: pathlib.Path | None = None,
			stdin: str | None = None,
		) -> local_stack_control.models.CommandResult:
			commands.append(argv)
			return local_stack_control.models.CommandResult(
				tuple(argv), 0,
				'route=/api/answers ple.webwork.cache event="renderer_call" source=private\n',
				"",
			)

		def stream(
			self,
			argv: list[str],
			environment: dict[str, str] | None = None,
			cwd: pathlib.Path | None = None,
		) -> int:
			return 0

	assert browser_suite_owner.read_webwork_renderer_evidence_logs(
		EvidenceRunner(), tmp_path, manifest
	) == 'ple.webwork.cache event="renderer_call"'
	assert commands[0][3] == "read-evidence-logs"

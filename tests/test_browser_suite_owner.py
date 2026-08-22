"""Offline contracts for the shared disposable production browser-suite owner."""

import dataclasses
import json
import pathlib
import sys

import pytest

import file_utils
import local_stack_control.models
import local_stack_control.private_state
import local_stack_control.process

E2E_DIRECTORY = pathlib.Path(file_utils.get_repo_root()) / "tests" / "e2e"
sys.path.insert(0, str(E2E_DIRECTORY))

import e2e_browser_suite_owner as browser_suite_owner
import e2e_browser_suite_oracles as browser_suite_oracles
import e2e_browser_scenario_contract as browser_scenario_contract
import e2e_browser_scenario_webwork_delivery as webwork_delivery

TEST_SCENARIO = "sysadmin_first_claim"
TEST_SPEC_PATH = "tests/playwright/e2e/sysadmin_first_claim.spec.ts"


def scenario_contract() -> browser_scenario_contract.ScenarioContract:
	"""Build the canonical first-claim journey as an explicit local fixture."""
	return browser_scenario_contract.ScenarioContract(
		scenario_id=TEST_SCENARIO,
		spec_path=TEST_SPEC_PATH,
		personas=("morgan_sysadmin",),
		baseline_reads=("genetics_practice_course",),
		ui_creates=("passkey",),
		sysadmin_requirement="unclaimed",
		visible_observation="sysadmin_passkey_reauthentication",
		exclusive_seed_mutations=("sysadmin_first_claim",),
	)


def webwork_contract() -> browser_scenario_contract.ScenarioContract:
	"""Build the catalog-only renderer journey for injected owner integration tests."""
	return browser_scenario_contract.ScenarioContract(
		scenario_id=webwork_delivery.SCENARIO_ID,
		spec_path="tests/playwright/e2e/webwork_delivery.spec.ts",
		personas=("elena_instructor", "mary_student"),
		baseline_reads=("base_course",),
		ui_creates=("course", "assignment", "invitation", "response"),
		sysadmin_requirement="not_required",
		visible_observation="visible_webwork_completion_persists_in_a_fresh_session",
		service_receipt="renderer_delivery",
	)


def patch_local_catalog(monkeypatch: pytest.MonkeyPatch) -> None:
	"""Install the one contract needed by tests that exercise the owner front door."""
	contract = scenario_contract()
	monkeypatch.setattr(
		browser_scenario_contract,
		"scenario_contracts",
		lambda: (contract,),
	)


def installed_receipt() -> str:
	"""Return the canonical completed lifecycle receipt needed by the H2 owner."""
	generation = "00000000-0000-0000-0000-000000000006"
	storage = json.dumps({"schemaVersion": 1, "baselineVersion": "base-course-v1", "installationGeneration": generation, "storageReceiptBucket": "private-content", "storageReceiptKey": "ple/live-demo/base-course-install-receipt.json", "objectManifest": []}, separators=(",", ":"))
	return json.dumps({"schemaVersion": 1, "action": "installed", "installState": "complete", "baselineVersion": "base-course-v1", "objectManifest": [], "installationGeneration": generation, "storageReceiptBucket": "private-content", "storageReceiptKey": "ple/live-demo/base-course-install-receipt.json", "storageReceiptJson": storage, "storageReceiptSha256": "a" * 64, "manifest": {"assignmentId": "a", "enrollmentId": "e", "questionId": "q", "problemId": "p", "versionId": "v"}}, separators=(",", ":"))


#============================================
def webauthn_continuation_contents(gateway_port: int = 55001) -> str:
	"""Return one canonical private continuation emitted by the visible setup child."""
	value = {
		"version": 1,
		"origin": f"https://localhost:{gateway_port}",
		"rpId": "localhost",
		"credentials": [{
			"credentialId": "AA",
			"isResidentCredential": True,
			"rpId": "localhost",
			"privateKey": "AQI",
			"signCount": 0,
			"userHandle": "Aw",
			"backupEligibility": False,
			"backupState": False,
		}],
	}
	return json.dumps(value, separators=(",", ":"))


#============================================
def webauthn_acknowledgement_contents(
	scenario_id: str,
	namespace: str,
	gateway_port: int = 55001,
) -> str:
	"""Return one canonical private acknowledgement after visible passkey entry."""
	value = {
		"event": "visible_sysadmin_passkey_sign_in",
		"namespace": namespace,
		"origin": f"https://localhost:{gateway_port}",
		"scenarioId": scenario_id,
		"schemaVersion": 1,
	}
	return json.dumps(value, separators=(",", ":"))


class OfflineRunner(local_stack_control.process.CommandRunner):
	"""Provide the abstract runner boundary without starting a subprocess."""

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
	"""Return the small complete selection shape needed to form private state."""
	result = {
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
	return result


#============================================
def offline_dependencies(
	tmp_path: pathlib.Path,
	child_failure: bool = False,
	child_failure_scenario: str | None = None,
	launch_failure: bool = False,
	cleanup_failure: bool = False,
	invalid_input: bool = False,
	removal_failure: bool = False,
	reporter_failure: bool = False,
	origin_mismatch: bool = False,
	produce_webauthn_continuation: bool = True,
	produce_webauthn_acknowledgement: bool = True,
	webauthn_acknowledgement_content: str | None = None,
	selection_values: dict[str, str] | None = None,
	webwork_seed_failure: bool = False,
	webwork_seed_receipt: str | None = None,
	webwork_evidence_windows: list[str] | None = None,
	produce_webwork_acknowledgement: bool = True,
	webwork_acknowledgement_content: str | None = None,
) -> tuple[browser_suite_owner.BrowserSuiteDependencies, list[list[str]], list[browser_suite_owner.BrowserSuiteReceipt]]:
	"""Build an injectable lifecycle that records commands and receipt behavior."""
	commands: list[list[str]] = []
	receipts: list[browser_suite_owner.BrowserSuiteReceipt] = []

	def write_input(
		path: pathlib.Path,
		port: int,
		claim_context_path: pathlib.Path,
		contract: browser_scenario_contract.ScenarioContract,
	) -> None:
		value: dict[str, object] = {
			"schemaVersion": 2,
			"scenarioId": contract.scenario_id,
			"namespace": f"bs1-0123456789ab-{contract.scenario_id}",
			"baseUrl": f"https://localhost:{port}/",
			"personas": list(contract.personas),
			"baselineReads": list(contract.baseline_reads),
			"sysadminRequirement": contract.sysadmin_requirement,
			"visibleObservation": contract.visible_observation,
		}
		if contract.sysadmin_requirement == "unclaimed":
			value["sysadminOwnershipProof"] = "A" * 43
		if contract.service_receipt is not None:
			value["serviceReceipt"] = contract.service_receipt
		if contract.fault_transition is not None:
			value["faultTransition"] = contract.fault_transition
		content = "not-json" if invalid_input else json.dumps(value, separators=(",", ":"))
		browser_suite_owner.private_file(path, content)

	def run_command(
		runner: local_stack_control.process.CommandRunner,
		argv: list[str],
		root: pathlib.Path,
		environment: dict[str, str] | None,
	) -> local_stack_control.process.SessionCommandResult:
		commands.append(argv)
		if launch_failure and argv[3] == "launch":
			return local_stack_control.process.SessionCommandResult(
				local_stack_control.process.ProcessSession(7101, 1, "injected", ""), 1
			)
		is_failed_child = child_failure and argv[0] == "npx"
		is_selected_failed_child = (
			child_failure_scenario is not None
			and argv[0] == "npx"
			and pathlib.Path(argv[3]).name
			== child_failure_scenario + ".spec.ts"
		)
		if is_failed_child or is_selected_failed_child:
			return local_stack_control.process.SessionCommandResult(
				local_stack_control.process.ProcessSession(7102, 1, "injected", ""), 1
			)
		if argv[0] == "npx":
			assert environment is not None
			input_path = pathlib.Path(environment["PLE_LIVE_DEMO_BROWSER_INPUT_FILE"])
			input_value = json.loads(input_path.read_text(encoding="ascii"))
			if (
				produce_webauthn_continuation
				and input_value["sysadminRequirement"] == "unclaimed"
			):
				continuation_path = pathlib.Path(
					environment["PLE_BROWSER_SUITE_WEBAUTHN_CONTINUATION_FILE"]
				)
				browser_suite_owner.private_file(
					continuation_path,
					webauthn_continuation_contents(gateway_port=55001),
				)
			if (
				produce_webauthn_acknowledgement
				and input_value["sysadminRequirement"] == "claimed"
			):
				acknowledgement_path = pathlib.Path(
					environment["PLE_BROWSER_SUITE_WEBAUTHN_CONTINUATION_ACK_FILE"]
				)
				content = webauthn_acknowledgement_content
				if content is None:
					content = webauthn_acknowledgement_contents(
						str(input_value["scenarioId"]),
						str(input_value["namespace"]),
					)
				browser_suite_owner.private_file(acknowledgement_path, content)
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
			receipt_path = pathlib.Path(environment["PLE_LIVE_DEMO_BROWSER_ORIGIN_RECEIPT_FILE"])
			browser_suite_owner.private_file(
				receipt_path,
				json.dumps({"pageOrigins": ["https://localhost:55001"], "requestOrigins": ["https://example.test" if origin_mismatch else "https://localhost:55001"]}, separators=(",", ":")),
			)
		if cleanup_failure and argv[3] == "cleanup":
			return local_stack_control.process.SessionCommandResult(
				local_stack_control.process.ProcessSession(7103, 1, "injected", ""), 1
			)
		return local_stack_control.process.SessionCommandResult(
			local_stack_control.process.ProcessSession(-1, 1, "injected", ""), 0
		)

	class StateWithRemovalFailure:
		"""Expose the required private-state surface while injecting removal behavior."""

		def __init__(self, state: local_stack_control.private_state.PrivateState) -> None:
			self.directory = state.directory
			self._state = state

		def remove(self) -> None:
			if removal_failure:
				raise browser_suite_owner.BrowserSuiteError("injected private-state removal failure")
			self._state.remove()

	def make_state(root: pathlib.Path, relative_root: pathlib.Path, prefix: str) -> StateWithRemovalFailure:
		state = local_stack_control.private_state.prepare(root, relative_root, prefix)
		baseline = state.directory / ".runtime"
		baseline.mkdir()
		browser_suite_owner.private_file(
			baseline / "base-course.json",
			installed_receipt(),
		)
		result = StateWithRemovalFailure(state)
		return result

	def report(receipt: browser_suite_owner.BrowserSuiteReceipt) -> None:
		receipts.append(receipt)
		if reporter_failure:
			raise browser_suite_owner.BrowserSuiteError("injected receipt reporter failure")

	def read_inventory(
		project: str,
		directory: pathlib.Path,
		runner: local_stack_control.process.CommandRunner,
		root: pathlib.Path,
		provider: browser_suite_oracles.ProviderReceipt,
		sessions: tuple[local_stack_control.process.ProcessSession, ...],
	) -> browser_suite_oracles.SuiteInventory:
		return browser_suite_oracles.SuiteInventory(
			project, (), (), (), browser_suite_oracles.private_artifacts(directory), (), provider
		)

	def read_provider(
		runner: local_stack_control.process.CommandRunner,
		root: pathlib.Path,
		manifest: pathlib.Path,
	) -> browser_suite_oracles.ProviderReceipt:
		return browser_suite_oracles.ProviderReceipt("podman-compose", ("podman-compose", "--in-pod", "false"), False)

	def seed_webwork_catalog(
		runner: local_stack_control.process.CommandRunner,
		root: pathlib.Path,
		directory: pathlib.Path,
		minio_port: int,
	) -> webwork_delivery.CatalogBaseline:
		commands.append(browser_suite_owner.webwork_catalog_seed_argv(minio_port))
		if webwork_seed_failure:
			raise browser_suite_owner.BrowserSuiteError("WebWork catalog baseline publication failed")
		contents = webwork_seed_receipt
		if contents is None:
			contents = '{"questionId":"ABC-1234","title":"Biochemistry: Identify hydrophobic compounds from formulas"}'
		return webwork_delivery.decode_catalog_baseline_receipt(contents)

	evidence_windows = list(webwork_evidence_windows or ())

	def read_webwork_logs(
		runner: local_stack_control.process.CommandRunner,
		root: pathlib.Path,
		manifest: pathlib.Path,
	) -> str:
		if not evidence_windows:
			return ""
		return browser_suite_owner.redacted_renderer_evidence_logs(evidence_windows.pop(0))

	dependencies = browser_suite_owner.BrowserSuiteDependencies(
		root=tmp_path,
		runner=OfflineRunner(),
		selections=selections() if selection_values is None else selection_values,
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
		receipt_reporter=report,
		webwork_catalog_seeder=seed_webwork_catalog,
		evidence_log_reader=read_webwork_logs,
	)
	result = dependencies, commands, receipts
	return result


#============================================
def test_invalid_selection_stops_before_owner_resources_exist(tmp_path: pathlib.Path) -> None:
	"""A closed scenario selector declines an unsupported suite before lifecycle setup."""
	dependencies, commands, receipts = offline_dependencies(tmp_path)
	with pytest.raises(browser_suite_owner.BrowserSuiteError, match="unsupported"):
		browser_suite_owner.run_selected_scenario("not-a-scenario", dependencies)
	assert not commands and not receipts and not (tmp_path / "target").exists()


#============================================
def test_direct_malformed_selection_stops_before_ports_state_commands_and_receipts(tmp_path: pathlib.Path) -> None:
	"""Non-CLI callers receive the same preallocation selection boundary."""
	dependencies, commands, receipts = offline_dependencies(tmp_path)
	malformed = browser_suite_owner.BrowserSuiteSelection(TEST_SCENARIO, None, "yes")  # type: ignore[arg-type]
	with pytest.raises(browser_suite_owner.BrowserSuiteError, match="selection"):
		browser_suite_owner.run_selection(malformed, dependencies)
	assert not commands and not receipts and not (tmp_path / "target").exists()


#============================================
def test_front_door_selection_translates_only_the_default_named_and_approved_file_forms() -> None:
	"""The public runner chooses the sole H1 journey without forwarding raw Playwright argv."""
	default_selection = browser_suite_owner.parse_selection(())
	named_selection = browser_suite_owner.parse_selection(("--scenario", TEST_SCENARIO))
	file_selection = browser_suite_owner.parse_selection((TEST_SPEC_PATH,))
	assert default_selection.scenario is None
	assert named_selection.scenario == TEST_SCENARIO and file_selection.spec_path == TEST_SPEC_PATH
	assert browser_suite_owner.playwright_argv(named_selection) == [
		"npx", "playwright", "test", TEST_SPEC_PATH, "--workers=1"
	]
	assert browser_suite_owner.parse_selection(("--build",)).build_requested


#============================================
def test_front_door_title_filter_uses_a_literal_grep_within_the_approved_scenario() -> None:
	"""A title substring remains a fixed child argv addition after closed parsing."""
	selection = browser_suite_owner.parse_selection(
		("--scenario", TEST_SCENARIO, "--grep", "first claim")
	)
	assert selection.title_filter == "first claim"
	assert browser_suite_owner.playwright_argv(selection)[-2:] == [
		"--grep", "first\\ claim"
	]


#============================================
def test_execution_order_runs_visible_first_claim_before_claimed_target() -> None:
	"""The owner sequencing rule does not depend on catalog order."""
	first_claim = scenario_contract()
	claimed = dataclasses.replace(
		first_claim,
		scenario_id="claimed_target",
		spec_path="tests/playwright/e2e/claimed_target.spec.ts",
		ui_creates=("question", "course"),
		sysadmin_requirement="claimed",
		exclusive_seed_mutations=(),
	)
	not_required = dataclasses.replace(
		first_claim,
		scenario_id="ordinary_target",
		spec_path="tests/playwright/e2e/ordinary_target.spec.ts",
		sysadmin_requirement="not_required",
		exclusive_seed_mutations=(),
		personas=("elena_instructor",),
		ui_creates=("course",),
	)
	assert [item.scenario_id for item in browser_suite_owner.ordered_execution_contracts(
		(claimed, first_claim, not_required), (claimed, first_claim, not_required),
	)] == [TEST_SCENARIO, "claimed_target", "ordinary_target"]
	assert browser_suite_owner.ordered_execution_contracts((not_required,), (first_claim, not_required)) == (not_required,)
	assert browser_suite_owner.ordered_execution_contracts((first_claim,), (first_claim,)) == (first_claim,)


#============================================
def test_multi_contract_owner_uses_one_lifecycle_and_isolates_child_inputs(
	tmp_path: pathlib.Path, monkeypatch: pytest.MonkeyPatch,
) -> None:
	"""Adversarial catalog order still gives each real child a private V2 projection."""
	base = scenario_contract()
	ordinary = dataclasses.replace(
		base, scenario_id="ordinary", spec_path="tests/playwright/e2e/ordinary.spec.ts",
		personas=("elena_instructor",), baseline_reads=("base_course",),
		ui_creates=("course",), sysadmin_requirement="not_required", exclusive_seed_mutations=(),
	)
	claimed = dataclasses.replace(
		base, scenario_id="claimed", spec_path="tests/playwright/e2e/claimed.spec.ts",
		ui_creates=("question", "course"),
		sysadmin_requirement="claimed", exclusive_seed_mutations=(),
	)
	unclaimed = dataclasses.replace(
		base, scenario_id="unclaimed", spec_path="tests/playwright/e2e/unclaimed.spec.ts",
		exclusive_seed_mutations=("sysadmin_first_claim", "avery_instructor_approval"),
	)
	catalog = (ordinary, claimed, unclaimed)
	monkeypatch.setattr(browser_scenario_contract, "scenario_contracts", lambda: catalog)
	dependencies, commands, receipts = offline_dependencies(tmp_path)
	inputs: list[dict[str, object]] = []

	def write_input(path: pathlib.Path, port: int, _: pathlib.Path, contract: browser_scenario_contract.ScenarioContract) -> None:
		value: dict[str, object] = {
			"schemaVersion": 2, "scenarioId": contract.scenario_id,
			"namespace": f"bs1-0123456789ab-{contract.scenario_id}",
			"baseUrl": f"https://localhost:{port}/", "personas": list(contract.personas),
			"baselineReads": list(contract.baseline_reads),
			"sysadminRequirement": contract.sysadmin_requirement,
			"visibleObservation": contract.visible_observation,
		}
		if contract.sysadmin_requirement == "unclaimed":
			value["sysadminOwnershipProof"] = "A" * 42 + "E"
		browser_suite_owner.private_file(path, json.dumps(value, separators=(",", ":")))
		inputs.append(value)

	dependencies = dataclasses.replace(dependencies, input_writer=write_input)
	receipt = browser_suite_owner.run_selection(browser_suite_owner.BrowserSuiteSelection(None, None, False), dependencies)
	child_commands = [item for item in commands if item[0] == "npx"]
	assert [item[3] for item in child_commands] == [ordinary.spec_path, unclaimed.spec_path, claimed.spec_path]
	assert [item["scenarioId"] for item in inputs] == ["ordinary", "unclaimed", "claimed"]
	assert [item["sysadminOwnershipProof"] if "sysadminOwnershipProof" in item else None for item in inputs] == [None, "A" * 42 + "E", None]
	assert [item.scenario_id for item in receipt.scenario_receipts] == ["ordinary", "unclaimed", "claimed"]
	assert all(item.child_succeeded for item in receipt.scenario_receipts)
	assert len({item.namespace for item in receipt.scenario_receipts}) == 3
	assert [item[3] for item in commands if len(item) > 3 and item[3] == "launch"] == ["launch"]
	assert [item[3] for item in commands if len(item) > 3 and item[3] == "cleanup"] == ["cleanup"]


#============================================
def test_multi_contract_failure_retains_ordered_receipts_and_scoped_cleanup(
	tmp_path: pathlib.Path,
	monkeypatch: pytest.MonkeyPatch,
) -> None:
	"""A later failed child preserves its successful prefix and ends the shared lifecycle."""
	base = scenario_contract()
	ordinary = dataclasses.replace(
		base,
		scenario_id="ordinary",
		spec_path="tests/playwright/e2e/ordinary.spec.ts",
		personas=("elena_instructor",),
		baseline_reads=("base_course",),
		ui_creates=("course",),
		sysadmin_requirement="not_required",
		exclusive_seed_mutations=(),
	)
	claimed = dataclasses.replace(
		base,
		scenario_id="claimed",
		spec_path="tests/playwright/e2e/claimed.spec.ts",
		ui_creates=("question", "course"),
		sysadmin_requirement="claimed",
		exclusive_seed_mutations=(),
	)
	catalog = (ordinary, base, claimed)
	monkeypatch.setattr(browser_scenario_contract, "scenario_contracts", lambda: catalog)
	dependencies, commands, receipts = offline_dependencies(
		tmp_path,
		child_failure_scenario=TEST_SCENARIO,
	)
	with pytest.raises(
		browser_suite_owner.BrowserSuiteError,
		match=f"browser scenario failed: {TEST_SCENARIO}",
	):
		browser_suite_owner.run_selection(
			browser_suite_owner.BrowserSuiteSelection(None, None, False),
			dependencies,
		)
	assert len(receipts) == 1
	receipt = receipts[0]
	assert [item.scenario_id for item in receipt.scenario_receipts] == [
		"ordinary",
		TEST_SCENARIO,
	]
	assert [item.child_succeeded for item in receipt.scenario_receipts] == [True, False]
	assert "claimed" not in [item.scenario_id for item in receipt.scenario_receipts]
	assert receipt.lifecycle_launch_attempted
	assert receipt.lifecycle_launch_completed
	assert receipt.cleanup_attempted
	assert receipt.cleanup_completed
	assert receipt.private_state_removed
	assert [item[3] for item in commands if len(item) > 3 and item[3] == "launch"] == [
		"launch"
	]
	assert [item[3] for item in commands if len(item) > 3 and item[3] == "cleanup"] == [
		"cleanup"
	]


#============================================
def test_focused_requirement_transitions_use_only_required_visible_children(
	tmp_path: pathlib.Path, monkeypatch: pytest.MonkeyPatch,
) -> None:
	"""Focused targets retain fresh-stack setup semantics without ambient fixtures."""
	base = scenario_contract()
	ordinary = dataclasses.replace(base, scenario_id="ordinary", spec_path="tests/playwright/e2e/ordinary.spec.ts", personas=("elena_instructor",), baseline_reads=("base_course",), ui_creates=("course",), sysadmin_requirement="not_required", exclusive_seed_mutations=())
	claimed = dataclasses.replace(base, scenario_id="claimed", spec_path="tests/playwright/e2e/claimed.spec.ts", ui_creates=("question", "course"), sysadmin_requirement="claimed", exclusive_seed_mutations=())
	unclaimed = dataclasses.replace(base, scenario_id="unclaimed", spec_path="tests/playwright/e2e/unclaimed.spec.ts")
	catalog = (ordinary, claimed, unclaimed)
	monkeypatch.setattr(browser_scenario_contract, "scenario_contracts", lambda: catalog)
	for scenario, expected in (("ordinary", [ordinary.spec_path]), ("unclaimed", [unclaimed.spec_path]), ("claimed", [unclaimed.spec_path, claimed.spec_path])):
		scenario_root = tmp_path / scenario
		scenario_root.mkdir()
		dependencies, commands, _ = offline_dependencies(scenario_root)
		def write_input(path: pathlib.Path, port: int, _: pathlib.Path, contract: browser_scenario_contract.ScenarioContract) -> None:
			value: dict[str, object] = {"schemaVersion": 2, "scenarioId": contract.scenario_id, "namespace": f"bs1-0123456789ab-{contract.scenario_id}", "baseUrl": f"https://localhost:{port}/", "personas": list(contract.personas), "baselineReads": list(contract.baseline_reads), "sysadminRequirement": contract.sysadmin_requirement, "visibleObservation": contract.visible_observation}
			if contract.sysadmin_requirement == "unclaimed": value["sysadminOwnershipProof"] = "A" * 42 + "E"
			browser_suite_owner.private_file(path, json.dumps(value, separators=(",", ":")))
		dependencies = dataclasses.replace(dependencies, input_writer=write_input)
		browser_suite_owner.run_selection(browser_suite_owner.BrowserSuiteSelection(scenario, None, False), dependencies)
		assert [item[3] for item in commands if item[0] == "npx"] == expected


#============================================
@pytest.mark.parametrize(
	"arguments",
	[
		("--scenario", "other"),
		("tests/playwright/smoke.spec.ts",),
		("--project", "chromium"),
		("--grep", "live.*"),
		("--config", "other.config.ts"),
	],
)
def test_front_door_rejects_redirecting_or_unsafe_selection_before_allocation(
	arguments: tuple[str, ...],
) -> None:
	"""The public parser leaves target, credentials, and Playwright configuration to the owner."""
	with pytest.raises((browser_suite_owner.BrowserSuiteError, SystemExit)):
		browser_suite_owner.parse_selection(arguments)


#============================================
@pytest.mark.parametrize(
	("selection_values", "message"),
	[
		({key: value for key, value in selections().items() if key != "PLE_GATEWAY_IMAGE_SHA256"}, "omit"),
		({**selections(), "PLE_WEBWORK_RENDERER_IMAGE": "unsafe\nvalue"}, "unsafe"),
	],
)
def test_invalid_canonical_selections_stop_before_owner_resources_exist(
	tmp_path: pathlib.Path,
	monkeypatch: pytest.MonkeyPatch,
	selection_values: dict[str, str],
	message: str,
) -> None:
	"""Malformed canonical selections decline before ports, state, or receipts exist."""
	patch_local_catalog(monkeypatch)
	dependencies, commands, receipts = offline_dependencies(tmp_path, selection_values=selection_values)
	with pytest.raises(browser_suite_owner.BrowserSuiteError, match=message):
		browser_suite_owner.run_selected_scenario(TEST_SCENARIO, dependencies)
	assert not commands and not receipts and not (tmp_path / "target").exists()


#============================================
def test_provider_mismatch_stops_before_lifecycle_launch_and_keeps_private_cleanup(
	tmp_path: pathlib.Path, monkeypatch: pytest.MonkeyPatch
) -> None:
	"""The receipt policy binds to the lifecycle provider rather than a no-pod constant."""
	patch_local_catalog(monkeypatch)
	dependencies, commands, receipts = offline_dependencies(tmp_path)

	def mismatched_provider(
		runner: local_stack_control.process.CommandRunner,
		root: pathlib.Path,
		manifest: pathlib.Path,
	) -> browser_suite_oracles.ProviderReceipt:
		raise browser_suite_oracles.BrowserSuiteOracleError("browser-suite lifecycle provider does not prove the no-pod policy")

	mismatched = dataclasses.replace(dependencies, provider_reader=mismatched_provider)
	with pytest.raises(browser_suite_oracles.BrowserSuiteOracleError, match="no-pod"):
		browser_suite_owner.run_selected_scenario(TEST_SCENARIO, mismatched)
	assert not commands and receipts[0].private_state_removed


#============================================
def test_success_receipt_records_https_origin_and_scoped_cleanup(
	tmp_path: pathlib.Path, monkeypatch: pytest.MonkeyPatch
) -> None:
	"""A finished child produces a non-secret receipt after its owned cleanup."""
	patch_local_catalog(monkeypatch)
	dependencies, commands, receipts = offline_dependencies(tmp_path)
	receipt = browser_suite_owner.run_selected_scenario(TEST_SCENARIO, dependencies)
	assert receipt.origin == "https://localhost:55001/" and receipt.lifecycle_launch_attempted and receipt.lifecycle_launch_completed and receipt.cleanup_completed and not pathlib.Path(receipt.private_state_directory).exists()
	assert commands[-1][3] == "cleanup" and receipts == [receipt]


#============================================
def test_child_failure_still_runs_scoped_cleanup_and_emits_receipt(
	tmp_path: pathlib.Path, monkeypatch: pytest.MonkeyPatch
) -> None:
	"""A Playwright child error retains the original failure after lifecycle cleanup."""
	patch_local_catalog(monkeypatch)
	dependencies, commands, receipts = offline_dependencies(tmp_path, child_failure=True)
	with pytest.raises(browser_suite_owner.BrowserSuiteError, match="command failed: npx"):
		browser_suite_owner.run_selected_scenario(TEST_SCENARIO, dependencies)
	assert commands[-1][3] == "cleanup" and receipts[0].private_state_removed


#============================================
def test_cleanup_failure_retains_private_state_and_emits_typed_receipt(
	tmp_path: pathlib.Path, monkeypatch: pytest.MonkeyPatch
) -> None:
	"""A typed cleanup error keeps private diagnostics and replaces no error detail."""
	patch_local_catalog(monkeypatch)
	dependencies, commands, receipts = offline_dependencies(tmp_path, cleanup_failure=True)
	with pytest.raises(browser_suite_owner.BrowserSuiteError, match="command failed"):
		browser_suite_owner.run_selected_scenario(TEST_SCENARIO, dependencies)
	assert commands[-1][3] == "cleanup" and not receipts[0].cleanup_completed and not receipts[0].private_state_removed
	assert pathlib.Path(receipts[0].private_state_directory).is_dir()


#============================================
def test_child_and_cleanup_failures_surface_together(
	tmp_path: pathlib.Path, monkeypatch: pytest.MonkeyPatch
) -> None:
	"""A user-journey failure and cleanup failure retain both actionable causes."""
	patch_local_catalog(monkeypatch)
	dependencies, commands, receipts = offline_dependencies(tmp_path, child_failure=True, cleanup_failure=True)
	with pytest.raises(BaseExceptionGroup, match="lifecycle failures") as raised:
		browser_suite_owner.run_selected_scenario(TEST_SCENARIO, dependencies)
	messages = {str(error) for error in raised.value.exceptions}
	assert {f"browser scenario failed: {TEST_SCENARIO}: production browser-suite command failed: npx"}.issubset(messages)
	assert len(messages) == 2
	assert commands[-1][3] == "cleanup" and not receipts[0].private_state_removed


#============================================
def test_launch_failure_arms_cleanup_without_claiming_launch_completion(
	tmp_path: pathlib.Path, monkeypatch: pytest.MonkeyPatch
) -> None:
	"""A failed launch still receives typed cleanup for partially created resources."""
	patch_local_catalog(monkeypatch)
	dependencies, commands, receipts = offline_dependencies(tmp_path, launch_failure=True)
	with pytest.raises(browser_suite_owner.BrowserSuiteError, match="command failed"):
		browser_suite_owner.run_selected_scenario(TEST_SCENARIO, dependencies)
	assert commands[-1][3] == "cleanup" and receipts[0].lifecycle_launch_attempted and not receipts[0].lifecycle_launch_completed


#============================================
def test_removal_and_reporter_failures_preserve_the_operation_failure(
	tmp_path: pathlib.Path, monkeypatch: pytest.MonkeyPatch
) -> None:
	"""Removal and reporting failures accompany rather than replace child failure."""
	patch_local_catalog(monkeypatch)
	dependencies, commands, receipts = offline_dependencies(tmp_path, child_failure=True, removal_failure=True, reporter_failure=True)
	with pytest.raises(BaseExceptionGroup, match="lifecycle failures") as raised:
		browser_suite_owner.run_selected_scenario(TEST_SCENARIO, dependencies)
	messages = {str(error) for error in raised.value.exceptions}
	assert {f"browser scenario failed: {TEST_SCENARIO}: production browser-suite command failed: npx", "injected private-state removal failure", "injected receipt reporter failure"}.issubset(messages)
	assert commands[-1][3] == "cleanup" and receipts[0].cleanup_completed and not receipts[0].private_state_removed


#============================================
def test_playwright_environment_excludes_ambient_diagnostic_controls(
	monkeypatch: pytest.MonkeyPatch, tmp_path: pathlib.Path
) -> None:
	"""The Playwright child receives only its runtime allowlist and private path."""
	report_path = tmp_path / "report"
	ambient = {
		"PATH": "/safe/path",
		"HOME": "/safe/home",
		"DEBUG": "playwright:*",
		"PWDEBUG": "console",
		"NODE_OPTIONS": "--inspect=127.0.0.1:9229",
		"PLAYWRIGHT_HTML_REPORT": str(report_path),
		"PLE_LIVE_DEMO_BROWSER_INPUT_FILE": "/unsafe/input",
	}
	monkeypatch.setattr(local_stack_control.process, "current_environment", lambda: ambient)
	input_path = tmp_path / "input.json"
	environment = browser_suite_owner.playwright_environment(input_path)
	assert environment["PATH"] == "/safe/path" and environment["PLE_LIVE_DEMO_BROWSER_INPUT_FILE"] == str(input_path)
	assert not {"DEBUG", "PWDEBUG", "NODE_OPTIONS", "PLAYWRIGHT_HTML_REPORT"}.intersection(environment)


#============================================
def test_invalid_private_input_stops_before_playwright_and_cleans_owner_state(
	tmp_path: pathlib.Path, monkeypatch: pytest.MonkeyPatch
) -> None:
	"""A malformed private ABI cannot reach Chromium and still records cleanup."""
	patch_local_catalog(monkeypatch)
	dependencies, commands, receipts = offline_dependencies(tmp_path, invalid_input=True)
	with pytest.raises(browser_suite_owner.BrowserSuiteError, match="not valid JSON"):
		browser_suite_owner.run_selected_scenario(TEST_SCENARIO, dependencies)
	assert all(command[0] != "npx" for command in commands) and receipts[0].cleanup_completed


#============================================
def test_origin_mismatch_aggregates_with_cleanup_and_keeps_public_safe_receipt(
	tmp_path: pathlib.Path, monkeypatch: pytest.MonkeyPatch
) -> None:
	"""A Chromium request outside the gateway fails after typed cleanup and excludes private proof."""
	patch_local_catalog(monkeypatch)
	dependencies, commands, receipts = offline_dependencies(tmp_path, origin_mismatch=True)
	with pytest.raises(browser_suite_owner.BrowserSuiteError, match=f"{TEST_SCENARIO}.*outside"):
		browser_suite_owner.run_selected_scenario(TEST_SCENARIO, dependencies)
	assert commands[-1][3] == "cleanup"
	assert "AAAAAAAA" not in receipts[0].as_json()
	assert "privateStateDirectory" not in receipts[0].as_json()


#============================================
def test_cleanup_oracle_rejects_an_owned_process_leak(
	tmp_path: pathlib.Path, monkeypatch: pytest.MonkeyPatch
) -> None:
	"""A residual owner process makes a successful child run fail its cleanup receipt."""
	patch_local_catalog(monkeypatch)
	dependencies, commands, receipts = offline_dependencies(tmp_path)
	reads = 0

	def read_inventory(
		project: str,
		directory: pathlib.Path,
		runner: local_stack_control.process.CommandRunner,
		root: pathlib.Path,
		provider: browser_suite_oracles.ProviderReceipt,
		sessions: tuple[local_stack_control.process.ProcessSession, ...],
	) -> browser_suite_oracles.SuiteInventory:
		nonlocal reads
		reads += 1
		processes = () if reads < 3 else (browser_suite_oracles.ProcessIdentity(999, 1, 999),)
		return browser_suite_oracles.SuiteInventory(project, (), (), (), (), processes, provider)

	leaky = dataclasses.replace(dependencies, inventory_reader=read_inventory)
	with pytest.raises(browser_suite_oracles.BrowserSuiteOracleError, match="background processes"):
		browser_suite_owner.run_selected_scenario(TEST_SCENARIO, leaky)
	assert commands[-1][3] == "cleanup" and receipts[0].after_inventory.owner_processes


#============================================
def test_cleanup_oracle_rejects_a_labelled_resource_leak(
	tmp_path: pathlib.Path, monkeypatch: pytest.MonkeyPatch
) -> None:
	"""A remaining labelled volume prevents the runner from reporting clean repeat-run readiness."""
	patch_local_catalog(monkeypatch)
	dependencies, commands, receipts = offline_dependencies(tmp_path)
	reads = 0

	def read_inventory(
		project: str,
		directory: pathlib.Path,
		runner: local_stack_control.process.CommandRunner,
		root: pathlib.Path,
		provider: browser_suite_oracles.ProviderReceipt,
		sessions: tuple[local_stack_control.process.ProcessSession, ...],
	) -> browser_suite_oracles.SuiteInventory:
		nonlocal reads
		reads += 1
		volumes = () if reads < 3 else (local_stack_control.models.VolumeResource("leftover", project),)
		return browser_suite_oracles.SuiteInventory(project, (), volumes, (), (), (), provider)

	leaky = dataclasses.replace(dependencies, inventory_reader=read_inventory)
	with pytest.raises(browser_suite_oracles.BrowserSuiteOracleError, match="labelled project resources"):
		browser_suite_owner.run_selected_scenario(TEST_SCENARIO, leaky)
	assert commands[-1][3] == "cleanup" and receipts[0].after_inventory.volumes


#============================================
def test_capability_mismatch_remains_a_cleanup_failure_in_the_owner_receipt(
	tmp_path: pathlib.Path, monkeypatch: pytest.MonkeyPatch
) -> None:
	"""The adapter capability boundary remains visible when cleanup rejects a forged resource."""
	patch_local_catalog(monkeypatch)
	dependencies, commands, receipts = offline_dependencies(tmp_path)

	def capability_checked_command(
		runner: local_stack_control.process.CommandRunner,
		argv: list[str],
		root: pathlib.Path,
		environment: dict[str, str] | None,
	) -> local_stack_control.process.ProcessSession:
		if argv[0] != "npx" and argv[3] == "cleanup":
			raise browser_suite_owner.BrowserSuiteError("injected disposable capability mismatch")
		return dependencies.command_runner(runner, argv, root, environment)

	capability_mismatch = dataclasses.replace(dependencies, command_runner=capability_checked_command)
	with pytest.raises(browser_suite_owner.BrowserSuiteError, match="capability mismatch"):
		browser_suite_owner.run_selected_scenario(TEST_SCENARIO, capability_mismatch)
	assert not receipts[0].cleanup_completed and not receipts[0].private_state_removed


#============================================
def test_failed_command_session_remains_in_the_final_process_oracle(
	tmp_path: pathlib.Path, monkeypatch: pytest.MonkeyPatch
) -> None:
	"""A nonzero child retains its session identity so a reparented process cannot disappear."""
	patch_local_catalog(monkeypatch)
	dependencies, commands, receipts = offline_dependencies(tmp_path)

	def failing_command(
		runner: local_stack_control.process.CommandRunner,
		argv: list[str],
		root: pathlib.Path,
		environment: dict[str, str] | None,
	) -> local_stack_control.process.SessionCommandResult:
		if argv[0] == "npx":
			return local_stack_control.process.SessionCommandResult(
				local_stack_control.process.ProcessSession(991, 1, "process-group-or-marker", "marker"), 1
			)
		return dependencies.command_runner(runner, argv, root, environment)

	def read_inventory(
		project: str,
		directory: pathlib.Path,
		runner: local_stack_control.process.CommandRunner,
		root: pathlib.Path,
		provider: browser_suite_oracles.ProviderReceipt,
		sessions: tuple[local_stack_control.process.ProcessSession, ...],
	) -> browser_suite_oracles.SuiteInventory:
		leaked = any(item.process_group_id == 991 for item in sessions)
		processes = (browser_suite_oracles.ProcessIdentity(42, 1, 991),) if leaked else ()
		return browser_suite_oracles.SuiteInventory(project, (), (), (), (), processes, provider)

	failed = dataclasses.replace(dependencies, command_runner=failing_command, inventory_reader=read_inventory)
	with pytest.raises(BaseExceptionGroup, match="lifecycle failures") as raised:
		browser_suite_owner.run_selected_scenario(TEST_SCENARIO, failed)
	messages = {str(error) for error in raised.value.exceptions}
	assert f"browser scenario failed: {TEST_SCENARIO}: production browser-suite command failed: npx" in messages
	assert "browser-suite cleanup left owner background processes" in messages
	assert receipts[0].after_inventory.owner_processes


#============================================
@pytest.mark.parametrize("change", [
	lambda value: value.update({"serviceReceipt": None}),
	lambda value: value.update({"sysadminOwnershipProof": "A" * 42 + "F"}),
])
def test_python_v1_parser_rejects_null_optionals_and_noncanonical_proof(
	tmp_path: pathlib.Path,
	change: object,
) -> None:
	"""Python enforces the same optional-key and base64url rules as TypeScript."""
	contract = scenario_contract()
	value = {
		"schemaVersion": 2,
		"scenarioId": contract.scenario_id,
		"namespace": f"bs1-0123456789ab-{TEST_SCENARIO}",
		"baseUrl": "https://localhost:55001/",
		"personas": list(contract.personas),
		"baselineReads": list(contract.baseline_reads),
		"sysadminRequirement": contract.sysadmin_requirement,
		"visibleObservation": contract.visible_observation,
		"sysadminOwnershipProof": "A" * 42 + "E",
	}
	change(value)  # type: ignore[operator]
	path = tmp_path / "input.json"
	browser_suite_owner.private_file(path, json.dumps(value, separators=(",", ":")))
	with pytest.raises(browser_suite_owner.BrowserSuiteError):
		browser_suite_owner.validate_browser_input(path, 55001, contract)


#============================================
def test_continuation_path_is_private_to_transition_children_and_receipts_redact_it(
	tmp_path: pathlib.Path, monkeypatch: pytest.MonkeyPatch,
) -> None:
	"""Only owner-created setup/claimed children receive the credential capability path."""
	base = scenario_contract()
	ordinary = dataclasses.replace(
		base,
		scenario_id="ordinary",
		spec_path="tests/playwright/e2e/ordinary.spec.ts",
		personas=("elena_instructor",),
		baseline_reads=("base_course",),
		ui_creates=("course",),
		sysadmin_requirement="not_required",
		exclusive_seed_mutations=(),
	)
	claimed = dataclasses.replace(
		base,
		scenario_id="claimed",
		spec_path="tests/playwright/e2e/claimed.spec.ts",
		ui_creates=("question", "course"),
		sysadmin_requirement="claimed",
		exclusive_seed_mutations=(),
	)
	monkeypatch.setattr(browser_scenario_contract, "scenario_contracts", lambda: (ordinary, base, claimed))
	dependencies, _commands, _receipts = offline_dependencies(tmp_path)
	child_environments: dict[str, dict[str, str]] = {}
	child_inputs: dict[str, dict[str, object]] = {}
	original_command_runner = dependencies.command_runner

	def record_child_environment(
		runner: local_stack_control.process.CommandRunner,
		argv: list[str],
		root: pathlib.Path,
		environment: dict[str, str] | None,
	) -> local_stack_control.process.SessionCommandResult:
		if argv[0] == "npx":
			assert environment is not None
			scenario_id = pathlib.Path(argv[3]).stem.removesuffix(".spec")
			child_environments[scenario_id] = dict(environment)
			child_inputs[scenario_id] = json.loads(
				pathlib.Path(environment["PLE_LIVE_DEMO_BROWSER_INPUT_FILE"]).read_text(
					encoding="ascii"
				)
			)
		return original_command_runner(runner, argv, root, environment)

	receipt = browser_suite_owner.run_selection(
		browser_suite_owner.BrowserSuiteSelection(None, None, False),
		dataclasses.replace(dependencies, command_runner=record_child_environment),
	)
	path_name = "PLE_BROWSER_SUITE_WEBAUTHN_CONTINUATION_FILE"
	acknowledgement_name = "PLE_BROWSER_SUITE_WEBAUTHN_CONTINUATION_ACK_FILE"
	assert path_name not in child_environments["ordinary"]
	assert acknowledgement_name not in child_environments["ordinary"]
	assert acknowledgement_name not in child_environments[TEST_SCENARIO]
	assert child_environments[TEST_SCENARIO][path_name] == child_environments["claimed"][path_name]
	continuation_path = child_environments["claimed"][path_name]
	acknowledgement_path = child_environments["claimed"][acknowledgement_name]
	assert continuation_path not in receipt.as_json()
	assert acknowledgement_path not in receipt.as_json()
	assert "AAAAAAAA" not in receipt.as_json()
	assert [item.webauthn_continuation_consumed for item in receipt.scenario_receipts] == [
		False,
		False,
		True,
	]
	for scenario_id, input_value in child_inputs.items():
		assert ("sysadminOwnershipProof" in input_value) == (scenario_id == TEST_SCENARIO)

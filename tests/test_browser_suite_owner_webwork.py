"""Offline contracts for the private WebWork renderer hand-off."""

import dataclasses
import pathlib
import sys

import pytest

import local_stack_control.models
import local_stack_control.process

from test_browser_suite_owner import (
	browser_scenario_contract,
	browser_suite_owner,
	offline_dependencies,
	webwork_contract,
)

#============================================
def test_webwork_delivery_seed_and_renderer_witness_are_private_and_scenario_scoped(
	tmp_path: pathlib.Path, monkeypatch: pytest.MonkeyPatch,
) -> None:
	"""The one catalog seed and content-free renderer receipt stay inside WebWork's child."""
	contract = webwork_contract()
	monkeypatch.setattr(browser_scenario_contract, "scenario_contracts", lambda: (contract,))
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
	ordinary = dataclasses.replace(
		webwork_contract(),
		scenario_id="ordinary",
		spec_path="tests/playwright/e2e/ordinary.spec.ts",
		service_receipt=None,
		visible_observation="ordinary_visible_observation",
	)
	webwork = webwork_contract()
	monkeypatch.setattr(browser_scenario_contract, "scenario_contracts", lambda: (ordinary, webwork))
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
		assert name not in child_environments["ordinary"]
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
	monkeypatch: pytest.MonkeyPatch,
	seed_failure: bool,
	seed_receipt: str | None,
	error: str,
) -> None:
	"""Seed command and its two-field receipt fail closed before Chromium starts."""
	contract = webwork_contract()
	monkeypatch.setattr(browser_scenario_contract, "scenario_contracts", lambda: (contract,))
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
	monkeypatch: pytest.MonkeyPatch,
	produce_acknowledgement: bool,
	acknowledgement: str | None,
	error: str,
) -> None:
	"""Renderer evidence follows a successful UI-issued acknowledgement only."""
	contract = webwork_contract()
	monkeypatch.setattr(browser_scenario_contract, "scenario_contracts", lambda: (contract,))
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
	tmp_path: pathlib.Path, monkeypatch: pytest.MonkeyPatch, after_logs: str,
) -> None:
	"""A zero or duplicate renderer receipt cannot be credited to a visible journey."""
	contract = webwork_contract()
	monkeypatch.setattr(browser_scenario_contract, "scenario_contracts", lambda: (contract,))
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

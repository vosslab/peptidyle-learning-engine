"""Offline owner tests for independent production browser children."""

import pathlib
import sys
from types import SimpleNamespace

import pytest

E2E_DIRECTORY = pathlib.Path(__file__).resolve().parent / "e2e"
sys.path.insert(0, str(E2E_DIRECTORY))

import e2e_browser_scenario_contract as browser_scenario_contract
import e2e_browser_suite_owner as browser_suite_owner


def test_owner_preserves_selected_order_without_setup_child() -> None:
	"""A direct role target receives no hidden first-claim predecessor."""
	direct = browser_scenario_contract.require_contract("direct_role_entry")
	assert browser_suite_owner.ordered_execution_contracts((direct,)) == (direct,)


def test_input_abi_has_no_role_claim_or_ownership_fields(tmp_path: pathlib.Path) -> None:
	"""The owner writes server-resolved personas, not browser role claims or proof material."""
	contract = browser_scenario_contract.require_contract("direct_role_entry")
	path = tmp_path / "input.json"
	browser_suite_owner.write_browser_input(path, 55001, contract)
	contents = path.read_text(encoding="ascii")
	assert "sysadminRequirement" not in contents
	assert "sysadminOwnershipProof" not in contents
	assert "first_claim" not in contents


def test_run_selection_passes_ordered_contracts_for_ordinary_selection(
	monkeypatch: pytest.MonkeyPatch,
) -> None:
	"""An ordinary selection reaches child execution with its catalog contract defined."""
	direct = browser_scenario_contract.require_contract("direct_role_entry")
	executed: list[tuple[object, ...]] = []
	lifecycle = SimpleNamespace(failures=[])
	receipt = object()
	dependencies = SimpleNamespace(
		selections={},
		ports=(55001, 55002, 55003, 55004),
		runner=object(),
		root=pathlib.Path("."),
		port_checker=lambda *_arguments: None,
	)
	monkeypatch.setattr(browser_suite_owner, "require_canonical_selections", lambda _value: None)
	monkeypatch.setattr(browser_suite_owner, "prepare_lifecycle_state", lambda *_args: lifecycle)
	monkeypatch.setattr(browser_suite_owner, "launch_production_stack", lambda *_args: pathlib.Path("manifest"))
	monkeypatch.setattr(
		browser_suite_owner,
		"execute_visible_scenarios",
		lambda _selection, _contracts, execution_contracts, _manifest, _dependencies, _lifecycle: executed.append(tuple(execution_contracts)),
	)
	monkeypatch.setattr(browser_suite_owner, "cleanup_and_observe", lambda *_args: None)
	monkeypatch.setattr(browser_suite_owner, "lifecycle_receipt", lambda *_args: receipt)
	monkeypatch.setattr(browser_suite_owner, "collect_screenshots_and_report", lambda *_args: None)
	monkeypatch.setattr(browser_suite_owner, "raise_lifecycle_failures", lambda _failures: None)

	browser_suite_owner.run_selection(
		browser_suite_owner.BrowserSuiteSelection("direct_role_entry", None, False),
		dependencies,
	)

	assert executed == [(direct,)]

"""Offline contracts for independent real-stack browser scenarios."""

import pathlib
import sys

import pytest

E2E_DIRECTORY = pathlib.Path(__file__).resolve().parent / "e2e"
sys.path.insert(0, str(E2E_DIRECTORY))

import e2e_browser_scenario_contract as browser_scenario_contract


def test_scenario_registry_requires_seeded_authorization_journey() -> None:
	"""Removing the real seeded authorization scenario fails closed."""
	registry = tuple(
		contract
		for contract in browser_scenario_contract.scenario_contracts()
		if contract.scenario_id != "auth_authorization"
	)
	with pytest.raises(browser_scenario_contract.ScenarioContractError, match="seeded authorization"):
		browser_scenario_contract.validate_registry(registry)


def test_contract_rejects_unknown_closed_values_before_lifecycle_allocation() -> None:
	"""The active schema rejects unsupported personas, receipts, and faults."""
	contract = browser_scenario_contract.ScenarioContract(
		scenario_id="direct",
		spec_path="tests/playwright/e2e/direct.spec.ts",
		personas=("morgan_sysadmin",),
		baseline_reads=("genetics_practice_course",),
		ui_creates=("teaching_invitation",),
		visible_observation="direct_entry",
		service_receipt="unknown",
	)
	with pytest.raises(browser_scenario_contract.ScenarioContractError):
		browser_scenario_contract.validate_contract(contract)


def test_contract_requires_visible_ascii_observation_evidence() -> None:
	"""Descriptive evidence contains visible ASCII rather than blank padding."""
	contract = browser_scenario_contract.dataclasses.replace(
		browser_scenario_contract.scenario_contracts()[0],
		visible_observation="   ",
	)
	with pytest.raises(browser_scenario_contract.ScenarioContractError, match="visible observation"):
		browser_scenario_contract.validate_contract(contract)


def test_registry_rejects_duplicate_scenarios_and_unsafe_selection() -> None:
	"""Focused execution must resolve one distinct checked-in scenario."""
	registry = browser_scenario_contract.scenario_contracts()
	with pytest.raises(browser_scenario_contract.ScenarioContractError, match="unique"):
		browser_scenario_contract.validate_registry((*registry, registry[0]))
	with pytest.raises(browser_scenario_contract.ScenarioContractError):
		browser_scenario_contract.resolve_selection(None, "unsafe.spec.ts", None, registry)


def test_namespace_and_seed_transition_policy_are_owner_bound() -> None:
	"""Public namespaces and every visible seed transition stay closed and owner-defined."""
	assert browser_scenario_contract.namespace_for("direct", "0123456789ab") == "bs1-0123456789ab-direct"
	with pytest.raises(browser_scenario_contract.ScenarioContractError):
		browser_scenario_contract.namespace_for("direct", "unsafe")
	registry = browser_scenario_contract.scenario_contracts()
	unsafe = browser_scenario_contract.dataclasses.replace(
		registry[0], seed_state_transitions=("caller_defined_transition",)
	)
	with pytest.raises(browser_scenario_contract.ScenarioContractError, match="seed state"):
		browser_scenario_contract.validate_contract(unsafe)

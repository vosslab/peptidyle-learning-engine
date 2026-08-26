"""Offline contracts for independent real-stack browser scenarios."""

import pathlib
import sys

import pytest

E2E_DIRECTORY = pathlib.Path(__file__).resolve().parent / "e2e"
sys.path.insert(0, str(E2E_DIRECTORY))

import e2e_browser_scenario_contract as browser_scenario_contract


def test_catalog_is_direct_role_only_and_consumes_its_own_inputs() -> None:
	"""Every registered spec begins independently and Morgan has a direct scenario."""
	registry = browser_scenario_contract.scenario_contracts()
	browser_scenario_contract.validate_registry(registry, E2E_DIRECTORY.parents[1])
	direct = browser_scenario_contract.require_contract("direct_role_entry", registry)
	assert direct.personas == ("morgan_sysadmin",)
	assert direct.ui_creates == ("passkey",)
	assert direct.seed_state_transitions == ()
	assert direct.screenshot_states == ("account_security_passkey",)
	auth = browser_scenario_contract.require_contract("auth_authorization", registry)
	assert "elena_instructor" in auth.personas
	assert "passkey" in auth.ui_creates
	assert auth.visible_observation.startswith("instructor_passkey_reauthentication")
	pool = browser_scenario_contract.require_contract("item_pool_delivery", registry)
	assert pool.personas == ("elena_instructor", "mary_student")
	assert pool.screenshot_states == ("pool_preview", "learner_delivered_pool")


@pytest.mark.parametrize("scenario_id", ["direct_role_entry", "auth_authorization"])
def test_catalog_requires_both_named_role_security_journeys(scenario_id: str) -> None:
	"""Elena and Morgan remain independent mandatory live-demo acceptance children."""
	registry = tuple(
		contract
		for contract in browser_scenario_contract.scenario_contracts()
		if contract.scenario_id != scenario_id
	)
	with pytest.raises(browser_scenario_contract.ScenarioContractError, match="role-security"):
		browser_scenario_contract.validate_registry(registry)


def test_contract_rejects_unknown_closed_values_before_lifecycle_allocation() -> None:
	"""The active schema rejects unsupported personas, receipts, and faults."""
	contract = browser_scenario_contract.ScenarioContract(
		scenario_id="direct",
		spec_path="tests/playwright/e2e/direct.spec.ts",
		personas=("morgan_sysadmin",),
		baseline_reads=("genetics_practice_course",),
		ui_creates=("passkey",),
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


def test_namespace_and_repeat_safe_seed_transition_are_owner_bound() -> None:
	"""Public namespaces and visible seed transitions stay closed and repeat-safe."""
	assert browser_scenario_contract.namespace_for("direct", "0123456789ab") == "bs1-0123456789ab-direct"
	with pytest.raises(browser_scenario_contract.ScenarioContractError):
		browser_scenario_contract.namespace_for("direct", "unsafe")
	registry = browser_scenario_contract.scenario_contracts()
	first = browser_scenario_contract.dataclasses.replace(
		registry[0], seed_state_transitions=("avery_instructor_approval",)
	)
	second = browser_scenario_contract.dataclasses.replace(
		registry[1], seed_state_transitions=("avery_instructor_approval",)
	)
	browser_scenario_contract.validate_registry((first, second, *registry[2:]))
	unsafe = browser_scenario_contract.dataclasses.replace(
		registry[0], seed_state_transitions=("caller_defined_transition",)
	)
	with pytest.raises(browser_scenario_contract.ScenarioContractError, match="seed state"):
		browser_scenario_contract.validate_contract(unsafe)

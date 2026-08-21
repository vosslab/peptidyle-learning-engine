"""Offline A1 cutover checks for the real-stack authentication scenario family."""

import pathlib
import sys

import pytest

import file_utils

E2E_DIRECTORY = pathlib.Path(file_utils.get_repo_root()) / "tests" / "e2e"
sys.path.insert(0, str(E2E_DIRECTORY))

import e2e_browser_scenario_contract as browser_scenario_contract
import e2e_browser_scenarios_auth as auth_scenarios
import e2e_browser_scenarios_legacy_live_demo as legacy_live_demo


#============================================
def test_a1_replaces_the_legacy_contract_with_two_explicit_real_specs() -> None:
	"""The catalog keeps no selector path to the retired combined journey."""
	assert legacy_live_demo.contracts() == ()
	contracts = auth_scenarios.contracts()
	assert tuple(item.scenario_id for item in contracts) == (
		"sysadmin_first_claim",
		"auth_authorization",
	)
	assert tuple(item.spec_path for item in contracts) == (
		"tests/playwright/e2e/sysadmin_first_claim.spec.ts",
		"tests/playwright/e2e/auth_authorization.spec.ts",
	)
	browser_scenario_contract.validate_registry(repo_root=pathlib.Path(file_utils.get_repo_root()))
	with pytest.raises(browser_scenario_contract.ScenarioContractError, match="unsupported"):
		browser_scenario_contract.require_contract("live_demo")


#============================================
def test_a1_transfers_each_fixed_identity_mutation_to_one_contract() -> None:
	"""The claim proof and Avery approval have one precise scenario owner each."""
	first_claim, authorization = auth_scenarios.contracts()
	assert first_claim.sysadmin_requirement == "unclaimed"
	assert first_claim.exclusive_seed_mutations == ("sysadmin_first_claim",)
	assert authorization.sysadmin_requirement == "claimed"
	assert authorization.exclusive_seed_mutations == ("avery_instructor_approval",)
	browser_scenario_contract.validate_registry(auth_scenarios.contracts())


#============================================
def test_a1_specs_consume_their_owner_input_and_keep_proof_private() -> None:
	"""Each real spec has a narrow owner projection, with proof only in first claim."""
	root = pathlib.Path(file_utils.get_repo_root())
	first_claim = (root / "tests/playwright/e2e/sysadmin_first_claim.spec.ts").read_text(
		encoding="utf-8"
	)
	authorization = (root / "tests/playwright/e2e/auth_authorization.spec.ts").read_text(
		encoding="utf-8"
	)
	assert 'scenarioInput.scenarioId).toBe("sysadmin_first_claim")' in first_claim
	assert "sysadminOwnershipProof" in first_claim
	assert "installVirtualAuthenticator" in first_claim
	assert 'scenarioInput.scenarioId).toBe("auth_authorization")' in authorization
	assert "sysadminOwnershipProof).toBeUndefined()" in authorization
	assert "installVirtualAuthenticator" not in authorization
	assert ".route(" not in first_claim + authorization


#============================================
def test_a1_authorization_spec_records_real_navigation_denial_without_transport_fakes() -> None:
	"""The role and cross-course checks observe the connected UI and actual responses."""
	path = pathlib.Path(file_utils.get_repo_root()) / "tests/playwright/e2e/auth_authorization.spec.ts"
	contents = path.read_text(encoding="utf-8")
	assert "mary.goto(geneticsPath)" in contents
	assert "navigationResponses).toContain(404)" in contents
	assert "protectedFollowOns).toEqual([])" in contents
	assert "You do not manage this course." in contents

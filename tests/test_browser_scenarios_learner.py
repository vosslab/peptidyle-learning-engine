"""Offline contracts for the learner delivery production-browser journey."""

import pathlib
import sys

import file_utils

E2E_DIRECTORY = pathlib.Path(file_utils.get_repo_root()) / "tests" / "e2e"
sys.path.insert(0, str(E2E_DIRECTORY))

import e2e_browser_scenarios_learner
import e2e_browser_scenario_contract


#============================================
def test_learner_delivery_contract_describes_the_visible_two_person_journey() -> None:
	"""The provider commits its real UI state and fresh-session observation boundary."""
	contracts = e2e_browser_scenarios_learner.contracts()
	assert len(contracts) == 1
	contract = contracts[0]
	e2e_browser_scenario_contract.validate_contract(contract)
	assert contract.scenario_id == "learner_delivery"
	assert contract.spec_path == "tests/playwright/e2e/learner_delivery.spec.ts"
	assert contract.personas == ("elena_instructor", "mary_student")
	assert contract.baseline_reads == ("base_course",)
	assert contract.ui_creates == (
		"question", "course", "assignment", "invitation", "response"
	)
	assert contract.sysadmin_requirement == "not_required"
	assert contract.visible_observation == "mary_completed_run_persists_after_fresh_session"


#============================================
def test_learner_delivery_spec_consumes_owner_state_and_stays_on_the_visible_ui_path() -> None:
	"""The real spec uses scenario-owned names and a fresh browser context, not a fixture API."""
	path = pathlib.Path(file_utils.get_repo_root()) / "tests/playwright/e2e/learner_delivery.spec.ts"
	contents = path.read_text(encoding="utf-8")
	assert "scenarioInput.scenarioId" in contents
	assert "scenarioInput.namespace" in contents
	assert "freshMaryContext.storageState" in contents
	assert "Claim this course" in contents
	assert "Start another practice" in contents
	assert "Completed runs" in contents
	assert ".route(" not in contents
	assert "fetch(" not in contents
	assert "sql" not in contents.lower()

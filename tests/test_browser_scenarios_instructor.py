"""Offline contracts for the instructor browser-scenario provider."""

import pathlib
import sys

import file_utils

E2E_DIRECTORY = pathlib.Path(file_utils.get_repo_root()) / "tests" / "e2e"
sys.path.insert(0, str(E2E_DIRECTORY))

import e2e_browser_scenario_contract as scenario_contract
import e2e_browser_scenarios_instructor as instructor_scenarios


#============================================
def test_instructor_authoring_provider_declares_the_ui_first_journey() -> None:
	"""The catalog entry has one production spec and no Sysadmin prerequisite."""
	(contract,) = instructor_scenarios.contracts()
	assert contract.scenario_id == "instructor_authoring"
	assert contract.spec_path == "tests/playwright/e2e/instructor_authoring.spec.ts"
	assert contract.personas == ("elena_instructor",)
	assert contract.baseline_reads == ("base_course",)
	assert contract.ui_creates == ("question", "course", "assignment", "invitation")
	assert contract.sysadmin_requirement == "not_required"
	assert contract.visible_observation == "instructor_authoring_persists_after_reload"
	scenario_contract.validate_contract(contract)


#============================================
def test_instructor_authoring_selection_is_exact_and_anchored() -> None:
	"""The provider's scenario ID and approved spec select the same journey."""
	(contract,) = instructor_scenarios.contracts()
	assert scenario_contract.resolve_selection(
		"instructor_authoring", None, None, (contract,)
	) == (contract,)
	assert scenario_contract.resolve_selection(
		None, contract.spec_path, "instructor authoring", (contract,)
	) == (contract,)


#============================================
def test_instructor_authoring_spec_consumes_its_owner_input_and_uses_visible_controls() -> None:
	"""The registered spec stays bound to its private input and real UI journey."""
	(contract,) = instructor_scenarios.contracts()
	contents = (pathlib.Path(file_utils.get_repo_root()) / contract.spec_path).read_text(
		encoding="utf-8"
	)
	assert "scenarioInput.scenarioId" in contents
	assert "scenarioInput.namespace" in contents
	assert "chooseSeededIdentity" in contents
	assert "Create flat question" in contents
	assert "Create course" in contents
	assert "Create invitation" in contents
	assert ".route(" not in contents
	assert "mock" not in contents.lower()

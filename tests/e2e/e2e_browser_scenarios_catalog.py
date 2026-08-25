"""Deterministic, reviewable production browser-scenario provider composition."""

from e2e_browser_scenario_contract import ScenarioContract
import e2e_browser_scenarios_auth as auth
import e2e_browser_scenarios_assignment_replacement as assignment_replacement
import e2e_browser_scenarios_conflict as conflict
import e2e_browser_scenarios_problem_curation as problem_curation
import e2e_browser_scenarios_reusable_curriculum as reusable_curriculum
import e2e_browser_scenarios_discovery as discovery
import e2e_browser_scenarios_failure as failure
import e2e_browser_scenarios_instructor as instructor
import e2e_browser_scenarios_item_pool as item_pool
import e2e_browser_scenarios_learner as learner
import e2e_browser_scenarios_preview as preview
import e2e_browser_scenarios_qti as qti
import e2e_browser_scenario_webwork_delivery as webwork_delivery


def contracts() -> tuple[ScenarioContract, ...]:
	"""Return real-stack scenario families in fixed execution order."""
	return (
		auth.contracts()
		+ instructor.contracts()
		+ preview.contracts()
		+ assignment_replacement.contracts()
		+ item_pool.contracts()
		+ conflict.contracts()
		+ learner.contracts()
		+ discovery.contracts()
		+ problem_curation.contracts()
		+ reusable_curriculum.contracts()
		+ (
			ScenarioContract(
				scenario_id=webwork_delivery.SCENARIO_ID,
				spec_path="tests/playwright/e2e/webwork_delivery.spec.ts",
				personas=("elena_instructor", "mary_student"),
				baseline_reads=("base_course",),
				ui_creates=("course", "assignment", "invitation", "response"),
				visible_observation="visible_webwork_completion_persists_in_a_fresh_session",
				service_receipt="renderer_delivery",
			),
		)
		+ failure.contracts()
		+ qti.contracts()
	)

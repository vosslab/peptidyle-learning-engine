"""Deterministic, reviewable production browser-scenario provider composition."""

from e2e_browser_scenario_contract import ScenarioContract
import e2e_browser_scenarios_auth as auth
import e2e_browser_scenarios_conflict as conflict
import e2e_browser_scenarios_discovery as discovery
import e2e_browser_scenarios_failure as failure
import e2e_browser_scenarios_instructor as instructor
import e2e_browser_scenarios_item_pool as item_pool
import e2e_browser_scenarios_learner as student
import e2e_browser_scenarios_preview as preview
import e2e_browser_scenario_webwork_delivery as webwork_delivery


def contracts() -> tuple[ScenarioContract, ...]:
	"""Return real-stack scenario families in fixed execution order."""
	return (
		auth.contracts()
		+ instructor.contracts()
		+ preview.contracts()
		+ item_pool.contracts()
		+ conflict.contracts()
		+ student.contracts()
		+ discovery.contracts()
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
	)

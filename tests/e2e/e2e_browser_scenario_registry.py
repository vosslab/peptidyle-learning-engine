"""Deterministic, reviewable production browser-scenario provider composition."""

from e2e_browser_scenario_contract import ScenarioContract
import e2e_browser_scenarios_auth as auth
import e2e_browser_scenarios_sysadmin_student_data_access as sysadmin_student_data_access


def contracts() -> tuple[ScenarioContract, ...]:
	"""Return real-stack scenario families in fixed execution order."""
	return (
		auth.contracts()
		+ sysadmin_student_data_access.contracts()
		+ (
			ScenarioContract(
				scenario_id="student_work",
				spec_path="tests/playwright/e2e/student_work.spec.ts",
				personas=("elena_instructor", "avery_student"),
				baseline_reads=("seeded_accounts", "base_course"),
				ui_creates=("assessment", "response"),
				visible_observation="student_preserves_backend_and_native_work_before_navigation_and_review",
				service_receipt="renderer_delivery",
			),
		)
	)

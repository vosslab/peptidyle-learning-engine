"""Lifecycle-controlled recovery journeys owned by the production browser suite."""

from e2e_browser_scenario_contract import ScenarioContract


def contracts() -> tuple[ScenarioContract, ...]:
	"""Return the learner gateway-recovery journey."""
	return (
		ScenarioContract(
			scenario_id="learner_gateway_recovery",
			spec_path="tests/playwright/e2e/learner_gateway_recovery.spec.ts",
			personas=("elena_instructor", "mary_student"),
			baseline_reads=("base_course",),
			ui_creates=("question", "course", "assignment", "invitation", "response"),
			sysadmin_requirement="not_required",
			visible_observation="saved_response_retries_after_gateway_recovery",
			fault_transition="gateway_submit_outage",
			screenshot_states=(
				"gateway_retry",
				"recovered_feedback",
				"recovered_completion",
				"fresh_session_score",
			),
		),
	)

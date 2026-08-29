"""Lifecycle-controlled recovery journeys owned by the production browser suite."""

from e2e_browser_scenario_contract import ScenarioContract


def contracts() -> tuple[ScenarioContract, ...]:
	"""Return lifecycle-controlled gateway and deterministic-grader recovery journeys."""
	return (
		ScenarioContract(
			scenario_id="automated_grading_recovery",
			spec_path="tests/playwright/e2e/automated_grading_recovery.spec.ts",
			personas=("elena_instructor", "mary_student"),
			baseline_reads=("base_course",),
			ui_creates=("course", "assignment", "question", "invitation", "response"),
			visible_observation="instructor_retry_resolves_deterministic_grader_exception",
			service_receipt="worker_completion",
			fault_transition="deterministic_grader_exception",
			screenshot_states=(
				"instructor_operation",
				"instructor_gradebook",
				"audited_student_work",
			),
		),
		ScenarioContract(
			scenario_id="learner_gateway_recovery",
			spec_path="tests/playwright/e2e/learner_gateway_recovery.spec.ts",
			personas=("elena_instructor", "mary_student"),
			baseline_reads=("base_course",),
			ui_creates=("question", "course", "assignment", "invitation", "response"),
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

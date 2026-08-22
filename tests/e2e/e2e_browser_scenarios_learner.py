"""Learner delivery journeys owned by the canonical production browser suite."""

from e2e_browser_scenario_contract import ScenarioContract


def contracts() -> tuple[ScenarioContract, ...]:
	"""Return learner journeys in their stable catalog order."""
	return (
		ScenarioContract(
			scenario_id="learner_delivery",
			spec_path="tests/playwright/e2e/learner_delivery.spec.ts",
			personas=("elena_instructor", "mary_student"),
			baseline_reads=("base_course",),
			ui_creates=("question", "course", "assignment", "invitation", "response"),
			sysadmin_requirement="not_required",
			visible_observation="mary_completed_run_persists_after_fresh_session",
			screenshot_states=(
				"assignment_list",
				"assignment_overview",
				"problem_ready",
				"response_selected",
				"feedback_correct",
				"completion",
				"repeat_run",
				"fresh_session_score",
				"instructor_active_roster",
				"instructor_gradebook",
				"appearance_saved",
				"access_preview_allowed",
				"access_preview_denied",
			),
		),
	)

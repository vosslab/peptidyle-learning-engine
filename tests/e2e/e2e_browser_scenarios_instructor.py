"""UI-first instructor-authoring scenarios in deterministic scenario-registry order."""

from e2e_browser_scenario_contract import ScenarioContract


def contracts() -> tuple[ScenarioContract, ...]:
	"""Return instructor journeys that start from the normal seeded baseline."""
	return (
		ScenarioContract(
			scenario_id="instructor_authoring",
			spec_path="tests/playwright/e2e/instructor_authoring.spec.ts",
			personas=("elena_instructor",),
			baseline_reads=("base_course",),
			ui_creates=("question", "course", "assignment", "invitation"),
			visible_observation="instructor_authoring_persists_after_reload",
			screenshot_states=("assignment_policies", "student_view"),
		),
	)

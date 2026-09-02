"""Student delivery journeys owned by the canonical production browser suite."""

from e2e_browser_scenario_contract import ScenarioContract


def contracts() -> tuple[ScenarioContract, ...]:
	"""Return student journeys in their stable scenario-registry order."""
	return (
		ScenarioContract(
			scenario_id="learner_delivery",
			spec_path="tests/playwright/e2e/learner_delivery.spec.ts",
			personas=("elena_instructor", "mary_student"),
			baseline_reads=("base_course",),
			ui_creates=("question", "course", "assignment", "invitation", "response"),
			visible_observation="mary_completed_assignment_attempt_persists_after_fresh_session",
		),
	)

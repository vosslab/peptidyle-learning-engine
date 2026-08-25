"""UI-first assignment question-replacement scenario facts."""

from e2e_browser_scenario_contract import ScenarioContract


def contracts() -> tuple[ScenarioContract, ...]:
	"""Return the independent issued-work replacement journey."""
	return (
		ScenarioContract(
			scenario_id="assignment_question_replacement",
			spec_path="tests/playwright/e2e/assignment_question_replacement.spec.ts",
			personas=("elena_instructor", "mary_student"),
			baseline_reads=("base_course",),
			ui_creates=("question", "course", "assignment", "invitation", "response"),
			visible_observation="issued_question_persists_while_future_run_uses_replacement",
		),
	)

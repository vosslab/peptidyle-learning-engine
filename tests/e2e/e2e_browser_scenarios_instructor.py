"""UI-first instructor-authoring scenarios in deterministic scenario-registry order."""

from e2e_browser_scenario_contract import ScenarioContract


def contracts() -> tuple[ScenarioContract, ...]:
	"""Return instructor journeys that start from the normal seeded baseline."""
	return (
		ScenarioContract(
			scenario_id="question_import",
			spec_path="tests/playwright/e2e/question_import.spec.ts",
			personas=("elena_instructor",),
			baseline_reads=("seeded_accounts",),
			ui_creates=("question",),
			visible_observation="seven_imported_question_kinds_preview_as_saved_drafts",
		),
		ScenarioContract(
			scenario_id="instructor_authoring",
			spec_path="tests/playwright/e2e/instructor_authoring.spec.ts",
			personas=("elena_instructor",),
			baseline_reads=("seeded_accounts",),
			ui_creates=("question",),
			visible_observation="instructor_publishes_private_draft_into_question_library",
		),
	)

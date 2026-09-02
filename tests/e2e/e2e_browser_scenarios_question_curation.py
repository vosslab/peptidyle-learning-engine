"""UI-first WP-INST-D2 Question Curation and reusable-selection production facts."""

from e2e_browser_scenario_contract import ScenarioContract


def contracts() -> tuple[ScenarioContract, ...]:
	"""Return the independent Instructor and student Question Curation journey."""
	return (
		ScenarioContract(
			scenario_id="question_curation",
			spec_path="tests/playwright/e2e/question_curation.spec.ts",
			personas=("elena_instructor", "mary_student"),
			baseline_reads=("base_course",),
			ui_creates=(
				"passkey",
				"question_folder",
				"saved_search",
				"course",
				"assignment",
				"invitation",
			),
			visible_observation=(
				"instructor_question_curation_reuses_public_questions_from_private_question_folders"
			),
			screenshot_states=(
				"curation_workspace",
				"revision_recovery",
				"assignment_picker",
			),
		),
	)

"""UI-first WP-INST-D2 curation and reusable-selection production facts."""

from e2e_browser_scenario_contract import ScenarioContract


def contracts() -> tuple[ScenarioContract, ...]:
	"""Return the independent Instructor and student private-curation journey."""
	return (
		ScenarioContract(
			scenario_id="problem_curation",
			spec_path="tests/playwright/e2e/problem_curation.spec.ts",
			personas=("elena_instructor", "mary_student"),
			baseline_reads=("base_course",),
			ui_creates=(
				"passkey",
				"collection",
				"saved_search",
				"course",
				"assignment",
				"invitation",
			),
			visible_observation=(
				"instructor_curation_reuses_public_questions_from_private_collections"
			),
			screenshot_states=(
				"curation_workspace",
				"revision_recovery",
				"assignment_picker",
			),
		),
	)

"""UI-first WP-PROF-B1 reusable-curriculum production facts."""

from e2e_browser_scenario_contract import ScenarioContract


def contracts() -> tuple[ScenarioContract, ...]:
	"""Return the Elena blueprint, Alpha, and shared-picker reuse journey."""
	return (
		ScenarioContract(
			scenario_id="reusable_curriculum",
			spec_path="tests/playwright/e2e/reusable_curriculum.spec.ts",
			personas=("elena_instructor",),
			baseline_reads=("base_course",),
			ui_creates=(
				"passkey",
				"question",
				"blueprint",
				"alpha_curriculum",
				"course",
				"assignment",
			),
			visible_observation=(
				"instructor_revises_reusable_curriculum_and_reuses_alpha_questions_in_assignment_authoring"
			),
			screenshot_states=(
				"reusable_curriculum_workspace",
				"reusable_curriculum_alpha_editor",
				"reusable_curriculum_alpha_reuse",
			),
		),
	)

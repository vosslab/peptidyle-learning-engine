"""UI-first WP-INST-B1 reusable-curriculum production facts."""

from e2e_browser_scenario_contract import ScenarioContract


def contracts() -> tuple[ScenarioContract, ...]:
	"""Return the live Alpha creator, approved-reader, and shared-picker reuse journey."""
	return (
		ScenarioContract(
			scenario_id="reusable_curriculum",
			spec_path="tests/playwright/e2e/reusable_curriculum.spec.ts",
			personas=("elena_instructor", "avery_student", "morgan_sysadmin"),
			baseline_reads=("base_course", "genetics_practice_course"),
			ui_creates=(
				"passkey",
				"question",
				"blueprint",
				"alpha_curriculum",
				"course",
				"assignment",
				"teaching_invitation",
			),
			visible_observation=(
				"instructor_revises_alpha_approved_reader_inspects_and_creator_reuses_question_set"
			),
			seed_state_transitions=("avery_instructor_approval",),
			screenshot_states=(
				"reusable_curriculum_workspace",
				"reusable_curriculum_alpha_editor",
				"reusable_curriculum_alpha_reuse",
				"alpha_reader_inspection",
			),
		),
	)

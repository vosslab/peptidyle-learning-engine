"""UI-first WP-INST-B2 curriculum-adoption production facts."""

from e2e_browser_scenario_contract import ScenarioContract


def contracts() -> tuple[ScenarioContract, ...]:
	"""Return the visible Alpha adoption, update, and rollover journey."""
	return (
		ScenarioContract(
			scenario_id="curriculum_adoption",
			spec_path="tests/playwright/e2e/curriculum_adoption.spec.ts",
			personas=("elena_instructor", "avery_student", "morgan_sysadmin"),
			baseline_reads=("base_course", "genetics_practice_course"),
			ui_creates=(
				"question",
				"alpha_curriculum",
				"course",
				"assignment",
				"teaching_invitation",
			),
			visible_observation=(
				"approved_instructors_fork_alpha_then_elena_corrects_dst_adopts_shifts_"
				"fast_forwards_preserves_divergence_and_rolls_over_a_fresh_course"
			),
			seed_state_transitions=("avery_instructor_approval",),
			screenshot_states=(
				"alpha_fork_review",
				"dst_correction",
				"controlled_update_decision",
				"divergent_recovery",
				"completed_destination_evidence",
			),
		),
	)

"""UI-first production discovery evidence facts for WP-INST-D1."""

from e2e_browser_scenario_contract import ScenarioContract


def contracts() -> tuple[ScenarioContract, ...]:
	"""Return the independent five-student discovery evidence journey."""
	return (
		ScenarioContract(
			scenario_id="question_library_evidence",
			spec_path="tests/playwright/e2e/question_library_discovery_evidence.spec.ts",
			personas=(
				"elena_instructor",
				"mary_student",
				"jack_student",
				"avery_student",
				"morgan_sysadmin",
			),
			baseline_reads=(
				"base_course",
				"genetics_practice_course",
				"mary_completed_assignment_attempt",
				"jack_open_assignment_attempt",
				"published_peptide_assignment",
			),
			ui_creates=("assignment", "invitation", "response"),
			visible_observation=(
				"five_independent_learners_across_two_courses_disclose_evidence_and_authorized_usage"
			),
			screenshot_states=(
				"disclosed_evidence",
				"authorized_usage",
				"filtered_library",
			),
		),
	)

"""Registered Course Appearance propagation scenario facts."""

from e2e_browser_scenario_contract import ScenarioContract


def contracts() -> tuple[ScenarioContract, ...]:
	"""Return the saved Instructor-to-enrolled-Student Course Appearance journey."""
	return (
		ScenarioContract(
			scenario_id="course_appearance_propagation",
			spec_path="tests/playwright/e2e/course_appearance_propagation.spec.ts",
			personas=("elena_instructor", "mary_student"),
			baseline_reads=("seeded_accounts", "base_course"),
			ui_creates=("course",),
			visible_observation="instructor_saved_course_appearance_reloads_for_enrolled_student_only",
		),
	)

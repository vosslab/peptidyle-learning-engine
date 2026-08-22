"""UI-first instructor-authoring scenarios in deterministic catalog order."""

from e2e_browser_scenario_contract import ScenarioContract


def contracts() -> tuple[ScenarioContract, ...]:
	"""Return instructor journeys that start from the normal seeded baseline."""
	return (
		ScenarioContract(
			scenario_id="instructor_authoring",
			spec_path="tests/playwright/e2e/instructor_authoring.spec.ts",
			personas=("elena_instructor",),
			baseline_reads=("base_course",),
			ui_creates=("question", "course", "assignment", "invitation"),
			sysadmin_requirement="not_required",
			visible_observation="instructor_authoring_persists_after_reload",
			screenshot_states=(
				"workspace",
				"question_editor",
				"workspace_draft_saved",
				"publication_success",
				"library",
				"question_detail",
				"course_created",
				"course_assignments",
				"assignment_create",
				"problem_catalog",
				"assignment_created",
				"assignment_editor",
				"assignment_published",
				"assignment_overview",
				"invitation_pending",
				"fresh_session_assignment",
			),
		),
	)

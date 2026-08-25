"""Explicit A1 authentication and authorization scenario facts."""

from e2e_browser_scenario_contract import ScenarioContract


def contracts() -> tuple[ScenarioContract, ...]:
	"""Return A1 journeys in transition-first catalog order."""
	return (
		ScenarioContract(
			scenario_id="direct_role_entry",
			spec_path="tests/playwright/e2e/direct_role_entry.spec.ts",
			personas=("morgan_sysadmin",),
			baseline_reads=("genetics_practice_course",),
			ui_creates=("passkey",),
			visible_observation="direct_sysadmin_passkey_reauthentication",
			screenshot_states=("account_security_passkey",),
		),
		ScenarioContract(
			scenario_id="auth_authorization",
			spec_path="tests/playwright/e2e/auth_authorization.spec.ts",
			personas=(
				"elena_instructor",
				"mary_student",
				"avery_student",
				"morgan_sysadmin",
			),
			baseline_reads=("base_course", "genetics_practice_course"),
			ui_creates=("passkey", "course_group", "teaching_invitation"),
			visible_observation=(
				"instructor_passkey_reauthentication_and_seeded_sessions_avery_approval_and_course_boundaries"
			),
			exclusive_seed_mutations=("avery_instructor_approval",),
			screenshot_states=(
				"teaching_operations_groups",
				"teaching_team_invited",
				"teaching_operations_retention",
				"pending_teaching_invitation",
				"student_instructor_denial",
			),
		),
	)

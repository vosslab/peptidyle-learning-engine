"""Explicit A1 authentication and authorization scenario facts."""

from e2e_browser_scenario_contract import ScenarioContract


def contracts() -> tuple[ScenarioContract, ...]:
	"""Return A1 journeys in transition-first catalog order."""
	return (
		ScenarioContract(
			scenario_id="sysadmin_first_claim",
			spec_path="tests/playwright/e2e/sysadmin_first_claim.spec.ts",
			personas=("morgan_sysadmin",),
			baseline_reads=("genetics_practice_course",),
			ui_creates=("passkey",),
			sysadmin_requirement="unclaimed",
			visible_observation="sysadmin_passkey_reauthentication",
			exclusive_seed_mutations=("sysadmin_first_claim",),
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
			ui_creates=("teaching_invitation",),
			sysadmin_requirement="claimed",
			visible_observation="seeded_sessions_avery_approval_and_course_boundaries",
			exclusive_seed_mutations=("avery_instructor_approval",),
		),
	)

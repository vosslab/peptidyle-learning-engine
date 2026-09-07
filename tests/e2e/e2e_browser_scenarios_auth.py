"""Explicit A1 authentication and authorization scenario facts."""

from e2e_browser_scenario_contract import ScenarioContract


def contracts() -> tuple[ScenarioContract, ...]:
	"""Return the required seeded-entry authorization journey."""
	return (
		ScenarioContract(
			scenario_id="auth_authorization",
			spec_path="tests/playwright/e2e/auth_authorization.spec.ts",
			personas=(
				"elena_instructor",
				"mary_student",
				"avery_student",
				"morgan_sysadmin",
			),
			baseline_reads=("seeded_accounts",),
			ui_creates=(),
			visible_observation="seeded_session_separation_and_no_record_boundaries",
		),
	)

"""Sysadmin student-data access browser scenario and its seed contract."""

from e2e_browser_scenario_contract import ScenarioContract


def contracts() -> tuple[ScenarioContract, ...]:
	"""Return the durable Sysadmin data-access confirmation journey."""
	return (
		ScenarioContract(
			scenario_id="sysadmin_student_data_access",
			spec_path="tests/playwright/e2e/sysadmin_student_data_access.spec.ts",
			personas=("morgan_sysadmin", "elena_instructor", "mary_student"),
			baseline_reads=("seeded_accounts", "base_course"),
			ui_creates=(),
			visible_observation="sysadmin_confirms_student_data_access_and_receives_audit_receipt",
		),
	)

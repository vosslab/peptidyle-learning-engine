"""Deterministic, reviewable production browser-scenario provider composition."""

from e2e_browser_scenario_contract import ScenarioContract
import e2e_browser_scenarios_auth as auth
import e2e_browser_scenarios_sysadmin_student_data_access as sysadmin_student_data_access


def contracts() -> tuple[ScenarioContract, ...]:
	"""Return real-stack scenario families in fixed execution order."""
	return (
		auth.contracts()
		+ sysadmin_student_data_access.contracts()
	)

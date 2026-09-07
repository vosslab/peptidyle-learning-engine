"""Deterministic, reviewable production browser-scenario provider composition."""

from e2e_browser_scenario_contract import ScenarioContract
import e2e_browser_scenarios_auth as auth
import e2e_browser_scenarios_failure as failure
import e2e_browser_scenarios_instructor as instructor


def contracts() -> tuple[ScenarioContract, ...]:
	"""Return real-stack scenario families in fixed execution order."""
	return (
		auth.contracts()
		+ instructor.contracts()
		+ failure.contracts()
	)

"""Deterministic, reviewable production browser-scenario provider composition."""

from e2e_browser_scenario_contract import ScenarioContract
import e2e_browser_scenarios_auth as auth
import e2e_browser_scenarios_instructor as instructor
import e2e_browser_scenarios_learner as learner
import e2e_browser_scenarios_legacy_live_demo as legacy_live_demo


def contracts() -> tuple[ScenarioContract, ...]:
	"""Return legacy first, then the three future family providers in fixed order."""
	return (
		legacy_live_demo.contracts()
		+ auth.contracts()
		+ instructor.contracts()
		+ learner.contracts()
	)

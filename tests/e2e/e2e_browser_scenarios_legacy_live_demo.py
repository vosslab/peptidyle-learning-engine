"""Retired catalog provider retained until R1 removes its unreachable source."""

from e2e_browser_scenario_contract import ScenarioContract


def contracts() -> tuple[ScenarioContract, ...]:
	"""Keep the provider import stable while A1 owns the replacement contracts."""
	return ()

"""Offline profile partitioning for the real-stack browser scenario registry."""

import pathlib
import sys

import pytest

E2E_DIRECTORY = pathlib.Path(__file__).resolve().parent / "e2e"
sys.path.insert(0, str(E2E_DIRECTORY))

import e2e_browser_scenario_contract
import e2e_browser_scenario_partition


def test_selected_contracts_share_the_canonical_browser_profile() -> None:
	"""Gateway recovery and ordinary journeys use the one browser topology."""
	registry = e2e_browser_scenario_contract.scenario_contracts()
	ordinary = e2e_browser_scenario_contract.require_contract("learner_delivery", registry)
	recovery = e2e_browser_scenario_contract.require_contract("learner_gateway_recovery", registry)
	groups = e2e_browser_scenario_partition.partition((ordinary, recovery))
	assert tuple(group.profile.value for group in groups) == ("browser",)
	assert groups[0].contracts == (ordinary, recovery)


def test_empty_selection_is_rejected() -> None:
	"""A browser run always names at least one scenario-registry-owned journey."""
	with pytest.raises(e2e_browser_scenario_partition.ScenarioPartitionError, match="empty"):
		e2e_browser_scenario_partition.partition(())

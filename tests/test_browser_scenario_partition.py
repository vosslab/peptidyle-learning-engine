"""Offline profile partitioning for the real-stack browser catalog."""

import pathlib
import sys

import pytest

E2E_DIRECTORY = pathlib.Path(__file__).resolve().parent / "e2e"
sys.path.insert(0, str(E2E_DIRECTORY))

import e2e_browser_scenario_contract
import e2e_browser_scenario_partition


def test_fault_contract_owns_a_fresh_profile_group() -> None:
	"""The deterministic exception journey cannot share ordinary browser state."""
	registry = e2e_browser_scenario_contract.scenario_contracts()
	fault = e2e_browser_scenario_contract.require_contract(
		"automated_grading_recovery", registry
	)
	ordinary = e2e_browser_scenario_contract.require_contract("learner_delivery", registry)
	groups = e2e_browser_scenario_partition.partition((ordinary, fault))
	assert tuple(group.profile.value for group in groups) == (
		"browser", "automated_grading_fault"
	)
	assert groups[1].contracts == (fault,)


def test_fault_group_rejects_multiple_deterministic_exception_journeys() -> None:
	"""One single-use fault worker remains bound to one selected scenario."""
	fault = e2e_browser_scenario_contract.require_contract("automated_grading_recovery")
	with pytest.raises(e2e_browser_scenario_partition.ScenarioPartitionError, match="exactly one"):
		e2e_browser_scenario_partition.partition((fault, fault))

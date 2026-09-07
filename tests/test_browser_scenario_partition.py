"""Offline profile partitioning for the real-stack browser scenario registry."""

import pathlib
import sys

import pytest

E2E_DIRECTORY = pathlib.Path(__file__).resolve().parent / "e2e"
sys.path.insert(0, str(E2E_DIRECTORY))

import e2e_browser_scenario_partition


def test_empty_selection_is_rejected() -> None:
	"""A browser run always names at least one scenario-registry-owned journey."""
	with pytest.raises(e2e_browser_scenario_partition.ScenarioPartitionError, match="empty"):
		e2e_browser_scenario_partition.partition(())

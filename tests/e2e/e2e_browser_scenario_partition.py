"""Partition real-stack scenarios by their owner-locked lifecycle profile."""

import dataclasses
from collections.abc import Sequence

import local_stack_control.models

import e2e_browser_scenario_contract


class ScenarioPartitionError(ValueError):
	"""A selected browser set cannot run in isolated fixed-stack groups."""


@dataclasses.dataclass(frozen=True)
class ScenarioProfileGroup:
	"""One fresh-stack group with one typed live-demo profile."""

	profile: local_stack_control.models.LiveDemoProfile
	contracts: tuple[e2e_browser_scenario_contract.ScenarioContract, ...]


def partition(
	contracts: Sequence[e2e_browser_scenario_contract.ScenarioContract],
) -> tuple[ScenarioProfileGroup, ...]:
	"""Run every selected journey in the one canonical fixed-stack profile."""
	if not contracts:
		raise ScenarioPartitionError("browser scenario selection is empty")
	return (
		ScenarioProfileGroup(local_stack_control.models.LiveDemoProfile.BROWSER, tuple(contracts)),
	)

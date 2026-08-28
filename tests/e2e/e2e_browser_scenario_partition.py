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
	"""Keep the deterministic grader fault alone; ordinary journeys share Browser."""
	ordinary: list[e2e_browser_scenario_contract.ScenarioContract] = []
	fault: list[e2e_browser_scenario_contract.ScenarioContract] = []
	for contract in contracts:
		if contract.fault_transition == "deterministic_grader_exception":
			fault.append(contract)
		else:
			ordinary.append(contract)
	if len(fault) > 1:
		raise ScenarioPartitionError(
			"deterministic grader recovery requires exactly one isolated scenario"
		)
	groups: list[ScenarioProfileGroup] = []
	if ordinary:
		groups.append(
			ScenarioProfileGroup(local_stack_control.models.LiveDemoProfile.BROWSER, tuple(ordinary))
		)
	if fault:
		groups.append(
			ScenarioProfileGroup(
				local_stack_control.models.LiveDemoProfile.AUTOMATED_GRADING_FAULT,
				tuple(fault),
			)
		)
	if not groups:
		raise ScenarioPartitionError("browser scenario selection is empty")
	return tuple(groups)

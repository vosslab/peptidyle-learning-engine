"""Shared serial lease lifecycle for fixed private acceptance profiles."""

from __future__ import annotations

import pathlib
from collections.abc import Callable

import local_stack_control.browser_suite_lease
import local_stack_control.browser_suite_reset
import local_stack_control.models
import local_stack_control.process


#============================================
def _resources_empty(snapshot: local_stack_control.models.ProjectSnapshot) -> bool:
	"""Report whether the fixed browser-owner inventory is empty."""
	result = not (snapshot.containers or snapshot.volumes or snapshot.networks)
	return result


#============================================
def run_owned_acceptance_profile(
	repository_root: pathlib.Path,
	profile_name: str,
	oracle_runner: Callable[[pathlib.Path, pathlib.Path, tuple[int, ...]], None],
	lease_factory: Callable[[pathlib.Path], local_stack_control.browser_suite_lease.BrowserSuiteLease],
	reset_runner_factory: Callable[[], local_stack_control.process.CommandRunner],
	ports_selector: Callable[[], tuple[int, ...]],
	port_checker: Callable[[tuple[int, ...], local_stack_control.process.CommandRunner, pathlib.Path], None],
) -> None:
	"""Run one leased profile in order and leave no fixed-stack resource behind.

	ASVS 2.3.1: the held suite lease serializes reset, port selection, private
	workspace creation, child execution, and final cleanup as one closed flow.
	"""
	lease = lease_factory(repository_root)
	failures: list[BaseException] = []
	try:
		local_stack_control.browser_suite_reset.reset_live_demo_browser(
			lease, reset_runner_factory(), repository_root
		)
		workspace = lease.reset_workspace()
		ports = ports_selector()
		if not ports or len(ports) != len(set(ports)):
			raise local_stack_control.models.ControllerError(
				f"{profile_name} ports are invalid"
			)
		port_checker(ports, reset_runner_factory(), repository_root)
		try:
			oracle_runner(repository_root, workspace, ports)
		except BaseException as error:
			failures.append(error)
	finally:
		try:
			final_snapshot = local_stack_control.browser_suite_reset.reset_live_demo_browser(
				lease, reset_runner_factory(), repository_root
			)
			if not _resources_empty(final_snapshot):
				raise local_stack_control.models.ControllerError(
					f"{profile_name} final reset left fixed browser resources"
				)
			workspace = lease.reset_workspace()
			if any(workspace.iterdir()):
				raise local_stack_control.models.ControllerError(
					f"{profile_name} final workspace is not empty"
				)
		except BaseException as error:
			failures.append(error)
		finally:
			lease.release()
	if len(failures) == 1:
		raise failures[0]
	if failures:
		raise BaseExceptionGroup(f"{profile_name} lifecycle failures", failures)

"""Exact reset action for the one disposable live-demo browser fixture."""

from __future__ import annotations

import pathlib

import local_stack_control.browser_suite_lease
import local_stack_control.browser_suite_ownership
import local_stack_control.discovery
import local_stack_control.models
import local_stack_control.process


#============================================
def _resource_count(snapshot: local_stack_control.models.ProjectSnapshot) -> int:
	"""Return the number of exact browser resources remaining in one inventory."""
	result = len(snapshot.containers) + len(snapshot.volumes) + len(snapshot.networks)
	return result


#============================================
def _browser_snapshot(
	runner: local_stack_control.process.CommandRunner,
	repo_root: pathlib.Path,
) -> local_stack_control.models.ProjectSnapshot:
	"""Discover only the fixed browser project; callers cannot choose a target."""
	snapshot = local_stack_control.discovery.discover_snapshot(
		runner,
		repo_root,
		local_stack_control.models.LIVE_DEMO_BROWSER_PROJECT,
	)
	return snapshot


#============================================
def _run_remove(
	runner: local_stack_control.process.CommandRunner,
	repo_root: pathlib.Path,
	argv: list[str],
) -> None:
	"""Run one exact resource deletion and retain a bounded failure surface."""
	result = runner.run(argv, cwd=repo_root)
	if not result.ok():
		raise local_stack_control.models.ControllerError("live-demo browser reset could not remove owned resources")


#============================================
def reset_live_demo_browser(
	lease: local_stack_control.browser_suite_lease.BrowserSuiteLease,
	runner: local_stack_control.process.CommandRunner,
	repo_root: pathlib.Path,
) -> local_stack_control.models.ProjectSnapshot:
	"""Remove only verified fixed browser resources and prove the result is empty.

	The action is intentionally manifest-free. It has one immutable project,
	one label authority, and direct engine IDs/names so an interrupted reset can
	be rerun by the next lease holder (ASVS 15.4.1 and 15.4.3).
	"""
	lease.require_held()
	if lease.repository_root != repo_root:
		raise local_stack_control.models.ControllerError("live-demo browser reset has an invalid checkout")
	before = _browser_snapshot(runner, repo_root)
	local_stack_control.browser_suite_ownership.require_live_demo_browser_ownership(before)
	if _resource_count(before) == 0:
		return before
	for container in before.containers:
		_run_remove(runner, repo_root, ["podman", "rm", "-f", container.id])
	for volume in before.volumes:
		_run_remove(runner, repo_root, ["podman", "volume", "rm", volume.name])
	for network in before.networks:
		_run_remove(runner, repo_root, ["podman", "network", "rm", network.name])
	after = _browser_snapshot(runner, repo_root)
	local_stack_control.browser_suite_ownership.require_live_demo_browser_ownership(after)
	if _resource_count(after) != 0:
		raise local_stack_control.models.ControllerError("live-demo browser reset left owned resources")
	return after

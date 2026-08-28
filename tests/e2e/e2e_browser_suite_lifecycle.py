"""Lease-owned lifecycle for the canonical real browser suite."""

from __future__ import annotations

import dataclasses
import pathlib
from collections.abc import Callable

import local_stack_control.browser_suite_lease
import local_stack_control.browser_suite_reset
import local_stack_control.models
import local_stack_control.private_state
import local_stack_control.process

import e2e_browser_scenario_contract
import e2e_browser_suite_evidence
import e2e_browser_fault_orchestrator
import e2e_browser_screenshot_publisher
import e2e_browser_suite_owner
import e2e_browser_suite_oracles


class _FixedWorkspaceState(local_stack_control.private_state.PrivateStateHandle):
	"""Expose the lease-owned state seam through the fixed workspace."""

	def __init__(
		self,
		lease: local_stack_control.browser_suite_lease.BrowserSuiteLease,
		directory: pathlib.Path,
	) -> None:
		self.directory = directory
		self._lease = lease

	def remove(self) -> None:
		"""Clear private artifacts while retaining the fixed checked workspace."""
		self._lease.reset_workspace()


#============================================
def _raise_failures(failures: list[BaseException]) -> None:
	"""Preserve the run failure while including an exact-reset failure."""
	if len(failures) == 1:
		raise failures[0]
	if failures:
		raise BaseExceptionGroup("browser suite lifecycle failures", failures)


#============================================
def run_owned_selection(
	selection: e2e_browser_suite_owner.BrowserSuiteSelection,
	dependency_factory: Callable[[], e2e_browser_suite_owner.BrowserSuiteDependencies],
	lease_factory: Callable[[pathlib.Path], local_stack_control.browser_suite_lease.BrowserSuiteLease] = local_stack_control.browser_suite_lease.BrowserSuiteLease.acquire,
	reset_runner_factory: Callable[[], local_stack_control.process.CommandRunner] = local_stack_control.process.SubprocessRunner,
	owner_process_reader: Callable[[tuple[local_stack_control.process.ProcessSession, ...]], tuple[e2e_browser_suite_oracles.ProcessIdentity, ...]] = e2e_browser_suite_oracles.owner_processes,
) -> e2e_browser_suite_owner.BrowserSuiteReceipt:
	"""Run one selection after exclusive ownership, fresh reset, and final reset.

	Selection validation is intentionally pure.  Every operation which can bind
	ports, read providers, create private inputs, build, or call Podman happens
	only after the checkout-scoped nonblocking lease is held.
	"""
	selection = e2e_browser_suite_owner.validate_selection(selection)
	e2e_browser_scenario_contract.validate_registry()
	root = e2e_browser_suite_owner.repo_root()
	lease = lease_factory(root)
	failures: list[BaseException] = []
	receipt: e2e_browser_suite_owner.BrowserSuiteReceipt | None = None
	inner_receipts: list[e2e_browser_suite_owner.BrowserSuiteReceipt] = []
	pending_screenshot_bundles: list[e2e_browser_screenshot_publisher.PendingScreenshotPublication] = []
	dependencies: e2e_browser_suite_owner.BrowserSuiteDependencies | None = None
	public_reporter: Callable[[e2e_browser_suite_owner.BrowserSuiteReceipt], None] | None = None
	workspace: pathlib.Path | None = None
	initial_reset_completed = False
	initial_fault_channel_reset_completed = False
	initial_fault_channel_directory_absent = False
	final_reset_completed = False
	final_fault_channel_directory_absent = False
	try:
		initial_fault_channel_directory_absent = (
			e2e_browser_fault_orchestrator.reset_stale_protocol_directory()
		)
		initial_fault_channel_reset_completed = True
		local_stack_control.browser_suite_reset.reset_live_demo_browser(
			lease, reset_runner_factory(), root
		)
		initial_reset_completed = True
		workspace = lease.reset_workspace()
		dependencies = dependency_factory()
		if dependencies.root != root:
			raise e2e_browser_suite_owner.BrowserSuiteError("browser suite dependencies have an invalid checkout")
		fixed_state = _FixedWorkspaceState(lease, workspace)
		public_reporter = dependencies.receipt_reporter
		dependencies = dataclasses.replace(
			dependencies,
			state_factory=lambda _root, _relative, _prefix: fixed_state,
			receipt_reporter=inner_receipts.append,
			_screenshot_collector=(
				pending_screenshot_bundles.append if selection.screenshots else None
			),
		)
		try:
			receipt = e2e_browser_suite_owner.run_selection(selection, dependencies)
			if len(inner_receipts) != 1 or inner_receipts[0] is not receipt:
				raise e2e_browser_suite_owner.BrowserSuiteError("browser suite inner receipt is incomplete")
		except BaseException as error:
			failures.append(error)
	finally:
		try:
			final_snapshot = local_stack_control.browser_suite_reset.reset_live_demo_browser(
				lease, reset_runner_factory(), root
			)
			final_reset_completed = True
			final_workspace = lease.reset_workspace()
			workspace_empty = not tuple(final_workspace.iterdir())
			if not workspace_empty:
				raise e2e_browser_suite_owner.BrowserSuiteError("browser suite final workspace is not empty")
			resources_empty = not (
				final_snapshot.containers or final_snapshot.volumes or final_snapshot.networks
			)
			if not resources_empty:
				raise e2e_browser_suite_owner.BrowserSuiteError("browser suite final reset left resources")
			if len(inner_receipts) == 1 and public_reporter is not None:
				inner = inner_receipts[0]
				processes_empty = not owner_process_reader(inner.owner_process_sessions)
				if not processes_empty:
					raise e2e_browser_suite_owner.BrowserSuiteError("browser suite final reset left owner processes")
				final_fault_channel_directory_absent = (
					e2e_browser_fault_orchestrator.require_protocol_directory_absent()
				)
				evidence = e2e_browser_suite_evidence.FinalFixtureEvidence(
					initial_reset_completed,
					final_reset_completed,
					workspace_empty,
					resources_empty,
					workspace_empty,
					processes_empty,
					initial_fault_channel_reset_completed,
					initial_fault_channel_directory_absent,
					final_fault_channel_directory_absent,
				)
				receipt = dataclasses.replace(inner, final_fixture_evidence=evidence)
				if selection.screenshots:
					if failures:
						raise e2e_browser_suite_owner.BrowserSuiteError(
							"screenshot capture did not complete before final publication"
						)
					if len(pending_screenshot_bundles) != 1:
						raise e2e_browser_suite_owner.BrowserSuiteError(
							"screenshot capture did not produce exactly one private bundle"
						)
					publication = e2e_browser_screenshot_publisher.publish(
						root, pending_screenshot_bundles[0]
					)
					receipt = dataclasses.replace(receipt, screenshot_evidence=publication)
				elif pending_screenshot_bundles:
					raise e2e_browser_suite_owner.BrowserSuiteError(
						"ordinary browser suite retained a screenshot bundle"
					)
				public_reporter(receipt)
		except BaseException as error:
			failures.append(error)
		finally:
			# Publication owns immutable image bytes.  Release the sole lifecycle
			# reference on every path once reset/publication has been decided.
			pending_screenshot_bundles.clear()
		lease.release()
	_raise_failures(failures)
	if receipt is None:
		raise e2e_browser_suite_owner.BrowserSuiteError("browser suite did not produce a receipt")
	print("Browser-suite: PASS")
	return receipt

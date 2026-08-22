"""Final fixed-fixture evidence for the canonical browser-suite receipt."""

from __future__ import annotations

import dataclasses

import e2e_browser_suite_oracles


@dataclasses.dataclass(frozen=True)
class FinalFixtureEvidence:
	"""Typed completion facts observed by the lease-owned lifecycle."""

	initial_reset_completed: bool
	final_exact_reset_completed: bool
	final_workspace_empty: bool
	final_resources_empty: bool
	final_private_artifacts_empty: bool
	final_owner_processes_empty: bool
	initial_fault_channel_reset_completed: bool = False
	initial_fault_channel_directory_absent: bool = False
	final_fault_channel_directory_absent: bool = False

	def as_value(self) -> dict[str, bool]:
		"""Return the small non-secret projection for the public JSON receipt."""
		return {
			"initialResetCompleted": self.initial_reset_completed,
			"finalExactResetCompleted": self.final_exact_reset_completed,
			"finalWorkspaceEmpty": self.final_workspace_empty,
			"finalResourcesEmpty": self.final_resources_empty,
			"finalPrivateArtifactsEmpty": self.final_private_artifacts_empty,
			"finalOwnerProcessesEmpty": self.final_owner_processes_empty,
			"initialFaultChannelResetCompleted": self.initial_fault_channel_reset_completed,
			"initialFaultChannelDirectoryAbsent": self.initial_fault_channel_directory_absent,
			"finalFaultChannelDirectoryAbsent": self.final_fault_channel_directory_absent,
		}


def origin_value(receipt: e2e_browser_suite_oracles.OriginReceipt) -> dict[str, object]:
	"""Project gateway observations without private browser or filesystem material."""
	return {
		"expectedOrigin": receipt.expected_origin,
		"observedPageOrigins": receipt.observed_page_origins,
		"observedRequestOrigins": receipt.observed_request_origins,
		"observedContexts": context_origins_value(receipt.observed_contexts),
	}


def context_origins_value(
	contexts: tuple[e2e_browser_suite_oracles.ContextOriginReceipt, ...],
) -> list[dict[str, object]]:
	"""Return deterministically ordered public proof for separately observed contexts."""
	return [
		{
			"name": context.name,
			"observedPageOrigins": context.observed_page_origins,
			"observedRequestOrigins": context.observed_request_origins,
		}
		for context in sorted(contexts, key=lambda item: item.name)
	]

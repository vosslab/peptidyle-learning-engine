"""Deterministic contracts for capability-bearing worker readiness polling."""

from __future__ import annotations

import dataclasses

import pytest

import local_stack_control.worker_readiness


@dataclasses.dataclass
class _Clock:
	"""A monotonic clock whose pause operation advances without sleeping."""

	now: float = 0.0

	def read(self) -> float:
		return self.now

	def pause(self, seconds: float) -> None:
		self.now += seconds


#============================================
def test_worker_readiness_requires_one_coherent_capability_receipt() -> None:
	"""The declared family count and required capability must both match."""
	assert local_stack_control.worker_readiness.attests_job_family(
		"peptidyle worker ready with 1 supported job families: GradeAcceptedSubmission",
		"GradeAcceptedSubmission",
	)
	assert not local_stack_control.worker_readiness.attests_job_family(
		"peptidyle worker ready with 2 supported job families: GradeAcceptedSubmission",
		"GradeAcceptedSubmission",
	)
	assert not local_stack_control.worker_readiness.attests_job_family(
		"peptidyle worker ready with 1 supported job families: ExportAssignment",
		"GradeAcceptedSubmission",
	)


#============================================
def test_worker_readiness_polls_until_one_coherent_receipt_is_available() -> None:
	"""A receipt emitted after launch is observed before the bounded wait expires."""
	clock = _Clock()
	observations = iter(
		(
			(False, "worker process is starting"),
			(True, "peptidyle worker ready with 1 supported job families: GradeAcceptedSubmission"),
		)
	)

	result = local_stack_control.worker_readiness.wait_for_job_family(
		lambda: next(observations),
		"GradeAcceptedSubmission",
		1.0,
		clock=clock.read,
		pause=clock.pause,
	)

	assert "GradeAcceptedSubmission" in result


#============================================
def test_worker_readiness_timeout_keeps_only_redacted_bounded_tail() -> None:
	"""A missing receipt reports useful redacted evidence without unbounded logs."""
	clock = _Clock()
	with pytest.raises(
		local_stack_control.worker_readiness.WorkerReadinessError,
		match="useful tail",
	) as error:
		local_stack_control.worker_readiness.wait_for_job_family(
			lambda: (False, "x" * 500 + "useful tail"),
			"GradeAcceptedSubmission",
			1.0,
			clock=clock.read,
			pause=clock.pause,
		)

	maximum_message = (
		len("selected stack did not become ready: ")
		+ local_stack_control.worker_readiness.MAXIMUM_FAILURE_DETAIL_CHARACTERS
	)
	assert len(str(error.value)) <= maximum_message


#============================================
def test_worker_readiness_failure_detail_stays_useful_and_bounded() -> None:
	"""Diagnostics preserve useful evidence without exposing an unbounded log."""
	empty = local_stack_control.worker_readiness.failure_detail("  ")
	short = local_stack_control.worker_readiness.failure_detail("  worker is starting  ")
	long = local_stack_control.worker_readiness.failure_detail("x" * 500 + "useful tail")

	assert empty
	assert short == "worker is starting"
	assert len(long) <= local_stack_control.worker_readiness.MAXIMUM_FAILURE_DETAIL_CHARACTERS
	assert long.endswith("useful tail")


#============================================
def test_worker_readiness_rejects_invalid_timing_before_waiting() -> None:
	"""A caller cannot disable the readiness bound or polling interval."""
	with pytest.raises(local_stack_control.worker_readiness.WorkerReadinessError):
		local_stack_control.worker_readiness.wait_for_job_family(
			lambda: (False, "not ready"),
			"GradeAcceptedSubmission",
			0.0,
		)

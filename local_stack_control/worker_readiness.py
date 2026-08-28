"""Parse the production worker's capability-bearing readiness receipt."""

from __future__ import annotations

import collections.abc
import re
import time

import local_stack_control.lifecycle_wait
import local_stack_control.models


_READINESS_PATTERN = re.compile(
	r"peptidyle worker ready with (?P<count>[1-9][0-9]*) supported job families: "
	r"(?P<families>[A-Za-z][A-Za-z0-9]*(?:, [A-Za-z][A-Za-z0-9]*)*)"
)
MAXIMUM_FAILURE_DETAIL_CHARACTERS = 320
WORKER_READINESS_TIMEOUT_SECONDS = 240.0
WORKER_READINESS_INTERVAL_SECONDS = 0.25

EvidenceRead = collections.abc.Callable[[], tuple[bool, str]]
Clock = collections.abc.Callable[[], float]
Pause = collections.abc.Callable[[float], None]


class WorkerReadinessError(local_stack_control.models.ControllerError):
	"""A bounded worker receipt failure at a lifecycle ownership boundary."""


def attests_job_family(log_text: str, required_job_family: str) -> bool:
	"""Return whether one coherent readiness receipt includes the required family."""
	for match in _READINESS_PATTERN.finditer(log_text):
		families = match.group("families").split(", ")
		if int(match.group("count")) == len(families) and required_job_family in families:
			return True
	return False


#============================================
def failure_detail(redacted_log_text: str) -> str:
	"""Return one bounded explanation from adapter-redacted worker evidence."""
	detail = redacted_log_text.strip()
	if detail == "":
		return "worker evidence was empty"
	return detail[-MAXIMUM_FAILURE_DETAIL_CHARACTERS:]


#============================================
def wait_for_job_family(
	read_evidence: EvidenceRead,
	required_job_family: str,
	timeout_seconds: float = WORKER_READINESS_TIMEOUT_SECONDS,
	*,
	clock: Clock = time.monotonic,
	pause: Pause = time.sleep,
	interval_seconds: float = WORKER_READINESS_INTERVAL_SECONDS,
) -> str:
	"""Wait for one coherent capability-bearing worker receipt."""
	def is_ready(observation: tuple[bool, str]) -> bool:
		command_succeeded, output = observation
		return command_succeeded and attests_job_family(output, required_job_family)

	def detail(observation: tuple[bool, str]) -> str:
		return failure_detail(observation[1])

	try:
		_observed_ok, output = local_stack_control.lifecycle_wait.poll_until(
			read_evidence,
			is_ready,
			timeout_seconds,
			clock=clock,
			pause=pause,
			interval_seconds=interval_seconds,
			detail=detail,
		)
	except local_stack_control.models.ControllerError as error:
		raise WorkerReadinessError(str(error)) from error
	return output

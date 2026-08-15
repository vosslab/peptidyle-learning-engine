"""Bounded polling and semantic readiness helpers for lifecycle owners."""

import collections.abc
import time

import local_stack_control.models
import local_stack_control.status


PollRead = collections.abc.Callable[[], local_stack_control.models.StatusReport]
Clock = collections.abc.Callable[[], float]
Pause = collections.abc.Callable[[float], None]


#============================================
def require_ready(report: local_stack_control.models.StatusReport) -> None:
	"""Raise an operator-facing failure when semantic readiness is incomplete."""
	if not report.ok:
		raise local_stack_control.models.ControllerError(
			"selected stack is not ready: " + report.message
		)


#============================================
def poll_ready(
	read_report: PollRead,
	timeout_seconds: float,
	clock: Clock = time.monotonic,
	pause: Pause = time.sleep,
	interval_seconds: float = 0.25,
) -> local_stack_control.models.StatusReport:
	"""Return a ready report or the last semantic failure before a bounded timeout."""
	if timeout_seconds <= 0 or interval_seconds <= 0:
		raise local_stack_control.models.ControllerError("readiness timing must be positive")
	deadline = clock() + timeout_seconds
	last_report = read_report()
	while not last_report.ok and clock() < deadline:
		pause(interval_seconds)
		last_report = read_report()
	if not last_report.ok:
		raise local_stack_control.models.ControllerError(
			"selected stack did not become ready: " + last_report.message
		)
	return last_report


#============================================
def require_one_shot_completion(report: local_stack_control.models.StatusReport) -> None:
	"""Reject missing, duplicate, running, or failed required one-shot services."""
	for service in report.services:
		if service.service not in local_stack_control.status.required_one_shots(report.with_smtp):
			continue
		if not service.complete:
			raise local_stack_control.models.ControllerError(
				f"required one-shot {service.service} has not completed successfully"
			)

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
def poll_until[PollValue](
	read_value: collections.abc.Callable[[], PollValue],
	is_ready: collections.abc.Callable[[PollValue], bool],
	timeout_seconds: float,
	*,
	clock: Clock = time.monotonic,
	pause: Pause = time.sleep,
	interval_seconds: float = 0.25,
	detail: collections.abc.Callable[[PollValue], str] | None = None,
) -> PollValue:
	"""Return a value once its semantic predicate is true within a bound."""
	if timeout_seconds <= 0 or interval_seconds <= 0:
		raise local_stack_control.models.ControllerError("readiness timing must be positive")
	deadline = clock() + timeout_seconds
	value = read_value()
	ready = is_ready(value)
	while not ready and clock() < deadline:
		remaining = deadline - clock()
		if remaining <= 0:
			break
		pause(min(interval_seconds, remaining))
		value = read_value()
		ready = is_ready(value)
	if not ready:
		message = "readiness condition was not met"
		if detail is not None:
			message = detail(value)
		raise local_stack_control.models.ControllerError(
			"selected stack did not become ready: " + message
		)
	return value


#============================================
def poll_ready(
	read_report: PollRead,
	timeout_seconds: float,
	clock: Clock = time.monotonic,
	pause: Pause = time.sleep,
	interval_seconds: float = 0.25,
) -> local_stack_control.models.StatusReport:
	"""Return a ready report or the last semantic failure before a bounded timeout."""
	return poll_until(
		read_report,
		lambda report: report.ok,
		timeout_seconds,
		clock=clock,
		pause=pause,
		interval_seconds=interval_seconds,
		detail=lambda report: report.message,
	)


#============================================
def require_one_shot_completion(report: local_stack_control.models.StatusReport) -> None:
	"""Reject missing, duplicate, running, or failed required one-shot services."""
	for service in report.services:
		if service.service not in local_stack_control.status.required_one_shots():
			continue
		if not service.complete:
			raise local_stack_control.models.ControllerError(
				f"required one-shot {service.service} has not completed successfully"
			)

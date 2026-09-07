"""Bounded redacted diagnostics for retained local-stack failures."""

import re

import local_stack_control.models


MAXIMUM_DIAGNOSTIC_CHARACTERS = 320
SQL_OR_SEED_LOG_LINE = re.compile(
	r"\b(?:alter|create|delete|drop|insert|select|seed|update)\b", re.IGNORECASE,
)


#============================================
def redacted_failure_detail(
	result: local_stack_control.models.CommandResult,
	private_values: tuple[str, ...] = (),
) -> str:
	"""Return a bounded child failure summary with supplied private material removed."""
	text = "child reported a failure"
	if private_values:
		text = "\n".join((result.stdout, result.stderr)).strip()
		for value in sorted(set(private_values), key=len, reverse=True):
			if value != "":
				text = text.replace(value, "[private]")
		text = re.sub(r"(?:postgres|postgresql)://[^@\s]+@", "postgres://[private]@", text)
		if text == "":
			text = "child reported a failure"
	text = text[-MAXIMUM_DIAGNOSTIC_CHARACTERS:]
	return text


#============================================
def redacted_postgres_service_detail(
	result: local_stack_control.models.CommandResult,
	private_values: tuple[str, ...],
) -> str:
	"""Return bounded PostgreSQL lifecycle evidence without SQL or seed content."""
	redacted = redacted_failure_detail(result, private_values)
	safe_lines = tuple(
		line for line in redacted.splitlines()
		if SQL_OR_SEED_LOG_LINE.search(line) is None
	)
	if len(safe_lines) == 0:
		return "PostgreSQL service reported no safe lifecycle detail"
	return "\n".join(safe_lines)[-MAXIMUM_DIAGNOSTIC_CHARACTERS:]

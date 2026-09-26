"""Bounded redacted diagnostics for retained local-stack failures."""

import re

import local_stack_control.models


MAXIMUM_DIAGNOSTIC_CHARACTERS = 320
MAXIMUM_COMMAND_EXCERPT_CHARACTERS = 2_048
SQL_OR_SEED_LOG_LINE = re.compile(
	r"\b(?:alter|create|delete|drop|insert|select|seed|update)\b", re.IGNORECASE,
)


#============================================
def redacted_failure_detail(
	result: local_stack_control.models.CommandResult,
	private_values: tuple[str, ...] = (),
) -> str:
	"""Return bounded redacted excerpts from both child output streams."""
	if not private_values:
		return "child reported a failure"
	private_markers = tuple(sorted(set(private_values), key=len, reverse=True))
	excerpts = []
	# ASVS 13.3.2, 16.2.5, and 16.5.1: redact each stream before retaining
	# its bounded diagnostic excerpt for the local operator.
	for stream_text in (result.stderr, result.stdout):
		redacted = stream_text
		for value in private_markers:
			if value != "":
				redacted = redacted.replace(value, "[private]")
		redacted = re.sub(r"(?:postgres|postgresql)://[^@\s]+@", "postgres://[private]@", redacted)
		if redacted.strip() != "":
			excerpts.append(redacted.strip()[-MAXIMUM_COMMAND_EXCERPT_CHARACTERS:])
	if len(excerpts) == 0:
		return "child reported a failure"
	# psql reports the schema failure on stdout when Compose attaches to a job.
	# Prefer that redacted cause over the provider banner and exit-status wrapper.
	database_errors = [
		line.strip() for excerpt in excerpts for line in excerpt.splitlines()
		if re.search(r"psql:.*\b(?:ERROR|FATAL):", line)
	]
	if database_errors:
		return "\n".join(database_errors)[-MAXIMUM_COMMAND_EXCERPT_CHARACTERS:]
	return "\n".join(excerpts)


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

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
	database_error_context = []
	rust_failure_context = []
	rust_error_context = []
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
		lines = redacted.splitlines()
		for index, line in enumerate(lines):
			cargo_test_failure = re.search(r"^\s*error:\s*test failed\b", line, re.IGNORECASE)
			compose_wrapper = re.search(
				r"^\s*Error:\s*executing\b.*\bcompose\b.*\bexit status\b",
				line,
				re.IGNORECASE,
			)
			if cargo_test_failure is None and compose_wrapper is None and re.search(
				r"^\s*(?:psql:[^\n]*?:\s*)?(?:ERROR|FATAL|PANIC):", line, re.IGNORECASE,
			):
				database_error_context.append(line.strip())
				for context_line in lines[index + 1:index + 5]:
					if re.search(r"^\s*(?:DETAIL|HINT|CONTEXT|LINE|QUERY):", context_line, re.IGNORECASE):
						database_error_context.append(context_line.strip())
			failure_header = re.search(
				r"^\s*(?:thread\s+['\"].+['\"](?:\s+\(\d+\))?\s+panicked\s+at|assertion\b.+\bfailed\b)",
				line,
				re.IGNORECASE,
			)
			error_header = re.search(r"^\s*error:", line, re.IGNORECASE)
			if not rust_failure_context and (failure_header or (error_header and not rust_error_context)):
				# Rust test failures put the panic location or assertion first, followed
				# by a small left/right or panic-message body.
				context = rust_failure_context if failure_header else rust_error_context
				for context_line in lines[index:index + 5]:
					if re.search(
						r"^\s*(?:test result:|---- .* stdout ----|running \d+ tests?)",
						context_line,
						re.IGNORECASE,
					) is not None:
						break
					bounded_line = context_line.strip()[:384]
					if bounded_line != "":
						context.append(bounded_line)
	if len(excerpts) == 0:
		return "child reported a failure"
	# PostgreSQL may report errors with or without a psql path prefix. Retain the
	# redacted cause before clipping long query output or provider boilerplate.
	if database_error_context:
		return "\n".join(database_error_context + rust_failure_context)[-MAXIMUM_COMMAND_EXCERPT_CHARACTERS:]
	if rust_failure_context:
		return "\n".join(rust_failure_context)[-MAXIMUM_COMMAND_EXCERPT_CHARACTERS:]
	if rust_error_context:
		return "\n".join(rust_error_context)[-MAXIMUM_COMMAND_EXCERPT_CHARACTERS:]
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

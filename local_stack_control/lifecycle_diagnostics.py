"""Bounded redacted diagnostics for retained local-stack failures."""

import local_stack_control.models


MAXIMUM_DIAGNOSTIC_CHARACTERS = 320


#============================================
def redacted_failure_detail(
	result: local_stack_control.models.CommandResult,
	private_values: tuple[str, ...] = (),
) -> str:
	"""Return a bounded child failure summary with supplied private material removed."""
	text = "child reported a failure"
	for value in private_values:
		if value != "":
			text = text.replace(value, "[private]")
	text = text[:MAXIMUM_DIAGNOSTIC_CHARACTERS]
	return text

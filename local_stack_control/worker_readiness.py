"""Parse the production worker's capability-bearing readiness receipt."""

from __future__ import annotations

import re


_READINESS_PATTERN = re.compile(
	r"peptidyle worker ready with (?P<count>[1-9][0-9]*) supported job families: "
	r"(?P<families>[A-Za-z][A-Za-z0-9]*(?:, [A-Za-z][A-Za-z0-9]*)*)"
)


def attests_job_family(log_text: str, required_job_family: str) -> bool:
	"""Return whether one coherent readiness receipt includes the required family."""
	for match in _READINESS_PATTERN.finditer(log_text):
		families = match.group("families").split(", ")
		if int(match.group("count")) == len(families) and required_job_family in families:
			return True
	return False

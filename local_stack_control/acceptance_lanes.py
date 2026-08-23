"""Ordered, typed aggregate browser-validation lanes."""

import dataclasses
import enum
import pathlib
import shlex

import local_stack_control.process


class EvidenceBoundary(enum.StrEnum):
	"""The specific claim an aggregate-validation lane contributes."""

	CANONICAL_PRODUCTION_BROWSER = "canonical production-browser behavior"
	REAL_SERVICE = "real-service boundary"


@dataclasses.dataclass(frozen=True)
class ValidationLane:
	"""One fixed aggregate-validation command and its declared evidence boundary."""

	name: str
	argv: tuple[str, ...]
	evidence_boundary: EvidenceBoundary


#============================================
def lanes() -> tuple[ValidationLane, ...]:
	"""Return the closed ordered validation contract without lifecycle authority."""
	result = (
		ValidationLane(
			"canonical production-browser behavior",
			("bash", "run_playwright_tests.sh", "--build"),
			EvidenceBoundary.CANONICAL_PRODUCTION_BROWSER,
		),
		ValidationLane(
			"isolated WebWork renderer service oracle",
			("bash", "tests/e2e/e2e_webwork_render_rpc.sh"),
			EvidenceBoundary.REAL_SERVICE,
		),
		ValidationLane(
			"isolated replica restart service oracle",
			("node", "tests/e2e/e2e_replica_restart.mjs"),
			EvidenceBoundary.REAL_SERVICE,
		),
	)
	return result


#============================================
def run(
	runner: local_stack_control.process.CommandRunner,
	repo_root: pathlib.Path,
	environment: dict[str, str],
) -> int:
	"""Run every fixed lane in order and preserve its nonzero result exactly."""
	for lane in lanes():
		print()
		print(f"==> Playwright validation: {lane.name}")
		print(f"Evidence boundary: {lane.evidence_boundary.value}")
		print("Command: " + shlex.join(lane.argv))
		result = runner.stream(list(lane.argv), environment, repo_root)
		if result != 0:
			return result
		print(f"PASS: {lane.name}")
	print()
	print("PASS: complete Playwright validation is green.")
	return 0

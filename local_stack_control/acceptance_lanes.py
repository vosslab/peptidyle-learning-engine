"""Ordered, typed aggregate browser-validation lanes."""

import dataclasses
import enum
import pathlib
import shlex
import sys

import local_stack_control.process


class EvidenceBoundary(enum.StrEnum):
	"""The specific claim an aggregate-validation lane contributes."""

	CANONICAL_PRODUCTION_BROWSER = "canonical production-browser behavior"
	TRANSITIONAL_VISUAL_FIXTURE = "transitional visual-fixture evidence"
	REAL_SERVICE = "real-service boundary"
	UI_WALKTHROUGH = "UI walkthrough boundary"


@dataclasses.dataclass(frozen=True)
class ValidationLane:
	"""One fixed aggregate-validation command and its declared evidence boundary."""

	name: str
	argv: tuple[str, ...]
	evidence_boundary: EvidenceBoundary


#============================================
def lanes(python_executable: str | None = None) -> tuple[ValidationLane, ...]:
	"""Return the closed ordered validation contract without lifecycle authority."""
	python = sys.executable if python_executable is None else python_executable
	result = (
		ValidationLane(
			"canonical production-browser behavior",
			("bash", "run_playwright_tests.sh", "--build"),
			EvidenceBoundary.CANONICAL_PRODUCTION_BROWSER,
		),
		ValidationLane(
			"transitional course-appearance visual fixture",
			("node", "tests/playwright/verify_course_appearance_visuals.mjs"),
			EvidenceBoundary.TRANSITIONAL_VISUAL_FIXTURE,
		),
		ValidationLane(
			"transitional instructor-page visual fixture corpus",
			("node", "tests/playwright/capture_instructor_page_visuals.mjs", "--verify-only"),
			EvidenceBoundary.TRANSITIONAL_VISUAL_FIXTURE,
		),
		ValidationLane(
			"canonical instructor-to-student walkthrough",
			(
				python,
				"-m",
				"tests.walkthrough.run_ui_walkthrough",
				"--master-seed",
				"42",
				"--build",
			),
			EvidenceBoundary.UI_WALKTHROUGH,
		),
		ValidationLane(
			"isolated Chapter 1 publication oracle",
			(python, "tests/e2e/e2e_chapter_one_pilot.py"),
			EvidenceBoundary.REAL_SERVICE,
		),
		ValidationLane(
			"isolated Chapter 1 real-browser journey with live Question-ID replacement",
			(python, "tests/e2e/e2e_chapter_one_browser.py"),
			EvidenceBoundary.REAL_SERVICE,
		),
		ValidationLane(
			"isolated disposable WebWork browser acceptance",
			("bash", "tests/e2e/e2e_webwork_render_rpc.sh"),
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

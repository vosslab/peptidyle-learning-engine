"""Ordered, typed aggregate browser-validation lanes."""

import dataclasses
import pathlib
import shlex
import sys

import local_stack_control.process


@dataclasses.dataclass(frozen=True)
class ValidationLane:
	"""One fixed aggregate-validation command and its human receipt name."""

	name: str
	argv: tuple[str, ...]


#============================================
def lanes(python_executable: str | None = None) -> tuple[ValidationLane, ...]:
	"""Return the closed ordered validation contract without lifecycle authority."""
	python = sys.executable if python_executable is None else python_executable
	result = (
		ValidationLane(
			"ordinary built demo-browser suite",
			("bash", "run_playwright_tests.sh", "--build"),
		),
		ValidationLane(
			"course-appearance visual evidence",
			("node", "tests/playwright/verify_course_appearance_visuals.mjs"),
		),
		ValidationLane(
			"demo instructor-page visual corpus",
			("node", "tests/playwright/capture_instructor_page_visuals.mjs", "--verify-only"),
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
		),
		ValidationLane(
			"isolated Chapter 1 publication oracle",
			(python, "tests/e2e/e2e_chapter_one_pilot.py"),
		),
		ValidationLane(
			"isolated Chapter 1 real-browser journey with live Question-ID replacement",
			(python, "tests/e2e/e2e_chapter_one_browser.py"),
		),
		ValidationLane(
			"isolated disposable WebWork browser acceptance",
			("bash", "tests/e2e/e2e_webwork_render_rpc.sh"),
		),
		ValidationLane(
			"connected ordinary-site live-demo browser journey",
			(python, "tests/e2e/e2e_live_demo_browser.py"),
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
		print("Command: " + shlex.join(lane.argv))
		result = runner.stream(list(lane.argv), environment, repo_root)
		if result != 0:
			return result
		print(f"PASS: {lane.name}")
	print()
	print("PASS: complete Playwright validation is green.")
	return 0

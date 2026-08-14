#!/usr/bin/env bash
# Private lane runner.  The local-stack controller owns lifecycle preflight.

set -euo pipefail

fail() {
	echo "FAIL: Playwright validation: $*" >&2
	exit 1
}

run_lane() {
	local lane_name="$1"
	shift
	echo
	echo "==> Playwright validation: ${lane_name}"
	"$@"
	echo "PASS: ${lane_name}"
}

if [ "$#" -ne 0 ]; then
	fail "the lane runner accepts no arguments; use ./run_playwright_validation.sh --live"
fi

run_lane "ordinary built mock-browser suite" bash run_playwright_tests.sh --build
run_lane "course-appearance visual evidence" node tests/playwright/verify_course_appearance_visuals.mjs
run_lane "simulated instructor-page visual corpus" node tests/playwright/capture_instructor_page_visuals.mjs --verify-only
run_lane "canonical instructor-to-student walkthrough" \
	bash tests/walkthrough/run_ui_walkthrough.sh --master-seed 42 --build
run_lane "isolated Chapter 1 real-browser journey" bash tests/e2e/e2e_chapter_one_browser.sh
run_lane "canonical-stack WebWork browser acceptance" bash tests/e2e/e2e_webwork_render_rpc.sh

echo
echo "PASS: complete Playwright validation is green."

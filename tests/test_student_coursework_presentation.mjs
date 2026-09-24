import assert from "node:assert/strict";
import test from "node:test";

import {
  hasSubmittedStudentAttempt,
  isInStudentDueSoonWindow,
  studentCourseworkDisplay,
} from "../src/pages/student_coursework_presentation.ts";

test("Coursework display distinguishes resumable, non-resumable unfinished, and completed work", () => {
  const cases = [
    ["attempt_limit_reached", "inProgress", true, "inProgress", "In progress"],
    ["attempt_limit_reached", "inProgress", false, "missed", "Not completed"],
    ["closed", "completed", false, "completed", "Completed"],
  ];

  for (const [decision, completion, canResume, expectedState, expectedCompletion] of cases) {
    const display = studentCourseworkDisplay(decision, completion, canResume);
    assert.equal(display.state, expectedState, `${decision}/${completion ?? "none"}`);
    assert.equal(
      display.completionLabel,
      expectedCompletion,
      `${decision}/${completion ?? "none"}`,
    );
  }
});

test("Coursework actions describe the overview destination without promising an immediate start", () => {
  const cases = [
    ["may_start", "inProgress", true, "Resume"],
    ["closed", "inProgress", false, "Open"],
    ["may_start", "completed", false, "Review"],
    ["attempt_limit_reached", "completed", false, "Review"],
    ["may_start", null, false, "Open"],
    ["not_yet_available", null, false, "Open"],
    ["closed", null, false, "Open"],
  ];

  for (const [decision, completion, canResume, expectedAction] of cases) {
    assert.equal(
      studentCourseworkDisplay(decision, completion, canResume).actionVerb,
      expectedAction,
      `${decision}/${completion ?? "none"}`,
    );
  }
});

test("Student Due Soon uses the server instant and a half-open rolling seven-day window", () => {
  const now = 1_790_000_000_000;
  const sevenDays = 7 * 24 * 60 * 60 * 1000;
  assert.equal(isInStudentDueSoonWindow(now - 1, now), false);
  assert.equal(isInStudentDueSoonWindow(now, now), true);
  assert.equal(isInStudentDueSoonWindow(now + sevenDays - 1, now), true);
  assert.equal(isInStudentDueSoonWindow(now + sevenDays, now), false);
  assert.equal(isInStudentDueSoonWindow(null, now), false);
});

test("Completed membership means at least one submitted Attempt", () => {
  for (const [count, expected] of [
    [0, false],
    [1, true],
    [3, true],
    [-1, false],
    [1.5, false],
  ]) {
    assert.equal(hasSubmittedStudentAttempt(count), expected);
  }
});

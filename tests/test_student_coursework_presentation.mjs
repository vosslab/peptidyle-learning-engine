import assert from "node:assert/strict";
import test from "node:test";

import { studentCourseworkDisplay } from "../src/pages/student_coursework_presentation.ts";

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

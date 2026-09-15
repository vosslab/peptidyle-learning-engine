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

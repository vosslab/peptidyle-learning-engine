import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import test from "node:test";

import {
  J3_VISIBLE_OUTCOME_CODES,
  passedJ3LeaveReturnFragment,
} from "./playwright/simulator/j3_leave_return_fragment.ts";

const COURSE = "123e4567-e89b-12d3-a456-426614174000";
const ASSIGNMENT = "123e4567-e89b-12d3-a456-426614174001";

test("J3 public evidence records only visible leave-return milestones", () => {
  const fragment = passedJ3LeaveReturnFragment(COURSE, ASSIGNMENT, 12);
  assert.deepEqual(fragment.visibleOutcomeCodes, J3_VISIBLE_OUTCOME_CODES);
  assert.deepEqual(fragment.diagnostics, []);
});

test("J3 public evidence rejects invalid identifiers and unbounded time", () => {
  assert.throws(() => passedJ3LeaveReturnFragment("not-a-uuid", ASSIGNMENT, 12));
  assert.throws(() => passedJ3LeaveReturnFragment(COURSE, ASSIGNMENT, 30 * 60 * 1000 + 1));
});

test("J3 uses the rendered recovery control instead of a pointer, Escape, or history shortcut", () => {
  const source = readFileSync("tests/playwright/ui_walkthrough_keyboard_j3.spec.ts", "utf8");
  assert.doesNotMatch(source, /\.click\(|keyboard\.press\("Escape"\)|goBack\(/u);
  assert.match(source, /test\.setTimeout\(90_000\)/u);
  assert.match(source, /await expect\(assignmentLink\)\.toHaveCount\(1\)/u);
  assert.match(source, /await expect\(assignmentLink\)\.toBeVisible\(\)/u);
  assert.match(source, /tabTo\(page, assignmentLink, "backward"\)/u);
  assert.match(source, /getByRole\("button", \{ name: "Return to assignment" \}\)/u);
  assert.match(source, /\[data-route-surface=assignmentOverview\]/u);
  assert.match(source, /\[data-route-surface=runAttempt\].*timeout: 15_000/su);
});

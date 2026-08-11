import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import test from "node:test";

import {
  J4_VISIBLE_OUTCOME_CODES,
  passedJ4PolicyContrastFragment,
} from "./playwright/simulator/j4_policy_contrast_fragment.ts";
import {
  classifyJ4TerminalSurface,
  isJ4TerminalSurfaceTerminal,
} from "./playwright/simulator/j4_terminal_surface.ts";

const COURSE = "123e4567-e89b-12d3-a456-426614174000";
const MASTERY = "123e4567-e89b-12d3-a456-426614174001";
const EXAM = "123e4567-e89b-12d3-a456-426614174002";

test("J4 public evidence contains only paired visible policy milestones", () => {
  const fragment = passedJ4PolicyContrastFragment(COURSE, MASTERY, EXAM, 12);
  assert.deepEqual(fragment.visibleOutcomeCodes, J4_VISIBLE_OUTCOME_CODES);
  assert.deepEqual(fragment.diagnostics, []);
  assert.equal(fragment.courseId, COURSE);
  assert.equal(fragment.masteryAssignmentId, MASTERY);
  assert.equal(fragment.examAssignmentId, EXAM);
});

test("J4 public evidence rejects private-looking identifiers and unbounded time", () => {
  assert.throws(() => passedJ4PolicyContrastFragment("answer-key", MASTERY, EXAM, 12));
  assert.throws(() => passedJ4PolicyContrastFragment(COURSE, MASTERY, EXAM, 30 * 60 * 1000 + 1));
});

test("J4 terminal classifier keeps asynchronous non-final surfaces transient", () => {
  const transient = [
    {
      freshPractice: false,
      masteryHeading: false,
      closedHeading: false,
      neutralHeading: false,
      feedback: false,
      inlineErrors: 0,
    },
    {
      freshPractice: false,
      masteryHeading: false,
      closedHeading: false,
      neutralHeading: true,
      feedback: false,
      inlineErrors: 0,
    },
    {
      freshPractice: false,
      masteryHeading: false,
      closedHeading: false,
      neutralHeading: false,
      feedback: true,
      inlineErrors: 0,
    },
  ];
  for (const observation of transient) {
    const surface = classifyJ4TerminalSurface(observation);
    assert.equal(isJ4TerminalSurfaceTerminal(surface), false);
  }
  assert.equal(
    classifyJ4TerminalSurface({
      freshPractice: true,
      masteryHeading: true,
      closedHeading: false,
      neutralHeading: false,
      feedback: false,
      inlineErrors: 0,
    }),
    "mastery",
  );
  assert.equal(
    classifyJ4TerminalSurface({
      freshPractice: false,
      masteryHeading: false,
      closedHeading: true,
      neutralHeading: false,
      feedback: false,
      inlineErrors: 0,
    }),
    "closed",
  );
});

test("J4 uses rendered keyboard controls and observes no feedback or correctness content", () => {
  const source = readFileSync("tests/playwright/ui_walkthrough_keyboard_j4.spec.ts", "utf8");
  assert.doesNotMatch(source, /\.click\(|\.evaluate\(|goBack\(|keyboard\.press\("Escape"\)/u);
  assert.doesNotMatch(source, /correct|incorrect|rationale|answerKey|score/iu);
  assert.match(source, /Start another practice run/u);
  assert.match(source, /This run is complete/u);
  assert.match(source, /Back to assignment/u);
  assert.match(source, /async function startVisibleMasteryRun/u);
  assert.match(source, /async function startVisibleExamRun/u);
  assert.match(source, /await expect\(freshPractice\)\.toHaveCount\(0\)/u);
  assert.match(source, /await expect\(masteryHeading\)\.toHaveCount\(0\)/u);
  assert.match(source, /await expect\(assignmentLink\)\.toHaveCount\(1\)/u);
  assert.match(source, /await expect\(assignmentLink\)\.toBeVisible\(\)/u);
  assert.match(source, /tabTo\(page, assignmentLink, "backward"\)/u);
  assert.match(source, /input\[type="radio"\]:visible/u);
  assert.match(source, /await expect\(radios\)\.toHaveCount\(2\)/u);
  assert.match(source, /choiceIndex === 0 \? "forward" : "backward"/u);
  assert.match(source, /waitForTerminalSurface/u);
  assert.doesNotMatch(source, /Arrow(?:Left|Right|Up|Down)|Digit[0-9]|tabindex/u);
});

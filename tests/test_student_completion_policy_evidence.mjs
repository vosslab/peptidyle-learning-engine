import assert from "node:assert/strict";
import test from "node:test";

import { passedStudentCompletionPolicyEvidence } from "./playwright/simulator/student_completion_policy_evidence.ts";
import {
  classifyStudentCompletionTerminalSurface,
  isStudentCompletionTerminalSurface,
} from "./playwright/simulator/student_completion_terminal_surface.ts";

const COURSE = "123e4567-e89b-12d3-a456-426614174000";
const MASTERY = "123e4567-e89b-12d3-a456-426614174001";
const EXAM = "123e4567-e89b-12d3-a456-426614174002";

test("completion-policy evidence rejects private-looking identifiers and unbounded time", () => {
  assert.throws(() => passedStudentCompletionPolicyEvidence("answer-key", MASTERY, EXAM, 12));
  assert.throws(() =>
    passedStudentCompletionPolicyEvidence(COURSE, MASTERY, EXAM, 30 * 60 * 1000 + 1),
  );
});

test("completion classifier keeps asynchronous surfaces non-terminal", () => {
  const surfaces = [
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
  ].map(classifyStudentCompletionTerminalSurface);
  assert.deepEqual(surfaces, ["pending", "neutral", "feedback"]);
  assert.deepEqual(surfaces.map(isStudentCompletionTerminalSurface), [false, false, false]);
});

test("completion classifier recognizes rendered mastery and closed outcomes", () => {
  const surfaces = [
    {
      freshPractice: true,
      masteryHeading: true,
      closedHeading: false,
      neutralHeading: false,
      feedback: false,
      inlineErrors: 0,
    },
    {
      freshPractice: false,
      masteryHeading: false,
      closedHeading: true,
      neutralHeading: false,
      feedback: false,
      inlineErrors: 0,
    },
  ].map(classifyStudentCompletionTerminalSurface);
  assert.deepEqual(surfaces, ["mastery", "closed"]);
  assert.deepEqual(surfaces.map(isStudentCompletionTerminalSurface), [true, true]);
});

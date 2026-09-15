// test_student_feedback_panel.mjs - permanent server-View behavior checks for Student Feedback UI.

import assert from "node:assert/strict";
import test from "node:test";

import { studentFeedbackAnnouncement } from "../src/components/student_feedback_panel.tsx";

test("withheld and released Student Feedback announce distinct, policy-neutral states", () => {
  const withheld = studentFeedbackAnnouncement({
    kind: "awaiting",
    feedback: null,
    assessmentScoringState: "current",
  });
  const released = studentFeedbackAnnouncement({
    kind: "released",
    feedback: { correctness: true },
    assessmentScoringState: "current",
  });

  assert.match(withheld, /not available/);
  assert.doesNotMatch(withheld, /Correct|Not quite/);
  assert.match(released, /Correct/);
});

test("non-current scores stay hidden behind recoverable Student Feedback states", () => {
  const recalculating = studentFeedbackAnnouncement({
    kind: "released",
    feedback: { correctness: true },
    assessmentScoringState: "recalculating",
  });
  const failed = studentFeedbackAnnouncement({
    kind: "released",
    feedback: { correctness: false },
    assessmentScoringState: "failed",
  });

  assert.match(recalculating, /score is being updated/);
  assert.match(failed, /waiting for instructor review/);
  assert.doesNotMatch(`${recalculating} ${failed}`, /Correct|Not quite/);
});

test("released Student Feedback with no disclosed fields remains neutral", () => {
  const announcement = studentFeedbackAnnouncement({
    kind: "released",
    feedback: {},
    assessmentScoringState: "current",
  });

  assert.match(announcement, /response (?:was )?recorded/i);
  assert.doesNotMatch(announcement, /Correct|Not quite/);
});

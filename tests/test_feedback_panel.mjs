// test_feedback_panel.mjs - permanent server-projection behavior checks for feedback UI.

import assert from "node:assert/strict";
import test from "node:test";

import { feedbackAnnouncement } from "../src/components/feedback_panel.tsx";

test("withheld and released feedback announce distinct, policy-neutral states", () => {
  assert.equal(
    feedbackAnnouncement({ kind: "awaiting", feedback: null, scoringStatus: "current" }),
    "Your response was recorded. Feedback is not available for this response.",
  );
  assert.equal(
    feedbackAnnouncement({
      kind: "released",
      feedback: { correctness: true },
      scoringStatus: "current",
    }),
    "Feedback released. Correct.",
  );
});

test("non-current scores announce recoverable grading states", () => {
  assert.equal(
    feedbackAnnouncement({
      kind: "released",
      feedback: { hint: [{ kind: "text", markdown: "Review the peptide bond." }] },
      scoringStatus: "recalculating",
    }),
    "Your response was recorded. Your score is being updated.",
  );
  assert.equal(
    feedbackAnnouncement({
      kind: "released",
      feedback: { hint: [{ kind: "text", markdown: "Review the peptide bond." }] },
      scoringStatus: "failed",
    }),
    "Your response was recorded. Your score is waiting for instructor review.",
  );
});

test("released feedback with no disclosed fields remains neutral", () => {
  assert.equal(
    feedbackAnnouncement({ kind: "released", feedback: {}, scoringStatus: "current" }),
    "Feedback released. Your response was recorded.",
  );
});

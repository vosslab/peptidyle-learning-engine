// test_feedback_panel.mjs - permanent server-projection behavior checks for feedback UI.

import assert from "node:assert/strict";
import test from "node:test";

import { feedbackAnnouncement } from "../src/components/feedback_panel.tsx";

test("withheld and released feedback announce distinct, policy-neutral states", () => {
  assert.equal(
    feedbackAnnouncement({ kind: "awaiting", feedback: null }),
    "Your response was recorded. Feedback is not available for this response.",
  );
  assert.equal(
    feedbackAnnouncement({ kind: "released", feedback: { correctness: true } }),
    "Feedback released. Correct.",
  );
});

test("released feedback with no disclosed fields remains neutral", () => {
  assert.equal(
    feedbackAnnouncement({ kind: "released", feedback: {} }),
    "Feedback released. Your response was recorded.",
  );
});
